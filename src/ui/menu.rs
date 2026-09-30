use crate::app::PealayerApp;
use eframe::egui;

fn estop_button(
    ui: &mut egui::Ui,
    active: bool,
    language: crate::config::AppLanguage,
) -> egui::Response {
    let (label, fill, help) = if active {
        (
            crate::ui::i18n::tr(language, "RESET E-STOP"),
            egui::Color32::from_rgb(231, 76, 60),
            "Reset the active emergency stop",
        )
    } else {
        (
            crate::ui::i18n::tr(language, "E-STOP"),
            egui::Color32::from_rgb(192, 57, 43),
            "Emergency stop: pause playback and stop hardware output",
        )
    };

    // The stop mark is painted as a vector octagon instead of relying on an
    // emoji glyph, whose appearance and availability vary by platform/font.
    let response = ui.add_sized(
        egui::vec2(if active { 132.0 } else { 104.0 }, 26.0),
        egui::Button::new(
            egui::RichText::new(format!("      {label}"))
                .color(egui::Color32::WHITE)
                .strong()
                .size(11.0),
        )
        .fill(fill),
    );
    let center = egui::pos2(response.rect.left() + 15.0, response.rect.center().y);
    let radius = 8.0;
    let points = (0..8)
        .map(|index| {
            let angle = std::f32::consts::FRAC_PI_8 + index as f32 * std::f32::consts::FRAC_PI_4;
            center + egui::vec2(angle.cos(), angle.sin()) * radius
        })
        .collect();
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        egui::Color32::WHITE,
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(110, 20, 20)),
    ));
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        "!",
        egui::FontId::proportional(10.0),
        fill,
    );
    response.on_hover_text(help)
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let language = app.language;
    let rtl = app.rtl;

    egui::Panel::top("menu_bar").show_inside(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.with_layout(crate::ui::i18n::layout(rtl, egui::Align::Center), |ui| {
                ui.menu_button(app.tr("File"), |ui| {
                    if ui.button(app.tr("Open Video File...")).clicked() {
                        ui.close();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Video Files", &["mp4", "mkv", "avi", "webm", "mov", "flv"])
                            .pick_file()
                        {
                            app.load_video_file(path);
                        }
                    }

                    if ui.button(app.tr("Open Location / URL...")).clicked() {
                        ui.close();
                        app.show_open_url_dialog = true;
                    }

                    ui.menu_button(app.tr("Open Recent"), |ui| {
                        if app.recent_media.is_empty() {
                            ui.label(app.tr("No recent media"));
                        } else {
                            for path in app.recent_media.clone() {
                                let file_name = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("Unknown");
                                if ui
                                    .button(app.display_text(file_name))
                                    .on_hover_text(path.display().to_string())
                                    .clicked()
                                {
                                    ui.close();
                                    app.load_video_file(path);
                                }
                            }
                            ui.separator();
                            if ui.button(app.tr("Clear Recent")).clicked() {
                                ui.close();
                                app.clear_recent_media();
                            }
                        }
                    });

                    let has_video = app.current_video_path.is_some();
                    if ui
                        .add_enabled(has_video, egui::Button::new(app.tr("Close Video")))
                        .clicked()
                    {
                        ui.close();
                        app.close_video();
                    }

                    if ui.button(app.tr("Open Timeline Project...")).clicked() {
                        ui.close();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Pealayer Timeline", &["json"])
                            .pick_file()
                        {
                            match crate::four_d::models::Timeline::load_from_file(&path) {
                                Ok(timeline) => {
                                    app.timeline = timeline;
                                    let compiled = crate::four_d::engine::compile_timeline(
                                        &app.timeline,
                                        &app.track_muted,
                                        &app.track_soloed,
                                    );
                                    let _ = app.engine_handle.sender.send(
                                        crate::four_d::engine::EngineMessage::UpdateQueue(compiled),
                                    );
                                }
                                Err(e) => {
                                    app.show_error =
                                        Some(format!("Failed to load timeline: {}", e));
                                }
                            }
                        }
                    }

                    ui.separator();

                    let save_enabled = app.current_video_path.is_some();
                    let save_btn = egui::Button::new(app.tr("Save Timeline (Sidecar)"));
                    if ui.add_enabled(save_enabled, save_btn).clicked() {
                        ui.close();
                        if let Some(ref video_path) = app.current_video_path {
                            let mut sidecar = video_path.clone();
                            sidecar.set_extension("4d.json");
                            if let Err(e) = app.timeline.save_to_file(&sidecar) {
                                app.show_error = Some(format!("Failed to save timeline: {}", e));
                            }
                        }
                    }

                    if ui.button(app.tr("Save Timeline As...")).clicked() {
                        ui.close();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Pealayer Timeline", &["json"])
                            .save_file()
                        {
                            if let Err(e) = app.timeline.save_to_file(&path) {
                                app.show_error = Some(format!("Failed to save timeline: {}", e));
                            }
                        }
                    }

                    ui.separator();
                    if ui
                        .button(format!(
                            "{} {}", crate::ui::icons::GEAR,
                            app.tr("Register as Default Media Player...")
                        ))
                        .clicked()
                    {
                        ui.close();
                        match crate::platform::association::register_as_default_player() {
                            Ok(msg) => app.set_osd(msg),
                            Err(err) => app.show_error = Some(err),
                        }
                    }

                    ui.separator();
                    if ui.button(app.tr("Quit")).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button(app.tr("Edit"), |ui| {
                    if ui
                        .button(format!("{} {}", crate::ui::icons::GEAR, app.tr("Preferences...")))
                        .clicked()
                    {
                        app.show_preferences_dialog = true;
                        ui.close();
                    }
                    ui.separator();
                    let mut undo_btn = egui::Button::new(app.tr("Undo"));
                    undo_btn = undo_btn.shortcut_text("Ctrl+Z");
                    if ui.add_enabled(false, undo_btn).clicked() {
                        ui.close();
                    }

                    let mut redo_btn = egui::Button::new(app.tr("Redo"));
                    redo_btn = redo_btn.shortcut_text("Ctrl+Y");
                    if ui.add_enabled(false, redo_btn).clicked() {
                        ui.close();
                    }
                });

                ui.menu_button(app.tr("Audio"), |ui| {
                    ui.menu_button(app.tr("Audio Track"), |ui| {
                        if ui
                            .selectable_label(app.current_aid == "no", app.tr("None"))
                            .clicked()
                        {
                            let _ = app.mpv.set_property("aid", "no");
                            ui.close();
                        }
                        for track in &app.audio_tracks {
                            let track_id_str = track.id.to_string();
                            let label = format_track_label(
                                track.id,
                                track.lang.as_deref(),
                                track.title.as_deref(),
                            );

                            if ui
                                .selectable_label(app.current_aid == track_id_str, label)
                                .clicked()
                            {
                                let _ = app.mpv.set_property("aid", track_id_str);
                                ui.close();
                            }
                        }
                    });

                    ui.separator();

                    if ui.button(app.tr("Audio Settings...")).clicked() {
                        app.show_audio_settings = true;
                        ui.close();
                    }
                });

                // Subtitles menu
                ui.menu_button(app.tr("Subtitles"), |ui| {
                    ui.menu_button(app.tr("Subtitle Track"), |ui| {
                        if ui
                            .selectable_label(app.current_sid == "no", app.tr("None"))
                            .clicked()
                        {
                            let _ = app.mpv.set_property("sid", "no");
                            ui.close();
                        }
                        for track in &app.sub_tracks {
                            let track_id_str = track.id.to_string();
                            let label = format_track_label(
                                track.id,
                                track.lang.as_deref(),
                                track.title.as_deref(),
                            );

                            if ui
                                .selectable_label(app.current_sid == track_id_str, label)
                                .clicked()
                            {
                                let _ = app.mpv.set_property("sid", track_id_str);
                                ui.close();
                            }
                        }
                    });

                    ui.separator();

                    let mut vis = app.sub_visibility;
                    if ui.checkbox(&mut vis, app.tr("Enable Subtitles")).changed() {
                        app.sub_visibility = vis;
                        let _ = app.mpv.set_property("sub-visibility", vis);
                    }

                    ui.separator();

                    if ui.button(app.tr("Subtitle Settings...")).clicked() {
                        app.show_sub_settings = true;
                        ui.close();
                    }
                });

                // Workspace switcher
                ui.menu_button(app.tr("Workspace"), |ui| {
                    if ui
                        .selectable_label(app.show_four_d_editor, app.tr("NLE Layout (Docked)"))
                        .clicked()
                    {
                        app.show_four_d_editor = true;
                        ui.close();
                    }
                    if ui
                        .selectable_label(!app.show_four_d_editor, app.tr("Simple Player"))
                        .clicked()
                    {
                        app.show_four_d_editor = false;
                        ui.close();
                    }
                });

                // Add right-aligned E-STOP and Serial controls
                ui.menu_button(app.tr("Help"), |ui| {
                    ui.menu_button(app.tr("Language"), |ui| {
                        for (preference, label) in [
                            (
                                crate::config::AppLanguage::System,
                                app.tr("System language"),
                            ),
                            (crate::config::AppLanguage::English, app.tr("English")),
                            (crate::config::AppLanguage::Persian, app.tr("Persian")),
                        ] {
                            if ui
                                .selectable_label(app.language_preference == preference, label)
                                .clicked()
                            {
                                    app.set_language(ui.ctx(), preference);
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button(app.tr("Direction"), |ui| {
                        for (preference, label) in [
                            (crate::config::AppDirection::Auto, app.tr("Automatic")),
                            (crate::config::AppDirection::Ltr, app.tr("Left to right")),
                            (crate::config::AppDirection::Rtl, app.tr("Right to left")),
                        ] {
                            if ui
                                .selectable_label(app.direction_preference == preference, label)
                                .clicked()
                            {
                                app.set_direction(preference);
                                ui.close();
                            }
                        }
                    });
                    ui.separator();
                    if ui
                        .button(format!("{} {}", crate::ui::icons::KEYBOARD, app.tr("Keyboard Shortcuts...")))
                        .clicked()
                    {
                        ui.close();
                        app.show_shortcuts_dialog = true;
                    }
                    if ui
                        .button(format!("{} {} {}", crate::ui::icons::INFO, app.tr("About"), app.app_name))
                        .clicked()
                    {
                        ui.close();
                        app.show_about_dialog = true;
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);

                    // 1. E-STOP Kill Switch Button
                    let board_ready = app
                        .advertised_hardware()
                        .is_some_and(|capabilities| capabilities.board_connected);
                    let btn = ui
                        .add_enabled_ui(board_ready, |ui| {
                            estop_button(ui, app.estop_active, language)
                        })
                        .inner
                        .on_disabled_hover_text(app.tr("E-STOP is available when a live board is connected"));

                    if btn.clicked() {
                        app.estop_active = !app.estop_active;
                        app.engine_handle
                            .estop_active
                            .store(app.estop_active, std::sync::atomic::Ordering::Relaxed);

                        if app.estop_active {
                            // Pause video playback immediately
                            app.pause();
                        }
                    }

                    ui.separator();

                    // 2. Coordinator/direct-diagnostic connection toggle & dropdown
                    let connection_requested = app
                        .engine_handle
                        .connection_requested
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let conn_text = if app.is_connected {
                        app.tr("Disconnect")
                    } else if connection_requested {
                        app.tr("Connecting…")
                    } else {
                        app.tr("Connect")
                    };
                    let connection_icon = if app.is_connected {
                        crate::ui::icons::X
                    } else {
                        crate::ui::icons::PLUG
                    };
                    let conn_btn = ui.add(
                        egui::Button::new(format!("{connection_icon} {conn_text}"))
                            .selected(app.is_connected),
                    );
                    if conn_btn.clicked() {
                        let should_connect = !(app.is_connected || connection_requested);
                        if should_connect {
                            let endpoint = app.serial_port.trim().to_owned();
                            if endpoint.is_empty() {
                                app.connection_notice = Some(
                                    app.tr("Enter a hardware endpoint before connecting."),
                                );
                            } else {
                                app.serial_port = endpoint.clone();
                                if let Ok(mut selected) = app.engine_handle.serial_port.lock() {
                                    *selected = endpoint;
                                }
                                app.connection_notice = None;
                                app.engine_handle.connection_requested.store(
                                    true,
                                    std::sync::atomic::Ordering::Relaxed,
                                );
                                app.save_config();
                            }
                        } else {
                            app.engine_handle.connection_requested.store(
                                false,
                                std::sync::atomic::Ordering::Relaxed,
                            );
                        }
                    }

                    // PCController owns the board during normal operation. Keep discovered
                    // devices convenient, but never make discovery the only way to choose a
                    // coordinator or direct diagnostic path: remote hosts and stable OS device
                    // paths are valid even when they are not visible to the local enumerator.
                    let mut endpoint_changed = false;
                    ui.add_enabled_ui(!app.is_connected && !connection_requested, |ui| {
                        ui.allocate_ui(egui::vec2(188.0, 20.0), |ui| {
                            egui::ComboBox::from_id_salt("hardware_endpoint_select")
                                .selected_text(&app.serial_port)
                                .width(240.0)
                                .height(240.0)
                                .show_ui(ui, |ui| {
                                    for endpoint in crate::four_d::controller::available_endpoints()
                                    {
                                        endpoint_changed |= ui
                                            .selectable_value(
                                                &mut app.serial_port,
                                                endpoint.clone(),
                                                endpoint,
                                            )
                                            .changed();
                                    }

                                    ui.separator();
                                    ui.label(app.tr("Custom endpoint or hardware path"));
                                    let endpoint_hint = app.tr(
                                        "pccontroller://host:port, tcp://host:port, or direct:<device>",
                                    );
                                    endpoint_changed |= ui
                                        .add_sized(
                                            [240.0, 22.0],
                                            egui::TextEdit::singleline(&mut app.serial_port)
                                                .hint_text(endpoint_hint),
                                        )
                                        .on_hover_text(app.tr(
                                            "Enter a PCController endpoint or an OS hardware path.",
                                        ))
                                        .changed();
                                });
                        });
                    });
                    if endpoint_changed {
                        app.save_config();
                    }

                    // Connection visual indicator dot
                    let dot_color = if app.is_connected {
                        egui::Color32::from_rgb(46, 204, 113) // Green
                    } else if connection_requested {
                        egui::Color32::from_rgb(241, 196, 15) // Amber
                    } else {
                        egui::Color32::from_rgb(231, 76, 60) // Red
                    };

                    let (dot_rect, _) =
                        ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter()
                        .circle_filled(dot_rect.center(), 4.0, dot_color);

                    let status_lbl = if app.is_connected {
                        if crate::four_d::controller::is_controller_endpoint(&app.serial_port) {
                            app.tr("PCController coordinator connected").to_string()
                        } else {
                            format!("{} direct diagnostic connection", app.serial_port)
                        }
                    } else if connection_requested {
                        format!("Connecting to {}…", app.serial_port)
                    } else {
                        app.tr("Hardware Disconnected").to_string()
                    };
                    ui.label(egui::RichText::new(status_lbl).size(10.0).weak());
                });
            });
        });
    });
}

pub fn format_track_label(id: i64, lang: Option<&str>, title: Option<&str>) -> String {
    let parts: Vec<&str> = vec![lang.unwrap_or(""), title.unwrap_or("")]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        format!("Track {}", id)
    } else {
        format!("Track {} ({})", id, parts.join(" - "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_track_label_variations() {
        assert_eq!(format_track_label(1, None, None), "Track 1");
        assert_eq!(format_track_label(2, Some("eng"), None), "Track 2 (eng)");
        assert_eq!(
            format_track_label(3, None, Some("Commentary")),
            "Track 3 (Commentary)"
        );
        assert_eq!(
            format_track_label(4, Some("eng"), Some("Director's Cut")),
            "Track 4 (eng - Director's Cut)"
        );
        assert_eq!(format_track_label(5, Some(""), Some("")), "Track 5");
    }
}
