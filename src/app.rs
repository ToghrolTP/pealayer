use crate::mpv::render::RenderContextWrapper;
use eframe::egui;
use libmpv2::Mpv;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DragMode {
    Move,
    ResizeLeft,
    ResizeRight,
}

pub fn classify_clip_drag_mode(clip_left: f32, clip_right: f32, press_x: f32) -> DragMode {
    let width = (clip_right - clip_left).max(1.0);
    let handle_w = (width * 0.35).min(10.0);
    if press_x <= clip_left + handle_w {
        DragMode::ResizeLeft
    } else if press_x >= clip_right - handle_w {
        DragMode::ResizeRight
    } else {
        DragMode::Move
    }
}

pub fn update_effect_duration(effect: &mut crate::four_d::models::Effect, new_dur_ms: u64) {
    if effect.actions.len() <= 2 {
        if effect.actions.len() == 2 {
            effect.actions[1].offset_ms = new_dur_ms;
        }
    } else {
        let old_dur = effect.duration_ms.max(1) as f64;
        let ratio = new_dur_ms as f64 / old_dur;
        let len = effect.actions.len();
        for a in &mut effect.actions[0..len - 1] {
            a.offset_ms = ((a.offset_ms as f64) * ratio).round() as u64;
        }
        if let Some(last) = effect.actions.last_mut() {
            last.offset_ms = new_dur_ms;
        }
    }
    effect.duration_ms = new_dur_ms;
}

#[derive(Clone, Debug)]
pub struct ActiveDragState {
    pub instance_id: uuid::Uuid,
    pub mode: DragMode,
    pub initial_start_time_ms: u64,
    pub initial_duration_ms: u64,
    pub drag_start_x: f32,
    pub initial_positions: Vec<(uuid::Uuid, u64)>,
}

#[derive(Debug, Clone)]
pub struct EffectPreset {
    pub category: String,
    pub effect: crate::four_d::models::Effect,
}

#[derive(Debug, Clone)]
pub struct EffectDragPayload {
    pub name: String,
    pub icon: String,
    pub duration_ms: u64,
    pub target: crate::four_d::models::HardwareTarget,
    pub actions: Vec<crate::four_d::models::AtomicAction>,
    pub controller_macro: Option<crate::four_d::models::ControllerMacroCue>,
}

