use eframe::egui;
use pealayer::four_d::controller::{
    HardwareFrontPanel, HardwareStatusLed,
};
use pealayer::four_d::seat_kinematics::CinemaSeatKinematics;

#[test]
fn test_digital_twin_attitude_and_panel_rendering() {
    let mut kinematics = CinemaSeatKinematics::default();
    assert_eq!(kinematics.left_stroke, 0.5);
    assert_eq!(kinematics.right_stroke, 0.5);

    // Simulate tick with 0.05s dt
    // Left (+1.0 Extend), Right (-1.0 Retract)
    for _ in 0..10 {
        kinematics.update(0.05, 1.0, -1.0, false);
    }
    assert!(
        kinematics.left_stroke > 0.5,
        "Left cylinder should extend on 'up'"
    );
    assert!(
        kinematics.right_stroke < 0.5,
        "Right cylinder should retract on 'down'"
    );

    let _pitch = kinematics.pitch_degrees();
    let roll = kinematics.roll_degrees();
    assert!(roll < 0.0, "Roll should be negative (tilting left)");

    // Test interlock halts motion
    let left_pos_before = kinematics.left_stroke;
    for _ in 0..5 {
        kinematics.update(0.05, 1.0, -1.0, true);
    }
    assert_eq!(
        kinematics.left_stroke, left_pos_before,
        "Interlock must freeze cylinder motion"
    );

    // Test egui drawing pass for front panel and seat attitude
    let front_panel = HardwareFrontPanel {
        schema: 1,
        raw_segments: vec![0x3F, 0x06, 0x5B, 0x4F],
        brightness: 7,
        blink: false,
        segments_active: true,
        pressed_keys: 0,
        menu_page: 0,
        program_mode: 0,
        host_captured: false,
        lcd_available: true,
        lcd_address: 0x27,
        lcd_line_1: "Pealayer Cinema".into(),
        lcd_line_2: "Demo Ready".into(),
    };

    let status_led = HardwareStatusLed {
        red: 0,
        green: 255,
        blue: 128,
        brightness: 255,
        effect: 0,
        condition: 0,
    };

    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        pealayer::ui::digital_twin::draw_front_panel(
            ui,
            Some(&front_panel),
            Some(&status_led),
            true,
        );
        pealayer::ui::digital_twin::draw_seat_attitude(ui, &kinematics, false);
    });
    output.textures_delta.clear();
}
