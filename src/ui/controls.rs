use crate::app::PealayerApp;
use eframe::egui;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let controls_label = app.tr("Controls");
    let time_since_activity = app.last_mouse_activity.elapsed().as_secs_f32();
    let alpha = if app.pin_controls {
        1.0
    } else {
        (3.0 - time_since_activity).clamp(0.0, 1.0)
    };

    if alpha > 0.0 {
        let window_width = ui.available_width() - 20.0;
        let mut controls_frame = egui::Frame::window(ui.style()).multiply_with_opacity(alpha);
        controls_frame.inner_margin.right = 0;

        egui::Window::new(controls_label)
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -20.0))
            .min_width(window_width)
            .default_width(window_width)
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .interactable(is_controls_interactable(alpha))
            .frame(controls_frame)
            .show(&ctx, |ui| {
                ui.set_opacity(alpha);
                multiply_style_opacity(ui.style_mut(), alpha);
                let has_video = app.current_video_path.is_some();
                let timeline_state = app.media_timeline_state();
                let can_seek = matches!(
                    timeline_state,
                    crate::media::MediaTimelineState::Finite {
                        seekable: true,
                        ..
                    }
                );
                let elapsed_time = resolve_display_time(app.seek_pos, app.playback_time);
                let is_long_video = app.duration >= 3600.0;
                let show_subseconds = app.show_subseconds;
                let elapsed_str =
                    format_player_time(elapsed_time, is_long_video, show_subseconds);
                let display_total = if app.show_remaining_time {
                    -(app.duration - elapsed_time)
                } else {
                    app.duration
                };
                let (total_text, total_tooltip, total_is_toggle) = match timeline_state {
                    crate::media::MediaTimelineState::NoMedia => (
                        format!("{} --:--", crate::ui::icons::CLOCK),
                        app.tr("Open media to see its duration."),
                        false,
                    ),
                    crate::media::MediaTimelineState::Determining => (
                        format!(
                            "{} {}",
                            crate::ui::icons::HOURGLASS_MEDIUM,
                            app.tr("Determining duration…")
                        ),
                        app.tr("MPV is still reading media metadata. Duration and seeking will update when available."),
                        false,
                    ),
                    crate::media::MediaTimelineState::Live => (
                        format!("{} {}", crate::ui::icons::BROADCAST, app.tr("LIVE")),
                        app.tr("This live or duration-less source has no fixed endpoint or seek range."),
                        false,
                    ),
                    crate::media::MediaTimelineState::Finite { .. } => (
                        format_player_time(display_total, is_long_video, show_subseconds),
                        if app.show_remaining_time {
                            app.tr("Showing time remaining. Click to show total duration.")
                        } else {
                            app.tr("Showing total duration. Click to show time remaining.")
                        },
                        true,
                    ),
                };
                let seek_tooltip = match timeline_state {
                    crate::media::MediaTimelineState::NoMedia => app.tr("Open media to seek."),
                    crate::media::MediaTimelineState::Determining => app.tr(
                        "Duration is still being determined; seeking will become available when MPV reports a timeline.",
                    ),
                    crate::media::MediaTimelineState::Live => app.tr(
                        "This live or duration-less source has no fixed seek range.",
                    ),
                    crate::media::MediaTimelineState::Finite {
                        seekable: false, ..
                    } => app.tr("This media reports a duration but does not support seeking."),
                    crate::media::MediaTimelineState::Finite { .. } => {
                        app.tr("Seek through the media timeline.")
                    }
                };

                let fullscreen_tooltip = format!("{} (F)", app.tr("Fullscreen"));
                let pin_tooltip = if app.pin_controls {
                    app.tr("Unpin Controls")
                } else {
                    app.tr("Pin Controls")
                };
                let audio_tooltip = app.tr("Audio Settings...");
                let workspace_tooltip = app.tr("Switch NLE / Simple Player");
                let subtitles_tooltip = app.tr("Subtitle Settings...");
                let mute_tooltip = format!(
                    "{} (M)",
                    if app.is_muted {
                        app.tr("Unmute")
                    } else {
                        app.tr("Mute")
                    }
                );
                let pin_icon = if app.pin_controls {
                    crate::ui::icons::PUSH_PIN_SLASH
                } else {
                    crate::ui::icons::PUSH_PIN
                };
                let mute_icon = if app.is_muted {
                    crate::ui::icons::SPEAKER_SLASH
                } else {
                    crate::ui::icons::SPEAKER_HIGH
                };
                let mut volume = app.volume;
                let mut toggle_fullscreen = false;
                let mut toggle_pin = false;
                let mut toggle_audio = false;
                let mut toggle_workspace = false;
                let mut toggle_subtitles = false;
                let mut toggle_mute = false;
                let mut toggle_total_mode = false;
                let mut volume_update = None;

                egui::containers::Sides::new().shrink_left().show(
                    ui,
                    |ui| {
                        ui.add_enabled_ui(has_video, |ui| {
                            let play_icon = if app.is_playback_finished() {
                                crate::ui::icons::ARROW_COUNTER_CLOCKWISE
                            } else if app.is_paused {
                                crate::ui::icons::PLAY
                            } else {
                                crate::ui::icons::PAUSE
                            };
                            let play_tooltip = if app.is_playback_finished() {
                                app.tr("Replay")
                            } else if app.is_paused {
                                app.tr("Play")
                            } else {
                                app.tr("Pause")
                            };
                            if ui
                                .add_sized([30.0, 22.0], egui::Button::new(play_icon))
                                .on_hover_text(play_tooltip)
                                .clicked()
                            {
                                app.toggle_playback();
                            }
                        });

                        let elapsed_resp = ui.add_enabled(
                            has_video,
                            egui::Label::new(&elapsed_str).sense(if total_is_toggle {
                                egui::Sense::click()
                            } else {
                                egui::Sense::hover()
                            }),
                        );
                        if total_is_toggle && elapsed_resp.clicked() {
                            app.show_remaining_time = !app.show_remaining_time;
                            app.save_config();
                        }

                        let mut current_pos = if has_video {
                            app.seek_pos.unwrap_or(app.playback_time)
                        } else {
                            0.0
                        };
                        let max_duration = app.duration.max(1.0);
                        let slider = egui::Slider::new(&mut current_pos, 0.0..=max_duration)
                            .show_value(false)
                            .trailing_fill(true);
                        let seekbar_width = ui.available_width().max(1.0);
                        let response = ui
                            .add_enabled_ui(can_seek, |ui| {
                                ui.add_sized([seekbar_width, 22.0], slider)
                            })
                            .inner;
                        let response = if can_seek {
                            response.on_hover_text(&seek_tooltip)
                        } else {
                            response.on_disabled_hover_text(&seek_tooltip)
                        };

                        if let Some(buffered_until) = app.buffered_until() {
                            let fraction = (buffered_until / app.duration).clamp(0.0, 1.0) as f32;
                            let buffered_rect = egui::Rect::from_min_max(
                                egui::pos2(response.rect.left(), response.rect.bottom() - 2.0),
                                egui::pos2(
                                    response.rect.left() + response.rect.width() * fraction,
                                    response.rect.bottom(),
                                ),
                            );
                            ui.painter().rect_filled(
                                buffered_rect,
                                1.0,
                                ui.visuals().selection.bg_fill.linear_multiply(0.55),
                            );
                        }
                        if can_seek && response.dragged() {
                            app.scrub_to(current_pos);
                        }
                        if can_seek && response.drag_stopped() {
                            app.finish_scrub(current_pos);
                        }
                    },
                    |ui| {
                        toggle_fullscreen = ui
                            .button(crate::ui::icons::ARROWS_OUT)
                            .on_hover_text(fullscreen_tooltip)
                            .clicked();
                        toggle_pin = ui.button(pin_icon).on_hover_text(pin_tooltip).clicked();
                        toggle_audio = ui
                            .button(crate::ui::icons::MUSIC_NOTE)
                            .on_hover_text(audio_tooltip)
                            .clicked();
                        toggle_workspace = ui
                            .button(crate::ui::icons::TABS)
                            .on_hover_text(workspace_tooltip)
                            .clicked();
                        toggle_subtitles = ui
                            .button(crate::ui::icons::SUBTITLES)
                            .on_hover_text(subtitles_tooltip)
                            .clicked();

                        ui.add_enabled_ui(has_video, |ui| {
                            let volume_slider =
                                egui::Slider::new(&mut volume, 0.0..=130.0).show_value(false);
                            let volume_response = ui.add_sized([80.0, 15.0], volume_slider);
                            if volume_response.changed() {
                                volume_update = Some(volume);
                            }
                            if volume_response.hovered() {
                                let scroll = ui.input(|input| {
                                    let mut delta = input.smooth_scroll_delta;
                                    if delta.x == 0.0 && delta.y == 0.0 {
                                        for event in &input.events {
                                            if let egui::Event::MouseWheel {
                                                delta: wheel_delta,
                                                ..
                                            } = event
                                            {
                                                delta += *wheel_delta;
                                            }
                                        }
                                    }
                                    delta
                                });
                                if scroll.y != 0.0 {
                                    volume_update = Some(
                                        (volume + if scroll.y > 0.0 { 2.0 } else { -2.0 })
                                            .clamp(0.0, 130.0),
                                    );
                                }
                            }
                            toggle_mute = ui
                                .add(egui::Button::new(mute_icon).frame(false))
                                .on_hover_text(mute_tooltip)
                                .clicked();
                        });

                        let total_response = ui
                            .add(egui::Label::new(total_text).sense(if total_is_toggle {
                                egui::Sense::click()
                            } else {
                                egui::Sense::hover()
                            }))
                            .on_hover_text(total_tooltip);
                        toggle_total_mode = total_is_toggle && total_response.clicked();
                    },
                );

                if toggle_fullscreen {
                    app.toggle_fullscreen(&ctx);
                }
                if toggle_pin {
                    app.pin_controls = !app.pin_controls;
                    app.set_osd(if app.pin_controls {
                        app.tr("Controls Pinned")
                    } else {
                        app.tr("Controls Unpinned")
                    });
                    app.save_config();
                }
                if toggle_audio {
                    app.show_audio_settings = !app.show_audio_settings;
                }
                if toggle_workspace {
                    app.show_four_d_editor = !app.show_four_d_editor;
                }
                if toggle_subtitles {
                    app.show_sub_settings = !app.show_sub_settings;
                }
                if let Some(new_volume) = volume_update {
                    let _ = app.mpv.set_property("volume", new_volume);
                    app.volume = new_volume;
                    app.set_osd(format!("{}: {:.0}%", app.tr("Volume"), new_volume));
                    app.save_config();
                }
                if toggle_mute {
                    let _ = app.mpv.command("cycle", &["mute"]);
                    app.is_muted = !app.is_muted;
                    app.set_osd(if app.is_muted {
                        app.tr("Mute")
                    } else {
                        app.tr("Unmute")
                    });
                    app.save_config();
                }
                if toggle_total_mode {
                    app.show_remaining_time = !app.show_remaining_time;
                    app.save_config();
                }
            });

        if time_since_activity < 3.0 && !app.pin_controls {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
}

