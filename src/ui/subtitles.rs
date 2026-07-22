use crate::app::PealayerApp;
use eframe::egui;

pub fn draw_settings_dialog(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_sub_settings {
        return;
    }

    let mut open = app.show_sub_settings;

    egui::Window::new("💬 Subtitle Settings")
        .open(&mut open)
        .collapsible(true)
        .resizable(true)
        .default_size([460.0, 360.0])
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ui.ctx(), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);

            // Visibility
            let mut vis = app.sub_visibility;
            if ui.checkbox(&mut vis, "Enable Subtitles").changed() {
                let _ = app.mpv.set_property("sub-visibility", vis);
            }

            ui.separator();

            // Track Selection
            ui.horizontal(|ui| {
                ui.label("Track:");
                let current_label = if app.current_sid == "no" {
                    "None".to_string()
                } else {
                    let mut label = format!("Track {}", app.current_sid);
                    for t in &app.sub_tracks {
                        if t.id.to_string() == app.current_sid {
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

                egui::ComboBox::from_id_salt("sub_track_combo")
                    .selected_text(current_label)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(&mut app.current_sid, "no".to_string(), "None")
                            .clicked()
                        {
                            let _ = app.mpv.set_property("sid", "no");
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
                                .selectable_value(&mut app.current_sid, track_id_str.clone(), label)
                                .clicked()
                            {
                                let _ = app.mpv.set_property("sid", track_id_str);
                            }
                        }
                    });
            });

            ui.separator();

            // Appearance
            ui.label("Appearance");
            ui.horizontal(|ui| {
                ui.label("Font Size:");
                let mut font_size = app.sub_font_size;
                if ui
                    .add(egui::Slider::new(&mut font_size, 10.0..=100.0))
                    .changed()
                {
                    let _ = app.mpv.set_property("sub-font-size", font_size);
                }
            });

            ui.separator();

            // Synchronization
            ui.label("Synchronization");
            ui.horizontal(|ui| {
                ui.label("Delay (s):");
                let mut delay = app.sub_delay;
                if ui
                    .add(
                        egui::DragValue::new(&mut delay)
                            .speed(0.1)
                            .range(-10.0..=10.0),
                    )
                    .changed()
                {
                    let _ = app.mpv.set_property("sub-delay", delay);
                }
                if ui.button("Reset").clicked() {
                    let _ = app.mpv.set_property("sub-delay", 0.0);
                }
            });

            ui.separator();

            // Load External
            if ui.button("Load External Subtitle...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Subtitles", &["srt", "vtt", "ass", "ssa"])
                    .pick_file()
                {
                    if let Some(path_str) = path.to_str() {
                        let _ = app.mpv.command("sub-add", &[path_str]);
                        // It takes a moment for the track to be added and selected.
                        // Ideally we observe track-list changes, but we can also just
                        // refresh manually or rely on the user to see the new track.
                        // Let's manually refresh after a slight delay or just call it directly.
                        app.refresh_sub_tracks();
                    }
                }
            }
        });

    app.show_sub_settings = open;
}
