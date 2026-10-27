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
    /// Keep the paused reservation intent until libmpv actually reports Play.
    pub play_starting: bool,
    pub play_intent_revision: u64,
    play_ready_intent: Option<u64>,
    pub clock_transport: super::media_sync::ClockTransportDiagnostics,
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
            play_starting: false,
            play_intent_revision: 0,
            play_ready_intent: None,
            clock_transport: Default::default(),
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
            self.cancel_play();
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
            || self.has_outputs()
    }
    pub fn has_outputs(&self) -> bool {
        ["cues", "actions"].iter().any(|key| {
                self.payload[key]
                    .as_array()
                    .is_some_and(|items| !items.is_empty())
            })
    }
    pub fn presentation_label(&self) -> &'static str {
        if self.compilation_error.is_some() { "Hardware plan invalid" }
        else if self.error.is_some() && (self.requires_reprepare || (self.clock_ack_revision == self.revision && self.feedback["state"] == "faulted")) { "Hardware timing fault" }
        else if self.error.is_some() { "Hardware preparation blocked" }
        else if !self.may_publish() { "Hardware authority required" }
        else if self.deferred_reason.is_some() { "Hardware waiting" }
        else if self.revision != self.acknowledged_revision { "Preparing hardware" }
        else if self.play_requested { "Arming hardware" }
        else { "Hardware timeline" }
    }
    /// A heartbeat for an old plan cannot repair compilation or preparation of
    /// the desired plan. Timing faults also require explicit re-arming.
    pub fn observe_clock_ack(&mut self, revision: u64, epoch: u64, feedback: Value,
        intent_revision: u64, sent_at: Instant) {
        if revision == self.revision && self.has_outputs() && !feedback["play_ready"].is_boolean() {
            self.error = Some("Invalid hardware acknowledgement: missing play readiness".into());
            self.requires_reprepare = true;
            self.last_ack = None;
            self.cancel_play();
            return;
        }
        // Time spent waiting for a reply cannot renew an old readiness lease.
        self.last_ack = Some(sent_at);
        self.clock_ack_revision = revision;
        self.clock_ack_epoch = epoch;
        self.play_ready_intent = (feedback["play_ready"] == true).then_some(intent_revision);
        self.feedback = feedback;
        if revision == self.revision && self.compilation_error.is_none()
            && self.deferred_reason.is_none() && !self.requires_reprepare {
            self.error = None;
        }
        if revision == self.revision && self.feedback["state"] == "faulted" {
            if self.error.is_none() {
                self.error = Some(self.feedback["error"].as_str()
                    .unwrap_or("Hardware deadline failure").to_owned());
            }
            self.requires_reprepare = true;
            self.cancel_play();
        }
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
                self.play_starting = false;
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
                self.play_starting = false;
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
            && if self.has_outputs() {
                self.feedback["armed_epoch"].as_u64() == Some(epoch)
                    && matches!(self.feedback["state"].as_str(), Some("paused" | "playing"))
            } else {
                // Empty plans clear old outputs but have no executor to arm.
                self.revision != 0 && self.feedback["revision"].as_u64() == Some(self.revision)
                    && self.feedback["step_count"].as_u64() == Some(0)
                    && self.feedback["state"].as_str() == Some("ready")
            }
    }
    pub fn consume_ready_play(&mut self, epoch: u64, loaded: bool, eof: bool, buffering: bool) -> bool {
        if self.play_requested && !self.play_starting && loaded && !eof && !buffering && self.ready_to_play(epoch)
            && self.last_ack.is_some_and(|ack| ack.elapsed() < Duration::from_millis(250)) {
            self.play_starting = true;
            true
        } else { false }
    }

    pub fn ready_to_play(&self, epoch: u64) -> bool {
        !self.requires_reprepare && self.ready_for(epoch)
            && (!self.has_outputs() || (self.feedback["play_ready"] == true
                && self.play_ready_intent == Some(self.play_intent_revision)))
    }

    pub fn prepare_play_intent(&self, epoch: u64, loaded: bool, playing: bool, buffering: bool) -> bool {
        self.play_requested && loaded && !playing && !buffering && self.has_outputs()
            && self.ready_for(epoch) && !self.requires_reprepare
    }

    pub fn cancel_play(&mut self) {
        self.play_intent_revision = self.play_intent_revision.saturating_add(1);
        self.play_ready_intent = None;
        self.play_requested = false;
        self.play_starting = false;
        if let Some(feedback) = self.feedback.as_object_mut() {
            feedback.insert("play_ready".into(), Value::Bool(false));
        }
    }

    pub fn observe_playing(&mut self) {
        if self.play_starting {
            self.play_requested = false;
            self.play_starting = false;
        }
    }

    /// Explicit Play may retry a cancelled/faulted controller executor, but
    /// must obtain a new revision and arm acknowledgement before unpausing.
    /// Reusing the old revision only returns its terminal state on the peer.
    pub fn request_play(&mut self) -> bool {
        if self.compilation_error.is_some() {
            self.cancel_play();
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
        self.play_starting = false;
        self.play_intent_revision = self.play_intent_revision.saturating_add(1);
        self.play_ready_intent = None;
        self.play_requested = true;
        reprepare
    }
}

