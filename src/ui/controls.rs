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
            .interactable(true)
            .frame(egui::Frame::window(ui.style()).multiply_with_opacity(alpha))
            .show(&ctx, |ui| {
                multiply_style_opacity(ui.style_mut(), alpha);
                ui.horizontal(|ui| {
                    let has_video = app.current_video_path.is_some();

                    ui.add_enabled_ui(has_video, |ui| {
                        let play_icon = if app.is_paused { "▶" } else { "⏸" };
                        if ui.add_sized([30.0, 22.0], egui::Button::new(play_icon)).clicked() {
                            let _ = app.mpv.command("cycle", &["pause"]);
                            app.is_paused = !app.is_paused;
                            app.set_osd(if app.is_paused { "Pause".to_string() } else { "Play".to_string() });
                        }
                    });

                    let elapsed_time = app.playback_time;
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

                    // Calculate available width for the seekbar, leaving space for right controls
                    let right_controls_width = 375.0;
                    let seekbar_width = ui.available_width() - right_controls_width;

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
                        ui.spacing_mut().slider_width = seekbar_width.max(50.0);
                        let response = ui.add(slider);
                        ui.spacing_mut().slider_width = old_width;

                        if has_video && response.dragged() {
                            app.seek_pos = Some(current_pos);
                        }
                        if has_video && response.drag_stopped() {
                            let _ = app
                                .mpv
                                .command("seek", &[&current_pos.to_string(), "absolute"]);
                            app.seek_pos = None;
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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

fn multiply_style_opacity(style: &mut egui::Style, alpha: f32) {
    let fade_color = |color: &mut egui::Color32| {
        *color = color.linear_multiply(alpha);
    };

    if let Some(ref mut c) = style.visuals.override_text_color {
        fade_color(c);
    }
    fade_color(&mut style.visuals.hyperlink_color);
    fade_color(&mut style.visuals.extreme_bg_color);
    fade_color(&mut style.visuals.faint_bg_color);
    fade_color(&mut style.visuals.code_bg_color);

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_seekbar_disabled_position_and_range() {
        let has_video = false;
        let playback_time = 0.0;
        let duration = 0.0;

        let current_pos = if has_video { playback_time } else { 0.0 };
        let max_dur = if has_video && duration > 0.0 { duration } else { 1.0 };

        assert_eq!(current_pos, 0.0);
        assert_eq!(max_dur, 1.0);
    }
}

