use crate::app::PealayerApp;
use eframe::egui;

pub fn draw_settings_dialog(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_audio_settings {
        return;
    }

    let mut open = app.show_audio_settings;

    egui::Window::new("🎵 Audio Settings")
        .open(&mut open)
        .collapsible(true)
        .resizable(true)
        .default_size([420.0, 320.0])
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ui.ctx(), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);

            // Track Selection
            ui.horizontal(|ui| {
                ui.label("Track:");
                let current_label = if app.current_aid == "no" {
                    "None".to_string()
                } else {
                    let mut label = format!("Track {}", app.current_aid);
                    for t in &app.audio_tracks {
                        if t.id.to_string() == app.current_aid {
                            let parts: Vec<&str> = vec![
                                t.lang.as_deref().unwrap_or(""),
                                t.title.as_deref().unwrap_or(""),
                            ]
                            .into_iter()
                            .filter(|s| !s.is_empty())
                            .collect();
                            if !parts.is_empty() {
                                label = format!("Track {} ({})", t.id, parts.join(" - "));
                            }
                            break;
                        }
                    }
                    label
                };

                egui::ComboBox::from_id_salt("audio_track_combo")
                    .selected_text(current_label)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(&mut app.current_aid, "no".to_string(), "None")
                            .clicked()
                        {
                            let _ = app.mpv.set_property("aid", "no");
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
                                .selectable_value(&mut app.current_aid, track_id_str.clone(), label)
                                .clicked()
                            {
                                let _ = app.mpv.set_property("aid", track_id_str);
                            }
                        }
                    });
            });

            ui.separator();

            // Synchronization
            ui.label("Synchronization");
            ui.horizontal(|ui| {
                ui.label("Delay (s):");
                let mut delay = app.audio_delay;
                if ui
                    .add(
                        egui::DragValue::new(&mut delay)
                            .speed(0.1)
                            .range(-10.0..=10.0),
                    )
                    .changed()
                {
                    let _ = app.mpv.set_property("audio-delay", delay);
                }
                if ui.button("Reset").clicked() {
                    let _ = app.mpv.set_property("audio-delay", 0.0);
                }
            });

            ui.separator();

            // Load External
            if ui.button("Load External Audio...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Audio Files", &["mp3", "flac", "wav", "m4a", "aac", "ogg"])
                    .pick_file()
                {
                    if let Some(path_str) = path.to_str() {
                        let _ = app.mpv.command("audio-add", &[path_str]);
                        app.refresh_audio_tracks();
                    }
                }
            }
        });

    app.show_audio_settings = open;
}
