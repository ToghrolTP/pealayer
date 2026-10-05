//! One cross-platform icon vocabulary. These are font-backed Phosphor vectors,
//! not emoji, so glyph availability and color do not vary by host OS.

pub use egui_phosphor::regular::{
    APP_WINDOW, ARROW_CLOCKWISE, ARROW_COUNTER_CLOCKWISE, ARROW_DOWN, ARROW_RIGHT,
    ARROW_SQUARE_OUT, ARROW_UP, ARROWS_IN, ARROWS_OUT, BELL, BROADCAST, CAR, CARET_DOWN,
    CARET_RIGHT, CHECK, CHECK_SQUARE, CIRCUITRY, CLIPBOARD, CLOCK, CLOCK_COUNTER_CLOCKWISE, COPY,
    CPU, DIAMOND, DOOR, DOT_OUTLINE, DOTS_SIX_VERTICAL, DOTS_THREE, ERASER, EYE, EYE_SLASH, FAN,
    FAST_FORWARD, FILE_VIDEO, FIRE, FLOPPY_DISK, FOLDER_OPEN, FRAME_CORNERS, GAUGE, GEAR, GLOBE,
    HAND_TAP, HEADPHONES, HOURGLASS_MEDIUM, INFO, KEYBOARD, LAMP, LIGHTBULB, LIGHTNING, LINK,
    LINK_SIMPLE, LIST_CHECKS, LOCK, MAGNIFYING_GLASS, MINUS, MONITOR_PLAY, MUSIC_NOTE, PALETTE,
    PAPER_PLANE_TILT, PAUSE, PENCIL_SIMPLE, PLAY, PLUG, PLUS, POWER, PROHIBIT, PUSH_PIN,
    PUSH_PIN_SLASH, RADIO, RECORD, REWIND, SCISSORS, SEAT, SELECTION_ALL, SKIP_BACK, SKIP_FORWARD,
    SLIDERS_HORIZONTAL, SNOWFLAKE, SPARKLE, SPEAKER_HIGH, SPEAKER_NONE, SPEAKER_SLASH, STOP_CIRCLE,
    SUBTITLES, TABS, TARGET, TEXT_ALIGN_LEFT, THERMOMETER, TOGGLE_RIGHT, TRASH, WARNING, WAVEFORM,
    X,
};

pub const CONTROL_ICON_PRESETS: &[(&str, &str, &str)] = &[
    ("sparkle", "Sparkle", SPARKLE),
    ("folder", "Folder", FOLDER_OPEN),
    ("plug", "Plug", PLUG),
    ("lightning", "Lightning", LIGHTNING),
    ("lightbulb", "Light bulb", LIGHTBULB),
    ("lamp", "Lamp", LAMP),
    ("fan", "Fan", FAN),
    ("power", "Power", POWER),
    ("speaker", "Speaker", SPEAKER_HIGH),
    ("radio", "Radio", RADIO),
    ("seat", "Seat", SEAT),
    ("car", "Car", CAR),
    ("door", "Door", DOOR),
    ("bell", "Bell", BELL),
    ("fire", "Fire", FIRE),
    ("snowflake", "Snowflake", SNOWFLAKE),
    ("thermometer", "Thermometer", THERMOMETER),
    ("waveform", "Waveform", WAVEFORM),
    ("circuitry", "Circuitry", CIRCUITRY),
    ("gear", "Gear", GEAR),
];

pub const WORKSPACE_ICON_PRESETS: &[(&str, &str, &str)] = &[
    ("monitor", "Monitor", MONITOR_PLAY),
    ("timeline", "Timeline", WAVEFORM),
    ("tabs", "Tabs", TABS),
    ("window", "Window", APP_WINDOW),
    ("video", "Video", FILE_VIDEO),
    ("hardware", "Hardware", CIRCUITRY),
    ("effects", "Effects", SPARKLE),
    ("layout", "Layout", SELECTION_ALL),
];

pub fn workspace_icon(name: &str) -> &'static str {
    WORKSPACE_ICON_PRESETS
        .iter()
        .find(|(key, _, _)| key.eq_ignore_ascii_case(name.trim()))
        .map(|(_, _, glyph)| *glyph)
        .unwrap_or(APP_WINDOW)
}

