//! Playback telemetry uses its own bounded RPC stream, never the actuator queue.
use super::controller::ControllerClient;
use serde_json::json;
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, SyncSender},
};
use std::time::{Duration, Instant};

/// Bounded, process-local evidence for clock starvation; no media paths or logs.
/// These counters do not trigger UI repaints on every telemetry packet.
#[derive(Default, serde::Serialize)]
pub struct ClockTransportDiagnostics {
    updates: u64,
    failed_updates: u64,
    stale_observation_skips: u64,
    last_send_gap_ms: u64,
    max_playing_send_gap_ms: u64,
    last_round_trip_ms: u64,
    max_round_trip_ms: u64,
    last_observation_age_ms: u64,
    last_authority_request_ms: u64,
    max_authority_request_ms: u64,
    last_instance_report_ms: u64,
}

impl ClockTransportDiagnostics {
    fn record_update(&mut self, gap: Option<(Duration, bool)>, round_trip: Duration,
        observation_age: Duration, failed: bool) {
        self.updates = self.updates.saturating_add(1);
        self.failed_updates = self.failed_updates.saturating_add(u64::from(failed));
        self.last_send_gap_ms = gap.map_or(0, |(gap, _)| gap.as_millis() as u64);
        if gap.is_some_and(|(_, was_playing)| was_playing) {
            self.max_playing_send_gap_ms = self.max_playing_send_gap_ms.max(self.last_send_gap_ms);
        }
        self.last_round_trip_ms = round_trip.as_millis() as u64;
        self.max_round_trip_ms = self.max_round_trip_ms.max(self.last_round_trip_ms);
        self.last_observation_age_ms = observation_age.as_millis() as u64;
    }
}

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
    // Even an acknowledged empty plan has a coordinator revision; do not
    // reuse it after losing that coordinator session.
    if plan.revision != 0 { plan.revision = plan.revision.saturating_add(1); }
    plan.acknowledged_revision = 0;
    plan.clock_ack_revision = 0;
    plan.clock_ack_epoch = 0;
    plan.last_ack = None;
    plan.feedback = serde_json::Value::Null;
    plan.play_requested = false;
}

fn take_coordinator_session<T>(
    client: &mut Option<T>,
    timeline: &Mutex<super::media_timeline::PreparedTimeline>,
) -> Option<T> {
    let old = client.take()?;
    // Invalidate once per live stream, not on every disconnected polling pass.
    // Every transport-loss path shares this transition; no old arm or Play
    // request may survive an ACK/heartbeat failure on an otherwise live engine.
    if let Ok(mut plan) = timeline.lock() {
        invalidate_coordinator_session(&mut plan);
    }
    Some(old)
}

#[derive(serde::Deserialize)]
struct ControllerPlanIdentity {
    client_id: String,
    revision: u64,
}

#[derive(serde::Deserialize)]
struct ControllerClockIdentity {
    client_id: String,
    sequence: u64,
}

fn reconcile_clock_sequence(
    client_id: &str,
    sequence: &mut u64,
    feedback: serde_json::Value,
) -> Result<(), String> {
    let remote: ControllerClockIdentity = serde_json::from_value(feedback)
        .map_err(|error| format!("Invalid hardware clock identity: {error}"))?;
    let floor = if remote.client_id == client_id {
        (*sequence).max(remote.sequence)
    } else {
        *sequence
    };
    floor.checked_add(1)
        .ok_or_else(|| "Hardware clock sequence exhausted".to_string())?;
    // A stable publisher can restart faster than its old clock lease expires.
    // Continue only its own counter; do not release its exclusive authority,
    // adopt another publisher's epoch, or replay any previous output.
    // The next outgoing paused arm/update increments this floor before sending.
    *sequence = floor;
    Ok(())
}

