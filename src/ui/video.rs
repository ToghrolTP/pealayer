use crate::app::PealayerApp;
use crate::mpv::render::GetProcAddress;
use eframe::egui;
use std::sync::Arc;

const DEFAULT_VIDEO_ASPECT_RATIO: f64 = 16.0 / 9.0;

fn resolved_video_aspect_ratio(aspect_ratio: f64) -> f32 {
    if aspect_ratio.is_finite() && (0.05..=20.0).contains(&aspect_ratio) {
        aspect_ratio as f32
    } else {
        DEFAULT_VIDEO_ASPECT_RATIO as f32
    }
}

fn aspect_matched_inner_size(
    current_inner_size: egui::Vec2,
    current_video_size: egui::Vec2,
    monitor_size: Option<egui::Vec2>,
    aspect_ratio: f32,
) -> Option<egui::Vec2> {
    if !current_inner_size.is_finite()
        || !current_video_size.is_finite()
        || !aspect_ratio.is_finite()
        || current_inner_size.x <= 0.0
        || current_inner_size.y <= 0.0
        || current_video_size.x <= 0.0
        || current_video_size.y <= 0.0
        || !(0.05..=20.0).contains(&aspect_ratio)
    {
        return None;
    }

    let chrome = (current_inner_size - current_video_size).max(egui::Vec2::ZERO);
    let mut target = egui::vec2(
        current_inner_size.x,
        chrome.y + current_video_size.x / aspect_ratio,
    );

    // Preserve the current video width whenever practical. Extremely tall or
    // wide media is fitted to the monitor instead of producing an unreachable
    // window. eframe performs the final platform work-area clamp as well.
    if let Some(monitor) = monitor_size.filter(|size| size.x > 0.0 && size.y > 0.0) {
        let maximum = egui::vec2((monitor.x - 48.0).max(320.0), (monitor.y - 96.0).max(240.0));
        if target.y > maximum.y {
            let video_height = (maximum.y - chrome.y).max(1.0);
            target = egui::vec2(chrome.x + video_height * aspect_ratio, maximum.y);
        }
        if target.x > maximum.x {
            let video_width = (maximum.x - chrome.x).max(1.0);
            target = egui::vec2(maximum.x, chrome.y + video_width / aspect_ratio);
        }
    }

    Some(target.max(egui::vec2(320.0, 240.0)))
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct VideoSurfaceGesture {
    pub(crate) button: egui::PointerButton,
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

fn should_consume_gesture_click(
    action: crate::config::PlayerDragAction,
    dragged: bool,
    held_for: std::time::Duration,
) -> bool {
    dragged
        || (action == crate::config::PlayerDragAction::TemporaryFastForward
            && held_for >= std::time::Duration::from_millis(180))
}

fn gesture_action_for_button(
    app: &PealayerApp,
    button: egui::PointerButton,
) -> crate::config::PlayerDragAction {
    match button {
        egui::PointerButton::Primary if app.is_paused => app.paused_drag_action,
        egui::PointerButton::Primary => app.playing_drag_action,
        egui::PointerButton::Middle => app.middle_hold_action,
        egui::PointerButton::Secondary => app.right_hold_action,
        egui::PointerButton::Extra1 | egui::PointerButton::Extra2 => {
            crate::config::PlayerDragAction::None
        }
    }
}

fn begin_video_surface_gesture(
    app: &mut PealayerApp,
    button: egui::PointerButton,
    action: crate::config::PlayerDragAction,
) {
    if app.video_surface_gesture.is_some() {
        return;
    }
    let gesture = VideoSurfaceGesture {
        button,
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
            app.set_osd(format!(
                "{}: {:.1}×",
                app.tr("Playback speed"),
                gesture.start_rate
            ));
        }
        crate::config::PlayerDragAction::MoveWindow | crate::config::PlayerDragAction::None => {}
    }
    should_consume_gesture_click(
        gesture.action,
        gesture.dragged,
        gesture.started_at.elapsed(),
    )
}

fn perform_video_surface_click(
    app: &mut PealayerApp,
    ctx: &egui::Context,
    action: crate::config::PlayerClickAction,
) {
    match action {
        crate::config::PlayerClickAction::PlayPause => {
            if app.current_video_path.is_some() {
                app.toggle_playback();
            }
        }
        crate::config::PlayerClickAction::ToggleMute => {
            if app.current_video_path.is_some() {
                app.toggle_audio_muted();
            }
        }
        crate::config::PlayerClickAction::ToggleFullscreen => {
            if app.current_video_path.is_some() {
                app.toggle_fullscreen(ctx);
            }
        }
        crate::config::PlayerClickAction::ContextMenu | crate::config::PlayerClickAction::None => {}
    }
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let video_size = ui.available_size();
    if video_size.x <= 0.0 || video_size.y <= 0.0 {
        return;
    }

    let (rect, response) = ui.allocate_exact_size(video_size, egui::Sense::click_and_drag());
    let aspect_ratio = resolved_video_aspect_ratio(app.video_aspect_ratio);
    let is_fullscreen = app.fullscreen_intent(ui.ctx());
    let simple_aspect_lock =
        app.consistent_video_aspect_ratio && !app.show_four_d_editor && !is_fullscreen;
    let pixels_per_point = ui.ctx().pixels_per_point();
    let viewport = ui.input(|input| input.viewport().clone());
    if simple_aspect_lock {
        if let Some(outer_rect) = viewport.outer_rect {
            let outer_width = (outer_rect.width() * pixels_per_point).round() as i32;
            let outer_height = (outer_rect.height() * pixels_per_point).round() as i32;
            let video_width = (rect.width() * pixels_per_point).round() as i32;
            let video_height = (rect.height() * pixels_per_point).round() as i32;
            crate::platform::windows::set_simple_video_aspect_constraint(
                true,
                f64::from(aspect_ratio),
                outer_width.saturating_sub(video_width),
                outer_height.saturating_sub(video_height),
            );
        } else {
            crate::platform::windows::set_simple_video_aspect_constraint(
                false,
                f64::from(aspect_ratio),
                0,
                0,
            );
        }
    } else {
        crate::platform::windows::set_simple_video_aspect_constraint(
            false,
            f64::from(aspect_ratio),
            0,
            0,
        );
    }

    if simple_aspect_lock
        && app.pending_video_aspect_resize
        && app.current_video_path.is_some()
        && !viewport.maximized.unwrap_or(false)
        && !viewport.minimized.unwrap_or(false)
        && !crate::platform::windows::native_window_operation_active()
        && let Some(inner_rect) = viewport.inner_rect
        && let Some(target_size) = aspect_matched_inner_size(
            inner_rect.size(),
            rect.size(),
            viewport.monitor_size,
            aspect_ratio,
        )
    {
        // Avoid a redundant OS resize (and its visual layout work) when the
        // current surface is already within one logical pixel of the target.
        if (target_size.y - inner_rect.height()).abs() > 1.0
            || (target_size.x - inner_rect.width()).abs() > 1.0
        {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(target_size));
        }
        app.pending_video_aspect_resize = false;
    }

    const GESTURE_BUTTONS: [egui::PointerButton; 3] = [
        egui::PointerButton::Primary,
        egui::PointerButton::Middle,
        egui::PointerButton::Secondary,
    ];

    if response.hovered() && app.current_video_path.is_some() {
        // Temporary fast-forward is a hold gesture, so it starts on mouse-down
        // without requiring movement. Every supported pointer button follows
        // the same rule and one button owns the gesture until release.
        for button in GESTURE_BUTTONS {
            let pressed = ui.input(|input| input.pointer.button_pressed(button));
            let action = gesture_action_for_button(app, button);
            if pressed && action == crate::config::PlayerDragAction::TemporaryFastForward {
                begin_video_surface_gesture(app, button, action);
                break;
            }
        }
    }

    if app.video_surface_gesture.is_none() && app.current_video_path.is_some() {
        for button in GESTURE_BUTTONS {
            if response.drag_started_by(button) {
                let action = gesture_action_for_button(app, button);
                if action != crate::config::PlayerDragAction::None {
                    begin_video_surface_gesture(app, button, action);
                    if action == crate::config::PlayerDragAction::Seek {
                        app.scrub_to(app.playback_time);
                    }
                }
                break;
            }
        }
    }

    if app.current_video_path.is_some()
        && let Some(mut gesture) = app.video_surface_gesture
        && response.dragged_by(gesture.button)
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

    let gesture_released = app.video_surface_gesture.is_some_and(|gesture| {
        response.drag_stopped_by(gesture.button)
            || !ui.input(|input| input.pointer.button_down(gesture.button))
    });
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
            app.open_media_file_dialog(ui.ctx());
        }
    }

    if !suppress_click && response.clicked_by(egui::PointerButton::Middle) {
        let action = app.middle_click_action;
        perform_video_surface_click(app, ui.ctx(), action);
    }
    if !suppress_click
        && response.clicked_by(egui::PointerButton::Secondary)
        && app.right_click_action != crate::config::PlayerClickAction::ContextMenu
    {
        let action = app.right_click_action;
        perform_video_surface_click(app, ui.ctx(), action);
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

    let middle_context_menu =
        app.middle_click_action == crate::config::PlayerClickAction::ContextMenu;
    let right_context_menu =
        app.right_click_action == crate::config::PlayerClickAction::ContextMenu;
    if middle_context_menu || right_context_menu {
        let should_open = !suppress_click
            && ((middle_context_menu && response.clicked_by(egui::PointerButton::Middle))
                || (right_context_menu && response.clicked_by(egui::PointerButton::Secondary)));
        let open_command = if should_open {
            Some(egui::SetOpenCommand::Bool(true))
        } else if response.clicked() {
            Some(egui::SetOpenCommand::Bool(false))
        } else {
            None
        };
        egui::Popup::menu(&response)
            .open_memory(open_command)
            .at_pointer_fixed()
            .show(|ui| {
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::PLAY,
                        app.tr("Open Video File...")
                    ))
                    .clicked()
                {
                    ui.close();
                    app.open_media_file_dialog(ui.ctx());
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
                        egui::Button::new(format!(
                            "{} {}",
                            crate::ui::icons::X,
                            app.tr("Close Video")
                        )),
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
                    app.toggle_audio_muted();
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
    }

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

    // 1. Calculate the destination from libmpv's post-filter display aspect.
    // The same value drives native WM_SIZING, so the outer window and the
    // rendered image cannot drift apart during a Simple-workspace resize.
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

    let taskbar_video_rect =
        (app.windows_video_taskbar_thumbnail && app.current_video_path.is_some()).then(|| {
            [
                (dest_rect.left() * pixels_per_point).round() as i32,
                (dest_rect.top() * pixels_per_point).round() as i32,
                (dest_rect.right() * pixels_per_point).round() as i32,
                (dest_rect.bottom() * pixels_per_point).round() as i32,
            ]
        });
    let hwnd = app
        .window_handle
        .unwrap_or_else(crate::platform::windows::get_registered_hwnd);
    #[cfg(target_os = "windows")]
    let composition_media = app
        .current_video_path
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned());
    #[cfg(target_os = "windows")]
    let taskbar_media = app
        .windows_video_taskbar_thumbnail
        .then(|| composition_media.clone())
        .flatten();
    app.taskbar_video_rect = taskbar_video_rect;

    // 2. Calculate DPI-aware physical pixel dimensions
    let ppi = pixels_per_point;
    let (target_phys_w, target_phys_h) = calculate_physical_bounds(dest_rect, ppi);

    #[cfg(all(target_os = "windows", feature = "d3d11-composition-experiment"))]
    let composition_popup_fallback = app.active_windows_video_renderer
        == crate::config::WindowsVideoRenderer::D3D11
        && !app.windows_detached_video_panel
        && egui::Popup::is_any_open(ui.ctx());
    #[cfg(all(target_os = "windows", feature = "d3d11-composition-experiment"))]
    let composition_active = app.active_windows_video_renderer
        == crate::config::WindowsVideoRenderer::D3D11
        && if app.windows_detached_video_panel {
            crate::platform::d3d11_composition::active()
        } else if composition_popup_fallback {
            if let Some(media) = composition_media.as_deref() {
                crate::platform::d3d11_composition::capture_for_overlay(
                    hwnd,
                    media,
                    (target_phys_w.max(1) as u32, target_phys_h.max(1) as u32),
                );
                // The zero-copy visual has to be hidden while an egui popup is
                // above it, but keep the temporary high-quality texture moving
                // at compositor cadence rather than an 8 Hz taskbar cadence.
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(16));
            }
            if let Some(frame) = crate::platform::d3d11_composition::overlay_frame_rgba() {
                let size = [frame.width() as usize, frame.height() as usize];
                let image = egui::ColorImage::from_rgba_unmultiplied(size, frame.as_raw());
                if let Some(texture) = app.d3d11_overlay_texture.as_mut() {
                    texture.set(image, egui::TextureOptions::LINEAR);
                } else {
                    app.d3d11_overlay_texture = Some(ui.ctx().load_texture(
                        "d3d11-popup-video-fallback",
                        image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
            }
            let _ = crate::platform::d3d11_composition::update(
                &app.mpv_client,
                hwnd,
                dest_rect,
                pixels_per_point,
                false,
                taskbar_media.as_deref(),
            );
            false
        } else {
            crate::platform::d3d11_composition::clear_overlay_frame();
            app.d3d11_overlay_texture = None;
            crate::platform::d3d11_composition::update(
                &app.mpv_client,
                hwnd,
                dest_rect,
                pixels_per_point,
                app.current_video_path.is_some(),
                taskbar_media.as_deref(),
            )
        };
    #[cfg(not(all(target_os = "windows", feature = "d3d11-composition-experiment")))]
    let composition_active = false;

    // Draw the offscreen texture if registered
    let texture_id_opt = app
        .rtt_state
        .try_lock()
        .ok()
        .and_then(|rtt| rtt.video_texture_id);

    if !composition_active {
        #[cfg(all(target_os = "windows", feature = "d3d11-composition-experiment"))]
        let popup_texture = composition_popup_fallback
            .then(|| {
                app.d3d11_overlay_texture
                    .as_ref()
                    .map(egui::TextureHandle::id)
            })
            .flatten();
        #[cfg(not(all(target_os = "windows", feature = "d3d11-composition-experiment")))]
        let popup_texture = None;
        if let Some(texture_id) = popup_texture.or(texture_id_opt) {
            ui.painter().image(
                texture_id,
                dest_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
    }

    // 2.5 Draw OSD overlay if active
    if let Some((msg, timestamp)) = &app.osd_message {
        let elapsed = timestamp.elapsed().as_secs_f32();
        let options = app.osd_display_options.as_ref();
        let timeout = options
            .and_then(|options| options.timeout_seconds)
            .unwrap_or(app.osd_timeout_seconds)
            .max(0.25);
        let fade_start = (timeout - 0.6).max(0.4);
        if elapsed < timeout {
            let alpha = if elapsed < fade_start {
                1.0
            } else {
                ((timeout - elapsed) / (timeout - fade_start)).clamp(0.0, 1.0)
            };
            let text_color = options
                .and_then(|options| options.text_color.as_deref())
                .and_then(parse_osd_color)
                .unwrap_or(egui::Color32::WHITE)
                .linear_multiply(alpha);
            let bg_color = options
                .and_then(|options| options.background_color.as_deref())
                .and_then(parse_osd_color)
                .unwrap_or_else(|| egui::Color32::from_black_alpha(180))
                .linear_multiply(alpha);
            let font_size = options
                .and_then(|options| options.font_size)
                .unwrap_or(22.0)
                .clamp(8.0, 128.0);
            let font_id = egui::FontId::proportional(font_size);
            let osd_text = osd_text_with_icon(
                options.and_then(|options| options.icon.as_deref()),
                msg,
            );
            let galley = ui.painter().layout_no_wrap(osd_text, font_id, text_color);
            let padding = egui::vec2(
                options
                    .and_then(|options| options.padding_x)
                    .unwrap_or(12.0),
                options.and_then(|options| options.padding_y).unwrap_or(8.0),
            );
            let box_size = galley.size() + padding * 2.0;
            let default_anchor = match app.osd_position {
                crate::config::OsdPosition::TopLeft => crate::platform::interop::OsdAnchor::TopLeft,
                crate::config::OsdPosition::Center => crate::platform::interop::OsdAnchor::Center,
            };
            let anchor = options
                .and_then(|options| options.position)
                .unwrap_or(default_anchor);
            let rect = osd_rect(
                dest_rect,
                box_size,
                anchor,
                options.and_then(|options| options.x_percent),
                options.and_then(|options| options.y_percent),
            );
            let corner_radius = options
                .and_then(|options| options.corner_radius)
                .unwrap_or(8.0);
            ui.painter().rect_filled(rect, corner_radius, bg_color);
            ui.painter()
                .galley(rect.min + padding, galley, egui::Color32::PLACEHOLDER);

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
            // Check the configured native policy at paint time as well as the
            // egui-sampled drag flag. WM_ENTERSIZEMOVE can arrive after this
            // frame was assembled; live mode must still present it, while the
            // compatibility fallback deliberately retains the last frame.
            if is_operating || !crate::platform::windows::native_window_video_rendering_allowed() {
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

                            #[cfg(target_os = "windows")]
                            if let Some(media) = taskbar_media.as_deref() {
                                crate::platform::taskbar_preview::capture(
                                    gl,
                                    hwnd,
                                    video_fbo,
                                    target_phys_w,
                                    target_phys_h,
                                    media,
                                );
                            }

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

fn parse_osd_color(value: &str) -> Option<egui::Color32> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    let expand = |value: u8| value.saturating_mul(17);
    match hex.len() {
        3 | 4 => {
            let mut digits = hex
                .chars()
                .map(|character| character.to_digit(16).map(|v| v as u8));
            let red = expand(digits.next()??);
            let green = expand(digits.next()??);
            let blue = expand(digits.next()??);
            let alpha = digits.next().flatten().map(expand).unwrap_or(255);
            Some(egui::Color32::from_rgba_unmultiplied(
                red, green, blue, alpha,
            ))
        }
        6 | 8 => {
            let byte = |start: usize| u8::from_str_radix(&hex[start..start + 2], 16).ok();
            Some(egui::Color32::from_rgba_unmultiplied(
                byte(0)?,
                byte(2)?,
                byte(4)?,
                if hex.len() == 8 { byte(6)? } else { 255 },
            ))
        }
        _ => None,
    }
}

fn osd_rect(
    bounds: egui::Rect,
    size: egui::Vec2,
    anchor: crate::platform::interop::OsdAnchor,
    x_percent: Option<f32>,
    y_percent: Option<f32>,
) -> egui::Rect {
    use crate::platform::interop::OsdAnchor;

    let margin = 24.0;
    let min_x = bounds.left() + margin;
    let center_x = bounds.center().x;
    let max_x = bounds.right() - margin;
    let min_y = bounds.top() + margin;
    let center_y = bounds.center().y;
    let max_y = bounds.bottom() - margin;
    let (point, horizontal, vertical) = match anchor {
        OsdAnchor::TopLeft => (egui::pos2(min_x, min_y), -1, -1),
        OsdAnchor::TopCenter => (egui::pos2(center_x, min_y), 0, -1),
        OsdAnchor::TopRight => (egui::pos2(max_x, min_y), 1, -1),
        OsdAnchor::CenterLeft => (egui::pos2(min_x, center_y), -1, 0),
        OsdAnchor::Center => (egui::pos2(center_x, center_y), 0, 0),
        OsdAnchor::CenterRight => (egui::pos2(max_x, center_y), 1, 0),
        OsdAnchor::BottomLeft => (egui::pos2(min_x, max_y), -1, 1),
        OsdAnchor::BottomCenter => (egui::pos2(center_x, max_y), 0, 1),
        OsdAnchor::BottomRight => (egui::pos2(max_x, max_y), 1, 1),
    };
    let custom = x_percent.is_some() || y_percent.is_some();
    let point = egui::pos2(
        x_percent
            .map(|value| bounds.left() + bounds.width() * value.clamp(0.0, 100.0) / 100.0)
            .unwrap_or(point.x),
        y_percent
            .map(|value| bounds.top() + bounds.height() * value.clamp(0.0, 100.0) / 100.0)
            .unwrap_or(point.y),
    );
    let mut rect = if custom || (horizontal == 0 && vertical == 0) {
        egui::Rect::from_center_size(point, size)
    } else {
        let left = match horizontal {
            -1 => point.x,
            1 => point.x - size.x,
            _ => point.x - size.x / 2.0,
        };
        let top = match vertical {
            -1 => point.y,
            1 => point.y - size.y,
            _ => point.y - size.y / 2.0,
        };
        egui::Rect::from_min_size(egui::pos2(left, top), size)
    };
    let dx = if rect.left() < bounds.left() {
        bounds.left() - rect.left()
    } else if rect.right() > bounds.right() {
        bounds.right() - rect.right()
    } else {
        0.0
    };
    let dy = if rect.top() < bounds.top() {
        bounds.top() - rect.top()
    } else if rect.bottom() > bounds.bottom() {
        bounds.bottom() - rect.bottom()
    } else {
        0.0
    };
    rect = rect.translate(egui::vec2(dx, dy));
    rect
}

pub(crate) fn osd_text_with_icon(name: Option<&str>, message: &str) -> String {
    requested_osd_icon(name, message)
        .map(|icon| format!("{icon}  {message}"))
        .unwrap_or_else(|| message.to_owned())
}

fn requested_osd_icon(name: Option<&str>, message: &str) -> Option<&'static str> {
    let Some(name) = name.map(str::trim).filter(|name| !name.is_empty()) else {
        return Some(osd_icon(message));
    };
    match name.to_ascii_lowercase().as_str() {
        "none" | "hidden" => None,
        "auto" => Some(osd_icon(message)),
        "info" => Some(crate::ui::icons::INFO),
        "play" => Some(crate::ui::icons::PLAY),
        "pause" => Some(crate::ui::icons::PAUSE),
        "stop" => Some(crate::ui::icons::STOP_CIRCLE),
        "warning" => Some(crate::ui::icons::WARNING),
        "volume" | "speaker" => Some(crate::ui::icons::SPEAKER_HIGH),
        "mute" => Some(crate::ui::icons::SPEAKER_SLASH),
        "fast_forward" | "fast-forward" => Some(crate::ui::icons::FAST_FORWARD),
        "rewind" => Some(crate::ui::icons::REWIND),
        "record" => Some(crate::ui::icons::RECORD),
        "subtitle" | "subtitles" => Some(crate::ui::icons::SUBTITLES),
        "video" => Some(crate::ui::icons::FILE_VIDEO),
        "fullscreen" => Some(crate::ui::icons::ARROWS_OUT),
        other => crate::ui::icons::named_control_icon(other).or(Some(crate::ui::icons::INFO)),
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
    fn video_aspect_uses_live_value_and_rejects_invalid_input() {
        assert!((resolved_video_aspect_ratio(2.35) - 2.35).abs() < f32::EPSILON);
        assert_eq!(
            resolved_video_aspect_ratio(f64::NAN),
            DEFAULT_VIDEO_ASPECT_RATIO as f32
        );
        assert_eq!(
            resolved_video_aspect_ratio(0.0),
            DEFAULT_VIDEO_ASPECT_RATIO as f32
        );
    }

    #[test]
    fn automatic_aspect_resize_preserves_video_width_and_non_video_chrome() {
        let target = aspect_matched_inner_size(
            egui::vec2(1_000.0, 700.0),
            egui::vec2(900.0, 550.0),
            Some(egui::vec2(1_920.0, 1_080.0)),
            4.0 / 3.0,
        )
        .unwrap();
        assert_eq!(target.x, 1_000.0);
        assert!((target.y - 825.0).abs() < f32::EPSILON);
    }

    #[test]
    fn automatic_aspect_resize_fits_portrait_video_to_monitor() {
        let target = aspect_matched_inner_size(
            egui::vec2(1_000.0, 700.0),
            egui::vec2(900.0, 550.0),
            Some(egui::vec2(1_920.0, 1_080.0)),
            9.0 / 16.0,
        )
        .unwrap();
        assert!(target.y <= 984.0);
        assert!(target.x < 1_000.0);
    }

    #[test]
    fn automatic_aspect_resize_rejects_invalid_geometry() {
        assert!(aspect_matched_inner_size(
            egui::Vec2::ZERO,
            egui::vec2(900.0, 550.0),
            None,
            16.0 / 9.0,
        )
        .is_none());
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
        assert!(!should_consume_gesture_click(
            crate::config::PlayerDragAction::TemporaryFastForward,
            false,
            std::time::Duration::from_millis(50),
        ));
        assert!(should_consume_gesture_click(
            crate::config::PlayerDragAction::TemporaryFastForward,
            false,
            std::time::Duration::from_millis(250),
        ));
        assert!(should_consume_gesture_click(
            crate::config::PlayerDragAction::TemporaryFastForward,
            true,
            std::time::Duration::ZERO,
        ));
        assert!(should_consume_gesture_click(
            crate::config::PlayerDragAction::Seek,
            true,
            std::time::Duration::ZERO,
        ));
    }

    #[test]
    fn native_and_egui_osd_share_the_same_icon_text() {
        assert_eq!(
            osd_text_with_icon(None, "Pause"),
            format!("{}  Pause", crate::ui::icons::PAUSE)
        );
        assert_eq!(osd_text_with_icon(Some("none"), "Pause"), "Pause");
        assert_eq!(
            osd_text_with_icon(Some("volume"), "Volume: 50%"),
            format!("{}  Volume: 50%", crate::ui::icons::SPEAKER_HIGH)
        );
    }
}
