//! Pure, explicitly selected scene planning and sequential execution bookkeeping.
//!
//! This module performs no I/O, queues no engine messages and never actuates a
//! board. The integration owner dispatches exactly the request returned by
//! `Operation::next_request`, and returns its ticket with a typed completion.
//! Tick while a call is pending: losing the pin or the overall deadline makes
//! that write Unknown, not safely retryable. Never feed these requests through
//! the coalescing/500 ms live-slider queue.

use crate::four_d::controller::{HardwareCapabilities, HardwareControl};
use crate::projects::{ChannelValue, Scene};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fmt;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub const MAX_OUTPUTS: usize = 64;
pub const DEADLINE: Duration = Duration::from_secs(30);
pub const METHOD: &str = "controller.output.set";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    InvalidScene,
    EmptySelection,
    TooManyOutputs,
    InvalidKey,
    DuplicateSelection,
    Unsampled,
    MissingValue,
    UnknownControl,
    AmbiguousControl,
    Locked,
    UnsupportedControl,
    MotionNotAllowed,
    ReservedRelay,
    MissingPin,
    DifferentBoard,
    PinChanged,
    Cancelled,
    DeadlineExceeded,
    Rejected,
    Unproven,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Error {
    pub code: Code,
    pub key: Option<String>,
}

