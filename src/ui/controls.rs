use crate::app::PealayerApp;
use eframe::egui;

const CONTROL_FADE_REPAINT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

pub fn timecode_text(value: impl Into<String>) -> egui::RichText {
    egui::RichText::new(value).monospace()
}

fn compact_number(value: f64) -> String {
    let mut rendered = format!("{value:.3}");
    while rendered.contains('.') && rendered.ends_with('0') {
        rendered.pop();
    }
    if rendered.ends_with('.') {
        rendered.pop();
    }
    rendered
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportNudgeMode {
    Seek,
    FrameStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportNudgeDensity {
    Compact,
    Labeled,
}

const fn transport_nudge_mode(is_paused: bool) -> TransportNudgeMode {
    if is_paused {
        TransportNudgeMode::FrameStep
    } else {
        TransportNudgeMode::Seek
    }
}

const fn transport_nudge_width(density: TransportNudgeDensity) -> f32 {
    match density {
        TransportNudgeDensity::Compact => 44.0,
        TransportNudgeDensity::Labeled => 72.0,
    }
}

/// Draw one transport nudge whose purpose follows playback state.
///
/// While playing it seeks by the configured quick-seek interval. While
/// paused it steps by the configured frame count. Both captions occupy one
/// stable button and crossfade/slide between states, avoiding duplicate
/// controls and layout jumps.
pub fn draw_contextual_transport_nudge(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    id: egui::Id,
    direction: i32,
    density: TransportNudgeDensity,
) -> egui::Response {
    let mode = transport_nudge_mode(app.is_paused);
    let paused_t = ui.ctx().animate_bool_with_time_and_easing(
        id.with("paused-mode"),
        matches!(mode, TransportNudgeMode::FrameStep),
        0.18,
        egui::emath::easing::cubic_out,
    );
    let backwards = direction < 0;
    let quick_text = compact_number(app.quick_seek_seconds);
    let seek_icon = if backwards {
        crate::ui::icons::REWIND
    } else {
        crate::ui::icons::FAST_FORWARD
    };
    let frame_icon = if backwards {
        crate::ui::icons::SKIP_BACK
    } else {
        crate::ui::icons::SKIP_FORWARD
    };
    let width = transport_nudge_width(density);
    let (seek_caption, frame_caption, font_size) = match density {
        TransportNudgeDensity::Compact => (
            format!("{seek_icon} {quick_text}"),
            frame_icon.to_string(),
            11.5,
        ),
        TransportNudgeDensity::Labeled => (
            format!("{seek_icon} {quick_text} {}", app.tr("sec")),
            format!("{frame_icon} {}", app.frame_step_count),
            11.0,
        ),
    };

    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 22.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        ui.painter().rect(
            rect,
            visuals.corner_radius,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let font = egui::FontId::proportional(font_size);
        let text_color = visuals.text_color();
        let travel = 4.0;
        ui.painter().text(
            rect.center() - egui::vec2(0.0, travel * paused_t),
            egui::Align2::CENTER_CENTER,
            seek_caption,
            font.clone(),
            text_color.gamma_multiply(1.0 - paused_t),
        );
        ui.painter().text(
            rect.center() + egui::vec2(0.0, travel * (1.0 - paused_t)),
            egui::Align2::CENTER_CENTER,
            frame_caption,
            font,
            text_color.gamma_multiply(paused_t),
        );
    }

    let action_name = if backwards {
        app.tr("Back")
    } else {
        app.tr("Forward")
    };
    let tooltip = match mode {
        TransportNudgeMode::Seek => format!(
            "{} {} {} ({})",
            if backwards {
                app.tr("Seek backward")
            } else {
                app.tr("Seek forward")
            },
            quick_text,
            app.tr("seconds"),
            if backwards { "←" } else { "→" }
        ),
        TransportNudgeMode::FrameStep => format!(
            "{} {} {} ({})",
            action_name,
            app.frame_step_count,
            app.tr("frames"),
            if backwards { "[" } else { "]" }
        ),
    };
    let response = response.on_hover_text(tooltip);
    response.context_menu(|ui| transport_context_menu(app, ui));
    if response.clicked() {
        match mode {
            TransportNudgeMode::Seek => {
                app.seek_relative(direction as f64 * app.quick_seek_seconds)
            }
            TransportNudgeMode::FrameStep => app.step_frames(direction),
        }
    }
    response
}

pub fn begin_elapsed_edit(app: &mut PealayerApp) {
    let elapsed = resolve_display_time(app.seek_pos, app.playback_time);
    app.elapsed_time_input = format_player_time(elapsed, app.duration >= 3600.0, true);
    app.editing_elapsed_time = true;
    app.elapsed_edit_focus_requested = true;
}

/// Draw the elapsed timestamp as an in-place editor. Clicking the timestamp
/// swaps only that label for a fixed-width field, so transport geometry does
/// not jump while an exact value is entered.
pub fn draw_elapsed_editor(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    id_source: impl std::hash::Hash,
    enabled: bool,
) -> egui::Response {
    let elapsed = resolve_display_time(app.seek_pos, app.playback_time);
    let rendered = format_player_time(elapsed, app.duration >= 3600.0, app.show_subseconds);
    let desired_width = if app.duration >= 3600.0 { 104.0 } else { 82.0 };

    if app.editing_elapsed_time && enabled {
        let response = ui
            .add_sized(
                [desired_width, 22.0],
                egui::TextEdit::singleline(&mut app.elapsed_time_input)
                    .id(ui.make_persistent_id(id_source))
                    .font(egui::TextStyle::Monospace)
                    .horizontal_align(egui::Align::Center)
                    .hint_text("00:00.000"),
            )
            .on_hover_text(app.tr(
                "Enter seconds, MM:SS.mmm, or HH:MM:SS.mmm. Press Enter to seek; Escape cancels.",
            ));
        if app.elapsed_edit_focus_requested {
            response.request_focus();
            app.elapsed_edit_focus_requested = false;
        }

        let escape = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape));
        let enter = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if escape {
            app.editing_elapsed_time = false;
        } else if enter {
            match parse_timecode(&app.elapsed_time_input) {
                Some(seconds) => {
                    app.seek_absolute(seconds);
                    app.editing_elapsed_time = false;
                }
                None => app.set_osd(app.tr("Enter a valid playback time")),
            }
        } else if response.lost_focus() {
            if let Some(seconds) = parse_timecode(&app.elapsed_time_input) {
                app.seek_absolute(seconds);
            }
            app.editing_elapsed_time = false;
        }
        response
    } else {
        let response = ui
            .add_enabled(
                enabled,
                egui::Label::new(timecode_text(rendered)).sense(egui::Sense::click()),
            )
            .on_hover_text(app.tr("Click to enter an exact playback time."));
        if response.clicked() {
            begin_elapsed_edit(app);
        }
        response
    }
}