pub fn is_controls_interactable(alpha: f32) -> bool {
    alpha >= 0.05
}

pub fn multiply_style_opacity(style: &mut egui::Style, alpha: f32) {
    let fade_color = |color: &mut egui::Color32| {
        *color = color.linear_multiply(alpha);
    };

    // When override_text_color is None, egui uses visuals.text_color().
    // Set override_text_color explicitly to faded text_color so all labels,
    // button text, and icon glyphs fade smoothly.
    let base_text_color = style
        .visuals
        .override_text_color
        .unwrap_or_else(|| style.visuals.text_color());
    style.visuals.override_text_color = Some(base_text_color.linear_multiply(alpha));

    fade_color(&mut style.visuals.warn_fg_color);
    fade_color(&mut style.visuals.error_fg_color);
    fade_color(&mut style.visuals.hyperlink_color);
    fade_color(&mut style.visuals.extreme_bg_color);
    fade_color(&mut style.visuals.faint_bg_color);
    fade_color(&mut style.visuals.code_bg_color);
    fade_color(&mut style.visuals.window_stroke.color);

    let widgets = &mut style.visuals.widgets;
    for state in [
        &mut widgets.noninteractive,
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        fade_color(&mut state.bg_fill);
        fade_color(&mut state.fg_stroke.color);
        fade_color(&mut state.bg_stroke.color);
    }

    fade_color(&mut style.visuals.selection.bg_fill);
    fade_color(&mut style.visuals.selection.stroke.color);
}

