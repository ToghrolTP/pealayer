use std::path::PathBuf;
use uuid::Uuid;
use crate::four_d::curve::{AnalogTrack, Interpolation, Keyframe};
use crate::four_d::models::{
    AtomicAction, CueDurationPolicy, DirectControlCue, DirectCueBehavior, DirectMotionCue,
    DirectMotionDirection, Effect, EffectInstance, HardwareTarget, Timeline, TimelineKeyframe,
};

pub fn demo_media_target() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("test-data/jellyfish.mp4"),
        PathBuf::from("assets/jellyfish.mp4"),
    ];
    for candidate in candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn build_demo_showcase_timeline() -> Timeline {
    let mut timeline = Timeline::default();

    // 1. Template: Seat A Pitch Up
    let seat_a_pitch_id = Uuid::new_v4();
    let seat_a_pitch = Effect {
        id: seat_a_pitch_id,
        name: "Seat A Pitch Up".to_string(),
        icon: "seat".to_string(),
        duration_ms: 1200,
        target: HardwareTarget::Any,
        actions: Vec::new(),
        controller_macro: None,
        controller_strip_effect: None,
        controller_lane: None,
        direct_control: Some(DirectControlCue {
            control_key: "seat.a".to_string(),
            value_basis_points: 7500,
            behavior: DirectCueBehavior::Hold,
            end_value_basis_points: 0,
            motion: Some(DirectMotionCue {
                side: 0,
                direction: DirectMotionDirection::Up,
            }),
        }),
        duration_policy: CueDurationPolicy::Resizable,
        audio_effect: None,
    };
    timeline.templates.push(seat_a_pitch);

    // 2. Template: Wind Relay Strobe
    let wind_relay_id = Uuid::new_v4();
    let wind_relay = Effect {
        id: wind_relay_id,
        name: "Wind Strobe Burst".to_string(),
        icon: "fan".to_string(),
        duration_ms: 1500,
        target: HardwareTarget::Relay(1),
        actions: vec![AtomicAction {
            relay_id: 1,
            state: true,
            offset_ms: 0,
        }],
        controller_macro: None,
        controller_strip_effect: None,
        controller_lane: None,
        direct_control: Some(DirectControlCue {
            control_key: "relay.1".to_string(),
            value_basis_points: 10000,
            behavior: DirectCueBehavior::Hold,
            end_value_basis_points: 0,
            motion: None,
        }),
        duration_policy: CueDurationPolicy::Resizable,
        audio_effect: None,
    };
    timeline.templates.push(wind_relay);

    // 3. Placed Instances
    timeline.instances.push(EffectInstance {
        id: Uuid::new_v4(),
        effect_id: seat_a_pitch_id,
        start_time_ms: 1000,
    });
    timeline.instances.push(EffectInstance {
        id: Uuid::new_v4(),
        effect_id: wind_relay_id,
        start_time_ms: 2500,
    });

    // 4. Analog Fan Curve Track (PWM Channel 0)
    timeline.analog_tracks.push(AnalogTrack {
        id: Uuid::new_v4(),
        name: "Wind Speed (Fan PWM)".to_string(),
        channel: 0,
        enabled: true,
        muted: false,
        soloed: false,
        locked: false,
        armed: false,
        keyframes: vec![
            Keyframe::new(0, 0.0, Interpolation::Linear),
            Keyframe::new(1500, 0.4, Interpolation::Smooth),
            Keyframe::new(2500, 0.85, Interpolation::Smooth),
            Keyframe::new(4500, 0.2, Interpolation::Linear),
            Keyframe::new(6000, 0.0, Interpolation::Step),
        ],
    });

    // 5. Keyframes
    timeline.keyframes.push(TimelineKeyframe {
        id: Uuid::new_v4(),
        time_ms: 1000,
        label: "Descent Incline".to_string(),
    });
    timeline.keyframes.push(TimelineKeyframe {
        id: Uuid::new_v4(),
        time_ms: 2500,
        label: "Current Burst".to_string(),
    });

    timeline
}