pub fn workspace_icon_name(name: &str) -> &'static str {
    WORKSPACE_ICON_PRESETS
        .iter()
        .find(|(key, _, _)| key.eq_ignore_ascii_case(name.trim()))
        .map(|(_, label, _)| *label)
        .unwrap_or("Window")
}

pub fn named_control_icon(name: &str) -> Option<&'static str> {
    CONTROL_ICON_PRESETS
        .iter()
        .find(|(key, _, _)| key.eq_ignore_ascii_case(name.trim()))
        .map(|(_, _, glyph)| *glyph)
}

pub fn control_icon_name(name: &str) -> Option<&'static str> {
    CONTROL_ICON_PRESETS
        .iter()
        .find(|(key, _, _)| key.eq_ignore_ascii_case(name.trim()))
        .map(|(_, label, _)| *label)
}

fn icon_preset_matches(key: &str, label: &str, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty() || key.to_lowercase().contains(&query) || label.to_lowercase().contains(&query)
}

/// Presentation and catalog data for the shared icon picker. Keeping this in
/// one component prevents workspace, effect, and hardware-channel selectors
/// from drifting back to unrelated text boxes and preset menus.
#[derive(Clone, Copy)]
pub struct IconPickerConfig<'a> {
    pub presets: &'a [(&'a str, &'a str, &'a str)],
    pub fallback_glyph: &'a str,
    pub fallback_name: &'a str,
    pub width: f32,
    pub show_selected_name: bool,
    pub search_hint: &'a str,
    pub presets_label: &'a str,
    pub no_matches_label: &'a str,
    pub clear_label: Option<&'a str>,
}

fn icon_preset<'a>(
    presets: &'a [(&'a str, &'a str, &'a str)],
    value: &str,
) -> Option<&'a (&'a str, &'a str, &'a str)> {
    presets
        .iter()
        .find(|(key, _, _)| key.eq_ignore_ascii_case(value.trim()))
}

/// Draw the popup body used by both combobox selectors and compact icon
/// buttons. The first row is always search; optional defaults and preset
/// results form separate, consistently divided sections below it.
pub fn searchable_icon_picker_contents(
    ui: &mut eframe::egui::Ui,
    value: &mut String,
    search: &mut String,
    search_id: eframe::egui::Id,
    request_focus: bool,
    config: IconPickerConfig<'_>,
) -> bool {
    use eframe::egui;

    let previous = value.clone();
    let search_response = ui
        .horizontal(|ui| {
            ui.label(egui::RichText::new(MAGNIFYING_GLASS).color(ui.visuals().weak_text_color()));
            ui.add(
                egui::TextEdit::singleline(search)
                    .id_salt(search_id.with("input"))
                    .hint_text(config.search_hint)
                    .desired_width(ui.available_width()),
            )
        })
        .inner;
    if request_focus {
        search_response.request_focus();
    }

    if let Some(clear_label) = config.clear_label {
        ui.separator();
        if ui
            .selectable_label(
                value.trim().is_empty(),
                format!("{}  {clear_label}", config.fallback_glyph),
            )
            .clicked()
        {
            value.clear();
            search.clear();
            ui.close();
        }
    }

    ui.separator();
    ui.label(
        egui::RichText::new(config.presets_label)
            .small()
            .strong()
            .color(ui.visuals().weak_text_color()),
    );

    let mut match_count = 0;
    for (key, label, glyph) in config.presets {
        if !icon_preset_matches(key, label, search) {
            continue;
        }
        match_count += 1;
        if ui
            .selectable_label(value.eq_ignore_ascii_case(key), format!("{glyph}  {label}"))
            .clicked()
        {
            *value = (*key).to_string();
            search.clear();
            ui.close();
        }
    }
    if match_count == 0 {
        ui.label(
            egui::RichText::new(config.no_matches_label)
                .italics()
                .color(ui.visuals().weak_text_color()),
        );
    }

    *value != previous
}

