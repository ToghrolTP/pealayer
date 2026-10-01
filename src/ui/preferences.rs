use crate::app::PealayerApp;
use crate::config::{
    AppLanguage, AppTheme, MotionControlMode, OsdPosition, PlayerDragAction, VideoBackground,
};
use eframe::egui;

const TABS: [(&str, &str); 5] = [
    (crate::ui::icons::SPARKLE, "Appearance"),
    (crate::ui::icons::PLAY, "Playback"),
    (crate::ui::icons::PLUG, "Hardware"),
    (crate::ui::icons::SLIDERS_HORIZONTAL, "Input"),
    (crate::ui::icons::GEAR, "Advanced"),
];

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_preferences_dialog {
        return;
    }
    let mut open = app.show_preferences_dialog;
    let bounds = ui.ctx().content_rect().shrink(18.0);
    let max_size = egui::vec2(bounds.width().min(700.0), bounds.height().min(620.0));
    let default_size = egui::vec2(max_size.x.min(620.0), max_size.y.min(520.0));
    let min_size = egui::vec2(max_size.x.min(390.0), max_size.y.min(330.0));
    let default_rect = crate::ui::dialog::centered_default_rect(bounds, default_size);
    if crate::ui::dialog::escape_pressed(ui.ctx()) {
        open = false;
    }
    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::GEAR,
        app.tr("Preferences")
    ))
    .id(egui::Id::new("preferences_dialog_bounded_v2"))
    .open(&mut open)
    .default_rect(default_rect)
    .min_size(min_size)
    .max_size(max_size)
    .constrain_to(bounds)
    .resizable(true)
    .movable(true)
    .collapsible(false)
    .show(ui.ctx(), |ui| {
        let mut changed = false;
        // A vertical tab rail plus the minimum useful settings column needs
        // considerably more than 500 points. Switch before either side starts
        // squeezing controls into overlapping or single-glyph columns.
        let narrow = ui.available_width() < 580.0;
        if narrow {
            draw_compact_tab_selector(app, ui);
            ui.separator();
            draw_preferences_content(app, ui, &mut changed);
        } else {
            let content_height = ui.available_height();
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(126.0, content_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| draw_tabs(app, ui, false),
                );
                ui.separator();
                draw_preferences_content(app, ui, &mut changed);
            });
        }
        if changed {
            app.save_config();
        }
    });
    app.show_preferences_dialog = open;
}

fn draw_tabs(app: &mut PealayerApp, ui: &mut egui::Ui, compact: bool) {
    ui.spacing_mut().item_spacing.y = 5.0;
    for (index, (icon, tab)) in TABS.into_iter().enumerate() {
        let width = if compact {
            (ui.available_width() / 2.0 - 4.0).max(112.0)
        } else {
            120.0
        };
        if ui
            .add_sized(
                [width, 32.0],
                egui::Button::new(format!("{icon}  {}", app.tr(tab)))
                    .selected(app.preferences_tab == index),
            )
            .clicked()
        {
            app.preferences_tab = index;
        }
    }
}

fn draw_compact_tab_selector(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let (active_icon, active_name) = TABS[app.preferences_tab.min(TABS.len() - 1)];
    egui::ComboBox::from_id_salt("preferences_compact_tab")
        .width(ui.available_width())
        .selected_text(format!("{active_icon}  {}", app.tr(active_name)))
        .show_ui(ui, |ui| {
            for (index, (icon, name)) in TABS.into_iter().enumerate() {
                if ui
                    .selectable_label(
                        app.preferences_tab == index,
                        format!("{icon}  {}", app.tr(name)),
                    )
                    .clicked()
                {
                    app.preferences_tab = index;
                }
            }
        });
}

fn draw_preferences_content(app: &mut PealayerApp, ui: &mut egui::Ui, changed: &mut bool) {
    let detail_width = ui.available_width();
    crate::ui::dialog::scroll_column(ui, "preferences_content_v2", None, |ui| {
        ui.set_max_width((detail_width - 8.0).max(180.0));
        ui.spacing_mut().item_spacing.y = 8.0;
        match app.preferences_tab {
            0 => appearance_preferences(app, ui, changed),
            1 => playback_preferences(app, ui, changed),
            2 => hardware_preferences(app, ui, changed),
            3 => input_preferences(app, ui, changed),
            _ => advanced_preferences(app, ui),
        }
    });
}

