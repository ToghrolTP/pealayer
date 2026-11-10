use pealayer::ui::digital_twin::{format_lcd_line, decode_7segment_text, attitude_badge_label};

#[test]
fn test_format_lcd_line_exact_16_chars() {
    assert_eq!(format_lcd_line("Hello"), "Hello           ");
    assert_eq!(format_lcd_line("1234567890123456"), "1234567890123456");
    assert_eq!(format_lcd_line("1234567890123456789"), "1234567890123456");
    assert_eq!(format_lcd_line(""), "                ");
}

#[test]
fn test_decode_7segment_text_defaults_and_truncation() {
    assert_eq!(decode_7segment_text(&[], "BOOT"), "BOOT");
    assert_eq!(decode_7segment_text(&[], "PLAYING"), "PLAY");
    assert_eq!(decode_7segment_text(&[], ""), "----");
}

#[test]
fn test_attitude_badge_label_formatting() {
    let pitch_label = attitude_badge_label("Pitch", 3.3);
    assert_eq!(pitch_label, "Pitch: +3.3°");

    let roll_label = attitude_badge_label("Roll", -1.82);
    assert_eq!(roll_label, "Roll: -1.8°");

    let neutral_label = attitude_badge_label("Pitch", 0.0);
    assert_eq!(neutral_label, "Pitch: 0.0°");
}
