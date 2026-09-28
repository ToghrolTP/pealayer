use crate::app::PealayerApp;
use crate::mpv::render::GetProcAddress;
use eframe::egui;
use std::sync::Arc;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let video_files_label = app.tr("Video Files");
    let video_size = ui.available_size();
    if video_size.x <= 0.0 || video_size.y <= 0.0 {
        return;
    }

    let (rect, response) = ui.allocate_exact_size(video_size, egui::Sense::click_and_drag());

    if response.clicked_by(egui::PointerButton::Primary) {
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

    if response.dragged() && app.current_video_path.is_some() {
        let action = if app.is_paused {
            app.paused_drag_action
        } else {
            app.playing_drag_action
        };
        match action {
            crate::config::PlayerDragAction::MoveWindow => {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            crate::config::PlayerDragAction::Seek => {
                let delta = ui.input(|input| input.pointer.delta().x) as f64 / 12.0;
                if delta.abs() >= 0.01 {
                    app.seek_relative(delta);
                }
            }
            crate::config::PlayerDragAction::TemporaryFastForward => {
                let speed = (1.0 + response.drag_delta().x.abs() as f64 / 160.0).clamp(1.0, 4.0);
                let _ = app.mpv.set_property("speed", speed);
                app.set_osd(format!("{}: {speed:.1}×", app.tr("Playback speed")));
            }
            crate::config::PlayerDragAction::None => {}
        }
    }
    if response.drag_stopped() {
        let _ = app.mpv.set_property("speed", 1.0_f64);
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
            .button(format!("▣ {}", app.tr("Open Video File...")))
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
            .button(format!("↗ {}", app.tr("Open Location / URL...")))
            .clicked()
        {
            ui.close();
            app.show_open_url_dialog = true;
        }

        let has_video = app.current_video_path.is_some();
        if ui
            .add_enabled(
                has_video,
                egui::Button::new(format!("× {}", app.tr("Close Video"))),
            )
            .clicked()
        {
            ui.close();
            app.close_video();
        }

        ui.separator();

        let play_title = if app.is_playback_finished() {
            format!("↺ {}", app.tr("Replay"))
        } else if app.is_paused {
            format!("▶ {}", app.tr("Play"))
        } else {
            format!("⏸ {}", app.tr("Pause"))
        };
        if ui
            .add_enabled(has_video, egui::Button::new(play_title))
            .clicked()
        {
            ui.close();
            app.toggle_playback();
        }

        let is_fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
        let fs_title = if is_fullscreen {
            format!("⤡ {}", app.tr("Exit Fullscreen"))
        } else {
            format!("⛶ {}", app.tr("Fullscreen"))
        };
        if ui.button(fs_title).clicked() {
            ui.close();
            app.set_fullscreen(ui.ctx(), !is_fullscreen);
        }

        let mute_title = if app.is_muted {
            format!("🔊 {}", app.tr("Unmute"))
        } else {
            format!("🔇 {}", app.tr("Mute"))
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

        ui.menu_button(format!("◷ {}", app.tr("Open Recent")), |ui| {
            if app.recent_media.is_empty() {
                ui.label(app.tr("No recent media"));
            } else {
                for path in app.recent_media.clone() {
                    let file_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(str::to_owned)
                        .unwrap_or_else(|| app.tr("Unknown"));
                    if ui
                        .button(app.display_text(&file_name))
                        .on_hover_text(path.display().to_string())
                        .clicked()
                    {
                        ui.close();
                        app.load_video_file(path);
                    }
                }
                ui.separator();
                if ui.button(app.tr("Clear Recent")).clicked() {
                    ui.close();
                    app.clear_recent_media();
                }
            }
        });

        let pin_title = if app.pin_controls {
            format!("◆ {}", app.tr("Unpin Controls"))
        } else {
            format!("◇ {}", app.tr("Pin Controls"))
        };
        if ui.button(pin_title).clicked() {
            ui.close();
            app.pin_controls = !app.pin_controls;
            app.save_config();
        }
    });

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
            let galley = ui
                .painter()
                .layout_no_wrap(msg.clone(), font_id, text_color);
            let rect = match app.osd_position {
                crate::config::OsdPosition::TopLeft => egui::Rect::from_min_size(
                    center,
                    galley.size() + egui::vec2(24.0, 16.0),
                ),
                crate::config::OsdPosition::Center => egui::Rect::from_center_size(
                    center,
                    galley.size() + egui::vec2(24.0, 16.0),
                ),
            };
            ui.painter().rect_filled(rect, 8.0, bg_color);
            ui.painter().galley(
                rect.min + egui::vec2(12.0, 8.0),
                galley,
                egui::Color32::PLACEHOLDER,
            );

            // Request a repaint to animate the fade-out
            ui.ctx().request_repaint();
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
            format!("▣ {}", app.tr("Drop video file here to play")),
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
}
