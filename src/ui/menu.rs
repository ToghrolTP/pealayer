use crate::app::PealayerApp;
use eframe::egui;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();

    egui::Panel::top("top_panel").show_inside(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open File...").clicked() {
                    ui.close();
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Video Files", &["mp4", "mkv", "avi", "webm", "mov", "flv"])
                        .pick_file()
                    {
                        let path_str = path.to_str().unwrap_or("");
                        if !path_str.is_empty() {
                            let _ = app.mpv.command("loadfile", &[path_str, "replace"]);
                        }
                    }
                }
                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Subtitles", |ui| {
                ui.menu_button("Subtitle Track", |ui| {
                    if ui.selectable_label(app.current_sid == "no", "None").clicked() {
                        let _ = app.mpv.set_property("sid", "no");
                        ui.close();
                    }
                    for track in &app.sub_tracks {
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
                    let _ = app.mpv.set_property("sub-visibility", vis);
                }

                ui.separator();

                if ui.button("Subtitle Settings...").clicked() {
                    app.show_sub_settings = true;
                    ui.close();
                }
            });

            ui.menu_button("Audio", |ui| {
                ui.menu_button("Audio Track", |ui| {
                    if ui.selectable_label(app.current_aid == "no", "None").clicked() {
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

                        if ui.selectable_label(app.current_aid == track_id_str, label).clicked() {
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
        });
    });
}