fn update_observer_error(shared: &mut Option<String>, reported: &mut Option<String>, next: Option<String>) {
    if next != *reported {
        if next.is_some() || *shared == *reported { *shared = next.clone(); }
        *reported = next;
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
        let mut reported_error = None;
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
            let mut set_pause = None;
            let handoff_pause = plan.lock().is_ok_and(|plan| plan.authority.as_ref()
                .is_some_and(|state| state.unattended_claim(&plan.authority_client_id, unattended).is_some()));
            // Read position after the other potentially blocking properties and
            // locks. Keep separate position-anchor and freshness timestamps;
            // neither is the later point at which the sample lock is acquired.
            let observed_at = Instant::now();
            let position = source
                .get_property::<f64>("time-pos")
                .ok()
                .filter(|value| value.is_finite());
            let read_finished_at = Instant::now();
            let loaded = !path.is_empty() && position.is_some();
            let mut playing = loaded && !paused && !eof && !buffering
                && !estop.load(std::sync::atomic::Ordering::Acquire);
            if let Ok(mut clock) = sample.lock() {
                let next = position
                    .unwrap_or(clock.position_ms as f64 / 1000.)
                    .max(0.0)
                    * 1000.0;
                let projected_ms = clock.position_at(read_finished_at);
                let read_ms = read_finished_at.saturating_duration_since(observed_at).as_millis() as u64;
                let apply_age_ms = observed_at.elapsed().as_millis() as u64;
                clock.observation_timing.read_ms = read_ms;
                clock.observation_timing.apply_age_ms = apply_age_ms;
                clock.observation_timing.max_read_ms = clock.observation_timing.max_read_ms.max(read_ms);
                clock.observation_timing.max_apply_age_ms = clock.observation_timing.max_apply_age_ms.max(apply_age_ms);
                clock.observation_timing.delta_at_read_ms = next.round() as i64 - projected_ms as i64;
                let discontinuity = loaded && !buffering && clock.discontinuity_at(next.round() as u64, read_finished_at);
                let slow_read = playing && !super::media_sync::PlaybackSample::position_read_within_budget(observed_at, read_finished_at);
                let rebase = path != previous_path || restarted;
                if rebase {
                    clock.observation_timing.fault = None;
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
                        prepared.play_starting = false;
                        prepared.play_requested |= playing;
                    }
                }
                // A deliberate seek permits a new position; it cannot make a
                // slow native read precise. Neither case skips hardware cues.
                if ((discontinuity && !rebase) || slow_read)
                    && coordinator
                    && let Ok(mut prepared) = plan.lock()
                    && prepared.has_items()
                {
                    clock.observation_timing.fault.get_or_insert(super::media_sync::ObservationFault {
                        reason: if slow_read { "native-read-budget" } else { "discontinuity" },
                        position_ms: next.round() as u64, projected_ms, read_ms, apply_age_ms,
                    });
                    prepared.error = Some(if slow_read {
                        "Media clock observation was delayed; hardware playback paused".into()
                    } else {
                        "Media clock discontinuity without a seek; hardware playback paused".into()
                    });
                    prepared.requires_reprepare = true;
                    prepared.cancel_play();
                }
                previous_path = path;
                if position.is_some() {
                    clock.observe_position_read(next.round() as u64, playing, rate, rebase, observed_at, read_finished_at);
                }
                clock.duration_ms = duration.map(|value| (value * 1000.0).round() as u64);
                clock.loaded = loaded;
                clock.buffering = buffering;
                clock.rate = rate;
                if coordinator
                    && let Ok(mut prepared) = plan.lock()
                    && (prepared.has_items() || prepared.play_requested)
                {
                    if playing { prepared.observe_playing(); }
                    let fresh = prepared
                        .last_ack
                        .is_some_and(|ack| ack.elapsed() < Duration::from_millis(250));
                    let ready = prepared.ready_to_play(clock.epoch);
                    if playing && (!ready || !fresh) {
                        if prepared.error.is_none() && !ready {
                            prepared.play_requested = true;
                        }
                        if ready && !fresh {
                            prepared.error = Some("Hardware clock acknowledgement expired; playback paused. Press Play to prepare and acknowledge a new plan before retrying.".into());
                            prepared.requires_reprepare = true;
                            prepared.cancel_play();
                        }
                        set_pause = Some(true);
                        playing = false;
                    }
                    if prepared.consume_ready_play(clock.epoch, loaded, eof, buffering) {
                        set_pause = Some(false);
                        // Publish a paused preparation intent until a later mpv
                        // observation confirms that the native command worked.
                    }
                }
                clock.playing = playing;
                if handoff_pause {
                    // This is a real libmpv pause, not a fabricated paused clock.
                    // The publisher accepts only after observing and ACKing it.
                    set_pause = Some(true);
                    if let Ok(mut prepared) = plan.lock() { prepared.cancel_play(); }
                }
                // Clear only the diagnostic this observer previously installed,
                // never an independent connection error. Empty-plan completion
                // also passes here after leaving the hardware gating block.
                if let Ok(prepared) = plan.lock() {
                    let next = prepared.error.clone();
                    if next != reported_error && let Ok(mut value) = error.lock() {
                        update_observer_error(&mut value, &mut reported_error, next);
                    }
                }
            }
            // Never hold shared state while waiting for an mpv command.
            if let Some(paused) = set_pause {
                // A user Pause can cancel an acknowledged start before the
                // observer reaches the native command. Never revive that intent.
                if paused || plan.lock().is_ok_and(|plan| plan.play_requested && plan.play_starting) {
                    if let Err(failure) = source.set_property("pause", paused) {
                        if !paused && let Ok(mut prepared) = plan.lock() {
                            prepared.cancel_play();
                            prepared.error = Some(format!("Media could not start after hardware readiness: {failure}"));
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::four_d::models::{Effect, EffectInstance, Timeline};
    fn paused_output_plan() -> PreparedTimeline {
        let mut plan = PreparedTimeline::default();
        plan.replace(Ok(json!({"cues":[],"actions":[{"id":"seat"}]})));
        plan.acknowledged_revision = plan.revision;
        plan.observe_clock_ack(plan.revision, 4,
            json!({"state":"paused","armed_epoch":4,"play_ready":false}), plan.play_intent_revision, Instant::now());
        plan
    }

    #[test]
    fn paused_arm_does_not_start_media_before_serial_reservation_ack() {
        let mut plan = paused_output_plan();
        plan.request_play();
        assert!(plan.ready_for(4));
        assert!(plan.prepare_play_intent(4, true, false, false));
        assert!(!plan.consume_ready_play(4, true, false, false));
        plan.observe_clock_ack(plan.revision, 4,
            json!({"state":"paused","armed_epoch":4,"play_ready":true}), plan.play_intent_revision, Instant::now());
        assert!(!plan.consume_ready_play(5, true, false, false));
        assert!(plan.consume_ready_play(4, true, false, false));
        assert!(plan.play_requested);
        assert!(plan.play_starting);
        assert!(plan.prepare_play_intent(4, true, false, false));
        assert!(!plan.consume_ready_play(4, true, false, false));
        plan.observe_playing();
        assert!(!plan.play_requested);
        assert!(!plan.play_starting);
        assert!(!plan.prepare_play_intent(4, true, true, false));
    }

    #[test]
    fn cancel_wins_over_a_late_grant_and_requires_a_new_reservation() {
        let mut plan = paused_output_plan();
        plan.request_play();
        let cancelled_intent = plan.play_intent_revision;
        plan.cancel_play();
        plan.observe_clock_ack(plan.revision, 4,
            json!({"state":"paused","armed_epoch":4,"play_ready":true}), cancelled_intent, Instant::now());
        assert!(!plan.consume_ready_play(4, true, false, false));
        assert!(!plan.prepare_play_intent(4, true, false, false));
        // Cancel an already consumed grant in the native unpause gap too.
        plan.request_play();
        assert!(!plan.consume_ready_play(4, true, false, false));
        plan.observe_clock_ack(plan.revision, 4,
            json!({"state":"paused","armed_epoch":4,"play_ready":true}), plan.play_intent_revision, Instant::now());
        assert!(plan.consume_ready_play(4, true, false, false));
        plan.cancel_play();
        assert!(!plan.play_starting);
        plan.request_play();
        assert!(!plan.ready_to_play(4));
        assert!(!plan.consume_ready_play(4, true, false, false));
    }

    #[test]
    fn previous_play_grant_cannot_authorize_a_later_play_request() {
        let mut plan = paused_output_plan();
        plan.request_play();
        let old_intent = plan.play_intent_revision;
        plan.cancel_play();
        plan.request_play();
        plan.observe_clock_ack(plan.revision, 4,
            json!({"state":"paused","armed_epoch":4,"play_ready":true}), old_intent, Instant::now());
        assert!(!plan.ready_to_play(4));
        assert!(!plan.consume_ready_play(4, true, false, false));
        assert!(plan.prepare_play_intent(4, true, false, false));
    }

    #[test]
    fn missing_current_readiness_contract_fails_closed() {
        let mut plan = paused_output_plan();
        plan.request_play();
        plan.observe_clock_ack(plan.revision, 4, json!({"state":"paused","armed_epoch":4}), plan.play_intent_revision, Instant::now());
        assert!(plan.error.as_deref().unwrap().contains("missing play readiness"));
        assert!(!plan.play_requested);
        assert!(!plan.ready_to_play(4));
    }

    #[test]
    fn delayed_reservation_reply_does_not_renew_readiness_on_arrival() {
        let mut plan = paused_output_plan();
        plan.request_play();
        plan.observe_clock_ack(plan.revision, 4,
            json!({"state":"paused","armed_epoch":4,"play_ready":true}),
            plan.play_intent_revision, Instant::now() - Duration::from_millis(251));
        assert!(!plan.consume_ready_play(4, true, false, false));
        assert!(plan.prepare_play_intent(4, true, false, false));
        plan.observe_clock_ack(plan.revision, 4,
            json!({"state":"paused","armed_epoch":4,"play_ready":true}),
            plan.play_intent_revision, Instant::now());
        assert!(plan.consume_ready_play(4, true, false, false));
    }
    #[test]
    fn recovered_observer_diagnostic_clears_only_its_own_connection_error() {
        let mut shared = None;
        let mut reported = None;
        update_observer_error(&mut shared,&mut reported,Some("prepare failed".into()));
        assert_eq!(shared.as_deref(),Some("prepare failed"));
        update_observer_error(&mut shared,&mut reported,None);
        assert!(shared.is_none());
        update_observer_error(&mut shared,&mut reported,Some("prepare failed".into()));
        shared = Some("board disconnected".into());
        update_observer_error(&mut shared,&mut reported,None);
        assert_eq!(shared.as_deref(),Some("board disconnected"));
    }
    fn acknowledged_empty_plan() -> PreparedTimeline {
        let mut plan = PreparedTimeline::default();
        plan.revision = 2;
        plan.acknowledged_revision = 2;
        plan.observe_clock_ack(2, 3, json!({"revision":2,"state":"ready","step_count":0,"armed_epoch":0}), plan.play_intent_revision, Instant::now());
        plan
    }
    #[test]
    fn empty_clear_ack_releases_pending_play_without_an_executor() {
        let mut plan = acknowledged_empty_plan();
        plan.play_requested = true;
        assert!(!plan.has_items());
        assert!(plan.consume_ready_play(3, true, false, false));
        assert!(!plan.consume_ready_play(3, true, false, false));
    }
    #[test]
    fn empty_clear_still_requires_matching_revision_epoch_and_zero_steps() {
        let mut plan = acknowledged_empty_plan();
        plan.play_requested = true;
        plan.acknowledged_revision = 1;
        assert!(plan.has_items());
        assert!(!plan.consume_ready_play(3, true, false, false));
        plan.acknowledged_revision = 2;
        assert!(!plan.consume_ready_play(4, true, false, false));
        plan.feedback["revision"] = json!(1);
        assert!(!plan.ready_for(3));
        plan.feedback["revision"] = json!(2);
        plan.feedback["step_count"] = json!(1);
        assert!(!plan.ready_for(3));
        plan.feedback["step_count"] = Value::Null;
        assert!(!plan.ready_for(3));
    }
    #[test]
    fn pending_empty_play_never_resumes_unloaded_buffered_ended_or_stale_media() {
        let mut plan = acknowledged_empty_plan();
        plan.play_requested = true;
        for (loaded, eof, buffering) in [(false,false,false),(true,true,false),(true,false,true)] {
            assert!(!plan.consume_ready_play(3, loaded, eof, buffering));
        }
        plan.last_ack = Some(Instant::now() - Duration::from_millis(251));
        assert!(!plan.consume_ready_play(3, true, false, false));
        assert!(plan.play_requested);
    }
    #[test]
    fn old_plan_heartbeat_cannot_erase_new_preparation_or_compilation_failure() {
        let mut plan = acknowledged_empty_plan();
        plan.revision = 3;
        plan.error = Some("new plan rejected".into());
        plan.observe_clock_ack(2,3,json!({"state":"paused"}), plan.play_intent_revision, Instant::now());
        assert_eq!(plan.error.as_deref(), Some("new plan rejected"));
        assert_eq!(plan.presentation_label(), "Hardware preparation blocked");
        plan.replace(Err("invalid cue".into()));
        plan.observe_clock_ack(2,3,json!({"state":"paused"}), plan.play_intent_revision, Instant::now());
        assert_eq!(plan.error.as_deref(), Some("invalid cue"));
        assert_eq!(plan.presentation_label(), "Hardware plan invalid");
    }
    #[test]
    fn old_executor_fault_does_not_cancel_explicit_new_plan_retry() {
        let mut plan = acknowledged_empty_plan();
        plan.revision = 3;
        plan.play_requested = true;
        plan.observe_clock_ack(2,3,json!({"state":"faulted","error":"old executor"}), plan.play_intent_revision, Instant::now());
        assert!(plan.play_requested);
        assert!(!plan.requires_reprepare);
        assert!(!plan.ready_for(3));
    }
    #[test]
    fn genuine_fault_stays_latched_until_explicit_retry_not_a_healthy_heartbeat() {
        let mut plan = acknowledged_empty_plan();
        plan.play_requested = true;
        plan.observe_clock_ack(2,3,json!({"state":"faulted","error":"late ACK"}), plan.play_intent_revision, Instant::now());
        assert!(plan.requires_reprepare);
        assert!(!plan.play_requested);
        plan.observe_clock_ack(2,3,json!({"revision":2,"state":"ready","step_count":0}), plan.play_intent_revision, Instant::now());
        assert_eq!(plan.error.as_deref(), Some("late ACK"));
        assert!(!plan.ready_for(3));
        assert!(plan.request_play());
        assert_eq!(plan.revision, 3);
        assert!(plan.play_requested);
        assert!(!plan.ready_for(3));
    }
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
