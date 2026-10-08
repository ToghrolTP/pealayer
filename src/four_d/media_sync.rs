//! Playback telemetry uses its own bounded RPC stream, never the actuator queue.
use super::controller::ControllerClient;
use serde_json::json;
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct PlaybackSample {
    pub name: String,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub playing: bool,
    pub buffering: bool,
    pub loaded: bool,
    pub rate: f64,
    pub sampled_at: Instant,
    pub observed_at: Instant,
    pub epoch: u64,
}
impl Default for PlaybackSample {
    fn default() -> Self {
        Self {
            name: String::new(),
            position_ms: 0,
            duration_ms: None,
            playing: false,
            buffering: false,
            loaded: false,
            rate: 1.0,
            sampled_at: Instant::now(),
            observed_at: Instant::now(),
            epoch: 1,
        }
    }
}
impl PlaybackSample {
    pub(crate) fn position_now(&self) -> u64 {
        let advance = if self.playing {
            self.sampled_at.elapsed().as_secs_f64() * 1000.0 * self.rate
        } else {
            0.0
        };
        let position = self.position_ms.saturating_add(advance.max(0.0) as u64);
        position
            .min(self.duration_ms.unwrap_or(u32::MAX as u64))
            .min(u32::MAX as u64)
    }
    fn changed_from(&self, previous: &Self) -> bool {
        self.epoch != previous.epoch
            || self.loaded != previous.loaded
            || self.playing != previous.playing
            || self.duration_ms != previous.duration_ms
            || self.rate != previous.rate
            || self.position_now().abs_diff(previous.position_now()) > 80
    }
}

fn playback_publish_interval(sample: &PlaybackSample) -> Duration {
    if sample.playing {
        Duration::from_millis(40)
    } else {
        Duration::from_secs(1)
    }
}

fn handoff_pause_acknowledged(sample: &PlaybackSample, valid_echo: bool) -> bool {
    valid_echo && !sample.playing && !sample.buffering
        && sample.observed_at.elapsed() < Duration::from_millis(250)
}

#[derive(Default)]
struct AutomaticClaim {
    attempted: bool,
    owner: Option<String>,
}
impl AutomaticClaim {
    fn should_request(&mut self, loaded: bool, status: &super::authority::Status) -> bool {
        if self.owner.as_ref().is_some_and(|owner| !owner.is_empty()) && status.owner_id.is_empty() {
            self.attempted = false;
        }
        self.owner = Some(status.owner_id.clone());
        if !loaded || self.attempted { return false; }
        self.attempted = true;
        // Never take over an existing owner automatically, even after reconnect.
        status.owner_id.is_empty()
    }
}

fn invalidate_coordinator_session(plan: &mut super::media_timeline::PreparedTimeline) {
    if plan.has_items() { plan.revision = plan.revision.saturating_add(1); }
    plan.acknowledged_revision = 0;
    plan.clock_ack_revision = 0;
    plan.clock_ack_epoch = 0;
    plan.last_ack = None;
    plan.feedback = serde_json::Value::Null;
    plan.play_requested = false;
}

// Wake idle UI/Web consumers for semantic transitions, not for every clock echo
// or changing ACK age. The executor/observer never waits for a repaint.
#[derive(PartialEq)]
struct SyncPresentation {
    authority: Option<super::authority::Status>,
    revision: u64,
    prepared_revision: u64,
    clock_revision: u64,
    clock_epoch: u64,
    error: Option<String>,
    deferred: Option<String>,
    requires_reprepare: bool,
    executor_state: Option<String>,
    acknowledged: Option<u64>,
}

fn notify_sync_change(
    timeline: &Mutex<super::media_timeline::PreparedTimeline>,
    notifier: &Mutex<Option<super::engine::StateNotifier>>,
    previous: &mut Option<SyncPresentation>,
) {
    let presentation = timeline.lock().ok().map(|plan| SyncPresentation {
        authority: plan.authority.clone(),
        revision: plan.revision,
        prepared_revision: plan.acknowledged_revision,
        clock_revision: plan.clock_ack_revision,
        clock_epoch: plan.clock_ack_epoch,
        error: plan.error.clone(),
        deferred: plan.deferred_reason.clone(),
        requires_reprepare: plan.requires_reprepare,
        executor_state: plan.feedback["state"].as_str().map(str::to_owned),
        acknowledged: plan.feedback["acknowledged"].as_u64(),
    });
    if presentation != *previous {
        *previous = presentation;
        // Callback may read timeline state; no application lock is held here.
        super::engine::notify_state_change(notifier);
    }
}

