use crate::app::PealayerApp;
use eframe::egui;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let time_since_activity = app.last_mouse_activity.elapsed().as_secs_f32();
    let alpha = (3.0 - time_since_activity).clamp(0.0, 1.0);

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
                // Fade the content (text, buttons, slider) in sync with the window frame.
                ui.set_opacity(alpha);

                ui.horizontal(|ui| {
                    // Fixed-width play/pause button so the UI does not shift when the symbol changes.
                    let play_icon = if app.is_paused { "▶" } else { "⏸" };
                    if ui
                        .add(egui::Button::new(play_icon).min_size(egui::vec2(28.0, 0.0)))
                        .clicked()
                    {
                        let _ = app.mpv.command("cycle", &["pause"]);
                    }

                    let format_time = |t: f64| {
                        let s = t as i64;
                        if app.duration >= 3600.0 {
                            format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
                        } else {
                            format!("{:02}:{:02}", (s / 60) % 60, s % 60)
                        }
                    };

                    ui.label(format!(
                        "{} / {}",
                        format_time(app.playback_time),
                        format_time(app.duration)
                    ));

                    // Reserve enough space for: mute icon + volume slider + sub + audio + fullscreen buttons.
                    // Measured conservatively to prevent the seekbar from running into the mute button.
                    let right_controls_width = 240.0;
                    let seekbar_width = ui.available_width() - right_controls_width;

                    // When no video is loaded use a dummy range so the handle sits at the left edge
                    // (value 0 on 0..=1) rather than at the midpoint of a degenerate 0..=0 range.
                    let (seek_range, mut current_pos) = if app.has_video {
                        (0.0..=app.duration, app.seek_pos.unwrap_or(app.playback_time))
                    } else {
                        (0.0..=1.0, 0.0)
                    };

                    let slider = egui::Slider::new(&mut current_pos, seek_range)
                        .show_value(false)
                        .trailing_fill(true);

                    let old_width = ui.spacing().slider_width;
                    ui.spacing_mut().slider_width = seekbar_width.max(50.0);
                    // Disable the slider entirely when there is nothing loaded.
                    let response = ui.add_enabled(app.has_video, slider);
                    ui.spacing_mut().slider_width = old_width;

                    if response.dragged() {
                        app.seek_pos = Some(current_pos);
                    }
                    if response.drag_stopped() {
                        let _ = app
                            .mpv
                            .command("seek", &[&current_pos.to_string(), "absolute"]);
                        app.seek_pos = None;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("⛶").clicked() {
                            let is_fullscreen =
                                ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                                !is_fullscreen,
                            ));
                        }

                        if ui.button("🎵").clicked() {
                            app.show_audio_settings = !app.show_audio_settings;
                        }

                        if ui.button("💬").clicked() {
                            app.show_sub_settings = !app.show_sub_settings;
                        }

                        let mut vol = app.volume;
                        let vol_slider =
                            egui::Slider::new(&mut vol, 0.0..=130.0).show_value(false);
                        let vol_response = ui.add_sized([80.0, 15.0], vol_slider);
                        if vol_response.changed() {
                            let _ = app.mpv.set_property("volume", vol);
                        }

                        // Mousewheel on the volume slider area adjusts volume.
                        if vol_response.hovered() {
                            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                            if scroll != 0.0 {
                                let delta = scroll.round() as f64;
                                let _ = app.mpv.command("add", &["volume", &delta.to_string()]);
                                app.show_osd(format!(
                                    "🔊 Volume: {:.0}%",
                                    (app.volume + delta).clamp(0.0, 130.0)
                                ));
                            }
                        }

                        let mute_icon = if app.is_muted { "🔇" } else { "🔊" };
                        if ui.add(egui::Button::new(mute_icon).frame(false)).clicked() {
                            let _ = app.mpv.command("cycle", &["mute"]);
                        }
                    });
                });
            });

        if time_since_activity < 3.0 {
            ctx.request_repaint();
        }
    }
}
