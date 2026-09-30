//! One cross-platform icon vocabulary. These are font-backed Phosphor vectors,
//! not emoji, so glyph availability and color do not vary by host OS.

pub use egui_phosphor::regular::{
    ARROW_COUNTER_CLOCKWISE, ARROW_DOWN, ARROW_UP, ARROWS_OUT, CHECK_SQUARE,
    DOT_OUTLINE, FLOPPY_DISK, GAUGE, GEAR, LIGHTBULB, LIGHTNING,
    MAGNIFYING_GLASS, MONITOR_PLAY, PAPER_PLANE_TILT, PENCIL_SIMPLE, PLAY,
    PLUG, RADIO, RECORD, SEAT, SLIDERS_HORIZONTAL, SPARKLE, STOP_CIRCLE,
    TABS, WARNING, WAVEFORM, X,
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
