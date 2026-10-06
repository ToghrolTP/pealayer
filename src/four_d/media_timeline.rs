//! Volatile acknowledged plans; PCController remains the effect library owner.
use super::{curve::AnalogTrack, engine, models::Timeline};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

pub struct PreparedTimeline {
    pub revision: u64,
    pub payload: Value,
    pub acknowledged_revision: u64,
    pub clock_ack_revision: u64,
    pub clock_ack_epoch: u64,
    pub feedback: Value,
    pub error: Option<String>,
    pub compilation_error: Option<String>,
    pub last_ack: Option<Instant>,
    pub play_requested: bool,
}
impl Default for PreparedTimeline {
    fn default() -> Self {
        Self {
            revision: 0,
            payload: json!({"cues":[],"actions":[],"max_lateness_ms":50}),
            acknowledged_revision: 0,
            clock_ack_revision: 0,
            clock_ack_epoch: 0,
            feedback: Value::Null,
            error: None,
            compilation_error: None,
            last_ack: None,
            play_requested: false,
        }
    }
}
impl PreparedTimeline {
    pub fn has_items(&self) -> bool {
        self.revision != self.acknowledged_revision
            || self.compilation_error.is_some()
            || ["cues", "actions"].iter().any(|key| {
                self.payload[key]
                    .as_array()
                    .is_some_and(|items| !items.is_empty())
            })
    }
    pub fn replace(&mut self, compiled: Result<Value, String>) {
        match compiled {
            Ok(payload) if payload != self.payload || self.compilation_error.is_some() => {
                self.payload = payload;
                self.revision = self.revision.saturating_add(1);
                self.compilation_error = None;
                self.error = None;
                self.last_ack = None;
            }
            Err(error) => {
                if self.compilation_error.as_ref() != Some(&error) {
                    self.revision = self.revision.saturating_add(1);
                }
                self.compilation_error = Some(error.clone());
                self.error = Some(error);
                self.last_ack = None;
            }
            _ => {}
        }
    }
    pub fn ready_for(&self, epoch: u64) -> bool {
        self.revision == self.acknowledged_revision
            && self.revision == self.clock_ack_revision
            && self.clock_ack_epoch == epoch
            && self.error.is_none()
            && self.compilation_error.is_none()
            && self.feedback["armed_epoch"].as_u64() == Some(epoch)
            && matches!(self.feedback["state"].as_str(), Some("paused" | "playing"))
    }
}

pub fn compile_plan(
    timeline: &Timeline,
    relays: &[engine::CompiledAction],
    analog: &[AnalogTrack],
) -> Result<Value, String> {
    let mut cues = Vec::new();
    for instance in &timeline.instances {
        let Some(effect) = timeline
            .templates
            .iter()
            .find(|item| item.id == instance.effect_id)
        else {
            continue;
        };
        let lane = effect.controller_lane.unwrap_or_default();
        let state =
            timeline.track_state(&super::models::controller_effect_timeline_track_key(lane));
        if !state.linked {
            continue;
        }
        let reference = if let Some(sequence) = &effect.controller_macro {
            format!("effect:{}", sequence.id)
        } else if let Some(strip) = &effect.controller_strip_effect {
            format!("effect:{}", strip.id)
        } else {
            continue;
        };
        cues.push(json!({"id":instance.id.to_string(),"reference":reference,"time_ms":instance.start_time_ms,"duration_ms":effect.duration_ms}));
    }
    let mut actions = relays.iter().enumerate().map(|(i, edge)| json!({"id":format!("relay-{i}"),"time_ms":edge.time_ms,
        "step":{"kind":"relay","target":edge.relay_id.saturating_sub(1),"value":u8::from(edge.state)}})).collect::<Vec<_>>();
    let direct_pwm = engine::compile_direct_pwm_cues(timeline);
    let solo = analog.iter().any(|track| track.enabled && track.soloed);
    let pwm_channels = analog
        .iter()
        .filter(|track| track.allows_output(solo))
        .map(|track| track.channel)
        .chain(direct_pwm.iter().map(|cue| cue.channel))
        .collect::<std::collections::BTreeSet<_>>();
    for channel in pwm_channels {
        let track = analog
            .iter()
            .rev()
            .find(|track| track.channel == channel && track.allows_output(solo));
        let channel_cues = direct_pwm
            .iter()
            .filter(|cue| cue.channel == channel)
            .collect::<Vec<_>>();
        let last_curve_time = track
            .and_then(|track| track.keyframes.last())
            .map_or(0, |keyframe| keyframe.time_ms);
        let last_cue_time = channel_cues
            .iter()
            .map(|cue| cue.end_time_ms)
            .max()
            .unwrap_or(0);
        let end = last_curve_time.max(last_cue_time);
        if end / 34 > 200_000 {
            return Err("PWM timeline exceeds prepared sample capacity; shorten or split it".into());
        }
        let mut times = (0..=end / 34)
            .map(|sample| sample * 34)
            .collect::<Vec<_>>();
        if times.last().copied() != Some(end) {
            times.push(end);
        }
        for cue in &channel_cues {
            times.push(cue.start_time_ms);
            times.push(cue.end_time_ms);
        }
        times.sort_unstable();
        times.dedup();
        let mut previous = None;
        for at in times {
            let direct = channel_cues.iter().rev().find(|cue| {
                at >= cue.start_time_ms && at < cue.end_time_ms
            });
            let value = direct.map_or_else(
                || track.map_or(0, |track| (track.evaluate(at) * 4095.0).round() as u16),
                |cue| {
                    ((u32::from(cue.value_basis_points) * 4095 + 5_000) / 10_000) as u16
                },
            );
            if previous != Some(value) {
                actions.push(json!({"id":format!("pwm-{channel}-{at}"),"time_ms":at,
                    "step":{"kind":"pwm","target":channel,"value":value}}));
                previous = Some(value);
            }
            if actions.len() > 65535 {
                return Err("Timeline exceeds prepared action capacity; split the project".into());
            }
        }
    }
    if cues.len() > 4096 || actions.len() > 65535 {
        return Err("Timeline exceeds prepared cue/action capacity".into());
    }
    Ok(json!({"cues":cues,"actions":actions,"max_lateness_ms":50}))
}

