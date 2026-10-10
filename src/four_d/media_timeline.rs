//! Volatile acknowledged plans; PCController remains the effect library owner.
use super::{curve::AnalogTrack, engine, models::Timeline};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

pub struct PreparedTimeline {
    pub authority: Option<super::authority::Status>,
    pub authority_client_id: String,
    pub revision: u64,
    pub payload: Value,
    pub acknowledged_revision: u64,
    pub clock_ack_revision: u64,
    pub clock_ack_epoch: u64,
    pub feedback: Value,
    pub error: Option<String>,
    pub deferred_reason: Option<String>,
    pub compilation_error: Option<String>,
    /// A timing discontinuity invalidated the controller's previous arm. Keep
    /// the media paused until an explicit Play requests a new prepare/ack cycle.
    pub requires_reprepare: bool,
    pub last_ack: Option<Instant>,
    pub play_requested: bool,
}
impl Default for PreparedTimeline {
    fn default() -> Self {
        Self {
            authority: None,
            authority_client_id: crate::platform::interop::controller_instance_id(),
            revision: 0,
            payload: json!({"cues":[],"actions":[],"max_lateness_ms":50}),
            acknowledged_revision: 0,
            clock_ack_revision: 0,
            clock_ack_epoch: 0,
            feedback: Value::Null,
            error: None,
            deferred_reason: None,
            compilation_error: None,
            requires_reprepare: false,
            last_ack: None,
            play_requested: false,
        }
    }
}
impl PreparedTimeline {
    pub fn update_authority(&mut self, status: super::authority::Status) -> bool {
        let changed = self.authority.as_ref().map(|value| value.owner_id.as_str())
            != Some(status.owner_id.as_str());
        if changed {
            self.acknowledged_revision = 0;
            self.clock_ack_revision = 0;
            self.clock_ack_epoch = 0;
            self.last_ack = None;
            self.play_requested = false;
            self.feedback = Value::Null;
            self.error = self.compilation_error.clone();
            self.deferred_reason = None;
            self.revision = self.revision.saturating_add(1);
        }
        self.authority = Some(status);
        changed
    }
    pub fn may_publish(&self) -> bool {
        // Identity is fixed when the engine is created (or supplied by the
        // authority for a remote consumer). Re-resolving it here reloads and
        // validates the entire config under this lock on every clock sample.
        self.authority.as_ref().is_none_or(|authority|authority.may_publish(&self.authority_client_id))
    }
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
                self.deferred_reason = None;
                self.requires_reprepare = false;
                self.last_ack = None;
            }
            Err(error) => {
                if self.compilation_error.as_ref() != Some(&error) {
                    self.revision = self.revision.saturating_add(1);
                }
                self.compilation_error = Some(error.clone());
                self.error = Some(error);
                self.deferred_reason = None;
                self.requires_reprepare = false;
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
            && self.deferred_reason.is_none()
            && self.compilation_error.is_none()
            && self.feedback["armed_epoch"].as_u64() == Some(epoch)
            && matches!(self.feedback["state"].as_str(), Some("paused" | "playing"))
    }

    /// Explicit Play may retry a cancelled/faulted controller executor, but
    /// must obtain a new revision and arm acknowledgement before unpausing.
    /// Reusing the old revision only returns its terminal state on the peer.
    pub fn request_play(&mut self) -> bool {
        if self.compilation_error.is_some() {
            self.play_requested = false;
            return false;
        }
        let reprepare = self.requires_reprepare
            || matches!(self.feedback["state"].as_str(), Some("stopped" | "faulted"));
        if reprepare {
            self.revision = self.revision.saturating_add(1);
            self.acknowledged_revision = 0;
            self.clock_ack_revision = 0;
            self.clock_ack_epoch = 0;
            self.last_ack = None;
            self.error = None;
            self.deferred_reason = None;
            self.requires_reprepare = false;
            self.feedback = Value::Null;
        }
        self.play_requested = true;
        reprepare
    }
}

