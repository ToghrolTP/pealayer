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
                ui.horizontal(|ui| {
                    let play_icon = if app.is_paused { "▶" } else { "⏸" };
                    if ui.button(play_icon).clicked() {
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

                    // Calculate available width for the seekbar, leaving space for the right controls
                    let right_controls_width = 250.0;
                    let seekbar_width = ui.available_width() - right_controls_width;

                    let mut current_pos = app.seek_pos.unwrap_or(app.playback_time);
                    let slider = egui::Slider::new(&mut current_pos, 0.0..=app.duration)
                        .show_value(false)
                        .trailing_fill(true);

                    let old_width = ui.spacing().slider_width;
                    ui.spacing_mut().slider_width = seekbar_width.max(50.0);
                    let response = ui.add(slider);
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

                        if ui.button("🎬").clicked() {
                            app.show_four_d_editor = !app.show_four_d_editor;
                        }

                        if ui.button("💬").clicked() {
                            app.show_sub_settings = !app.show_sub_settings;
                        }

                        let mut vol = app.volume;
                        let vol_slider = egui::Slider::new(&mut vol, 0.0..=130.0).show_value(false);
                        if ui.add_sized([80.0, 15.0], vol_slider).changed() {
                            let _ = app.mpv.set_property("volume", vol);
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
