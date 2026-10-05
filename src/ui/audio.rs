use crate::app::{MediaTrackKey, MediaTrackType, PealayerApp};
use crate::ui::{dialog, icons};
use eframe::egui;

const AUDIO_DIALOG_WIDTH: f32 = 520.0;
const AUDIO_DIALOG_DEFAULT_HEIGHT: f32 = 500.0;
const AUDIO_DIALOG_MIN_HEIGHT: f32 = 330.0;
const AUDIO_DIALOG_MAX_HEIGHT: f32 = 620.0;
const AUDIO_DIALOG_FOOTER_RESERVE: f32 = 42.0;
const AUDIO_TRACK_POPUP_HEIGHT: f32 = 220.0;

pub const MIN_AUDIO_DELAY: f64 = -600.0;
pub const MAX_AUDIO_DELAY: f64 = 600.0;

pub fn draw_settings_dialog(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_audio_settings {
        return;
    }

    let mut open = app.show_audio_settings;
    let geometry = dialog::bounded_geometry(
        ui.ctx().content_rect(),
        20.0,
        egui::vec2(AUDIO_DIALOG_WIDTH, AUDIO_DIALOG_DEFAULT_HEIGHT),
        egui::vec2(390.0, AUDIO_DIALOG_MIN_HEIGHT),
        egui::vec2(600.0, AUDIO_DIALOG_MAX_HEIGHT),
    );
    let mut close_requested = dialog::escape_pressed(ui.ctx());

    egui::Window::new(format!(
        "{} {}",
        icons::MUSIC_NOTE,
        app.tr("Audio Settings")
    ))
    .id(egui::Id::new("audio_settings_dialog_professional_v4"))
    .open(&mut open)
    .collapsible(false)
    .resizable(true)
    .default_rect(geometry.default_rect)
    .min_size(geometry.min_size)
    .max_size(geometry.max_size)
    .constrain_to(geometry.bounds)
    .movable(true)
    .frame(dialog::opaque_window_frame_from_context(ui.ctx()))
    .show(ui.ctx(), |ui| {
        let body_height = (ui.available_height() - AUDIO_DIALOG_FOOTER_RESERVE).max(120.0);
        dialog::scroll_column(ui, "audio_settings_body_v4", Some(body_height), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

            draw_audio_track(app, ui);
            ui.add_space(5.0);
            draw_audio_output(app, ui);
            ui.add_space(5.0);
            draw_audio_timing(app, ui);
            ui.add_space(5.0);
            draw_external_audio(app, ui);
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);
        dialog::action_row(ui, app.rtl, |ui| {
            if dialog::action_button(ui, icons::X, &app.tr("Close"))
                .on_hover_text("Esc")
                .clicked()
            {
                close_requested = true;
            }
        });
    });

    app.show_audio_settings = open && !close_requested;
}

fn draw_audio_track(app: &mut PealayerApp, ui: &mut egui::Ui) {
    dialog::section(ui, icons::LIST_CHECKS, &app.tr("Audio track"), |ui| {
        let current_label = audio_track_label(app, &app.current_aid);
        let tracks = app.audio_tracks.clone();
        dialog::setting_row(
            ui,
            icons::MUSIC_NOTE,
            &app.tr("Active track"),
            Some(&app.tr("Choose an embedded or externally loaded audio stream")),
            |ui| {
                egui::ComboBox::from_id_salt("audio_track_combo_v4")
                    .selected_text(current_label)
                    .width(ui.available_width().clamp(150.0, 275.0))
                    .height(AUDIO_TRACK_POPUP_HEIGHT)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(app.current_aid == "no", app.tr("None"))
                            .clicked()
                        {
                            app.disable_media_track(MediaTrackType::Audio);
                        }
                        for track in tracks {
                            let id = track.id.to_string();
                            let label = audio_track_label(app, &id);
                            if ui.selectable_label(app.current_aid == id, label).clicked() {
                                app.select_media_track(MediaTrackKey {
                                    kind: MediaTrackType::Audio,
                                    id: track.id,
                                });
                            }
                        }
                    });
            },
        );
    });
}

