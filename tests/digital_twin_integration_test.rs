use pealayer::app::PealayerApp;
use pealayer::four_d::controller::{HardwareCapabilities, HardwareFrontPanel, VirtualSeatSample};
use pealayer::four_d::seat_kinematics::CinemaSeatKinematics;

#[test]
fn live_twin_consumes_the_published_panel_and_applied_seat_positions() {
    let app = PealayerApp::default();
    let panel = HardwareFrontPanel {
        schema: 1,
        raw_segments: vec![0x3F, 0x86, 0x5B, 0x4F],
        brightness: 7,
        segments_active: true,
        lcd_available: true,
        lcd_address: 0x27,
        lcd_line_1: "Pealayer Cinema".into(),
        lcd_line_2: "Demo Ready".into(),
        ..Default::default()
    };
    let sample: VirtualSeatSample = serde_json::from_value(serde_json::json!({
        "position_basis_points":[7000,3000],
        "full_travel_ms":3000,
        "sampled_at":"2026-11-11T00:00:00Z"
    })).unwrap();
    app.update_hardware_capabilities(Some(HardwareCapabilities {
        board_connected: true,
        front_panel: Some(panel),
        virtual_seats: Some(sample),
        ..Default::default()
    }));

    let caps = app.advertised_hardware().expect("published hardware snapshot");
    let panel = caps.front_panel.expect("published firmware panel");
    assert_eq!(panel.raw_segments, [0x3F, 0x86, 0x5B, 0x4F]);
    assert_eq!(panel.lcd_line_1, "Pealayer Cinema");
    assert_eq!(panel.lcd_line_2, "Demo Ready");
    let sample = caps.virtual_seats.expect("applied position sample");
    assert_eq!(sample.full_travel_ms, 3000);
    let seat = CinemaSeatKinematics::from_positions(sample.positions());
    assert_eq!([seat.left_stroke, seat.right_stroke], [0.7, 0.3]);
    assert!(seat.roll_degrees() < 0.0);
}
