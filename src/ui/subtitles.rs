use crate::app::PealayerApp;
use eframe::egui;

const SUBTITLE_DIALOG_WIDTH: f32 = 420.0;
const SUBTITLE_DIALOG_MAX_HEIGHT: f32 = 440.0;
const SUBTITLE_DIALOG_BODY_HEIGHT: f32 = 340.0;
const SUBTITLE_TRACK_WIDTH: f32 = 240.0;
const SUBTITLE_TRACK_POPUP_HEIGHT: f32 = 200.0;

pub fn draw_settings_dialog(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_sub_settings {
        return;
    }

    let mut open = app.show_sub_settings;
    let mut close_requested = false;

    egui::Window::new(format!("CC {}", app.tr("Subtitle Settings")))
        // Use a new stable id so installs that remembered the old, accidentally
        // full-height geometry immediately return to the compact dialog.
        .id(egui::Id::new("subtitle_settings_dialog_compact"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width(SUBTITLE_DIALOG_WIDTH)
        .max_height(SUBTITLE_DIALOG_MAX_HEIGHT)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ui.ctx(), |ui| {
          ui.set_width(SUBTITLE_DIALOG_WIDTH - 24.0);
          egui::ScrollArea::vertical()
            .id_salt("subtitle_settings_body")
            .max_height(SUBTITLE_DIALOG_BODY_HEIGHT)
            .auto_shrink([false, true])
            .show(ui, |ui| {
          ui.with_layout(crate::ui::i18n::vertical_layout(app.rtl), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);

            // Visibility
            let mut vis = app.sub_visibility;
            if ui.checkbox(&mut vis, app.tr("Enable Subtitles")).changed() {
                let _ = app.mpv.set_property("sub-visibility", vis);
            }

            ui.separator();

            // Track Selection
            egui::Grid::new("subtitle_track_row")
              .num_columns(2)
              .spacing([12.0, 8.0])
              .show(ui, |ui| {
                ui.label(app.tr("Track:"));
                let current_label = if app.current_sid == "no" {
                    app.tr("None").to_string()
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

                let none_label = app.tr("None");
                egui::ComboBox::from_id_salt("sub_track_combo")
                    .selected_text(current_label)
                    .width(SUBTITLE_TRACK_WIDTH)
                    // A media file may contain many subtitle tracks. The popup
                    // should scroll instead of stretching to viewport height.
                    .height(SUBTITLE_TRACK_POPUP_HEIGHT)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(&mut app.current_sid, "no".to_string(), none_label)
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
                ui.end_row();
            });

            ui.separator();

            // Appearance
            ui.label(app.tr("Appearance"));
            ui.with_layout(crate::ui::i18n::layout(app.rtl, egui::Align::Center), |ui| {
                ui.label(app.tr("Font Size:"));
                let mut font_size = app.sub_font_size;
                if ui
                    .add(egui::Slider::new(&mut font_size, 10.0..=100.0))
                    .changed()
                {
                    app.sub_font_size = font_size;
                    let _ = app.mpv.set_property("sub-font-size", font_size);
                }
            });

            ui.separator();

            // Synchronization
            ui.label(app.tr("Synchronization"));
            ui.with_layout(crate::ui::i18n::layout(app.rtl, egui::Align::Center), |ui| {
                ui.label(app.tr("Delay (s):"));
                let mut delay = app.sub_delay;
                if ui
                    .add(
                        egui::DragValue::new(&mut delay)
                            .speed(0.1)
                            .range(MIN_SUB_DELAY..=MAX_SUB_DELAY),
                    )
                    .changed()
                {
                    app.sub_delay = delay;
                    let _ = app.mpv.set_property("sub-delay", delay);
                }
                if ui.button(app.tr("Reset")).clicked() {
                    app.sub_delay = 0.0;
                    let _ = app.mpv.set_property("sub-delay", 0.0);
                }
            });

            ui.separator();

            // Load External
            if ui.button(app.tr("Load External Subtitle...")).clicked() {
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
          });
          ui.separator();
          ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(app.tr("Close")).clicked() {
                close_requested = true;
            }
          });
        });

    app.show_sub_settings = open && !close_requested;
}

pub const MIN_SUB_DELAY: f64 = -600.0;
pub const MAX_SUB_DELAY: f64 = 600.0;

pub fn clamp_sub_delay(delay: f64) -> f64 {
    delay.clamp(MIN_SUB_DELAY, MAX_SUB_DELAY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sub_delay_range_clamping() {
        assert_eq!(clamp_sub_delay(0.0), 0.0);
        assert_eq!(clamp_sub_delay(-750.0), -600.0);
        assert_eq!(clamp_sub_delay(800.0), 600.0);
        assert_eq!(clamp_sub_delay(35.5), 35.5);
    }

    #[test]
    fn subtitle_dialog_and_track_popup_remain_bounded() {
        assert!(SUBTITLE_DIALOG_MAX_HEIGHT < 500.0);
        assert!(SUBTITLE_DIALOG_BODY_HEIGHT < SUBTITLE_DIALOG_MAX_HEIGHT);
        assert!(SUBTITLE_TRACK_POPUP_HEIGHT < SUBTITLE_DIALOG_MAX_HEIGHT);
        assert!(SUBTITLE_TRACK_WIDTH < SUBTITLE_DIALOG_WIDTH - 100.0);
    }
}