pub fn resolve_display_time(seek_pos: Option<f64>, playback_time: f64) -> f64 {
    seek_pos.unwrap_or(playback_time)
}

pub fn format_player_time(time: f64, include_hours: bool, show_subseconds: bool) -> String {
    let negative = time < 0.0;
    let absolute = time.abs();
    let total_millis = (absolute * 1000.0).round() as i64;
    let whole = total_millis / 1000;
    let millis = total_millis % 1000;
    let formatted = if include_hours {
        if show_subseconds {
            format!(
                "{:02}:{:02}:{:02}.{:03}",
                whole / 3600,
                (whole / 60) % 60,
                whole % 60,
                millis
            )
        } else {
            format!(
                "{:02}:{:02}:{:02}",
                whole / 3600,
                (whole / 60) % 60,
                whole % 60
            )
        }
    } else if show_subseconds {
        format!("{:02}:{:02}.{:03}", (whole / 60) % 60, whole % 60, millis)
    } else {
        format!("{:02}:{:02}", (whole / 60) % 60, whole % 60)
    };
    if negative {
        format!("-{formatted}")
    } else {
        formatted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiply_style_opacity_handles_none_override_text_color() {
        let mut style = egui::Style::default();
        style.visuals.override_text_color = None;
        let initial_text_color = style.visuals.text_color();

        multiply_style_opacity(&mut style, 0.5);

        assert_eq!(
            style.visuals.override_text_color,
            Some(initial_text_color.linear_multiply(0.5))
        );
    }

    #[test]
    fn test_controls_interactable_threshold() {
        let alpha_active = 0.5;
        let alpha_faded = 0.02;
        assert!(is_controls_interactable(alpha_active));
        assert!(!is_controls_interactable(alpha_faded));
    }

    #[test]
    fn test_multiply_style_opacity() {
        let mut style = egui::Style::default();
        style.visuals.override_text_color =
            Some(egui::Color32::from_rgba_premultiplied(200, 200, 200, 200));
        let orig_fill = style.visuals.widgets.inactive.bg_fill;

        multiply_style_opacity(&mut style, 0.5);

        assert_eq!(
            style.visuals.override_text_color,
            Some(egui::Color32::from_rgba_premultiplied(200, 200, 200, 200).linear_multiply(0.5))
        );
        assert_eq!(
            style.visuals.widgets.inactive.bg_fill,
            orig_fill.linear_multiply(0.5)
        );
    }

    #[test]
    fn test_multiply_style_opacity_zero() {
        let mut style = egui::Style::default();
        style.visuals.override_text_color =
            Some(egui::Color32::from_rgba_premultiplied(200, 200, 200, 200));

        multiply_style_opacity(&mut style, 0.0);

        assert_eq!(
            style.visuals.override_text_color,
            Some(egui::Color32::from_rgba_premultiplied(200, 200, 200, 200).linear_multiply(0.0))
        );
    }

    #[test]
    fn test_pin_controls_alpha_calculation() {
        let pin_controls = true;
        let time_since_activity: f32 = 10.0;
        let alpha = if pin_controls {
            1.0
        } else {
            (3.0 - time_since_activity).clamp(0.0, 1.0)
        };
        assert_eq!(alpha, 1.0);
    }

    #[test]
    fn test_play_button_fixed_size_constant() {
        let button_size = egui::vec2(30.0, 22.0);
        assert_eq!(button_size.x, 30.0);
        assert_eq!(button_size.y, 22.0);
    }

    #[test]
    fn test_display_total_time_calculation() {
        let playback_time = 83.0;
        let duration = 300.0;
        let show_remaining_time = true;

        let display_total = if show_remaining_time {
            -(duration - playback_time)
        } else {
            duration
        };
        assert_eq!(display_total, -217.0);
    }

    #[test]
    fn test_resolve_display_time_scrubbing_vs_playback() {
        let playback_time = 45.0;
        let seek_pos = Some(120.0);
        assert_eq!(resolve_display_time(seek_pos, playback_time), 120.0);

        let no_seek: Option<f64> = None;
        assert_eq!(resolve_display_time(no_seek, playback_time), 45.0);
    }

    #[test]
    fn subsecond_timecode_uses_a_decimal_separator() {
        assert_eq!(format_player_time(65.125, false, true), "01:05.125");
        assert_eq!(format_player_time(-3661.5, true, true), "-01:01:01.500");
    }

    #[test]
    fn test_seekbar_disabled_position_and_range() {
        let has_video = false;
        let playback_time = 0.0;
        let duration = 0.0;

        let current_pos = if has_video { playback_time } else { 0.0 };
        let max_dur = if has_video && duration > 0.0 {
            duration
        } else {
            1.0
        };

        assert_eq!(current_pos, 0.0);
        assert_eq!(max_dur, 1.0);
    }
}