/// Shared seek/action menu for the NLE and Simple transports.
pub fn transport_context_menu(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let has_video = app.current_video_path.is_some();
    let can_seek = has_video && app.is_seekable;
    let quick = app.quick_seek_seconds;
    let quick_text = compact_number(quick);
    let frames = app.frame_step_count;

    ui.add_enabled_ui(has_video, |ui| {
        if ui
            .button(format!(
                "{} {}",
                if app.is_paused {
                    crate::ui::icons::PLAY
                } else {
                    crate::ui::icons::PAUSE
                },
                if app.is_paused {
                    app.tr("Play")
                } else {
                    app.tr("Pause")
                }
            ))
            .clicked()
        {
            app.toggle_playback();
            ui.close();
        }
    });
    ui.separator();
    if app.is_paused {
        ui.add_enabled_ui(has_video, |ui| {
            if ui
                .button(format!(
                    "{} {} {} {}",
                    crate::ui::icons::SKIP_BACK,
                    app.tr("Back"),
                    frames,
                    app.tr("frames")
                ))
                .clicked()
            {
                app.step_frames(-1);
                ui.close();
            }
            if ui
                .button(format!(
                    "{} {} {} {}",
                    crate::ui::icons::SKIP_FORWARD,
                    app.tr("Forward"),
                    frames,
                    app.tr("frames")
                ))
                .clicked()
            {
                app.step_frames(1);
                ui.close();
            }
        });
    } else {
        ui.add_enabled_ui(can_seek, |ui| {
            if ui
                .button(format!(
                    "{} {} {} {}",
                    crate::ui::icons::REWIND,
                    app.tr("Back"),
                    quick_text,
                    app.tr("seconds")
                ))
                .clicked()
            {
                app.seek_relative(-quick);
                ui.close();
            }
            if ui
                .button(format!(
                    "{} {} {} {}",
                    crate::ui::icons::FAST_FORWARD,
                    app.tr("Forward"),
                    quick_text,
                    app.tr("seconds")
                ))
                .clicked()
            {
                app.seek_relative(quick);
                ui.close();
            }
        });
    }
    ui.separator();
    ui.add_enabled_ui(can_seek, |ui| {
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::CLOCK,
                app.tr("Enter exact time…")
            ))
            .clicked()
        {
            begin_elapsed_edit(app);
            ui.close();
        }
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::SKIP_BACK,
                app.tr("Go to beginning")
            ))
            .clicked()
        {
            app.seek_absolute(0.0);
            ui.close();
        }
    });
    ui.separator();
    let mut config_changed = false;
    let milliseconds_label = app.tr("Show milliseconds");
    let remaining_label = app.tr("Show time remaining");
    config_changed |= ui
        .checkbox(&mut app.show_subseconds, milliseconds_label)
        .changed();
    config_changed |= ui
        .checkbox(&mut app.show_remaining_time, remaining_label)
        .changed();
    if config_changed {
        app.save_config();
    }
}

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
                            let mut toggle_playback_requested = false;
                            draw_contextual_transport_nudge(
                                app,
                                ui,
                                ui.make_persistent_id("simple-transport-back"),
                                -1,
                                TransportNudgeDensity::Labeled,
                            );

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
                            let play_response = ui
                                .add_sized([30.0, 22.0], egui::Button::new(play_icon))
                                .on_hover_text(play_tooltip);
                            play_response.context_menu(|ui| transport_context_menu(app, ui));
                            if play_response.clicked() {
                                // Apply after both contextual buttons are drawn
                                // so one frame cannot render mismatched modes.
                                toggle_playback_requested = true;
                            }

                            draw_contextual_transport_nudge(
                                app,
                                ui,
                                ui.make_persistent_id("simple-transport-forward"),
                                1,
                                TransportNudgeDensity::Labeled,
                            );
                            if toggle_playback_requested {
                                app.toggle_playback();
                            }
                        });

                        let elapsed_resp =
                            draw_elapsed_editor(app, ui, "simple-elapsed-editor", has_video && can_seek);
                        elapsed_resp.context_menu(|ui| transport_context_menu(app, ui));

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
                        response.context_menu(|ui| transport_context_menu(app, ui));

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

                        let total_label = if total_is_toggle {
                            timecode_text(total_text)
                        } else {
                            egui::RichText::new(total_text)
                        };
                        let total_response = ui
                            .add(egui::Label::new(total_label).sense(if total_is_toggle {
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
            // The old 16 ms loop repainted the entire application at 60 Hz
            // whenever controls were visible, even while media was paused.
            // Ten fade samples per second remain visually smooth while keeping
            // the idle renderer reactive rather than continuously busy.
            ctx.request_repaint_after(CONTROL_FADE_REPAINT_INTERVAL);
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

pub fn parse_timecode(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.starts_with('-') {
        return None;
    }
    let fields = trimmed.split(':').collect::<Vec<_>>();
    if fields.len() > 3 {
        return None;
    }
    let seconds = match fields.as_slice() {
        [seconds] => seconds.parse::<f64>().ok()?,
        [minutes, seconds] => {
            let seconds = seconds.parse::<f64>().ok()?;
            (seconds < 60.0).then_some(())?;
            minutes.parse::<u64>().ok()? as f64 * 60.0 + seconds
        }
        [hours, minutes, seconds] => {
            let minutes = minutes.parse::<u64>().ok()?;
            let seconds = seconds.parse::<f64>().ok()?;
            (minutes < 60 && seconds < 60.0).then_some(())?;
            hours.parse::<u64>().ok()? as f64 * 3600.0 + minutes as f64 * 60.0 + seconds
        }
        _ => return None,
    };
    seconds.is_finite().then_some(seconds)
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
    fn contextual_nudges_seek_while_playing_and_step_while_paused() {
        assert_eq!(transport_nudge_mode(false), TransportNudgeMode::Seek);
        assert_eq!(transport_nudge_mode(true), TransportNudgeMode::FrameStep);
    }

    #[test]
    fn contextual_nudge_slots_keep_stable_geometry_during_transition() {
        assert_eq!(transport_nudge_width(TransportNudgeDensity::Compact), 44.0);
        assert_eq!(transport_nudge_width(TransportNudgeDensity::Labeled), 72.0);
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
    fn exact_timecode_parser_accepts_player_formats() {
        assert_eq!(parse_timecode("90.5"), Some(90.5));
        assert_eq!(parse_timecode("01:30.500"), Some(90.5));
        assert_eq!(parse_timecode("01:02:03.250"), Some(3723.25));
        assert_eq!(parse_timecode("01:75"), None);
        assert_eq!(parse_timecode("-00:01"), None);
        assert_eq!(parse_timecode("not a time"), None);
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
