use crate::app::PealayerApp;
use crate::mpv::render::GetProcAddress;
use eframe::egui;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub(crate) struct VideoSurfaceGesture {
    pub(crate) action: crate::config::PlayerDragAction,
    pub(crate) start_time: f64,
    pub(crate) start_rate: f64,
    pub(crate) was_paused: bool,
    pub(crate) target_time: f64,
    pub(crate) started_at: std::time::Instant,
    pub(crate) dragged: bool,
}

fn seek_target_from_drag(start_time: f64, drag_delta_x: f32, duration: f64) -> f64 {
    (start_time + drag_delta_x as f64 / 12.0).clamp(0.0, duration.max(0.0))
}

fn temporary_fast_forward_rate(start_rate: f64, configured_rate: f64, drag_delta_x: f32) -> f64 {
    // A press without movement must still fast-forward. Horizontal movement
    // can then increase the configured temporary rate without changing the
    // persisted normal playback speed. Never slow an already-faster session.
    (configured_rate.max(start_rate) + drag_delta_x.abs() as f64 / 160.0).clamp(1.0, 16.0)
}

fn should_consume_fast_forward_click(
    action: crate::config::PlayerDragAction,
    dragged: bool,
    held_for: std::time::Duration,
) -> bool {
    action == crate::config::PlayerDragAction::TemporaryFastForward
        && (dragged || held_for >= std::time::Duration::from_millis(180))
}

