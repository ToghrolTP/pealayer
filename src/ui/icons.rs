//! One cross-platform icon vocabulary. These are font-backed Phosphor vectors,
//! not emoji, so glyph availability and color do not vary by host OS.

pub use egui_phosphor::regular::{
    ARROW_COUNTER_CLOCKWISE, ARROW_DOWN, ARROW_SQUARE_OUT, ARROW_UP, ARROWS_IN,
    ARROWS_OUT, CHECK_SQUARE, CLIPBOARD, CLOCK_COUNTER_CLOCKWISE, DOT_OUTLINE, ERASER,
    FILE_VIDEO, FLOPPY_DISK, GAUGE, GEAR, INFO, KEYBOARD, LIGHTBULB, LIGHTNING, LINK,
    MAGNIFYING_GLASS, MONITOR_PLAY, MUSIC_NOTE, PAPER_PLANE_TILT, PAUSE,
    PENCIL_SIMPLE, PLAY, PLUG, PUSH_PIN, PUSH_PIN_SLASH, RADIO, RECORD, SEAT,
    SELECTION_ALL, SLIDERS_HORIZONTAL, SPARKLE, SPEAKER_HIGH, SPEAKER_SLASH,
    STOP_CIRCLE, SUBTITLES, TABS, WARNING, WAVEFORM, X,
};

pub fn control(kind: &str, advertised: &str) -> &'static str {
    match advertised.trim().to_ascii_lowercase().as_str() {
        "armchair" | "seat" => SEAT,
        "lightbulb" | "light" => LIGHTBULB,
        "radio" | "rf" => RADIO,
        _ => match kind {
            "seat" | "motion" => SEAT,
            "mosfet" | "pwm" => LIGHTBULB,
            "relay" => PLUG,
            _ => GAUGE,
        },
    }
}

pub fn action(verb: &str) -> &'static str {
    match verb {
        "up" => ARROW_UP,
        "down" => ARROW_DOWN,
        "stop" | "off" => STOP_CIRCLE,
        "on" => LIGHTNING,
        _ => PLAY,
    }
}
