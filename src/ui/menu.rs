use crate::app::PealayerApp;
use eframe::egui;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();

    egui::Panel::top("menu_bar").show_inside(ui, |ui| {
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
        });
    });
}