fn begin_video_surface_gesture(app: &mut PealayerApp, action: crate::config::PlayerDragAction) {
    if app.video_surface_gesture.is_some() {
        return;
    }
    let gesture = VideoSurfaceGesture {
        action,
        start_time: app.playback_time,
        start_rate: app.playback_rate,
        was_paused: app.is_paused,
        target_time: app.playback_time,
        started_at: std::time::Instant::now(),
        dragged: false,
    };
    app.video_surface_gesture = Some(gesture);

    if action == crate::config::PlayerDragAction::TemporaryFastForward {
        if gesture.was_paused {
            let _ = app.mpv.set_property("pause", false);
            app.is_paused = false;
            app.engine_handle
                .is_playing
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let speed =
            temporary_fast_forward_rate(gesture.start_rate, app.temporary_fast_forward_speed, 0.0);
        let _ = app.mpv.set_property("speed", speed);
        app.playback_rate = speed;
        app.set_osd(format!("{}: {speed:.1}×", app.tr("Playback speed")));
    }
}

fn finish_video_surface_gesture(app: &mut PealayerApp) -> bool {
    let Some(gesture) = app.video_surface_gesture.take() else {
        return false;
    };
    match gesture.action {
        crate::config::PlayerDragAction::Seek => app.finish_scrub(gesture.target_time),
        crate::config::PlayerDragAction::TemporaryFastForward => {
            let _ = app.mpv.set_property("speed", gesture.start_rate);
            app.playback_rate = gesture.start_rate;
            if gesture.was_paused {
                let _ = app.mpv.set_property("pause", true);
                app.is_paused = true;
                app.engine_handle
                    .is_playing
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
        }
        crate::config::PlayerDragAction::MoveWindow | crate::config::PlayerDragAction::None => {}
    }
    should_consume_fast_forward_click(
        gesture.action,
        gesture.dragged,
        gesture.started_at.elapsed(),
    )
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let video_files_label = app.tr("Video Files");
    let video_size = ui.available_size();
    if video_size.x <= 0.0 || video_size.y <= 0.0 {
        return;
    }

    let (rect, response) = ui.allocate_exact_size(video_size, egui::Sense::click_and_drag());

    let selected_action = if app.is_paused {
        app.paused_drag_action
    } else {
        app.playing_drag_action
    };
    let primary_pressed =
        ui.input(|input| input.pointer.button_pressed(egui::PointerButton::Primary));
    let primary_down = ui.input(|input| input.pointer.primary_down());

    // Temporary fast-forward is a hold gesture, so it starts on mouse-down
    // without requiring the pointer to move far enough to become an egui drag.
    if response.hovered()
        && primary_pressed
        && app.current_video_path.is_some()
        && selected_action == crate::config::PlayerDragAction::TemporaryFastForward
    {
        begin_video_surface_gesture(app, selected_action);
    }

    if response.drag_started() && app.current_video_path.is_some() {
        begin_video_surface_gesture(app, selected_action);
        if selected_action == crate::config::PlayerDragAction::Seek {
            app.scrub_to(app.playback_time);
        }
    }

    if response.dragged()
        && app.current_video_path.is_some()
        && let Some(mut gesture) = app.video_surface_gesture
    {
        gesture.dragged = true;
        app.video_surface_gesture = Some(gesture);
        match gesture.action {
            crate::config::PlayerDragAction::MoveWindow => {
                app.is_window_operating = true;
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            crate::config::PlayerDragAction::Seek => {
                let target = seek_target_from_drag(
                    gesture.start_time,
                    response.drag_delta().x,
                    app.duration,
                );
                if (target - gesture.target_time).abs() >= 0.001 {
                    gesture.target_time = target;
                    app.video_surface_gesture = Some(gesture);
                    app.scrub_to(target);
                }
            }
            crate::config::PlayerDragAction::TemporaryFastForward => {
                let speed = temporary_fast_forward_rate(
                    gesture.start_rate,
                    app.temporary_fast_forward_speed,
                    response.drag_delta().x,
                );
                let _ = app.mpv.set_property("speed", speed);
                app.playback_rate = speed;
                app.set_osd(format!("{}: {speed:.1}×", app.tr("Playback speed")));
            }
            crate::config::PlayerDragAction::None => {}
        }
    }

    let gesture_released =
        response.drag_stopped() || (app.video_surface_gesture.is_some() && !primary_down);
    let suppress_click = if gesture_released {
        finish_video_surface_gesture(app)
    } else {
        app.video_surface_gesture.is_some_and(|gesture| {
            gesture.action == crate::config::PlayerDragAction::TemporaryFastForward
        })
    };

    if response.double_clicked_by(egui::PointerButton::Primary) && app.current_video_path.is_some()
    {
        app.toggle_fullscreen(ui.ctx());
    } else if !suppress_click && response.clicked_by(egui::PointerButton::Primary) {
        if app.current_video_path.is_some() && app.click_player_to_toggle {
            app.toggle_playback();
        } else if app.current_video_path.is_none() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(
                    &video_files_label,
                    &["mp4", "mkv", "avi", "webm", "mov", "flv"],
                )
                .pick_file()
            {
                app.load_video_file(path);
            }
        }
    }

    if response.hovered() {
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
        let adjust_volume = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);

        if adjust_volume && scroll.y != 0.0 {
            let vol_change = if scroll.y > 0.0 { 2.0 } else { -2.0 };
            let new_vol = (app.volume + vol_change).clamp(0.0, 130.0);
            let _ = app.mpv.set_property("volume", new_vol);
            app.volume = new_vol;
            app.set_osd(format!("Volume: {:.0}%", new_vol));
        } else {
            let delta = if scroll.y != 0.0 { -scroll.y } else { scroll.x };
            if delta != 0.0 && app.current_video_path.is_some() {
                let seek_change = if delta > 0.0 {
                    app.wheel_seek_seconds
                } else {
                    -app.wheel_seek_seconds
                };
                app.seek_relative(seek_change);
            }
        }
    }

    response.context_menu(|ui| {
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::PLAY,
                app.tr("Open Video File...")
            ))
            .clicked()
        {
            ui.close();
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(
                    &video_files_label,
                    &["mp4", "mkv", "avi", "webm", "mov", "flv"],
                )
                .pick_file()
            {
                app.load_video_file(path);
            }
        }

        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::ARROW_SQUARE_OUT,
                app.tr("Open Location / URL...")
            ))
            .clicked()
        {
            ui.close();
            app.show_open_url_dialog = true;
        }

        let has_video = app.current_video_path.is_some();
        if ui
            .add_enabled(
                has_video,
                egui::Button::new(format!("{} {}", crate::ui::icons::X, app.tr("Close Video"))),
            )
            .clicked()
        {
            ui.close();
            app.close_video();
        }

        ui.separator();

        let play_title = if app.is_playback_finished() {
            format!(
                "{} {}",
                crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                app.tr("Replay")
            )
        } else if app.is_paused {
            format!("{} {}", crate::ui::icons::PLAY, app.tr("Play"))
        } else {
            format!("{} {}", crate::ui::icons::STOP_CIRCLE, app.tr("Pause"))
        };
        if ui
            .add_enabled(has_video, egui::Button::new(play_title))
            .clicked()
        {
            ui.close();
            app.toggle_playback();
        }

        crate::ui::icons::submenu(
            ui,
            format!("{} {}", crate::ui::icons::GAUGE, app.tr("Playback speed")),
            |ui| {
                for speed in [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0] {
                    let selected = (app.configured_playback_speed - speed).abs() < 0.001;
                    let label = if selected {
                        format!("{} {speed}×", crate::ui::icons::CHECK)
                    } else {
                        format!("   {speed}×")
                    };
                    if ui
                        .add_enabled(has_video, egui::Button::new(label))
                        .clicked()
                    {
                        ui.close();
                        app.set_playback_speed(speed, true);
                    }
                }
            },
        );

        let is_fullscreen = app.fullscreen_intent(ui.ctx());
        let fs_title = if is_fullscreen {
            format!(
                "{} {}",
                crate::ui::icons::ARROWS_OUT,
                app.tr("Exit Fullscreen")
            )
        } else {
            format!("{} {}", crate::ui::icons::ARROWS_OUT, app.tr("Fullscreen"))
        };
        if ui.button(fs_title).clicked() {
            ui.close();
            app.toggle_fullscreen(ui.ctx());
        }

        let mute_title = if app.is_muted {
            format!("{} {}", crate::ui::icons::SPEAKER_HIGH, app.tr("Unmute"))
        } else {
            format!("{} {}", crate::ui::icons::SPEAKER_SLASH, app.tr("Mute"))
        };
        if ui
            .add_enabled(has_video, egui::Button::new(mute_title))
            .clicked()
        {
            ui.close();
            let _ = app.mpv.command("cycle", &["mute"]);
            app.is_muted = !app.is_muted;
            app.set_osd(if app.is_muted {
                app.tr("Mute")
            } else {
                app.tr("Unmute")
            });
        }

        ui.separator();

        crate::ui::icons::submenu(
            ui,
            format!(
                "{} {}",
                crate::ui::icons::CLOCK_COUNTER_CLOCKWISE,
                app.tr("Open Recent")
            ),
            |ui| {
                if app.recent_media.is_empty() {
                    ui.label(app.tr("No recent media"));
                } else {
                    for path in app.recent_media.clone() {
                        let target = path.to_string_lossy();
                        let label = crate::media::media_target_label(&target);
                        if ui
                            .button(app.display_text(&label))
                            .on_hover_text(crate::media::redact_media_target(&target))
                            .clicked()
                        {
                            ui.close();
                            app.load_media_target(&target);
                        }
                    }
                    ui.separator();
                    if ui.button(app.tr("Clear Recent")).clicked() {
                        ui.close();
                        app.clear_recent_media();
                    }
                }
            },
        );

        let pin_title = if app.pin_controls {
            format!(
                "{} {}",
                crate::ui::icons::PUSH_PIN_SLASH,
                app.tr("Unpin Controls")
            )
        } else {
            format!("{} {}", crate::ui::icons::PUSH_PIN, app.tr("Pin Controls"))
        };
        if ui.button(pin_title).clicked() {
            ui.close();
            app.pin_controls = !app.pin_controls;
            app.save_config();
        }
    });

    let is_fullscreen = app.fullscreen_intent(ui.ctx());
    let surface_background = if is_fullscreen {
        match app.fullscreen_video_background {
            crate::config::VideoBackground::Black => egui::Color32::BLACK,
            crate::config::VideoBackground::DarkGray => egui::Color32::from_rgb(18, 18, 20),
            crate::config::VideoBackground::Theme => ui.visuals().panel_fill,
        }
    } else {
        ui.visuals().panel_fill
    };
    ui.painter().rect_filled(rect, 0.0, surface_background);

    // 1. Calculate destination rect maintaining 16:9 aspect ratio
    let aspect_ratio = 16.0 / 9.0;
    let rect_w = rect.width();
    let rect_h = rect.height();

    let dest_rect = if rect_w / rect_h > aspect_ratio {
        // Height-constrained
        let new_w = rect_h * aspect_ratio;
        let x_offset = (rect_w - new_w) / 2.0;
        egui::Rect::from_min_size(
            egui::pos2(rect.min.x + x_offset, rect.min.y),
            egui::vec2(new_w, rect_h),
        )
    } else {
        // Width-constrained
        let new_h = rect_w / aspect_ratio;
        let y_offset = (rect_h - new_h) / 2.0;
        egui::Rect::from_min_size(
            egui::pos2(rect.min.x, rect.min.y + y_offset),
            egui::vec2(rect_w, new_h),
        )
    };

    // 2. Calculate DPI-aware physical pixel dimensions
    let ppi = ui.ctx().pixels_per_point();
    let (target_phys_w, target_phys_h) = calculate_physical_bounds(dest_rect, ppi);

    // Draw the offscreen texture if registered
    let texture_id_opt = app
        .rtt_state
        .try_lock()
        .ok()
        .and_then(|rtt| rtt.video_texture_id);

    if let Some(texture_id) = texture_id_opt {
        ui.painter().image(
            texture_id,
            dest_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    // 2.5 Draw OSD overlay if active
    if let Some((msg, timestamp)) = &app.osd_message {
        let elapsed = timestamp.elapsed().as_secs_f32();
        let timeout = app.osd_timeout_seconds.max(1.0);
        let fade_start = (timeout - 0.6).max(0.4);
        if elapsed < timeout {
            let alpha = if elapsed < fade_start {
                1.0
            } else {
                ((timeout - elapsed) / (timeout - fade_start)).clamp(0.0, 1.0)
            };
            let text_color = egui::Color32::WHITE.linear_multiply(alpha);
            let bg_color = egui::Color32::from_black_alpha((180.0 * alpha) as u8);

            let center = match app.osd_position {
                crate::config::OsdPosition::TopLeft => {
                    dest_rect.left_top() + egui::vec2(24.0, 24.0)
                }
                crate::config::OsdPosition::Center => dest_rect.center(),
            };
            let font_id = egui::FontId::proportional(22.0);
            let osd_text = format!("{}  {}", osd_icon(msg), msg);
            let galley = ui.painter().layout_no_wrap(osd_text, font_id, text_color);
            let rect = match app.osd_position {
                crate::config::OsdPosition::TopLeft => {
                    egui::Rect::from_min_size(center, galley.size() + egui::vec2(24.0, 16.0))
                }
                crate::config::OsdPosition::Center => {
                    egui::Rect::from_center_size(center, galley.size() + egui::vec2(24.0, 16.0))
                }
            };
            ui.painter().rect_filled(rect, 8.0, bg_color);
            ui.painter().galley(
                rect.min + egui::vec2(12.0, 8.0),
                galley,
                egui::Color32::PLACEHOLDER,
            );

            // Keep a precise expiry without forcing the entire application to
            // render at 60 Hz while the message is static. During the short
            // fade, 10 Hz is smooth enough for an overlay and dramatically
            // reduces idle GPU/CPU work on Windows.
            let repaint_after = if elapsed < fade_start {
                std::time::Duration::from_secs_f32((fade_start - elapsed).max(0.01))
            } else {
                std::time::Duration::from_millis(100)
            };
            ui.ctx().request_repaint_after(repaint_after);
        }
    }

    // 3. Schedule PaintCallback to render current frame at exact physical pixel size
    let render_context = app.render_context.clone();
    let rtt_state = app.rtt_state.clone();
    let is_operating = app.is_window_operating;

    let callback = egui::PaintCallback {
        rect,
        callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |_info, painter| {
            if is_operating {
                return;
            }
            let gl = painter.gl();
            if let Ok(mut rtt) = rtt_state.try_lock() {
                if let (Some(video_fbo), Some(tex), Ok(rc_guard)) =
                    (rtt.video_fbo, rtt.video_texture, render_context.try_lock())
                {
                    if let Some(ref rc) = *rc_guard {
                        unsafe {
                            use eframe::glow::HasContext;

                            // Query original FBO binding
                            let raw_fbo =
                                gl.get_parameter_i32(eframe::glow::FRAMEBUFFER_BINDING) as u32;
                            let target_fbo = std::num::NonZeroU32::new(raw_fbo)
                                .map(eframe::glow::NativeFramebuffer);

                            // Query original viewport to restore it later
                            let mut original_viewport = [0; 4];
                            gl.get_parameter_i32_slice(
                                eframe::glow::VIEWPORT,
                                &mut original_viewport,
                            );

                            // Dynamic resizing of texture if physical dimensions changed
                            if rtt.texture_width != target_phys_w as u32
                                || rtt.texture_height != target_phys_h as u32
                            {
                                gl.bind_texture(eframe::glow::TEXTURE_2D, Some(tex));
                                gl.tex_image_2d(
                                    eframe::glow::TEXTURE_2D,
                                    0,
                                    eframe::glow::RGBA8 as i32,
                                    target_phys_w,
                                    target_phys_h,
                                    0,
                                    eframe::glow::RGBA,
                                    eframe::glow::UNSIGNED_BYTE,
                                    eframe::glow::PixelUnpackData::Slice(None),
                                );
                                rtt.texture_width = target_phys_w as u32;
                                rtt.texture_height = target_phys_h as u32;
                            }

                            // Bind our offscreen FBO
                            gl.bind_framebuffer(eframe::glow::FRAMEBUFFER, Some(video_fbo));

                            // Set viewport to exact physical framebuffer size
                            gl.viewport(0, 0, target_phys_w, target_phys_h);

                            // Render MPV frame at physical pixel size
                            let fbo_id = video_fbo.0.get() as i32;
                            let _ = rc.0.render::<GetProcAddress>(
                                fbo_id,
                                target_phys_w,
                                target_phys_h,
                                false,
                            );

                            // Restore original FBO binding
                            gl.bind_framebuffer(eframe::glow::FRAMEBUFFER, target_fbo);

                            // Restore original viewport
                            gl.viewport(
                                original_viewport[0],
                                original_viewport[1],
                                original_viewport[2],
                                original_viewport[3],
                            );
                        }
                    }
                }
            }
        })),
    };

    ui.painter().add(callback);

    let is_hovering_file = ui.input(|i| !i.raw.hovered_files.is_empty());
    if is_hovering_file {
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(180));
        let font_id = egui::FontId::proportional(26.0);
        let galley = ui.painter().layout_no_wrap(
            format!(
                "{} {}",
                crate::ui::icons::FILE_VIDEO,
                app.tr("Drop video file here to play")
            ),
            font_id,
            egui::Color32::WHITE,
        );
        ui.painter().galley(
            rect.center() - galley.size() / 2.0,
            galley,
            egui::Color32::PLACEHOLDER,
        );
    }
}