/// A single searchable icon combobox shared by all native egui editors. The
/// closed control shows the selected Phosphor icon and, when space permits,
/// its human-readable name.
pub fn searchable_icon_picker(
    ui: &mut eframe::egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    value: &mut String,
    config: IconPickerConfig<'_>,
) -> bool {
    use eframe::egui;

    // ComboBox converts the supplied salt to IdSalt before deriving its
    // widget ID. Match that conversion so search survives successive frames
    // and focus is requested only when the popup first opens.
    let button_id = ui.make_persistent_id(egui::IdSalt::new(&id_salt));
    let search_id = button_id.with("search");
    let was_open = egui::ComboBox::is_open(ui.ctx(), button_id);
    let mut search = ui.data_mut(|data| data.get_temp::<String>(search_id).unwrap_or_default());
    let previous = value.clone();
    let selected = icon_preset(config.presets, value);
    let selected_glyph = selected
        .map(|(_, _, glyph)| *glyph)
        .unwrap_or(config.fallback_glyph);
    let selected_name = selected.map(|(_, label, _)| *label).unwrap_or_else(|| {
        if value.trim().is_empty() {
            config.fallback_name
        } else {
            value.trim()
        }
    });
    let selected_text = if config.show_selected_name && !selected_name.is_empty() {
        format!("{selected_glyph}  {selected_name}")
    } else {
        selected_glyph.to_string()
    };

    egui::ComboBox::from_id_salt(&id_salt)
        .width(config.width)
        .height(320.0)
        .truncate()
        .icon(|ui, rect, visuals, _| {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                CARET_DOWN,
                egui::FontId::proportional(14.0),
                visuals.text_color(),
            );
        })
        .selected_text(selected_text)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            ui.set_min_width(config.width.max(260.0));
            searchable_icon_picker_contents(ui, value, &mut search, search_id, !was_open, config);
        });

    if egui::ComboBox::is_open(ui.ctx(), button_id) {
        ui.data_mut(|data| data.insert_temp(search_id, search));
    } else {
        ui.data_mut(|data| data.remove::<String>(search_id));
    }
    *value != previous
}

/// Compact entry point for channel-card icons. It shares the combobox's
/// search, selection, default option, and popup lifecycle without adding a
/// second selector to the card.
pub fn searchable_icon_button(
    ui: &mut eframe::egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    value: &mut String,
    size: f32,
    tooltip: &str,
    config: IconPickerConfig<'_>,
) -> bool {
    use eframe::egui;

    let picker_id = ui.make_persistent_id(&id_salt);
    let popup_id = picker_id.with("popup");
    let search_id = picker_id.with("search");
    let was_open = egui::Popup::is_id_open(ui.ctx(), popup_id);
    let glyph = icon_preset(config.presets, value)
        .map(|(_, _, glyph)| *glyph)
        .unwrap_or(config.fallback_glyph);
    let response = ui
        .push_id(&id_salt, |ui| {
            ui.add(
                egui::Button::new(egui::RichText::new(glyph).size(size))
                    .frame(false)
                    .min_size(egui::vec2(size + 6.0, size + 6.0)),
            )
        })
        .inner
        .on_hover_text(tooltip);
    let mut search = ui.data_mut(|data| data.get_temp::<String>(search_id).unwrap_or_default());
    let previous = value.clone();
    egui::Popup::menu(&response)
        .id(popup_id)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(config.width.max(260.0));
            searchable_icon_picker_contents(ui, value, &mut search, search_id, !was_open, config);
        });
    if egui::Popup::is_id_open(ui.ctx(), popup_id) {
        ui.data_mut(|data| data.insert_temp(search_id, search));
    } else {
        ui.data_mut(|data| data.remove::<String>(search_id));
    }
    *value != previous
}

pub fn searchable_control_icon_picker(
    ui: &mut eframe::egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    value: &mut String,
    width: f32,
    search_hint: &str,
    presets_label: &str,
    no_matches_label: &str,
) -> bool {
    searchable_icon_picker(
        ui,
        id_salt,
        value,
        IconPickerConfig {
            presets: CONTROL_ICON_PRESETS,
            fallback_glyph: SPARKLE,
            fallback_name: "Sparkle",
            width,
            show_selected_name: true,
            search_hint,
            presets_label,
            no_matches_label,
            clear_label: None,
        },
    )
}