pub struct RttState {
    pub video_texture: Option<eframe::glow::Texture>,
    pub video_fbo: Option<eframe::glow::Framebuffer>,
    pub video_texture_id: Option<eframe::egui::TextureId>,
    pub texture_width: u32,
    pub texture_height: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyframeDragState {
    pub track_id: uuid::Uuid,
    pub keyframe_index: usize,
    pub start_pointer_pos: egui::Pos2,
    pub original_time_ms: u64,
    pub original_value: f32,
    pub group_originals: Vec<(uuid::Uuid, usize, u64, f32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DroppedFileKind {
    Media,
    Subtitle,
    Timeline,
}

pub fn dropped_file_kind(path: &std::path::Path) -> DroppedFileKind {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "srt" | "vtt" | "ass" | "ssa" | "sub" | "idx" | "sup" => DroppedFileKind::Subtitle,
        "json" => DroppedFileKind::Timeline,
        // Let mpv make the final decision for its broad set of supported audio,
        // video and playlist formats instead of maintaining a brittle allowlist.
        _ => DroppedFileKind::Media,
    }
}

pub struct PealayerApp {
    pub(crate) app_name: String,
    pub(crate) app_publisher: Option<String>,
    pub(crate) app_copyright: Option<String>,
    pub(crate) last_window_title: String,
    pub(crate) language_preference: crate::config::AppLanguage,
    pub(crate) language: crate::config::AppLanguage,
    pub(crate) direction_preference: crate::config::AppDirection,
    pub(crate) theme_preference: crate::config::AppTheme,
    pub(crate) rtl: bool,
    pub(crate) mpv: &'static Mpv,
    pub(crate) mpv_client: libmpv2::Mpv,
    pub(crate) render_context: Arc<Mutex<Option<RenderContextWrapper>>>,

    pub playback_time: f64,
    pub duration: f64,
    pub(crate) media_fps: f64,
    pub is_paused: bool,
    pub is_eof: bool,
    pub(crate) volume: f64,
    pub(crate) is_muted: bool,

    pub seek_pos: Option<f64>,
    pub(crate) seek_controller: crate::mpv::seek::SeekController,
    pub(crate) was_playing_before_scrub: bool,
    pub(crate) is_scrubbing: bool,
    pub(crate) last_mouse_activity: std::time::Instant,
    pub(crate) pin_controls: bool,

    pub show_error: Option<String>,

    // Subtitle state
    pub(crate) show_sub_settings: bool,
    pub(crate) sub_visibility: bool,
    pub(crate) sub_font_size: f64,
    pub(crate) sub_delay: f64,
    pub(crate) current_sid: String,
    pub(crate) sub_tracks: Vec<SubtitleTrack>,

    // Audio state
    pub(crate) show_audio_settings: bool,
    pub(crate) audio_delay: f64,
    pub(crate) current_aid: String,
    pub(crate) audio_tracks: Vec<AudioTrack>,

    // 4D Cinema state
    pub(crate) show_four_d_editor: bool,
    pub(crate) dock_state: egui_dock::DockState<crate::ui::layout::PealayerTab>,
    pub timeline: crate::four_d::models::Timeline,
    pub(crate) engine_handle: crate::four_d::engine::EngineHandle,

    pub(crate) recording_session: crate::four_d::curve_record::RecordingSession,
    pub(crate) input_capture: crate::four_d::input_capture::InputCaptureState,
    pub(crate) is_recording: bool,

    // Phase 4 & 5 Selection/Override state
    pub(crate) selected_instance_ids: std::collections::HashSet<uuid::Uuid>,
    pub selected_keyframes: std::collections::HashSet<(uuid::Uuid, usize)>,
    pub active_keyframe_drag: Option<KeyframeDragState>,
    pub timeline_zoom: f32,
    pub undo_stack: crate::four_d::history::UndoStack,
    pub(crate) recording_keys:
        std::collections::HashMap<eframe::egui::Key, (uuid::Uuid, std::time::Instant, u8)>,
    pub(crate) relay_overrides: std::collections::BTreeSet<u8>,

    // Phase 6 Preset Library state
    pub(crate) effects_search_query: String,
    pub(crate) track_muted: std::collections::BTreeSet<u8>,
    pub(crate) track_soloed: std::collections::BTreeSet<u8>,
    pub(crate) track_locked: std::collections::BTreeSet<u8>,
    pub(crate) active_drag: Option<ActiveDragState>,
    pub(crate) estop_active: bool,
    pub(crate) serial_port: String,
    pub(crate) is_connected: bool,
    pub(crate) lasso_origin: Option<egui::Pos2>,
    pub(crate) lasso_rect: Option<egui::Rect>,
    pub(crate) rtt_state: Arc<Mutex<RttState>>,
    pub(crate) current_video_path: Option<std::path::PathBuf>,
    pub(crate) show_remaining_time: bool,
    pub(crate) osd_message: Option<(String, std::time::Instant)>,
    pub(crate) recent_media: Vec<std::path::PathBuf>,
    pub(crate) show_open_url_dialog: bool,
    pub(crate) url_input_buffer: String,
    pub(crate) is_window_operating: bool,
    pub(crate) show_shortcuts_dialog: bool,
    pub(crate) show_about_dialog: bool,
    pub(crate) show_preferences_dialog: bool,
    pub(crate) preferences_tab: usize,
    pub(crate) pause_on_hardware_disconnect: bool,
    pub(crate) auto_connect_hardware: bool,
    pub(crate) click_player_to_toggle: bool,
    pub(crate) show_subseconds: bool,
    pub(crate) wheel_seek_seconds: f64,
    pub(crate) osd_position: crate::config::OsdPosition,
    pub(crate) osd_timeout_seconds: f32,
    pub(crate) paused_drag_action: crate::config::PlayerDragAction,
    pub(crate) playing_drag_action: crate::config::PlayerDragAction,
    pub(crate) was_hardware_connected: bool,
    pub(crate) was_board_connected: bool,
    pub(crate) connection_notice: Option<String>,
    pub(crate) workspace_before_fullscreen: Option<bool>,
    pub(crate) was_fullscreen: bool,
    pub(crate) desired_fullscreen: Option<bool>,
    pub(crate) interop_rx: std::sync::mpsc::Receiver<crate::platform::interop::InteropCommand>,
    pub(crate) controller_cmd_rx:
        std::sync::mpsc::Receiver<crate::platform::interop::ControllerDelivery>,
    pub(crate) web_state_tx: std::sync::mpsc::Sender<String>,
    pub(crate) web_cmd_rx: std::sync::mpsc::Receiver<crate::platform::interop::InteropCommand>,
    pub(crate) last_web_broadcast: Option<std::time::Instant>,
    pub(crate) media_controls: Option<crate::platform::media_controls::MediaControlsManager>,
    pub(crate) media_cmd_tx: std::sync::mpsc::Sender<crate::platform::interop::InteropCommand>,
    pub window_handle: Option<isize>,
    pub shell_initialized: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubtitleTrack {
    pub id: i64,
    pub title: Option<String>,
    pub lang: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioTrack {
    pub id: i64,
    pub title: Option<String>,
    pub lang: Option<String>,
}

impl eframe::App for PealayerApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        #[cfg(target_os = "windows")]
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if crate::platform::windows::get_registered_hwnd() == 0 {
                if let Ok(handle) = frame.window_handle() {
                    if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                        let hwnd = win32_handle.hwnd.get() as isize;
                        crate::platform::windows::register_window_hwnd(hwnd);
                        self.window_handle = Some(hwnd);
                    }
                }
            } else if self.window_handle.is_none() {
                self.window_handle = Some(crate::platform::windows::get_registered_hwnd());
            }
        }

        self.ensure_shell_initialized();

        if self.media_controls.is_none() {
            let hwnd = self
                .window_handle
                .unwrap_or_else(crate::platform::windows::get_registered_hwnd);
            self.media_controls = Some(crate::platform::media_controls::MediaControlsManager::new(
                hwnd,
                self.media_cmd_tx.clone(),
                ui.ctx().clone(),
            ));
        }

        // Track active window/panel drag operations safely without lock nesting
        let is_pointer_down = ui.input(|i| i.pointer.any_down());
        let is_using_pointer = ui.ctx().egui_is_using_pointer();
        self.is_window_operating =
            is_pointer_down && (self.active_drag.is_some() || is_using_pointer);

        // Process drag and dropped files
        let dropped_file_paths = ui.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .collect::<Vec<_>>()
        });
        if !dropped_file_paths.is_empty() {
            self.load_dropped_files(dropped_file_paths);
        }

        // Drain every command source before mutating player state. This keeps IPC,
        // Web UI, media controls, and PCController actions on one behavior path.
        let mut inbound_commands = Vec::new();
        while let Ok(command) = self.interop_rx.try_recv() {
            inbound_commands.push(("IPC", command));
        }
        while let Ok(command) = self.web_cmd_rx.try_recv() {
            inbound_commands.push(("Web UI", command));
        }
        while let Ok(delivery) = self.controller_cmd_rx.try_recv() {
            self.apply_interop_command(ui.ctx(), delivery.command.clone(), "PCController");
            delivery.acknowledge_applied();
        }
        for (source, command) in inbound_commands {
            self.apply_interop_command(ui.ctx(), command, source);
        }

        // Reconcile the workspace with the viewport before publishing status.
        // A fullscreen request can be observed in this same frame; preserving
        // the already-staged workspace prevents that observation from replacing
        // an NLE restore target with the forced Simple workspace.
        let is_fullscreen = ui.input(|input| input.viewport().fullscreen.unwrap_or(false));
        self.observe_fullscreen_state(is_fullscreen);

        // Broadcast state JSON to Web-UI clients (throttled to 10Hz to save CPU / network spam)
        let now = std::time::Instant::now();
        let should_broadcast = match self.last_web_broadcast {
            Some(last) => now.duration_since(last) >= std::time::Duration::from_millis(100),
            None => true,
        };

        if should_broadcast {
            self.last_web_broadcast = Some(now);
            let hardware = self.advertised_hardware();
            let controller_connected = self
                .engine_handle
                .is_connected
                .load(std::sync::atomic::Ordering::Relaxed);
            let hardware_connected = hardware
                .as_ref()
                .is_some_and(|capabilities| capabilities.board_connected);
            let status_resp = crate::platform::interop::PlayerStatusResponse {
                status: if !controller_connected {
                    "connecting"
                } else if !hardware_connected {
                    "hardware_unavailable"
                } else {
                    "ok"
                }
                .to_string(),
                playing: !self.is_paused && self.current_video_path.is_some(),
                volume: self.volume,
                playback_time: self.playback_time,
                duration: self.duration,
                current_video: self
                    .current_video_path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string()),
                fullscreen: is_fullscreen,
                workspace: if self.show_four_d_editor { "nle" } else { "simple" }.to_string(),
                controller_connected,
                hardware_connected,
                hardware: hardware.map(|capabilities| {
                    crate::platform::interop::HardwareStatusSummary {
                        board_name: capabilities.board_name,
                        relay_count: capabilities.relays.len(),
                        pwm_count: capabilities.pwm_channels.len(),
                        supports_rf_transmit: capabilities.supports_rf_transmit,
                        supports_addressable_led: capabilities.supports_addressable_led,
                        supports_segment_display: capabilities.supports_segment_display,
                        supports_lcd_display: capabilities.supports_lcd_display,
                    }
                }),
            };
            crate::platform::interop::set_live_status(status_resp.clone());
            if let Ok(json) = serde_json::to_string(&status_resp) {
                let _ = self.web_state_tx.send(json);
            }
        }

        // Initialize RTT texture once if not done yet
        let mut init_rtt = false;
        let mut rtt_data = None;

        {
            let rtt = self.rtt_state.lock().unwrap();
            if rtt.video_texture.is_none() {
                init_rtt = true;
            }
        }

        if init_rtt {
            if let Some(gl) = frame.gl() {
                unsafe {
                    use eframe::glow::HasContext;

                    let tex = gl.create_texture().unwrap();
                    gl.bind_texture(eframe::glow::TEXTURE_2D, Some(tex));
                    gl.tex_image_2d(
                        eframe::glow::TEXTURE_2D,
                        0,
                        eframe::glow::RGBA8 as i32,
                        1920,
                        1080,
                        0,
                        eframe::glow::RGBA,
                        eframe::glow::UNSIGNED_BYTE,
                        eframe::glow::PixelUnpackData::Slice(None),
                    );
                    gl.tex_parameter_i32(
                        eframe::glow::TEXTURE_2D,
                        eframe::glow::TEXTURE_MIN_FILTER,
                        eframe::glow::LINEAR as i32,
                    );
                    gl.tex_parameter_i32(
                        eframe::glow::TEXTURE_2D,
                        eframe::glow::TEXTURE_MAG_FILTER,
                        eframe::glow::LINEAR as i32,
                    );

                    let fbo = gl.create_framebuffer().unwrap();
                    gl.bind_framebuffer(eframe::glow::FRAMEBUFFER, Some(fbo));
                    gl.framebuffer_texture_2d(
                        eframe::glow::FRAMEBUFFER,
                        eframe::glow::COLOR_ATTACHMENT0,
                        eframe::glow::TEXTURE_2D,
                        Some(tex),
                        0,
                    );

                    gl.bind_framebuffer(eframe::glow::FRAMEBUFFER, None);

                    // Register the texture with eframe/egui
                    let texture_id = frame.register_native_glow_texture(tex);

                    rtt_data = Some((tex, fbo, texture_id));
                }
            }
        }

        if let Some((tex, fbo, texture_id)) = rtt_data {
            if let Ok(mut rtt) = self.rtt_state.try_lock() {
                rtt.video_texture = Some(tex);
                rtt.video_fbo = Some(fbo);
                rtt.video_texture_id = Some(texture_id);
            }
        }

        let ctx = ui.ctx().clone();

        self.process_events();
        self.update_shell_state();
        if let Some(ref mut mc) = self.media_controls {
            mc.update_playback(self.is_paused, self.playback_time, self.duration);
        }

        // Connection loss and retry are normal runtime states. Surface them in
        // the status chrome instead of interrupting playback with a modal.
        if let Ok(mut err_guard) = self.engine_handle.connection_error.try_lock() {
            if let Some(err) = err_guard.take() {
                self.connection_notice = Some(err);
            }
        }
        let connected_now = self
            .engine_handle
            .is_connected
            .load(std::sync::atomic::Ordering::Relaxed);
        let board_connected_now = connected_now
            && self
                .advertised_hardware()
                .is_some_and(|capabilities| capabilities.board_connected);
        if hardware_connection_was_lost(
            self.was_hardware_connected,
            connected_now,
            self.was_board_connected,
            board_connected_now,
        ) && self.pause_on_hardware_disconnect
        {
            self.pause();
            self.set_osd(self.tr("Hardware disconnected — playback paused"));
        }
        if connected_now && !self.was_hardware_connected {
            self.connection_notice = None;
            self.save_config();
        }
        self.is_connected = connected_now;
        self.was_hardware_connected = connected_now;
        self.was_board_connected = board_connected_now;
        if connected_now {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        let connection_requested = self
            .engine_handle
            .connection_requested
            .load(std::sync::atomic::Ordering::Relaxed);
        let window_title = contextual_window_title(
            &self.app_name,
            self.current_video_path.as_deref(),
            self.is_connected,
            connection_requested,
        );
        if self.last_window_title != window_title {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Title(window_title.clone()));
            self.last_window_title = window_title;
        }
        crate::platform::windows::set_window_theme(ui.style().visuals.dark_mode);

        if !is_fullscreen {
            crate::ui::menu::draw(self, ui);
            crate::ui::status_bar::draw(self, ui);
        }

        // Track mouse movement
        if ctx.input(|i| {
            i.pointer.delta().length() > 0.0 || i.pointer.any_click() || i.pointer.any_pressed()
        }) {
            self.last_mouse_activity = std::time::Instant::now();
        } else if self.last_mouse_activity.elapsed().as_secs_f32() > 3.0 && !self.pin_controls {
            // Hide mouse cursor when inactive
            ctx.set_cursor_icon(egui::CursorIcon::None);
        }

        // Handle Keyboard Shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.toggle_playback();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F)) {
            self.toggle_fullscreen(&ctx);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::M)) {
            let _ = self.mpv.command("cycle", &["mute"]);
            self.is_muted = !self.is_muted;
            self.set_osd(if self.is_muted {
                "Mute".to_string()
            } else {
                "Unmute".to_string()
            });
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            self.seek_relative(-5.0);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            self.seek_relative(5.0);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Period) || i.key_pressed(egui::Key::CloseBracket))
        {
            let _ = self.mpv.command("frame-step", &[]);
            if self.current_video_path.is_some() {
                self.set_osd("Frame Step: +1".to_string());
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Comma) || i.key_pressed(egui::Key::OpenBracket)) {
            let _ = self.mpv.command("frame-back-step", &[]);
            if self.current_video_path.is_some() {
                self.set_osd("Frame Step: -1".to_string());
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            let _ = self.mpv.command("add", &["volume", "5"]);
            self.volume = (self.volume + 5.0).clamp(0.0, 130.0);
            self.set_osd(format!("Volume: {:.0}%", self.volume));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            let _ = self.mpv.command("add", &["volume", "-5"]);
            self.volume = (self.volume - 5.0).clamp(0.0, 130.0);
            self.set_osd(format!("Volume: {:.0}%", self.volume));
        }

        const MACRO_KEYS: [egui::Key; 8] = [
            egui::Key::F1,
            egui::Key::F2,
            egui::Key::F3,
            egui::Key::F4,
            egui::Key::F5,
            egui::Key::F6,
            egui::Key::F7,
            egui::Key::F8,
        ];
        let shortcut_relays = self
            .advertised_hardware()
            .filter(|capabilities| capabilities.board_connected)
            .map(|capabilities| capabilities.relays)
            .unwrap_or_default();

        let mut timeline_dirty = false;

        for (index, key) in MACRO_KEYS.into_iter().enumerate() {
            if ctx.input(|i| i.key_pressed(key)) && !self.recording_keys.contains_key(&key) {
                let Some(relay) = shortcut_relays.get(index) else {
                    continue;
                };
                let start_time = (self.playback_time * 1000.0) as u64;

                let actions = crate::four_d::patterns::generate_constant(relay.id, true, 100);
                let template = crate::four_d::models::Effect::with_target(
                    relay.name.clone(),
                    String::new(),
                    100,
                    crate::four_d::models::HardwareTarget::Relay(relay.id),
                    actions,
                );
                let template_id = template.id;
                self.timeline.templates.push(template);

                let instance = crate::four_d::models::EffectInstance::new(template_id, start_time);
                let instance_id = instance.id;
                self.timeline.instances.push(instance);

                self.recording_keys
                    .insert(key, (instance_id, std::time::Instant::now(), relay.id));
                timeline_dirty = true;
            }

            if ctx.input(|i| i.key_released(key)) {
                if let Some((instance_id, start_instant, relay_id)) =
                    self.recording_keys.remove(&key)
                {
                    if let Some(instance) =
                        self.timeline.instances.iter().find(|i| i.id == instance_id)
                    {
                        let mut duration = start_instant.elapsed().as_millis() as u64;
                        if duration < 100 {
                            duration = 100; // minimum duration
                        }

                        if let Some(template) = self
                            .timeline
                            .templates
                            .iter_mut()
                            .find(|t| t.id == instance.effect_id)
                        {
                            template.duration_ms = duration;
                            template.actions = crate::four_d::patterns::generate_constant(
                                relay_id, true, duration,
                            );
                        }
                        timeline_dirty = true;
                    }
                }
            }
        }

        if timeline_dirty {
            self.sync_timeline_engine();
        }

        let mut frame = egui::Frame::central_panel(&ui.style());
        frame.inner_margin = egui::Margin::same(0);

        egui::CentralPanel::default()
            .frame(frame)
            .show_inside(ui, |ui| {
                if self.show_four_d_editor {
                    let mut dock_state =
                        std::mem::replace(&mut self.dock_state, egui_dock::DockState::new(vec![]));
                    let dock_response = ui.scope(|ui| {
                        let mut tab_viewer = crate::ui::layout::PealayerTabViewer { app: self };
                        egui_dock::DockArea::new(&mut dock_state)
                            .show_inside(ui, &mut tab_viewer);
                    });
                    self.dock_state = dock_state;
                    let pointer_is_in_primary_tab_header = ui
                        .ctx()
                        .pointer_latest_pos()
                        .is_some_and(|pointer| {
                            pointer.y >= dock_response.response.rect.top()
                                && pointer.y <= dock_response.response.rect.top() + 32.0
                        });
                    if pointer_is_in_primary_tab_header {
                        dock_response.response.context_menu(|ui| {
                            ui.label(egui::RichText::new(self.tr("Workspace")).strong());
                            ui.separator();
                            if ui
                                .button(format!(
                                    "▦ {}",
                                    self.tr("Reset workspace layout")
                                ))
                                .clicked()
                            {
                                self.dock_state = crate::ui::layout::create_initial_layout();
                                ui.close();
                            }
                            if ui
                                .button(format!(
                                    "▶ {}",
                                    self.tr("Switch to Simple Player")
                                ))
                                .clicked()
                            {
                                self.show_four_d_editor = false;
                                ui.close();
                            }
                            ui.separator();
                            if ui
                                .button(format!("⚙ {}", self.tr("Preferences...")))
                                .clicked()
                            {
                                self.show_preferences_dialog = true;
                                ui.close();
                            }
                        });
                    }
                } else {
                    crate::ui::video::draw(self, ui);
                    crate::ui::controls::draw(self, ui);
                    crate::ui::four_d::draw_editor(self, ui);
                }
                crate::ui::error::draw(self, ui);
                crate::ui::subtitles::draw_settings_dialog(self, ui);
                crate::ui::audio::draw_settings_dialog(self, ui);
                crate::ui::preferences::draw(self, ui);

                if self.show_open_url_dialog {
                    let mut open_url = false;
                    let mut close_dialog = false;

                    egui::Window::new(format!("↗ {}", self.tr("Open Location / URL")))
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                        .show(ui.ctx(), |ui| {
                            ui.with_layout(crate::ui::i18n::vertical_layout(self.rtl), |ui| {
                                ui.label(
                                    self.tr(
                                        "Enter direct video URL, HTTP/HTTPS stream, or HLS link:",
                                    ),
                                );
                                ui.add_space(6.0);

                                ui.with_layout(
                                    crate::ui::i18n::layout(self.rtl, egui::Align::Center),
                                    |ui| {
                                        let text_edit = ui.add(
                                            egui::TextEdit::singleline(&mut self.url_input_buffer)
                                                .desired_width(340.0)
                                                .hint_text("https://..."),
                                        );
                                        if text_edit.lost_focus()
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                        {
                                            open_url = true;
                                        }

                                        if ui.button(format!("📋 {}", self.tr("Paste"))).clicked()
                                        {
                                            if let Some(text) = ui.input(|i| {
                                                i.raw.events.iter().find_map(|e| match e {
                                                    egui::Event::Paste(t) => Some(t.clone()),
                                                    _ => None,
                                                })
                                            }) {
                                                self.url_input_buffer = text;
                                            }
                                        }
                                    },
                                );

                                ui.add_space(10.0);
                                ui.with_layout(
                                    crate::ui::i18n::layout(self.rtl, egui::Align::Center),
                                    |ui| {
                                        if ui.button(self.tr("Open")).clicked() {
                                            open_url = true;
                                        }
                                        if ui.button(self.tr("Cancel")).clicked() {
                                            close_dialog = true;
                                        }
                                    },
                                );
                            });
                        });

                    if open_url {
                        let url = self.url_input_buffer.clone();
                        self.load_url(&url);
                        self.url_input_buffer.clear();
                        self.show_open_url_dialog = false;
                    } else if close_dialog {
                        self.show_open_url_dialog = false;
                    }
                }

                if self.show_shortcuts_dialog {
                    let language = self.language;
                    egui::Window::new(format!("⌨ {}", self.tr("Keyboard Shortcuts & Controls")))
                        .collapsible(false)
                        .resizable(true)
                        .default_size([460.0, 360.0])
                        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                        .open(&mut self.show_shortcuts_dialog)
                        .show(ui.ctx(), |ui| {
                            egui::Grid::new("shortcuts_grid")
                                .striped(true)
                                .spacing([20.0, 8.0])
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(crate::ui::i18n::tr(
                                            language, "Shortcut",
                                        ))
                                        .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(crate::ui::i18n::tr(
                                            language, "Action",
                                        ))
                                        .strong(),
                                    );
                                    ui.end_row();

                                    ui.label("Space");
                                    ui.label(crate::ui::i18n::tr(language, "Play / Pause video"));
                                    ui.end_row();
                                    ui.label("F");
                                    ui.label(crate::ui::i18n::tr(
                                        language,
                                        "Toggle Fullscreen mode",
                                    ));
                                    ui.end_row();
                                    ui.label("M");
                                    ui.label(crate::ui::i18n::tr(language, "Toggle Audio Mute"));
                                    ui.end_row();
                                    ui.label("← / →");
                                    ui.label(crate::ui::i18n::tr(language, "Seek -5s / +5s"));
                                    ui.end_row();
                                    ui.label("↑ / ↓");
                                    ui.label(crate::ui::i18n::tr(language, "Volume -5% / +5%"));
                                    ui.end_row();
                                    ui.label(".  or  ]");
                                    ui.label(crate::ui::i18n::tr(
                                        language,
                                        "Frame Step Forward (+1 frame)",
                                    ));
                                    ui.end_row();
                                    ui.label(",  or  [");
                                    ui.label(crate::ui::i18n::tr(
                                        language,
                                        "Frame Step Backward (-1 frame)",
                                    ));
                                    ui.end_row();
                                    ui.label("Mouse Wheel");
                                    ui.label(crate::ui::i18n::tr(
                                        language,
                                        "Adjust Volume on player/bar",
                                    ));
                                    ui.end_row();
                                    ui.label("Shift + Mouse Wheel");
                                    ui.label(crate::ui::i18n::tr(
                                        language,
                                        "Seek forward / backward",
                                    ));
                                    ui.end_row();
                                    ui.label("Double Click");
                                    ui.label(crate::ui::i18n::tr(
                                        language,
                                        "Toggle Fullscreen / Open Video",
                                    ));
                                    ui.end_row();
                                    ui.label("Right Click");
                                    ui.label(crate::ui::i18n::tr(
                                        language,
                                        "Open Player Context Menu",
                                    ));
                                    ui.end_row();
                                    ui.label("Drag & Drop");
                                    ui.label(crate::ui::i18n::tr(
                                        language,
                                        "Drop media file onto window to play",
                                    ));
                                    ui.end_row();
                                });
                        });
                }

                if self.show_about_dialog {
                    let app_name = self.app_name.clone();
                    let app_publisher = self.app_publisher.clone();
                    let app_copyright = self.app_copyright.clone();
                    egui::Window::new(format!("ℹ {} {}", self.tr("About"), self.app_name))
                        .collapsible(false)
                        .resizable(false)
                        .default_size([380.0, 240.0])
                        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                        .open(&mut self.show_about_dialog)
                        .show(ui.ctx(), |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(8.0);
                                ui.heading(format!("▣ {app_name} v{}", env!("CARGO_PKG_VERSION")));
                                if let Some(publisher) = &app_publisher {
                                    ui.label(publisher);
                                }
                                if let Some(copyright) = &app_copyright {
                                    ui.label(egui::RichText::new(copyright).small().weak());
                                }
                            });
                        });
                }
            });
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.save_config();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(hwnd) = self.window_handle {
            let _ = crate::platform::windows::remove_system_tray_icon(hwnd);
        } else {
            let hwnd = crate::platform::windows::get_registered_hwnd();
            if hwnd != 0 {
                let _ = crate::platform::windows::remove_system_tray_icon(hwnd);
            }
        }
        self.save_config();
    }
}

