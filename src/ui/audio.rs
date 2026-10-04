use crate::app::PealayerApp;
use crate::ui::dialog;
use eframe::egui;

const AUDIO_DIALOG_WIDTH: f32 = 420.0;
const AUDIO_DIALOG_DEFAULT_HEIGHT: f32 = 300.0;
const AUDIO_DIALOG_MIN_HEIGHT: f32 = 220.0;
const AUDIO_DIALOG_MAX_HEIGHT: f32 = 430.0;
const AUDIO_DIALOG_FOOTER_RESERVE: f32 = 38.0;

pub fn draw_settings_dialog(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_audio_settings {
        return;
    }

    let mut open = app.show_audio_settings;
    let bounds = ui.ctx().content_rect().shrink(20.0);
    let max_size = egui::vec2(
        bounds.width().min(460.0),
        bounds.height().min(AUDIO_DIALOG_MAX_HEIGHT),
    );
    let default_size = egui::vec2(
        max_size.x.min(AUDIO_DIALOG_WIDTH),
        max_size.y.min(AUDIO_DIALOG_DEFAULT_HEIGHT),
    );
    let default_rect = crate::ui::dialog::centered_default_rect(bounds, default_size);
    let mut close_requested = crate::ui::dialog::escape_pressed(ui.ctx());

    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::MUSIC_NOTE,
        app.tr("Audio Settings")
    ))
    // The v3 id intentionally discards remembered geometry from the older,
    // over-tall dialog so existing installations receive the compact default.
    .id(egui::Id::new("audio_settings_dialog_content_sized_v3"))
    .open(&mut open)
    .collapsible(false)
    .resizable(true)
    .default_rect(default_rect)
    .min_size([
        340.0_f32.min(max_size.x),
        AUDIO_DIALOG_MIN_HEIGHT.min(max_size.y),
    ])
    .max_size(max_size)
    .constrain_to(bounds)
    .movable(true)
    .show(ui.ctx(), |ui| {
        let body_max_height = (ui.available_height() - AUDIO_DIALOG_FOOTER_RESERVE).max(96.0);
        dialog::fit_scroll(ui, "audio_settings_body_v3", Some(body_max_height), |ui| {
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

                let none_label = app.tr("No audio track");
                let tracks = app.audio_tracks.clone();
                egui::ComboBox::from_id_salt("audio_track_combo")
                    .selected_text(current_label)
                    .width((ui.available_width() - 4.0).clamp(140.0, 240.0))
                    .height(200.0)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(app.current_aid == "no", none_label)
                            .clicked()
                        {
                            app.disable_media_track(crate::app::MediaTrackType::Audio);
                        }
                        for track in tracks {
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
                                app.select_media_track(crate::app::MediaTrackKey {
                                    kind: crate::app::MediaTrackType::Audio,
                                    id: track.id,
                                });
                            }
                        }
                    });
            });
            ui.add_space(8.0);

            crate::ui::dialog::section(
                ui,
                crate::ui::icons::CLOCK_COUNTER_CLOCKWISE,
                &app.tr("Synchronization"),
                |ui| {
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
                        if ui
                            .button(format!(
                                "{} {}",
                                crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                                app.tr("Reset")
                            ))
                            .clicked()
                        {
                            app.audio_delay = 0.0;
                            let _ = app.mpv.set_property("audio-delay", 0.0);
                        }
                    });
                },
            );
            ui.add_space(8.0);

            // Load External
            if ui
                .button(format!(
                    "{} {}",
                    crate::ui::icons::FOLDER_OPEN,
                    app.tr("Load External Audio...")
                ))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Audio Files", &["mp3", "flac", "wav", "m4a", "aac", "ogg"])
                    .pick_file()
                {
                    if let Some(path_str) = path.to_str() {
                        let _ = app.mpv.command("audio-add", &[path_str]);
                        app.refresh_media_tracks();
                    }
                }
            }
        });
        ui.separator();
        crate::ui::dialog::action_row(ui, app.rtl, |ui| {
            if crate::ui::dialog::action_button(ui, crate::ui::icons::X, &app.tr("Close"))
                .on_hover_text("Esc")
                .clicked()
            {
                close_requested = true;
            }
        });
    });

    app.show_audio_settings = open && !close_requested;
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

    #[test]
    fn audio_dialog_defaults_are_compact_and_resizable() {
        assert!(AUDIO_DIALOG_MIN_HEIGHT < AUDIO_DIALOG_DEFAULT_HEIGHT);
        assert!(AUDIO_DIALOG_DEFAULT_HEIGHT < AUDIO_DIALOG_MAX_HEIGHT);
        assert_eq!(AUDIO_DIALOG_DEFAULT_HEIGHT, 300.0);
    }
}
