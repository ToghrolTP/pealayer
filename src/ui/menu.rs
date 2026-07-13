use crate::app::PealayerApp;
use eframe::egui;

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
                        let path_str = path.to_str().unwrap_or("");
                        if !path_str.is_empty() {
                            let _ = app.mpv.command("loadfile", &[path_str, "replace"]);
                            app.current_video_path = Some(path.clone());
                            
                            // Auto-load matching sidecar timeline
                            let mut sidecar = path.clone();
                            sidecar.set_extension("4d.json");
                            if !sidecar.exists() {
                                sidecar.set_extension("json");
                            }
                            if sidecar.exists() {
                                if let Ok(timeline) = crate::four_d::models::Timeline::load_from_file(&sidecar) {
                                    app.timeline = timeline;
                                    let compiled = crate::four_d::engine::compile_timeline(&app.timeline, &app.track_muted, &app.track_soloed);
                                    let _ = app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                }
                            }
                        }
                    }
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
                        let parts: Vec<&str> = vec![
                            track.lang.as_deref().unwrap_or(""),
                            track.title.as_deref().unwrap_or(""),
                        ]
                        .into_iter()
                        .filter(|s| !s.is_empty())
                        .collect();

                        let label = if parts.is_empty() {
                            format!("Track {}", track.id)
                        } else {
                            format!("Track {} ({})", track.id, parts.join(" - "))
                        };

                        if ui
                            .selectable_label(app.current_aid == track_id_str, label)
                            .clicked()
                        {
                            let _ = app.mpv.set_property("aid", track_id_str);
                            ui.close();
                        }
                    }
                });
            });

            // Subtitles menu
            ui.menu_button("Subtitles", |ui| {
                if ui.checkbox(&mut app.sub_visibility, "Show Subtitles").changed() {
                    let _ = app.mpv.set_property("sub-visibility", app.sub_visibility);
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
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                
                // 1. E-STOP Kill Switch Button
                let estop_text = if app.estop_active {
                    "🔴 RESET E-STOP"
                } else {
                    "🛑 E-STOP"
                };
                
                let estop_color = if app.estop_active {
                    egui::Color32::from_rgb(231, 76, 60) // Bright red
                } else {
                    egui::Color32::from_rgb(192, 57, 43) // Dark red
                };
                
                let btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new(estop_text)
                            .color(egui::Color32::WHITE)
                            .strong()
                            .size(11.0)
                    )
                    .fill(estop_color)
                );
                
                if btn.clicked() {
                    app.estop_active = !app.estop_active;
                    app.engine_handle.estop_active.store(app.estop_active, std::sync::atomic::Ordering::Relaxed);
                    
                    if app.estop_active {
                        // Pause video playback immediately
                        let _ = app.mpv.set_property("pause", true);
                    }
                }
                
                ui.separator();
                
                // 2. Serial connection toggle & dropdown
                let conn_text = if app.is_connected { "Disconnect" } else { "Connect" };
                let conn_btn = ui.selectable_label(app.is_connected, conn_text);
                if conn_btn.clicked() {
                    app.is_connected = !app.is_connected;
                    app.engine_handle.is_connected.store(app.is_connected, std::sync::atomic::Ordering::Relaxed);
                    
                    {
                        let mut port_guard = app.engine_handle.serial_port.lock().unwrap();
                        *port_guard = app.serial_port.clone();
                    }
                }
                
                // Serial Port Dropdown
                ui.allocate_ui(egui::vec2(100.0, 20.0), |ui| {
                    egui::ComboBox::from_id_salt("serial_port_select")
                        .selected_text(&app.serial_port)
                        .show_ui(ui, |ui| {
                            let ports = ["COM1", "COM2", "COM3", "COM4", "/dev/ttyUSB0", "/dev/ttyUSB1", "/dev/ttyACM0"];
                            for p in ports {
                                let res = ui.selectable_value(&mut app.serial_port, p.to_string(), p);
                                if res.changed() && app.is_connected {
                                    let mut port_guard = app.engine_handle.serial_port.lock().unwrap();
                                    *port_guard = app.serial_port.clone();
                                }
                            }
                        });
                });
                
                // Connection visual indicator dot
                let dot_color = if app.is_connected {
                    egui::Color32::from_rgb(46, 204, 113) // Green
                } else {
                    egui::Color32::from_rgb(231, 76, 60) // Red
                };
                
                let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter().circle_filled(dot_rect.center(), 4.0, dot_color);
                
                let status_lbl = if app.is_connected {
                    format!("{} Connected", app.serial_port)
                } else {
                    "Hardware Disconnected".to_string()
                };
                ui.label(egui::RichText::new(status_lbl).size(10.0).weak());
            });
        });
    });
}

