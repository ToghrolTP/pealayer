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

pub fn spawn(
    lifecycle: Weak<()>,
    sample: Arc<Mutex<PlaybackSample>>,
    connected: Arc<AtomicBool>,
    endpoint: Arc<Mutex<String>>,
    timeline: Arc<Mutex<super::media_timeline::PreparedTimeline>>,
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
        loop {
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
                sequence += 1;
                if rpc.call("controller.media.playback.update",json!({"client_id":id,"sequence":sequence,"position_ms":current.position_now(),"duration_ms":current.duration_ms,"playing":false,"loaded":current.loaded,"rate":current.rate,"epoch":current.epoch})).is_err() {client=None;previous=None;continue}
                payload["client_id"] = json!(id);
                payload["revision"] = json!(revision);
                match rpc.call_detailed("controller.media.timeline.prepare", payload) {
                    Ok(feedback) => {
                        if let Ok(mut plan) = timeline.lock()
                            && plan.revision == revision
                        {
                            plan.acknowledged_revision = revision;
                            plan.feedback = feedback;
                            plan.error = None;
                            plan.deferred_reason = None;
                            plan.last_ack = None;
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
                                plan.error = None;
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
                sequence += 1;
                let result = rpc.call("controller.media.playback.update",json!({
                    "client_id":id,"sequence":sequence,"position_ms":current.position_now(),
                    "duration_ms":current.duration_ms,"playing":current.playing,"loaded":current.loaded,"rate":current.rate,"epoch":current.epoch,"plan_revision":revision}));
                match result {
                    Ok(feedback) => {
                        if let Ok(mut plan) = timeline.lock() {
                            if feedback["sequence"].as_u64() != Some(sequence)
                                || feedback["client_id"] != id
                                || feedback["epoch"].as_u64() != Some(current.epoch)
                                || feedback["plan_revision"].as_u64().unwrap_or(0) != revision
                            {
                                plan.error =
                                    Some("Hardware clock echo mismatch; playback paused".into());
                                plan.play_requested = false;
                            } else {
                                plan.last_ack = Some(Instant::now());
                                plan.clock_ack_revision = revision;
                                plan.clock_ack_epoch = current.epoch;
                                plan.feedback = feedback["timeline"].clone();
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
}