impl PealayerApp {
    /// Ensures Windows Shell components (thumbnail toolbar and system tray icon)
    /// are initialized once a valid window handle is registered.
    pub fn ensure_shell_initialized(&mut self) {
        if !self.shell_initialized {
            let hwnd = self
                .window_handle
                .unwrap_or_else(crate::platform::windows::get_registered_hwnd);
            if hwnd != 0 {
                let _ = crate::platform::windows::init_taskbar_thumbnail_toolbar(hwnd);
                let _ = crate::platform::windows::register_system_tray_icon(hwnd, "Pealayer");
                self.shell_initialized = true;
            }
        }
    }

    /// Synchronizes playback time, duration, pause state, and error condition
    /// with the Windows taskbar progress state and thumbnail toolbar buttons.
    pub fn update_shell_state(&self) {
        crate::platform::windows::update_windows_taskbar_state_ext(
            self.playback_time,
            self.duration,
            self.is_paused,
            self.show_error.is_some(),
        );
        let hwnd = self
            .window_handle
            .unwrap_or_else(crate::platform::windows::get_registered_hwnd);
        if hwnd != 0 {
            let _ = crate::platform::windows::update_taskbar_thumbnail_buttons(
                hwnd,
                self.is_paused,
                self.current_video_path.is_some(),
            );
        }
    }