pub fn searchable_workspace_icon_picker(
    ui: &mut eframe::egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    value: &mut String,
    width: f32,
    search_hint: &str,
    presets_label: &str,
    no_matches_label: &str,
) -> bool {
    searchable_icon_picker(
        ui,
        id_salt,
        value,
        IconPickerConfig {
            presets: WORKSPACE_ICON_PRESETS,
            fallback_glyph: APP_WINDOW,
            fallback_name: "Window",
            width,
            show_selected_name: true,
            search_hint,
            presets_label,
            no_matches_label,
            clear_label: None,
        },
    )
}

pub fn control(kind: &str, advertised: &str) -> &'static str {
    if let Some(icon) = named_control_icon(advertised) {
        return icon;
    }
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
    id_salt: impl std::hash::Hash + std::fmt::Debug,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_icon_presets_are_searchable_by_stable_name() {
        assert_eq!(named_control_icon("sparkle"), Some(SPARKLE));
        assert_eq!(named_control_icon("  LAMP "), Some(LAMP));
        assert_eq!(control_icon_name("seat"), Some("Seat"));
        assert_eq!(named_control_icon("not-a-preset"), None);
        assert!(icon_preset_matches("lightbulb", "Light bulb", "bulb"));
        assert!(icon_preset_matches("lightbulb", "Light bulb", "LIGHT"));
        assert!(!icon_preset_matches("seat", "Seat", "lamp"));
        assert!(icon_preset_matches("timeline", "Timeline", "time"));
    }

    #[test]
    fn control_icon_uses_kind_fallback_for_unknown_saved_names() {
        assert_eq!(control("relay", "custom-future-icon"), PLUG);
        assert_eq!(control("mosfet", ""), LIGHTBULB);
    }

    #[test]
    fn workspace_icons_share_stable_names_across_surfaces() {
        assert_eq!(workspace_icon("timeline"), WAVEFORM);
        assert_eq!(workspace_icon_name("monitor"), "Monitor");
        assert_eq!(workspace_icon("unknown"), APP_WINDOW);
    }

    #[test]
    fn every_native_icon_editor_uses_the_shared_searchable_picker() {
        let effects = include_str!("effects_library.rs");
        let hardware = include_str!("hardware_control.rs");
        let layout = include_str!("layout.rs");
        let workspaces = include_str!("workspace_profiles.rs");
        let four_d = include_str!("four_d.rs");

        assert!(effects.contains("searchable_control_icon_picker"));
        assert!(hardware.contains("searchable_icon_picker"));
        assert!(layout.contains("searchable_icon_button"));
        assert!(layout.contains("searchable_icon_picker"));
        assert!(workspaces.contains("searchable_workspace_icon_picker"));
        assert!(four_d.contains("searchable_control_icon_picker"));

        assert!(!hardware.contains("draw_control_icon_choices"));
        assert!(!layout.contains(concat!("effect_group_icon_", "preset")));
        assert!(!four_d.contains("text_edit_singleline(&mut icon)"));
    }

    fn picker_frame(
        context: &eframe::egui::Context,
        value: &mut String,
        compact: bool,
        events: Vec<eframe::egui::Event>,
    ) -> (
        eframe::egui::FullOutput,
        eframe::egui::Rect,
        eframe::egui::Id,
    ) {
        use eframe::egui;
        let mut button_rect = egui::Rect::NOTHING;
        let mut picker_id = egui::Id::NULL;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 700.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                button_rect = ui
                    .horizontal(|ui| {
                        picker_id = if compact {
                            ui.make_persistent_id("test-icon-picker")
                        } else {
                            ui.make_persistent_id(egui::IdSalt::new("test-icon-picker"))
                        };
                        let config = IconPickerConfig {
                            presets: CONTROL_ICON_PRESETS,
                            fallback_glyph: PLUG,
                            fallback_name: "Use channel default",
                            width: 260.0,
                            show_selected_name: true,
                            search_hint: "Search icons...",
                            presets_label: "Presets",
                            no_matches_label: "No matching icons",
                            clear_label: Some("Use channel default"),
                        };
                        if compact {
                            searchable_icon_button(
                                ui,
                                "test-icon-picker",
                                value,
                                18.0,
                                "Choose icon",
                                config,
                            );
                        } else {
                            searchable_icon_picker(ui, "test-icon-picker", value, config);
                        }
                        ui.min_rect()
                    })
                    .inner;
            },
        );
        output.textures_delta.clear();
        (output, button_rect, picker_id)
    }

    fn painted_text_rect(
        output: &eframe::egui::FullOutput,
        expected: &str,
    ) -> Option<eframe::egui::Rect> {
        fn find(shape: &eframe::egui::epaint::Shape, expected: &str) -> Option<eframe::egui::Rect> {
            match shape {
                eframe::egui::epaint::Shape::Text(text) if text.galley.job.text == expected => {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                eframe::egui::epaint::Shape::Vec(shapes) => {
                    shapes.iter().find_map(|shape| find(shape, expected))
                }
                _ => None,
            }
        }
        output
            .shapes
            .iter()
            .find_map(|shape| find(&shape.shape, expected))
    }

    fn click_picker(
        context: &eframe::egui::Context,
        value: &mut String,
        compact: bool,
        position: eframe::egui::Pos2,
    ) -> (
        eframe::egui::FullOutput,
        eframe::egui::Rect,
        eframe::egui::Id,
    ) {
        use eframe::egui;
        picker_frame(
            context,
            value,
            compact,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        picker_frame(
            context,
            value,
            compact,
            vec![egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )
    }

    #[test]
    fn icon_picker_search_click_filter_select_and_reopen_work_in_both_forms() {
        use eframe::egui;
        for compact in [false, true] {
            let context = egui::Context::default();
            let mut value = "sparkle".to_string();
            let (_, button, _) = picker_frame(&context, &mut value, compact, Vec::new());
            click_picker(&context, &mut value, compact, button.center());
            let (output, _, picker_id) = picker_frame(&context, &mut value, compact, Vec::new());
            let popup_id = picker_id.with("popup");
            assert!(egui::Popup::is_id_open(&context, popup_id));
            assert!(
                context.egui_wants_keyboard_input(),
                "search must receive focus"
            );

            let search = painted_text_rect(&output, "Search icons...")
                .expect("search is the first popup section");
            click_picker(&context, &mut value, compact, search.center());
            assert!(
                egui::Popup::is_id_open(&context, popup_id),
                "clicking the search field must not dismiss the picker"
            );
            let (output, _, _) = picker_frame(
                &context,
                &mut value,
                compact,
                vec![egui::Event::Text("la".to_string())],
            );
            assert!(painted_text_rect(&output, &format!("{LAMP}  Lamp")).is_some());
            picker_frame(&context, &mut value, compact, Vec::new());
            let (output, _, _) = picker_frame(
                &context,
                &mut value,
                compact,
                vec![egui::Event::Text("mp".to_string())],
            );
            let lamp = painted_text_rect(&output, &format!("{LAMP}  Lamp"))
                .expect("search must show the matching preset");
            assert!(painted_text_rect(&output, &format!("{SEAT}  Seat")).is_none());
            click_picker(&context, &mut value, compact, lamp.center());
            assert_eq!(value, "lamp");
            assert!(!egui::Popup::is_id_open(&context, popup_id));

            let (_, button, _) = picker_frame(&context, &mut value, compact, Vec::new());
            click_picker(&context, &mut value, compact, button.center());
            let (output, _, _) = picker_frame(&context, &mut value, compact, Vec::new());
            assert!(painted_text_rect(&output, "Search icons...").is_some());
            assert!(painted_text_rect(&output, &format!("{SEAT}  Seat")).is_some());

            let (output, _, _) = picker_frame(
                &context,
                &mut value,
                compact,
                vec![egui::Event::Text("unmatched-search".to_string())],
            );
            assert!(painted_text_rect(&output, "No matching icons").is_some());
            let default = painted_text_rect(&output, &format!("{PLUG}  Use channel default"))
                .expect("channel default remains available when searching");
            click_picker(&context, &mut value, compact, default.center());
            assert!(value.is_empty());
            assert!(!egui::Popup::is_id_open(&context, popup_id));
        }
    }
}
