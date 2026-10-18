use pealayer::four_d::seat_kinematics::CinemaSeatKinematics;

#[test]
fn test_seat_kinematics_default_neutral() {
    let k = CinemaSeatKinematics::default();
    assert_eq!(k.left_stroke, 0.5);
    assert_eq!(k.right_stroke, 0.5);
    assert_eq!(k.pitch_degrees(), 0.0);
    assert_eq!(k.roll_degrees(), 0.0);
}

#[test]
fn test_seat_kinematics_pitch_slew_and_clamp() {
    let mut k = CinemaSeatKinematics::default();

    // 100ms step moving both up
    k.update(0.1, 1.0, 1.0, false);
    // 0.5 + 0.1 * 0.4 = 0.54
    assert!((k.left_stroke - 0.54).abs() < 1e-4);
    assert!((k.right_stroke - 0.54).abs() < 1e-4);
    assert!(k.pitch_degrees() > 0.0);
    assert_eq!(k.roll_degrees(), 0.0);

    // Run for 10 seconds up - should clamp to 1.0
    for _ in 0..200 {
        k.update(0.05, 1.0, 1.0, false);
    }
    assert_eq!(k.left_stroke, 1.0);
    assert_eq!(k.right_stroke, 1.0);
    assert!((k.pitch_degrees() - 15.0).abs() < 1e-4);
}

#[test]
fn test_seat_kinematics_roll_slew() {
    let mut k = CinemaSeatKinematics::default();

    // Left down, Right up
    k.update(0.1, -1.0, 1.0, false);
    assert!(k.left_stroke < 0.5);
    assert!(k.right_stroke > 0.5);
    assert!(k.roll_degrees() > 0.0);
}

#[test]
fn test_seat_kinematics_interlock_freeze() {
    let mut k = CinemaSeatKinematics::default();

    // With interlock tripped (door open), stroke must not move
    k.update(0.1, 1.0, 1.0, true);
    assert_eq!(k.left_stroke, 0.5);
    assert_eq!(k.right_stroke, 0.5);
}
