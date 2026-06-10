use crate::app::PealayerApp;
use eframe::egui;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();

    egui::Panel::top("top_panel").show_inside(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open File…").clicked() {
                    ui.close();
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(
                            "Video Files",
                            &["mp4", "mkv", "avi", "webm", "mov", "flv", "wmv", "m4v", "ts", "m2ts"],
                        )
                        .pick_file()
                    {
                        let path_str = path.to_str().unwrap_or("");
                        if !path_str.is_empty() {
                            let _ = app.mpv.command("loadfile", &[path_str, "replace"]);
                        }
                    }
                }
                if ui.button("Open URL…").clicked() {
                    app.show_url_dialog = true;
                    ui.close();
                }
                if app.has_video {
                    if ui.button("Close Video").clicked() {
                        let _ = app.mpv.command("stop", &[]);
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
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
            });
        });
    });
}

/// Modal dialog that lets the user type a URL and load it in mpv.
pub fn draw_url_dialog(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_url_dialog {
        return;
    }

    let mut open = app.show_url_dialog;
    let mut load = false;
    let mut cancel = false;

    egui::Window::new("Open URL")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ui.ctx(), |ui| {
            ui.label("Enter a URL to open:");
            let response = ui.add(
                egui::TextEdit::singleline(&mut app.url_input)
                    .desired_width(360.0)
                    .hint_text("https://…"),
            );
            // Allow pressing Enter to submit.
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                load = true;
            }
            ui.horizontal(|ui| {
                if ui.button("Load").clicked() {
                    load = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });

    if load {
        let url = app.url_input.trim().to_owned();
        if !url.is_empty() {
            let _ = app.mpv.command("loadfile", &[&url, "replace"]);
            app.url_input.clear();
            open = false;
        }
    }
    if cancel {
        open = false;
    }

    app.show_url_dialog = open;
}
