use crate::app::PealayerApp;
use eframe::egui;

const SUBTITLE_DIALOG_WIDTH: f32 = 420.0;
const SUBTITLE_DIALOG_MAX_HEIGHT: f32 = 440.0;
const SUBTITLE_DIALOG_BODY_HEIGHT: f32 = 360.0;
const SUBTITLE_TRACK_WIDTH: f32 = 240.0;
const SUBTITLE_TRACK_POPUP_HEIGHT: f32 = 200.0;

pub fn draw_settings_dialog(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_sub_settings {
        return;
    }

    let mut open = app.show_sub_settings;
    let mut close_requested = false;

    let bounds = ui.ctx().content_rect().shrink(20.0);
    let max_size = egui::vec2(
        bounds.width().min(460.0),
        bounds.height().min(SUBTITLE_DIALOG_MAX_HEIGHT),
    );
    let default_size = egui::vec2(max_size.x.min(SUBTITLE_DIALOG_WIDTH), max_size.y.min(420.0));
    let default_rect = crate::ui::dialog::centered_default_rect(bounds, default_size);

    if crate::ui::dialog::escape_pressed(ui.ctx()) {
        close_requested = true;
    }

    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::SUBTITLES,
        app.tr("Subtitle Settings")
    ))
    // Reset geometry remembered by both earlier unbounded implementations.
    .id(egui::Id::new("subtitle_settings_dialog_bounded_v3"))
    .open(&mut open)
    .collapsible(false)
    .resizable(true)
    .default_rect(default_rect)
    .min_size([340.0_f32.min(max_size.x), 280.0_f32.min(max_size.y)])
    .max_size(max_size)
    .constrain_to(bounds)
    .movable(true)
    .show(ui.ctx(), |ui| {
        ui.set_max_width(max_size.x);
        crate::ui::dialog::scroll_column(
            ui,
            "subtitle_settings_body_v3",
            Some(SUBTITLE_DIALOG_BODY_HEIGHT.min(ui.available_height() - 38.0)),
            |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);

                crate::ui::dialog::section(
                    ui,
                    crate::ui::icons::SUBTITLES,
                    &app.tr("Subtitles"),
                    |ui| {
                        let mut vis = app.sub_visibility;
                        if ui.checkbox(&mut vis, app.tr("Enable Subtitles")).changed() {
                            app.set_subtitle_visibility(vis);
                        }
                    },
                );
                ui.add_space(8.0);

                crate::ui::dialog::section(
                    ui,
                    crate::ui::icons::LIST_CHECKS,
                    &app.tr("Track"),
                    |ui| {
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

                        let none_label = app.tr("Subtitles hidden");
                        let tracks = app.sub_tracks.clone();
                        let combo_width =
                            (ui.available_width() - 4.0).clamp(140.0, SUBTITLE_TRACK_WIDTH);
                        egui::ComboBox::from_id_salt("sub_track_combo")
                            .selected_text(current_label)
                            .width(combo_width)
                            .height(SUBTITLE_TRACK_POPUP_HEIGHT)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(app.current_sid == "no", none_label)
                                    .clicked()
                                {
                                    app.disable_media_track(crate::app::MediaTrackType::Subtitle);
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
                                        .selectable_label(app.current_sid == track_id_str, label)
                                        .clicked()
                                    {
                                        app.select_media_track(crate::app::MediaTrackKey {
                                            kind: crate::app::MediaTrackType::Subtitle,
                                            id: track.id,
                                        });
                                    }
                                }
                            });
                    },
                );
                ui.add_space(8.0);

                crate::ui::dialog::section(
                    ui,
                    crate::ui::icons::SPARKLE,
                    &app.tr("Appearance"),
                    |ui| {
                        crate::ui::dialog::compact_row(ui, app.rtl, |ui| {
                            ui.label(app.tr("Font Size:"));
                            let mut font_size = app.sub_font_size;
                            if ui
                                .add(egui::Slider::new(&mut font_size, 10.0..=100.0))
                                .changed()
                            {
                                app.sub_font_size = font_size;
                                let _ = app.mpv.set_property("sub-font-size", font_size);
                                app.sync_subtitle_rendering();
                            }
                        });
                    },
                );
                ui.add_space(8.0);

                crate::ui::dialog::section(
                    ui,
                    crate::ui::icons::CLOCK_COUNTER_CLOCKWISE,
                    &app.tr("Synchronization"),
                    |ui| {
                        crate::ui::dialog::compact_row(ui, app.rtl, |ui| {
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
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                                    app.tr("Reset")
                                ))
                                .clicked()
                            {
                                app.sub_delay = 0.0;
                                let _ = app.mpv.set_property("sub-delay", 0.0);
                            }
                        });
                    },
                );
                ui.add_space(8.0);

                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::FOLDER_OPEN,
                        app.tr("Load External Subtitle...")
                    ))
                    .clicked()
                {
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
                            app.refresh_media_tracks();
                        }
                    }
                }
            },
        );
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