pub fn spawn(
    lifecycle: Weak<()>,
    sample: Arc<Mutex<PlaybackSample>>,
    connected: Arc<AtomicBool>,
    endpoint: Arc<Mutex<String>>,
    timeline: Arc<Mutex<super::media_timeline::PreparedTimeline>>,
    notifier: Arc<Mutex<Option<super::engine::StateNotifier>>>,
) {
    std::thread::spawn(move || {
        let id = crate::platform::interop::controller_instance_id();
        let mut client: Option<ControllerClient> = None;
        let mut active_endpoint = String::new();
        let mut sequence = 0_u64;
        let mut previous: Option<PlaybackSample> = None;
        let mut sent_at = Instant::now();
        let mut reported_at = Instant::now();
        let mut retry_at = Instant::now();
        let mut preparation_retry_at = Instant::now();
        let mut preparation_retry_revision = 0_u64;
        let mut last_error = String::new();
        let mut authority_at = Instant::now()-Duration::from_secs(2);
        let mut automatic_claim = AutomaticClaim::default();
        let mut authority_ready = false;
        let mut owner_endpoint_cache: (String, Option<String>) = (String::new(), None);
        let mut owner_endpoint_at = Instant::now() - Duration::from_secs(10);
        let mut unattended_attempt: Option<(String, String)> = None;
        let mut previous_presentation = None;
        loop {
            notify_sync_change(&timeline, &notifier, &mut previous_presentation);
            let alive = lifecycle.strong_count() > 0;
            let requested_endpoint = endpoint.lock().map(|s| s.clone()).unwrap_or_default();
            let usable = alive
                && connected.load(Ordering::Relaxed)
                && super::controller::is_controller_endpoint(&requested_endpoint);
            if !usable || active_endpoint != requested_endpoint {
                if let Some(mut old) = client.take() {
                    sequence += 1;
                    let _ = old.call("controller.media.playback.update", json!({
                        "client_id":id,"sequence":sequence,"position_ms":0,"loaded":false,"playing":false,"rate":1.0}));
                    let _ = old.call("controller.app.instance.remove", json!({"id":id}));
                }
                previous = None;
                authority_ready = false;
                authority_at = Instant::now() - Duration::from_secs(2);
                if let Ok(mut plan) = timeline.lock() {
                    plan.acknowledged_revision = 0;
                    plan.last_ack = None;
                    plan.deferred_reason = None;
                }
                preparation_retry_revision = 0;
                if !alive {
                    return;
                }
            }
            let current = sample.lock().map(|s| s.clone()).unwrap_or_default();
            if usable && !current.name.is_empty() && client.is_none() && Instant::now() >= retry_at
            {
                let attempt = ControllerClient::connect_playback_events(&requested_endpoint)
                    .and_then(|mut new| {
                        new.call(
                            "controller.app.instance.report",
                            identity(&id, &current.name),
                        )?;
                        Ok(new)
                    });
                match attempt {
                    Ok(new) => {
                        client = Some(new);
                        active_endpoint = requested_endpoint;
                        reported_at = Instant::now();
                        previous = None;
                        authority_ready = false;
                        authority_at = Instant::now() - Duration::from_secs(2);
                        automatic_claim = AutomaticClaim::default();
                    }
                    Err(error) => {
                        if error != last_error {
                            eprintln!("Playback sync: {error}");
                            last_error = error;
                        }
                        retry_at = Instant::now() + Duration::from_secs(2);
                    }
                }
            }
            if let Some(ref mut rpc)=client {
                if authority_at.elapsed()>=Duration::from_secs(1) {
                    authority_at=Instant::now();
                    let result=rpc.call("controller.media.authority.get",json!({})).and_then(|value|
                        serde_json::from_value::<super::authority::Status>(value).map_err(|error|format!("Invalid publishing authority state: {error}")));
                    match result {
                        Ok(mut status)=>{
                            authority_ready = true;
                            if automatic_claim.should_request(current.loaded, &status) {
                                match rpc.call_detailed("controller.media.authority.change",json!({"client_id":id,"operation":"request"})) {
                                    Ok(value) => {
                                        if let Ok(updated)=serde_json::from_value(value){status=updated;}
                                    }
                                    Err(error) => {
                                        if let Ok(mut plan)=timeline.lock() {
                                            if error.transport_failed { invalidate_coordinator_session(&mut plan); }
                                            plan.error=Some(format!("Publishing claim failed: {error}"));
                                            plan.play_requested=false;
                                        }
                                        if error.transport_failed {
                                            // Claim outcome is unknown. Reconnect, read authority
                                            // first, and never replay an output or handoff accept.
                                            client=None;
                                            authority_ready=false;
                                            retry_at=Instant::now()+Duration::from_secs(2);
                                            continue;
                                        }
                                    }
                                }
                            }
                            if status.owner_id != owner_endpoint_cache.0 || owner_endpoint_at.elapsed() >= Duration::from_secs(10) {
                                owner_endpoint_at = Instant::now();
                                // Only observers need the remote alternative. Do not
                                // add an address lookup to the active publisher's clock path.
                                owner_endpoint_cache = (status.owner_id.clone(), if status.owner_id.is_empty() || status.owner_id == id { None } else {
                                    rpc.call("controller.app.instance.get", json!({"id": status.owner_id}))
                                        .ok().and_then(|value| super::authority::owner_endpoint(&status.owner_id, &value))
                                });
                            }
                            status.owner_endpoint = owner_endpoint_cache.1.clone();
                            if let Ok(mut plan)=timeline.lock() {
                                if plan.update_authority(status) {
                                    previous=None;
                                }
                            }
                        }
                        Err(error)=>{
                            authority_ready = false;
                            if let Ok(mut plan)=timeline.lock(){
                                invalidate_coordinator_session(&mut plan);
                                plan.error=Some(format!("Publishing authority unavailable: {error}"));
                            }
                            // A failed query cannot leave a dead socket pinned forever.
                            // The new stream queries authority before preparing or sending.
                            client=None;
                            previous=None;
                            retry_at=Instant::now()+Duration::from_secs(2);
                            continue;
                        }
                    }
                }
                if !authority_ready || timeline.lock().is_ok_and(|plan|!plan.may_publish()) {
                    if reported_at.elapsed()>=Duration::from_secs(10) {
                        let _=rpc.call("controller.app.instance.report",identity(&id,&current.name));reported_at=Instant::now();
                    }
                    std::thread::sleep(Duration::from_millis(20));continue;
                }
            }
            // A prepared hardware timeline does not make a paused media clock
            // active. Publish at 25 Hz only while playback advances; explicit
            // state changes still bypass the interval through changed_from().
            let interval = playback_publish_interval(&current);
            let due = previous
                .as_ref()
                .is_none_or(|old| current.changed_from(old))
                || sent_at.elapsed() >= interval;
            let pending = timeline.lock().ok().and_then(|plan| {
                (plan.compilation_error.is_none()
                    && plan.revision != 0
                    && plan.revision != plan.acknowledged_revision
                    && (plan.revision != preparation_retry_revision || Instant::now() >= preparation_retry_at))
                    .then(|| (plan.revision, plan.payload.clone()))
            });
            if let Some((revision, mut payload)) = pending
                && let Some(ref mut rpc) = client
            {
                if current.playing || current.buffering {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                payload["client_id"] = json!(id);
                payload["revision"] = json!(revision);
                match rpc.call_detailed("controller.media.timeline.prepare", payload) {
                    Ok(feedback) => {
                        sequence += 1;
                        let arm_result = rpc.call(
                            "controller.media.playback.update",
                            json!({
                                "client_id": id,
                                "sequence": sequence,
                                "position_ms": current.position_now(),
                                "duration_ms": current.duration_ms,
                                "playing": false,
                                "loaded": current.loaded,
                                "rate": current.rate,
                                "epoch": current.epoch,
                                "plan_revision": revision,
                            }),
                        );
                        if let Ok(mut plan) = timeline.lock()
                            && plan.revision == revision
                        {
                            plan.acknowledged_revision = revision;
                            plan.error = None;
                            plan.deferred_reason = None;
                            if let Ok(ref arm_feedback) = arm_result {
                                if arm_feedback["epoch"].as_u64() == Some(current.epoch)
                                    && arm_feedback["plan_revision"].as_u64() == Some(revision)
                                {
                                    plan.clock_ack_revision = revision;
                                    plan.clock_ack_epoch = current.epoch;
                                    plan.feedback = arm_feedback["timeline"].clone();
                                    plan.last_ack = Some(Instant::now());
                                } else {
                                    plan.feedback = feedback;
                                    plan.last_ack = None;
                                }
                            } else {
                                plan.feedback = feedback;
                                plan.last_ack = None;
                            }
                        }
                        preparation_retry_revision = 0;
                        previous = None;
                    }
                    Err(error) => {
                        preparation_retry_revision = revision;
                        if error.is_resource_busy("addressable_strip") {
                            let retry_ms = error.retry_after_ms.unwrap_or(2_000).clamp(500, 10_000);
                            preparation_retry_at = Instant::now() + Duration::from_millis(retry_ms);
                            if let Ok(mut plan) = timeline.lock() {
                                if !plan.requires_reprepare {
                                    plan.error = None;
                                }
                                plan.deferred_reason = Some(format!("Hardware timeline is waiting: {}", error.message));
                                plan.play_requested = false;
                            }
                            continue;
                        }
                        preparation_retry_at = Instant::now() + Duration::from_secs(5);
                        if let Ok(mut plan) = timeline.lock() {
                            plan.deferred_reason = None;
                            plan.error = Some(format!("Hardware timeline not prepared: {}", error.message));
                            plan.play_requested = false;
                        }
                        continue;
                    }
                }
            }
            if due && let Some(ref mut rpc) = client {
                let revision = timeline
                    .lock()
                    .map(|plan| plan.acknowledged_revision)
                    .unwrap_or(0);
                if current.playing && current.observed_at.elapsed() > Duration::from_millis(250) {
                    continue;
                }
                let has_hardware = timeline.lock().is_ok_and(|plan| plan.has_items());
                let is_armed_for_epoch = timeline.lock().ok().is_some_and(|plan| {
                    plan.feedback["armed_epoch"].as_u64() == Some(current.epoch)
                        && plan.clock_ack_epoch == current.epoch
                        && plan.clock_ack_revision == revision
                        && plan.acknowledged_revision == revision
                });
                let outgoing_playing = if has_hardware {
                    current.playing && is_armed_for_epoch
                } else {
                    current.playing
                };
                sequence += 1;
                let result = rpc.call("controller.media.playback.update",json!({
                    "client_id":id,"sequence":sequence,"position_ms":current.position_now(),
                    "duration_ms":current.duration_ms,"playing":outgoing_playing,"loaded":current.loaded,"rate":current.rate,"epoch":current.epoch,"plan_revision":revision}));
                match result {
                    Ok(feedback) => {
                        let valid_echo = feedback["sequence"].as_u64() == Some(sequence)
                            && feedback["client_id"] == id
                            && feedback["epoch"].as_u64() == Some(current.epoch)
                            && feedback["plan_revision"].as_u64().unwrap_or(0) == revision;
                        if let Ok(mut plan) = timeline.lock() {
                            if !valid_echo {
                                plan.error =
                                    Some("Hardware clock echo mismatch; playback paused".into());
                                plan.play_requested = false;
                            } else {
                                plan.last_ack = Some(Instant::now());
                                plan.clock_ack_revision = revision;
                                plan.clock_ack_epoch = current.epoch;
                                plan.feedback = feedback["timeline"].clone();
                                // A transient transport failure can clear on a fresh
                                // acknowledgement. A discontinuity fault is different:
                                // keep it latched until explicit Play requests re-arming.
                                if !plan.requires_reprepare {
                                    plan.error = None;
                                }
                                if plan.feedback["state"] == "faulted" {
                                    plan.error = Some(
                                        plan.feedback["error"]
                                            .as_str()
                                            .unwrap_or("Hardware deadline failure")
                                            .to_owned(),
                                    );
                                    plan.play_requested = false;
                                }
                            }
                        }
                        previous = Some(current.clone());
                        sent_at = Instant::now();
                        last_error.clear();
                        // Owner consent is configured locally. A real observed pause
                        // and its successful echo precede PCController's existing
                        // acknowledged cleanup/transfer. Never replay a failed accept.
                        let claim = if handoff_pause_acknowledged(&current, valid_echo) {
                            let enabled = crate::platform::interop::allow_unattended_hardware_takeover();
                            timeline.lock().ok().and_then(|plan| plan.authority.as_ref()
                                .and_then(|status| status.unattended_claim(&id, enabled)).cloned())
                        } else { None };
                        if let Some(claim) = claim {
                            let key = (claim.client_id.clone(), claim.requested_at.clone());
                            if unattended_attempt.as_ref() != Some(&key) {
                                unattended_attempt = Some(key);
                                let result = rpc.call("controller.media.authority.change", json!({
                                    "client_id": id, "operation": "accept", "requester_id": claim.client_id
                                })).and_then(|value| serde_json::from_value::<super::authority::Status>(value)
                                    .map_err(|error| format!("Invalid authority handoff acknowledgement: {error}")));
                                match result {
                                    Ok(status) => {
                                        if let Ok(mut plan) = timeline.lock() { plan.update_authority(status); }
                                        previous = None;
                                        authority_at = Instant::now() - Duration::from_secs(2);
                                        let _ = crate::messaging::publish(crate::messaging::ToastRequest {
                                            id: Some("hardware.authority".into()), title: "Publishing handoff completed".into(),
                                            message: format!("Playback paused. {} now owns hardware publishing.", claim.label),
                                            severity: crate::messaging::Severity::Info, timeout_ms: 6000,
                                        }, "hardware");
                                    }
                                    Err(error) => {
                                        let _ = crate::messaging::publish(crate::messaging::ToastRequest {
                                            id: Some("hardware.authority".into()), title: "Unattended handoff failed".into(),
                                            message: format!("{error}. Playback remains paused; review the handoff manually."),
                                            severity: crate::messaging::Severity::Error, timeout_ms: 10000,
                                        }, "hardware");
                                    }
                                }
                            }
                        }
                    }
                    Err(error) => {
                        if let Ok(mut plan) = timeline.lock() {
                            if plan.has_items() {
                                plan.error =
                                    Some(format!("Hardware clock acknowledgement failed: {error}"));
                                plan.play_requested = false;
                            }
                        }
                        if error != last_error {
                            eprintln!("Playback sync: {error}");
                            last_error = error;
                        }
                        client = None;
                        previous = None;
                        retry_at = Instant::now() + Duration::from_secs(2);
                    }
                }
            }
            if reported_at.elapsed() >= Duration::from_secs(10)
                && let Some(ref mut rpc) = client
            {
                if rpc
                    .call(
                        "controller.app.instance.report",
                        identity(&id, &current.name),
                    )
                    .is_err()
                {
                    client = None;
                }
                reported_at = Instant::now();
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });
}
fn identity(id: &str, name: &str) -> serde_json::Value {
    crate::platform::interop::controller_instance_identity(id, name)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_claim_recovers_after_release_or_reconnect_without_taking_an_owner() {
        let mut status = super::super::authority::Status {
            owner_id: String::new(), owner_label: String::new(), exclusive: false,
            revision: 1, pending: vec![], owner_endpoint: None,
        };
        let mut claim = AutomaticClaim::default();
        assert!(!claim.should_request(false, &status));
        assert!(claim.should_request(true, &status));
        assert!(!claim.should_request(true, &status));
        status.owner_id = "another-publisher".into();
        assert!(!claim.should_request(true, &status));
        let mut reconnected = AutomaticClaim::default();
        assert!(!reconnected.should_request(true, &status));
        status.exclusive = true;
        assert!(!reconnected.should_request(true, &status));
        status.owner_id.clear();
        status.exclusive = false;
        assert!(claim.should_request(true, &status));
        assert!(!claim.should_request(true, &status));
        assert!(AutomaticClaim::default().should_request(true, &status));
    }

    #[test]
    fn failed_authority_query_invalidates_arm_and_requires_a_new_plan_revision() {
        let mut plan = super::super::media_timeline::PreparedTimeline::default();
        plan.revision = 7;
        plan.acknowledged_revision = 7;
        plan.clock_ack_revision = 7;
        plan.clock_ack_epoch = 3;
        plan.feedback = json!({"armed_epoch":3,"state":"playing"});
        plan.last_ack = Some(Instant::now());
        plan.play_requested = true;
        assert!(plan.ready_for(3));
        invalidate_coordinator_session(&mut plan);
        assert_eq!(plan.revision, 8);
        assert_eq!(plan.acknowledged_revision, 0);
        assert_eq!(plan.clock_ack_revision, 0);
        assert_eq!(plan.clock_ack_epoch, 0);
        assert!(plan.last_ack.is_none());
        assert!(!plan.play_requested);
        assert!(!plan.ready_for(3));
    }

    #[test]
    fn sync_notifications_wake_on_transitions_not_clock_echoes_and_release_locks() {
        let plan = Arc::new(Mutex::new(super::super::media_timeline::PreparedTimeline::default()));
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let callback_plan = plan.clone();
        let callback_calls = calls.clone();
        let callback: super::super::engine::StateNotifier = Arc::new(move || {
            assert!(callback_plan.try_lock().is_ok());
            callback_calls.fetch_add(1, Ordering::Relaxed);
        });
        let notifier = Mutex::new(Some(callback));
        let mut previous = None;
        notify_sync_change(&plan, &notifier, &mut previous);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        {
            let mut plan = plan.lock().unwrap();
            plan.last_ack = Some(Instant::now());
            plan.feedback = json!({"clock_sequence":20});
        }
        notify_sync_change(&plan, &notifier, &mut previous);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        plan.lock().unwrap().acknowledged_revision = 1;
        notify_sync_change(&plan, &notifier, &mut previous);
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        plan.lock().unwrap().feedback = json!({"state":"playing","acknowledged":1});
        notify_sync_change(&plan, &notifier, &mut previous);
        assert_eq!(calls.load(Ordering::Relaxed), 3);
        plan.lock().unwrap().error = Some("cue deadline failed".into());
        notify_sync_change(&plan, &notifier, &mut previous);
        assert_eq!(calls.load(Ordering::Relaxed), 4);
        notify_sync_change(&plan, &notifier, &mut previous);
        assert_eq!(calls.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn playback_and_action_subscriptions_share_one_complete_client_identity() {
        let id = crate::platform::interop::controller_instance_id();
        let value = identity(&id, "Custom Player");
        assert_eq!(value["id"], id);
        assert_eq!(value["values"]["application"], "Custom Player");
        assert!(
            value["values"]["app_actions"]
                .as_str()
                .unwrap()
                .contains("pealayer.pause")
        );
        assert!(
            value["self"]["vars"]["rpc"]
                .as_str()
                .unwrap()
                .ends_with("/api/rpc")
        );
    }
    #[test]
    fn playback_sample_detects_state_seek_duration_and_rate_without_false_seek() {
        let a = PlaybackSample {
            name: "Player".into(),
            loaded: true,
            playing: true,
            position_ms: 10_000,
            ..Default::default()
        };
        assert!(!a.changed_from(&a));
        let mut b = a.clone();
        b.playing = false;
        assert!(b.changed_from(&a));
        b = a.clone();
        b.position_ms += 1000;
        assert!(b.changed_from(&a));
        b = a.clone();
        b.rate = 2.0;
        assert!(b.changed_from(&a));
        b = a.clone();
        b.duration_ms = Some(15_000);
        assert!(b.changed_from(&a));
    }
    #[test]
    fn paused_sample_does_not_advance_and_playing_position_is_bounded() {
        let mut value = PlaybackSample {
            position_ms: 10_000,
            sampled_at: Instant::now() - Duration::from_secs(1),
            ..Default::default()
        };
        assert_eq!(value.position_now(), 10_000);
        value.playing = true;
        value.rate = 2.;
        value.duration_ms = Some(11_000);
        assert_eq!(value.position_now(), 11_000);
    }

    #[test]
    fn paused_hardware_timeline_uses_idle_publish_cadence() {
        let mut value = PlaybackSample::default();
        assert_eq!(playback_publish_interval(&value), Duration::from_secs(1));
        value.playing = true;
        assert_eq!(
            playback_publish_interval(&value),
            Duration::from_millis(40)
        );
    }

    #[test]
    fn unattended_handoff_requires_fresh_observed_pause_and_matching_echo() {
        let mut value = PlaybackSample::default();
        assert!(handoff_pause_acknowledged(&value, true));
        assert!(!handoff_pause_acknowledged(&value, false));
        value.playing = true;
        assert!(!handoff_pause_acknowledged(&value, true));
        value.playing = false;
        value.buffering = true;
        assert!(!handoff_pause_acknowledged(&value, true));
        value.buffering = false;
        value.observed_at = Instant::now() - Duration::from_secs(1);
        assert!(!handoff_pause_acknowledged(&value, true));
    }
}
