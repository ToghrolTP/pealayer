use crate::app::PealayerApp;
use crate::config::{AppLanguage, AppTheme, OsdPosition, PlayerDragAction};
use eframe::egui;

const TABS: [&str; 5] = ["Appearance", "Playback", "Hardware", "Input", "Advanced"];

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_preferences_dialog {
        return;
    }

    let mut open = app.show_preferences_dialog;
    let title = app.tr("Preferences");
    egui::Window::new(title)
        .open(&mut open)
        .default_width(680.0)
        .min_width(560.0)
        .resizable(true)
        .collapsible(false)
        .show(ui.ctx(), |ui| {
            ui.horizontal_wrapped(|ui| {
                for (index, tab) in TABS.into_iter().enumerate() {
                    if ui
                        .selectable_label(app.preferences_tab == index, app.tr(tab))
                        .clicked()
                    {
                        app.preferences_tab = index;
                    }
                }
            });
            ui.separator();

            let mut changed = false;
            match app.preferences_tab {
                0 => {
                    ui.heading(app.tr("Appearance and language"));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(app.tr("Theme"));
                        for (theme, label) in [
                            (AppTheme::System, "System"),
                            (AppTheme::Light, "Light"),
                            (AppTheme::Dark, "Dark"),
                        ] {
                            if ui
                                .selectable_label(app.theme_preference == theme, app.tr(label))
                                .clicked()
                            {
                                app.set_theme(ui.ctx(), theme);
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(app.tr("Language"));
                        for (language, label) in [
                            (AppLanguage::System, "System language"),
                            (AppLanguage::English, "English"),
                            (AppLanguage::Persian, "Persian"),
                        ] {
                            if ui
                                .selectable_label(
                                    app.language_preference == language,
                                    app.tr(label),
                                )
                                .clicked()
                            {
                                app.set_language(ui.ctx(), language);
                            }
                        }
                    });
                    ui.add_space(8.0);
                    ui.label(app.tr("On-screen display"));
                    let top_left_label = app.tr("Top left");
                    let center_label = app.tr("Center");
                    egui::ComboBox::from_id_salt("preferences_osd_position")
                        .selected_text(match app.osd_position {
                            OsdPosition::TopLeft => top_left_label.clone(),
                            OsdPosition::Center => center_label.clone(),
                        })
                        .show_ui(ui, |ui| {
                            changed |= ui
                                .selectable_value(
                                    &mut app.osd_position,
                                    OsdPosition::TopLeft,
                                    top_left_label,
                                )
                                .changed();
                            changed |= ui
                                .selectable_value(
                                    &mut app.osd_position,
                                    OsdPosition::Center,
                                    center_label,
                                )
                                .changed();
                        });
                    let osd_timeout_label = app.tr("OSD timeout (seconds)");
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut app.osd_timeout_seconds, 1.0..=10.0)
                                .text(osd_timeout_label),
                        )
                        .changed();
                }
                1 => {
                    ui.heading(app.tr("Playback behavior"));
                    let click_to_toggle_label =
                        app.tr("Single-click the picture to play or pause");
                    changed |= ui
                        .checkbox(
                            &mut app.click_player_to_toggle,
                            click_to_toggle_label,
                        )
                        .changed();
                    let subseconds_label = app.tr("Show milliseconds in time displays");
                    changed |= ui
                        .checkbox(
                            &mut app.show_subseconds,
                            subseconds_label,
                        )
                        .changed();
                    let wheel_seek_label = app.tr("Mouse-wheel seek step (seconds)");
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut app.wheel_seek_seconds, 0.1..=60.0)
                                .logarithmic(true)
                                .text(wheel_seek_label),
                        )
                        .changed();
                    ui.label(app.tr("Click the duration display to toggle total and remaining time."));
                }
                2 => {
                    ui.heading(app.tr("PCController and hardware"));
                    let reconnect_label =
                        app.tr("Reconnect the last healthy hardware endpoint on startup");
                    changed |= ui
                        .checkbox(
                            &mut app.auto_connect_hardware,
                            reconnect_label,
                        )
                        .changed();
                    let pause_disconnect_label =
                        app.tr("Pause playback when hardware disconnects unexpectedly");
                    changed |= ui
                        .checkbox(
                            &mut app.pause_on_hardware_disconnect,
                            pause_disconnect_label,
                        )
                        .changed();
                    ui.label(app.tr("Preferred endpoint"));
                    let endpoint_hint = app.tr(
                        "pccontroller://host:port, tcp://host:port, or direct:<device>",
                    );
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(&mut app.serial_port)
                                .hint_text(endpoint_hint),
                        )
                        .changed();
                    if let Some(notice) = &app.connection_notice {
                        ui.colored_label(ui.visuals().warn_fg_color, notice);
                    }
                }
                3 => {
                    ui.heading(app.tr("Mouse and gesture bindings"));
                    changed |= drag_action_selector(
                        ui,
                        app.tr("Drag while paused"),
                        &mut app.paused_drag_action,
                    );
                    changed |= drag_action_selector(
                        ui,
                        app.tr("Drag while playing"),
                        &mut app.playing_drag_action,
                    );
                    ui.label(app.tr("Ctrl+wheel over the picture adjusts volume; wheel seeks in the reversed Y direction."));
                }
                _ => {
                    ui.heading(app.tr("Configuration"));
                    ui.label(app.tr("Settings are stored in the active portable or per-user configuration file."));
                    ui.horizontal(|ui| {
                        ui.label(app.tr("Config file"));
                        ui.monospace(crate::config::AppConfig::get_config_path().display().to_string());
                    });
                    if ui.button(app.tr("Save now")).clicked() {
                        app.save_config();
                        app.set_osd(app.tr("Preferences saved"));
                    }
                }
            }

            if changed {
                app.save_config();
            }
        });
    app.show_preferences_dialog = open;
}

fn drag_action_selector(
    ui: &mut egui::Ui,
    label: String,
    value: &mut PlayerDragAction,
) -> bool {
    let before = *value;
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(ui.next_auto_id())
            .selected_text(drag_action_name(*value))
            .show_ui(ui, |ui| {
                for action in [
                    PlayerDragAction::MoveWindow,
                    PlayerDragAction::Seek,
                    PlayerDragAction::TemporaryFastForward,
                    PlayerDragAction::None,
                ] {
                    ui.selectable_value(value, action, drag_action_name(action));
                }
            });
    });
    before != *value
}

fn drag_action_name(action: PlayerDragAction) -> &'static str {
    match action {
        PlayerDragAction::MoveWindow => "Move window",
        PlayerDragAction::Seek => "Seek",
        PlayerDragAction::TemporaryFastForward => "Temporary fast-forward",
        PlayerDragAction::None => "No action",
    }
}
