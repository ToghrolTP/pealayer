use pealayer::four_d::curve::{AnalogTrack, Interpolation, Keyframe};

#[test]
fn test_empty_track_evaluation() {
    let track = AnalogTrack::new("Wind Fan", 0);
    assert_eq!(track.evaluate(0), 0.0);
    assert_eq!(track.evaluate(5000), 0.0);
    assert_eq!(track.evaluate_u8(1000), 0);
    assert_eq!(track.evaluate_u16(1000), 0);
}

#[test]
fn test_single_keyframe_evaluation() {
    let mut track = AnalogTrack::new("Wind Fan", 0);
    track.add_keyframe(Keyframe::new(1000, 0.75, Interpolation::Linear));

    // Before keyframe: clamps or starts at keyframe value
    assert_eq!(track.evaluate(500), 0.75);
    // At keyframe
    assert_eq!(track.evaluate(1000), 0.75);
    // After keyframe
    assert_eq!(track.evaluate(2000), 0.75);
}

#[test]
fn test_step_interpolation() {
    let mut track = AnalogTrack::new("Wind Fan", 0);
    track.add_keyframe(Keyframe::new(1000, 0.2, Interpolation::Step));
    track.add_keyframe(Keyframe::new(2000, 0.8, Interpolation::Step));

    assert_eq!(track.evaluate(1000), 0.2);
    assert_eq!(track.evaluate(1500), 0.2); // Holds 0.2 until 2000
    assert_eq!(track.evaluate(1999), 0.2);
    assert_eq!(track.evaluate(2000), 0.8);
    assert_eq!(track.evaluate(3000), 0.8);
}

#[test]
fn test_linear_interpolation() {
    let mut track = AnalogTrack::new("Wind Fan", 0);
    track.add_keyframe(Keyframe::new(1000, 0.0, Interpolation::Linear));
    track.add_keyframe(Keyframe::new(2000, 1.0, Interpolation::Linear));

    assert_eq!(track.evaluate(1000), 0.0);
    assert_eq!(track.evaluate(1500), 0.5); // Midpoint
    assert_eq!(track.evaluate(1250), 0.25);
    assert_eq!(track.evaluate(1750), 0.75);
    assert_eq!(track.evaluate(2000), 1.0);

    // Quantized evaluations
    assert_eq!(track.evaluate_u8(1500), 128);
    assert_eq!(track.evaluate_u16(1500), 2048);
    assert_eq!(track.evaluate_u8(2000), 255);
    assert_eq!(track.evaluate_u16(2000), 4095);
}

#[test]
fn test_smooth_cubic_interpolation() {
    let mut track = AnalogTrack::new("Wind Fan", 0);
    track.add_keyframe(Keyframe::new(0, 0.0, Interpolation::Smooth));
    track.add_keyframe(Keyframe::new(1000, 1.0, Interpolation::Smooth));

    // Smooth ease-in-out S-curve properties:
    // At t=0 -> 0.0
    // At midpoint t=500ms -> 0.5
    // Near beginning t=250ms -> slope is shallower than linear (val < 0.25)
    // Near end t=750ms -> slope is shallower than linear (val > 0.75)
    let val_250 = track.evaluate(250);
    let val_500 = track.evaluate(500);
    let val_750 = track.evaluate(750);

    assert_eq!(track.evaluate(0), 0.0);
    assert!((val_500 - 0.5).abs() < 0.001);
    assert!(val_250 < 0.25, "Ease-in should be shallower than linear (0.25), got {}", val_250);
    assert!(val_750 > 0.75, "Ease-out should be steeper than linear (0.75), got {}", val_750);
    assert_eq!(track.evaluate(1000), 1.0);
}

#[test]
fn test_keyframe_sorting_and_deduplication() {
    let mut track = AnalogTrack::new("Rumble Chair", 1);
    track.add_keyframe(Keyframe::new(3000, 0.3, Interpolation::Linear));
    track.add_keyframe(Keyframe::new(1000, 0.1, Interpolation::Linear));
    track.add_keyframe(Keyframe::new(2000, 0.2, Interpolation::Linear));

    // Must be sorted by time_ms
    assert_eq!(track.keyframes.len(), 3);
    assert_eq!(track.keyframes[0].time_ms, 1000);
    assert_eq!(track.keyframes[1].time_ms, 2000);
    assert_eq!(track.keyframes[2].time_ms, 3000);

    // Overwriting existing time_ms updates value
    track.add_keyframe(Keyframe::new(2000, 0.9, Interpolation::Step));
    assert_eq!(track.keyframes.len(), 3);
    assert_eq!(track.keyframes[1].time_ms, 2000);
    assert_eq!(track.keyframes[1].value, 0.9);
    assert_eq!(track.keyframes[1].interpolation, Interpolation::Step);
}

#[test]
fn test_serde_roundtrip() {
    let mut track = AnalogTrack::new("Water Mist", 2);
    track.add_keyframe(Keyframe::new(100, 0.1, Interpolation::Step));
    track.add_keyframe(Keyframe::new(500, 0.8, Interpolation::Smooth));

    let json = serde_json::to_string_pretty(&track).expect("Serialization failed");
    let deserialized: AnalogTrack = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(track, deserialized);
}
