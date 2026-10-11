use crate::ui::dropdown::DropdownUiExt;
use crate::app::{MediaTrackKey, MediaTrackType, PealayerApp};
use crate::ui::{dialog, icons};
use crate::ui::effects_library::designer_tr;
use eframe::egui;

const AUDIO_DIALOG_WIDTH: f32 = 520.0;
const AUDIO_DIALOG_DEFAULT_HEIGHT: f32 = 500.0;
const AUDIO_DIALOG_MIN_HEIGHT: f32 = 330.0;
const AUDIO_DIALOG_MAX_HEIGHT: f32 = 620.0;
const AUDIO_DIALOG_FOOTER_RESERVE: f32 = 42.0;
const AUDIO_TRACK_POPUP_HEIGHT: f32 = 220.0;

pub(crate) fn draw_sfx_editor(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let mut open = app.show_effect_library_editor;
    let geometry = dialog::bounded_geometry(ui.ctx().content_rect(), 20.0, egui::vec2(650.0, 470.0), egui::vec2(440.0, 340.0), egui::vec2(850.0, 680.0));
    let effects = app.audio_effects();
    let devices = crate::peer::client().and_then(|c| c.snapshot()).and_then(|s| serde_json::from_value::<Vec<crate::mpv::audio_output::AudioDevice>>(s.session.status.get("audio_devices")?.clone()).ok()).unwrap_or_else(crate::mpv::audio_output::available_audio_devices);
    egui::Window::new(format!("{} {}", icons::SPEAKER_HIGH, app.tr("Effects Designer")))
        .id(egui::Id::new("controller_effect_library_dialog")).open(&mut open)
        .default_rect(geometry.default_rect).min_size(geometry.min_size).max_size(geometry.max_size)
        .constrain_to(geometry.bounds).frame(dialog::opaque_window_frame(ui)).order(egui::Order::Foreground)
        .collapsible(false).resizable(true).show(ui.ctx(), |ui| {
            egui::ScrollArea::vertical().id_salt("sfx-editor").show(ui, |ui| {
                ui.horizontal(|ui| {
                    let selected = app.effect_library_draft.name.clone();
                    crate::ui::dropdown::ComboBox::from_id_salt("sfx-library-selection").selected_text(if selected.is_empty() { designer_tr(ui, "New audio effect") } else { selected }).show_ui(ui, |ui| {
                        for effect in &effects { if ui.dropdown_choice(app.effect_library_draft.id == effect.id.to_string(), &effect.name).clicked() { app.select_audio_effect(effect.id); } }
                    });
                    if ui.button(format!("{} {}", icons::PLUS, designer_tr(ui, "New"))).clicked() { app.begin_audio_effect(); }
                });
                ui.separator();
                let draft = &mut app.effect_library_draft;
                let mut program: crate::mpv::sfx::AudioProgram = serde_json::from_str(&draft.program_json).unwrap_or_default();
                if let Some(file) = crate::ui::peer_browser::take_preference_file(ui.ctx(), "sfx_source") { program.source = file; }
                egui::Grid::new("sfx-identity").num_columns(2).spacing([14.0, 10.0]).show(ui, |ui| {
                    ui.label(designer_tr(ui, "Name")); ui.add(dialog::singleline_text_edit(&mut draft.name).desired_width(ui.available_width())); ui.end_row();
                    ui.label(designer_tr(ui, "Group")); crate::ui::group_picker::group_picker(ui, "sfx-group", &mut draft.category, effects.iter().map(|effect| effect.group.as_str()), ui.available_width(), app.language); ui.end_row();
                    ui.label(designer_tr(ui, "Icon")); crate::ui::icons::searchable_control_icon_picker(ui, "sfx-icon", &mut draft.icon, ui.available_width(), &designer_tr(ui, "Search icons..."), &designer_tr(ui, "Presets"), &designer_tr(ui, "No matching icons"), app.language); ui.end_row();
                    ui.label(designer_tr(ui, "Audio file / URL")); ui.horizontal(|ui| {
                        ui.add(dialog::singleline_text_edit(&mut program.source).desired_width((ui.available_width()-86.0).max(100.0)));
                        if ui.button(format!("{} {}", icons::FOLDER_OPEN, designer_tr(ui, "Browse"))).clicked() {
                            if crate::peer::active() { crate::ui::peer_browser::open(ui.ctx(), crate::ui::peer_browser::Purpose::PreferenceFile { key: "sfx_source".into(), extensions: vec!["wav","mp3","ogg","flac","m4a","aac","opus"].into_iter().map(str::to_string).collect() }, None); }
                            else if let Some(file) = rfd::FileDialog::new().set_title(designer_tr(ui, "Choose sound effect")).add_filter(designer_tr(ui, "Audio"), &["wav","mp3","ogg","flac","m4a","aac","opus"]).pick_file() { program.source = file.to_string_lossy().into(); }
                        }
                    }); ui.end_row();
                    ui.label(designer_tr(ui, "Volume")); ui.add(egui::Slider::new(&mut program.volume, 0..=100).suffix("%")); ui.end_row();
                    ui.label(designer_tr(ui, "Output device / backend"));
                    let selected = devices.iter().find(|d| d.name == program.output_device).map(|d| d.description.clone()).unwrap_or_else(|| designer_tr(ui, if program.output_device.is_empty() { "Preferences SFX output" } else { "Unavailable device" }));
                    let popup = crate::ui::dropdown::ComboBox::from_id_salt("sfx-output").width(ui.available_width()).selected_text(selected).show_ui(ui, |ui| {
                        ui.dropdown_value(&mut program.output_device, String::new(), designer_tr(ui, "Preferences SFX output"));
                        for device in &devices { ui.dropdown_value(&mut program.output_device, device.name.clone(), &device.description).on_hover_text(&device.name); }
                    });
                    let key = ui.id().with("sfx-output-open"); let was_open = ui.data_mut(|d| d.get_temp::<bool>(key).unwrap_or(false));
                    if popup.inner.is_some() && !was_open {
                        if let Some(client) = crate::peer::client() {
                            let _ = client.queue("/api/player/command", serde_json::to_value(crate::platform::interop::InteropCommand::RefreshAudioOutputs).unwrap());
                        } else {
                            let ctx = ui.ctx().clone();
                            std::thread::spawn(move || { crate::mpv::audio_output::refresh_devices(); ctx.request_repaint(); });
                        }
                    }
                    ui.data_mut(|d| d.insert_temp(key, popup.inner.is_some())); ui.end_row();
                    ui.label(designer_tr(ui, "Length")); ui.weak(if draft.duration_ms == 0 { designer_tr(ui, "Read from audio on Save") } else { crate::duration::format_effect_duration_for_language(app.language, draft.duration_ms) }); ui.end_row();
                });
                draft.program_json = serde_json::to_string(&program).unwrap();
                ui.add_space(8.0); ui.separator();
                let reference = app.effect_library_draft.reference.clone();
                let pending = crate::mpv::sfx::importing();
                ui.horizontal_wrapped(|ui| {
                    if ui.add_enabled(!pending && !app.effect_library_draft.name.trim().is_empty() && program.validate().is_ok(), egui::Button::new(format!("{} {}", icons::FLOPPY_DISK, designer_tr(ui, "Save")))).clicked() { if let Err(error) = app.save_controller_effect() { app.set_osd(error); } }
                    if ui.add_enabled(!pending && !reference.is_empty() && !app.estop_active, egui::Button::new(format!("{} {}", icons::PLAY, designer_tr(ui, "Preview")))).clicked() { if let Err(error) = app.play_controller_effect(&reference) { app.set_osd(error); } }
                    if ui.button(format!("{} {}", icons::STOP_CIRCLE, designer_tr(ui, "Stop"))).clicked() { let _ = app.stop_controller_effect(&reference); }
                    if ui.add_enabled(!reference.is_empty() && !pending, egui::Button::new(format!("{} {}", icons::TRASH, designer_tr(ui, "Delete")))).clicked() { if let Err(error) = app.delete_controller_effect() { app.set_osd(error); } }
                    if pending { ui.spinner(); ui.weak(designer_tr(ui, "Reading audio…")); }
                });
                if !app.hardware_effect_authoring.status.is_empty() { ui.weak(&app.hardware_effect_authoring.status); }
            });
        });
    app.show_effect_library_editor = open;
}

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
                crate::ui::dropdown::ComboBox::from_id_salt("audio_track_combo_v4")
                    .selected_text(current_label)
                    .width(ui.available_width().clamp(150.0, 275.0))
                    .height(AUDIO_TRACK_POPUP_HEIGHT)
                    .show_ui(ui, |ui| {
                        if ui
                            .dropdown_choice(app.current_aid == "no", app.tr("None"))
                            .clicked()
                        {
                            app.disable_media_track(MediaTrackType::Audio);
                        }
                        for track in tracks {
                            let id = track.id.to_string();
                            let label = audio_track_label(app, &id);
                            if ui.dropdown_choice(app.current_aid == id, label).clicked() {
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
                let mut changed = dialog::receive_numeric_paste(ui, "volume", &mut volume, &(0.0..=130.0), "%");
                let response = ui.add_sized(
                    [190.0, 24.0],
                    egui::Slider::new(&mut volume, 0.0..=130.0).suffix("%"),
                );
                let wheel_steps = app.numeric_input_steps.get("volume").copied()
                    .unwrap_or_else(|| crate::config::NumericInputSteps::for_step(1.0));
                changed |= dialog::numeric_slider_wheel(ui, &response, &mut volume, 0.0..=130.0, wheel_steps);
                changed |= dialog::numeric_context_menu(ui, &response, "volume", &mut volume, 0.0..=130.0, 1.0, crate::config::AppConfig::default().volume, "%", &mut app.numeric_input_steps, app.language, 1e-9);
                if response.changed() || changed {
                    app.volume = volume;
                    let _ = app.mpv.set_property("volume", volume);
                }
                if changed || response.drag_stopped() || (response.changed() && !response.dragged()) {
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
                        "audio_delay_seconds",
                        &mut delay,
                        MIN_AUDIO_DELAY..=MAX_AUDIO_DELAY,
                        0.1,
                        0.0,
                        1,
                        " s",
                        &mut app.numeric_input_steps,
                        app.language,
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
        {
            if crate::peer::active() {
                crate::ui::peer_browser::open(ui.ctx(), crate::ui::peer_browser::Purpose::Audio, None);
            } else if let Some(path) = rfd::FileDialog::new()
                .add_filter("Audio Files", &["mp3", "flac", "wav", "m4a", "aac", "ogg"])
                .pick_file()
            && let Some(path_str) = path.to_str()
            {
            let _ = app.mpv.command("audio-add", &[path_str]);
            app.refresh_media_tracks();
            }
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