pub fn compile_plan(
    timeline: &Timeline,
    relays: &[engine::CompiledAction],
    motions: &[engine::CompiledMotionAction],
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
    actions.extend(motions.iter().enumerate().map(|(index, edge)| {
        json!({
            "id": format!("motion-{index}"),
            "time_ms": edge.time_ms,
            "step": {"kind":"motion", "target":edge.side, "value":edge.motion}
        })
    }));
    let direct_pwm = engine::compile_direct_pwm_cues(timeline);
    let solo = analog.iter().any(|track| track.enabled && track.soloed);
    let pwm_channels = analog
        .iter()
        .map(|track| track.channel)
        .chain(direct_pwm.iter().map(|cue| cue.channel))
        .collect::<std::collections::BTreeSet<_>>();
    for channel in pwm_channels {
        // Prepared playback needs an explicit zero just like the live engine;
        // omitting a muted/solo-excluded track leaves its old value latched.
        if analog.iter().rev().find(|track| track.channel == channel)
            .is_some_and(|track| !track.allows_output(solo)) {
            actions.push(json!({"id":format!("pwm-{channel}-muted"),"time_ms":0,
                "step":{"kind":"pwm","target":channel,"value":0}}));
            continue;
        }
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
        if last_curve_time / 34 > 200_000 {
            return Err("PWM timeline exceeds prepared sample capacity; shorten or split it".into());
        }
        let mut times = Vec::new();
        if track.is_some() {
            times.extend((0..=last_curve_time / 34).map(|sample| sample * 34));
            times.push(last_curve_time);
        }
        for cue in &channel_cues {
            times.push(cue.start_time_ms);
            if cue.behavior != super::models::DirectCueBehavior::SetKeep { times.push(cue.end_time_ms); }
            if cue.behavior == super::models::DirectCueBehavior::Ramp {
                let samples = cue.end_time_ms.saturating_sub(cue.start_time_ms) / 34;
                if samples > 200_000 || times.len() as u64 + samples > 200_000 {
                    return Err("PWM timeline exceeds prepared sample capacity; shorten or split it".into());
                }
                times.extend((0..=samples).map(|sample| cue.start_time_ms.saturating_add(sample * 34)));
            }
        }
        times.sort_unstable();
        times.dedup();
        let mut previous = None;
        for at in times {
            let direct = channel_cues.iter().filter(|cue| at >= cue.start_time_ms)
                .max_by_key(|cue| (cue.priority_at(at), cue.start_time_ms));
            if direct.is_none() && track.is_none() { continue; }
            let value = direct.map_or_else(
                || track.map_or(0, |track| (track.evaluate(at) * 4095.0).round() as u16),
                |cue| {
                    ((u32::from(cue.value_at(at)) * 4095 + 5_000) / 10_000) as u16
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
    let connected = handle.is_connected.clone();
    std::thread::spawn(move || {
        let mut previous_path = String::new();
        let mut seeking = false;
        let source = crate::mpv::player::Player(mpv);
        let mut external_seek_revision = 0;
        while lifecycle.strong_count() > 0 {
            let unattended = crate::platform::interop::allow_unattended_hardware_takeover();
            let mut restarted = false;
            let external = crate::mpv::external::active();
            for _ in 0..128 {
                match client.wait_event(0.0) {
                    Some(Ok(libmpv2::events::Event::Seek)) if !external => seeking = true,
                    Some(Ok(libmpv2::events::Event::PlaybackRestart)) if !external => {
                        restarted = seeking;
                        seeking = false;
                    }
                    Some(_) => {}
                    None => break,
                }
            }
            if external {
                let revision = crate::mpv::external::seek_revision();
                restarted |= revision != external_seek_revision;
                external_seek_revision = revision;
                seeking = source.get_property::<bool>("seeking").unwrap_or(false);
            }
            let path = source.get_property::<String>("path").unwrap_or_default();
            let position = source
                .get_property::<f64>("time-pos")
                .ok()
                .filter(|value| value.is_finite());
            let loaded = !path.is_empty() && position.is_some();
            let paused = source.get_property::<bool>("pause").unwrap_or(true);
            let eof = source.get_property::<bool>("eof-reached").unwrap_or(false);
            let buffering = seeking
                || source
                    .get_property::<bool>("paused-for-cache")
                    .unwrap_or(false);
            let rate = source
                .get_property::<f64>("speed")
                .unwrap_or(1.0)
                .clamp(0.25, 4.0);
            let duration = source
                .get_property::<f64>("duration")
                .ok()
                .filter(|value| value.is_finite() && *value > 0.0);
            let coordinator = connected.load(std::sync::atomic::Ordering::Acquire)
                && plan.lock().is_ok_and(|plan|plan.may_publish())
                && endpoint
                    .lock()
                    .is_ok_and(|value| super::controller::is_controller_endpoint(&value));
            let mut playing = loaded
                && !paused
                && !eof
                && !buffering
                && !estop.load(std::sync::atomic::Ordering::Acquire);
            let mut set_pause = None;
            let handoff_pause = plan.lock().is_ok_and(|plan| plan.authority.as_ref()
                .is_some_and(|state| state.unattended_claim(&plan.authority_client_id, unattended).is_some()));
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
                    prepared.requires_reprepare = true;
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
                        prepared.deferred_reason = None;
                        prepared.requires_reprepare = false;
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
                            prepared.requires_reprepare = true;
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
                if handoff_pause {
                    // This is a real libmpv pause, not a fabricated paused clock.
                    // The publisher accepts only after observing and ACKing it.
                    set_pause = Some(true);
                    if let Ok(mut prepared) = plan.lock() { prepared.play_requested = false; }
                }
            }
            // Never hold shared state while waiting for an mpv command.
            if let Some(paused) = set_pause {
                let _ = source.set_property("pause", paused);
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
    fn prepared_strip_window_retains_its_reference_and_authored_duration() {
        use super::super::models::{Effect, EffectInstance};
        let mut timeline = Timeline::default();
        let mut strip = Effect::controller_strip_effect("Stream".into(), 2_000, "advertised-strip".into());
        strip.duration_ms = 8_000;
        let id = strip.id;
        timeline.templates.push(strip);
        timeline.instances.push(EffectInstance::new(id, 10_000));
        let plan = compile_plan(&timeline, &[], &[], &[]).unwrap();
        assert_eq!(plan["cues"][0]["reference"], "effect:advertised-strip");
        assert_eq!(plan["cues"][0]["time_ms"], 10_000);
        assert_eq!(plan["cues"][0]["duration_ms"], 8_000);
    }

    #[test]
    fn publication_uses_the_prepared_engines_stable_identity() {
        let mut plan = PreparedTimeline::default();
        plan.authority_client_id = "publisher:stable-engine".into();
        plan.authority = Some(super::super::authority::Status {
            owner_id: plan.authority_client_id.clone(), owner_label: "Publisher".into(),
            exclusive: false, revision: 1, pending: vec![], owner_endpoint: None,
        });
        assert!(plan.may_publish());
        plan.authority.as_mut().unwrap().exclusive = true;
        assert!(plan.may_publish());
        plan.authority.as_mut().unwrap().owner_id = "another-publisher".into();
        assert!(!plan.may_publish());
        plan.authority.as_mut().unwrap().owner_id.clear();
        assert!(!plan.may_publish());
    }

    #[test]
    fn authority_handoff_invalidates_old_arm_but_metadata_refresh_does_not() {
        let status = super::super::authority::Status {
            owner_id: "owner".into(), owner_label: "Publisher".into(),
            exclusive: false, revision: 1, pending: vec![], owner_endpoint: None,
        };
        let mut plan = PreparedTimeline::default();
        plan.authority = Some(status.clone());
        plan.acknowledged_revision = 7;
        plan.clock_ack_revision = 7;
        plan.clock_ack_epoch = 4;
        plan.last_ack = Some(Instant::now());
        plan.play_requested = true;
        let mut refresh = status;
        refresh.owner_endpoint = Some("http://publisher.example:8080/".into());
        assert!(!plan.update_authority(refresh.clone()));
        assert_eq!(plan.clock_ack_revision, 7);
        refresh.owner_id = "requester".into();
        assert!(plan.update_authority(refresh));
        assert_eq!(plan.acknowledged_revision, 0);
        assert_eq!(plan.clock_ack_revision, 0);
        assert_eq!(plan.clock_ack_epoch, 0);
        assert!(plan.last_ack.is_none());
        assert!(!plan.play_requested);
    }
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
        assert!(!plan.request_play());
        assert!(!plan.play_requested);
        assert!(plan.compilation_error.is_some());
    }

    #[test]
    fn explicit_play_reprepares_a_stopped_controller_without_reusing_its_arm() {
        let mut plan = PreparedTimeline::default();
        plan.replace(Ok(json!({"cues":[],"actions":[{"id":"a"}]})));
        plan.acknowledged_revision = 1;
        plan.clock_ack_revision = 1;
        plan.clock_ack_epoch = 3;
        plan.feedback = json!({"state":"stopped","armed_epoch":3});
        plan.last_ack = Some(Instant::now());

        assert!(plan.request_play());
        assert_eq!(plan.revision, 2);
        assert_eq!(plan.acknowledged_revision, 0);
        assert_eq!(plan.clock_ack_revision, 0);
        assert_eq!(plan.clock_ack_epoch, 0);
        assert!(plan.last_ack.is_none());
        assert!(plan.play_requested);
        assert!(!plan.ready_for(3));
        // Repeated Play while preparation is pending must not churn revisions.
        assert!(!plan.request_play());
        assert_eq!(plan.revision, 2);
    }

    #[test]
    fn explicit_play_retries_faults_but_not_a_healthy_paused_executor() {
        let mut plan = PreparedTimeline::default();
        plan.replace(Ok(json!({"cues":[],"actions":[{"id":"a"}]})));
        plan.feedback = json!({"state":"paused"});
        assert!(!plan.request_play());
        assert_eq!(plan.revision, 1);
        plan.feedback = json!({"state":"faulted"});
        plan.error = Some("executor cancelled".into());
        assert!(plan.request_play());
        assert_eq!(plan.revision, 2);
        assert!(plan.error.is_none());
    }
    #[test]
    fn retryable_resource_wait_is_distinct_from_a_timing_fault() {
        let mut plan = PreparedTimeline::default();
        plan.replace(Ok(json!({"cues":[],"actions":[{"id":"a"}]})));
        plan.deferred_reason = Some("addressable strip is busy".into());
        assert!(plan.error.is_none());
        assert!(!plan.ready_for(1));
        plan.replace(Ok(json!({"cues":[],"actions":[{"id":"b"}]})));
        assert!(plan.deferred_reason.is_none());
    }

    #[test]
    fn epoch_transition_requires_new_arming_feedback() {
        let mut plan = PreparedTimeline::default();
        let value = json!({"cues":[],"actions":[{"id":"a"}],"max_lateness_ms":50});
        plan.replace(Ok(value));
        plan.acknowledged_revision = 1;
        plan.clock_ack_revision = 1;
        plan.clock_ack_epoch = 1;
        plan.feedback = json!({"state":"paused","armed_epoch":1});
        assert!(plan.ready_for(1));

        // When epoch increments to 2 (e.g. after a seek or media switch)
        assert!(!plan.ready_for(2));

        // Feedback in faulted state must not be ready
        plan.feedback = json!({"state":"faulted","armed_epoch":2,"error":"hardware timeline is not armed for this media epoch"});
        plan.clock_ack_epoch = 2;
        assert!(!plan.ready_for(2));

        // Proper arming restores ready_for
        plan.feedback = json!({"state":"paused","armed_epoch":2});
        assert!(plan.ready_for(2));
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
        let plan = compile_plan(&timeline, &[], &[], &[]).expect("direct PWM plan");
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

    #[test]
    fn prepared_plan_keeps_semantic_motion_steps_out_of_raw_relay_commands() {
        let timeline = Timeline::new();
        let motions = vec![
            engine::CompiledMotionAction {
                time_ms: 750,
                side: 1,
                motion: 2,
            },
            engine::CompiledMotionAction {
                time_ms: 1_750,
                side: 1,
                motion: 0,
            },
        ];
        let plan = compile_plan(&timeline, &[], &motions, &[]).unwrap();
        assert_eq!(plan["actions"][0]["step"]["kind"], "motion");
        assert_eq!(plan["actions"][0]["step"]["target"], 1);
        assert_eq!(plan["actions"][0]["step"]["value"], 2);
        assert_eq!(plan["actions"][1]["step"]["value"], 0);
    }

    #[test]
    fn persistent_pwm_at_a_late_position_is_a_single_command_without_zero_or_exit() {
        let mut timeline = Timeline::new();
        let mut effect = Effect::direct_control("Keep".into(), String::new(), 1000, "pwm.3".into(), 5000, None);
        effect.direct_control.as_mut().unwrap().behavior = super::super::models::DirectCueBehavior::SetKeep;
        let id = effect.id;
        timeline.templates.push(effect);
        timeline.instances.push(EffectInstance::new(id, 86_000_000));
        let plan = compile_plan(&timeline, &[], &[], &[]).unwrap();
        let actions = plan["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0]["time_ms"], 86_000_000);
        assert_eq!(actions[0]["step"]["value"], 2048);
    }

    #[test]
    fn pwm_ramp_is_prepared_with_exact_start_and_end_values() {
        let mut timeline = Timeline::new();
        let mut effect = Effect::direct_control("Ramp".into(), String::new(), 1000, "pwm.3".into(), 0, None);
        let cue = effect.direct_control.as_mut().unwrap();
        cue.behavior = super::super::models::DirectCueBehavior::Ramp;
        cue.end_value_basis_points = 10_000;
        let id = effect.id;
        timeline.templates.push(effect);
        timeline.instances.push(EffectInstance::new(id, 10_000));
        let plan = compile_plan(&timeline, &[], &[], &[]).unwrap();
        let actions = plan["actions"].as_array().unwrap();
        assert_eq!(actions.first().unwrap()["time_ms"], 10_000);
        assert_eq!(actions.first().unwrap()["step"]["value"], 0);
        assert_eq!(actions.last().unwrap()["time_ms"], 11_000);
        assert_eq!(actions.last().unwrap()["step"]["value"], 4095);
        assert!(actions.iter().any(|action| action["step"]["value"].as_u64().is_some_and(|value| value > 0 && value < 4095)));
    }

    #[test]
    fn prepared_muted_pwm_track_explicitly_clears_latched_output() {
        let mut track = AnalogTrack::new("House light", 3);
        track.muted = true;
        let mut timeline = Timeline::new();
        timeline.analog_tracks.push(track.clone());
        let plan = compile_plan(&timeline, &[], &[], &[track]).unwrap();
        let actions = plan["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0]["time_ms"], 0);
        assert_eq!(actions[0]["step"]["target"], 3);
        assert_eq!(actions[0]["step"]["value"], 0);
    }
}
