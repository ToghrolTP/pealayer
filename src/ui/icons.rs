//! One cross-platform icon vocabulary. These are font-backed Phosphor vectors,
//! not emoji, so glyph availability and color do not vary by host OS.

pub use egui_phosphor::regular::{
    APP_WINDOW, ARROW_CLOCKWISE, ARROW_COUNTER_CLOCKWISE, ARROW_DOWN, ARROW_SQUARE_OUT, ARROW_UP,
    ARROWS_IN, ARROWS_OUT, BROADCAST, CARET_DOWN, CARET_RIGHT, CHECK, CHECK_SQUARE, CIRCUITRY,
    CLIPBOARD, CLOCK, CLOCK_COUNTER_CLOCKWISE, COPY, CPU, DOT_OUTLINE, DOTS_THREE, ERASER, EYE,
    EYE_SLASH, FAST_FORWARD, FILE_VIDEO, FLOPPY_DISK, FOLDER_OPEN, FRAME_CORNERS, GAUGE, GEAR,
    GLOBE, HOURGLASS_MEDIUM, INFO, KEYBOARD, LIGHTBULB, LIGHTNING, LINK, LINK_SIMPLE, LIST_CHECKS,
    LOCK, MAGNIFYING_GLASS, MONITOR_PLAY, MUSIC_NOTE, PAPER_PLANE_TILT, PAUSE, PENCIL_SIMPLE, PLAY,
    PLUG, PLUS, POWER, PUSH_PIN, PUSH_PIN_SLASH, RADIO, RECORD, REWIND, SCISSORS, SEAT,
    SELECTION_ALL, SKIP_BACK, SKIP_FORWARD, SLIDERS_HORIZONTAL, SPARKLE, SPEAKER_HIGH,
    SPEAKER_NONE, SPEAKER_SLASH, STOP_CIRCLE, SUBTITLES, TABS, TEXT_ALIGN_LEFT, TRASH, WARNING,
    WAVEFORM, X,
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

/// Draw a consistent Phosphor-backed disclosure control and persist its state.
/// This avoids platform/font-dependent triangle glyphs from egui's default
/// `CollapsingHeader` while preserving keyboard focus and a full-row hit target.
pub fn disclosure_header(
    ui: &mut eframe::egui::Ui,
    id_salt: impl std::hash::Hash,
    label: &str,
    default_open: bool,
) -> bool {
    let id = ui.make_persistent_id(id_salt);
    let mut open = ui.data_mut(|data| data.get_persisted::<bool>(id).unwrap_or(default_open));
    let icon = if open { CARET_DOWN } else { CARET_RIGHT };
    let response = ui.add(
        eframe::egui::Button::new(eframe::egui::RichText::new(format!("{icon}  {label}")).strong())
            .frame(false),
    );
    if response.clicked() {
        open = !open;
    }
    ui.data_mut(|data| data.insert_persisted(id, open));
    open
}

/// Draw a submenu entry with the same Phosphor caret used by the rest of the UI.
/// egui's stock submenu indicator is a text glyph (`⏵`), which looks out of
/// place next to the application's vector-icon font and varies across systems.
pub fn submenu(
    ui: &mut eframe::egui::Ui,
    label: String,
    add_contents: impl FnOnce(&mut eframe::egui::Ui),
) {
    let button = eframe::egui::Button::new(label).right_text(CARET_RIGHT);
    let _ = eframe::egui::containers::menu::SubMenuButton::from_button(button).ui(ui, add_contents);
}