impl Error {
    fn new(code: Code) -> Self {
        Self { code, key: None }
    }
    fn at(code: Code, key: &str) -> Self {
        Self {
            code,
            key: Some(key.into()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Scene apply {:?}", self.code)?;
        if let Some(key) = &self.key {
            write!(f, " ({key})")?;
        }
        Ok(())
    }
}
impl std::error::Error for Error {}

/// PWM configuration lives outside the board-profile revision. Pass the
/// authoritative catalog value explicitly; do not substitute a status timestamp.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TargetPin {
    pub host_instance_id: String,
    pub board_identity: String,
    pub connection_generation: u64,
    pub revision: String,
    pub pwm_configuration_revision: Option<String>,
}

impl TargetPin {
    fn capture(
        cap: &HardwareCapabilities,
        pwm_revision: Option<&str>,
        needs_pwm: bool,
    ) -> Result<Self, Error> {
        let profile = cap
            .board_profile
            .as_ref()
            .ok_or_else(|| Error::new(Code::MissingPin))?;
        let known = |s: &str| !s.trim().is_empty() && s.len() <= 512;
        if !cap.board_connected
            || !known(&cap.host_instance_id)
            || !known(&profile.board_identity)
            || cap.connection_generation == 0
            || !known(&profile.revision)
            || (needs_pwm && !pwm_revision.is_some_and(known))
        {
            return Err(Error::new(Code::MissingPin));
        }
        Ok(Self {
            host_instance_id: cap.host_instance_id.clone(),
            board_identity: profile.board_identity.clone(),
            connection_generation: cap.connection_generation,
            revision: profile.revision.clone(),
            pwm_configuration_revision: needs_pwm.then(|| pwm_revision.unwrap().to_owned()),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutputValue {
    Relay { value: u8 },
    Pwm { percent_milli: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Output {
    pub target: String,
    pub requested: OutputValue,
}

/// Only `preflight` constructs a plan. Loading/staging a scene is not a plan.
#[derive(Debug)]
pub struct Plan {
    pin: TargetPin,
    outputs: Vec<Output>,
}
impl Plan {
    pub fn pin(&self) -> &TargetPin {
        &self.pin
    }
    pub fn outputs(&self) -> &[Output] {
        &self.outputs
    }
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 128
        && key
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
}

fn resolve_output(
    key: &str,
    value: &ChannelValue,
    cap: &HardwareCapabilities,
    managed: &[HardwareControl],
) -> Result<Output, Error> {
    // managed_controls preserves advertised locks/semantics before physical
    // output fallbacks. A duplicate authoritative definition is not permission.
    if cap.controls.iter().filter(|c| c.key == key).count() > 1 {
        return Err(Error::at(Code::AmbiguousControl, key));
    }
    let control = managed
        .iter()
        .find(|c| c.key == key)
        .ok_or_else(|| Error::at(Code::UnknownControl, key))?;
    if control.locked {
        return Err(Error::at(Code::Locked, key));
    }
    if matches!(control.kind.as_str(), "motion" | "seat")
        || matches!(value, ChannelValue::Motion { .. })
    {
        return Err(Error::at(Code::MotionNotAllowed, key));
    }
    let relays: Vec<_> = cap.relays.iter().filter(|c| c.key == key).collect();
    let pwm: Vec<_> = cap.pwm_channels.iter().filter(|c| c.key == key).collect();
    if relays.len() + pwm.len() > 1 {
        return Err(Error::at(Code::AmbiguousControl, key));
    }
    let requested = match value {
        ChannelValue::Boolean { on } => {
            let output = relays
                .first()
                .ok_or_else(|| Error::at(Code::UnsupportedControl, key))?;
            if control.kind != "relay"
                || output.control != "relay-user"
                || control.control != "relay-user"
            {
                return Err(Error::at(Code::ReservedRelay, key));
            }
            OutputValue::Relay {
                value: u8::from(*on),
            }
        }
        ChannelValue::Pwm { percent_milli } => {
            if pwm.is_empty()
                || !matches!(control.kind.as_str(), "pwm" | "mosfet")
                || control.control == "unavailable"
                || pwm[0].control == "unavailable"
                || *percent_milli > 100_000
            {
                return Err(Error::at(Code::UnsupportedControl, key));
            }
            OutputValue::Pwm {
                percent_milli: *percent_milli,
            }
        }
        ChannelValue::Motion { .. } => return Err(Error::at(Code::MotionNotAllowed, key)),
    };
    Ok(Output {
        target: key.into(),
        requested,
    })
}

/// All selected outputs are validated before a dispatch can be obtained. The
/// selection is explicit, nonempty, duplicate-free and retains its given order.
/// Unselected motion/unsupported values remain data, never implicit commands.
pub fn preflight(
    scene: &Scene,
    selected: &[String],
    cap: &HardwareCapabilities,
    pwm_revision: Option<&str>,
) -> Result<Plan, Error> {
    scene
        .validate()
        .map_err(|_| Error::new(Code::InvalidScene))?;
    if selected.is_empty() {
        return Err(Error::new(Code::EmptySelection));
    }
    if selected.len() > MAX_OUTPUTS {
        return Err(Error::new(Code::TooManyOutputs));
    }
    let mut seen = BTreeSet::new();
    let mut outputs = Vec::with_capacity(selected.len());
    let managed = crate::ui::hardware_control::managed_controls(cap);
    for key in selected {
        if !valid_key(key) {
            return Err(Error::new(Code::InvalidKey));
        }
        if !seen.insert(key) {
            return Err(Error::at(Code::DuplicateSelection, key));
        }
        if scene.unsampled.contains(key) {
            return Err(Error::at(Code::Unsampled, key));
        }
        let value = scene
            .channels
            .get(key)
            .ok_or_else(|| Error::at(Code::MissingValue, key))?;
        outputs.push(resolve_output(key, value, cap, &managed)?);
    }
    let needs_pwm = outputs
        .iter()
        .any(|o| matches!(o.requested, OutputValue::Pwm { .. }));
    let pin = TargetPin::capture(cap, pwm_revision, needs_pwm)?;
    if scene.board_identity.is_empty() || scene.board_identity != pin.board_identity {
        return Err(Error::new(Code::DifferentBoard));
    }
    Ok(Plan { pin, outputs })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Ticket {
    pub operation_id: Uuid,
    pub index: usize,
}

#[derive(Clone, Debug)]
pub struct Request {
    pub ticket: Ticket,
    pub method: &'static str,
    pub params: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputStatus {
    Pending,
    InFlight,
    Acknowledged,
    Failed,
    Unknown,
    Skipped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Running,
    Completed,
    Failed,
    Unknown,
    Cancelled,
    TimedOut,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OutputReport {
    pub output: Output,
    pub status: OutputStatus,
    pub error: Option<Error>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub id: Uuid,
    pub pin: TargetPin,
    pub status: Status,
    pub cancel_requested: bool,
    pub error: Option<Error>,
    pub outputs: Vec<OutputReport>,
}

/// A domain proof, not an optimistic UI value. The transport adapter must only
/// construct this from the coordinator's typed confirmed response. Operation
/// still checks every echo and exact raw readback before accepting it.
#[derive(Clone, Debug)]
pub struct Confirmation {
    pub pin: TargetPin,
    pub target: String,
    pub acknowledged: bool,
    pub value: u16,
    pub observed_value: u16,
    pub percent_milli: Option<u32>,
}

#[derive(Clone, Debug)]
pub enum Completion {
    Confirmed(Confirmation),
    /// Only a typed pre-admission RPC rejection proves no write was attempted.
    Rejected,
    /// Transport failure, incomplete readback or any post-admission ambiguity.
    Unknown,
}

impl Completion {
    /// The RPC transport must first correlate its JSON-RPC id. This consumes
    /// only that call's result, never an unsolicited event or optimistic state.
    pub fn from_success(value: &Value) -> Self {
        #[derive(Deserialize)]
        struct Reply {
            accepted: bool,
            outcome: String,
            target: String,
            value: u16,
            observed_value: u16,
            host_instance_id: String,
            board_identity: String,
            connection_generation: u64,
            revision: String,
            pwm_configuration_revision: Option<String>,
            percent_milli: Option<u32>,
        }
        let Ok(reply) = serde_json::from_value::<Reply>(value.clone()) else {
            return Self::Unknown;
        };
        if !reply.accepted || reply.outcome != "confirmed" {
            return Self::Unknown;
        }
        Self::Confirmed(Confirmation {
            pin: TargetPin {
                host_instance_id: reply.host_instance_id,
                board_identity: reply.board_identity,
                connection_generation: reply.connection_generation,
                revision: reply.revision,
                pwm_configuration_revision: reply.pwm_configuration_revision,
            },
            target: reply.target,
            acknowledged: true,
            value: reply.value,
            observed_value: reply.observed_value,
            percent_milli: reply.percent_milli,
        })
    }
    /// Pass the correlated JSON-RPC error's `data`, not its human message.
    /// Only the endpoint's typed pre-runtime rejection proves a no-write result.
    pub fn from_error_data(data: Option<&Value>) -> Self {
        if data.and_then(|v| v.get("outcome")).and_then(Value::as_str) == Some("rejected") {
            Self::Rejected
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug)]
pub struct Operation {
    id: Uuid,
    plan: Plan,
    started: Instant,
    status: Status,
    cancel_requested: bool,
    error: Option<Error>,
    outputs: Vec<OutputReport>,
    in_flight: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::four_d::controller::{HardwareBoardProfile, HardwareControl, HardwareOutput};
    use crate::projects::MotionDirection;

    fn fixture() -> (Scene, HardwareCapabilities, Vec<String>) {
        let cap = HardwareCapabilities {
            board_connected: true,
            host_instance_id: "host-a".into(),
            connection_generation: 7,
            board_profile: Some(HardwareBoardProfile {
                board_identity: "board-a".into(),
                revision: "profile-a".into(),
                ..Default::default()
            }),
            relays: vec![HardwareOutput {
                id: 5,
                key: "relay.5".into(),
                name: "Door".into(),
                role: "user-output".into(),
                control: "relay-user".into(),
            }],
            pwm_channels: vec![HardwareOutput {
                id: 0,
                key: "pwm.0".into(),
                name: "Lamp".into(),
                role: "user-output".into(),
                control: "pwm-user".into(),
            }],
            ..Default::default()
        };
        let scene = Scene {
            name: "Scene".into(),
            board_identity: "board-a".into(),
            channels: [
                ("relay.5".into(), ChannelValue::Boolean { on: true }),
                (
                    "pwm.0".into(),
                    ChannelValue::Pwm {
                        percent_milli: 31_234,
                    },
                ),
            ]
            .into(),
            unsampled: vec![],
        };
        (scene, cap, vec!["relay.5".into(), "pwm.0".into()])
    }
    fn operation(now: Instant) -> (Operation, HardwareCapabilities) {
        let (scene, cap, selected) = fixture();
        (
            Operation::new(
                preflight(&scene, &selected, &cap, Some("curve-a")).unwrap(),
                now,
            ),
            cap,
        )
    }
    fn wire_result(request: &Request) -> Value {
        let p = &request.params;
        let raw = p.get("value").cloned().unwrap_or(json!(1279));
        let mut result = json!({"accepted": true, "outcome": "confirmed", "target": p["target"],
            "host_instance_id": p["expected_host_instance_id"], "board_identity": p["board_identity"],
            "connection_generation": p["expected_connection_generation"], "revision": p["expected_revision"],
            "value": raw, "observed_value": raw});
        if let Some(percent) = p.get("percent_milli") {
            result["percent_milli"] = percent.clone();
            result["pwm_configuration_revision"] = p["expected_pwm_configuration_revision"].clone();
        }
        result
    }
    fn proof(request: &Request) -> Completion {
        Completion::from_success(&wire_result(request))
    }
    fn statuses(op: &Operation) -> Vec<OutputStatus> {
        op.report().outputs.iter().map(|o| o.status).collect()
    }

    #[test]
    fn selection_is_explicit_bounded_unique_and_valid_before_a_plan_exists() {
        let (scene, cap, selected) = fixture();
        assert_eq!(
            preflight(&scene, &[], &cap, None).unwrap_err().code,
            Code::EmptySelection
        );
        assert_eq!(
            preflight(&scene, &vec!["relay.5".into(); 65], &cap, None)
                .unwrap_err()
                .code,
            Code::TooManyOutputs
        );
        assert_eq!(
            preflight(&scene, &vec!["relay.5".into(); 2], &cap, None)
                .unwrap_err()
                .code,
            Code::DuplicateSelection
        );
        assert_eq!(
            preflight(&scene, &[" relay.5".into()], &cap, None)
                .unwrap_err()
                .code,
            Code::InvalidKey
        );
        assert_eq!(
            preflight(&scene, &["missing".into()], &cap, None)
                .unwrap_err()
                .code,
            Code::MissingValue
        );
        let mut missing = scene.clone();
        missing.unsampled.push("unknown".into());
        assert_eq!(
            preflight(&missing, &["unknown".into()], &cap, None)
                .unwrap_err()
                .code,
            Code::Unsampled
        );
        assert_eq!(
            preflight(&scene, &selected, &cap, None).unwrap_err().code,
            Code::MissingPin
        );
        let mut invalid = scene.clone();
        invalid.channels.insert(
            "pwm.0".into(),
            ChannelValue::Pwm {
                percent_milli: 100_001,
            },
        );
        assert_eq!(
            preflight(&invalid, &selected, &cap, Some("curve-a"))
                .unwrap_err()
                .code,
            Code::InvalidScene
        );
    }

    #[test]
    fn exact_limit_is_accepted_without_assuming_reserved_relay_numbers() {
        let (mut scene, mut cap, _) = fixture();
        cap.relays.clear();
        scene.channels.clear();
        let selected: Vec<_> = (1..=64)
            .map(|id| {
                let key = format!("relay.{id}");
                cap.relays.push(HardwareOutput {
                    id,
                    key: key.clone(),
                    name: String::new(),
                    role: "user-output".into(),
                    control: "relay-user".into(),
                });
                scene
                    .channels
                    .insert(key.clone(), ChannelValue::Boolean { on: false });
                key
            })
            .collect();
        assert_eq!(
            preflight(&scene, &selected, &cap, None)
                .unwrap()
                .outputs()
                .len(),
            MAX_OUTPUTS
        );
    }

    #[test]
    fn advertised_locks_semantics_and_physical_types_override_fallbacks() {
        let (mut scene, mut cap, selected) = fixture();
        cap.controls.push(HardwareControl {
            key: "relay.5".into(),
            kind: "relay".into(),
            control: "relay-user".into(),
            locked: true,
            ..Default::default()
        });
        assert_eq!(
            preflight(&scene, &selected, &cap, Some("curve-a"))
                .unwrap_err()
                .code,
            Code::Locked
        );
        cap.controls[0].locked = false;
        cap.controls[0].kind = "motion".into();
        assert_eq!(
            preflight(&scene, &selected, &cap, Some("curve-a"))
                .unwrap_err()
                .code,
            Code::MotionNotAllowed
        );
        cap.controls.clear();
        cap.relays[0].control = "unavailable".into();
        assert_eq!(
            preflight(&scene, &selected, &cap, Some("curve-a"))
                .unwrap_err()
                .code,
            Code::ReservedRelay
        );
        cap.relays[0].control = "relay-user".into();
        cap.relays.push(cap.relays[0].clone());
        assert_eq!(
            preflight(&scene, &selected, &cap, Some("curve-a"))
                .unwrap_err()
                .code,
            Code::AmbiguousControl
        );
        cap.relays.pop();
        cap.relays.clear();
        assert_eq!(
            preflight(&scene, &selected, &cap, Some("curve-a"))
                .unwrap_err()
                .code,
            Code::UnknownControl
        );
        cap = fixture().1;
        scene.channels.insert(
            "relay.5".into(),
            ChannelValue::Pwm {
                percent_milli: 1000,
            },
        );
        assert_eq!(
            preflight(&scene, &selected, &cap, Some("curve-a"))
                .unwrap_err()
                .code,
            Code::UnsupportedControl
        );
    }

    #[test]
    fn unselected_motion_is_retained_as_data_and_not_implicitly_applied() {
        let (mut scene, cap, selected) = fixture();
        scene.channels.insert(
            "seat.a".into(),
            ChannelValue::Motion {
                direction: MotionDirection::Up,
            },
        );
        let plan = preflight(&scene, &selected, &cap, Some("curve-a")).unwrap();
        assert_eq!(plan.outputs().len(), 2);
        assert_eq!(plan.outputs()[0].target, "relay.5");
        assert!(scene.channels.contains_key("seat.a"));
    }

    #[test]
    fn all_pin_fields_must_be_known_and_scene_identity_must_match() {
        let (scene, cap, selected) = fixture();
        for field in 0..6 {
            let mut bad = cap.clone();
            let mut curve = Some("curve-a");
            match field {
                0 => bad.board_connected = false,
                1 => bad.host_instance_id.clear(),
                2 => bad.board_profile.as_mut().unwrap().board_identity.clear(),
                3 => bad.connection_generation = 0,
                4 => bad.board_profile.as_mut().unwrap().revision.clear(),
                _ => curve = Some(""),
            }
            assert_eq!(
                preflight(&scene, &selected, &bad, curve).unwrap_err().code,
                Code::MissingPin
            );
        }
        let mut other = scene.clone();
        other.board_identity = "board-b".into();
        assert_eq!(
            preflight(&other, &selected, &cap, Some("curve-a"))
                .unwrap_err()
                .code,
            Code::DifferentBoard
        );
    }

    #[test]
    fn requests_are_one_at_a_time_and_do_not_use_slider_expiry_or_raw_pwm_guessing() {
        let start = Instant::now();
        let (mut op, cap) = operation(start);
        let first = op.next_request(start, &cap, Some("curve-a")).unwrap();
        assert_eq!(first.method, METHOD);
        assert_eq!(first.params["value"], 1);
        assert!(first.params.get("percent_milli").is_none());
        assert!(
            op.next_request(start + Duration::from_secs(2), &cap, Some("curve-a"))
                .is_none()
        );
        assert!(op.complete(first.ticket, proof(&first), start + Duration::from_secs(2)));
        let second = op
            .next_request(start + Duration::from_secs(3), &cap, Some("curve-a"))
            .unwrap();
        assert_eq!(second.params["percent_milli"], 31_234);
        assert!(second.params.get("value").is_none());
        assert_eq!(
            second.params["expected_pwm_configuration_revision"],
            "curve-a"
        );
        assert!(op.complete(
            second.ticket,
            proof(&second),
            start + Duration::from_secs(4)
        ));
        assert_eq!(op.report().status, Status::Completed);
        assert_eq!(statuses(&op), vec![OutputStatus::Acknowledged; 2]);
        assert!(
            op.next_request(start + Duration::from_secs(5), &cap, Some("curve-a"))
                .is_none()
        );
    }

    #[test]
    fn foreign_duplicate_and_late_callbacks_never_advance_a_sibling() {
        let start = Instant::now();
        let (mut op, cap) = operation(start);
        let first = op.next_request(start, &cap, Some("curve-a")).unwrap();
        let foreign = Ticket {
            operation_id: Uuid::new_v4(),
            ..first.ticket
        };
        assert!(!op.complete(foreign, proof(&first), start));
        assert!(!op.complete(
            Ticket {
                index: 1,
                ..first.ticket
            },
            proof(&first),
            start
        ));
        assert_eq!(
            statuses(&op),
            vec![OutputStatus::InFlight, OutputStatus::Pending]
        );
        assert!(op.complete(first.ticket, proof(&first), start));
        let second = op.next_request(start, &cap, Some("curve-a")).unwrap();
        assert!(!op.complete(first.ticket, proof(&first), start));
        assert_eq!(
            statuses(&op),
            vec![OutputStatus::Acknowledged, OutputStatus::InFlight]
        );
        assert!(op.complete(second.ticket, proof(&second), start));
        assert!(!op.complete(second.ticket, Completion::Unknown, start));
    }

    #[test]
    fn cancellation_finishes_only_the_current_call_and_skips_every_remaining_output() {
        let start = Instant::now();
        let (mut op, cap) = operation(start);
        let first = op.next_request(start, &cap, Some("curve-a")).unwrap();
        op.cancel(start + Duration::from_millis(50));
        assert!(op.has_in_flight());
        assert!(!op.is_finished());
        assert!(op.next_request(start, &cap, Some("curve-a")).is_none());
        assert!(op.complete(first.ticket, proof(&first), start + Duration::from_secs(1)));
        assert_eq!(op.report().status, Status::Cancelled);
        assert_eq!(
            statuses(&op),
            vec![OutputStatus::Acknowledged, OutputStatus::Skipped]
        );
        let (mut before, cap) = operation(start);
        before.cancel(start);
        assert!(before.next_request(start, &cap, Some("curve-a")).is_none());
        assert_eq!(statuses(&before), vec![OutputStatus::Skipped; 2]);
    }

    #[test]
    fn failures_preserve_prior_acknowledgements_and_never_retry_or_rollback() {
        for completion in [Completion::Rejected, Completion::Unknown] {
            let start = Instant::now();
            let (mut op, cap) = operation(start);
            let first = op.next_request(start, &cap, Some("curve-a")).unwrap();
            op.complete(first.ticket, proof(&first), start);
            let second = op.next_request(start, &cap, Some("curve-a")).unwrap();
            let rejected = matches!(completion, Completion::Rejected);
            op.complete(second.ticket, completion, start);
            assert_eq!(
                statuses(&op),
                vec![
                    OutputStatus::Acknowledged,
                    if rejected {
                        OutputStatus::Failed
                    } else {
                        OutputStatus::Unknown
                    }
                ]
            );
            assert!(op.next_request(start, &cap, Some("curve-a")).is_none());
            assert!(!op.complete(second.ticket, proof(&second), start));
        }
        let start = Instant::now();
        let (mut op, cap) = operation(start);
        let first = op.next_request(start, &cap, Some("curve-a")).unwrap();
        op.complete(first.ticket, Completion::Unknown, start);
        assert_eq!(
            statuses(&op),
            vec![OutputStatus::Unknown, OutputStatus::Skipped]
        );
    }

    #[test]
    fn thirty_second_deadline_marks_in_flight_unknown_not_failed_or_safe_to_retry() {
        let start = Instant::now();
        let (mut op, cap) = operation(start);
        let first = op.next_request(start, &cap, Some("curve-a")).unwrap();
        assert!(!op.complete(first.ticket, proof(&first), start + DEADLINE));
        assert_eq!(op.report().status, Status::TimedOut);
        assert_eq!(
            statuses(&op),
            vec![OutputStatus::Unknown, OutputStatus::Skipped]
        );
        assert!(
            op.next_request(start + DEADLINE, &cap, Some("curve-a"))
                .is_none()
        );
        let (mut idle, cap) = operation(start);
        assert!(
            idle.next_request(start + DEADLINE, &cap, Some("curve-a"))
                .is_none()
        );
        assert_eq!(statuses(&idle), vec![OutputStatus::Skipped; 2]);
    }

    #[test]
    fn every_pin_change_stops_pending_work_and_an_in_flight_change_is_unknown() {
        let start = Instant::now();
        for field in 0..6 {
            let (mut op, mut cap) = operation(start);
            let first = op.next_request(start, &cap, Some("curve-a")).unwrap();
            let mut curve = Some("curve-a");
            match field {
                0 => cap.board_connected = false,
                1 => cap.host_instance_id = "host-b".into(),
                2 => cap.board_profile.as_mut().unwrap().board_identity = "board-b".into(),
                3 => cap.connection_generation += 1,
                4 => cap.board_profile.as_mut().unwrap().revision = "profile-b".into(),
                _ => curve = Some("curve-b"),
            }
            op.tick(start, &cap, curve);
            assert_eq!(
                statuses(&op),
                vec![OutputStatus::Unknown, OutputStatus::Skipped]
            );
            assert!(!op.complete(first.ticket, proof(&first), start));
            assert!(op.next_request(start, &cap, curve).is_none());
        }
        let (mut op, mut cap) = operation(start);
        cap.connection_generation += 1;
        assert!(op.next_request(start, &cap, Some("curve-a")).is_none());
        assert_eq!(statuses(&op), vec![OutputStatus::Skipped; 2]);
    }

    #[test]
    fn a_lock_added_after_preflight_rejects_before_dispatch() {
        let start = Instant::now();
        let (mut op, mut cap) = operation(start);
        cap.controls.push(HardwareControl {
            key: "relay.5".into(),
            kind: "relay".into(),
            control: "relay-user".into(),
            locked: true,
            ..Default::default()
        });
        assert!(op.next_request(start, &cap, Some("curve-a")).is_none());
        assert_eq!(
            statuses(&op),
            vec![OutputStatus::Failed, OutputStatus::Skipped]
        );
        // A later selected member becoming invalid must prevent the first
        // dispatch too, not merely reject after an earlier relay has changed.
        let (mut op, mut cap) = operation(start);
        cap.controls.push(HardwareControl {
            key: "pwm.0".into(),
            kind: "pwm".into(),
            control: "pwm-user".into(),
            locked: true,
            ..Default::default()
        });
        assert!(op.next_request(start, &cap, Some("curve-a")).is_none());
        assert_eq!(
            statuses(&op),
            vec![OutputStatus::Skipped, OutputStatus::Failed]
        );
    }

    #[test]
    fn confirmation_requires_matching_owner_session_target_ack_and_readback() {
        let start = Instant::now();
        for field in 0..9 {
            let (mut op, cap) = operation(start);
            let request = op.next_request(start, &cap, Some("curve-a")).unwrap();
            let Completion::Confirmed(mut proof) = proof(&request) else {
                panic!("fixture proof");
            };
            match field {
                0 => proof.acknowledged = false,
                1 => proof.target = "relay.6".into(),
                2 => proof.pin.host_instance_id = "other".into(),
                3 => proof.pin.board_identity = "other".into(),
                4 => proof.pin.connection_generation += 1,
                5 => proof.pin.revision = "other".into(),
                6 => proof.observed_value = 0,
                7 => {
                    proof.value = 0;
                    proof.observed_value = 0;
                }
                _ => proof.percent_milli = Some(0),
            }
            op.complete(request.ticket, Completion::Confirmed(proof), start);
            assert_eq!(
                statuses(&op),
                vec![OutputStatus::Unknown, OutputStatus::Skipped]
            );
        }
        for field in 0..3 {
            let (mut op, cap) = operation(start);
            let first = op.next_request(start, &cap, Some("curve-a")).unwrap();
            op.complete(first.ticket, proof(&first), start);
            let second = op.next_request(start, &cap, Some("curve-a")).unwrap();
            let Completion::Confirmed(mut proof) = proof(&second) else {
                panic!("fixture proof");
            };
            match field {
                0 => proof.percent_milli = None,
                1 => proof.pin.pwm_configuration_revision = None,
                _ => proof.pin.pwm_configuration_revision = Some("other".into()),
            }
            op.complete(second.ticket, Completion::Confirmed(proof), start);
            assert_eq!(
                statuses(&op),
                vec![OutputStatus::Acknowledged, OutputStatus::Unknown]
            );
        }
    }

    #[test]
    fn rpc_proof_is_typed_and_unproven_or_post_runtime_errors_remain_unknown() {
        for value in [
            json!({}),
            json!({"accepted": true}),
            json!({"accepted": true, "outcome": "confirmed"}),
            json!({"accepted": true, "outcome": "unknown"}),
        ] {
            assert!(matches!(
                Completion::from_success(&value),
                Completion::Unknown
            ));
        }
        assert!(matches!(
            Completion::from_error_data(None),
            Completion::Unknown
        ));
        assert!(matches!(
            Completion::from_error_data(Some(&json!({"outcome": "unknown"}))),
            Completion::Unknown
        ));
        assert!(matches!(
            Completion::from_error_data(Some(&json!({"message": "rejected"}))),
            Completion::Unknown
        ));
        assert!(matches!(
            Completion::from_error_data(Some(&json!({"outcome": "rejected"}))),
            Completion::Rejected
        ));
        let start = Instant::now();
        let (mut op, cap) = operation(start);
        let request = op.next_request(start, &cap, Some("curve-a")).unwrap();
        for (field, wrong) in [
            ("accepted", json!(false)),
            ("outcome", json!("unknown")),
            ("value", json!("1")),
            ("observed_value", json!(1.5)),
            ("host_instance_id", Value::Null),
            ("connection_generation", json!("7")),
        ] {
            let mut result = wire_result(&request);
            result[field] = wrong;
            assert!(matches!(
                Completion::from_success(&result),
                Completion::Unknown
            ));
        }
    }
}

impl Operation {
    pub fn new(plan: Plan, now: Instant) -> Self {
        let outputs = plan
            .outputs
            .iter()
            .cloned()
            .map(|output| OutputReport {
                output,
                status: OutputStatus::Pending,
                error: None,
            })
            .collect();
        Self {
            id: Uuid::new_v4(),
            plan,
            started: now,
            status: Status::Running,
            cancel_requested: false,
            error: None,
            outputs,
            in_flight: None,
        }
    }
    pub fn id(&self) -> Uuid {
        self.id
    }
    /// Pass this absolute deadline to the tracked engine call too, so a request
    /// delayed in its queue cannot enter the runtime after this operation ends.
    pub fn deadline(&self) -> Instant {
        self.started + DEADLINE
    }
    pub fn is_finished(&self) -> bool {
        self.status != Status::Running
    }
    pub fn has_in_flight(&self) -> bool {
        self.in_flight.is_some()
    }
    pub fn report(&self) -> Report {
        Report {
            id: self.id,
            pin: self.plan.pin.clone(),
            status: self.status,
            cancel_requested: self.cancel_requested,
            error: self.error.clone(),
            outputs: self.outputs.clone(),
        }
    }
    fn stop(&mut self, status: Status, error: Error) {
        let uncertain = self.in_flight.is_some();
        if let Some(index) = self.in_flight.take() {
            self.outputs[index].status = OutputStatus::Unknown;
            self.outputs[index].error = Some(error.clone());
        }
        for output in &mut self.outputs {
            if output.status == OutputStatus::Pending {
                output.status = OutputStatus::Skipped;
                output.error = Some(error.clone());
            }
        }
        self.status = if uncertain && status == Status::Failed {
            Status::Unknown
        } else {
            status
        };
        self.error = Some(error);
    }
    fn expire(&mut self, now: Instant) {
        if !self.is_finished() && now.saturating_duration_since(self.started) >= DEADLINE {
            self.stop(Status::TimedOut, Error::new(Code::DeadlineExceeded));
        }
    }
    /// Call while awaiting a response too. Nothing is retried or rolled back.
    pub fn tick(&mut self, now: Instant, cap: &HardwareCapabilities, pwm_revision: Option<&str>) {
        self.expire(now);
        if self.is_finished() {
            return;
        }
        let pin = TargetPin::capture(
            cap,
            pwm_revision,
            self.plan.pin.pwm_configuration_revision.is_some(),
        );
        if pin.as_ref() != Ok(&self.plan.pin) {
            self.stop(Status::Failed, Error::new(Code::PinChanged));
        }
    }
    /// Returning a request consumes its one dispatch opportunity. The caller
    /// must not clone/requeue/retry it. Bind the RPC callback to its ticket.
    pub fn next_request(
        &mut self,
        now: Instant,
        cap: &HardwareCapabilities,
        pwm_revision: Option<&str>,
    ) -> Option<Request> {
        self.tick(now, cap, pwm_revision);
        if self.is_finished() || self.in_flight.is_some() {
            return None;
        }
        if self.cancel_requested {
            self.stop(Status::Cancelled, Error::new(Code::Cancelled));
            return None;
        }
        let Some(index) = self
            .outputs
            .iter()
            .position(|o| o.status == OutputStatus::Pending)
        else {
            self.status = Status::Completed;
            return None;
        };
        let managed = crate::ui::hardware_control::managed_controls(cap);
        for pending in 0..self.outputs.len() {
            if self.outputs[pending].status != OutputStatus::Pending {
                continue;
            }
            let output = &self.plan.outputs[pending];
            let value = match output.requested {
                OutputValue::Relay { value } => ChannelValue::Boolean { on: value != 0 },
                OutputValue::Pwm { percent_milli } => ChannelValue::Pwm { percent_milli },
            };
            if let Err(error) = resolve_output(&output.target, &value, cap, &managed) {
                self.outputs[pending].status = OutputStatus::Failed;
                self.outputs[pending].error = Some(error.clone());
                self.stop(Status::Failed, error);
                return None;
            }
        }
        let output = &self.plan.outputs[index];
        let pin = &self.plan.pin;
        let mut params = json!({ "target": output.target,
            "expected_host_instance_id": pin.host_instance_id, "board_identity": pin.board_identity,
            "expected_connection_generation": pin.connection_generation, "expected_revision": pin.revision });
        match output.requested {
            OutputValue::Relay { value } => params["value"] = json!(value),
            OutputValue::Pwm { percent_milli } => {
                params["percent_milli"] = json!(percent_milli);
                params["expected_pwm_configuration_revision"] =
                    json!(pin.pwm_configuration_revision);
            }
        }
        self.outputs[index].status = OutputStatus::InFlight;
        self.in_flight = Some(index);
        Some(Request {
            ticket: Ticket {
                operation_id: self.id,
                index,
            },
            method: METHOD,
            params,
        })
    }
    pub fn cancel(&mut self, now: Instant) {
        self.expire(now);
        if self.is_finished() {
            return;
        }
        self.cancel_requested = true;
        if self.in_flight.is_none() {
            self.stop(Status::Cancelled, Error::new(Code::Cancelled));
        }
    }
    /// False means a late, duplicate or foreign callback: it cannot affect the
    /// current item, finish an operation or unlock another dispatch opportunity.
    pub fn complete(&mut self, ticket: Ticket, completion: Completion, now: Instant) -> bool {
        if self.is_finished()
            || ticket.operation_id != self.id
            || self.in_flight != Some(ticket.index)
        {
            return false;
        }
        self.expire(now);
        if self.is_finished() {
            return false;
        }
        let output = &self.plan.outputs[ticket.index];
        let valid = match &completion {
            Completion::Confirmed(proof) => {
                let expected = &self.plan.pin;
                proof.acknowledged
                    && proof.target == output.target
                    && proof.value == proof.observed_value
                    && proof.pin.host_instance_id == expected.host_instance_id
                    && proof.pin.board_identity == expected.board_identity
                    && proof.pin.connection_generation == expected.connection_generation
                    && proof.pin.revision == expected.revision
                    && match output.requested {
                        OutputValue::Relay { value } => {
                            proof.value == u16::from(value) && proof.percent_milli.is_none()
                        }
                        OutputValue::Pwm { percent_milli } => {
                            proof.percent_milli == Some(percent_milli)
                                && proof.pin.pwm_configuration_revision
                                    == expected.pwm_configuration_revision
                        }
                    }
            }
            _ => false,
        };
        self.in_flight = None;
        if valid {
            self.outputs[ticket.index].status = OutputStatus::Acknowledged;
            if self
                .outputs
                .iter()
                .all(|o| o.status == OutputStatus::Acknowledged)
            {
                self.status = Status::Completed;
            } else if self.cancel_requested {
                self.stop(Status::Cancelled, Error::new(Code::Cancelled));
            }
        } else {
            let rejected = matches!(completion, Completion::Rejected);
            let error = Error::at(
                if rejected {
                    Code::Rejected
                } else {
                    Code::Unproven
                },
                &output.target,
            );
            self.outputs[ticket.index].status = if rejected {
                OutputStatus::Failed
            } else {
                OutputStatus::Unknown
            };
            self.outputs[ticket.index].error = Some(error.clone());
            self.stop(
                if rejected {
                    Status::Failed
                } else {
                    Status::Unknown
                },
                error,
            );
        }
        true
    }
}
