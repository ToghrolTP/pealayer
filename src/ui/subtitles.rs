use crate::ui::dropdown::DropdownUiExt;
use crate::app::{MediaTrackKey, MediaTrackType, PealayerApp};
use crate::ui::{dialog, icons};
use eframe::egui;

const SUBTITLE_DIALOG_WIDTH: f32 = 540.0;
const SUBTITLE_DIALOG_DEFAULT_HEIGHT: f32 = 570.0;
const SUBTITLE_DIALOG_MIN_HEIGHT: f32 = 350.0;
const SUBTITLE_DIALOG_MAX_HEIGHT: f32 = 680.0;
const SUBTITLE_DIALOG_FOOTER_RESERVE: f32 = 42.0;
const SUBTITLE_TRACK_POPUP_HEIGHT: f32 = 220.0;

pub const MIN_SUB_DELAY: f64 = -600.0;
pub const MAX_SUB_DELAY: f64 = 600.0;
pub const MIN_SUB_POSITION: f64 = 0.0;
pub const MAX_SUB_POSITION: f64 = 100.0;

pub fn draw_settings_dialog(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_sub_settings {
        return;
    }

    let mut open = app.show_sub_settings;
    let geometry = dialog::bounded_geometry(
        ui.ctx().content_rect(),
        20.0,
        egui::vec2(SUBTITLE_DIALOG_WIDTH, SUBTITLE_DIALOG_DEFAULT_HEIGHT),
        egui::vec2(390.0, SUBTITLE_DIALOG_MIN_HEIGHT),
        egui::vec2(620.0, SUBTITLE_DIALOG_MAX_HEIGHT),
    );
    let mut close_requested = dialog::escape_pressed(ui.ctx());

    egui::Window::new(format!(
        "{} {}",
        icons::SUBTITLES,
        app.tr("Subtitle Settings")
    ))
    .id(egui::Id::new("subtitle_settings_dialog_professional_v4"))
    .order(egui::Order::Foreground)
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
        let body_height = (ui.available_height() - SUBTITLE_DIALOG_FOOTER_RESERVE).max(120.0);
        dialog::scroll_column(ui, "subtitle_settings_body_v4", Some(body_height), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

            draw_subtitle_status(app, ui);
            ui.add_space(5.0);
            draw_subtitle_track(app, ui);
            ui.add_space(5.0);
            draw_subtitle_appearance(app, ui);
            ui.add_space(5.0);
            draw_subtitle_timing(app, ui);
            ui.add_space(5.0);
            draw_external_subtitle(app, ui);
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

    app.show_sub_settings = open && !close_requested;
}

fn draw_subtitle_status(app: &mut PealayerApp, ui: &mut egui::Ui) {
    dialog::section(ui, icons::EYE, &app.tr("Visibility"), |ui| {
        dialog::setting_row(
            ui,
            icons::SUBTITLES,
            &app.tr("Show subtitles"),
            Some(&app.tr("Render the selected subtitle track over the video")),
            |ui| {
                let mut visible = app.sub_visibility;
                if ui.toggle_value(&mut visible, app.tr("Enabled")).changed() {
                    app.set_subtitle_visibility(visible);
                    app.save_config();
                }
            },
        );
    });
}

fn draw_subtitle_track(app: &mut PealayerApp, ui: &mut egui::Ui) {
    dialog::section(ui, icons::LIST_CHECKS, &app.tr("Track"), |ui| {
        let current_label = subtitle_track_label(app, &app.current_sid);
        let tracks = app.sub_tracks.clone();
        dialog::setting_row(
            ui,
            icons::SUBTITLES,
            &app.tr("Subtitle track"),
            Some(&app.tr("Choose an embedded or externally loaded track")),
            |ui| {
                crate::ui::dropdown::ComboBox::from_id_salt("sub_track_combo_v4")
                    .selected_text(current_label)
                    .width(ui.available_width().clamp(150.0, 275.0))
                    .height(SUBTITLE_TRACK_POPUP_HEIGHT)
                    .show_ui(ui, |ui| {
                        if ui
                            .dropdown_choice(app.current_sid == "no", app.tr("None"))
                            .clicked()
                        {
                            app.disable_media_track(MediaTrackType::Subtitle);
                        }
                        for track in tracks {
                            let id = track.id.to_string();
                            let label = subtitle_track_label(app, &id);
                            if ui.dropdown_choice(app.current_sid == id, label).clicked() {
                                app.select_media_track(MediaTrackKey {
                                    kind: MediaTrackType::Subtitle,
                                    id: track.id,
                                });
                            }
                        }
                    });
            },
        );
    });
}

fn draw_subtitle_appearance(app: &mut PealayerApp, ui: &mut egui::Ui) {
    dialog::section(
        ui,
        icons::SPARKLE,
        &app.tr("Appearance and placement"),
        |ui| {
            dialog::setting_row(
                ui,
                icons::TEXT_ALIGN_LEFT,
                &app.tr("Text direction"),
                Some(&app.tr("Automatic, left-to-right, or right-to-left layout")),
                |ui| {
                    let mut direction = app.subtitle_direction;
                    crate::ui::dropdown::ComboBox::from_id_salt("subtitle_direction_combo")
                        .selected_text(match direction {
                            crate::subtitle::SubtitleDirection::Auto => app.tr("Automatic"),
                            crate::subtitle::SubtitleDirection::Ltr => app.tr("Left to right"),
                            crate::subtitle::SubtitleDirection::Rtl => app.tr("Right to left"),
                        })
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            ui.dropdown_value(
                                &mut direction,
                                crate::subtitle::SubtitleDirection::Auto,
                                app.tr("Automatic"),
                            );
                            ui.dropdown_value(
                                &mut direction,
                                crate::subtitle::SubtitleDirection::Ltr,
                                app.tr("Left to right"),
                            );
                            ui.dropdown_value(
                                &mut direction,
                                crate::subtitle::SubtitleDirection::Rtl,
                                app.tr("Right to left"),
                            );
                        });
                    if direction != app.subtitle_direction {
                        app.subtitle_direction = direction;
                        app.sync_subtitle_rendering();
                        app.save_config();
                    }
                },
            );
            ui.separator();

            dialog::setting_row(
                ui,
                icons::TEXT_ALIGN_CENTER,
                &app.tr("Text alignment"),
                Some(&app.tr("Horizontal placement is independent of text direction")),
                |ui| {
                    use crate::subtitle::SubtitleAlignment;
                    let mut alignment = app.subtitle_alignment;
                    let label = match alignment {
                        SubtitleAlignment::Left => "Left",
                        SubtitleAlignment::Center => "Center",
                        SubtitleAlignment::Right => "Right",
                        SubtitleAlignment::SubtitleStyle => "Subtitle style",
                    };
                    crate::ui::dropdown::ComboBox::from_id_salt("subtitle_alignment_combo")
                        .selected_text(app.tr(label))
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for (value, label, icon) in [
                                (SubtitleAlignment::Left, "Left", icons::TEXT_ALIGN_LEFT),
                                (SubtitleAlignment::Center, "Center", icons::TEXT_ALIGN_CENTER),
                                (SubtitleAlignment::Right, "Right", icons::TEXT_ALIGN_RIGHT),
                                (SubtitleAlignment::SubtitleStyle, "Subtitle style", icons::SUBTITLES),
                            ] {
                                let response = ui.dropdown_value(&mut alignment, value, format!("{icon}  {}", app.tr(label)));
                                if value == SubtitleAlignment::SubtitleStyle {
                                    response.on_hover_text(app.tr("Preserve subtitle styling without text processing; processed text stays centered"));
                                }
                            }
                        });
                    if alignment != app.subtitle_alignment {
                        app.subtitle_alignment = alignment;
                        app.sync_subtitle_rendering();
                        app.save_config();
                    }
                },
            );
            ui.separator();

            dialog::setting_row(
                ui,
                icons::SLIDERS_HORIZONTAL,
                &app.tr("Font size"),
                Some(&app.tr("Scale subtitle text without changing the video")),
                |ui| {
                    let mut size = app.sub_font_size;
                    let mut changed = dialog::receive_numeric_paste(ui, "subtitle_font_size", &mut size, &(10.0..=100.0), " px");
                    let response = ui.add_sized(
                        [185.0, 24.0],
                        egui::Slider::new(&mut size, 10.0..=100.0).suffix(" px"),
                    );
                    changed |= dialog::numeric_context_menu(ui, &response, "subtitle_font_size", &mut size, 10.0..=100.0, 1.0, 55.0, " px", &mut app.numeric_input_steps, app.language, 1e-9);
                    if response.changed() || changed {
                        app.sub_font_size = size;
                        let _ = app.mpv.set_property("sub-font-size", size);
                        app.sync_subtitle_rendering();
                    }
                    if changed || response.drag_stopped() || (response.changed() && !response.dragged()) {
                        app.save_config();
                    }
                },
            );
            ui.separator();

            dialog::setting_row(
                ui,
                icons::ARROWS_IN,
                &app.tr("Location offset"),
                Some(&app.tr("0% places subtitles at the top; 100% places them at the bottom")),
                |ui| {
                    let mut position = app.sub_position_percent;
                    if dialog::numeric_stepper(
                        ui,
                        "subtitle_position_percent",
                        &mut position,
                        MIN_SUB_POSITION..=MAX_SUB_POSITION,
                        1.0,
                        100.0,
                        0,
                        "%",
                        &mut app.numeric_input_steps,
                        app.language,
                    ) {
                        app.sub_position_percent = position;
                        let _ = app.mpv.set_property("sub-pos", position);
                        app.sync_subtitle_rendering();
                        app.save_config();
                    }
                },
            );
        },
    );
}