fn draw_audio_output(app: &mut PealayerApp, ui: &mut egui::Ui) {
    dialog::section(ui, icons::SPEAKER_HIGH, &app.tr("Output"), |ui| {
        dialog::setting_row(
            ui,
            if app.is_muted {
                icons::SPEAKER_SLASH
            } else {
                icons::SPEAKER_HIGH
            },
            &app.tr("Sound"),
            Some(&app.tr("Temporarily silence playback without changing volume")),
            |ui| {
                let mut audible = !app.is_muted;
                if ui.toggle_value(&mut audible, app.tr("Enabled")).changed() {
                    app.set_audio_muted(!audible);
                }
            },
        );
        ui.separator();

        dialog::setting_row(
            ui,
            icons::SLIDERS_HORIZONTAL,
            &app.tr("Volume"),
            Some(&app.tr("Adjust playback volume from 0% to 130%")),
            |ui| {
                let mut volume = app.volume;
                let response = ui.add_sized(
                    [190.0, 24.0],
                    egui::Slider::new(&mut volume, 0.0..=130.0).suffix("%"),
                );
                if response.changed() {
                    app.volume = volume;
                    let _ = app.mpv.set_property("volume", volume);
                }
                if response.drag_stopped() || (response.changed() && !response.dragged()) {
                    app.save_config();
                }
            },
        );
    });
}

fn draw_audio_timing(app: &mut PealayerApp, ui: &mut egui::Ui) {
    dialog::section(
        ui,
        icons::CLOCK_COUNTER_CLOCKWISE,
        &app.tr("Synchronization"),
        |ui| {
            dialog::setting_row(
                ui,
                icons::CLOCK,
                &app.tr("Audio delay"),
                Some(&app.tr("Use negative values when audio is heard too late")),
                |ui| {
                    let mut delay = app.audio_delay;
                    if dialog::numeric_stepper(
                        ui,
                        &mut delay,
                        MIN_AUDIO_DELAY..=MAX_AUDIO_DELAY,
                        0.1,
                        1,
                        " s",
                    ) {
                        app.audio_delay = clamp_audio_delay(delay);
                        let _ = app.mpv.set_property("audio-delay", app.audio_delay);
                        app.save_config();
                    }
                },
            );
            ui.horizontal(|ui| {
                if ui
                    .small_button(format!(
                        "{}  {}",
                        icons::ARROW_COUNTER_CLOCKWISE,
                        app.tr("Reset timing")
                    ))
                    .clicked()
                {
                    app.audio_delay = 0.0;
                    let _ = app.mpv.set_property("audio-delay", 0.0);
                    app.save_config();
                }
            });
        },
    );
}

fn draw_external_audio(app: &mut PealayerApp, ui: &mut egui::Ui) {
    dialog::section(ui, icons::FOLDER_OPEN, &app.tr("External audio"), |ui| {
        ui.label(
            egui::RichText::new(app.tr("Attach an audio file to the current media"))
                .small()
                .weak(),
        );
        if ui
            .button(format!("{}  {}", icons::PLUS, app.tr("Add audio file...")))
            .clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Audio Files", &["mp3", "flac", "wav", "m4a", "aac", "ogg"])
                .pick_file()
            && let Some(path_str) = path.to_str()
        {
            let _ = app.mpv.command("audio-add", &[path_str]);
            app.refresh_media_tracks();
        }
    });
}

fn audio_track_label(app: &PealayerApp, id: &str) -> String {
    if id == "no" {
        return app.tr("None");
    }
    let Some(track) = app
        .audio_tracks
        .iter()
        .find(|track| track.id.to_string() == id)
    else {
        return format!("{} {id}", app.tr("Track"));
    };
    let details = [track.lang.as_deref(), track.title.as_deref()]
        .into_iter()
        .flatten()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    if details.is_empty() {
        format!("{} {}", app.tr("Track"), track.id)
    } else {
        format!("{} {} — {details}", app.tr("Track"), track.id)
    }
}

pub fn clamp_audio_delay(delay: f64) -> f64 {
    delay.clamp(MIN_AUDIO_DELAY, MAX_AUDIO_DELAY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_values_and_geometry_are_bounded() {
        assert_eq!(clamp_audio_delay(0.0), 0.0);
        assert_eq!(clamp_audio_delay(-800.0), -600.0);
        assert_eq!(clamp_audio_delay(950.0), 600.0);
        assert!(AUDIO_DIALOG_MIN_HEIGHT < AUDIO_DIALOG_DEFAULT_HEIGHT);
        assert!(AUDIO_DIALOG_DEFAULT_HEIGHT < AUDIO_DIALOG_MAX_HEIGHT);
        assert!(AUDIO_TRACK_POPUP_HEIGHT < AUDIO_DIALOG_MAX_HEIGHT);
    }
}