/// libmpv observer independent of the egui event/render thread.
pub fn observe_mpv(handle: &engine::EngineHandle, mpv: &'static libmpv2::Mpv) {
    let Ok(client) = mpv.create_client(None) else {
        return;
    };
    handle
        .media_clock_owned
        .store(true, std::sync::atomic::Ordering::Release);
    let lifecycle = handle.playback_lifecycle();
    let sample = handle.media_playback.clone();
    let plan = handle.prepared_timeline.clone();
    let endpoint = handle.serial_port.clone();
    let estop = handle.estop_active.clone();
    let error = handle.connection_error.clone();
    std::thread::spawn(move || {
        let mut previous_path = String::new();
        let mut seeking = false;
        while lifecycle.strong_count() > 0 {
            let mut restarted = false;
            for _ in 0..128 {
                match client.wait_event(0.0) {
                    Some(Ok(libmpv2::events::Event::Seek)) => seeking = true,
                    Some(Ok(libmpv2::events::Event::PlaybackRestart)) => {
                        restarted = seeking;
                        seeking = false;
                    }
                    Some(_) => {}
                    None => break,
                }
            }
            let path = client.get_property::<String>("path").unwrap_or_default();
            let position = client
                .get_property::<f64>("time-pos")
                .ok()
                .filter(|value| value.is_finite());
            let loaded = !path.is_empty() && position.is_some();
            let paused = client.get_property::<bool>("pause").unwrap_or(true);
            let eof = client.get_property::<bool>("eof-reached").unwrap_or(false);
            let buffering = seeking
                || client
                    .get_property::<bool>("paused-for-cache")
                    .unwrap_or(false);
            let rate = client
                .get_property::<f64>("speed")
                .unwrap_or(1.0)
                .clamp(0.25, 4.0);
            let duration = client
                .get_property::<f64>("duration")
                .ok()
                .filter(|value| value.is_finite() && *value > 0.0);
            let coordinator = endpoint
                .lock()
                .is_ok_and(|value| super::controller::is_controller_endpoint(&value));
            let mut playing = loaded
                && !paused
                && !eof
                && !buffering
                && !estop.load(std::sync::atomic::Ordering::Acquire);
            let mut set_pause = None;
            if let Ok(mut clock) = sample.lock() {
                let next = position
                    .unwrap_or(clock.position_ms as f64 / 1000.)
                    .max(0.0)
                    * 1000.0;
                let discontinuity =
                    loaded && !buffering && (next - clock.position_now() as f64).abs() > 250.0;
                let rebase = path != previous_path || restarted;
                // Clock drift/stalls are not deliberate seeks. Never clear a
                // timing fault or skip past cues merely because the clock jumped.
                if discontinuity
                    && !rebase
                    && coordinator
                    && let Ok(mut prepared) = plan.lock()
                    && prepared.has_items()
                {
                    prepared.error = Some(
                        "Media clock discontinuity without a seek; hardware playback paused".into(),
                    );
                    prepared.play_requested = false;
                }
                if rebase {
                    clock.epoch = clock.epoch.saturating_add(1);
                    if coordinator
                        && let Ok(mut prepared) = plan.lock()
                        && prepared.has_items()
                    {
                        prepared.revision = prepared.revision.saturating_add(1);
                        prepared.error = prepared.compilation_error.clone();
                        prepared.last_ack = None;
                        prepared.play_requested |= playing;
                    }
                }
                previous_path = path;
                clock.position_ms = next.round() as u64;
                clock.duration_ms = duration.map(|value| (value * 1000.0).round() as u64);
                clock.loaded = loaded;
                clock.buffering = buffering;
                clock.rate = rate;
                clock.sampled_at = Instant::now();
                clock.observed_at = Instant::now();
                if coordinator
                    && let Ok(mut prepared) = plan.lock()
                    && prepared.has_items()
                {
                    let fresh = prepared
                        .last_ack
                        .is_some_and(|ack| ack.elapsed() < Duration::from_millis(250));
                    let ready = prepared.ready_for(clock.epoch);
                    if playing && (!ready || !fresh) {
                        if prepared.error.is_none() && !ready {
                            prepared.play_requested = true;
                        }
                        if ready && !fresh {
                            prepared.error = Some("Hardware clock acknowledgement expired; playback paused. Seek and reprepare before retrying.".into());
                            prepared.play_requested = false;
                        }
                        set_pause = Some(true);
                        playing = false;
                    }
                    if prepared.play_requested && ready && fresh && !eof && !buffering {
                        set_pause = Some(false);
                        playing = true;
                        prepared.play_requested = false;
                    }
                    if let Some(message) = prepared.error.clone()
                        && let Ok(mut value) = error.lock()
                    {
                        *value = Some(message)
                    }
                }
                clock.playing = playing;
            }
            // Never hold shared state while waiting for an mpv command.
            if let Some(paused) = set_pause {
                let _ = client.set_property("pause", paused);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::four_d::models::{Effect, EffectInstance, Timeline};
    #[test]
    fn revision_is_content_driven_and_requires_clock_arming_ack() {
        let mut plan = PreparedTimeline::default();
        let value = json!({"cues":[],"actions":[{"id":"a"}],"max_lateness_ms":50});
        plan.replace(Ok(value.clone()));
        assert_eq!(plan.revision, 1);
        plan.replace(Ok(value));
        assert_eq!(plan.revision, 1);
        plan.acknowledged_revision = 1;
        assert!(!plan.ready_for(2));
        plan.clock_ack_revision = 1;
        plan.clock_ack_epoch = 2;
        plan.feedback = json!({"state":"paused","armed_epoch":2});
        assert!(plan.ready_for(2));
        assert!(!plan.ready_for(3));
        plan.error = Some("late".into());
        assert!(!plan.ready_for(2));
    }
    #[test]
    fn compile_failure_stays_fail_closed() {
        let mut plan = PreparedTimeline::default();
        plan.replace(Err("too many actions".into()));
        assert!(plan.has_items());
        assert!(!plan.ready_for(1));
    }

    #[test]
    fn prepared_plan_contains_exact_direct_pwm_start_and_release() {
        let mut timeline = Timeline::new();
        let effect = Effect::direct_control(
            "PWM cue".into(),
            String::new(),
            1_000,
            "pwm.3".into(),
            5_000,
            None,
        );
        let effect_id = effect.id;
        timeline.templates.push(effect);
        timeline.instances.push(EffectInstance::new(effect_id, 10_000));
        let plan = compile_plan(&timeline, &[], &[]).expect("direct PWM plan");
        let actions = plan["actions"].as_array().expect("prepared actions");
        assert!(actions.iter().any(|action| {
            action["time_ms"] == 10_000
                && action["step"]["target"] == 3
                && action["step"]["value"] == 2_048
        }));
        assert!(actions.iter().any(|action| {
            action["time_ms"] == 11_000
                && action["step"]["target"] == 3
                && action["step"]["value"] == 0
        }));
    }
}
