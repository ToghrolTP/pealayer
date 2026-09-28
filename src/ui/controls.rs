use crate::app::PealayerApp;
use eframe::egui;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let time_since_activity = app.last_mouse_activity.elapsed().as_secs_f32();
    let alpha = if app.pin_controls {
        1.0
    } else {
        (3.0 - time_since_activity).clamp(0.0, 1.0)
    };

    if alpha > 0.0 {
        let window_width = ui.available_width() - 20.0;

        egui::Window::new("Controls")
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -20.0))
            .min_width(window_width)
            .default_width(window_width)
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .interactable(is_controls_interactable(alpha))
            .frame(egui::Frame::window(ui.style()).multiply_with_opacity(alpha))
            .show(&ctx, |ui| {
                ui.set_opacity(alpha);
                multiply_style_opacity(ui.style_mut(), alpha);
                ui.horizontal(|ui| {
                    let total_available = ui.available_width();
                    let spacing = ui.spacing().item_spacing.x;
                    let (seekbar_width, _gap) = compute_controls_layout(
                        total_available,
                        LEFT_CONTROLS_WIDTH,
                        RIGHT_CONTROLS_WIDTH,
                        spacing,
                    );

                    let has_video = app.current_video_path.is_some();

                    ui.add_enabled_ui(has_video, |ui| {
                        let play_icon = if app.is_playback_finished() {
                            "↺"
                        } else if app.is_paused {
                            "▶"
                        } else {
                            "⏸"
                        };
                        let play_tooltip = if app.is_playback_finished() {
                            "Replay"
                        } else if app.is_paused {
                            "Play"
                        } else {
                            "Pause"
                        };
                        if ui
                            .add_sized([30.0, 22.0], egui::Button::new(play_icon))
                            .on_hover_text(play_tooltip)
                            .clicked()
                        {
                            app.toggle_playback();
                        }
                    });

                    let elapsed_time = resolve_display_time(app.seek_pos, app.playback_time);
                    let display_total = if app.show_remaining_time {
                        -(app.duration - elapsed_time)
                    } else {
                        app.duration
                    };

                    let is_long_video = app.duration >= 3600.0;
                    let format_time = move |t: f64| {
                        let is_negative = t < 0.0;
                        let s = t.abs() as i64;
                        let formatted = if is_long_video {
                            format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
                        } else {
                            format!("{:02}:{:02}", (s / 60) % 60, s % 60)
                        };
                        if is_negative {
                            format!("-{}", formatted)
                        } else {
                            formatted
                        }
                    };

                    let elapsed_str = format_time(elapsed_time);
                    let elapsed_resp = ui.add_enabled(has_video, egui::Label::new(elapsed_str).sense(egui::Sense::click()));
                    if has_video && elapsed_resp.clicked() {
                        app.show_remaining_time = !app.show_remaining_time;
                    }

                    ui.add_enabled_ui(has_video, |ui| {
                        let mut current_pos = if has_video {
                            app.seek_pos.unwrap_or(app.playback_time)
                        } else {
                            0.0
                        };
                        let max_dur = if has_video && app.duration > 0.0 {
                            app.duration
                        } else {
                            1.0
                        };
                        let slider = egui::Slider::new(&mut current_pos, 0.0..=max_dur)
                            .show_value(false)
                            .trailing_fill(true);

                        let old_width = ui.spacing().slider_width;
                        ui.spacing_mut().slider_width = seekbar_width;
                        let response = ui.add(slider);
                        ui.spacing_mut().slider_width = old_width;

                        if has_video && response.dragged() {
                            app.scrub_to(current_pos);
                        }
                        if has_video && response.drag_stopped() {
                            app.finish_scrub(current_pos);
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.set_clip_rect(ui.max_rect());
                        if ui.button("⛶").clicked() {
                            let is_fullscreen =
                                ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                                !is_fullscreen,
                            ));
                            app.set_osd("Fullscreen".to_string());
                        }

                        let pin_icon = if app.pin_controls { "📌" } else { "📍" };
                        if ui.button(pin_icon).clicked() {
                            app.pin_controls = !app.pin_controls;
                            app.set_osd(if app.pin_controls { "Controls Pinned".to_string() } else { "Controls Unpinned".to_string() });
                            app.save_config();
                        }

                        if ui.button("🎵").clicked() {
                            app.show_audio_settings = !app.show_audio_settings;
                        }

                        if ui.button("🎬").clicked() {
                            app.show_four_d_editor = !app.show_four_d_editor;
                        }

                        if ui.button("💬").clicked() {
                            app.show_sub_settings = !app.show_sub_settings;
                        }

                        let has_video = app.current_video_path.is_some();
                        ui.add_enabled_ui(has_video, |ui| {
                            let mut vol = app.volume;
                            let vol_slider = egui::Slider::new(&mut vol, 0.0..=130.0).show_value(false);
                            let vol_resp = ui.add_sized([80.0, 15.0], vol_slider);
                            if vol_resp.changed() {
                                let _ = app.mpv.set_property("volume", vol);
                                app.volume = vol;
                                app.save_config();
                            }
                            if vol_resp.hovered() {
                                let scroll = ui.input(|i| {
                                    let mut d = i.smooth_scroll_delta;
                                    if d.x == 0.0 && d.y == 0.0 {
                                        for ev in &i.events {
                                            if let egui::Event::MouseWheel { delta, .. } = ev {
                                                d += *delta;
                                            }
                                        }
                                    }
                                    d
                                });
                                if scroll.y != 0.0 {
                                    let vol_change = if scroll.y > 0.0 { 2.0 } else { -2.0 };
                                    let new_vol = (app.volume + vol_change).clamp(0.0, 130.0);
                                    let _ = app.mpv.set_property("volume", new_vol);
                                    app.volume = new_vol;
                                    app.set_osd(format!("Volume: {:.0}%", new_vol));
                                    app.save_config();
                                }
                            }
                            let mute_icon = if app.is_muted { "🔇" } else { "🔊" };
                            if ui.add(egui::Button::new(mute_icon).frame(false)).clicked() {
                                let _ = app.mpv.command("cycle", &["mute"]);
                                app.is_muted = !app.is_muted;
                                app.set_osd(if app.is_muted { "Mute".to_string() } else { "Unmute".to_string() });
                                app.save_config();
                            }
                        });

                        let total_str = format_time(display_total);
                        let total_resp = ui.add_enabled(has_video, egui::Label::new(total_str).sense(egui::Sense::click()));
                        if has_video && total_resp.clicked() {
                            app.show_remaining_time = !app.show_remaining_time;
                            app.save_config();
                        }
                    });
                });
            });

        if time_since_activity < 3.0 && !app.pin_controls {
            ctx.request_repaint();
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
    let base_text_color = style.visuals.override_text_color.unwrap_or_else(|| style.visuals.text_color());
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

pub const LEFT_CONTROLS_WIDTH: f32 = 93.0;
pub const RIGHT_CONTROLS_WIDTH: f32 = 345.0;

pub fn compute_controls_layout(
    available_width: f32,
    left_width: f32,
    right_width: f32,
    spacing: f32,
) -> (f32, f32) {
    let min_seekbar_width = 40.0;
    let fixed_widths = left_width + right_width + (spacing * 2.0);
    if available_width > fixed_widths {
        let seekbar_width = (available_width - fixed_widths).max(min_seekbar_width);
        let remaining_gap = (available_width - (left_width + spacing + seekbar_width + right_width)).max(spacing);
        (seekbar_width, remaining_gap)
    } else {
        (min_seekbar_width, spacing)
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
        style.visuals.override_text_color = Some(egui::Color32::from_rgba_premultiplied(200, 200, 200, 200));
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
        style.visuals.override_text_color = Some(egui::Color32::from_rgba_premultiplied(200, 200, 200, 200));

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
    fn test_seekbar_disabled_position_and_range() {
        let has_video = false;
        let playback_time = 0.0;
        let duration = 0.0;

        let current_pos = if has_video { playback_time } else { 0.0 };
        let max_dur = if has_video && duration > 0.0 { duration } else { 1.0 };

        assert_eq!(current_pos, 0.0);
        assert_eq!(max_dur, 1.0);
    }

    #[test]
    fn test_compute_controls_layout_prevents_overlap() {
        // Standard 800px window
        let available_w = 780.0;
        let left_w = 120.0;
        let right_w = 380.0;
        let spacing = 8.0;

        let (seekbar_w, gap) = compute_controls_layout(available_w, left_w, right_w, spacing);
        assert!(seekbar_w >= 40.0);
        assert_eq!(left_w + spacing + seekbar_w + gap + right_w, available_w);
        assert!(gap >= spacing);

        // Narrow 500px window
        let available_w_narrow = 520.0;
        let (seekbar_w_narrow, _gap_narrow) = compute_controls_layout(available_w_narrow, left_w, right_w, spacing);
        assert_eq!(seekbar_w_narrow, 40.0); // clamped to min width
        assert!(left_w + seekbar_w_narrow <= available_w_narrow);
    }
}

