use pealayer::four_d::seat_kinematics::CinemaSeatKinematics;

#[test]
fn equal_applied_positions_give_pitch_without_roll() {
    for (position, pitch) in [(0.0, -15.0), (0.5, 0.0), (1.0, 15.0)] {
        let seat = CinemaSeatKinematics::from_positions([position, position]);
        assert_eq!(seat.left_stroke, position);
        assert_eq!(seat.right_stroke, position);
        assert_eq!(seat.pitch_degrees(), pitch);
        assert_eq!(seat.roll_degrees(), 0.0);
    }
}

#[test]
fn mirrored_applied_positions_reverse_roll_without_changing_pitch() {
    let left_low = CinemaSeatKinematics::from_positions([0.25, 0.75]);
    let right_low = CinemaSeatKinematics::from_positions([0.75, 0.25]);
    assert_eq!(left_low.pitch_degrees(), 0.0);
    assert_eq!(right_low.pitch_degrees(), 0.0);
    assert_eq!(left_low.roll_degrees(), 5.0);
    assert_eq!(right_low.roll_degrees(), -5.0);
}
