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
        }
    }
}
impl PlaybackSample {
    fn position_now(&self) -> u64 {
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
        self.loaded != previous.loaded
            || self.playing != previous.playing
            || self.duration_ms != previous.duration_ms
            || self.rate != previous.rate
            || self.position_now().abs_diff(previous.position_now()) > 80
    }
}

pub fn spawn(
    lifecycle: Weak<()>,
    sample: Arc<Mutex<PlaybackSample>>,
    connected: Arc<AtomicBool>,
    endpoint: Arc<Mutex<String>>,
) {
    std::thread::spawn(move || {
        let id = format!("pealayer:{}", crate::messaging::snapshot().instance_id);
        let mut client: Option<ControllerClient> = None;
        let mut active_endpoint = String::new();
        let mut sequence = 0_u64;
        let mut previous: Option<PlaybackSample> = None;
        let mut sent_at = Instant::now();
        let mut reported_at = Instant::now();
        let mut retry_at = Instant::now();
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
            let interval = if current.playing {
                Duration::from_millis(100)
            } else {
                Duration::from_secs(1)
            };
            let due = previous
                .as_ref()
                .is_none_or(|old| current.changed_from(old))
                || sent_at.elapsed() >= interval;
            if due && let Some(ref mut rpc) = client {
                sequence += 1;
                let result = rpc.call("controller.media.playback.update",json!({
                    "client_id":id,"sequence":sequence,"position_ms":current.position_now(),
                    "duration_ms":current.duration_ms,"playing":current.playing,"loaded":current.loaded,"rate":current.rate}));
                match result {
                    Ok(_) => {
                        previous = Some(current.clone());
                        sent_at = Instant::now();
                        last_error.clear();
                    }
                    Err(error) => {
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
    json!({"id":id,"surface":"pealayer","state":"active","lease_seconds":30,
        "self":{"kind":"process","pid":std::process::id()},
        "values":{"application":name,"version":env!("CARGO_PKG_VERSION"),
            "commit":env!("PEALAYER_GIT_COMMIT"),"os":std::env::consts::OS,"arch":std::env::consts::ARCH}})
}
#[cfg(test)]
mod tests {
    use super::*;
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
}