fn reconcile_controller_revision(
    plan: &mut super::media_timeline::PreparedTimeline,
    feedback: serde_json::Value,
) -> Result<(), String> {
    let remote: ControllerPlanIdentity = serde_json::from_value(feedback)
        .map_err(|error| format!("Invalid hardware timeline identity: {error}"))?;
    // Revisions are monotonic per publisher, not per Pealayer process. The
    // coordinator can retain our old plan across an application update.
    if remote.client_id == plan.authority_client_id && remote.revision >= plan.revision {
        let next = remote.revision.checked_add(1)
            .ok_or_else(|| "Hardware timeline revision exhausted".to_string())?;
        invalidate_coordinator_session(plan);
        plan.revision = next;
    }
    Ok(())
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

struct ClockMetadataRequest {
    endpoint: String,
    generation: u64,
    id: String,
    name: String,
}

struct ClockMetadataReply {
    endpoint: String,
    generation: u64,
    authority: Result<super::authority::Status, String>,
    authority_elapsed: Duration,
    report_elapsed: Option<Duration>,
}

/// Metadata must never occupy the deadline-bound playback RPC stream. Both
/// directions are capacity-one, so a slow peer cannot build an unbounded queue.
fn spawn_clock_metadata<F>(mut make_identity: F) -> (SyncSender<ClockMetadataRequest>, Receiver<ClockMetadataReply>)
where F: FnMut(&ClockMetadataRequest) -> serde_json::Value + Send + 'static {
    let (request_tx, request_rx) = mpsc::sync_channel::<ClockMetadataRequest>(1);
    let (reply_tx, reply_rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut client: Option<ControllerClient> = None;
        let mut endpoint = String::new();
        let mut generation = 0;
        let mut reported_at = Instant::now() - Duration::from_secs(10);
        let mut owner_endpoint_cache: (String, Option<String>) = (String::new(), None);
        let mut owner_endpoint_at = Instant::now() - Duration::from_secs(10);
        while let Ok(request) = request_rx.recv() {
            if request.endpoint != endpoint || request.generation != generation {
                client = None;
                endpoint.clone_from(&request.endpoint);
                generation = request.generation;
                owner_endpoint_cache = (String::new(), None);
            }
            let mut authority_elapsed = Duration::ZERO;
            let mut report_elapsed = None;
            let authority = (|| -> Result<super::authority::Status, String> {
                if client.is_none() {
                    client = Some(ControllerClient::connect_playback_events(&request.endpoint)?);
                    reported_at = Instant::now() - Duration::from_secs(10);
                }
                let rpc = client.as_mut().expect("connected metadata stream");
                if reported_at.elapsed() >= Duration::from_secs(10) {
                    let started = Instant::now();
                    let result = rpc.call("controller.app.instance.report", make_identity(&request));
                    report_elapsed = Some(started.elapsed());
                    result?;
                    reported_at = Instant::now();
                }
                let started = Instant::now();
                let result = rpc.call("controller.media.authority.get", json!({}));
                authority_elapsed = started.elapsed();
                let mut status: super::authority::Status = serde_json::from_value(result?)
                    .map_err(|error| format!("Invalid publishing authority state: {error}"))?;
                if status.owner_id != owner_endpoint_cache.0 || owner_endpoint_at.elapsed() >= Duration::from_secs(10) {
                    owner_endpoint_at = Instant::now();
                    owner_endpoint_cache = (status.owner_id.clone(), if status.owner_id.is_empty() || status.owner_id == request.id { None } else {
                        rpc.call("controller.app.instance.get", json!({"id": status.owner_id}))
                            .ok().and_then(|value| super::authority::owner_endpoint(&status.owner_id, &value))
                    });
                }
                status.owner_endpoint = owner_endpoint_cache.1.clone();
                Ok(status)
            })();
            if authority.is_err() { client = None; }
            if reply_tx.send(ClockMetadataReply { endpoint: request.endpoint, generation: request.generation, authority, authority_elapsed, report_elapsed }).is_err() {
                break;
            }
        }
    });
    (request_tx, reply_rx)
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
        let mut retry_at = Instant::now();
        let mut preparation_retry_at = Instant::now();
        let mut preparation_retry_revision = 0_u64;
        let mut last_error = String::new();
        let mut authority_at = Instant::now()-Duration::from_secs(2);
        let mut automatic_claim = AutomaticClaim::default();
        let mut authority_ready = false;
        let mut authority_ack_at = Instant::now();
        let mut metadata_pending = false;
        let mut metadata_generation = 0_u64;
        let (mut metadata_tx, mut metadata_rx) = spawn_clock_metadata(|request| identity(&request.id, &request.name));
        let mut reconcile_session = true;
        let mut unattended_attempt: Option<(String, String)> = None;
        let mut previous_presentation = None;
        let mut last_clock_send: Option<(Instant, bool)> = None;
        loop {
            notify_sync_change(&timeline, &notifier, &mut previous_presentation);
            let alive = lifecycle.strong_count() > 0;
            let requested_endpoint = endpoint.lock().map(|s| s.clone()).unwrap_or_default();
            let usable = alive
                && connected.load(Ordering::Relaxed)
                && super::controller::is_controller_endpoint(&requested_endpoint);
            if !usable || active_endpoint != requested_endpoint {
                if let Some(mut old) = take_coordinator_session(&mut client, &timeline) {
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
                        metadata_generation = metadata_generation.saturating_add(1);
                        active_endpoint = requested_endpoint;
                        previous = None;
                        authority_ready = false;
                        authority_at = Instant::now() - Duration::from_secs(2);
                        automatic_claim = AutomaticClaim::default();
                        reconcile_session = true;
                        last_clock_send = None;
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
                if !metadata_pending && authority_at.elapsed() >= Duration::from_secs(1)
                    && metadata_tx.try_send(ClockMetadataRequest {
                        endpoint: active_endpoint.clone(), generation: metadata_generation,
                        id: id.clone(), name: current.name.clone(),
                    }).is_ok()
                {
                    authority_at = Instant::now();
                    metadata_pending = true;
                }
                let reply = match metadata_rx.try_recv() {
                    Ok(reply) => Some(reply),
                    Err(mpsc::TryRecvError::Empty) => None,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        // Recover an exited metadata worker without retaining
                        // its old authority or blocking the playback stream.
                        (metadata_tx, metadata_rx) = spawn_clock_metadata(|request| identity(&request.id, &request.name));
                        metadata_pending = false;
                        authority_ready = false;
                        if let Ok(mut plan) = timeline.lock() {
                            plan.error = Some("Publishing metadata worker restarted; refreshing authority".into());
                        }
                        let _ = take_coordinator_session(&mut client, &timeline);
                        previous = None;
                        retry_at = Instant::now() + Duration::from_secs(2);
                        continue;
                    }
                };
                if let Some(reply) = reply {
                    metadata_pending = false;
                    // A result from a replaced endpoint cannot authorize its successor.
                    if reply.endpoint != active_endpoint || reply.generation != metadata_generation {
                        authority_at = Instant::now() - Duration::from_secs(2);
                        continue;
                    }
                    if let Ok(mut plan) = timeline.lock() {
                        let elapsed = reply.authority_elapsed.as_millis() as u64;
                        plan.clock_transport.last_authority_request_ms = elapsed;
                        plan.clock_transport.max_authority_request_ms = plan.clock_transport.max_authority_request_ms.max(elapsed);
                        if let Some(elapsed) = reply.report_elapsed {
                            plan.clock_transport.last_instance_report_ms = elapsed.as_millis() as u64;
                        }
                    }
                    match reply.authority {
                        Ok(mut status)=>{
                            authority_ready = true;
                            authority_ack_at = Instant::now();
                            if automatic_claim.should_request(current.loaded, &status) {
                                match rpc.call_detailed("controller.media.authority.change",json!({"client_id":id,"operation":"request"})) {
                                    Ok(value) => {
                                        if let Ok(updated)=serde_json::from_value(value){status=updated;}
                                    }
                                    Err(error) => {
                                        if let Ok(mut plan)=timeline.lock() {
                                            plan.error=Some(format!("Publishing claim failed: {error}"));
                                            plan.play_requested=false;
                                        }
                                        if error.transport_failed {
                                            // Claim outcome is unknown. Reconnect, read authority
                                            // first, and never replay an output or handoff accept.
                                            let _ = take_coordinator_session(&mut client, &timeline);
                                            authority_ready=false;
                                            retry_at=Instant::now()+Duration::from_secs(2);
                                            continue;
                                        }
                                    }
                                }
                            }
                            if let Ok(mut plan)=timeline.lock() {
                                if plan.update_authority(status) {
                                    previous=None;
                                }
                            }
                        }
                        Err(error)=>{
                            authority_ready = false;
                            if let Ok(mut plan)=timeline.lock(){
                                plan.error=Some(format!("Publishing authority unavailable: {error}"));
                            }
                            // A failed query cannot leave a dead socket pinned forever.
                            // The new stream queries authority before preparing or sending.
                            let _ = take_coordinator_session(&mut client, &timeline);
                            previous=None;
                            retry_at=Instant::now()+Duration::from_secs(2);
                            continue;
                        }
                    }
                }
                // Clock RPCs still validate ownership on the coordinator. A
                // stalled metadata worker nevertheless cannot retain local
                // permission indefinitely or silently replay a stale plan.
                if authority_ready && authority_ack_at.elapsed() >= Duration::from_secs(3) {
                    authority_ready = false;
                    if let Ok(mut plan) = timeline.lock() {
                        plan.error = Some("Publishing authority refresh expired; playback paused".into());
                    }
                    let _ = take_coordinator_session(&mut client, &timeline);
                    previous = None;
                    retry_at = Instant::now() + Duration::from_secs(2);
                    continue;
                }
                if !authority_ready || timeline.lock().is_ok_and(|plan|!plan.may_publish()) {
                    std::thread::sleep(Duration::from_millis(20));continue;
                }
            }
            if reconcile_session && let Some(ref mut rpc) = client {
                let result = rpc.call("controller.media.playback.get", json!({}))
                    .map_err(|error| format!("Playback clock snapshot: {error}"))
                    .and_then(|feedback| reconcile_clock_sequence(&id, &mut sequence, feedback))
                    .and_then(|_| rpc.call("controller.media.timeline.get", json!({})))
                    .and_then(|feedback| {
                        let mut plan = timeline.lock()
                            .map_err(|_| "Hardware timeline state unavailable".to_string())?;
                        reconcile_controller_revision(&mut plan, feedback)
                    });
                if let Err(error) = result {
                    if let Ok(mut plan) = timeline.lock() {
                        plan.error = Some(format!("Hardware coordinator state unavailable: {error}"));
                    }
                    let _ = take_coordinator_session(&mut client, &timeline);
                    authority_ready = false;
                    previous = None;
                    retry_at = Instant::now() + Duration::from_secs(2);
                    continue;
                }
                reconcile_session = false;
                preparation_retry_revision = 0;
                previous = None;
            }
            // A prepared hardware timeline does not make a paused media clock
            // active. Publish at 25 Hz only while playback advances; explicit
            // state changes still bypass the interval through changed_from().
            // Registration, reconciliation or a prepare may have taken time.
            // Publish the newest decoder observation, not the loop's old copy.
            let current = sample.lock().map(|s| s.clone()).unwrap_or_default();
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
                        if let Err(error) = arm_result {
                            let _ = take_coordinator_session(&mut client, &timeline);
                            if let Ok(mut plan) = timeline.lock() {
                                plan.error = Some(format!("Hardware clock acknowledgement failed: {error}"));
                            }
                            authority_ready = false;
                            previous = None;
                            retry_at = Instant::now() + Duration::from_secs(2);
                            continue;
                        }
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
                        if error.transport_failed {
                            let _ = take_coordinator_session(&mut client, &timeline);
                            authority_ready = false;
                            previous = None;
                            retry_at = Instant::now() + Duration::from_secs(2);
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
                    if let Ok(mut plan) = timeline.lock() {
                        plan.clock_transport.stale_observation_skips = plan.clock_transport.stale_observation_skips.saturating_add(1);
                        plan.clock_transport.last_observation_age_ms = current.observed_at.elapsed().as_millis() as u64;
                    }
                    // The stale clock remains fail-closed. Busy-spinning here
                    // consumed a core and further starved the media observer.
                    std::thread::sleep(Duration::from_millis(10));
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
                let started = Instant::now();
                let observation_age = current.observed_at.elapsed();
                let gap = last_clock_send.map(|(last, playing)| (started.duration_since(last), playing));
                last_clock_send = Some((started, outgoing_playing));
                let result = rpc.call("controller.media.playback.update",json!({
                    "client_id":id,"sequence":sequence,"position_ms":current.position_now(),
                    "duration_ms":current.duration_ms,"playing":outgoing_playing,"loaded":current.loaded,"rate":current.rate,"epoch":current.epoch,"plan_revision":revision}));
                if let Ok(mut plan) = timeline.lock() {
                    plan.clock_transport.record_update(gap, started.elapsed(), observation_age, result.is_err());
                }
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
                        let _ = take_coordinator_session(&mut client, &timeline);
                        authority_ready = false;
                        previous = None;
                        retry_at = Instant::now() + Duration::from_secs(2);
                    }
                }
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
    fn blocked_metadata_does_not_occupy_the_clock_rpc_stream() {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("pccontroller://{}", listener.local_addr().unwrap());
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let server = std::thread::spawn(move || {
            let mut handlers = Vec::new();
            let mut release_rx = Some(release_rx);
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let started_tx = started_tx.clone();
                let release_rx = release_rx.take();
                handlers.push(std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    loop {
                        let mut line = String::new();
                        if reader.read_line(&mut line).unwrap() == 0 { break; }
                        let request: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
                        let method = request["method"].as_str().unwrap();
                        let result = match method {
                            "controller.ping" | "controller.app.instance.report" => json!({"ok":true}),
                            "controller.media.authority.get" => {
                                started_tx.send(()).unwrap();
                                release_rx.as_ref().unwrap().recv_timeout(Duration::from_secs(5)).unwrap();
                                json!({"owner_id":"pealayer:test:8080","owner_label":"Test","exclusive":false,"revision":1,"pending":[]})
                            }
                            "controller.media.playback.update" => request["params"].clone(),
                            other => panic!("Unexpected fixture RPC: {other}"),
                        };
                        writeln!(stream, "{}", json!({"jsonrpc":"2.0","id":request["id"],"result":result})).unwrap();
                        if method.starts_with("controller.media.") { break; }
                    }
                }));
            }
            for handler in handlers { handler.join().unwrap(); }
        });
        // Inject a fixture identity: never consult/persist the user's config.
        let (metadata_tx, metadata_rx) = spawn_clock_metadata(|request| json!({"id":request.id}));
        metadata_tx.send(ClockMetadataRequest {
            endpoint: endpoint.clone(), generation: 1, id: "pealayer:test:8080".into(), name: "Test".into(),
        }).unwrap();
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let mut clock = ControllerClient::connect_playback_events(&endpoint).unwrap();
        clock.call("controller.app.instance.report", json!({"id":"pealayer:test:8080"})).unwrap();
        let echo = clock.call("controller.media.playback.update", json!({"sequence":42})).unwrap();
        assert_eq!(echo["sequence"], 42);
        assert!(matches!(metadata_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        release_tx.send(()).unwrap();
        let reply = metadata_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(reply.generation, 1);
        assert_eq!(reply.authority.unwrap().owner_id, "pealayer:test:8080");
        drop(metadata_tx);
        drop(metadata_rx);
        server.join().unwrap();
    }

    #[test]
    fn clock_diagnostics_exclude_idle_gaps_and_retain_playing_starvation() {
        let mut stats = ClockTransportDiagnostics::default();
        stats.record_update(Some((Duration::from_secs(1), false)), Duration::from_millis(2), Duration::from_millis(5), false);
        assert_eq!(stats.max_playing_send_gap_ms, 0);
        stats.record_update(Some((Duration::from_millis(310), true)), Duration::from_millis(270), Duration::from_millis(8), true);
        stats.record_update(Some((Duration::from_millis(40), true)), Duration::from_millis(1), Duration::from_millis(3), false);
        assert_eq!(stats.max_playing_send_gap_ms, 310);
        assert_eq!(stats.max_round_trip_ms, 270);
        assert_eq!(stats.failed_updates, 1);
        assert_eq!(stats.updates, 3);
    }
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
    fn stream_loss_invalidates_once_and_disconnected_polls_never_churn_revisions() {
        let mut plan = super::super::media_timeline::PreparedTimeline::default();
        plan.revision = 7;
        plan.acknowledged_revision = 7;
        plan.clock_ack_revision = 7;
        plan.clock_ack_epoch = 3;
        plan.feedback = json!({"armed_epoch":3,"state":"playing"});
        plan.last_ack = Some(Instant::now());
        plan.play_requested = true;
        plan.requires_reprepare = true;
        let timeline = Mutex::new(plan);
        let mut client = Some("live clock stream");
        assert_eq!(take_coordinator_session(&mut client, &timeline), Some("live clock stream"));
        for _ in 0..100 {
            assert!(take_coordinator_session(&mut client, &timeline).is_none());
        }
        {
            let plan = timeline.lock().unwrap();
            assert_eq!(plan.revision, 8);
            assert_eq!(plan.acknowledged_revision, 0);
            assert_eq!(plan.clock_ack_revision, 0);
            assert_eq!(plan.clock_ack_epoch, 0);
            assert!(plan.last_ack.is_none());
            assert!(plan.feedback.is_null());
            assert!(!plan.play_requested);
            assert!(plan.requires_reprepare, "Semantic safety faults stay latched");
            assert!(!plan.ready_for(3));
        }
        client = Some("replacement clock stream");
        let _ = take_coordinator_session(&mut client, &timeline);
        assert_eq!(timeline.lock().unwrap().revision, 9);
    }

    #[test]
    fn fresh_empty_coordinator_uses_new_revision_after_clock_transport_loss() {
        let mut plan = super::super::media_timeline::PreparedTimeline::default();
        plan.revision = 3;
        plan.acknowledged_revision = 3;
        plan.clock_ack_revision = 3;
        plan.clock_ack_epoch = 2;
        plan.play_requested = true;
        let timeline = Mutex::new(plan);
        let _ = take_coordinator_session(&mut Some(()), &timeline);
        let mut plan = timeline.lock().unwrap();
        reconcile_controller_revision(&mut plan, json!({"client_id":"","revision":0})).unwrap();
        assert_eq!(plan.revision, 4);
        assert_eq!(plan.acknowledged_revision, 0);
        assert!(!plan.play_requested);
    }

    #[test]
    fn restarted_publisher_adopts_its_retained_revision_without_resuming() {
        let mut plan = super::super::media_timeline::PreparedTimeline::default();
        plan.revision = 3;
        plan.acknowledged_revision = 3;
        plan.clock_ack_revision = 3;
        plan.clock_ack_epoch = 2;
        plan.play_requested = true;
        plan.requires_reprepare = true;
        plan.feedback = json!({"state":"faulted"});
        let actor = plan.authority_client_id.clone();
        reconcile_controller_revision(&mut plan, json!({"client_id":actor,"revision":17})).unwrap();
        assert_eq!(plan.revision, 18);
        assert_eq!(plan.acknowledged_revision, 0);
        assert_eq!(plan.clock_ack_revision, 0);
        assert!(!plan.play_requested);
        assert!(plan.requires_reprepare);
        assert!(plan.feedback.is_null());
    }

    #[test]
    fn restarted_publisher_continues_only_its_own_clock_counter_without_releasing_authority() {
        let mut sequence = 0;
        reconcile_clock_sequence("publisher", &mut sequence,
            json!({"client_id":"publisher","sequence":7578,"epoch":99})).unwrap();
        assert_eq!(sequence, 7578);
        sequence += 1; // The first paused arm is strictly newer than the retained clock.
        assert_eq!(sequence, 7579);
        reconcile_clock_sequence("publisher", &mut sequence,
            json!({"client_id":"publisher","sequence":7000})).unwrap();
        assert_eq!(sequence, 7579, "A reconnect never moves the local counter backwards");
    }

    #[test]
    fn clock_counter_reconciliation_ignores_other_actors_and_handles_an_empty_coordinator() {
        let mut sequence = 0;
        reconcile_clock_sequence("publisher", &mut sequence,
            json!({"client_id":"other","sequence":u64::MAX})).unwrap();
        assert_eq!(sequence, 0);
        reconcile_clock_sequence("publisher", &mut sequence,
            json!({"client_id":"","sequence":0})).unwrap();
        sequence += 1;
        assert_eq!(sequence, 1);
    }

    #[test]
    fn clock_counter_reconciliation_rejects_malformed_or_exhausted_state_without_mutation() {
        let mut sequence = 5;
        for feedback in [json!({"sequence":7}), json!({"client_id":"publisher"}),
            json!({"client_id":"publisher","sequence":"7"}),
            json!({"client_id":"publisher","sequence":u64::MAX})] {
            assert!(reconcile_clock_sequence("publisher", &mut sequence, feedback).is_err());
            assert_eq!(sequence, 5);
        }
    }

    #[test]
    fn revision_reconciliation_leaves_other_publishers_and_newer_local_plans_untouched() {
        let mut plan = super::super::media_timeline::PreparedTimeline::default();
        plan.revision = 8;
        plan.acknowledged_revision = 8;
        reconcile_controller_revision(&mut plan, json!({"client_id":"other","revision":99})).unwrap();
        assert_eq!(plan.revision, 8);
        let actor = plan.authority_client_id.clone();
        reconcile_controller_revision(&mut plan, json!({"client_id":actor,"revision":7})).unwrap();
        assert_eq!(plan.revision, 8);
        assert_eq!(plan.acknowledged_revision, 8);
    }

    #[test]
    fn revision_reconciliation_rejects_malformed_identity_and_overflow_without_mutation() {
        let mut plan = super::super::media_timeline::PreparedTimeline::default();
        plan.revision = 3;
        let actor = plan.authority_client_id.clone();
        for feedback in [json!({"revision":5}), json!({"client_id":actor,"revision":"5"}),
            json!({"client_id":actor,"revision":u64::MAX})] {
            assert!(reconcile_controller_revision(&mut plan, feedback).is_err());
            assert_eq!(plan.revision, 3);
        }
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
