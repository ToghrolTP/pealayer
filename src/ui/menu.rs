use crate::app::PealayerApp;
use eframe::egui;

fn estop_button(ui: &mut egui::Ui, active: bool) -> egui::Response {
    let (label, fill, help) = if active {
        (
            "RESET E-STOP",
            egui::Color32::from_rgb(231, 76, 60),
            "Reset the active emergency stop",
        )
    } else {
        (
            "E-STOP",
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
            let angle = std::f32::consts::FRAC_PI_8
                + index as f32 * std::f32::consts::FRAC_PI_4;
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

    egui::Panel::top("menu_bar").show_inside(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open Video File...").clicked() {
                    ui.close();
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Video Files", &["mp4", "mkv", "avi", "webm", "mov", "flv"])
                        .pick_file()
                    {
                        app.load_video_file(path);
                    }
                }

                if ui.button("Open Location / URL...").clicked() {
                    ui.close();
                    app.show_open_url_dialog = true;
                }

                ui.menu_button("Open Recent", |ui| {
                    if app.recent_media.is_empty() {
                        ui.label("No recent media");
                    } else {
                        for path in app.recent_media.clone() {
                            let file_name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown");
                            if ui.button(file_name).on_hover_text(path.display().to_string()).clicked() {
                                ui.close();
                                app.load_video_file(path);
                            }
                        }
                        ui.separator();
                        if ui.button("Clear Recent").clicked() {
                            ui.close();
                            app.clear_recent_media();
                        }
                    }
                });

                let has_video = app.current_video_path.is_some();
                if ui.add_enabled(has_video, egui::Button::new("Close Video")).clicked() {
                    ui.close();
                    app.close_video();
                }

                if ui.button("Open Timeline Project...").clicked() {
                    ui.close();
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Pealayer Timeline", &["json"])
                        .pick_file()
                    {
                        match crate::four_d::models::Timeline::load_from_file(&path) {
                            Ok(timeline) => {
                                app.timeline = timeline;
                                let compiled = crate::four_d::engine::compile_timeline(&app.timeline, &app.track_muted, &app.track_soloed);
                                let _ = app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                            }
                            Err(e) => {
                                app.show_error = Some(format!("Failed to load timeline: {}", e));
                            }
                        }
                    }
                }

                ui.separator();

                let save_enabled = app.current_video_path.is_some();
                let save_btn = egui::Button::new("Save Timeline (Sidecar)");
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

                if ui.button("Save Timeline As...").clicked() {
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
                if ui.button("⚙ Register as Default Media Player...").clicked() {
                    ui.close();
                    match crate::platform::association::register_as_default_player() {
                        Ok(msg) => app.set_osd(msg),
                        Err(err) => app.show_error = Some(err),
                    }
                }

                ui.separator();
                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Edit", |ui| {
                let mut undo_btn = egui::Button::new("Undo");
                undo_btn = undo_btn.shortcut_text("Ctrl+Z");
                if ui.add_enabled(false, undo_btn).clicked() {
                    ui.close();
                }

                let mut redo_btn = egui::Button::new("Redo");
                redo_btn = redo_btn.shortcut_text("Ctrl+Y");
                if ui.add_enabled(false, redo_btn).clicked() {
                    ui.close();
                }
            });

            ui.menu_button("Audio", |ui| {
                ui.menu_button("Audio Track", |ui| {
                    if ui
                        .selectable_label(app.current_aid == "no", "None")
                        .clicked()
                    {
                        let _ = app.mpv.set_property("aid", "no");
                        ui.close();
                    }
                    for track in &app.audio_tracks {
                        let track_id_str = track.id.to_string();
                        let label = format_track_label(track.id, track.lang.as_deref(), track.title.as_deref());

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

                if ui.button("Audio Settings...").clicked() {
                    app.show_audio_settings = true;
                    ui.close();
                }
            });

            // Subtitles menu
            ui.menu_button("Subtitles", |ui| {
                ui.menu_button("Subtitle Track", |ui| {
                    if ui.selectable_label(app.current_sid == "no", "None").clicked() {
                        let _ = app.mpv.set_property("sid", "no");
                        ui.close();
                    }
                    for track in &app.sub_tracks {
                        let track_id_str = track.id.to_string();
                        let label = format_track_label(track.id, track.lang.as_deref(), track.title.as_deref());

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
                if ui.checkbox(&mut vis, "Enable Subtitles").changed() {
                    app.sub_visibility = vis;
                    let _ = app.mpv.set_property("sub-visibility", vis);
                }

                ui.separator();

                if ui.button("Subtitle Settings...").clicked() {
                    app.show_sub_settings = true;
                    ui.close();
                }
            });

            // Workspace switcher
            ui.menu_button("Workspace", |ui| {
                if ui
                    .selectable_label(app.show_four_d_editor, "NLE Layout (Docked)")
                    .clicked()
                {
                    app.show_four_d_editor = true;
                    ui.close();
                }
                if ui
                    .selectable_label(!app.show_four_d_editor, "Simple Player")
                    .clicked()
                {
                    app.show_four_d_editor = false;
                    ui.close();
                }
            });
            
            // Add right-aligned E-STOP and Serial controls
            ui.menu_button("Help", |ui| {
                if ui.button("⌨ Keyboard Shortcuts...").clicked() {
                    ui.close();
                    app.show_shortcuts_dialog = true;
                }
                if ui.button("ℹ About Pealayer...").clicked() {
                    ui.close();
                    app.show_about_dialog = true;
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                
                // 1. E-STOP Kill Switch Button
                let btn = estop_button(ui, app.estop_active);
                
                if btn.clicked() {
                    app.estop_active = !app.estop_active;
                    app.engine_handle.estop_active.store(app.estop_active, std::sync::atomic::Ordering::Relaxed);
                    
                    if app.estop_active {
                        // Pause video playback immediately
                        let _ = app.mpv.set_property("pause", true);
                    }
                }
                
                ui.separator();
                
                // 2. Coordinator/direct-diagnostic connection toggle & dropdown
                let connection_requested = app
                    .engine_handle
                    .connection_requested
                    .load(std::sync::atomic::Ordering::Relaxed);
                let conn_text = if app.is_connected {
                    "Disconnect"
                } else if connection_requested {
                    "Connecting…"
                } else {
                    "Connect"
                };
                let conn_btn = ui.add_enabled(
                    !connection_requested || app.is_connected,
                    egui::Button::new(conn_text).selected(app.is_connected),
                );
                if conn_btn.clicked() {
                    {
                        let mut port_guard = app.engine_handle.serial_port.lock().unwrap();
                        *port_guard = app.serial_port.clone();
                    }
                    app.engine_handle
                        .connection_requested
                        .store(!app.is_connected, std::sync::atomic::Ordering::Relaxed);
                }
                
                // PCController owns the board during normal operation. Direct serial is
                // deliberately labelled and additionally guarded by the engine.
                ui.add_enabled_ui(!app.is_connected && !connection_requested, |ui| {
                    ui.allocate_ui(egui::vec2(210.0, 20.0), |ui| {
                        egui::ComboBox::from_id_salt("serial_port_select")
                            .selected_text(&app.serial_port)
                            .show_ui(ui, |ui| {
                                for endpoint in crate::four_d::controller::available_endpoints() {
                                    ui.selectable_value(
                                        &mut app.serial_port,
                                        endpoint.clone(),
                                        endpoint,
                                    );
                                }
                            });
                    });
                });
                
                // Connection visual indicator dot
                let dot_color = if app.is_connected {
                    egui::Color32::from_rgb(46, 204, 113) // Green
                } else if connection_requested {
                    egui::Color32::from_rgb(241, 196, 15) // Amber
                } else {
                    egui::Color32::from_rgb(231, 76, 60) // Red
                };
                
                let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter().circle_filled(dot_rect.center(), 4.0, dot_color);
                
                let status_lbl = if app.is_connected {
                    if crate::four_d::controller::is_controller_endpoint(&app.serial_port) {
                        "PCController coordinator connected".to_string()
                    } else {
                        format!("{} direct diagnostic connection", app.serial_port)
                    }
                } else if connection_requested {
                    format!("Connecting to {}…", app.serial_port)
                } else {
                    "Hardware Disconnected".to_string()
                };
                ui.label(egui::RichText::new(status_lbl).size(10.0).weak());
            });
        });
    });
}

pub fn format_track_label(id: i64, lang: Option<&str>, title: Option<&str>) -> String {
    let parts: Vec<&str> = vec![
        lang.unwrap_or(""),
        title.unwrap_or(""),
    ]
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
        assert_eq!(format_track_label(3, None, Some("Commentary")), "Track 3 (Commentary)");
        assert_eq!(format_track_label(4, Some("eng"), Some("Director's Cut")), "Track 4 (eng - Director's Cut)");
        assert_eq!(format_track_label(5, Some(""), Some("")), "Track 5");
    }
}
