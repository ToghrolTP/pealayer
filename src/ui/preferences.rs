use crate::app::PealayerApp;
use crate::config::{AppLanguage, AppTheme, OsdPosition, PlayerDragAction, VideoBackground};
use eframe::egui;

const TABS: [(&str, &str); 5] = [
    (crate::ui::icons::SPARKLE, "Appearance"),
    (crate::ui::icons::PLAY, "Playback"),
    (crate::ui::icons::PLUG, "Hardware"),
    (crate::ui::icons::SLIDERS_HORIZONTAL, "Input"),
    (crate::ui::icons::GEAR, "Advanced"),
];

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_preferences_dialog { return; }
    let mut open = app.show_preferences_dialog;
    egui::Window::new(app.tr("Preferences"))
        .open(&mut open)
        .default_size([590.0, 470.0])
        .min_size([500.0, 360.0])
        .max_size([980.0, 820.0])
        .resizable(true)
        .collapsible(false)
        .show(ui.ctx(), |ui| {
            let mut changed = false;
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(138.0);
                    ui.spacing_mut().item_spacing.y = 5.0;
                    for (index, (icon, tab)) in TABS.into_iter().enumerate() {
                        if ui.add_sized(
                            [132.0, 34.0],
                            egui::Button::new(format!("{icon}  {}", app.tr(tab)))
                                .selected(app.preferences_tab == index),
                        ).clicked() { app.preferences_tab = index; }
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().id_salt("preferences_content").show(ui, |ui| {
                    ui.set_min_width(330.0);
                    ui.spacing_mut().item_spacing.y = 8.0;
                    match app.preferences_tab {
                        0 => appearance_preferences(app, ui, &mut changed),
                        1 => playback_preferences(app, ui, &mut changed),
                        2 => hardware_preferences(app, ui, &mut changed),
                        3 => input_preferences(app, ui, &mut changed),
                        _ => advanced_preferences(app, ui),
                    }
                });
            });
            if changed { app.save_config(); }
        });
    app.show_preferences_dialog = open;
}

fn appearance_preferences(app: &mut PealayerApp, ui: &mut egui::Ui, changed: &mut bool) {
    ui.heading(app.tr("Appearance and language"));
    egui::Grid::new("appearance_preferences_grid").num_columns(2).spacing([18.0, 12.0]).show(ui, |ui| {
        ui.label(app.tr("Theme"));
        ui.horizontal(|ui| {
            for (theme, label) in [(AppTheme::System, "System"), (AppTheme::Light, "Light"), (AppTheme::Dark, "Dark")] {
                if ui.selectable_label(app.theme_preference == theme, app.tr(label)).clicked() { app.set_theme(ui.ctx(), theme); }
            }
        });
        ui.end_row();
        ui.label(app.tr("Language"));
        egui::ComboBox::from_id_salt("preferences_language")
            .selected_text(match app.language_preference { AppLanguage::System => app.tr("System language"), AppLanguage::English => app.tr("English"), AppLanguage::Persian => app.tr("Persian") })
            .show_ui(ui, |ui| {
                for (language, label) in [(AppLanguage::System, "System language"), (AppLanguage::English, "English"), (AppLanguage::Persian, "Persian")] {
                    if ui.selectable_label(app.language_preference == language, app.tr(label)).clicked() { app.set_language(ui.ctx(), language); }
                }
            });
        ui.end_row();
        ui.label(app.tr("Fullscreen background"));
        let background_label = match app.fullscreen_video_background { VideoBackground::Black => app.tr("Black"), VideoBackground::DarkGray => app.tr("Dark gray"), VideoBackground::Theme => app.tr("Use app theme") };
        egui::ComboBox::from_id_salt("fullscreen_video_background").selected_text(background_label).show_ui(ui, |ui| {
            *changed |= ui.selectable_value(&mut app.fullscreen_video_background, VideoBackground::Black, app.tr("Black")).changed();
            *changed |= ui.selectable_value(&mut app.fullscreen_video_background, VideoBackground::DarkGray, app.tr("Dark gray")).changed();
            *changed |= ui.selectable_value(&mut app.fullscreen_video_background, VideoBackground::Theme, app.tr("Use app theme")).changed();
        });
        ui.end_row();
    });
    ui.separator();
    ui.strong(app.tr("On-screen display"));
    egui::ComboBox::from_id_salt("preferences_osd_position").selected_text(match app.osd_position { OsdPosition::TopLeft => app.tr("Top left"), OsdPosition::Center => app.tr("Center") }).show_ui(ui, |ui| {
        *changed |= ui.selectable_value(&mut app.osd_position, OsdPosition::TopLeft, app.tr("Top left")).changed();
        *changed |= ui.selectable_value(&mut app.osd_position, OsdPosition::Center, app.tr("Center")).changed();
    });
    let timeout_label = app.tr("OSD timeout (seconds)");
    *changed |= ui.add(egui::Slider::new(&mut app.osd_timeout_seconds, 1.0..=12.0).text(timeout_label)).changed();
}

