//! One cross-platform icon vocabulary. These are font-backed Phosphor vectors,
//! not emoji, so glyph availability and color do not vary by host OS.

pub use egui_phosphor::regular::{
    APP_WINDOW, ARROW_COUNTER_CLOCKWISE, ARROW_DOWN, ARROW_SQUARE_OUT, ARROW_UP,
    ARROWS_IN, ARROWS_OUT, CARET_DOWN, CARET_RIGHT, CHECK_SQUARE, CIRCUITRY,
    CLIPBOARD, CLOCK_COUNTER_CLOCKWISE, COPY, CPU, DOT_OUTLINE, ERASER, EYE,
    EYE_SLASH, FILE_VIDEO, FLOPPY_DISK, FOLDER_OPEN, GAUGE, GEAR, INFO, KEYBOARD,
    LIGHTBULB, LIGHTNING, LINK, LIST_CHECKS, MAGNIFYING_GLASS, MONITOR_PLAY,
    MUSIC_NOTE, PAPER_PLANE_TILT, PAUSE, PENCIL_SIMPLE, PLAY, PLUG, POWER, PUSH_PIN,
    PUSH_PIN_SLASH, RADIO, RECORD, SEAT, SELECTION_ALL, SLIDERS_HORIZONTAL, SPARKLE,
    SPEAKER_HIGH, SPEAKER_NONE, SPEAKER_SLASH, STOP_CIRCLE, SUBTITLES, TABS, TRASH,
    WARNING, WAVEFORM, X,
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