fn draw_subtitle_timing(app: &mut PealayerApp, ui: &mut egui::Ui) {
    dialog::section(
        ui,
        icons::CLOCK_COUNTER_CLOCKWISE,
        &app.tr("Synchronization"),
        |ui| {
            dialog::setting_row(
                ui,
                icons::CLOCK,
                &app.tr("Subtitle delay"),
                Some(&app.tr("Use negative values when subtitles appear too late")),
                |ui| {
                    let mut delay = app.sub_delay;
                    if dialog::numeric_stepper(
                        ui,
                        "subtitle_delay_seconds",
                        &mut delay,
                        MIN_SUB_DELAY..=MAX_SUB_DELAY,
                        0.1,
                        0.0,
                        1,
                        " s",
                        &mut app.numeric_input_steps,
                        app.language,
                    ) {
                        app.sub_delay = clamp_sub_delay(delay);
                        let _ = app.mpv.set_property("sub-delay", app.sub_delay);
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
                    app.sub_delay = 0.0;
                    let _ = app.mpv.set_property("sub-delay", 0.0);
                    app.save_config();
                }
            });
        },
    );
}

fn draw_external_subtitle(app: &mut PealayerApp, ui: &mut egui::Ui) {
    dialog::section(ui, icons::FOLDER_OPEN, &app.tr("External subtitle"), |ui| {
        ui.label(
            egui::RichText::new(
                app.tr("Attach an SRT, VTT, ASS, or SSA file to the current media"),
            )
            .small()
            .weak(),
        );
        if ui
            .button(format!(
                "{}  {}",
                icons::PLUS,
                app.tr("Add subtitle file...")
            ))
            .clicked()
        {
            if crate::peer::active() {
                crate::ui::peer_browser::open(ui.ctx(), crate::ui::peer_browser::Purpose::Subtitle, None);
            } else if let Some(path) = rfd::FileDialog::new()
                .add_filter("Subtitles", &["srt", "vtt", "ass", "ssa"])
                .pick_file()
            && let Some(path_str) = path.to_str()
            {
            let _ = app.mpv.command("sub-add", &[path_str]);
            app.refresh_media_tracks();
            }
        }
    });
}

fn subtitle_track_label(app: &PealayerApp, id: &str) -> String {
    if id == "no" {
        return app.tr("None");
    }
    let Some(track) = app
        .sub_tracks
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

pub fn clamp_sub_delay(delay: f64) -> f64 {
    delay.clamp(MIN_SUB_DELAY, MAX_SUB_DELAY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtitle_values_are_bounded() {
        assert_eq!(clamp_sub_delay(0.0), 0.0);
        assert_eq!(clamp_sub_delay(-750.0), -600.0);
        assert_eq!(clamp_sub_delay(800.0), 600.0);
        assert!(SUBTITLE_DIALOG_MIN_HEIGHT < SUBTITLE_DIALOG_DEFAULT_HEIGHT);
        assert!(SUBTITLE_DIALOG_DEFAULT_HEIGHT < SUBTITLE_DIALOG_MAX_HEIGHT);
        assert!(SUBTITLE_TRACK_POPUP_HEIGHT < SUBTITLE_DIALOG_MAX_HEIGHT);
    }
}