    /// Replaces the controller-advertised hardware snapshot used by the UI and
    /// authoring engine. Embedded transports can feed the same authoritative
    /// snapshot without reaching into the engine implementation.
    pub fn update_hardware_capabilities(
        &self,
        capabilities: Option<crate::four_d::controller::HardwareCapabilities>,
    ) {
        if let Ok(mut current) = self.engine_handle.hardware_capabilities.lock() {
            *current = capabilities;
        }
    }

    pub fn advertised_hardware(&self) -> Option<crate::four_d::controller::HardwareCapabilities> {
        self.engine_handle
            .hardware_capabilities
            .lock()
            .ok()
            .and_then(|capabilities| capabilities.clone())
    }

    pub fn advertised_effect_presets(&self) -> Vec<EffectPreset> {
        let Some(capabilities) = self
            .advertised_hardware()
            .filter(|capabilities| capabilities.board_connected)
        else {
            return Vec::new();
        };

        capabilities
            .macros
            .iter()
            .map(controller_macro_effect_preset)
            .collect()
    }

    fn apply_interop_command(
        &mut self,
        ctx: &egui::Context,
        command: crate::platform::interop::InteropCommand,
        source: &str,
    ) {
        use crate::platform::interop::InteropCommand;

        match command {
            InteropCommand::Launch { request } => {
                if let Some(value) = request.volume {
                    let _ = self.mpv.set_property("volume", value);
                    self.volume = value;
                    self.save_config();
                }
                if let Some(target) = request.target {
                    if target.starts_with("http://") || target.starts_with("https://") {
                        self.load_url(&target);
                    } else {
                        let path = std::path::PathBuf::from(target);
                        let resolved = if path.is_relative() {
                            request
                                .sender_working_directory
                                .as_deref()
                                .map(std::path::Path::new)
                                .map(|directory| directory.join(&path))
                                .unwrap_or(path)
                        } else {
                            path
                        };
                        self.load_video_file(resolved);
                    }
                }
                if request.fullscreen {
                    self.set_fullscreen(ctx, true);
                }
                if request.activate {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
            }
            InteropCommand::Play => self.play(),
            InteropCommand::Pause => self.pause(),
            InteropCommand::TogglePause => self.toggle_playback(),
            InteropCommand::Seek { seconds } => self.seek_relative(seconds),
            InteropCommand::SeekAbs { percentage } => {
                let clamped = percentage.clamp(0.0, 100.0);
                let target = self.duration * (clamped / 100.0);
                self.scrub_to(target);
                self.finish_scrub(target);
            }
            InteropCommand::SetVolume { value } => {
                let clamped = value.clamp(0.0, 130.0);
                let _ = self.mpv.set_property("volume", clamped);
                self.volume = clamped;
                self.save_config();
            }
            InteropCommand::Open { target } => {
                if target.starts_with("http://") || target.starts_with("https://") {
                    self.load_url(&target);
                } else {
                    self.load_video_file(std::path::PathBuf::from(&target));
                }
            }
            InteropCommand::SetFullscreen { enabled } => self.set_fullscreen(ctx, enabled),
            InteropCommand::ToggleFullscreen => self.toggle_fullscreen(ctx),
            InteropCommand::SetWorkspace { nle } => {
                let observed = ctx.input(|input| input.viewport().fullscreen.unwrap_or(false));
                self.apply_workspace_request(nle, observed);
            }
            InteropCommand::GetStatus => {}
        }
        self.set_osd(format!("{source}: command applied"));
    }

    /// Polls and processes all pending MPV events and updates application state.
    pub fn process_events(&mut self) {
        use libmpv2::events::{Event, PropertyData};

        loop {
            match self.mpv_client.wait_event(0.0) {
                Some(Ok(Event::PropertyChange {
                    reply_userdata,
                    change,
                    ..
                })) => match (reply_userdata, change) {
                    (1, PropertyData::Double(v)) => {
                        if !self.is_scrubbing {
                            self.playback_time = v;
                            self.engine_handle
                                .playback_time_ms
                                .store((v * 1000.0) as u64, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    (2, PropertyData::Double(v)) => self.duration = v,
                    (3, PropertyData::Flag(v)) => {
                        let prev_paused = self.is_paused;
                        self.is_paused = v;
                        self.engine_handle
                            .is_playing
                            .store(!v, std::sync::atomic::Ordering::Relaxed);
                        if self.current_video_path.is_some() && prev_paused != v {
                            self.set_osd(if v {
                                "Pause".to_string()
                            } else {
                                "Play".to_string()
                            });
                        }
                    }
                    (4, PropertyData::Double(v)) => {
                        let prev_vol = self.volume;
                        self.volume = v;
                        if self.current_video_path.is_some() && (prev_vol - v).abs() > 0.01 {
                            self.set_osd(format!("Volume: {}%", v as i32));
                        }
                    }
                    (5, PropertyData::Flag(v)) => {
                        let prev_muted = self.is_muted;
                        self.is_muted = v;
                        if self.current_video_path.is_some() && prev_muted != v {
                            self.set_osd(if v {
                                "Mute: On".to_string()
                            } else {
                                "Mute: Off".to_string()
                            });
                        }
                    }
                    (6, PropertyData::Flag(v)) => self.sub_visibility = v,
                    (7, PropertyData::Double(v)) => self.sub_font_size = v,
                    (8, PropertyData::Double(v)) => self.sub_delay = v,
                    (9, PropertyData::Str(v)) => self.current_sid = v.to_string(),
                    (9, PropertyData::OsdStr(v)) => self.current_sid = v.to_string(),
                    (10, PropertyData::Double(v)) => self.audio_delay = v,
                    (11, PropertyData::Str(v)) => self.current_aid = v.to_string(),
                    (11, PropertyData::OsdStr(v)) => self.current_aid = v.to_string(),
                    (12, PropertyData::Flag(v)) => {
                        self.is_eof = v;
                    }
                    (13, PropertyData::Double(v)) => {
                        self.media_fps = v;
                    }
                    _ => {}
                },
                Some(Ok(Event::EndFile(reason))) => {
                    if reason == 4 {
                        // MPV_END_FILE_REASON_ERROR
                        self.show_error =
                            Some("Error: Failed to play the selected file.".to_string());
                    }
                }
                Some(Ok(Event::Seek)) => {
                    if !self.is_scrubbing {
                        self.seek_pos = None;
                    }
                    let current_pos_ms =
                        (self.seek_pos.unwrap_or(self.playback_time) * 1000.0) as u64;
                    let _ = self
                        .engine_handle
                        .sender
                        .send(crate::four_d::engine::EngineMessage::Seek(current_pos_ms));
                }
                Some(Ok(Event::StartFile)) => {
                    self.show_error = None;
                    self.is_eof = false;
                    self.refresh_sub_tracks();
                    self.refresh_audio_tracks();
                }
                Some(Ok(_)) => {}
                _ => break,
            }
        }
        self.update_shell_state();
    }

    /// Returns true if video playback has finished (at EOF or at duration limit while paused).
    pub fn is_playback_finished(&self) -> bool {
        self.current_video_path.is_some()
            && (self.is_eof
                || (self.duration > 0.0 && self.playback_time >= self.duration && self.is_paused))
    }

    /// Restarts playback of the current video from the beginning (0.0s).
    pub fn replay(&mut self) {
        if self.current_video_path.is_none() {
            return;
        }
        let _ = self.mpv.command("seek", &["0", "absolute+exact"]);
        let _ = self.mpv.set_property("pause", false);
        self.is_paused = false;
        self.is_eof = false;
        self.playback_time = 0.0;
        self.seek_pos = None;
        self.engine_handle
            .is_playing
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.engine_handle
            .playback_time_ms
            .store(0, std::sync::atomic::Ordering::Relaxed);
        let _ = self
            .engine_handle
            .sender
            .send(crate::four_d::engine::EngineMessage::Seek(0));
        self.set_osd("Replay".to_string());
    }

    /// Resumes playback, or restarts if playback finished.
    pub fn play(&mut self) {
        if self.current_video_path.is_none() {
            return;
        }
        if self.is_playback_finished() {
            self.replay();
        } else {
            let _ = self.mpv.set_property("pause", false);
            self.is_paused = false;
            self.engine_handle
                .is_playing
                .store(true, std::sync::atomic::Ordering::Relaxed);
            self.set_osd("Play".to_string());
        }
    }

    /// Pauses playback.
    pub fn pause(&mut self) {
        if self.current_video_path.is_none() {
            return;
        }
        let _ = self.mpv.set_property("pause", true);
        self.is_paused = true;
        self.engine_handle
            .is_playing
            .store(false, std::sync::atomic::Ordering::Relaxed);
        self.set_osd("Pause".to_string());
    }

    /// Toggles play/pause, restarting if playback has reached the end.
    pub fn toggle_playback(&mut self) {
        if self.current_video_path.is_none() {
            return;
        }
        if self.is_playback_finished() {
            self.replay();
        } else if self.is_paused {
            self.play();
        } else {
            self.pause();
        }
    }

    pub fn set_fullscreen(&mut self, ctx: &egui::Context, enabled: bool) {
        let observed = ctx.input(|input| input.viewport().fullscreen.unwrap_or(false));
        self.request_fullscreen(ctx, enabled, observed);
    }

    pub fn toggle_fullscreen(&mut self, ctx: &egui::Context) {
        let observed = ctx.input(|input| input.viewport().fullscreen.unwrap_or(false));
        let intended = self.desired_fullscreen.unwrap_or(observed);
        self.request_fullscreen(ctx, !intended, observed);
    }

    pub fn fullscreen_intent(&self, ctx: &egui::Context) -> bool {
        self.desired_fullscreen.unwrap_or_else(|| {
            ctx.input(|input| input.viewport().fullscreen.unwrap_or(false))
        })
    }

    fn apply_workspace_request(&mut self, nle: bool, observed_fullscreen: bool) {
        if observed_fullscreen || self.was_fullscreen || self.desired_fullscreen == Some(true) {
            self.workspace_before_fullscreen = Some(nle);
            self.show_four_d_editor = false;
        } else {
            self.show_four_d_editor = nle;
        }
    }

    fn request_fullscreen(&mut self, ctx: &egui::Context, enabled: bool, observed: bool) {
        if enabled {
            if !self.was_fullscreen && self.workspace_before_fullscreen.is_none() {
                self.workspace_before_fullscreen = Some(self.show_four_d_editor);
            }
            self.show_four_d_editor = false;
        } else if !self.was_fullscreen && !observed {
            // The OS may not have observed a rapid ON request yet. Cancelling
            // that pending entry must restore the workspace staged above.
            if let Some(previous_workspace) = self.workspace_before_fullscreen.take() {
                self.show_four_d_editor = previous_workspace;
            }
        }
        self.desired_fullscreen = (enabled != observed).then_some(enabled);
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(enabled));
        self.set_osd(if enabled {
            self.tr("Enter fullscreen")
        } else {
            self.tr("Exit fullscreen")
        });
    }

    fn observe_fullscreen_state(&mut self, is_fullscreen: bool) {
        if is_fullscreen && !self.was_fullscreen {
            // set_fullscreen() may already have staged the pre-fullscreen
            // workspace before the viewport reports its new state.
            if self.workspace_before_fullscreen.is_none() {
                self.workspace_before_fullscreen = Some(self.show_four_d_editor);
            }
            self.show_four_d_editor = false;
        } else if !is_fullscreen && self.was_fullscreen {
            if let Some(previous_workspace) = self.workspace_before_fullscreen.take() {
                self.show_four_d_editor = previous_workspace;
            }
        }
        self.was_fullscreen = is_fullscreen;
        if self.desired_fullscreen == Some(is_fullscreen) {
            self.desired_fullscreen = None;
        }
    }

    /// Performs an exact relative seek by the given number of seconds.
    pub fn seek_relative(&mut self, seconds: f64) {
        if self.current_video_path.is_none() {
            return;
        }
        let sec_str = seconds.to_string();
        let _ = self.mpv.command("seek", &[&sec_str, "relative+exact"]);
        if seconds < 0.0 {
            self.is_eof = false;
        }
        let target = (self.playback_time + seconds).clamp(0.0, self.duration.max(0.0));
        self.seek_pos = Some(target);
        let sign = if seconds > 0.0 { "+" } else { "" };
        self.set_osd(format!("Seek: {}{:.0}s", sign, seconds));
    }

    /// Begins or updates an active scrub session.
    /// Sets seek_pos, pauses playback smoothly during drag, and dispatches a non-blocking
    /// preview seek to the background worker thread with latest-target coalescing.
    pub fn scrub_to(&mut self, target_time: f64) {
        let clamped = if self.duration > 0.0 {
            target_time.clamp(0.0, self.duration)
        } else {
            target_time.max(0.0)
        };

        if !self.is_scrubbing {
            self.is_scrubbing = true;
            self.was_playing_before_scrub = !self.is_paused
                && self.current_video_path.is_some()
                && !self.is_playback_finished();
            if self.was_playing_before_scrub {
                let _ = self.mpv.set_property("pause", true);
            }
            self.commit_recorded_samples();
        }

        if clamped < self.duration {
            self.is_eof = false;
        }

        self.seek_pos = Some(clamped);
        self.seek_controller.request_scrub(clamped);
    }

    /// Ends an active scrub session. Dispatches a final exact commit seek and
    /// restores playback if the video was playing prior to scrubbing.
    pub fn finish_scrub(&mut self, target_time: f64) {
        let clamped = if self.duration > 0.0 {
            target_time.clamp(0.0, self.duration)
        } else {
            target_time.max(0.0)
        };

        if clamped < self.duration {
            self.is_eof = false;
        }

        self.seek_pos = Some(clamped);
        self.seek_controller.request_commit(clamped);
        self.commit_recorded_samples();

        if self.was_playing_before_scrub {
            let _ = self.mpv.set_property("pause", false);
            self.was_playing_before_scrub = false;
        }
        self.is_scrubbing = false;
    }

    pub(crate) fn refresh_sub_tracks(&mut self) {
        self.sub_tracks.clear();
        if let Ok(count) = self.mpv.get_property::<i64>("track-list/count") {
            for i in 0..count {
                let track_type_prop = format!("track-list/{}/type", i);
                if let Ok(track_type) = self.mpv.get_property::<String>(&track_type_prop) {
                    if track_type == "sub" {
                        let id_prop = format!("track-list/{}/id", i);
                        let lang_prop = format!("track-list/{}/lang", i);
                        let title_prop = format!("track-list/{}/title", i);

                        if let Ok(id) = self.mpv.get_property::<i64>(&id_prop) {
                            let lang = self.mpv.get_property::<String>(&lang_prop).ok();
                            let title = self.mpv.get_property::<String>(&title_prop).ok();

                            self.sub_tracks.push(SubtitleTrack { id, title, lang });
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn refresh_audio_tracks(&mut self) {
        self.audio_tracks.clear();
        if let Ok(count) = self.mpv.get_property::<i64>("track-list/count") {
            for i in 0..count {
                let track_type_prop = format!("track-list/{}/type", i);
                if let Ok(track_type) = self.mpv.get_property::<String>(&track_type_prop) {
                    if track_type == "audio" {
                        let id_prop = format!("track-list/{}/id", i);
                        let lang_prop = format!("track-list/{}/lang", i);
                        let title_prop = format!("track-list/{}/title", i);

                        if let Ok(id) = self.mpv.get_property::<i64>(&id_prop) {
                            let lang = self.mpv.get_property::<String>(&lang_prop).ok();
                            let title = self.mpv.get_property::<String>(&title_prop).ok();

                            self.audio_tracks.push(AudioTrack { id, title, lang });
                        }
                    }
                }
            }
        }
    }

    pub fn load_video_file(&mut self, path: std::path::PathBuf) {
        let path_str = path.to_str().unwrap_or("");
        if !path_str.is_empty() {
            let _ = self.mpv.set_property("keep-open", "always");
            let _ = self.mpv.command("loadfile", &[path_str, "replace"]);
            self.current_video_path = Some(path.clone());
            self.is_eof = false;
            self.is_paused = false;
            self.playback_time = 0.0;
            self.seek_pos = None;
            self.add_recent_media(path.clone());
            let title = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path_str);
            if let Some(ref mut mc) = self.media_controls {
                mc.update_metadata(Some(title));
            }
            self.set_osd(format!("Loaded: {}", title));

            // Auto-load matching sidecar timeline
            let mut sidecar = path.clone();
            sidecar.set_extension("4d.json");
            if !sidecar.exists() {
                sidecar.set_extension("json");
            }
            if sidecar.exists() {
                if let Ok(timeline) = crate::four_d::models::Timeline::load_from_file(&sidecar) {
                    self.timeline = timeline;
                    self.sync_timeline_engine();
                }
            }
        }
    }

    pub fn load_dropped_files(&mut self, paths: Vec<std::path::PathBuf>) {
        let mut first_media = true;
        let mut subtitle_added = false;
        let mut queued_media = 0usize;

        // Load/queue media before attaching subtitles so a mixed Windows drop
        // reliably attaches subtitle files to the newly selected item.
        for path in paths
            .iter()
            .filter(|path| dropped_file_kind(path) == DroppedFileKind::Media)
        {
            if first_media {
                self.load_video_file(path.clone());
                first_media = false;
            } else if let Some(path_str) = path.to_str() {
                if self
                    .mpv
                    .command("loadfile", &[path_str, "append-play"])
                    .is_ok()
                {
                    self.add_recent_media(path.clone());
                    queued_media += 1;
                }
            }
        }

        for path in paths
            .iter()
            .filter(|path| dropped_file_kind(path) == DroppedFileKind::Subtitle)
        {
            if let Some(path_str) = path.to_str() {
                if self.mpv.command("sub-add", &[path_str]).is_ok() {
                    subtitle_added = true;
                }
            }
        }
        if subtitle_added {
            self.refresh_sub_tracks();
        }

        for path in paths
            .iter()
            .filter(|path| dropped_file_kind(path) == DroppedFileKind::Timeline)
        {
            match crate::four_d::models::Timeline::load_from_file(path) {
                Ok(timeline) => {
                    self.timeline = timeline;
                    self.sync_timeline_engine();
                }
                Err(error) => {
                    self.show_error = Some(format!(
                        "Failed to load dropped timeline {}: {error}",
                        path.display()
                    ));
                }
            }
        }

        if queued_media > 0 {
            self.set_osd(format!("Queued {queued_media} additional media file(s)"));
        } else if subtitle_added {
            self.set_osd("External subtitle loaded".to_string());
        }
    }

    pub fn load_url(&mut self, url: &str) {
        let trimmed = url.trim();
        if !trimmed.is_empty() {
            let _ = self.mpv.set_property("keep-open", "always");
            let _ = self.mpv.command("loadfile", &[trimmed, "replace"]);
            let path = std::path::PathBuf::from(trimmed);
            self.current_video_path = Some(path.clone());
            self.is_eof = false;
            self.is_paused = false;
            self.playback_time = 0.0;
            self.seek_pos = None;
            self.add_recent_media(path);
            if let Some(ref mut mc) = self.media_controls {
                mc.update_metadata(Some(trimmed));
            }
            self.set_osd(format!("Loading URL: {}", trimmed));
        }
    }

    pub fn close_video(&mut self) {
        let _ = self.mpv.command("stop", &[]);
        self.current_video_path = None;
        self.playback_time = 0.0;
        self.duration = 0.0;
        self.is_eof = false;
        self.is_paused = false;
        self.seek_pos = None;
        if let Some(ref mut mc) = self.media_controls {
            mc.update_metadata(None);
            mc.update_playback(false, 0.0, 0.0);
        }
        self.set_osd("Video Closed".to_string());
    }

    pub fn save_config(&self) {
        // Preserve deployment-owned branding while saving mutable player
        // preferences through one typed configuration contract.
        let mut cfg = crate::config::AppConfig::load();
        cfg.volume = self.volume;
        cfg.is_muted = self.is_muted;
        cfg.pin_controls = self.pin_controls;
        cfg.show_remaining_time = self.show_remaining_time;
        cfg.recent_media = self.recent_media.clone();
        cfg.language = self.language_preference;
        cfg.direction = self.direction_preference;
        cfg.theme = self.theme_preference;
        cfg.hardware_endpoint = (!self.serial_port.trim().is_empty())
            .then(|| self.serial_port.clone());
        cfg.auto_connect_hardware = self.auto_connect_hardware;
        cfg.pause_on_hardware_disconnect = self.pause_on_hardware_disconnect;
        cfg.click_player_to_toggle = self.click_player_to_toggle;
        cfg.show_subseconds = self.show_subseconds;
        cfg.wheel_seek_seconds = self.wheel_seek_seconds;
        cfg.osd_position = self.osd_position;
        cfg.osd_timeout_seconds = self.osd_timeout_seconds;
        cfg.paused_drag_action = self.paused_drag_action;
        cfg.playing_drag_action = self.playing_drag_action;
        cfg.save();
    }

    pub(crate) fn tr(&self, english: &'static str) -> String {
        crate::ui::i18n::tr(self.language, english)
    }

    pub(crate) fn display_text(&self, logical: &str) -> String {
        crate::ui::i18n::visual_text(self.language, logical)
    }

    pub(crate) fn set_language(
        &mut self,
        ctx: &egui::Context,
        preference: crate::config::AppLanguage,
    ) {
        self.language_preference = preference;
        self.language = crate::config::resolve_language(preference);
        self.rtl = crate::config::resolve_rtl(self.direction_preference, self.language);
        crate::ui::i18n::configure_ui_fonts(
            ctx,
            self.language == crate::config::AppLanguage::Persian,
        );
        self.save_config();
    }

    pub(crate) fn set_direction(&mut self, preference: crate::config::AppDirection) {
        self.direction_preference = preference;
        self.rtl = crate::config::resolve_rtl(preference, self.language);
        self.save_config();
    }

    pub(crate) fn set_theme(&mut self, ctx: &egui::Context, theme: crate::config::AppTheme) {
        self.theme_preference = theme;
        ctx.set_theme(match theme {
            crate::config::AppTheme::System => egui::ThemePreference::System,
            crate::config::AppTheme::Light => egui::ThemePreference::Light,
            crate::config::AppTheme::Dark => egui::ThemePreference::Dark,
        });
        crate::platform::windows::set_window_theme(ctx.global_style().visuals.dark_mode);
        self.save_config();
    }

    pub fn add_recent_media(&mut self, path: std::path::PathBuf) {
        self.recent_media.retain(|p| p != &path);
        self.recent_media.insert(0, path);
        if self.recent_media.len() > 10 {
            self.recent_media.truncate(10);
        }
        crate::platform::windows::sync_windows_jump_list(&self.recent_media);
        self.save_config();
    }

    pub fn clear_recent_media(&mut self) {
        self.recent_media.clear();
        crate::platform::windows::sync_windows_jump_list(&[]);
        self.save_config();
    }

    pub fn set_osd(&mut self, msg: String) {
        self.osd_message = Some((msg, std::time::Instant::now()));
    }

    pub(crate) fn commit_recorded_samples(&mut self) -> bool {
        let mut changed = false;
        for track in self.timeline.analog_tracks.iter_mut() {
            if self.recording_session.sample_count(track.id) > 0 {
                self.recording_session.commit_to_track(
                    track,
                    0.015,
                    crate::four_d::curve::Interpolation::Smooth,
                );
                changed = true;
            }
        }
        if changed {
            let _ = self.engine_handle.sender.send(
                crate::four_d::engine::EngineMessage::UpdateAnalogTracks(
                    self.timeline.analog_tracks.clone(),
                ),
            );
        }
        changed
    }

    pub fn snapshot_timeline(&self) -> crate::four_d::history::TimelineSnapshot {
        crate::four_d::history::TimelineSnapshot {
            instances: self.timeline.instances.clone(),
            analog_tracks: self.timeline.analog_tracks.clone(),
            templates: self.timeline.templates.clone(),
        }
    }

    /// Rebuilds every hardware lane from the authoritative project timeline.
    /// Keeping relay edges and controller-owned macro cues together prevents
    /// load, undo, delete, and drag operations from updating only one lane.
    pub fn sync_timeline_engine(&self) {
        let relays = crate::four_d::engine::compile_timeline(
            &self.timeline,
            &self.track_muted,
            &self.track_soloed,
        );
        let macros = crate::four_d::engine::compile_controller_macros(&self.timeline);
        let _ = self
            .engine_handle
            .sender
            .send(crate::four_d::engine::EngineMessage::UpdateQueue(relays));
        let _ = self.engine_handle.sender.send(
            crate::four_d::engine::EngineMessage::UpdateControllerMacros(macros),
        );
    }

    pub fn restore_timeline_snapshot(
        &mut self,
        snapshot: crate::four_d::history::TimelineSnapshot,
    ) {
        self.timeline.instances = snapshot.instances;
        self.timeline.analog_tracks = snapshot.analog_tracks;
        self.timeline.templates = snapshot.templates;
        let _ = self.engine_handle.sender.send(
            crate::four_d::engine::EngineMessage::UpdateAnalogTracks(
                self.timeline.analog_tracks.clone(),
            ),
        );
        self.sync_timeline_engine();
    }

    pub fn isolate_template_for_instance(&mut self, instance_id: uuid::Uuid) -> Option<uuid::Uuid> {
        let (target_effect_id, is_shared) = {
            let instance = self
                .timeline
                .instances
                .iter()
                .find(|i| i.id == instance_id)?;
            let effect_id = instance.effect_id;
            let count = self
                .timeline
                .instances
                .iter()
                .filter(|i| i.effect_id == effect_id)
                .count();
            (effect_id, count > 1)
        };

        if !is_shared {
            return if self
                .timeline
                .templates
                .iter()
                .any(|t| t.id == target_effect_id)
            {
                Some(target_effect_id)
            } else {
                None
            };
        }

        let template = self
            .timeline
            .templates
            .iter()
            .find(|t| t.id == target_effect_id)?;
        let mut new_template = template.clone();
        let new_id = uuid::Uuid::new_v4();
        new_template.id = new_id;
        self.timeline.templates.push(new_template);

        if let Some(inst) = self
            .timeline
            .instances
            .iter_mut()
            .find(|i| i.id == instance_id)
        {
            inst.effect_id = new_id;
        }

        Some(new_id)
    }
}

fn controller_macro_effect_preset(
    hardware_macro: &crate::four_d::controller::HardwareMacro,
) -> EffectPreset {
    EffectPreset {
        category: hardware_macro.category.clone(),
        effect: crate::four_d::models::Effect::controller_macro(
            hardware_macro.name.clone(),
            String::new(),
            hardware_macro.duration_ms,
            hardware_macro.id,
            hardware_macro.mode.clone(),
        ),
    }
}

fn get_shared_mpv() -> &'static libmpv2::Mpv {
    static GLOBAL_MPV: std::sync::OnceLock<libmpv2::Mpv> = std::sync::OnceLock::new();
    GLOBAL_MPV.get_or_init(|| {
        libmpv2::Mpv::with_initializer(|init| {
            let _ = init.set_option("vo", "null");
            let _ = init.set_option("ao", "null");
            let _ = init.set_option("keep-open", "always");
            Ok(())
        })
        .expect("Failed to initialize mpv")
    })
}

impl Default for PealayerApp {
    fn default() -> Self {
        let mpv = get_shared_mpv();
        let _ = mpv.set_property("keep-open", "always");
        let mpv_client = mpv
            .create_client(None)
            .expect("Failed to create mpv client");
        let _ = mpv_client.observe_property("time-pos", libmpv2::Format::Double, 1);
        let _ = mpv_client.observe_property("duration", libmpv2::Format::Double, 2);
        let _ = mpv_client.observe_property("pause", libmpv2::Format::Flag, 3);
        let _ = mpv_client.observe_property("volume", libmpv2::Format::Double, 4);
        let _ = mpv_client.observe_property("mute", libmpv2::Format::Flag, 5);
        let _ = mpv_client.observe_property("sub-visibility", libmpv2::Format::Flag, 6);
        let _ = mpv_client.observe_property("sub-font-size", libmpv2::Format::Double, 7);
        let _ = mpv_client.observe_property("sub-delay", libmpv2::Format::Double, 8);
        let _ = mpv_client.observe_property("sid", libmpv2::Format::String, 9);
        let _ = mpv_client.observe_property("audio-delay", libmpv2::Format::Double, 10);
        let _ = mpv_client.observe_property("aid", libmpv2::Format::String, 11);
        let _ = mpv_client.observe_property("eof-reached", libmpv2::Format::Flag, 12);
        let _ = mpv_client.observe_property("estimated-vf-fps", libmpv2::Format::Double, 13);
        let (interop_tx, interop_rx) = std::sync::mpsc::channel();
        let (_controller_cmd_tx, controller_cmd_rx) =
            std::sync::mpsc::channel::<crate::platform::interop::ControllerDelivery>();
        let (web_state_tx, _web_state_rx) = std::sync::mpsc::channel();
        let (_web_cmd_tx, web_cmd_rx) = std::sync::mpsc::channel();

        Self {
            app_name: crate::config::resolved_app_name(&crate::config::AppConfig::default()),
            app_publisher: crate::config::resolved_app_publisher(
                &crate::config::AppConfig::default(),
            ),
            app_copyright: crate::config::resolved_app_copyright(
                &crate::config::AppConfig::default(),
            ),
            last_window_title: String::new(),
            language_preference: crate::config::AppLanguage::System,
            language: crate::config::resolve_language(crate::config::AppLanguage::System),
            direction_preference: crate::config::AppDirection::Auto,
            theme_preference: crate::config::AppTheme::System,
            rtl: crate::config::resolve_rtl(
                crate::config::AppDirection::Auto,
                crate::config::resolve_language(crate::config::AppLanguage::System),
            ),
            mpv,
            mpv_client,
            render_context: Arc::new(Mutex::new(None)),
            playback_time: 0.0,
            duration: 0.0,
            media_fps: 0.0,
            is_paused: false,
            is_eof: false,
            volume: 100.0,
            is_muted: false,
            seek_pos: None,
            seek_controller: crate::mpv::seek::SeekController::new(
                crate::mpv::seek::MpvSeekBackend::new(mpv),
            ),
            was_playing_before_scrub: false,
            is_scrubbing: false,
            last_mouse_activity: std::time::Instant::now(),
            pin_controls: false,
            show_error: None,
            show_sub_settings: false,
            sub_visibility: true,
            sub_font_size: 55.0,
            sub_delay: 0.0,
            current_sid: "no".to_string(),
            sub_tracks: Vec::new(),
            show_audio_settings: false,
            audio_delay: 0.0,
            current_aid: "no".to_string(),
            audio_tracks: Vec::new(),
            show_four_d_editor: true,
            dock_state: crate::ui::layout::create_initial_layout(),
            timeline: crate::four_d::models::Timeline::new(),
            engine_handle: crate::four_d::engine::spawn_engine(),
            recording_session: crate::four_d::curve_record::RecordingSession::new(),
            input_capture: crate::four_d::input_capture::InputCaptureState::new(),
            is_recording: false,
            selected_instance_ids: std::collections::HashSet::new(),
            selected_keyframes: std::collections::HashSet::new(),
            active_keyframe_drag: None,
            timeline_zoom: 100.0,
            undo_stack: crate::four_d::history::UndoStack::default(),
            recording_keys: std::collections::HashMap::new(),
            relay_overrides: std::collections::BTreeSet::new(),
            effects_search_query: String::new(),
            track_muted: std::collections::BTreeSet::new(),
            track_soloed: std::collections::BTreeSet::new(),
            track_locked: std::collections::BTreeSet::new(),
            active_drag: None,
            estop_active: false,
            serial_port: String::new(),
            is_connected: false,
            lasso_origin: None,
            lasso_rect: None,
            rtt_state: Arc::new(Mutex::new(RttState {
                video_texture: None,
                video_fbo: None,
                video_texture_id: None,
                texture_width: 1920,
                texture_height: 1080,
            })),
            current_video_path: None,
            show_remaining_time: false,
            osd_message: None,
            recent_media: Vec::new(),
            show_open_url_dialog: false,
            url_input_buffer: String::new(),
            is_window_operating: false,
            show_shortcuts_dialog: false,
            show_about_dialog: false,
            show_preferences_dialog: false,
            preferences_tab: 0,
            pause_on_hardware_disconnect: true,
            auto_connect_hardware: true,
            click_player_to_toggle: true,
            show_subseconds: true,
            wheel_seek_seconds: 5.0,
            osd_position: crate::config::OsdPosition::TopLeft,
            osd_timeout_seconds: 3.5,
            paused_drag_action: crate::config::PlayerDragAction::MoveWindow,
            playing_drag_action: crate::config::PlayerDragAction::TemporaryFastForward,
            was_hardware_connected: false,
            was_board_connected: false,
            connection_notice: None,
            workspace_before_fullscreen: None,
            was_fullscreen: false,
            desired_fullscreen: None,
            interop_rx,
            controller_cmd_rx,
            web_state_tx,
            web_cmd_rx,
            last_web_broadcast: None,
            media_controls: None,
            media_cmd_tx: interop_tx,
            window_handle: None,
            shell_initialized: false,
        }
    }
}

pub fn contextual_window_title(
    app_name: &str,
    media_path: Option<&std::path::Path>,
    connected: bool,
    connection_requested: bool,
) -> String {
    if let Some(media_name) = media_path.and_then(std::path::Path::file_name) {
        return format!("{} — {}", media_name.to_string_lossy(), app_name);
    }
    if connected {
        format!("{app_name} — Hardware connected")
    } else if connection_requested {
        format!("{app_name} — Connecting…")
    } else {
        app_name.to_string()
    }
}

fn hardware_connection_was_lost(
    was_transport_connected: bool,
    transport_connected: bool,
    was_board_connected: bool,
    board_connected: bool,
) -> bool {
    (was_transport_connected && !transport_connected)
        || (was_board_connected && !board_connected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{
        DroppedFileKind, contextual_window_title, controller_macro_effect_preset, dropped_file_kind,
    };

    #[test]
    fn test_recent_media_deduplication_and_cap() {
        let mut list: Vec<std::path::PathBuf> = Vec::new();
        let add = |l: &mut Vec<std::path::PathBuf>, p: std::path::PathBuf| {
            l.retain(|item| item != &p);
            l.insert(0, p);
            if l.len() > 10 {
                l.truncate(10);
            }
        };

        for i in 0..15 {
            add(
                &mut list,
                std::path::PathBuf::from(format!("/video{}.mp4", i)),
            );
        }

        assert_eq!(list.len(), 10);
        assert_eq!(list[0], std::path::PathBuf::from("/video14.mp4"));

        // Re-adding /video5.mp4 moves it to front
        add(&mut list, std::path::PathBuf::from("/video5.mp4"));
        assert_eq!(list.len(), 10);
        assert_eq!(list[0], std::path::PathBuf::from("/video5.mp4"));
    }

    #[test]
    fn test_url_formatting_and_trim() {
        let raw_url = "   https://example.com/stream.m3u8   ";
        let trimmed = raw_url.trim();
        assert_eq!(trimmed, "https://example.com/stream.m3u8");
        let path = std::path::PathBuf::from(trimmed);
        assert_eq!(path.to_str().unwrap(), "https://example.com/stream.m3u8");
    }

    #[test]
    fn test_window_operating_flag_state() {
        let pointer_down = true;
        let active_drag = true;
        let is_operating = pointer_down && active_drag;
        assert!(is_operating);
    }

    struct DummyStorage;
    impl eframe::Storage for DummyStorage {
        fn get_string(&self, _key: &str) -> Option<String> {
            None
        }
        fn set_string(&mut self, _key: &str, _value: String) {}
        fn flush(&mut self) {}
    }

    #[test]
    fn test_lifecycle_hooks_invoke_save_config() {
        use eframe::App;
        let mut app = PealayerApp::default();
        app.volume = 95.0;
        let mut storage = DummyStorage;
        app.save(&mut storage);
        app.on_exit(None);
    }
    #[test]
    fn window_title_reflects_media_and_connection_context() {
        assert_eq!(
            contextual_window_title("Studio", None, false, false),
            "Studio"
        );
        assert_eq!(
            contextual_window_title("Studio", None, false, true),
            "Studio — Connecting…"
        );
        assert_eq!(
            contextual_window_title("Studio", None, true, true),
            "Studio — Hardware connected"
        );
        assert_eq!(
            contextual_window_title(
                "Studio",
                Some(std::path::Path::new("C:/media/demo.mp4")),
                true,
                true,
            ),
            "demo.mp4 — Studio"
        );
    }

    #[test]
    fn board_loss_is_detected_behind_a_healthy_controller_transport() {
        assert!(hardware_connection_was_lost(true, true, true, false));
        assert!(hardware_connection_was_lost(true, false, true, false));
        assert!(!hardware_connection_was_lost(true, true, false, false));
        assert!(!hardware_connection_was_lost(false, true, false, true));
    }

    #[test]
    fn fullscreen_startup_uses_an_entry_transition() {
        let app = PealayerApp::default();
        assert!(!app.was_fullscreen);
        assert!(app.show_four_d_editor);
    }

    #[test]
    fn rpc_fullscreen_round_trip_restores_the_staged_nle_workspace() {
        let mut app = PealayerApp::default();
        let ctx = egui::Context::default();
        app.show_four_d_editor = true;

        // The RPC request stages NLE before the OS reports the viewport change.
        app.set_fullscreen(&ctx, true);
        assert!(!app.show_four_d_editor);
        assert_eq!(app.workspace_before_fullscreen, Some(true));

        // Observing entry must not replace that target with forced Simple mode.
        app.observe_fullscreen_state(true);
        assert_eq!(app.workspace_before_fullscreen, Some(true));
        assert!(!app.show_four_d_editor);

        app.set_fullscreen(&ctx, false);
        app.observe_fullscreen_state(false);
        assert!(app.show_four_d_editor);
        assert_eq!(app.workspace_before_fullscreen, None);
    }

    #[test]
    fn rapid_remote_fullscreen_on_off_restores_nle_before_viewport_entry() {
        let mut app = PealayerApp::default();
        let ctx = egui::Context::default();
        app.show_four_d_editor = true;

        app.set_fullscreen(&ctx, true);
        app.set_fullscreen(&ctx, false);

        assert!(app.show_four_d_editor);
        assert_eq!(app.workspace_before_fullscreen, None);
        assert!(!app.was_fullscreen);

        // If the delayed ON reaches the viewport despite cancellation, the
        // observed fullscreen frame still forces Simple and exits back to NLE.
        app.observe_fullscreen_state(true);
        assert!(!app.show_four_d_editor);
        assert_eq!(app.workspace_before_fullscreen, Some(true));
        app.observe_fullscreen_state(false);
        assert!(app.show_four_d_editor);
    }

    #[test]
    fn two_pending_fullscreen_toggles_cancel_each_other() {
        let mut app = PealayerApp::default();
        let ctx = egui::Context::default();
        app.show_four_d_editor = true;

        app.toggle_fullscreen(&ctx);
        assert_eq!(app.desired_fullscreen, Some(true));
        assert!(!app.show_four_d_editor);

        app.toggle_fullscreen(&ctx);
        assert_eq!(app.desired_fullscreen, None);
        assert!(app.show_four_d_editor);
        assert_eq!(app.workspace_before_fullscreen, None);
    }

    #[test]
    fn external_fullscreen_and_workspace_requests_preserve_simple_in_fullscreen() {
        let mut app = PealayerApp::default();
        app.show_four_d_editor = false;

        // Startup or an OS-native transition has no pending application intent.
        app.observe_fullscreen_state(true);
        assert_eq!(app.desired_fullscreen, None);
        assert!(!app.show_four_d_editor);
        assert_eq!(app.workspace_before_fullscreen, Some(false));

        // A remote workspace request changes the post-exit target, never the
        // fullscreen surface itself.
        app.apply_workspace_request(true, true);
        assert!(!app.show_four_d_editor);
        assert_eq!(app.workspace_before_fullscreen, Some(true));

        app.observe_fullscreen_state(false);
        assert!(app.show_four_d_editor);
        assert_eq!(app.workspace_before_fullscreen, None);
    }

    #[test]
    fn workspace_request_during_pending_fullscreen_updates_the_exit_target() {
        let mut app = PealayerApp::default();
        let ctx = egui::Context::default();
        app.show_four_d_editor = true;

        app.toggle_fullscreen(&ctx);
        assert_eq!(app.desired_fullscreen, Some(true));
        app.apply_workspace_request(false, false);
        assert!(!app.show_four_d_editor);
        assert_eq!(app.workspace_before_fullscreen, Some(false));

        app.observe_fullscreen_state(true);
        assert!(!app.show_four_d_editor);
        app.observe_fullscreen_state(false);
        assert!(!app.show_four_d_editor);
        assert_eq!(app.workspace_before_fullscreen, None);
    }

    #[test]
    fn dropped_files_are_routed_by_runtime_type() {
        assert_eq!(
            dropped_file_kind(std::path::Path::new("movie.MKV")),
            DroppedFileKind::Media
        );
        assert_eq!(
            dropped_file_kind(std::path::Path::new("captions.SRT")),
            DroppedFileKind::Subtitle
        );
        assert_eq!(
            dropped_file_kind(std::path::Path::new("show.4d.json")),
            DroppedFileKind::Timeline
        );
        assert_eq!(
            dropped_file_kind(std::path::Path::new("stream.m3u8")),
            DroppedFileKind::Media
        );
    }

    #[test]
    fn controller_relay_macro_remains_an_opaque_controller_cue() {
        let hardware_macro = crate::four_d::controller::HardwareMacro {
            id: 12,
            name: "Live Air Burst".to_string(),
            category: "Cinema".to_string(),
            mode: "mcu".to_string(),
            duration_ms: 250,
            steps: vec![
                crate::four_d::controller::HardwareMacroStep {
                    at_us: 0,
                    kind: "relay".to_string(),
                    target: Some(6),
                    value: Some(1),
                },
                crate::four_d::controller::HardwareMacroStep {
                    at_us: 250_000,
                    kind: "relays-off".to_string(),
                    target: None,
                    value: None,
                },
            ],
        };
        let preset = controller_macro_effect_preset(&hardware_macro);
        assert_eq!(preset.effect.name, "Live Air Burst");
        assert!(preset.effect.actions.is_empty());
        assert_eq!(preset.effect.target, crate::four_d::models::HardwareTarget::ControllerMacro);
        assert_eq!(preset.effect.controller_macro.as_ref().unwrap().id, 12);
        assert_eq!(preset.effect.duration_ms, 250);
    }

    #[test]
    fn non_relay_controller_macro_is_available_without_relay_flattening() {
        let hardware_macro = crate::four_d::controller::HardwareMacro {
            id: 7,
            name: "Display only".to_string(),
            category: "Display".to_string(),
            mode: "host".to_string(),
            duration_ms: 1,
            steps: vec![crate::four_d::controller::HardwareMacroStep {
                at_us: 0,
                kind: "display".to_string(),
                target: None,
                value: None,
            }],
        };
        let preset = controller_macro_effect_preset(&hardware_macro);
        assert_eq!(preset.effect.controller_macro.as_ref().unwrap().id, 7);
        assert!(preset.effect.actions.is_empty());
    }
}
