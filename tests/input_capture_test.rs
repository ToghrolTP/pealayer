use pealayer::four_d::curve::AnalogTrack;
use pealayer::four_d::input_capture::InputCaptureState;

#[test]
fn test_analog_track_armed_field_default_false() {
    let track = AnalogTrack::new("Wind Fan", 0);
    assert!(!track.armed);
}

#[test]
fn test_input_capture_throttle_clamping_and_ramping() {
    let mut input = InputCaptureState::new();
    assert_eq!(input.current_throttle, 0.0);

    // Set absolute value
    input.set_throttle(0.75);
    assert_eq!(input.current_throttle, 0.75);

    input.set_throttle(1.5); // Clamped
    assert_eq!(input.current_throttle, 1.0);

    input.set_throttle(-0.5); // Clamped
    assert_eq!(input.current_throttle, 0.0);

    // Ramp up by delta
    input.ramp_throttle(0.2);
    assert!((input.current_throttle - 0.2).abs() < 0.001);

    input.ramp_throttle(-0.1);
    assert!((input.current_throttle - 0.1).abs() < 0.001);
}

#[test]
fn test_input_capture_keyboard_update() {
    let mut input = InputCaptureState::new();
    input.ramp_rate_per_sec = 2.0;

    // Up pressed for 0.25s -> +0.50
    input.update_from_keyboard(true, false, 0.25);
    assert!((input.current_throttle - 0.5).abs() < 0.001);

    // Both pressed -> no change
    input.update_from_keyboard(true, true, 0.25);
    assert!((input.current_throttle - 0.5).abs() < 0.001);

    // Neither pressed -> no change
    input.update_from_keyboard(false, false, 0.25);
    assert!((input.current_throttle - 0.5).abs() < 0.001);

    // Down pressed for 0.1s -> -0.20 -> 0.30
    input.update_from_keyboard(false, true, 0.1);
    assert!((input.current_throttle - 0.3).abs() < 0.001);
}

#[test]
fn test_analog_track_serde_backward_compatible_armed() {
    // JSON without armed field
    let json = r#"{"id":"00000000-0000-0000-0000-000000000000","name":"Test","channel":1,"keyframes":[]}"#;
    let track: AnalogTrack = serde_json::from_str(json).expect("Deserialization failed");
    assert!(!track.armed);
}