fn appearance_preferences(app: &mut PealayerApp, ui: &mut egui::Ui, changed: &mut bool) {
    ui.heading(app.tr("Appearance and language"));
    preference_section(ui, crate::ui::icons::SPARKLE, &app.tr("Interface"), |ui| {
        preference_grid(ui, "appearance_preferences_grid", |ui| {
            ui.label(app.tr("Theme"));
            let theme_label = match app.theme_preference {
                AppTheme::System => app.tr("System"),
                AppTheme::Light => app.tr("Light"),
                AppTheme::Dark => app.tr("Dark"),
            };
            egui::ComboBox::from_id_salt("preferences_theme")
                .selected_text(theme_label)
                .show_ui(ui, |ui| {
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
            ui.end_row();
            ui.label(app.tr("Language"));
            egui::ComboBox::from_id_salt("preferences_language")
                .selected_text(match app.language_preference {
                    AppLanguage::System => app.tr("System language"),
                    AppLanguage::English => app.tr("English"),
                    AppLanguage::Persian => app.tr("Persian"),
                })
                .show_ui(ui, |ui| {
                    for (language, label) in [
                        (AppLanguage::System, "System language"),
                        (AppLanguage::English, "English"),
                        (AppLanguage::Persian, "Persian"),
                    ] {
                        if ui
                            .selectable_label(app.language_preference == language, app.tr(label))
                            .clicked()
                        {
                            app.set_language(ui.ctx(), language);
                        }
                    }
                });
            ui.end_row();
            ui.label(app.tr("Fullscreen background"));
            let background_label = match app.fullscreen_video_background {
                VideoBackground::Black => app.tr("Black"),
                VideoBackground::DarkGray => app.tr("Dark gray"),
                VideoBackground::Theme => app.tr("Use app theme"),
            };
            let black_label = app.tr("Black");
            let dark_gray_label = app.tr("Dark gray");
            let theme_label = app.tr("Use app theme");
            egui::ComboBox::from_id_salt("fullscreen_video_background")
                .selected_text(background_label)
                .show_ui(ui, |ui| {
                    *changed |= ui
                        .selectable_value(
                            &mut app.fullscreen_video_background,
                            VideoBackground::Black,
                            black_label,
                        )
                        .changed();
                    *changed |= ui
                        .selectable_value(
                            &mut app.fullscreen_video_background,
                            VideoBackground::DarkGray,
                            dark_gray_label,
                        )
                        .changed();
                    *changed |= ui
                        .selectable_value(
                            &mut app.fullscreen_video_background,
                            VideoBackground::Theme,
                            theme_label,
                        )
                        .changed();
                });
            ui.end_row();
        });
    });
    preference_section(
        ui,
        crate::ui::icons::MONITOR_PLAY,
        &app.tr("On-screen display"),
        |ui| {
            let top_left_label = app.tr("Top left");
            let center_label = app.tr("Center");
            preference_grid(ui, "osd_preferences_grid", |ui| {
                ui.label(app.tr("Position"));
                egui::ComboBox::from_id_salt("preferences_osd_position")
                    .selected_text(match app.osd_position {
                        OsdPosition::TopLeft => top_left_label.clone(),
                        OsdPosition::Center => center_label.clone(),
                    })
                    .show_ui(ui, |ui| {
                        *changed |= ui
                            .selectable_value(
                                &mut app.osd_position,
                                OsdPosition::TopLeft,
                                top_left_label,
                            )
                            .changed();
                        *changed |= ui
                            .selectable_value(
                                &mut app.osd_position,
                                OsdPosition::Center,
                                center_label,
                            )
                            .changed();
                    });
                ui.end_row();
            });
            let timeout_label = app.tr("OSD timeout (seconds)");
            *changed |= ui
                .add(
                    egui::Slider::new(&mut app.osd_timeout_seconds, 1.0..=12.0).text(timeout_label),
                )
                .changed();
        },
    );
}

fn playback_preferences(app: &mut PealayerApp, ui: &mut egui::Ui, changed: &mut bool) {
    ui.heading(app.tr("Playback behavior"));
    preference_section(
        ui,
        crate::ui::icons::PLAY,
        &app.tr("Player controls"),
        |ui| {
            let click_label = app.tr("Single-click the picture to play or pause");
            *changed |= ui
                .checkbox(&mut app.click_player_to_toggle, click_label)
                .changed();
            let subseconds_label = app.tr("Show milliseconds in time displays");
            *changed |= ui
                .checkbox(&mut app.show_subseconds, subseconds_label)
                .changed();
            let wheel_label = app.tr("Mouse-wheel seek step (seconds)");
            *changed |= ui
                .add(
                    egui::Slider::new(&mut app.wheel_seek_seconds, 0.1..=60.0)
                        .logarithmic(true)
                        .text(wheel_label),
                )
                .changed();
            ui.label(
                egui::RichText::new(
                    app.tr("Click the duration display to toggle total and remaining time."),
                )
                .weak(),
            );
        },
    );
}

fn hardware_preferences(app: &mut PealayerApp, ui: &mut egui::Ui, changed: &mut bool) {
    ui.heading(app.tr("PCController and hardware"));
    preference_section(ui, crate::ui::icons::PLUG, &app.tr("Connection"), |ui| {
        let reconnect_label = app.tr("Discover and connect to PCController on startup");
        *changed |= ui
            .checkbox(&mut app.auto_connect_hardware, reconnect_label)
            .changed();
        let pause_label = app.tr("Pause playback when hardware disconnects unexpectedly");
        *changed |= ui
            .checkbox(&mut app.pause_on_hardware_disconnect, pause_label)
            .changed();
        ui.label(app.tr("Preferred endpoint"));
        let endpoint_hint = app.tr("pccontroller://host:port, tcp://host:port, or direct:<device>");
        *changed |= ui
            .add(
                egui::TextEdit::singleline(&mut app.serial_port)
                    .desired_width(ui.available_width())
                    .hint_text(endpoint_hint),
            )
            .changed();
        if let Some(notice) = &app.connection_notice {
            ui.colored_label(ui.visuals().warn_fg_color, notice);
        }
    });
    preference_section(
        ui,
        crate::ui::icons::SEAT,
        &app.tr("Motion controls"),
        |ui| {
            let toggle_label = app.tr("Toggle on press");
            let hold_label = app.tr("Run only while held");
            let compact_label = app.tr("Use one-row compact hardware controls");
            preference_grid(ui, "motion_control_preferences", |ui| {
                ui.label(app.tr("Button behavior"));
                egui::ComboBox::from_id_salt("motion_control_mode")
                    .selected_text(match app.motion_control_mode {
                        MotionControlMode::Toggle => toggle_label.clone(),
                        MotionControlMode::Hold => hold_label.clone(),
                    })
                    .show_ui(ui, |ui| {
                        *changed |= ui
                            .selectable_value(
                                &mut app.motion_control_mode,
                                MotionControlMode::Toggle,
                                &toggle_label,
                            )
                            .changed();
                        *changed |= ui
                            .selectable_value(
                                &mut app.motion_control_mode,
                                MotionControlMode::Hold,
                                &hold_label,
                            )
                            .changed();
                    });
                ui.end_row();
            });
            *changed |= ui
                .checkbox(&mut app.compact_hardware_controls, compact_label)
                .changed();
            ui.label(
                egui::RichText::new(app.tr(
                    "Toggle mode keeps a direction active until another action is chosen. Hold mode sends Stop when the pressed direction is released.",
                ))
                .small()
                .weak(),
            );
        },
    );
}

fn input_preferences(app: &mut PealayerApp, ui: &mut egui::Ui, changed: &mut bool) {
    ui.heading(app.tr("Mouse and gesture bindings"));
    preference_section(
        ui,
        crate::ui::icons::SELECTION_ALL,
        &app.tr("Video surface"),
        |ui| {
            *changed |=
                drag_action_selector(ui, app.tr("Drag while paused"), &mut app.paused_drag_action);
            *changed |= drag_action_selector(
                ui,
                app.tr("Drag while playing"),
                &mut app.playing_drag_action,
            );
            ui.label(egui::RichText::new(app.tr("Ctrl+wheel over the picture adjusts volume; wheel seeks in the reversed Y direction.")).weak());
        },
    );
}

fn advanced_preferences(app: &mut PealayerApp, ui: &mut egui::Ui) {
    ui.heading(app.tr("Configuration"));
    preference_section(
        ui,
        crate::ui::icons::APP_WINDOW,
        &app.tr("Windows graphics and composition"),
        |ui| {
            let mut changed = false;
            let dwm_label = app.tr("Use DWM title-bar theming");
            let mica_label = app.tr("Use Mica backdrop (may flicker with some OpenGL drivers)");
            let vsync_label = app.tr("Use OpenGL vertical sync");
            changed |= ui
                .checkbox(&mut app.windows_dwm_theming, dwm_label)
                .changed();
            changed |= ui
                .checkbox(&mut app.windows_mica_backdrop, mica_label)
                .changed();
            changed |= ui.checkbox(&mut app.opengl_vsync, vsync_label).changed();
            ui.label(
                egui::RichText::new(app.tr(
                    "Mica and DWM changes apply immediately. OpenGL vertical sync applies after restart.",
                ))
                .small()
                .weak(),
            );
            if changed {
                crate::platform::windows::configure_window_composition(
                    app.windows_dwm_theming,
                    app.windows_mica_backdrop,
                );
                app.save_config();
            }
        },
    );
    preference_section(ui, crate::ui::icons::GAUGE, &app.tr("Status bar"), |ui| {
        let hardware_label = app.tr("Hardware connection");
        let rgb_label = app.tr("Physical status RGB");
        let warnings_label = app.tr("Hardware warnings");
        let media_rate_label = app.tr("Media rate");
        let telemetry_label = app.tr("Telemetry");
        let workspace_label = app.tr("Workspace mode");
        let mut changed = false;
        changed |= ui
            .checkbox(&mut app.status_bar.hardware, hardware_label)
            .changed();
        changed |= ui
            .checkbox(&mut app.status_bar.status_rgb, rgb_label)
            .changed();
        changed |= ui
            .checkbox(&mut app.status_bar.warnings, warnings_label)
            .changed();
        changed |= ui
            .checkbox(&mut app.status_bar.media_rate, media_rate_label)
            .changed();
        changed |= ui
            .checkbox(&mut app.status_bar.telemetry, telemetry_label)
            .changed();
        changed |= ui
            .checkbox(&mut app.status_bar.workspace, workspace_label)
            .changed();
        if changed {
            app.save_config();
        }
    });
    preference_section(
        ui,
        crate::ui::icons::FLOPPY_DISK,
        &app.tr("Config file"),
        |ui| {
            ui.label(app.tr("Settings are stored atomically in the active portable or OS-native per-user JSON configuration and watched for external changes."));
            ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        crate::config::AppConfig::get_config_path()
                            .display()
                            .to_string(),
                    )
                    .monospace(),
                )
                .wrap(),
            );
            ui.horizontal(|ui| {
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::FLOPPY_DISK,
                        app.tr("Save now")
                    ))
                    .clicked()
                {
                    app.save_config();
                    app.set_osd(app.tr("Preferences saved"));
                }
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                        app.tr("Reload from disk")
                    ))
                    .clicked()
                {
                    match app.reload_config_from_disk(ui.ctx()) {
                        Ok(()) => app.set_osd(app.tr("Preferences reloaded from disk")),
                        Err(error) => app.set_osd(error),
                    }
                }
            });
            if !app.config_status.is_empty() {
                ui.label(egui::RichText::new(&app.config_status).small().weak());
            }
            ui.label(
                egui::RichText::new(
                    "API: GET/POST /api/config · JSON-RPC pealayer.config.get/update/reload",
                )
                .small()
                .monospace()
                .weak(),
            );
        },
    );
}

fn drag_action_selector(ui: &mut egui::Ui, label: String, value: &mut PlayerDragAction) -> bool {
    let before = *value;
    egui::Grid::new(ui.next_auto_id())
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
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
            ui.end_row();
        });
    before != *value
}

fn preference_section(
    ui: &mut egui::Ui,
    icon: &str,
    title: &str,
    body: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(12))
        .corner_radius(9.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(format!("{icon}  {title}")).strong());
                ui.add_space(6.0);
                body(ui);
            });
        });
    ui.add_space(8.0);
}

fn preference_grid(ui: &mut egui::Ui, id: &'static str, body: impl FnOnce(&mut egui::Ui)) {
    let narrow = ui.available_width() < 360.0;
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([if narrow { 8.0 } else { 18.0 }, 10.0])
        .min_col_width(if narrow { 72.0 } else { 120.0 })
        .show(ui, body);
}

fn drag_action_name(action: PlayerDragAction) -> &'static str {
    match action {
        PlayerDragAction::MoveWindow => "Move window",
        PlayerDragAction::Seek => "Seek",
        PlayerDragAction::TemporaryFastForward => "Temporary fast-forward",
        PlayerDragAction::None => "No action",
    }
}
