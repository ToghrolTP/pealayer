use crate::app::PealayerApp;
use eframe::egui;

pub fn draw_settings_dialog(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_audio_settings {
        return;
    }

    let mut open = app.show_audio_settings;
    let bounds = ui.ctx().content_rect().shrink(20.0);
    let max_size = egui::vec2(bounds.width().min(460.0), bounds.height().min(430.0));
    let default_size = egui::vec2(max_size.x.min(420.0), max_size.y.min(380.0));

    egui::Window::new(format!("{} {}", crate::ui::icons::MUSIC_NOTE, app.tr("Audio Settings")))
        .id(egui::Id::new("audio_settings_dialog_bounded_v2"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size(default_size)
        .min_size([340.0_f32.min(max_size.x), 260.0_f32.min(max_size.y)])
        .max_size(max_size)
        .constrain_to(bounds)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ui.ctx(), |ui| {
          crate::ui::dialog::scroll_column(ui, "audio_settings_body_v2", None, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);

            crate::ui::dialog::section(ui, crate::ui::icons::MUSIC_NOTE, &app.tr("Track"), |ui| {
                let current_label = if app.current_aid == "no" {
                    app.tr("None").to_string()
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

                let none_label = app.tr("None");
                egui::ComboBox::from_id_salt("audio_track_combo")
                    .selected_text(current_label)
                    .width((ui.available_width() - 4.0).clamp(140.0, 240.0))
                    .height(200.0)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(&mut app.current_aid, "no".to_string(), none_label)
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
            ui.add_space(8.0);

            crate::ui::dialog::section(ui, crate::ui::icons::CLOCK_COUNTER_CLOCKWISE, &app.tr("Synchronization"), |ui| {
              crate::ui::dialog::compact_row(ui, app.rtl, |ui| {
                ui.label(app.tr("Delay (s):"));
                let mut delay = app.audio_delay;
                if ui
                    .add(
                        egui::DragValue::new(&mut delay)
                            .speed(0.1)
                            .range(MIN_AUDIO_DELAY..=MAX_AUDIO_DELAY),
                    )
                    .changed()
                {
                    app.audio_delay = delay;
                    let _ = app.mpv.set_property("audio-delay", delay);
                }
                if ui.button(format!("{} {}", crate::ui::icons::ARROW_COUNTER_CLOCKWISE, app.tr("Reset"))).clicked() {
                    app.audio_delay = 0.0;
                    let _ = app.mpv.set_property("audio-delay", 0.0);
                }
              });
            });
            ui.add_space(8.0);

            // Load External
            if ui.button(format!("{} {}", crate::ui::icons::FOLDER_OPEN, app.tr("Load External Audio..."))).clicked() {
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
        });

    app.show_audio_settings = open;
}

pub const MIN_AUDIO_DELAY: f64 = -600.0;
pub const MAX_AUDIO_DELAY: f64 = 600.0;

pub fn clamp_audio_delay(delay: f64) -> f64 {
    delay.clamp(MIN_AUDIO_DELAY, MAX_AUDIO_DELAY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_delay_range_clamping() {
        assert_eq!(clamp_audio_delay(0.0), 0.0);
        assert_eq!(clamp_audio_delay(-800.0), -600.0);
        assert_eq!(clamp_audio_delay(950.0), 600.0);
        assert_eq!(clamp_audio_delay(-12.4), -12.4);
    }
}