fn playback_preferences(app: &mut PealayerApp, ui: &mut egui::Ui, changed: &mut bool) {
    ui.heading(app.tr("Playback behavior"));
    let click_label = app.tr("Single-click the picture to play or pause");
    *changed |= ui.checkbox(&mut app.click_player_to_toggle, click_label).changed();
    let subseconds_label = app.tr("Show milliseconds in time displays");
    *changed |= ui.checkbox(&mut app.show_subseconds, subseconds_label).changed();
    let wheel_label = app.tr("Mouse-wheel seek step (seconds)");
    *changed |= ui.add(egui::Slider::new(&mut app.wheel_seek_seconds, 0.1..=60.0).logarithmic(true).text(wheel_label)).changed();
    ui.label(app.tr("Click the duration display to toggle total and remaining time."));
}

fn hardware_preferences(app: &mut PealayerApp, ui: &mut egui::Ui, changed: &mut bool) {
    ui.heading(app.tr("PCController and hardware"));
    let reconnect_label = app.tr("Discover and connect to PCController on startup");
    *changed |= ui.checkbox(&mut app.auto_connect_hardware, reconnect_label).changed();
    let pause_label = app.tr("Pause playback when hardware disconnects unexpectedly");
    *changed |= ui.checkbox(&mut app.pause_on_hardware_disconnect, pause_label).changed();
    ui.label(app.tr("Preferred endpoint"));
    let endpoint_hint = app.tr("pccontroller://host:port, tcp://host:port, or direct:<device>");
    *changed |= ui.add(egui::TextEdit::singleline(&mut app.serial_port).desired_width(300.0).hint_text(endpoint_hint)).changed();
    if let Some(notice) = &app.connection_notice { ui.colored_label(ui.visuals().warn_fg_color, notice); }
}

fn input_preferences(app: &mut PealayerApp, ui: &mut egui::Ui, changed: &mut bool) {
    ui.heading(app.tr("Mouse and gesture bindings"));
    *changed |= drag_action_selector(ui, app.tr("Drag while paused"), &mut app.paused_drag_action);
    *changed |= drag_action_selector(ui, app.tr("Drag while playing"), &mut app.playing_drag_action);
    ui.label(app.tr("Ctrl+wheel over the picture adjusts volume; wheel seeks in the reversed Y direction."));
}

fn advanced_preferences(app: &mut PealayerApp, ui: &mut egui::Ui) {
    ui.heading(app.tr("Configuration"));
    ui.label(app.tr("Settings are stored in the active portable or per-user configuration file."));
    ui.horizontal_wrapped(|ui| { ui.label(app.tr("Config file")); ui.monospace(crate::config::AppConfig::get_config_path().display().to_string()); });
    if ui.button(format!("{} {}", crate::ui::icons::FLOPPY_DISK, app.tr("Save now"))).clicked() { app.save_config(); app.set_osd(app.tr("Preferences saved")); }
}

fn drag_action_selector(ui: &mut egui::Ui, label: String, value: &mut PlayerDragAction) -> bool {
    let before = *value;
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(ui.next_auto_id()).selected_text(drag_action_name(*value)).show_ui(ui, |ui| {
            for action in [PlayerDragAction::MoveWindow, PlayerDragAction::Seek, PlayerDragAction::TemporaryFastForward, PlayerDragAction::None] { ui.selectable_value(value, action, drag_action_name(action)); }
        });
    });
    before != *value
}

fn drag_action_name(action: PlayerDragAction) -> &'static str {
    match action { PlayerDragAction::MoveWindow => "Move window", PlayerDragAction::Seek => "Seek", PlayerDragAction::TemporaryFastForward => "Temporary fast-forward", PlayerDragAction::None => "No action" }
}