fn osd_icon(message: &str) -> &'static str {
    let message = message.to_ascii_lowercase();
    if message.contains("pause") {
        crate::ui::icons::PAUSE
    } else if message.contains("play") || message.contains("replay") {
        crate::ui::icons::PLAY
    } else if message.contains("unmute") {
        crate::ui::icons::SPEAKER_HIGH
    } else if message.contains("mute") {
        crate::ui::icons::SPEAKER_SLASH
    } else if message.contains("volume") || message.contains("audio") {
        crate::ui::icons::SPEAKER_HIGH
    } else if message.contains("frame") {
        crate::ui::icons::FRAME_CORNERS
    } else if message.contains("seek") {
        crate::ui::icons::FAST_FORWARD
    } else if message.contains("fullscreen") {
        crate::ui::icons::ARROWS_OUT
    } else if message.contains("subtitle") {
        crate::ui::icons::SUBTITLES
    } else if message.contains("open") || message.contains("load") || message.contains("video") {
        crate::ui::icons::FILE_VIDEO
    } else if message.contains("error") || message.contains("failed") {
        crate::ui::icons::WARNING
    } else {
        crate::ui::icons::INFO
    }
}

pub fn calculate_physical_bounds(rect: egui::Rect, ppi: f32) -> (i32, i32) {
    let w = (rect.width() * ppi).round() as i32;
    let h = (rect.height() * ppi).round() as i32;
    (w.max(1), h.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_physical_bounds() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));

        // 1.0x scaling (standard DPI)
        assert_eq!(calculate_physical_bounds(rect, 1.0), (800, 600));

        // 1.25x scaling (125% High DPI)
        assert_eq!(calculate_physical_bounds(rect, 1.25), (1000, 750));

        // 1.5x scaling (150% High DPI)
        assert_eq!(calculate_physical_bounds(rect, 1.5), (1200, 900));

        // 2.0x scaling (200% Retina / 4K DPI)
        assert_eq!(calculate_physical_bounds(rect, 2.0), (1600, 1200));
    }

    #[test]
    fn playing_seek_drag_uses_one_cumulative_origin() {
        assert_eq!(seek_target_from_drag(100.0, 120.0, 500.0), 110.0);
        assert_eq!(seek_target_from_drag(100.0, -1_800.0, 500.0), 0.0);
        assert_eq!(seek_target_from_drag(490.0, 240.0, 500.0), 500.0);
    }

    #[test]
    fn temporary_fast_forward_starts_without_pointer_motion() {
        assert_eq!(temporary_fast_forward_rate(1.0, 3.0, 0.0), 3.0);
        assert_eq!(temporary_fast_forward_rate(1.5, 2.0, 0.0), 2.0);
        assert_eq!(temporary_fast_forward_rate(3.0, 2.0, 0.0), 3.0);
        assert_eq!(temporary_fast_forward_rate(1.0, 2.0, 2_240.0), 16.0);
    }

    #[test]
    fn fast_forward_hold_does_not_turn_release_into_play_pause_click() {
        assert!(!should_consume_fast_forward_click(
            crate::config::PlayerDragAction::TemporaryFastForward,
            false,
            std::time::Duration::from_millis(50),
        ));
        assert!(should_consume_fast_forward_click(
            crate::config::PlayerDragAction::TemporaryFastForward,
            false,
            std::time::Duration::from_millis(250),
        ));
        assert!(should_consume_fast_forward_click(
            crate::config::PlayerDragAction::TemporaryFastForward,
            true,
            std::time::Duration::ZERO,
        ));
    }
}
