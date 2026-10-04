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
    pub group_icon: String,
    pub effect: crate::four_d::models::Effect,
    pub source: EffectPresetSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectPresetSource {
    ControllerMacro(u64),
    ControllerStrip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerEffectDraft {
    pub reference: String,
    pub id: String,
    pub name: String,
    pub category: String,
    pub icon: String,
    pub description: String,
    pub kind: String,
    pub program_json: String,
    pub color: String,
    pub default_fps: u8,
    pub duration_ms: u64,
    pub default_pixels: u16,
    pub engine: String,
    pub steps: Vec<crate::four_d::controller::HardwareMacroStep>,
    pub label: String,
    pub lcd_message: String,
    pub timing_tolerance_us: u32,
    pub keep_outputs_on_cancel: bool,
    pub board_profile_key: String,
    pub board_profile_mode: String,
    pub is_new: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerEffectGroupDraft {
    pub original_name: String,
    pub name: String,
    pub icon: String,
}

impl Default for ControllerEffectDraft {
    fn default() -> Self {
        Self {
            reference: String::new(),
            id: String::new(),
            name: String::new(),
            category: "Lighting".to_string(),
            icon: "sparkle".to_string(),
            description: String::new(),
            kind: "strip-stream".to_string(),
            program_json: String::new(),
            color: "green".to_string(),
            default_fps: 20,
            duration_ms: 5_000,
            default_pixels: 100,
            engine: "auto".to_string(),
            steps: Vec::new(),
            label: String::new(),
            lcd_message: String::new(),
            timing_tolerance_us: 0,
            keep_outputs_on_cancel: false,
            board_profile_key: String::new(),
            board_profile_mode: String::new(),
            is_new: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectDragPayload {
    pub name: String,
    pub icon: String,
    pub duration_ms: u64,
    pub target: crate::four_d::models::HardwareTarget,
    pub actions: Vec<crate::four_d::models::AtomicAction>,
    pub controller_macro: Option<crate::four_d::models::ControllerMacroCue>,
    pub controller_strip_effect: Option<crate::four_d::models::ControllerStripEffectCue>,
    pub controller_lane: Option<crate::four_d::models::ControllerEffectLane>,
}

#[derive(Debug, Default)]
pub struct HardwareEffectAuthoringState {
    pub name: String,
    pub active: bool,
    pub preview_active: bool,
    pub anchor_ms: u64,
    pub pending_operation: Option<String>,
    pub pending_saved_macro_id: Option<u64>,
    pub status: String,
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
    pub(crate) window_geometry: Option<crate::config::WindowGeometry>,
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
    pub is_seekable: bool,
    pub(crate) media_metadata_loaded: bool,
    pub(crate) cache_duration: Option<f64>,
    pub(crate) cache_buffering_percent: Option<f64>,
    pub(crate) media_fps: f64,
    pub is_paused: bool,
    pub is_eof: bool,
    pub(crate) volume: f64,
    pub(crate) is_muted: bool,
    pub(crate) playback_rate: f64,

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

    // Video state
    pub(crate) current_vid: String,
    pub(crate) video_tracks: Vec<VideoTrack>,

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
    pub(crate) hardware_effect_authoring: HardwareEffectAuthoringState,

    // Phase 4 & 5 Selection/Override state
    pub(crate) selected_instance_ids: std::collections::HashSet<uuid::Uuid>,
    pub selected_keyframes: std::collections::HashSet<(uuid::Uuid, usize)>,
    pub(crate) selected_timeline_keyframe: Option<uuid::Uuid>,
    pub active_keyframe_drag: Option<KeyframeDragState>,
    pub timeline_zoom: f32,
    pub undo_stack: crate::four_d::history::UndoStack,
    pub(crate) recording_keys:
        std::collections::HashMap<eframe::egui::Key, (uuid::Uuid, std::time::Instant, u8)>,

    // Phase 6 Preset Library state
    pub(crate) effects_search_query: String,
    pub(crate) show_effect_library_editor: bool,
    pub(crate) effect_library_selection: Option<String>,
    pub(crate) effect_library_draft: ControllerEffectDraft,
    pub(crate) effect_group_draft: Option<ControllerEffectGroupDraft>,
    pub(crate) track_muted: std::collections::BTreeSet<u8>,
    pub(crate) track_soloed: std::collections::BTreeSet<u8>,
    pub(crate) track_locked: std::collections::BTreeSet<u8>,
    pub(crate) active_drag: Option<ActiveDragState>,
    pub(crate) estop_active: bool,
    pub(crate) show_estop_control: bool,
    pub(crate) confirm_estop_release: bool,
    pub(crate) show_estop_release_dialog: bool,
    pub(crate) skip_estop_release_confirmation_draft: bool,
    pub(crate) serial_port: String,
    pub(crate) is_connected: bool,
    pub(crate) lasso_origin: Option<egui::Pos2>,
    pub(crate) lasso_rect: Option<egui::Rect>,
    pub(crate) rtt_state: Arc<Mutex<RttState>>,
    pub(crate) current_video_path: Option<std::path::PathBuf>,
    pub(crate) show_remaining_time: bool,
    pub(crate) editing_elapsed_time: bool,
    pub(crate) elapsed_time_input: String,
    pub(crate) elapsed_edit_focus_requested: bool,
    pub(crate) osd_message: Option<(String, std::time::Instant)>,
    pub(crate) recent_media: Vec<std::path::PathBuf>,
    pub(crate) last_media_target: Option<std::path::PathBuf>,
    pub(crate) restore_last_media_on_startup: bool,
    pub(crate) remember_playback_position: bool,
    pub(crate) playback_position_history_limit: u32,
    pub(crate) playback_positions: Vec<crate::config::PlaybackPositionEntry>,
    pub(crate) pending_resume_position: Option<f64>,
    pub(crate) last_playback_position_checkpoint: std::time::Instant,
    pub(crate) show_open_url_dialog: bool,
    pub(crate) url_input_buffer: String,
    pub(crate) open_url_multiline: bool,
    pub(crate) open_url_history_expanded: bool,
    pub(crate) open_url_recent_click_edits: bool,
    pub(crate) open_url_fetch_remote_info: bool,
    pub(crate) open_url_fetch_remote_thumbnail: bool,
    pub(crate) open_url_use_proxy: bool,
    pub(crate) open_url_proxy_url: String,
    pub(crate) url_inspector: crate::ui::open_url::UrlInspector,
    pub(crate) is_window_operating: bool,
    pub(crate) show_shortcuts_dialog: bool,
    pub(crate) show_about_dialog: bool,
    pub(crate) about_tab: usize,
    pub(crate) about_icon: Option<egui::TextureHandle>,
    pub(crate) show_preferences_dialog: bool,
    pub(crate) preferences_tab: usize,
    pub(crate) preferences_draft: Option<crate::ui::preferences::PreferencesDraft>,
    pub(crate) show_board_info_dialog: bool,
    pub(crate) board_info_tab: usize,
    pub(crate) board_name_draft: String,
    pub(crate) show_hardware_channels_dialog: bool,
    pub(crate) show_workspace_profiles_dialog: bool,
    pub(crate) workspace_profile_name_draft: String,
    pub(crate) workspace_profile_icon_draft: String,
    pub(crate) workspace_profiles:
        std::collections::BTreeMap<String, crate::config::WorkspaceProfile>,
    pub(crate) active_workspace_profile: Option<String>,
    pub(crate) hardware_control_dialog_key: Option<String>,
    pub(crate) hardware_channel_detail_active: bool,
    pub(crate) hardware_control_name_draft: String,
    pub(crate) hardware_control_group_draft: String,
    pub(crate) hardware_control_icon_draft: String,
    pub(crate) hardware_control_icon_search: String,
    pub(crate) hardware_control_color_draft: String,
    pub(crate) hardware_control_up_color_draft: String,
    pub(crate) hardware_control_down_color_draft: String,
    pub(crate) hardware_control_pwm_percent: f64,
    pub(crate) hardware_key_bindings: Vec<crate::config::HardwareKeyBinding>,
    pub(crate) hardware_binding_dialog_channel: Option<String>,
    pub(crate) hardware_binding_draft: Option<crate::config::HardwareKeyBinding>,
    pub(crate) hardware_binding_capturing: bool,
    pub(crate) hardware_hotkey_runtime: crate::hardware_shortcuts::GlobalHardwareShortcutRuntime,
    pub(crate) active_hardware_bindings: std::collections::BTreeSet<String>,
    pub(crate) board_operation: Option<String>,
    pub(crate) board_operation_status: String,
    pub(crate) board_settings_draft: Option<crate::four_d::controller::HardwareBoardSettings>,
    pub(crate) board_settings_dirty: bool,
    pub(crate) board_reboot_armed: bool,
    pub(crate) front_panel_refresh_attempted: bool,
    pub(crate) front_panel_pending_key: Option<String>,
    pub(crate) pause_on_hardware_disconnect: bool,
    pub(crate) auto_connect_hardware: bool,
    pub(crate) click_player_to_toggle: bool,
    pub(crate) show_subseconds: bool,
    pub(crate) quick_seek_seconds: f64,
    pub(crate) frame_step_count: u32,
    pub(crate) wheel_seek_seconds: f64,
    pub(crate) osd_position: crate::config::OsdPosition,
    pub(crate) osd_timeout_seconds: f32,
    pub(crate) paused_drag_action: crate::config::PlayerDragAction,
    pub(crate) playing_drag_action: crate::config::PlayerDragAction,
    pub(crate) fullscreen_video_background: crate::config::VideoBackground,
    pub(crate) motion_control_mode: crate::config::MotionControlMode,
    pub(crate) held_motion_action: Option<(String, String)>,
    pub(crate) compact_hardware_controls: bool,
    pub(crate) non_user_control_visibility: crate::config::NonUserControlVisibility,
    pub(crate) prefix_relay_identifiers: bool,
    pub(crate) live_pwm_updates: bool,
    pub(crate) hardware_actions_on_press: bool,
    pub(crate) single_instance: bool,
    pub(crate) windows_mica_backdrop: bool,
    pub(crate) windows_dwm_theming: bool,
    pub(crate) opengl_vsync: bool,
    pub(crate) native_dialog_windows: bool,
    pub(crate) native_preferences: Option<crate::ui::preferences::NativePreferencesController>,
    pub(crate) status_bar: crate::config::StatusBarConfig,
    pub(crate) config_fingerprint: Option<u64>,
    pub(crate) config_watcher: Option<crate::config::ConfigFileWatcher>,
    pub(crate) config_reload_due: Option<std::time::Instant>,
    pub(crate) auto_reload_config: bool,
    pub(crate) preference_preview_original: Option<crate::config::AppConfig>,
    pub(crate) config_status: String,
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
    pub(crate) last_taskbar_state: Option<crate::platform::windows::TaskbarState>,
    pub(crate) last_thumbnail_button_state: Option<(bool, bool)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubtitleTrack {
    pub id: i64,
    pub title: Option<String>,
    pub lang: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VideoTrack {
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
        self.process_shell_commands(ui.ctx());
        self.process_controller_call_results();
        self.poll_external_config(ui.ctx());

        // PCController owns the shared latch. A second client or the Web/TUI
        // may engage or release it, so reflect the engine's push/reconnect
        // observation without echoing another command back to the controller.
        let authoritative_estop = self
            .engine_handle
            .estop_active
            .load(std::sync::atomic::Ordering::SeqCst);
        if authoritative_estop != self.estop_active {
            self.estop_active = authoritative_estop;
            self.held_motion_action = None;
            if authoritative_estop {
                self.pause();
                self.set_osd(self.tr("E-STOP ACTIVE"));
            }
        }

        // A held seat direction captures the pointer until the physical button
        // is released. This remains active even if a repaint moves the cursor
        // outside the original button or the panel is hidden mid-gesture.
        if !ui.input(|input| input.pointer.primary_down())
            && let Some((_, stop_action)) = self.held_motion_action.take()
        {
            let _ = self.engine_handle.sender.send(
                crate::four_d::engine::EngineMessage::InvokeControllerAction {
                    action_id: stop_action,
                },
            );
        }

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
                .map(|file| file.path().to_owned())
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
        let viewport = ui.input(|input| input.viewport().clone());
        let is_fullscreen = viewport.fullscreen.unwrap_or(false);
        if !is_fullscreen && !viewport.minimized.unwrap_or(false) {
            if let (Some(inner), Some(outer)) = (viewport.inner_rect, viewport.outer_rect) {
                let geometry = crate::config::WindowGeometry {
                    x: outer.min.x,
                    y: outer.min.y,
                    width: inner.width(),
                    height: inner.height(),
                    maximized: viewport.maximized.unwrap_or(false),
                };
                if geometry.is_valid() {
                    self.window_geometry = Some(geometry);
                }
            }
        }
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
            let hardware_details = hardware
                .as_ref()
                .map(|capabilities| web_hardware_details(capabilities, self.motion_control_mode));
            let controller_effects = hardware
                .as_ref()
                .map(|capabilities| {
                    capabilities
                        .macros
                        .iter()
                        .map(|effect| crate::platform::interop::WebControllerEffect {
                            reference: format!("effect:{}", effect.id),
                            id: effect.id.to_string(),
                            name: effect.name.clone(),
                            icon: effect.icon.clone(),
                            category: effect.category.clone(),
                            description: String::new(),
                            kind: "sequence".to_string(),
                            duration_ms: effect.duration_ms,
                            duration_display: crate::duration::format_effect_duration_for_language(
                                self.language,
                                effect.duration_ms,
                            ),
                            action_count: effect.steps.len(),
                            editable: true,
                            program: serde_json::json!({
                                "steps": effect.steps,
                                "properties": {
                                    "mode": effect.mode,
                                    "color": effect.color,
                                    "label": effect.label,
                                    "lcd_message": effect.lcd_message,
                                    "timing_tolerance_us": effect.timing_tolerance_us,
                                    "keep_outputs_on_cancel": effect.keep_outputs_on_cancel,
                                    "board_profile_key": effect.board_profile_key,
                                    "board_profile_mode": effect.board_profile_mode,
                                }
                            }),
                            default_fps: None,
                            default_pixels: None,
                            lane: controller_effect_lane_name(controller_macro_lane(effect))
                                .to_string(),
                        })
                        .chain(capabilities.strip_effects.iter().map(|effect| {
                            crate::platform::interop::WebControllerEffect {
                                reference: format!("effect:{}", effect.id),
                                id: effect.id.clone(),
                                name: effect.name.clone(),
                                icon: effect.icon.clone(),
                                category: effect.category.clone(),
                                description: effect.description.clone(),
                                kind: "strip-stream".to_string(),
                                duration_ms: effect.default_duration_ms.unwrap_or_default(),
                                duration_display:
                                    crate::duration::format_effect_duration_for_language(
                                        self.language,
                                        effect.default_duration_ms.unwrap_or_default(),
                                    ),
                                action_count: 1,
                                editable: effect.editable,
                                lane: "lighting".to_string(),
                                program: effect.program.clone(),
                                default_fps: effect.default_fps,
                                default_pixels: effect.default_pixels,
                            }
                        }))
                        .collect()
                })
                .unwrap_or_default();
            let controller_connected = self
                .engine_handle
                .is_connected
                .load(std::sync::atomic::Ordering::Relaxed);
            let hardware_connection_requested = self
                .engine_handle
                .connection_requested
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
                muted: self.is_muted,
                playback_rate: self.playback_rate,
                playback_time: self.playback_time,
                duration: self.duration,
                current_video: self
                    .current_video_path
                    .as_ref()
                    .map(|p| crate::media::redact_media_target(&p.to_string_lossy())),
                seekable: self.is_seekable,
                live: self.is_live_media(),
                buffered_until: self.buffered_until(),
                buffering_percent: self.cache_buffering_percent,
                fullscreen: is_fullscreen,
                workspace: if self.show_four_d_editor {
                    "nle"
                } else {
                    "simple"
                }
                .to_string(),
                active_workspace_profile: self.active_workspace_profile.clone(),
                workspace_profiles: self
                    .ordered_workspace_profiles()
                    .into_iter()
                    .map(
                        |(id, profile)| crate::platform::interop::WebWorkspaceProfile {
                            id,
                            name: profile.name,
                            icon: profile.icon,
                            order: profile.order,
                            mode: if profile.nle { "nle" } else { "simple" }.to_string(),
                        },
                    )
                    .collect(),
                controller_connected,
                hardware_connection_requested,
                hardware_endpoint: self.serial_port.clone(),
                hardware_transport: self
                    .engine_handle
                    .active_transport
                    .try_lock()
                    .ok()
                    .and_then(|transport| transport.clone()),
                hardware_connected,
                hardware_error: self
                    .engine_handle
                    .connection_error
                    .try_lock()
                    .ok()
                    .and_then(|error| error.clone()),
                estop_active: self.estop_active,
                hardware: hardware.as_ref().map(|capabilities| {
                    crate::platform::interop::HardwareStatusSummary {
                        board_name: capabilities.board_name.clone(),
                        relay_count: capabilities.relays.len(),
                        pwm_count: capabilities.pwm_channels.len(),
                        supports_rf_transmit: capabilities.supports_rf_transmit,
                        supports_addressable_led: capabilities.supports_addressable_led,
                        supports_segment_display: capabilities.supports_segment_display,
                        supports_lcd_display: capabilities.supports_lcd_display,
                    }
                }),
                hardware_details,
                recording: self.is_recording,
                recording_armed: self.timeline.analog_tracks.iter().any(|track| track.armed),
                recordable_track_count: self.timeline.analog_tracks.len(),
                effects: self
                    .timeline
                    .templates
                    .iter()
                    .map(|effect| crate::platform::interop::WebEffectProfile {
                        id: effect.id.to_string(),
                        name: effect.name.clone(),
                        duration_ms: effect.duration_ms,
                        duration_display: crate::duration::format_effect_duration_for_language(
                            self.language,
                            effect.duration_ms,
                        ),
                        action_count: effect.actions.len(),
                        target: match effect.target {
                            crate::four_d::models::HardwareTarget::Any => "any".to_string(),
                            crate::four_d::models::HardwareTarget::Relay(relay) => {
                                format!("relay:{relay}")
                            }
                            crate::four_d::models::HardwareTarget::ControllerMacro => {
                                "controller".to_string()
                            }
                        },
                        lane: effect
                            .controller_lane
                            .map(controller_effect_lane_name)
                            .unwrap_or("sequence")
                            .to_string(),
                    })
                    .collect(),
                controller_effects,
                cues: self
                    .timeline
                    .instances
                    .iter()
                    .filter_map(|instance| {
                        self.timeline
                            .templates
                            .iter()
                            .find(|effect| effect.id == instance.effect_id)
                            .map(|effect| crate::platform::interop::WebEffectCue {
                                id: instance.id.to_string(),
                                effect_id: effect.id.to_string(),
                                name: effect.name.clone(),
                                start_time_ms: instance.start_time_ms,
                                duration_ms: effect.duration_ms,
                                duration_display:
                                    crate::duration::format_effect_duration_for_language(
                                        self.language,
                                        effect.duration_ms,
                                    ),
                            })
                    })
                    .collect(),
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
        if let Ok(err_guard) = self.engine_handle.connection_error.try_lock() {
            if let Some(err) = err_guard.as_ref()
                && self.connection_notice.as_deref() != Some(err)
            {
                self.connection_notice = Some(err.clone());
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
        let hardware_lost = hardware_connection_was_lost(
            self.was_hardware_connected,
            connected_now,
            self.was_board_connected,
            board_connected_now,
        );
        if hardware_lost {
            let message = if self.pause_on_hardware_disconnect {
                self.pause();
                self.tr("Hardware disconnected — playback paused")
            } else {
                self.tr("Hardware disconnected")
            };
            self.set_osd(message.clone());
            if let Some(hwnd) = self.window_handle {
                let _ = crate::platform::windows::show_system_notification(
                    hwnd,
                    &self.app_name,
                    &message,
                );
            }
        }
        if connected_now && !self.was_hardware_connected {
            self.connection_notice = None;
            self.save_config();
        }
        self.is_connected = connected_now;
        self.was_hardware_connected = connected_now;
        self.was_board_connected = board_connected_now;
        // Controller/WebSocket callbacks already request repaint on real state
        // changes. Do not keep the opaque OpenGL window on a synthetic timer:
        // that needlessly recomposes the entire UI and can present as flicker.
        let connection_requested = self
            .engine_handle
            .connection_requested
            .load(std::sync::atomic::Ordering::Relaxed);
        if connection_requested && (!connected_now || !board_connected_now) {
            // Connection transitions happen on the engine thread. Keep the
            // UI, title, and local status API truthful during recovery without
            // returning to a permanent repaint loop that burns GPU while idle.
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
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
        let timeline_keyboard_active =
            ctx.memory(|memory| memory.has_focus(crate::ui::layout::timeline_keyboard_focus_id()));
        let transport_shortcuts_enabled =
            !ctx.egui_wants_keyboard_input() && !timeline_keyboard_active;
        self.process_hardware_key_bindings(
            &ctx,
            transport_shortcuts_enabled && self.hardware_binding_dialog_channel.is_none(),
        );
        if transport_shortcuts_enabled && ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.toggle_playback();
        }
        if transport_shortcuts_enabled && ctx.input(|i| i.key_pressed(egui::Key::F)) {
            self.toggle_fullscreen(&ctx);
        }
        if transport_shortcuts_enabled && ctx.input(|i| i.key_pressed(egui::Key::M)) {
            let _ = self.mpv.command("cycle", &["mute"]);
            self.is_muted = !self.is_muted;
            self.set_osd(if self.is_muted {
                "Mute".to_string()
            } else {
                "Unmute".to_string()
            });
        }
        if transport_shortcuts_enabled && ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            self.seek_relative(-self.quick_seek_seconds);
        }
        if transport_shortcuts_enabled && ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            self.seek_relative(self.quick_seek_seconds);
        }
        if transport_shortcuts_enabled
            && ctx.input(|i| {
                i.key_pressed(egui::Key::Period) || i.key_pressed(egui::Key::CloseBracket)
            })
        {
            self.step_frames(1);
        }
        if transport_shortcuts_enabled
            && ctx
                .input(|i| i.key_pressed(egui::Key::Comma) || i.key_pressed(egui::Key::OpenBracket))
        {
            self.step_frames(-1);
        }
        if transport_shortcuts_enabled && ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            let _ = self.mpv.command("add", &["volume", "5"]);
            self.volume = (self.volume + 5.0).clamp(0.0, 130.0);
            self.set_osd(format!("Volume: {:.0}%", self.volume));
        }
        if transport_shortcuts_enabled && ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
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
                    if self.dock_state.iter_all_tabs().count() == 0 {
                        ui.centered_and_justified(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(egui::RichText::new(crate::ui::icons::TABS).size(36.0));
                                ui.add_space(8.0);
                                ui.heading(self.tr("All workspace panels are closed"));
                                ui.label(self.tr("Open panels from the Window menu above, or reset the workspace."));
                                ui.add_space(12.0);
                                crate::ui::layout::draw_workspace_tab_menu(self, ui);
                            });
                        });
                    } else {
                        let workspace_tab_rects_id = egui::Id::new("workspace-tab-button-rects");
                        ui.ctx().data_mut(|data| {
                            data.insert_temp(workspace_tab_rects_id, Vec::<egui::Rect>::new());
                        });
                        let mut dock_state =
                            std::mem::replace(&mut self.dock_state, egui_dock::DockState::new(vec![]));
                        let mut dock_style = egui_dock::Style::from_egui(ui.style().as_ref());
                        let dock_tab_bar_height = dock_style.tab_bar.height;
                        // egui_dock paints legacy solid triangles for leaf disclosure.
                        // Keep its hit target and behavior, but let Pealayer paint the
                        // matching Phosphor caret from the application's icon vocabulary.
                        dock_style.buttons.collapse_tabs_color = egui::Color32::TRANSPARENT;
                        dock_style.buttons.collapse_tabs_active_color = egui::Color32::TRANSPARENT;
                        let dock_response = ui.scope(|ui| {
                            let mut tab_viewer = crate::ui::layout::PealayerTabViewer { app: self };
                            egui_dock::DockArea::new(&mut dock_state)
                                .style(dock_style)
                                .show_leaf_collapse_buttons(true)
                                .show_inside(ui, &mut tab_viewer);
                        });
                        self.dock_state = dock_state;
                        crate::ui::layout::paint_dock_disclosure_icons(
                            ui,
                            &self.dock_state,
                            dock_tab_bar_height,
                        );

                        let tab_rects = ui.ctx().data_mut(|data| {
                            data.get_temp::<Vec<egui::Rect>>(workspace_tab_rects_id)
                                .unwrap_or_default()
                        });
                        let pointer = ui.ctx().pointer_hover_pos();
                        let open_empty_tab_menu = ui.ctx().input(|input| {
                            input.pointer.button_clicked(egui::PointerButton::Secondary)
                        }) && pointer.is_some_and(|position| {
                            dock_response.response.rect.contains(position)
                                && tab_rects
                                    .iter()
                                    .any(|rect| rect.y_range().contains(position.y))
                                && !tab_rects.iter().any(|rect| rect.contains(position))
                        });
                        let popup_anchor = ui.interact(
                            dock_response.response.rect,
                            egui::Id::new("workspace-empty-tabbar-context-anchor"),
                            egui::Sense::hover(),
                        );
                        egui::Popup::menu(&popup_anchor)
                            .id(egui::Id::new("workspace-empty-tabbar-context-menu"))
                            .at_pointer_fixed()
                            .open_memory(
                                open_empty_tab_menu
                                    .then_some(egui::SetOpenCommand::Bool(true)),
                            )
                            .show(|ui| {
                                ui.strong(self.tr("Panels"));
                                ui.separator();
                                crate::ui::layout::draw_workspace_tab_menu(self, ui);
                            });

                        dock_response.response.context_menu(|ui| {
                            ui.label(egui::RichText::new(self.tr("Workspace")).strong());
                            ui.separator();
                            crate::ui::layout::draw_workspace_tab_menu(self, ui);
                            ui.separator();
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::TABS,
                                    self.tr("Reset workspace layout")
                                ))
                                .clicked()
                            {
                                self.dock_state = crate::ui::layout::create_initial_layout();
                                self.save_dock_layout();
                                ui.close();
                            }
                            crate::ui::icons::submenu(
                                ui,
                                format!("{} {}", crate::ui::icons::TABS, self.tr("Workspaces")),
                                |ui| {
                                    for (id, profile) in self.ordered_workspace_profiles() {
                                        let active = self.active_workspace_profile.as_deref()
                                            == Some(id.as_str());
                                        if ui
                                            .selectable_label(
                                                active,
                                                format!(
                                                    "{}  {}",
                                                    crate::ui::icons::workspace_icon(&profile.icon),
                                                    profile.name
                                                ),
                                            )
                                            .clicked()
                                        {
                                            self.restore_workspace_profile(ui.ctx(), &id);
                                            ui.close();
                                        }
                                    }
                                },
                            );
                            ui.separator();
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::GEAR,
                                    self.tr("Preferences...")
                                ))
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
                crate::ui::menu::draw_estop_release_dialog(self, ui);
                crate::ui::subtitles::draw_settings_dialog(self, ui);
                crate::ui::audio::draw_settings_dialog(self, ui);
                crate::ui::preferences::draw(self, ui);
                crate::ui::effects_library::draw_editor(self, ui);
                crate::ui::board_info::draw(self, ui);
                crate::ui::hardware_control::draw(self, ui);
                crate::ui::workspace_profiles::draw(self, ui);

                crate::ui::open_url::draw(self, ui);

                if self.show_shortcuts_dialog {
                    let language = self.language;
                    let bounds = ui.ctx().content_rect().shrink(20.0);
                    let max_size = egui::vec2(bounds.width().min(600.0), bounds.height().min(520.0));
                    let default_size = egui::vec2(max_size.x.min(520.0), max_size.y.min(420.0));
                    let default_rect = crate::ui::dialog::centered_default_rect(bounds, default_size);
                    if crate::ui::dialog::escape_pressed(ui.ctx()) {
                        self.show_shortcuts_dialog = false;
                    }
                    egui::Window::new(format!(
                        "{} {}",
                        crate::ui::icons::KEYBOARD,
                        self.tr("Keyboard Shortcuts & Controls")
                    ))
                    .id(egui::Id::new("keyboard_shortcuts_dialog_bounded_v2"))
                    .collapsible(false)
                    .resizable(true)
                    .default_rect(default_rect)
                    .min_size([max_size.x.min(360.0), max_size.y.min(280.0)])
                    .max_size(max_size)
                    .constrain_to(bounds)
                    .movable(true)
                    .open(&mut self.show_shortcuts_dialog)
                    .show(ui.ctx(), |ui| {
                      crate::ui::dialog::scroll_column(ui, "shortcuts_content_v2", None, |ui| {
                        egui::Grid::new("shortcuts_grid")
                            .striped(true)
                            .spacing([20.0, 8.0])
                            .show(ui, |ui| {
                                ui.label("");
                                ui.label(
                                    egui::RichText::new(crate::ui::i18n::tr(language, "Shortcut"))
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(crate::ui::i18n::tr(language, "Action"))
                                        .strong(),
                                );
                                ui.end_row();
                                for (icon, shortcut, action) in [
                                    (crate::ui::icons::PLAY, "Space", "Play / Pause video"),
                                    (crate::ui::icons::ARROWS_OUT, "F", "Toggle Fullscreen mode"),
                                    (crate::ui::icons::SPEAKER_HIGH, "M", "Toggle Audio Mute"),
                                    (crate::ui::icons::ARROW_COUNTER_CLOCKWISE, "← / →", "Seek -5s / +5s"),
                                    (crate::ui::icons::SPEAKER_HIGH, "↑ / ↓", "Volume -5% / +5%"),
                                    (crate::ui::icons::ARROW_DOWN, ".  or  ]", "Frame Step Forward (+1 frame)"),
                                    (crate::ui::icons::ARROW_UP, ",  or  [", "Frame Step Backward (-1 frame)"),
                                    (crate::ui::icons::SLIDERS_HORIZONTAL, "Mouse Wheel", "Adjust Volume on player/bar"),
                                    (crate::ui::icons::CLOCK_COUNTER_CLOCKWISE, "Shift + Mouse Wheel", "Seek forward / backward"),
                                    (crate::ui::icons::ARROWS_OUT, "Double Click", "Toggle Fullscreen / Open Video"),
                                    (crate::ui::icons::LIST_CHECKS, "Right Click", "Open Player Context Menu"),
                                    (crate::ui::icons::FILE_VIDEO, "Drag & Drop", "Drop media file onto window to play"),
                                ] {
                                    ui.label(icon);
                                    ui.label(shortcut);
                                    ui.add(egui::Label::new(crate::ui::i18n::tr(language, action)).wrap());
                                    ui.end_row();
                                }
                            });
                      });
                    });
                }

                if self.show_about_dialog {
                    crate::ui::about::draw(self, ui);
                }
            });
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.save_config();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Never leave a press-and-hold channel active merely because Pealayer
        // was closed before the operating system delivered the key-up event.
        self.release_active_hardware_bindings();
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

fn board_settings_command(
    settings: &crate::four_d::controller::HardwareBoardSettings,
) -> Result<String, String> {
    if settings.light_mode > 2 {
        return Err("Light mode must be Off, Automatic, or On".to_string());
    }
    if settings.display_brightness > 7 || settings.display_closed_brightness > 7 {
        return Err("Display brightness must be between 0 and 7".to_string());
    }
    if settings.output_persistence & !0x0F != 0 {
        return Err("Output persistence contains unsupported flags".to_string());
    }
    if settings.stream_period_ms > u16::MAX as u64 {
        return Err("Telemetry period must fit in 0..65535 ms".to_string());
    }
    if settings.default_page > 13 {
        return Err("Default front-panel page must be between 0 and 13".to_string());
    }
    if settings.status_color > 4 {
        return Err("Status color is not supported by the connected board".to_string());
    }
    if settings.voltage_decimals > 2 || settings.current_decimals > 2 {
        return Err("Measurement decimals must be between 0 and 2".to_string());
    }
    if !(1..=31).contains(&settings.motion_exit_hold_seconds) {
        return Err("Motion exit hold must be between 1 and 31 seconds".to_string());
    }
    if !(1..=255).contains(&settings.motion_break_ms) {
        return Err("Motion break must be between 1 and 255 ms".to_string());
    }

    // Preserve the unassigned high bit while regenerating every documented
    // settings flag from the editor's semantic controls.
    let mut flags = settings.flags & 0x80;
    if settings.silent {
        flags |= 0x01;
    }
    if settings.programming_latch {
        flags |= 0x02;
    }
    if settings.swap_temperature_roles {
        flags |= 0x04;
    }
    flags |= (settings.motion_door_policy & 0x03) << 3;
    if !settings.door_audio_enabled {
        flags |= 0x20;
    }
    if !settings.relay_audio_enabled {
        flags |= 0x40;
    }

    Ok(format!(
        "settings set {flags} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
        settings.light_mode,
        settings.on_brightness,
        settings.off_brightness,
        settings.display_brightness,
        settings.display_closed_brightness,
        settings.status_brightness,
        settings.output_persistence,
        settings.stream_period_ms,
        settings.default_page,
        u8::from(settings.save_last_page),
        settings.status_color,
        settings.voltage_decimals,
        settings.current_decimals,
        settings.motion_exit_hold_seconds,
        settings.motion_break_ms,
        settings.relay_restore_mask,
    ))
}

impl PealayerApp {
    pub(crate) fn open_hardware_bindings_for_channel(&mut self, channel_key: &str) {
        self.hardware_binding_dialog_channel = Some(channel_key.to_string());
        self.hardware_binding_draft = None;
        self.hardware_binding_capturing = false;
    }

    pub(crate) fn open_hardware_binding_editor(
        &mut self,
        control: &crate::four_d::controller::HardwareControl,
        existing_id: Option<&str>,
    ) {
        let binding = existing_id
            .and_then(|id| {
                self.hardware_key_bindings
                    .iter()
                    .find(|binding| binding.id == id)
            })
            .cloned()
            .unwrap_or_else(|| {
                let action = if matches!(control.kind.as_str(), "pwm" | "mosfet") {
                    crate::config::HardwareKeyBindingAction::SetPwm { percent: 100.0 }
                } else if let Some(action) = control.actions.first() {
                    crate::config::HardwareKeyBindingAction::Invoke {
                        action_id: action.id.clone(),
                        label: action.name.clone(),
                    }
                } else {
                    crate::config::HardwareKeyBindingAction::default()
                };
                crate::config::HardwareKeyBinding {
                    channel_key: control.key.clone(),
                    action,
                    ..Default::default()
                }
            });
        self.hardware_binding_dialog_channel = Some(control.key.clone());
        self.hardware_binding_draft = Some(binding);
        self.hardware_binding_capturing = false;
    }

    pub(crate) fn save_hardware_binding_draft(&mut self) -> Result<(), String> {
        let binding = self
            .hardware_binding_draft
            .clone()
            .ok_or_else(|| "No keyboard binding is being edited".to_string())?;
        let mut candidate = self.runtime_config_snapshot();
        if let Some(existing) = candidate
            .hardware_key_bindings
            .iter_mut()
            .find(|existing| existing.id == binding.id)
        {
            *existing = binding.clone();
        } else {
            candidate.hardware_key_bindings.push(binding.clone());
        }
        candidate.validate()?;
        if self.active_hardware_bindings.contains(&binding.id) {
            self.dispatch_hardware_binding(&binding.id, false);
        }
        if let Some(existing) = self
            .hardware_key_bindings
            .iter_mut()
            .find(|existing| existing.id == binding.id)
        {
            *existing = binding;
        } else {
            self.hardware_key_bindings.push(binding);
        }
        self.hardware_binding_draft = None;
        self.hardware_binding_capturing = false;
        self.save_config();
        Ok(())
    }

    pub(crate) fn delete_hardware_binding(&mut self, id: &str) {
        if self.active_hardware_bindings.contains(id) {
            self.dispatch_hardware_binding(id, false);
        }
        self.hardware_key_bindings
            .retain(|binding| binding.id != id);
        self.active_hardware_bindings.remove(id);
        self.hardware_binding_draft = None;
        self.hardware_binding_capturing = false;
        self.save_config();
    }

    fn release_active_hardware_bindings(&mut self) {
        let active = self
            .active_hardware_bindings
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        for binding_id in active {
            self.dispatch_hardware_binding(&binding_id, false);
        }
        self.active_hardware_bindings.clear();
    }

    fn dispatch_hardware_binding(&mut self, binding_id: &str, pressed: bool) {
        let Some(binding) = self
            .hardware_key_bindings
            .iter()
            .find(|binding| binding.id == binding_id && binding.enabled)
            .cloned()
        else {
            return;
        };
        if pressed {
            if !self.active_hardware_bindings.insert(binding.id.clone()) {
                return;
            }
        } else if !self.active_hardware_bindings.remove(&binding.id) {
            return;
        }

        let Some(capabilities) = self
            .advertised_hardware()
            .filter(|capabilities| capabilities.board_connected)
        else {
            if pressed {
                self.set_osd(self.tr("Keyboard shortcut ignored: no board is connected"));
            }
            return;
        };
        let control = crate::ui::hardware_control::managed_controls(&capabilities)
            .into_iter()
            .find(|control| control.key == binding.channel_key);
        if pressed && self.estop_active {
            self.active_hardware_bindings.remove(&binding.id);
            self.set_osd(self.tr("Keyboard shortcut ignored: all outputs are disabled"));
            return;
        }
        if pressed && control.as_ref().is_some_and(|control| control.locked) {
            self.active_hardware_bindings.remove(&binding.id);
            self.set_osd(self.tr("Keyboard shortcut ignored: this channel is locked"));
            return;
        }
        let invoke = |app: &mut Self, action_id: &str| {
            if let Err(error) =
                crate::ui::hardware_control::invoke_action_by_id(app, &capabilities, action_id)
            {
                app.set_osd(error);
            }
        };

        match binding.action {
            crate::config::HardwareKeyBindingAction::Invoke { action_id, .. } => {
                if pressed {
                    invoke(self, &action_id);
                }
            }
            crate::config::HardwareKeyBindingAction::Toggle {
                on_action_id,
                off_action_id,
            } => {
                if pressed {
                    let active = control.as_ref().is_some_and(|control| {
                        crate::ui::hardware_control::channel_is_active(&capabilities, control)
                    });
                    invoke(
                        self,
                        if active {
                            &off_action_id
                        } else {
                            &on_action_id
                        },
                    );
                }
            }
            crate::config::HardwareKeyBindingAction::SetPwm { percent } => {
                if pressed {
                    if let Some(channel) = capabilities
                        .pwm_channels
                        .iter()
                        .find(|channel| channel.key == binding.channel_key)
                        .map(|channel| channel.id)
                    {
                        crate::ui::hardware_control::set_pwm(self, channel, percent);
                    } else {
                        self.set_osd(self.tr("PWM channel is no longer advertised"));
                    }
                }
            }
            crate::config::HardwareKeyBindingAction::Hold {
                press_action_id,
                release_action_id,
                ..
            } => invoke(
                self,
                if pressed {
                    &press_action_id
                } else {
                    &release_action_id
                },
            ),
        }
    }

    fn process_hardware_key_bindings(&mut self, ctx: &egui::Context, allow_local: bool) {
        self.hardware_hotkey_runtime
            .sync(&self.hardware_key_bindings);
        let global_events = self.hardware_hotkey_runtime.drain_events();
        for event in global_events {
            // Recording a new chord must never trigger another binding, but a
            // key-up from a previously active hold binding is safety-critical:
            // it still has to send the configured release/stop action.
            if !event.pressed || !self.hardware_binding_capturing {
                self.dispatch_hardware_binding(&event.binding_id, event.pressed);
            }
        }

        let events = ctx.input(|input| input.events.clone());
        let bindings = self
            .hardware_key_bindings
            .iter()
            .filter(|binding| binding.enabled && !binding.global)
            .cloned()
            .collect::<Vec<_>>();
        for event in &events {
            let mut matched = false;
            for binding in &bindings {
                if let Some(pressed) =
                    crate::hardware_shortcuts::event_matches_chord(event, &binding.chord)
                    && (!pressed || (allow_local && !self.hardware_binding_capturing))
                {
                    self.dispatch_hardware_binding(&binding.id, pressed);
                    matched = true;
                }
            }
            if matched && let egui::Event::Key { key, modifiers, .. } = event {
                ctx.input_mut(|input| {
                    input.consume_key(*modifiers, *key);
                });
            }
        }
    }

    /// Ensures Windows Shell components (thumbnail toolbar and system tray icon)
    /// are initialized once a valid window handle is registered.
    pub fn ensure_shell_initialized(&mut self) {
        if !self.shell_initialized {
            let hwnd = self
                .window_handle
                .unwrap_or_else(crate::platform::windows::get_registered_hwnd);
            if hwnd != 0 {
                let result = crate::platform::windows::install_shell_message_hook(hwnd)
                    .and_then(|_| crate::platform::windows::init_taskbar_thumbnail_toolbar(hwnd))
                    .and_then(|_| {
                        crate::platform::windows::register_system_tray_icon(hwnd, &self.app_name)
                    });
                self.shell_initialized = result.is_ok();
                if self.shell_initialized {
                    // Initialization creates a fresh Windows Shell surface. Force
                    // one state publication, then only publish subsequent deltas.
                    self.last_taskbar_state = None;
                    self.last_thumbnail_button_state = None;
                }
                if let Err(error) = result {
                    log::warn!("Windows shell integration is not ready; retrying: {error}");
                }
            }
        }
    }

    fn process_shell_commands(&mut self, ctx: &egui::Context) {
        crate::platform::windows::update_shell_command_state(
            self.is_paused,
            self.is_muted,
            self.current_video_path.is_some(),
        );
        while let Some(command) = crate::platform::windows::take_shell_command() {
            match command {
                crate::platform::windows::THUMB_BUTTON_PREV => self.seek_relative(-10.0),
                crate::platform::windows::THUMB_BUTTON_PLAYPAUSE
                | crate::platform::windows::TRAY_CMD_PLAYPAUSE => self.toggle_playback(),
                crate::platform::windows::THUMB_BUTTON_NEXT => self.seek_relative(10.0),
                crate::platform::windows::TRAY_CMD_MUTE => {
                    let _ = self.mpv.command("cycle", &["mute"]);
                    self.is_muted = !self.is_muted;
                }
                crate::platform::windows::TRAY_CMD_OPEN => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Video Files", &["mp4", "mkv", "avi", "webm", "mov", "flv"])
                        .pick_file()
                    {
                        self.load_video_file(path);
                    }
                }
                crate::platform::windows::TRAY_CMD_EXIT => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                crate::platform::windows::TRAY_CMD_SHOW => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                _ => {}
            }
        }
    }

    /// Synchronizes playback time, duration, pause state, and error condition
    /// with the Windows taskbar progress state and thumbnail toolbar buttons.
    pub fn update_shell_state(&mut self) {
        let taskbar_state = crate::platform::windows::compute_taskbar_state_with_error(
            self.playback_time,
            self.duration,
            self.is_paused,
            self.show_error.is_some(),
        );
        if self.last_taskbar_state != Some(taskbar_state) {
            crate::platform::windows::update_windows_taskbar_state_ext(
                self.playback_time,
                self.duration,
                self.is_paused,
                self.show_error.is_some(),
            );
            self.last_taskbar_state = Some(taskbar_state);
        }

        let hwnd = self
            .window_handle
            .unwrap_or_else(crate::platform::windows::get_registered_hwnd);
        let thumbnail_state = (self.is_paused, self.current_video_path.is_some());
        if self.shell_initialized
            && hwnd != 0
            && self.last_thumbnail_button_state != Some(thumbnail_state)
            && crate::platform::windows::update_taskbar_thumbnail_buttons(
                hwnd,
                thumbnail_state.0,
                thumbnail_state.1,
            )
            .is_ok()
        {
            self.last_thumbnail_button_state = Some(thumbnail_state);
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

    pub fn connected_board_display_name(&self) -> Option<String> {
        self.advertised_hardware()
            .filter(|capabilities| capabilities.board_connected)
            .map(|capabilities| self.display_text(capabilities.board_name.trim()))
            .filter(|name| !name.trim().is_empty())
    }

    pub fn hardware_unavailable_detail(
        &self,
        capabilities: Option<&crate::four_d::controller::HardwareCapabilities>,
    ) -> String {
        if let Some(notice) = self
            .connection_notice
            .as_deref()
            .map(str::trim)
            .filter(|notice| !notice.is_empty())
        {
            return format!("{}: {notice}", self.tr("Board connection failed"));
        }
        if let Some(capabilities) = capabilities {
            if let Some(warning) = capabilities.warnings.iter().find(|warning| {
                !warning.message.trim().is_empty()
                    && matches!(
                        warning.severity.trim().to_ascii_lowercase().as_str(),
                        "error" | "critical" | "fatal"
                    )
            }) {
                return format!(
                    "{}: {}",
                    self.tr("Board connection failed"),
                    self.display_text(warning.message.trim())
                );
            }
            let port = capabilities.port.name.trim();
            if !port.is_empty() {
                return format!(
                    "{} {port}, {}",
                    self.tr("PCController detected hardware on"),
                    self.tr("but the board is not responding. Check its USB cable, power, and operating-system device status.")
                );
            }
        }
        self.tr("PCController is reachable, but no board is connected. Connect the board and check its USB cable, power, and port.")
    }

    pub fn advertised_effect_presets(&self) -> Vec<EffectPreset> {
        let Some(capabilities) = self.advertised_hardware() else {
            return Vec::new();
        };

        capabilities
            .macros
            .iter()
            .map(controller_macro_effect_preset)
            .chain(
                capabilities
                    .strip_effects
                    .iter()
                    .map(controller_strip_effect_preset),
            )
            .collect()
    }

    fn controller_command_argument(value: &str) -> Option<String> {
        let value = value.trim();
        (!value.is_empty()
            && value.len() <= 64
            && value
                .chars()
                .all(|character| character.is_alphanumeric() || " -_".contains(character)))
        .then(|| format!("\"{value}\""))
    }

    fn controller_effect_program_hex(value: &str) -> Result<(String, String), String> {
        let program: serde_json::Value = serde_json::from_str(value)
            .map_err(|error| format!("Invalid lighting program JSON: {error}"))?;
        let primitive = program
            .get("primitive")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "Lighting program requires a primitive".to_string())?
            .to_string();
        let canonical = serde_json::to_vec(&program)
            .map_err(|error| format!("Encode lighting program: {error}"))?;
        let encoded = canonical
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join("");
        Ok((primitive, encoded))
    }

    fn request_hardware_effect_command(
        &mut self,
        operation: &str,
        command: String,
    ) -> Result<(), String> {
        if self.hardware_effect_authoring.pending_operation.is_some() {
            return Err("another hardware effect operation is still running".to_string());
        }
        self.engine_handle.request_controller_call(
            operation,
            "controller.command.execute",
            serde_json::json!({"command": command}),
        )?;
        self.hardware_effect_authoring.pending_operation = Some(operation.to_string());
        self.hardware_effect_authoring.status = "Waiting for PCController…".to_string();
        Ok(())
    }

    pub(crate) fn start_hardware_effect_recording(&mut self) -> Result<(), String> {
        let name = Self::controller_command_argument(&self.hardware_effect_authoring.name)
            .ok_or_else(|| {
                "Effect name must be 1–64 letters, numbers, spaces, dashes, or underscores"
                    .to_string()
            })?;
        self.hardware_effect_authoring.anchor_ms =
            (self.seek_pos.unwrap_or(self.playback_time).max(0.0) * 1_000.0) as u64;
        let command = format!("effect record start {name} Pealayer violet");
        self.request_hardware_effect_command("macro-start", command)
    }

    pub(crate) fn refresh_hardware_effect_recording(&mut self) -> Result<(), String> {
        self.request_hardware_effect_command("macro-status", "effect record status".to_string())
    }

    pub(crate) fn save_hardware_effect_recording(&mut self) -> Result<(), String> {
        self.request_hardware_effect_command("macro-save", "effect record save".to_string())
    }

    pub(crate) fn discard_hardware_effect_recording(&mut self) -> Result<(), String> {
        self.request_hardware_effect_command("macro-discard", "effect record discard".to_string())
    }

    pub(crate) fn preview_strip_effect(&mut self, id: &str) -> Result<(), String> {
        let advertised = self.advertised_hardware().is_some_and(|capabilities| {
            capabilities.board_connected
                && capabilities
                    .strip_effects
                    .iter()
                    .any(|effect| effect.id == id)
        });
        if !advertised {
            return Err("the selected strip effect is no longer advertised".to_string());
        }
        self.request_hardware_effect_command("effect-preview", format!("effect play {id}"))
    }

    pub(crate) fn stop_strip_preview(&mut self) -> Result<(), String> {
        self.request_hardware_effect_command("effect-stop", "effect stop".to_string())
    }

    fn addressable_strip_contract(
        &self,
        mode: &str,
    ) -> Result<crate::four_d::controller::HardwareStripControl, String> {
        let capabilities = self
            .advertised_hardware()
            .filter(|capabilities| capabilities.board_connected)
            .ok_or_else(|| "No live board is connected".to_string())?;
        let strip = capabilities.strip_control.ok_or_else(|| {
            "The connected board does not advertise addressable LED controls".to_string()
        })?;
        if !mode.is_empty() && !strip.supports(mode) {
            return Err(format!(
                "The connected PCController does not advertise {mode} strip control"
            ));
        }
        Ok(strip)
    }

    fn validate_strip_settings(
        strip: &crate::four_d::controller::HardwareStripControl,
        pixels: u16,
        fps: Option<u8>,
    ) -> Result<(), String> {
        if !(strip.minimum_pixels..=strip.maximum_pixels).contains(&pixels) {
            return Err(format!(
                "Pixel count must be {}–{} for this controller",
                strip.minimum_pixels, strip.maximum_pixels
            ));
        }
        if let Some(fps) = fps {
            if !(strip.minimum_fps..=strip.maximum_fps).contains(&fps) {
                return Err(format!(
                    "Frame rate must be {}–{} FPS for this controller",
                    strip.minimum_fps, strip.maximum_fps
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn configure_addressable_strip(&mut self, pixels: u16) -> Result<(), String> {
        let strip = self.addressable_strip_contract("")?;
        Self::validate_strip_settings(&strip, pixels, None)?;
        self.request_hardware_effect_command("strip-config", format!("strip config {pixels}"))
    }

    pub(crate) fn fill_addressable_strip(
        &mut self,
        red: u8,
        green: u8,
        blue: u8,
        brightness: u8,
    ) -> Result<(), String> {
        self.addressable_strip_contract("")?;
        self.request_hardware_effect_command(
            "strip-fill",
            format!("strip fill {red} {green} {blue} {brightness}"),
        )
    }

    pub(crate) fn set_addressable_strip_pixel(
        &mut self,
        pixel: u16,
        pixels: u16,
        red: u8,
        green: u8,
        blue: u8,
        brightness: u8,
    ) -> Result<(), String> {
        let strip = self.addressable_strip_contract("pixel")?;
        Self::validate_strip_settings(&strip, pixels, None)?;
        if pixel >= pixels {
            return Err(format!(
                "Pixel index must be 0–{}",
                pixels.saturating_sub(1)
            ));
        }
        self.request_hardware_effect_command(
            "strip-pixel",
            format!("strip pixel {pixel} {red} {green} {blue} {brightness}"),
        )
    }

    pub(crate) fn send_addressable_strip_frame(
        &mut self,
        pixels: u16,
        rgb: &[u8],
    ) -> Result<(), String> {
        let strip = self.addressable_strip_contract("frame")?;
        Self::validate_strip_settings(&strip, pixels, None)?;
        if rgb.len() != usize::from(pixels) * 3 {
            return Err("The color frame does not match the configured pixel count".to_string());
        }
        let encoded = rgb
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        self.request_hardware_effect_command("strip-frame", format!("strip frame {encoded}"))
    }

    pub(crate) fn start_addressable_strip_rainbow(
        &mut self,
        pixels: u16,
        fps: u8,
    ) -> Result<(), String> {
        let strip = self.addressable_strip_contract("rainbow")?;
        Self::validate_strip_settings(&strip, pixels, Some(fps))?;
        self.request_hardware_effect_command(
            "strip-rainbow",
            format!("strip rainbow {pixels} {fps}"),
        )
    }

    pub(crate) fn start_addressable_strip_effect(
        &mut self,
        id: &str,
        pixels: u16,
        fps: u8,
    ) -> Result<(), String> {
        let strip = self.addressable_strip_contract("effect")?;
        Self::validate_strip_settings(&strip, pixels, Some(fps))?;
        let advertised = self.advertised_hardware().is_some_and(|capabilities| {
            capabilities
                .strip_effects
                .iter()
                .any(|effect| effect.id == id)
        });
        if !advertised || !crate::four_d::controller::valid_strip_effect_id(id) {
            return Err("The selected strip effect is no longer advertised".to_string());
        }
        self.request_hardware_effect_command(
            "effect-preview",
            format!("strip effect play {id} {pixels} {fps}"),
        )
    }

    pub(crate) fn clear_addressable_strip(&mut self) -> Result<(), String> {
        self.addressable_strip_contract("solid")?;
        self.request_hardware_effect_command("strip-clear", "strip clear".to_string())
    }

    pub(crate) fn stop_addressable_strip(&mut self) -> Result<(), String> {
        self.addressable_strip_contract("")?;
        self.request_hardware_effect_command("strip-stop", "strip stop".to_string())
    }

    pub(crate) fn refresh_addressable_strip_status(&mut self) -> Result<(), String> {
        self.addressable_strip_contract("")?;
        self.request_hardware_effect_command("strip-status", "strip status".to_string())
    }

    pub(crate) fn save_controller_effect(&mut self) -> Result<(), String> {
        let mut draft = self.effect_library_draft.clone();
        if draft.kind == "sequence" {
            draft.steps.sort_by_key(|step| step.at_us);
            draft.duration_ms = crate::ui::effects_library::sequence_duration_ms(&draft.steps);
        }
        let checked_text = |value: &str, field: &str, required: bool| -> Result<String, String> {
            let value = value.trim();
            if (required && value.is_empty())
                || value.chars().count() > 64
                || value.chars().any(char::is_control)
            {
                return Err(format!("{field} must contain 1–64 printable characters"));
            }
            Ok(value.to_string())
        };
        let name = checked_text(&draft.name, "Effect name", true)?;
        let category = checked_text(&draft.category, "Effect category", true)?;
        let icon = checked_text(&draft.icon, "Effect icon", false)?;
        let descriptor = if draft.kind == "sequence" {
            let id = draft
                .id
                .parse::<u8>()
                .map_err(|_| "Sequence ID must be 0–255".to_string())?;
            if !matches!(draft.engine.as_str(), "auto" | "host" | "mcu") {
                return Err(
                    "Sequence execution must be automatic, host, or device clock".to_string(),
                );
            }
            serde_json::json!({
                "reference": format!("effect:{id}"),
                "id": id.to_string(),
                "name": name,
                "category": category,
                "icon": icon,
                "kind": "sequence",
                "engine": draft.engine,
                "editable": true,
                "duration_ms": draft.duration_ms,
                "steps": draft.steps,
                "properties": {
                    "color": draft.color,
                    "label": draft.label,
                    "lcd_message": draft.lcd_message,
                    "timing_tolerance_us": draft.timing_tolerance_us,
                    "keep_outputs_on_cancel": draft.keep_outputs_on_cancel,
                    "board_profile_key": draft.board_profile_key,
                    "board_profile_mode": draft.board_profile_mode,
                }
            })
        } else {
            let id = draft.id.trim();
            if !crate::four_d::controller::valid_strip_effect_id(id) {
                return Err("Strip effect ID must use 1–64 lowercase letters, digits, dots, dashes, or underscores".to_string());
            }
            let program: serde_json::Value = serde_json::from_str(&draft.program_json)
                .map_err(|error| format!("Invalid lighting program JSON: {error}"))?;
            serde_json::json!({
                "reference": format!("effect:{id}"),
                "id": id,
                "name": name,
                "category": category,
                "icon": icon,
                "description": draft.description.trim(),
                "kind": "strip-stream",
                "engine": "host",
                "editable": true,
                "duration_ms": draft.duration_ms,
                "default_fps": draft.default_fps,
                "default_pixels": draft.default_pixels,
                "program": program,
            })
        };
        let encoded = serde_json::to_vec(&descriptor)
            .map_err(|error| format!("Encode effect definition: {error}"))?
            .into_iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let command = format!("effect upsert-json {encoded}");
        self.request_hardware_effect_command("effect-save", command)
    }

    pub(crate) fn delete_controller_effect(&mut self) -> Result<(), String> {
        let reference = self.effect_library_draft.reference.trim();
        if reference.is_empty() || self.effect_library_draft.is_new {
            return Err("Select a saved PCController effect first".to_string());
        }
        self.request_hardware_effect_command("effect-delete", format!("effect delete {reference}"))
    }

    pub(crate) fn save_controller_effect_group(
        &mut self,
        draft: ControllerEffectGroupDraft,
    ) -> Result<(), String> {
        let original = Self::controller_command_argument(&draft.original_name)
            .ok_or_else(|| "The original effect group name is invalid".to_string())?;
        let name = Self::controller_command_argument(&draft.name).ok_or_else(|| {
            "Effect group name must use 1–64 letters, numbers, spaces, dashes, or underscores"
                .to_string()
        })?;
        let icon = if draft.icon.trim().is_empty() {
            "-".to_string()
        } else {
            Self::controller_command_argument(&draft.icon).ok_or_else(|| {
                "Effect group icon must use 1–64 letters, numbers, spaces, dashes, or underscores"
                    .to_string()
            })?
        };
        self.request_hardware_effect_command(
            "effect-group-save",
            format!("effect group update {original} {name} {icon}"),
        )
    }

    pub(crate) fn play_controller_effect(&mut self, reference: &str) -> Result<(), String> {
        if reference.trim().is_empty() {
            return Err("Select a PCController effect first".to_string());
        }
        self.request_hardware_effect_command(
            "effect-play",
            format!("effect play {}", reference.trim()),
        )
    }

    pub(crate) fn stop_controller_effect(&mut self, reference: &str) -> Result<(), String> {
        if reference.trim().is_empty() {
            return Err("Select a PCController effect first".to_string());
        }
        self.request_hardware_effect_command(
            "effect-stop",
            format!("effect stop {}", reference.trim()),
        )
    }

    fn request_board_operation(
        &mut self,
        operation: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(), String> {
        if self.board_operation.is_some() {
            return Err("another board operation is still running".to_string());
        }
        if !self
            .advertised_hardware()
            .is_some_and(|capabilities| capabilities.board_connected)
        {
            return Err("no live board is connected".to_string());
        }
        self.engine_handle
            .request_controller_call(operation, method, params)?;
        self.board_operation = Some(operation.to_string());
        self.board_operation_status = "Waiting for PCController…".to_string();
        Ok(())
    }

    pub(crate) fn rename_board(&mut self) -> Result<(), String> {
        let name = self.board_name_draft.trim();
        if name.is_empty()
            || name.len() > 8
            || !name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || " -_".contains(character))
        {
            return Err(
                "Board name must be 1–8 ASCII letters, numbers, spaces, dashes, or underscores"
                    .to_string(),
            );
        }
        let name = Self::controller_command_argument(name)
            .ok_or_else(|| "Board name is not safe to send".to_string())?;
        self.request_board_operation(
            "board-name",
            "controller.command.execute",
            serde_json::json!({"command": format!("board name set {name}")}),
        )
    }

    pub(crate) fn save_board_settings(&mut self) -> Result<(), String> {
        let settings = self
            .board_settings_draft
            .as_ref()
            .ok_or_else(|| "No board settings are available to save".to_string())?;
        let command = board_settings_command(settings)?;
        self.request_board_operation(
            "board-settings",
            "controller.command.execute",
            serde_json::json!({"command": command}),
        )
    }

    pub(crate) fn set_status_led_override(
        &mut self,
        color: [u8; 3],
        brightness: u8,
    ) -> Result<(), String> {
        self.request_board_operation(
            "board-status-led-override",
            "controller.status_led.set",
            serde_json::json!({
                "red": color[0],
                "green": color[1],
                "blue": color[2],
                "brightness": brightness,
            }),
        )
    }

    pub(crate) fn release_status_led_override(&mut self) -> Result<(), String> {
        self.request_board_operation(
            "board-status-led-release",
            "controller.status_led.release",
            serde_json::json!({}),
        )
    }

    pub(crate) fn press_front_panel_key(&mut self, key: &str) -> Result<(), String> {
        let host_captured = self
            .engine_handle
            .hardware_capabilities
            .lock()
            .ok()
            .and_then(|capabilities| {
                capabilities
                    .as_ref()
                    .and_then(|value| value.front_panel.as_ref())
                    .map(|panel| panel.host_captured)
            })
            .unwrap_or(false);
        let command = front_panel_command(key, host_captured)?;
        self.front_panel_pending_key = Some(key.to_string());
        let result = self.request_board_operation(
            "board-front-panel-key",
            "controller.command.execute",
            serde_json::json!({"command": command}),
        );
        if result.is_err() {
            self.front_panel_pending_key = None;
        }
        result
    }

    pub(crate) fn refresh_front_panel(&mut self) -> Result<(), String> {
        self.front_panel_refresh_attempted = true;
        self.request_board_operation(
            "board-front-panel-refresh",
            "controller.front_panel",
            serde_json::json!({}),
        )
    }

    pub(crate) fn reboot_board(&mut self) -> Result<(), String> {
        self.request_board_operation(
            "board-reboot",
            "controller.reset",
            serde_json::json!({"pulse_ms": 50}),
        )
    }

    fn process_controller_call_results(&mut self) {
        let results = self
            .engine_handle
            .controller_call_results
            .lock()
            .ok()
            .map(|mut queue| queue.drain(..).collect::<Vec<_>>())
            .unwrap_or_default();
        for result in results {
            let is_board_operation = result.operation.starts_with("board-");
            let is_presentation_operation = result.operation.starts_with("presentation-");
            if is_board_operation {
                self.board_operation = None;
                self.board_reboot_armed = false;
                if result.operation == "board-front-panel-key" {
                    self.front_panel_pending_key = None;
                }
            } else if !is_presentation_operation {
                self.hardware_effect_authoring.pending_operation = None;
            }
            match result.result {
                Ok(value) => {
                    let front_panel_apply_error = if result.operation == "board-front-panel-refresh"
                    {
                        self.engine_handle
                            .hardware_capabilities
                            .lock()
                            .map_err(|_| "hardware catalog lock is unavailable".to_string())
                            .and_then(|mut current| {
                                current
                                    .as_mut()
                                    .ok_or_else(|| "hardware catalog is unavailable".to_string())?
                                    .apply_front_panel_state(&value)
                                    .map(|_| ())
                            })
                            .err()
                    } else {
                        None
                    };
                    let presentation_apply_error = if is_presentation_operation {
                        self.engine_handle
                            .hardware_capabilities
                            .lock()
                            .map_err(|_| "hardware catalog lock is unavailable".to_string())
                            .and_then(|mut current| {
                                current
                                    .as_mut()
                                    .ok_or_else(|| "hardware catalog is unavailable".to_string())?
                                    .apply_presentation_update(&value)
                                    .map(|_| ())
                            })
                            .err()
                    } else {
                        None
                    };
                    let output = value
                        .get("output")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(if result.operation.starts_with("presentation-order:") {
                            "Channel order saved"
                        } else if is_presentation_operation {
                            "Channel presentation saved"
                        } else if result.operation == "board-front-panel-refresh" {
                            "Physical front panel refreshed"
                        } else {
                            "PCController accepted the operation"
                        })
                        .to_string();
                    match result.operation.as_str() {
                        "macro-start" => self.hardware_effect_authoring.active = true,
                        "macro-status" => {
                            self.hardware_effect_authoring.active = output.contains("active=true");
                        }
                        "macro-save" => {
                            self.hardware_effect_authoring.active = false;
                            self.hardware_effect_authoring.pending_saved_macro_id =
                                saved_macro_id(&output);
                            self.engine_handle.request_catalog_refresh();
                        }
                        "macro-discard" => {
                            self.hardware_effect_authoring.active = false;
                            self.hardware_effect_authoring.pending_saved_macro_id = None;
                        }
                        "effect-preview" | "strip-rainbow" => {
                            self.hardware_effect_authoring.preview_active = true;
                            self.engine_handle.request_catalog_refresh();
                        }
                        "effect-stop" | "strip-stop" | "strip-clear" => {
                            self.hardware_effect_authoring.preview_active = false;
                            self.engine_handle.request_catalog_refresh();
                        }
                        "board-name"
                        | "board-settings"
                        | "board-status-led-override"
                        | "board-status-led-release"
                        | "board-reboot" => {
                            if result.operation == "board-settings" {
                                self.board_settings_dirty = false;
                            }
                            self.engine_handle.request_catalog_refresh();
                        }
                        "board-front-panel-key" => {
                            // The command acknowledgement proves delivery, but
                            // the follow-up exact read proves what the physical
                            // board is now showing after it processed the key.
                            if let Err(error) = self.refresh_front_panel() {
                                self.board_operation_status = error;
                            }
                        }
                        "board-front-panel-refresh" => {
                            self.board_operation_status = self.tr("Physical front panel refreshed");
                        }
                        "effect-save" | "effect-delete" | "strip-config" | "strip-fill"
                        | "strip-frame" | "strip-pixel" | "strip-status" => {
                            self.engine_handle.request_catalog_refresh();
                        }
                        _ => {}
                    }
                    if is_presentation_operation {
                        // The authoritative response has already updated the
                        // rendered catalog and revision. Refresh in the
                        // background to verify the complete catalog and to
                        // recover gracefully from an older controller that did
                        // not return the typed presentation payload.
                        self.engine_handle.request_catalog_refresh();
                    }
                    if is_board_operation {
                        if result.operation != "board-front-panel-refresh" {
                            self.board_operation_status = output.clone();
                        }
                    } else if !is_presentation_operation {
                        self.hardware_effect_authoring.status = output.clone();
                    }
                    if let Some(error) = front_panel_apply_error {
                        self.board_operation_status = error.clone();
                        self.set_osd(error);
                    } else if let Some(error) = presentation_apply_error {
                        self.set_osd(format!("Channel saved; refreshing details: {error}"));
                    } else {
                        self.set_osd(output);
                    }
                }
                Err(error) => {
                    if result.operation == "effect-preview" {
                        self.hardware_effect_authoring.preview_active = false;
                    }
                    if is_board_operation {
                        self.board_operation_status = error.clone();
                    } else if !is_presentation_operation {
                        self.hardware_effect_authoring.status = error.clone();
                    }
                    if is_presentation_operation {
                        self.engine_handle.request_catalog_refresh();
                    }
                    self.set_osd(error);
                }
            }
        }
        self.insert_pending_saved_macro();
    }

    fn insert_pending_saved_macro(&mut self) {
        let Some(id) = self.hardware_effect_authoring.pending_saved_macro_id else {
            return;
        };
        let Some(hardware_macro) = self
            .advertised_hardware()
            .and_then(|capabilities| capabilities.macros.into_iter().find(|item| item.id == id))
        else {
            return;
        };
        let effect = controller_macro_effect_preset(&hardware_macro).effect;
        let effect_id = effect.id;
        self.undo_stack.push(self.snapshot_timeline());
        self.timeline.templates.push(effect);
        let instance = crate::four_d::models::EffectInstance::new(
            effect_id,
            self.hardware_effect_authoring.anchor_ms,
        );
        self.selected_instance_ids.clear();
        self.selected_instance_ids.insert(instance.id);
        self.timeline.instances.push(instance);
        crate::ui::effects_library::select_sequence(self, &hardware_macro);
        self.show_effect_library_editor = true;
        self.hardware_effect_authoring.pending_saved_macro_id = None;
        self.hardware_effect_authoring.status = format!(
            "Saved '{}' and placed it at {:.3}s",
            hardware_macro.name,
            self.hardware_effect_authoring.anchor_ms as f64 / 1_000.0
        );
        self.sync_timeline_engine();
    }

    pub(crate) fn apply_interop_command(
        &mut self,
        ctx: &egui::Context,
        command: crate::platform::interop::InteropCommand,
        source: &str,
    ) {
        use crate::platform::interop::InteropCommand;

        match command {
            InteropCommand::Launch { request } => {
                let crate::platform::interop::LaunchRequest {
                    sender_working_directory,
                    target,
                    fullscreen,
                    volume,
                    activate,
                    commands,
                    ..
                } = request;
                if let Some(value) = volume {
                    let _ = self.mpv.set_property("volume", value);
                    self.volume = value;
                    self.save_config();
                }
                if let Some(target) = target {
                    if crate::media::is_remote_media_target(&target) {
                        self.load_media_target(&target);
                    } else {
                        let path = std::path::PathBuf::from(target);
                        let resolved = if path.is_relative() {
                            sender_working_directory
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
                if fullscreen {
                    self.set_fullscreen(ctx, true);
                }
                if activate {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                for command in commands {
                    self.apply_interop_command(ctx, command, source);
                }
            }
            InteropCommand::Play => self.play(),
            InteropCommand::Pause => self.pause(),
            InteropCommand::TogglePause => self.toggle_playback(),
            InteropCommand::Stop => self.close_video(),
            InteropCommand::Next => {
                let _ = self.mpv.command("playlist-next", &["force"]);
            }
            InteropCommand::Previous => {
                let _ = self.mpv.command("playlist-prev", &["force"]);
            }
            InteropCommand::Seek { seconds } => self.seek_relative(seconds),
            InteropCommand::SeekTo { seconds } => {
                if self.is_seekable {
                    let target = seconds.max(0.0).min(self.duration);
                    self.scrub_to(target);
                    self.finish_scrub(target);
                }
            }
            InteropCommand::SeekAbs { percentage } => {
                if self.is_seekable {
                    let clamped = percentage.clamp(0.0, 100.0);
                    let target = self.duration * (clamped / 100.0);
                    self.scrub_to(target);
                    self.finish_scrub(target);
                }
            }
            InteropCommand::SetVolume { value } => {
                let clamped = value.clamp(0.0, 130.0);
                let _ = self.mpv.set_property("volume", clamped);
                self.volume = clamped;
                self.save_config();
            }
            InteropCommand::SetMute { muted } => {
                let _ = self.mpv.set_property("mute", muted);
                self.is_muted = muted;
                self.save_config();
            }
            InteropCommand::ToggleMute => {
                let muted = !self.is_muted;
                let _ = self.mpv.set_property("mute", muted);
                self.is_muted = muted;
                self.save_config();
            }
            InteropCommand::SetRate { rate } => {
                let _ = self.mpv.set_property("speed", rate);
                self.playback_rate = rate;
            }
            InteropCommand::Open { target } => self.load_media_target(&target),
            InteropCommand::SetFullscreen { enabled } => self.set_fullscreen(ctx, enabled),
            InteropCommand::ToggleFullscreen => self.toggle_fullscreen(ctx),
            InteropCommand::Activate => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            InteropCommand::Minimize => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            InteropCommand::Maximize => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }
            InteropCommand::Restore => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            InteropCommand::OpenPreferences => {
                self.show_preferences_dialog = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            InteropCommand::OpenBoardInformation { tab } => {
                self.board_info_tab = tab.min(3);
                self.show_board_info_dialog = true;
                if self.board_info_tab == 2 {
                    self.front_panel_refresh_attempted = false;
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            InteropCommand::ShowMessage { message } => {
                self.set_osd(message);
                return;
            }
            InteropCommand::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            InteropCommand::SetWorkspace { profile } => {
                self.restore_workspace_profile(ctx, &profile);
            }
            InteropCommand::CreateWorkspaceProfile { name, icon } => {
                self.workspace_profile_icon_draft = icon;
                self.save_workspace_profile(ctx, &name);
            }
            InteropCommand::UpdateWorkspaceProfile {
                id,
                name,
                icon,
                capture,
            } => {
                self.update_workspace_profile_metadata(&id, &name, &icon);
                if capture {
                    self.overwrite_workspace_profile(ctx, &id);
                }
            }
            InteropCommand::DeleteWorkspaceProfile { id } => {
                self.delete_workspace_profile(&id);
            }
            InteropCommand::MoveWorkspaceProfile { id, direction } => {
                self.move_workspace_profile(&id, direction);
            }
            InteropCommand::AddEffectCue {
                effect_id,
                start_time_ms,
            } => {
                let Ok(effect_id) = uuid::Uuid::parse_str(&effect_id) else {
                    self.set_osd(self.tr("Effect is no longer available"));
                    return;
                };
                if !self
                    .timeline
                    .templates
                    .iter()
                    .any(|effect| effect.id == effect_id)
                {
                    self.set_osd(self.tr("Effect is no longer available"));
                    return;
                }
                let instance = crate::four_d::models::EffectInstance::new(effect_id, start_time_ms);
                self.selected_instance_ids.clear();
                self.selected_instance_ids.insert(instance.id);
                self.timeline.instances.push(instance);
                self.sync_timeline_engine();
            }
            InteropCommand::RemoveEffectCue { instance_id } => {
                let Ok(instance_id) = uuid::Uuid::parse_str(&instance_id) else {
                    self.set_osd(self.tr("Cue is no longer available"));
                    return;
                };
                let previous_len = self.timeline.instances.len();
                self.timeline
                    .instances
                    .retain(|instance| instance.id != instance_id);
                self.selected_instance_ids.remove(&instance_id);
                if previous_len != self.timeline.instances.len() {
                    self.sync_timeline_engine();
                }
            }
            InteropCommand::AddControllerEffectCue {
                reference,
                start_time_ms,
            } => {
                let preset = self.advertised_effect_presets().into_iter().find(|preset| {
                    let candidate = match preset.source {
                        EffectPresetSource::ControllerMacro(id) => format!("effect:{id}"),
                        EffectPresetSource::ControllerStrip => preset
                            .effect
                            .controller_strip_effect
                            .as_ref()
                            .map(|effect| format!("effect:{}", effect.id))
                            .unwrap_or_default(),
                    };
                    candidate == reference
                });
                let Some(preset) = preset else {
                    self.set_osd(self.tr("Effect is no longer available"));
                    return;
                };
                let effect_id = preset.effect.id;
                self.timeline.templates.push(preset.effect);
                let instance = crate::four_d::models::EffectInstance::new(effect_id, start_time_ms);
                self.selected_instance_ids.clear();
                self.selected_instance_ids.insert(instance.id);
                self.timeline.instances.push(instance);
                self.sync_timeline_engine();
            }
            InteropCommand::PlayControllerEffect { reference } => {
                if let Err(error) = self.play_controller_effect(&reference) {
                    self.set_osd(error);
                    return;
                }
            }
            InteropCommand::StopControllerEffect => {
                if let Err(error) = self.stop_strip_preview() {
                    self.set_osd(error);
                    return;
                }
            }
            InteropCommand::DeleteControllerEffect { reference } => {
                self.effect_library_draft.reference = reference;
                self.effect_library_draft.is_new = false;
                if let Err(error) = self.delete_controller_effect() {
                    self.set_osd(error);
                    return;
                }
            }
            InteropCommand::SaveControllerEffect { effect } => {
                let steps = if effect.kind == "sequence" {
                    let value = effect
                        .program
                        .get("steps")
                        .cloned()
                        .unwrap_or_else(|| effect.program.clone());
                    match serde_json::from_value(value) {
                        Ok(steps) => steps,
                        Err(error) => {
                            self.set_osd(format!("Invalid effect sequence: {error}"));
                            return;
                        }
                    }
                } else {
                    Vec::new()
                };
                let properties = effect.program.get("properties");
                let property_string = |name: &str| {
                    properties
                        .and_then(|value| value.get(name))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string()
                };
                self.effect_library_draft = ControllerEffectDraft {
                    reference: effect.reference,
                    id: effect.id,
                    name: effect.name,
                    category: effect.category,
                    icon: effect.icon,
                    description: effect.description,
                    kind: effect.kind,
                    program_json: serde_json::to_string_pretty(&effect.program)
                        .unwrap_or_else(|_| "{}".to_string()),
                    color: effect.color,
                    default_fps: effect.default_fps,
                    duration_ms: effect.duration_ms,
                    default_pixels: effect.default_pixels,
                    engine: properties
                        .and_then(|value| value.get("mode"))
                        .and_then(serde_json::Value::as_str)
                        .filter(|mode| matches!(*mode, "host" | "mcu"))
                        .unwrap_or("host")
                        .to_string(),
                    steps,
                    label: property_string("label"),
                    lcd_message: property_string("lcd_message"),
                    timing_tolerance_us: properties
                        .and_then(|value| value.get("timing_tolerance_us"))
                        .and_then(serde_json::Value::as_u64)
                        .and_then(|value| u32::try_from(value).ok())
                        .unwrap_or_default(),
                    keep_outputs_on_cancel: properties
                        .and_then(|value| value.get("keep_outputs_on_cancel"))
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false),
                    board_profile_key: property_string("board_profile_key"),
                    board_profile_mode: property_string("board_profile_mode"),
                    is_new: effect.is_new,
                };
                if let Err(error) = self.save_controller_effect() {
                    self.set_osd(error);
                    return;
                }
            }
            InteropCommand::SetRecording { enabled } => {
                if self.is_recording && !enabled {
                    self.commit_recorded_samples();
                }
                for track in &mut self.timeline.analog_tracks {
                    track.armed = enabled;
                }
                if !enabled {
                    self.is_recording = false;
                } else if self.timeline.analog_tracks.is_empty() {
                    self.set_osd(self.tr("No recordable hardware tracks are available"));
                }
            }
            InteropCommand::SetEmergencyStop { active } => {
                self.set_emergency_stop(active);
                return;
            }
            InteropCommand::InvokeHardwareAction { action_id } => {
                let Some(capabilities) = self
                    .advertised_hardware()
                    .filter(|capabilities| capabilities.board_connected)
                else {
                    self.set_osd(self.tr("No board is connected or advertising live controls"));
                    return;
                };
                if let Err(error) = crate::ui::hardware_control::invoke_action_by_id(
                    self,
                    &capabilities,
                    &action_id,
                ) {
                    self.set_osd(error);
                    return;
                }
            }
            InteropCommand::SetHardwarePwm { channel, percent } => {
                let value = (percent.clamp(0.0, 100.0) * 4095.0 / 100.0).round() as u16;
                let _ = self.engine_handle.sender.send(
                    crate::four_d::engine::EngineMessage::ControllerCall {
                        method: "controller.pwm.set".to_string(),
                        params: serde_json::json!({"channel": channel, "value": value}),
                    },
                );
            }
            InteropCommand::UpdateHardwarePresentation { key, fields } => {
                let Some(capabilities) = self
                    .advertised_hardware()
                    .filter(|capabilities| capabilities.board_connected)
                else {
                    self.set_osd(self.tr("No board is connected or advertising live controls"));
                    return;
                };
                let Some(control) = capabilities
                    .controls
                    .iter()
                    .find(|control| control.key == key)
                    .cloned()
                else {
                    self.set_osd(format!("Unknown hardware channel: {key}"));
                    return;
                };
                crate::ui::layout::update_control_presentation(
                    self,
                    &capabilities,
                    &control,
                    "presentation-web",
                    fields,
                );
            }
            InteropCommand::ConfigureAddressableStrip { pixels } => {
                if let Err(error) = self.configure_addressable_strip(pixels) {
                    self.set_osd(error);
                    return;
                }
            }
            InteropCommand::FillAddressableStrip {
                red,
                green,
                blue,
                brightness,
            } => {
                if let Err(error) = self.fill_addressable_strip(red, green, blue, brightness) {
                    self.set_osd(error);
                    return;
                }
            }
            InteropCommand::ClearAddressableStrip => {
                if let Err(error) = self.clear_addressable_strip() {
                    self.set_osd(error);
                    return;
                }
            }
            InteropCommand::PressFrontPanelKey { key } => {
                if let Err(error) = self.press_front_panel_key(&key) {
                    self.set_osd(error);
                    return;
                }
            }
            InteropCommand::UpdateConfig { values } => {
                match self.apply_config_patch(ctx, &values) {
                    Ok(()) => self.set_osd(self.tr("Preferences updated")),
                    Err(error) => {
                        self.config_status = error.clone();
                        self.set_osd(format!("{}: {error}", self.tr("Preferences update failed")));
                    }
                }
                return;
            }
            InteropCommand::PreviewConfig { config } => {
                if let Err(error) = self.preview_runtime_config(ctx, *config) {
                    self.config_status = error;
                }
                return;
            }
            InteropCommand::CommitPreviewConfig { config } => {
                if let Err(error) = self.commit_preference_preview(ctx, *config) {
                    self.config_status = error;
                }
                return;
            }
            InteropCommand::CancelPreviewConfig => {
                if let Err(error) = self.cancel_preference_preview(ctx) {
                    self.config_status = error;
                }
                return;
            }
            InteropCommand::ReloadConfig => match self.reload_config_from_disk(ctx) {
                Ok(()) => {
                    self.set_osd(self.tr("Preferences reloaded from disk"));
                    return;
                }
                Err(error) => {
                    self.config_status = error.clone();
                    self.set_osd(format!("{}: {error}", self.tr("Preferences reload failed")));
                    return;
                }
            },
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
                    (14, PropertyData::Flag(v)) => self.is_seekable = v,
                    (15, PropertyData::Double(v)) => self.cache_duration = Some(v.max(0.0)),
                    (16, PropertyData::Int64(v)) => {
                        self.cache_buffering_percent = Some((v as f64).clamp(0.0, 100.0));
                    }
                    (17, PropertyData::Double(v)) => self.playback_rate = v,
                    (18, PropertyData::Str(v)) => self.current_vid = v.to_string(),
                    (18, PropertyData::OsdStr(v)) => self.current_vid = v.to_string(),
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
                    self.playback_time = 0.0;
                    self.duration = 0.0;
                    self.is_seekable = false;
                    self.media_metadata_loaded = false;
                    self.cache_duration = None;
                    self.cache_buffering_percent = None;
                    self.seek_pos = None;
                    self.refresh_sub_tracks();
                    self.refresh_audio_tracks();
                    self.refresh_video_tracks();
                }
                Some(Ok(Event::FileLoaded)) => {
                    self.media_metadata_loaded = true;
                    if let Ok(duration) = self.mpv.get_property::<f64>("duration")
                        && duration.is_finite()
                        && duration > 0.0
                    {
                        self.duration = duration;
                    }
                    if let Ok(seekable) = self.mpv.get_property::<bool>("seekable") {
                        self.is_seekable = seekable;
                    }
                    if let Some(position) = self.pending_resume_position.take() {
                        if self.is_seekable {
                            let target = if self.duration > 0.0 {
                                position.min((self.duration - 0.25).max(0.0))
                            } else {
                                position
                            };
                            if target >= 1.0
                                && self
                                    .mpv
                                    .command("seek", &[&target.to_string(), "absolute+exact"])
                                    .is_ok()
                            {
                                self.playback_time = target;
                                self.set_osd(format!(
                                    "{} {}",
                                    self.tr("Resumed at"),
                                    crate::ui::controls::format_player_time(
                                        target,
                                        self.duration >= 3600.0,
                                        true,
                                    )
                                ));
                            }
                        }
                    }
                    self.refresh_sub_tracks();
                    self.refresh_audio_tracks();
                    self.refresh_video_tracks();
                }
                Some(Ok(_)) => {}
                _ => break,
            }
        }
        if self.last_playback_position_checkpoint.elapsed() >= std::time::Duration::from_secs(5) {
            self.last_playback_position_checkpoint = std::time::Instant::now();
            if self.capture_current_playback_position() {
                self.save_config();
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
        self.save_config();
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
            self.save_config();
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
        self.save_config();
    }

    pub(crate) fn set_emergency_stop(&mut self, active: bool) {
        if self.estop_active == active {
            return;
        }
        self.estop_active = active;
        self.held_motion_action = None;
        self.engine_handle.set_emergency_stop(active);
        if active {
            self.pause();
            self.set_osd(self.tr("E-STOP ACTIVE"));
        }
    }

    /// Request an E-STOP state change from an interactive GUI control.
    ///
    /// Engaging the latch is immediate. Releasing it can require a persisted
    /// confirmation, while API/CLI commands continue to use
    /// [`Self::set_emergency_stop`] directly for deterministic automation.
    pub(crate) fn request_emergency_stop_change(&mut self, active: bool) {
        if self.estop_active == active {
            return;
        }
        if !active && self.confirm_estop_release {
            self.skip_estop_release_confirmation_draft = false;
            self.show_estop_release_dialog = true;
        } else {
            self.set_emergency_stop(active);
        }
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
        self.desired_fullscreen
            .unwrap_or_else(|| ctx.input(|input| input.viewport().fullscreen.unwrap_or(false)))
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

    pub fn is_tab_open(&self, tab: crate::ui::layout::PealayerTab) -> bool {
        self.dock_state.find_tab(&tab).is_some()
    }

    pub fn open_or_focus_tab(&mut self, tab: crate::ui::layout::PealayerTab) {
        self.show_four_d_editor = true;
        if crate::ui::layout::reveal_and_focus_tab(&mut self.dock_state, tab) {
            self.save_dock_layout();
        }
    }

    pub fn toggle_tab(&mut self, tab: crate::ui::layout::PealayerTab) {
        if let Some(path) = self.dock_state.find_tab(&tab) {
            self.dock_state.remove_tab(path);
            self.save_dock_layout();
        } else {
            self.open_or_focus_tab(tab);
        }
    }

    pub fn save_dock_layout(&mut self) {
        crate::ui::layout::sanitize_dock_rects(&mut self.dock_state);
        self.save_config();
    }

    /// Capture every durable part of the current desktop workspace. egui's
    /// memory contributes movable dialog rectangles, active widgets, and
    /// stable ScrollArea offsets; application fields supply the surfaces that
    /// are currently open and their selected pages.
    pub(crate) fn capture_workspace_profile(
        &self,
        ctx: &egui::Context,
    ) -> crate::config::WorkspaceProfile {
        let mut dock_state = self.dock_state.clone();
        crate::ui::layout::sanitize_dock_rects(&mut dock_state);
        crate::config::WorkspaceProfile {
            name: String::new(),
            icon: String::new(),
            order: 0,
            nle: self.show_four_d_editor,
            window_geometry: self.window_geometry,
            dock_layout: serde_json::to_string(&dock_state).ok(),
            dialogs: crate::config::WorkspaceDialogs {
                subtitles: self.show_sub_settings,
                audio: self.show_audio_settings,
                open_location: self.show_open_url_dialog,
                shortcuts: self.show_shortcuts_dialog,
                about: self.show_about_dialog,
                about_tab: self.about_tab,
                preferences: self.show_preferences_dialog,
                preferences_tab: self.preferences_tab,
                board_information: self.show_board_info_dialog,
                board_information_tab: self.board_info_tab,
                channel_manager: self.show_hardware_channels_dialog,
                hardware_control_key: self.hardware_control_dialog_key.clone(),
                hardware_channel_detail_active: self.hardware_channel_detail_active,
                effects_manager: self.show_effect_library_editor,
                effects_selection: self.effect_library_selection.clone(),
                workspace_profiles: self.show_workspace_profiles_dialog,
            },
            egui_memory: ctx.memory(|memory| serde_json::to_string(memory).ok()),
        }
    }

    pub(crate) fn apply_workspace_profile(
        &mut self,
        ctx: &egui::Context,
        profile: &crate::config::WorkspaceProfile,
    ) -> Result<(), String> {
        let dock_state = profile
            .dock_layout
            .as_deref()
            .map(|json| {
                serde_json::from_str::<egui_dock::DockState<crate::ui::layout::PealayerTab>>(json)
                    .map_err(|error| format!("Could not restore workspace layout: {error}"))
            })
            .transpose()?;
        let memory = profile
            .egui_memory
            .as_deref()
            .map(|json| {
                serde_json::from_str::<egui::Memory>(json)
                    .map_err(|error| format!("Could not restore workspace positions: {error}"))
            })
            .transpose()?;

        self.show_four_d_editor = profile.nle;
        if let Some(mut dock_state) = dock_state {
            crate::ui::layout::sanitize_dock_rects(&mut dock_state);
            self.dock_state = dock_state;
        }
        let dialogs = &profile.dialogs;
        self.show_sub_settings = dialogs.subtitles;
        self.show_audio_settings = dialogs.audio;
        self.show_open_url_dialog = dialogs.open_location;
        self.show_shortcuts_dialog = dialogs.shortcuts;
        self.show_about_dialog = dialogs.about;
        self.about_tab = dialogs.about_tab;
        self.show_preferences_dialog = dialogs.preferences;
        self.preferences_tab = dialogs.preferences_tab;
        self.show_board_info_dialog = dialogs.board_information;
        self.board_info_tab = dialogs.board_information_tab;
        self.show_hardware_channels_dialog = dialogs.channel_manager;
        self.hardware_control_dialog_key = dialogs.hardware_control_key.clone();
        self.hardware_channel_detail_active = dialogs.hardware_channel_detail_active;
        self.show_effect_library_editor = dialogs.effects_manager;
        self.effect_library_selection = dialogs.effects_selection.clone();
        // Keep this manager open while switching profiles so the user can
        // immediately compare or return to another saved arrangement.
        self.show_workspace_profiles_dialog =
            self.show_workspace_profiles_dialog || dialogs.workspace_profiles;

        if let Some(geometry) = profile.window_geometry.filter(|value| value.is_valid()) {
            self.window_geometry = Some(geometry);
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                geometry.x, geometry.y,
            )));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                geometry.width,
                geometry.height,
            )));
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(geometry.maximized));
        }
        if let Some(memory) = memory {
            ctx.memory_mut(|current| *current = memory);
        }
        ctx.request_repaint();
        Ok(())
    }

    pub(crate) fn ordered_workspace_profiles(
        &self,
    ) -> Vec<(String, crate::config::WorkspaceProfile)> {
        let mut profiles = self
            .workspace_profiles
            .iter()
            .map(|(id, profile)| (id.clone(), profile.clone()))
            .collect::<Vec<_>>();
        profiles.sort_by(|(left_id, left), (right_id, right)| {
            left.order
                .cmp(&right.order)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                .then_with(|| left_id.cmp(right_id))
        });
        profiles
    }

    fn compact_workspace_profile_order(&mut self) {
        let ordered_ids = self
            .ordered_workspace_profiles()
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for (order, id) in ordered_ids.into_iter().enumerate() {
            if let Some(profile) = self.workspace_profiles.get_mut(&id) {
                profile.order = order as i32;
            }
        }
    }

    pub(crate) fn active_workspace_name(&self) -> String {
        self.active_workspace_profile
            .as_ref()
            .and_then(|id| self.workspace_profiles.get(id))
            .map(|profile| profile.name.clone())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| self.tr("Workspace"))
    }

    pub(crate) fn save_workspace_profile(&mut self, ctx: &egui::Context, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            self.config_status = self.tr("Enter a workspace profile name");
            return;
        }
        let mut profile = self.capture_workspace_profile(ctx);
        profile.name = name.to_string();
        profile.icon = if self.workspace_profile_icon_draft.trim().is_empty() {
            "window".to_string()
        } else {
            self.workspace_profile_icon_draft.trim().to_string()
        };
        profile.order = self
            .workspace_profiles
            .values()
            .map(|profile| profile.order)
            .max()
            .unwrap_or(-1)
            + 1;
        let id = format!("workspace-{}", uuid::Uuid::new_v4().simple());
        self.workspace_profiles.insert(id.clone(), profile);
        self.active_workspace_profile = Some(id);
        self.workspace_profile_name_draft.clear();
        self.config_status = format!("{}: {name}", self.tr("Workspace profile saved"));
        self.save_config();
    }

    pub(crate) fn restore_workspace_profile(&mut self, ctx: &egui::Context, id: &str) {
        let Some(profile) = self.workspace_profiles.get(id).cloned() else {
            self.config_status = format!("{}: {id}", self.tr("Workspace profile not found"));
            return;
        };
        match self.apply_workspace_profile(ctx, &profile) {
            Ok(()) => {
                if ctx.input(|input| input.viewport().fullscreen.unwrap_or(false)) {
                    self.workspace_before_fullscreen = Some(profile.nle);
                    self.show_four_d_editor = false;
                }
                self.active_workspace_profile = Some(id.to_string());
                self.config_status = format!(
                    "{}: {}",
                    self.tr("Workspace profile restored"),
                    profile.name
                );
                self.save_config();
            }
            Err(error) => self.config_status = error,
        }
    }

    pub(crate) fn update_workspace_profile_metadata(&mut self, id: &str, name: &str, icon: &str) {
        let name = name.trim();
        let icon = icon.trim();
        if name.is_empty() || icon.is_empty() {
            return;
        }
        if let Some(profile) = self.workspace_profiles.get_mut(id) {
            profile.name = name.to_string();
            profile.icon = icon.to_string();
            self.config_status = format!("{}: {name}", self.tr("Workspace profile saved"));
            self.save_config();
        }
    }

    pub(crate) fn overwrite_workspace_profile(&mut self, ctx: &egui::Context, id: &str) {
        let Some(previous) = self.workspace_profiles.get(id).cloned() else {
            return;
        };
        let mut profile = self.capture_workspace_profile(ctx);
        profile.name = previous.name.clone();
        profile.icon = previous.icon;
        profile.order = previous.order;
        self.workspace_profiles.insert(id.to_string(), profile);
        self.active_workspace_profile = Some(id.to_string());
        self.config_status = format!("{}: {}", self.tr("Workspace profile saved"), previous.name);
        self.save_config();
    }

    pub(crate) fn move_workspace_profile(&mut self, id: &str, direction: i32) {
        let profiles = self.ordered_workspace_profiles();
        let Some(index) = profiles.iter().position(|(candidate, _)| candidate == id) else {
            return;
        };
        let target = if direction < 0 {
            index.checked_sub(1)
        } else if direction > 0 && index + 1 < profiles.len() {
            Some(index + 1)
        } else {
            None
        };
        let Some(target) = target else { return };
        let other_id = &profiles[target].0;
        let current_order = profiles[index].1.order;
        let other_order = profiles[target].1.order;
        if let Some(profile) = self.workspace_profiles.get_mut(id) {
            profile.order = other_order;
        }
        if let Some(profile) = self.workspace_profiles.get_mut(other_id) {
            profile.order = current_order;
        }
        self.save_config();
    }

    pub(crate) fn cycle_workspace_profile(&mut self, ctx: &egui::Context) {
        let profiles = self.ordered_workspace_profiles();
        if profiles.is_empty() {
            return;
        }
        let current = self
            .active_workspace_profile
            .as_ref()
            .and_then(|active| profiles.iter().position(|(id, _)| id == active));
        let next = current
            .map(|index| (index + 1) % profiles.len())
            .unwrap_or(0);
        self.restore_workspace_profile(ctx, &profiles[next].0);
    }

    pub(crate) fn delete_workspace_profile(&mut self, id: &str) {
        if let Some(profile) = self.workspace_profiles.remove(id) {
            if self.active_workspace_profile.as_deref() == Some(id) {
                self.active_workspace_profile = None;
            }
            self.compact_workspace_profile_order();
            self.config_status =
                format!("{}: {}", self.tr("Workspace profile deleted"), profile.name);
            self.save_config();
        }
    }

    /// Performs an exact relative seek by the given number of seconds.
    pub fn seek_relative(&mut self, seconds: f64) {
        if self.current_video_path.is_none() || !self.is_seekable {
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

    /// Seeks to an exact absolute timestamp while preserving the normal
    /// non-blocking seek path used by the timeline scrubber.
    pub fn seek_absolute(&mut self, seconds: f64) {
        if self.current_video_path.is_none() || !self.is_seekable || !seconds.is_finite() {
            return;
        }
        let target = seconds.clamp(0.0, self.duration.max(0.0));
        self.finish_scrub(target);
        self.set_osd(format!(
            "{}: {}",
            self.tr("Seek"),
            crate::ui::controls::format_player_time(target, self.duration >= 3600.0, true)
        ));
    }

    /// Advances or reverses playback by the configured number of frames.
    pub fn step_frames(&mut self, direction: i32) {
        if self.current_video_path.is_none() || direction == 0 {
            return;
        }
        let command = if direction > 0 {
            "frame-step"
        } else {
            "frame-back-step"
        };
        let count = self.frame_step_count.clamp(1, 120);
        for _ in 0..count {
            let _ = self.mpv.command(command, &[]);
        }
        self.set_osd(format!(
            "{}: {:+}",
            self.tr("Frame step"),
            direction.signum() * count as i32
        ));
    }

    /// Begins or updates an active scrub session.
    /// Sets seek_pos, pauses playback smoothly during drag, and dispatches a non-blocking
    /// preview seek to the background worker thread with latest-target coalescing.
    pub fn scrub_to(&mut self, target_time: f64) {
        if !self.is_seekable {
            return;
        }
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
        if !self.is_seekable {
            return;
        }
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

    pub(crate) fn refresh_video_tracks(&mut self) {
        self.video_tracks.clear();
        if let Ok(count) = self.mpv.get_property::<i64>("track-list/count") {
            for i in 0..count {
                let track_type_prop = format!("track-list/{}/type", i);
                if let Ok(track_type) = self.mpv.get_property::<String>(&track_type_prop)
                    && track_type == "video"
                {
                    let id_prop = format!("track-list/{}/id", i);
                    let lang_prop = format!("track-list/{}/lang", i);
                    let title_prop = format!("track-list/{}/title", i);

                    if let Ok(id) = self.mpv.get_property::<i64>(&id_prop) {
                        let lang = self.mpv.get_property::<String>(&lang_prop).ok();
                        let title = self.mpv.get_property::<String>(&title_prop).ok();
                        self.video_tracks.push(VideoTrack { id, title, lang });
                    }
                }
            }
        }
    }

    pub fn load_video_file(&mut self, path: std::path::PathBuf) {
        let path_str = path.to_str().unwrap_or("");
        if !path_str.is_empty() {
            self.capture_current_playback_position();
            self.pending_resume_position = self.resume_position_for(path_str);
            let _ = self.mpv.set_property("keep-open", "always");
            let _ = self.mpv.command("loadfile", &[path_str, "replace"]);
            self.current_video_path = Some(path.clone());
            self.last_media_target = Some(path.clone());
            self.is_eof = false;
            self.is_paused = false;
            self.playback_time = 0.0;
            self.duration = 0.0;
            self.is_seekable = false;
            self.media_metadata_loaded = false;
            self.cache_duration = None;
            self.cache_buffering_percent = None;
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
            self.capture_current_playback_position();
            self.pending_resume_position = self.resume_position_for(trimmed);
            let _ = self.mpv.set_property("keep-open", "always");
            let _ = if crate::media::prefers_rtsp_tcp(trimmed) {
                // TCP interleaving is materially more reliable for surveillance
                // cameras crossing Windows firewalls/NAT and avoids short UDP
                // sessions being mistaken for finite clips.
                self.mpv.command(
                    "loadfile",
                    &[
                        trimmed,
                        "replace",
                        "-1",
                        "demuxer-lavf-o=rtsp_transport=tcp",
                    ],
                )
            } else {
                self.mpv.command("loadfile", &[trimmed, "replace"])
            };
            let path = std::path::PathBuf::from(trimmed);
            self.current_video_path = Some(path.clone());
            self.last_media_target = Some(path.clone());
            self.is_eof = false;
            self.is_paused = false;
            self.playback_time = 0.0;
            self.duration = 0.0;
            self.is_seekable = false;
            self.media_metadata_loaded = false;
            self.cache_duration = None;
            self.cache_buffering_percent = None;
            self.seek_pos = None;
            self.add_recent_media(path);
            let safe_target = crate::media::redact_media_target(trimmed);
            if let Some(ref mut mc) = self.media_controls {
                mc.update_metadata(Some(&safe_target));
            }
            self.set_osd(format!("Loading URL: {safe_target}"));
        }
    }

    pub(crate) fn synchronize_playback_proxy(&self) -> Result<(), String> {
        crate::mpv::proxy::apply_runtime(
            self.mpv,
            self.open_url_use_proxy,
            &self.open_url_proxy_url,
        )
    }

    pub fn load_media_target(&mut self, target: &str) {
        if crate::media::is_remote_media_target(target) {
            self.load_url(target);
        } else {
            self.load_video_file(std::path::PathBuf::from(target));
        }
    }

    pub fn media_timeline_state(&self) -> crate::media::MediaTimelineState {
        let target = self
            .current_video_path
            .as_ref()
            .map(|path| path.to_string_lossy());
        crate::media::timeline_state(
            target.as_deref(),
            self.media_metadata_loaded,
            self.duration,
            self.is_seekable,
        )
    }

    pub fn is_live_media(&self) -> bool {
        self.media_timeline_state() == crate::media::MediaTimelineState::Live
    }

    pub fn buffered_until(&self) -> Option<f64> {
        crate::media::buffered_until(self.duration, self.playback_time, self.cache_duration)
    }

    pub fn close_video(&mut self) {
        self.capture_current_playback_position();
        let _ = self.mpv.command("stop", &[]);
        self.current_video_path = None;
        self.last_media_target = None;
        self.playback_time = 0.0;
        self.duration = 0.0;
        self.is_seekable = false;
        self.media_metadata_loaded = false;
        self.cache_duration = None;
        self.cache_buffering_percent = None;
        self.is_eof = false;
        self.is_paused = false;
        self.seek_pos = None;
        if let Some(ref mut mc) = self.media_controls {
            mc.update_metadata(None);
            mc.update_playback(false, 0.0, 0.0);
        }
        self.set_osd("Video Closed".to_string());
        self.save_config();
    }

    pub(crate) fn runtime_config_snapshot(&self) -> crate::config::AppConfig {
        // Preserve deployment-owned branding while saving mutable player
        // preferences through one typed configuration contract.
        let mut cfg = crate::config::AppConfig::load();
        cfg.volume = self.volume;
        cfg.is_muted = self.is_muted;
        cfg.pin_controls = self.pin_controls;
        cfg.show_remaining_time = self.show_remaining_time;
        cfg.open_url_multiline = self.open_url_multiline;
        cfg.open_url_history_expanded = self.open_url_history_expanded;
        cfg.open_url_recent_click_edits = self.open_url_recent_click_edits;
        cfg.open_url_fetch_remote_info = self.open_url_fetch_remote_info;
        cfg.open_url_fetch_remote_thumbnail = self.open_url_fetch_remote_thumbnail;
        cfg.open_url_use_proxy = self.open_url_use_proxy;
        cfg.open_url_proxy_url = (!self.open_url_proxy_url.trim().is_empty())
            .then(|| self.open_url_proxy_url.trim().to_string());
        cfg.recent_media = self.recent_media.clone();
        cfg.last_media_target = self.last_media_target.clone();
        cfg.last_media_paused = self.current_video_path.is_some() && self.is_paused;
        cfg.restore_last_media_on_startup = self.restore_last_media_on_startup;
        cfg.remember_playback_position = self.remember_playback_position;
        cfg.playback_position_history_limit = self.playback_position_history_limit;
        cfg.playback_positions = self.playback_positions.clone();
        cfg.language = self.language_preference;
        cfg.direction = self.direction_preference;
        cfg.theme = self.theme_preference;
        cfg.hardware_endpoint =
            (!self.serial_port.trim().is_empty()).then(|| self.serial_port.clone());
        cfg.auto_connect_hardware = self.auto_connect_hardware;
        cfg.pause_on_hardware_disconnect = self.pause_on_hardware_disconnect;
        cfg.click_player_to_toggle = self.click_player_to_toggle;
        cfg.show_subseconds = self.show_subseconds;
        cfg.quick_seek_seconds = self.quick_seek_seconds;
        cfg.frame_step_count = self.frame_step_count;
        cfg.wheel_seek_seconds = self.wheel_seek_seconds;
        cfg.osd_position = self.osd_position;
        cfg.osd_timeout_seconds = self.osd_timeout_seconds;
        cfg.paused_drag_action = self.paused_drag_action;
        cfg.playing_drag_action = self.playing_drag_action;
        cfg.fullscreen_video_background = self.fullscreen_video_background;
        cfg.motion_control_mode = self.motion_control_mode;
        cfg.compact_hardware_controls = self.compact_hardware_controls;
        cfg.non_user_control_visibility = self.non_user_control_visibility;
        cfg.prefix_relay_identifiers = self.prefix_relay_identifiers;
        cfg.live_pwm_updates = self.live_pwm_updates;
        cfg.hardware_actions_on_press = self.hardware_actions_on_press;
        cfg.hardware_key_bindings = self.hardware_key_bindings.clone();
        cfg.show_estop_control = self.show_estop_control;
        cfg.confirm_estop_release = self.confirm_estop_release;
        cfg.single_instance = self.single_instance;
        cfg.windows_mica_backdrop = self.windows_mica_backdrop;
        cfg.windows_dwm_theming = self.windows_dwm_theming;
        cfg.opengl_vsync = self.opengl_vsync;
        cfg.native_dialog_windows = self.native_dialog_windows;
        cfg.auto_reload_config = self.auto_reload_config;
        cfg.status_bar = self.status_bar;
        cfg.window_geometry = self.window_geometry;
        let mut dock_state = self.dock_state.clone();
        crate::ui::layout::sanitize_dock_rects(&mut dock_state);
        if let Ok(json) = serde_json::to_string(&dock_state) {
            cfg.workspace_dock_layout = Some(json);
        }
        cfg.workspace_session = crate::config::WorkspaceProfile {
            name: String::new(),
            icon: String::new(),
            order: 0,
            nle: self.show_four_d_editor,
            window_geometry: self.window_geometry,
            dock_layout: cfg.workspace_dock_layout.clone(),
            dialogs: crate::config::WorkspaceDialogs {
                subtitles: self.show_sub_settings,
                audio: self.show_audio_settings,
                open_location: self.show_open_url_dialog,
                shortcuts: self.show_shortcuts_dialog,
                about: self.show_about_dialog,
                about_tab: self.about_tab,
                preferences: self.show_preferences_dialog,
                preferences_tab: self.preferences_tab,
                board_information: self.show_board_info_dialog,
                board_information_tab: self.board_info_tab,
                channel_manager: self.show_hardware_channels_dialog,
                hardware_control_key: self.hardware_control_dialog_key.clone(),
                hardware_channel_detail_active: self.hardware_channel_detail_active,
                effects_manager: self.show_effect_library_editor,
                effects_selection: self.effect_library_selection.clone(),
                workspace_profiles: self.show_workspace_profiles_dialog,
            },
            // The automatic last-session memory is owned by eframe's native
            // persistence store. Named profiles carry their own snapshots.
            egui_memory: None,
        };
        cfg.workspace_profiles = self.workspace_profiles.clone();
        cfg.active_workspace_profile = self.active_workspace_profile.clone();
        cfg
    }

    pub fn save_config(&mut self) {
        if self.preference_preview_original.is_some() {
            log::debug!(
                "Deferring automatic configuration save while Preferences is previewing changes"
            );
            return;
        }
        self.capture_current_playback_position();
        let cfg = self.runtime_config_snapshot();
        match cfg.save() {
            Ok(()) => {
                self.config_fingerprint = crate::config::AppConfig::fingerprint(
                    &crate::config::AppConfig::get_config_path(),
                )
                .ok();
                self.config_status = format!(
                    "Saved {}",
                    crate::config::display_config_path(&crate::config::AppConfig::get_config_path())
                );
                crate::platform::interop::set_live_config(cfg);
            }
            Err(error) => {
                self.config_status = error.clone();
                log::error!("Could not save Pealayer configuration: {error}");
            }
        }
    }

    pub(crate) fn apply_runtime_config(
        &mut self,
        ctx: &egui::Context,
        config: crate::config::AppConfig,
    ) -> Result<(), String> {
        config.validate()?;
        let endpoint = config
            .hardware_endpoint
            .clone()
            .unwrap_or_else(|| crate::four_d::controller::DEFAULT_ENDPOINT.to_string());
        let endpoint_changed = endpoint != self.serial_port;
        let connection_policy_changed = config.auto_connect_hardware != self.auto_connect_hardware;

        self.app_name = crate::config::resolved_app_name(&config);
        self.app_publisher = crate::config::resolved_app_publisher(&config);
        self.app_copyright = crate::config::resolved_app_copyright(&config);
        self.volume = config.volume;
        self.is_muted = config.is_muted;
        self.pin_controls = config.pin_controls;
        self.show_remaining_time = config.show_remaining_time;
        self.open_url_multiline = config.open_url_multiline;
        self.open_url_history_expanded = config.open_url_history_expanded;
        self.open_url_recent_click_edits = config.open_url_recent_click_edits;
        self.open_url_fetch_remote_info = config.open_url_fetch_remote_info;
        self.open_url_fetch_remote_thumbnail = config.open_url_fetch_remote_thumbnail;
        self.open_url_use_proxy = config.open_url_use_proxy;
        self.open_url_proxy_url = config.open_url_proxy_url.clone().unwrap_or_default();
        self.recent_media = config.recent_media.clone();
        self.last_media_target = config.last_media_target.clone();
        self.restore_last_media_on_startup = config.restore_last_media_on_startup;
        self.remember_playback_position = config.remember_playback_position;
        self.playback_position_history_limit = config.playback_position_history_limit;
        self.playback_positions = config.playback_positions.clone();
        self.trim_playback_positions();
        self.language_preference = crate::config::resolved_language_preference(&config);
        self.language = crate::config::resolve_language(self.language_preference);
        self.direction_preference = crate::config::resolved_direction_preference(&config);
        self.rtl = crate::config::resolve_rtl(self.direction_preference, self.language);
        self.theme_preference = crate::config::resolved_theme(&config);
        self.serial_port = endpoint.clone();
        self.auto_connect_hardware = config.auto_connect_hardware;
        self.pause_on_hardware_disconnect = config.pause_on_hardware_disconnect;
        self.click_player_to_toggle = config.click_player_to_toggle;
        self.show_subseconds = config.show_subseconds;
        self.quick_seek_seconds = config.quick_seek_seconds;
        self.frame_step_count = config.frame_step_count;
        self.wheel_seek_seconds = config.wheel_seek_seconds;
        self.osd_position = config.osd_position;
        self.osd_timeout_seconds = config.osd_timeout_seconds;
        self.paused_drag_action = config.paused_drag_action;
        self.playing_drag_action = config.playing_drag_action;
        self.fullscreen_video_background = config.fullscreen_video_background;
        self.motion_control_mode = config.motion_control_mode;
        self.compact_hardware_controls = config.compact_hardware_controls;
        self.non_user_control_visibility = config.non_user_control_visibility;
        self.prefix_relay_identifiers = config.prefix_relay_identifiers;
        self.live_pwm_updates = config.live_pwm_updates;
        self.hardware_actions_on_press = config.hardware_actions_on_press;
        if self.hardware_key_bindings != config.hardware_key_bindings {
            // A file-watcher/API update may unregister or alter a held global
            // shortcut. Release against the old contract before replacing it.
            self.release_active_hardware_bindings();
            self.hardware_key_bindings = config.hardware_key_bindings.clone();
        }
        self.show_estop_control = config.show_estop_control;
        self.confirm_estop_release = config.confirm_estop_release;
        self.single_instance = config.single_instance;
        self.windows_mica_backdrop = config.windows_mica_backdrop;
        self.windows_dwm_theming = config.windows_dwm_theming;
        self.opengl_vsync = config.opengl_vsync;
        self.native_dialog_windows = config.native_dialog_windows;
        self.auto_reload_config = config.auto_reload_config;
        self.status_bar = config.status_bar;
        self.workspace_profiles = config.workspace_profiles.clone();
        self.active_workspace_profile = config.active_workspace_profile.clone();
        crate::platform::windows::configure_window_composition(
            self.windows_dwm_theming,
            self.windows_mica_backdrop,
        );
        if let Some(layout_json) = config.workspace_dock_layout.as_deref()
            && let Ok(mut dock_state) = serde_json::from_str::<
                egui_dock::DockState<crate::ui::layout::PealayerTab>,
            >(layout_json)
        {
            crate::ui::layout::sanitize_dock_rects(&mut dock_state);
            self.dock_state = dock_state;
        }

        let _ = self.mpv.set_property("volume", self.volume);
        let _ = self.mpv.set_property("mute", self.is_muted);
        crate::mpv::proxy::apply_runtime(
            self.mpv,
            self.open_url_use_proxy,
            &self.open_url_proxy_url,
        )?;
        crate::platform::windows::sync_windows_jump_list(&self.recent_media);
        self.prune_recent_remote_thumbnail_cache();
        crate::ui::i18n::configure_ui_fonts(
            ctx,
            self.language == crate::config::AppLanguage::Persian,
        );
        ctx.set_theme(match self.theme_preference {
            crate::config::AppTheme::System => egui::ThemePreference::System,
            crate::config::AppTheme::Light => egui::ThemePreference::Light,
            crate::config::AppTheme::Dark => egui::ThemePreference::Dark,
        });
        crate::ui::configure_native_visuals(ctx, &config);
        crate::platform::windows::set_window_theme(ctx.global_style().visuals.dark_mode);

        if endpoint_changed || connection_policy_changed {
            let _ = self.engine_handle.sender.send(
                crate::four_d::engine::EngineMessage::ReconfigureEndpoint {
                    endpoint,
                    connect: self.auto_connect_hardware,
                },
            );
        }
        crate::platform::interop::set_live_config(config);
        Ok(())
    }

    pub(crate) fn preview_runtime_config(
        &mut self,
        ctx: &egui::Context,
        config: crate::config::AppConfig,
    ) -> Result<(), String> {
        if self.preference_preview_original.is_none() {
            self.preference_preview_original = Some(self.runtime_config_snapshot());
        }
        self.apply_runtime_config(ctx, config)
    }

    pub(crate) fn commit_preference_preview(
        &mut self,
        ctx: &egui::Context,
        config: crate::config::AppConfig,
    ) -> Result<(), String> {
        self.apply_runtime_config(ctx, config)?;
        self.preference_preview_original = None;
        self.config_fingerprint =
            crate::config::AppConfig::fingerprint(&crate::config::AppConfig::get_config_path())
                .ok();
        Ok(())
    }

    pub(crate) fn cancel_preference_preview(&mut self, ctx: &egui::Context) -> Result<(), String> {
        let Some(config) = self.preference_preview_original.take() else {
            return Ok(());
        };
        if let Err(error) = self.apply_runtime_config(ctx, config.clone()) {
            self.preference_preview_original = Some(config);
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn reload_config_from_disk(&mut self, ctx: &egui::Context) -> Result<(), String> {
        let path = crate::config::AppConfig::get_config_path();
        let config = crate::config::AppConfig::load_from_path(&path)?;
        self.apply_runtime_config(ctx, config.clone())?;
        self.config_fingerprint = crate::config::AppConfig::fingerprint(&path).ok();
        let status = format!("Reloaded {}", crate::config::display_config_path(&path));
        self.config_status = status.clone();
        let conflict_status =
            self.tr("Configuration changed on disk; reload it or save your draft");
        if let Some(draft) = self.preferences_draft.as_mut() {
            draft.apply_external_config(config, status, conflict_status);
        }
        Ok(())
    }

    fn poll_external_config(&mut self, ctx: &egui::Context) {
        if !self.auto_reload_config {
            self.config_watcher = None;
            self.config_reload_due = None;
            return;
        }
        let path = crate::config::AppConfig::get_config_path();
        if self.config_watcher.is_none() {
            let repaint = ctx.clone();
            match crate::config::ConfigFileWatcher::new(&path, move || {
                repaint.request_repaint();
            }) {
                Ok(watcher) => self.config_watcher = Some(watcher),
                Err(error) => {
                    self.config_status = error;
                    return;
                }
            }
        }
        let changed = self
            .config_watcher
            .as_ref()
            .map(crate::config::ConfigFileWatcher::take_changed)
            .transpose();
        match changed {
            Ok(Some(true)) => {
                let delay = std::time::Duration::from_millis(90);
                self.config_reload_due = Some(std::time::Instant::now() + delay);
                ctx.request_repaint_after(delay);
            }
            Ok(_) => {}
            Err(error) => {
                self.config_status = error;
                return;
            }
        }
        let Some(due) = self.config_reload_due else {
            return;
        };
        if std::time::Instant::now() < due {
            ctx.request_repaint_after(due.saturating_duration_since(std::time::Instant::now()));
            return;
        }
        self.config_reload_due = None;
        let Ok(fingerprint) = crate::config::AppConfig::fingerprint(&path) else {
            return;
        };
        if self.config_fingerprint == Some(fingerprint) {
            return;
        }
        match self.reload_config_from_disk(ctx) {
            Ok(()) => self.set_osd(self.tr("Preferences reloaded from disk")),
            Err(error) => {
                self.config_status = error.clone();
                log::warn!("Could not reload changed Pealayer configuration: {error}");
            }
        }
    }

    fn apply_config_patch(
        &mut self,
        ctx: &egui::Context,
        values: &serde_json::Value,
    ) -> Result<(), String> {
        let updated = self.runtime_config_snapshot().apply_patch(values)?;
        updated.save()?;
        self.apply_runtime_config(ctx, updated)?;
        self.config_fingerprint =
            crate::config::AppConfig::fingerprint(&crate::config::AppConfig::get_config_path())
                .ok();
        self.config_status = "Updated through API".to_string();
        Ok(())
    }

    pub fn tr(&self, english: &'static str) -> String {
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
        self.prune_recent_remote_thumbnail_cache();
        self.save_config();
    }

    pub fn clear_recent_media(&mut self) {
        self.recent_media.clear();
        crate::platform::windows::sync_windows_jump_list(&[]);
        self.prune_recent_remote_thumbnail_cache();
        self.save_config();
    }

    pub fn remove_recent_media(&mut self, target: &str) {
        self.recent_media
            .retain(|path| path.to_string_lossy() != target);
        crate::platform::windows::sync_windows_jump_list(&self.recent_media);
        self.prune_recent_remote_thumbnail_cache();
        self.save_config();
    }

    pub fn clear_recent_remote_media(&mut self) {
        self.recent_media
            .retain(|path| !crate::media::is_remote_media_target(&path.to_string_lossy()));
        crate::platform::windows::sync_windows_jump_list(&self.recent_media);
        self.prune_recent_remote_thumbnail_cache();
        self.save_config();
    }

    fn prune_recent_remote_thumbnail_cache(&self) {
        let remote = self
            .recent_media
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .filter(|target| crate::media::is_remote_media_target(target))
            .collect::<Vec<_>>();
        crate::server::thumbnails::prune_remote_thumbnail_cache(remote.iter().map(String::as_str));
    }

    fn resume_position_for(&self, target: &str) -> Option<f64> {
        if !self.remember_playback_position {
            return None;
        }
        let key = crate::media::playback_history_key(target);
        self.playback_positions
            .iter()
            .find(|entry| crate::media::playback_history_key(&entry.target) == key)
            .map(|entry| entry.position_seconds)
            .filter(|position| position.is_finite() && *position >= 1.0)
    }

    fn capture_current_playback_position(&mut self) -> bool {
        if !self.remember_playback_position {
            return false;
        }
        let Some(target) = self
            .current_video_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
        else {
            return false;
        };
        let key = crate::media::playback_history_key(&target);
        if key.is_empty() {
            return false;
        }

        let finished = self.is_eof
            || (self.duration > 0.0
                && self.playback_time >= (self.duration - 2.0).max(self.duration * 0.98));
        if finished {
            let old_len = self.playback_positions.len();
            self.playback_positions
                .retain(|entry| crate::media::playback_history_key(&entry.target) != key);
            return old_len != self.playback_positions.len();
        }
        if !self.playback_time.is_finite() || self.playback_time < 1.0 {
            return false;
        }
        if self.playback_positions.first().is_some_and(|entry| {
            crate::media::playback_history_key(&entry.target) == key
                && (entry.position_seconds - self.playback_time).abs() < 1.0
        }) {
            return false;
        }

        self.playback_positions
            .retain(|entry| crate::media::playback_history_key(&entry.target) != key);
        self.playback_positions.insert(
            0,
            crate::config::PlaybackPositionEntry {
                target,
                position_seconds: self.playback_time,
                updated_at_unix_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            },
        );
        self.trim_playback_positions();
        true
    }

    fn trim_playback_positions(&mut self) {
        let limit =
            self.playback_position_history_limit
                .clamp(1, crate::config::MAX_PLAYBACK_POSITION_HISTORY_LIMIT) as usize;
        self.playback_positions.truncate(limit);
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
                    self.linked_analog_tracks(),
                ),
            );
        }
        changed
    }

    pub(crate) fn linked_analog_tracks(&self) -> Vec<crate::four_d::curve::AnalogTrack> {
        self.timeline
            .analog_tracks
            .iter()
            .filter(|track| {
                self.timeline
                    .track_state(&crate::four_d::models::hardware_timeline_track_key(
                        &format!("pwm.{}", track.channel),
                    ))
                    .linked
            })
            .cloned()
            .collect()
    }

    pub fn snapshot_timeline(&self) -> crate::four_d::history::TimelineSnapshot {
        crate::four_d::history::TimelineSnapshot {
            instances: self.timeline.instances.clone(),
            analog_tracks: self.timeline.analog_tracks.clone(),
            templates: self.timeline.templates.clone(),
            keyframes: self.timeline.keyframes.clone(),
            track_states: self.timeline.track_states.clone(),
        }
    }

    fn persist_timeline_track_preferences(&mut self) {
        let Some(video_path) = self.current_video_path.as_ref() else {
            return;
        };
        if crate::media::is_remote_media_target(&video_path.to_string_lossy()) {
            return;
        }
        let mut sidecar = video_path.clone();
        sidecar.set_extension("4d.json");
        if !sidecar.exists() {
            let mut json_sidecar = video_path.clone();
            json_sidecar.set_extension("json");
            if json_sidecar.exists() {
                sidecar = json_sidecar;
            }
        }
        if let Err(error) = self.timeline.save_to_file(&sidecar) {
            self.set_osd(format!(
                "{}: {error}",
                self.tr("Could not save timeline settings")
            ));
        }
    }

    pub(crate) fn set_timeline_track_linked(&mut self, key: &str, linked: bool) {
        if self.timeline.track_state(key).linked == linked {
            return;
        }
        self.undo_stack.push(self.snapshot_timeline());
        self.timeline.set_track_linked(key.to_string(), linked);
        self.persist_timeline_track_preferences();
        self.sync_timeline_engine();
    }

    pub(crate) fn set_timeline_track_visible(&mut self, key: &str, visible: bool) {
        if self
            .timeline
            .track_states
            .get(key)
            .is_some_and(|state| state.visible == visible)
        {
            return;
        }
        self.undo_stack.push(self.snapshot_timeline());
        let mut state = self.timeline.track_state(key);
        state.visible = visible;
        self.timeline.track_states.insert(key.to_string(), state);
        self.persist_timeline_track_preferences();
    }

    pub(crate) fn set_timeline_tracks_linked(
        &mut self,
        keys: impl IntoIterator<Item = String>,
        linked: bool,
    ) {
        let keys = keys.into_iter().collect::<Vec<_>>();
        if !keys
            .iter()
            .any(|key| self.timeline.track_state(key).linked != linked)
        {
            return;
        }
        self.undo_stack.push(self.snapshot_timeline());
        for key in keys {
            self.timeline.set_track_linked(key, linked);
        }
        self.persist_timeline_track_preferences();
        self.sync_timeline_engine();
    }

    pub(crate) fn set_timeline_tracks_visible(
        &mut self,
        keys: impl IntoIterator<Item = String>,
        visible: bool,
    ) {
        let keys = keys.into_iter().collect::<Vec<_>>();
        if !keys.iter().any(|key| {
            self.timeline
                .track_states
                .get(key)
                .map_or(true, |state| state.visible != visible)
        }) {
            return;
        }
        self.undo_stack.push(self.snapshot_timeline());
        for key in keys {
            let mut state = self.timeline.track_state(&key);
            state.visible = visible;
            self.timeline.track_states.insert(key, state);
        }
        self.persist_timeline_track_preferences();
    }

    /// Rebuilds every hardware lane from the authoritative project timeline.
    /// Keeping relay edges and controller-owned macro cues together prevents
    /// load, undo, delete, and drag operations from updating only one lane.
    pub fn sync_timeline_engine(&self) {
        let mut muted_tracks = self.track_muted.clone();
        for (key, state) in &self.timeline.track_states {
            if !state.linked {
                if let Some(relay) = key
                    .strip_prefix("hardware:relay.")
                    .and_then(|value| value.parse::<u8>().ok())
                {
                    muted_tracks.insert(relay);
                }
            }
        }
        let relays = crate::four_d::engine::compile_timeline(
            &self.timeline,
            &muted_tracks,
            &self.track_soloed,
        );
        let macros = crate::four_d::engine::compile_controller_macros(&self.timeline);
        let strip_effects = crate::four_d::engine::compile_controller_strip_effects(&self.timeline);
        let _ = self
            .engine_handle
            .sender
            .send(crate::four_d::engine::EngineMessage::UpdateQueue(relays));
        let _ = self
            .engine_handle
            .sender
            .send(crate::four_d::engine::EngineMessage::UpdateControllerMacros(macros));
        let _ = self.engine_handle.sender.send(
            crate::four_d::engine::EngineMessage::UpdateControllerStripEffects(strip_effects),
        );
        let _ = self.engine_handle.sender.send(
            crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.linked_analog_tracks()),
        );
    }

    pub fn restore_timeline_snapshot(
        &mut self,
        snapshot: crate::four_d::history::TimelineSnapshot,
    ) {
        self.timeline.instances = snapshot.instances;
        self.timeline.analog_tracks = snapshot.analog_tracks;
        self.timeline.templates = snapshot.templates;
        self.timeline.keyframes = snapshot.keyframes;
        self.timeline.track_states = snapshot.track_states;
        self.persist_timeline_track_preferences();
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
    let mut effect = crate::four_d::models::Effect::controller_macro(
        hardware_macro.name.clone(),
        String::new(),
        hardware_macro.duration_ms,
        hardware_macro.id,
        hardware_macro.mode.clone(),
    );
    effect.icon = crate::ui::icons::named_control_icon(&hardware_macro.icon)
        .unwrap_or(crate::ui::icons::SPARKLE)
        .to_string();
    effect.controller_lane = Some(controller_macro_lane(hardware_macro));
    EffectPreset {
        category: hardware_macro.category.clone(),
        group_icon: hardware_macro.group_icon.clone(),
        source: EffectPresetSource::ControllerMacro(hardware_macro.id),
        effect,
    }
}

pub(crate) fn controller_macro_lane(
    hardware_macro: &crate::four_d::controller::HardwareMacro,
) -> crate::four_d::models::ControllerEffectLane {
    use crate::four_d::models::ControllerEffectLane;

    let mut lanes = std::collections::BTreeSet::new();
    for step in &hardware_macro.steps {
        let kind = step.kind.trim().to_ascii_lowercase();
        let lane = if step
            .action_ids
            .iter()
            .any(|action| action.starts_with("seat.") || action.starts_with("motion."))
        {
            ControllerEffectLane::Motion
        } else {
            match kind.as_str() {
                "relay" | "relay-mask" | "relays-off" => ControllerEffectLane::Relay,
                "pwm" | "mosfet" => ControllerEffectLane::Pwm,
                "display" | "message" => ControllerEffectLane::Display,
                "rf" | "rf-transmit" => ControllerEffectLane::Rf,
                "beep" | "tone" | "buzzer" => ControllerEffectLane::Audio,
                "rgb" | "status-led" | "addressable" | "ws2812" | "strip" | "strip-frame"
                | "pixel" => ControllerEffectLane::Lighting,
                _ => ControllerEffectLane::Sequence,
            }
        };
        lanes.insert(lane);
    }
    match lanes.len() {
        0 => ControllerEffectLane::Sequence,
        1 => *lanes.iter().next().expect("one classified effect lane"),
        _ => ControllerEffectLane::Composite,
    }
}

fn controller_effect_lane_name(lane: crate::four_d::models::ControllerEffectLane) -> &'static str {
    use crate::four_d::models::ControllerEffectLane;
    match lane {
        ControllerEffectLane::Motion => "motion",
        ControllerEffectLane::Relay => "relay",
        ControllerEffectLane::Pwm => "pwm",
        ControllerEffectLane::Lighting => "lighting",
        ControllerEffectLane::Display => "display",
        ControllerEffectLane::Rf => "rf",
        ControllerEffectLane::Audio => "audio",
        ControllerEffectLane::Sequence => "sequence",
        ControllerEffectLane::Composite => "composite",
    }
}

/// Publishes the same capability-derived hardware model used by egui. The web
/// client deliberately receives no demo channels or inferred board features.
fn web_hardware_details(
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    motion_control_mode: crate::config::MotionControlMode,
) -> serde_json::Value {
    let controls = capabilities
        .controls
        .iter()
        .map(|control| {
            let relay_id = capabilities
                .relays
                .iter()
                .find(|output| output.key == control.key)
                .map(|output| output.id);
            let pwm_channel = capabilities
                .pwm_channels
                .iter()
                .find(|output| output.key == control.key)
                .map(|output| output.id);
            let active = relay_id
                .map(|relay| capabilities.active_relays.contains(&relay))
                .or_else(|| {
                    crate::ui::layout::is_motion_control(control).then(|| {
                        matches!(
                            crate::ui::layout::motion_control_direction(capabilities, control),
                            crate::ui::layout::MotionDirectionState::Up
                                | crate::ui::layout::MotionDirectionState::Down
                        )
                    })
                });
            let pwm_percent = pwm_channel.and_then(|channel| {
                let status_component = capabilities.status_led.as_ref().and_then(|status| {
                    let role = capabilities
                        .pwm_channels
                        .iter()
                        .find(|output| output.id == channel)
                        .map(|output| output.role.to_ascii_lowercase())?;
                    if role.contains("status") && role.contains("red") {
                        Some(status.red)
                    } else if role.contains("status") && role.contains("green") {
                        Some(status.green)
                    } else if role.contains("status") && role.contains("blue") {
                        Some(status.blue)
                    } else {
                        None
                    }
                });
                if let Some(value) = status_component {
                    return Some(f64::from(value) * 100.0 / 255.0);
                }
                capabilities
                    .telemetry
                    .pwm_values
                    .get(usize::from(channel))
                    .copied()
                    .flatten()
                    .or_else(|| {
                        (capabilities.telemetry.pwm_channel == Some(channel))
                            .then_some(capabilities.telemetry.pwm_value)
                            .flatten()
                    })
                    .map(|value| f64::from(value) * 100.0 / 4095.0)
            });
            serde_json::json!({
                "key": control.key,
                "kind": control.kind,
                "order": control.order,
                "name": control.name,
                "default_name": control.default_name,
                "control": control.control,
                "icon": control.icon,
                "color": control.color,
                "group": control.group,
                "hidden": control.hidden,
                "locked": control.locked,
                "channel": relay_id.or(pwm_channel),
                "active": active,
                "percent": pwm_percent,
                "actions": control.actions.iter().map(|action| serde_json::json!({
                    "id": action.id,
                    "verb": action.verb,
                    "name": action.name,
                    "icon": action.icon,
                })).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    let profile = capabilities.board_profile.as_ref().map(|profile| {
        serde_json::json!({
            "key": profile.key,
            "mode": profile.mode,
            "configured": profile.configured,
            "attached": profile.attached,
            "revision": profile.revision,
            "expose_raw_relays": profile.expose_raw_relays,
        })
    });
    let settings = capabilities.settings.as_ref().map(|settings| {
        serde_json::json!({
            "silent": settings.silent,
            "light_mode": settings.light_mode,
            "on_brightness": settings.on_brightness,
            "off_brightness": settings.off_brightness,
            "display_brightness": settings.display_brightness,
            "status_brightness": settings.status_brightness,
            "stream_period_ms": settings.stream_period_ms,
            "default_page": settings.default_page,
            "motion_break_ms": settings.motion_break_ms,
            "persisted": settings.persisted,
        })
    });
    let front_panel = capabilities.front_panel.as_ref().map(|panel| {
        serde_json::json!({
            "raw_segments": panel.raw_segments,
            "brightness": panel.brightness,
            "blink": panel.blink,
            "segments_active": panel.segments_active,
            "pressed_keys": panel.pressed_keys,
            "menu_page": panel.menu_page,
            "program_mode": panel.program_mode,
            "host_captured": panel.host_captured,
            "lcd_available": panel.lcd_available,
            "lcd_address": panel.lcd_address,
            "lcd_line_1": panel.lcd_line_1,
            "lcd_line_2": panel.lcd_line_2,
        })
    });
    let strip = capabilities.strip_control.as_ref().map(|strip| {
        serde_json::json!({
            "minimum_pixels": strip.minimum_pixels,
            "maximum_pixels": strip.maximum_pixels,
            "default_pixels": strip.default_pixels,
            "minimum_fps": strip.minimum_fps,
            "maximum_fps": strip.maximum_fps,
            "default_fps": strip.default_fps,
            "modes": strip.modes,
            "running": strip.running,
            "active_name": strip.active_name,
        })
    });
    serde_json::json!({
        "board_name": capabilities.board_name,
        "capability_bits": capabilities.capability_bits,
        "host_instance_id": capabilities.host_instance_id,
        "motion_control_mode": match motion_control_mode {
            crate::config::MotionControlMode::Hold => "hold",
            crate::config::MotionControlMode::Toggle => "toggle",
        },
        "profile": profile,
        "port": {
            "name": capabilities.port.name,
            "display_name": capabilities.port.display_name,
            "friendly_name": capabilities.port.friendly_name,
            "product": capabilities.port.product,
            "manufacturer": capabilities.port.manufacturer,
            "vid": capabilities.port.vid,
            "pid": capabilities.port.pid,
            "serial_number": capabilities.port.serial_number,
        },
        "identity": {
            "product_name": capabilities.board_identity.product_name,
            "stored_name": capabilities.board_identity.stored_name,
            "build_hash": capabilities.board_identity.build_hash,
            "build_timestamp": capabilities.board_identity.build_timestamp,
        },
        "controls": controls,
        "active_relays": capabilities.active_relays,
        "telemetry": {
            "supply_mv": capabilities.telemetry.supply_mv,
            "bus_mv": capabilities.telemetry.bus_mv,
            "current_ma": capabilities.telemetry.current_ma,
            "power_mw": capabilities.telemetry.power_mw,
            "led_temperature_centi_c": capabilities.telemetry.led_temperature_centi_c,
            "audio_temperature_centi_c": capabilities.telemetry.audio_temperature_centi_c,
            "door_open": capabilities.telemetry.door_open,
        },
        "warnings": capabilities.warnings.iter().map(|warning| serde_json::json!({
            "code": warning.code,
            "severity": warning.severity,
            "message": warning.message,
        })).collect::<Vec<_>>(),
        "settings": settings,
        "front_panel": front_panel,
        "strip": strip,
        "supports": {
            "rf_transmit": capabilities.supports_rf_transmit,
            "segment_display": capabilities.supports_segment_display,
            "lcd_display": capabilities.supports_lcd_display,
            "addressable_led": capabilities.supports_addressable_led,
            "persistent_settings": capabilities.supports_persistent_settings,
            "measurements": capabilities.supports_measurements,
            "temperature_sensors": capabilities.supports_temperature_sensors,
        },
    })
}

fn front_panel_command(key: &str, host_captured: bool) -> Result<String, String> {
    if !matches!(key, "K1" | "K2" | "K3" | "K4") {
        return Err("front-panel key must be K1, K2, K3, or K4".to_string());
    }
    if host_captured {
        return Ok(format!("host-menu key {key} press"));
    }
    Ok(match key {
        "K1" => "menu previous",
        "K2" => "menu next",
        "K3" => "menu decrease",
        "K4" => "menu increase",
        _ => unreachable!("validated above"),
    }
    .to_string())
}

fn controller_strip_effect_preset(
    strip_effect: &crate::four_d::controller::HardwareStripEffect,
) -> EffectPreset {
    let mut effect = crate::four_d::models::Effect::controller_strip_effect(
        strip_effect.name.clone(),
        strip_effect.default_duration_ms.unwrap_or(5_000),
        strip_effect.id.clone(),
    );
    effect.icon = crate::ui::icons::named_control_icon(&strip_effect.icon)
        .unwrap_or(crate::ui::icons::SPARKLE)
        .to_string();
    EffectPreset {
        category: strip_effect.category.clone(),
        group_icon: strip_effect.group_icon.clone(),
        source: EffectPresetSource::ControllerStrip,
        effect,
    }
}

fn saved_macro_id(output: &str) -> Option<u64> {
    output
        .strip_prefix("macro ")?
        .split_once('/')?
        .0
        .parse()
        .ok()
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
        let _ = mpv_client.observe_property("container-fps", libmpv2::Format::Double, 13);
        let _ = mpv_client.observe_property("seekable", libmpv2::Format::Flag, 14);
        let _ = mpv_client.observe_property("demuxer-cache-duration", libmpv2::Format::Double, 15);
        let _ = mpv_client.observe_property("cache-buffering-state", libmpv2::Format::Int64, 16);
        let _ = mpv_client.observe_property("vid", libmpv2::Format::String, 18);
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
            window_geometry: None,
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
            is_seekable: false,
            media_metadata_loaded: false,
            cache_duration: None,
            cache_buffering_percent: None,
            media_fps: 0.0,
            is_paused: false,
            is_eof: false,
            volume: 100.0,
            is_muted: false,
            playback_rate: 1.0,
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
            current_vid: "no".to_string(),
            video_tracks: Vec::new(),
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
            hardware_effect_authoring: HardwareEffectAuthoringState::default(),
            selected_instance_ids: std::collections::HashSet::new(),
            selected_keyframes: std::collections::HashSet::new(),
            selected_timeline_keyframe: None,
            active_keyframe_drag: None,
            timeline_zoom: 100.0,
            undo_stack: crate::four_d::history::UndoStack::default(),
            recording_keys: std::collections::HashMap::new(),
            effects_search_query: String::new(),
            show_effect_library_editor: false,
            effect_library_selection: None,
            effect_library_draft: ControllerEffectDraft::default(),
            effect_group_draft: None,
            track_muted: std::collections::BTreeSet::new(),
            track_soloed: std::collections::BTreeSet::new(),
            track_locked: std::collections::BTreeSet::new(),
            active_drag: None,
            estop_active: false,
            show_estop_control: true,
            confirm_estop_release: true,
            show_estop_release_dialog: false,
            skip_estop_release_confirmation_draft: false,
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
            editing_elapsed_time: false,
            elapsed_time_input: String::new(),
            elapsed_edit_focus_requested: false,
            osd_message: None,
            recent_media: Vec::new(),
            last_media_target: None,
            restore_last_media_on_startup: true,
            remember_playback_position: true,
            playback_position_history_limit: crate::config::DEFAULT_PLAYBACK_POSITION_HISTORY_LIMIT,
            playback_positions: Vec::new(),
            pending_resume_position: None,
            last_playback_position_checkpoint: std::time::Instant::now(),
            show_open_url_dialog: false,
            url_input_buffer: String::new(),
            open_url_multiline: true,
            open_url_history_expanded: true,
            open_url_recent_click_edits: true,
            open_url_fetch_remote_info: true,
            open_url_fetch_remote_thumbnail: true,
            open_url_use_proxy: true,
            open_url_proxy_url: String::new(),
            url_inspector: crate::ui::open_url::UrlInspector::default(),
            is_window_operating: false,
            show_shortcuts_dialog: false,
            show_about_dialog: false,
            about_tab: 0,
            about_icon: None,
            show_preferences_dialog: false,
            preferences_tab: 0,
            preferences_draft: None,
            show_board_info_dialog: false,
            board_info_tab: 0,
            board_name_draft: String::new(),
            show_hardware_channels_dialog: false,
            show_workspace_profiles_dialog: false,
            workspace_profile_name_draft: String::new(),
            workspace_profile_icon_draft: "window".to_string(),
            workspace_profiles: crate::config::default_workspace_profiles(),
            active_workspace_profile: Some("nle".to_string()),
            hardware_control_dialog_key: None,
            hardware_channel_detail_active: false,
            hardware_control_name_draft: String::new(),
            hardware_control_group_draft: String::new(),
            hardware_control_icon_draft: String::new(),
            hardware_control_icon_search: String::new(),
            hardware_control_color_draft: String::new(),
            hardware_control_up_color_draft: String::new(),
            hardware_control_down_color_draft: String::new(),
            hardware_control_pwm_percent: 0.0,
            hardware_key_bindings: Vec::new(),
            hardware_binding_dialog_channel: None,
            hardware_binding_draft: None,
            hardware_binding_capturing: false,
            hardware_hotkey_runtime:
                crate::hardware_shortcuts::GlobalHardwareShortcutRuntime::default(),
            active_hardware_bindings: std::collections::BTreeSet::new(),
            board_operation: None,
            board_operation_status: String::new(),
            board_settings_draft: None,
            board_settings_dirty: false,
            board_reboot_armed: false,
            front_panel_refresh_attempted: false,
            front_panel_pending_key: None,
            pause_on_hardware_disconnect: true,
            auto_connect_hardware: true,
            click_player_to_toggle: true,
            show_subseconds: true,
            quick_seek_seconds: 10.0,
            frame_step_count: 1,
            wheel_seek_seconds: 5.0,
            osd_position: crate::config::OsdPosition::TopLeft,
            osd_timeout_seconds: 3.5,
            paused_drag_action: crate::config::PlayerDragAction::MoveWindow,
            playing_drag_action: crate::config::PlayerDragAction::TemporaryFastForward,
            fullscreen_video_background: crate::config::VideoBackground::Black,
            motion_control_mode: crate::config::MotionControlMode::Hold,
            held_motion_action: None,
            compact_hardware_controls: false,
            non_user_control_visibility: crate::config::NonUserControlVisibility::Dimmed,
            prefix_relay_identifiers: true,
            live_pwm_updates: true,
            hardware_actions_on_press: true,
            single_instance: true,
            windows_mica_backdrop: false,
            windows_dwm_theming: true,
            opengl_vsync: false,
            native_dialog_windows: true,
            native_preferences: None,
            status_bar: crate::config::StatusBarConfig::default(),
            config_fingerprint: crate::config::AppConfig::fingerprint(
                &crate::config::AppConfig::get_config_path(),
            )
            .ok(),
            config_watcher: None,
            config_reload_due: None,
            auto_reload_config: true,
            preference_preview_original: None,
            config_status: String::new(),
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
            last_taskbar_state: None,
            last_thumbnail_button_state: None,
        }
    }
}

pub fn contextual_window_title(
    app_name: &str,
    media_path: Option<&std::path::Path>,
    connected: bool,
    connection_requested: bool,
) -> String {
    if let Some(media_path) = media_path {
        let target = media_path.to_string_lossy();
        let media_name = crate::media::media_target_label(&target);
        return format!("{media_name} — {app_name}");
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
    (was_transport_connected && !transport_connected) || (was_board_connected && !board_connected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_panel_keys_route_to_the_board_or_active_host_menu() {
        assert_eq!(front_panel_command("K1", false).unwrap(), "menu previous");
        assert_eq!(front_panel_command("K2", false).unwrap(), "menu next");
        assert_eq!(front_panel_command("K3", false).unwrap(), "menu decrease");
        assert_eq!(front_panel_command("K4", false).unwrap(), "menu increase");
        assert_eq!(
            front_panel_command("K4", true).unwrap(),
            "host-menu key K4 press"
        );
        assert!(front_panel_command("K5", false).is_err());
    }

    #[test]
    fn board_settings_command_serializes_every_live_setting_in_contract_order() {
        let settings = crate::four_d::controller::HardwareBoardSettings {
            flags: 0x80,
            silent: true,
            programming_latch: true,
            swap_temperature_roles: false,
            motion_door_policy: 2,
            door_audio_enabled: false,
            relay_audio_enabled: true,
            light_mode: 2,
            on_brightness: 210,
            off_brightness: 12,
            display_brightness: 5,
            display_closed_brightness: 2,
            status_brightness: 128,
            output_persistence: 15,
            stream_period_ms: 250,
            default_page: 4,
            save_last_page: true,
            status_color: 3,
            voltage_decimals: 2,
            current_decimals: 1,
            motion_exit_hold_seconds: 7,
            motion_break_ms: 180,
            relay_restore_mask: 0xA5,
            ..Default::default()
        };

        assert_eq!(
            board_settings_command(&settings).unwrap(),
            "settings set 179 2 210 12 5 2 128 15 250 4 1 3 2 1 7 180 165"
        );
    }

    #[test]
    fn board_settings_command_rejects_an_invalid_motion_break() {
        let settings = crate::four_d::controller::HardwareBoardSettings {
            motion_exit_hold_seconds: 2,
            motion_break_ms: 0,
            ..Default::default()
        };

        assert_eq!(
            board_settings_command(&settings).unwrap_err(),
            "Motion break must be between 1 and 255 ms"
        );
    }

    #[test]
    fn cue_drag_hit_testing_keeps_a_move_region_between_resize_handles() {
        assert_eq!(
            classify_clip_drag_mode(100.0, 200.0, 104.0),
            DragMode::ResizeLeft
        );
        assert_eq!(classify_clip_drag_mode(100.0, 200.0, 150.0), DragMode::Move);
        assert_eq!(
            classify_clip_drag_mode(100.0, 200.0, 196.0),
            DragMode::ResizeRight
        );

        // Even an eight-pixel minimum-width cue keeps a center move target.
        assert_eq!(classify_clip_drag_mode(10.0, 18.0, 14.0), DragMode::Move);
    }

    #[test]
    fn controller_owned_cues_can_be_resized_without_rewriting_their_program() {
        let mut effect = crate::four_d::models::Effect::controller_macro(
            "Seat motion".to_string(),
            String::new(),
            1_000,
            7,
            "host".to_string(),
        );

        update_effect_duration(&mut effect, 2_500);

        assert_eq!(effect.duration_ms, 2_500);
        assert!(effect.actions.is_empty());
        assert_eq!(effect.controller_macro.as_ref().map(|cue| cue.id), Some(7));
    }
    use super::{
        DroppedFileKind, contextual_window_title, controller_macro_effect_preset, dropped_file_kind,
    };

    #[test]
    fn extracts_saved_controller_macro_id_for_catalog_reconciliation() {
        assert_eq!(
            super::saved_macro_id("macro 6/seat-motion saved with 7 mcu-timed steps"),
            Some(6)
        );
        assert_eq!(super::saved_macro_id("recording is empty"), None);
    }

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
    fn playback_position_history_is_bounded_and_deduplicated_by_media_identity() {
        let mut app = PealayerApp::default();
        app.playback_position_history_limit = 2;
        for (target, position) in [
            ("https://example.invalid/a.mp4#first", 10.0),
            ("https://example.invalid/b.mp4", 20.0),
            ("https://example.invalid/a.mp4#second", 30.0),
            ("https://example.invalid/c.mp4", 40.0),
        ] {
            app.current_video_path = Some(std::path::PathBuf::from(target));
            app.playback_time = position;
            app.duration = 120.0;
            app.is_eof = false;
            assert!(app.capture_current_playback_position());
        }

        assert_eq!(app.playback_positions.len(), 2);
        assert_eq!(
            app.resume_position_for("https://example.invalid/c.mp4"),
            Some(40.0)
        );
        assert_eq!(
            app.resume_position_for("https://example.invalid/a.mp4"),
            Some(30.0)
        );
        assert_eq!(
            app.resume_position_for("https://example.invalid/b.mp4"),
            None
        );
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
        fn remove_string(&mut self, _key: &str) {}
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
        assert_eq!(
            contextual_window_title(
                "Studio",
                Some(std::path::Path::new(
                    "rtsp://operator:secret@camera.invalid/live",
                )),
                false,
                false,
            ),
            "rtsp://***@camera.invalid/live — Studio"
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
            icon: String::new(),
            group_icon: String::new(),
            mode: "mcu".to_string(),
            duration_ms: 250,
            steps: vec![
                crate::four_d::controller::HardwareMacroStep {
                    at_us: 0,
                    kind: "relay".to_string(),
                    target: Some(6),
                    value: Some(1),
                    ..Default::default()
                },
                crate::four_d::controller::HardwareMacroStep {
                    at_us: 250_000,
                    kind: "relays-off".to_string(),
                    target: None,
                    value: None,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let preset = controller_macro_effect_preset(&hardware_macro);
        assert_eq!(preset.effect.name, "Live Air Burst");
        assert!(preset.effect.actions.is_empty());
        assert_eq!(
            preset.effect.target,
            crate::four_d::models::HardwareTarget::ControllerMacro
        );
        assert_eq!(preset.effect.controller_macro.as_ref().unwrap().id, 12);
        assert_eq!(preset.effect.duration_ms, 250);
    }

    #[test]
    fn non_relay_controller_macro_is_available_without_relay_flattening() {
        let hardware_macro = crate::four_d::controller::HardwareMacro {
            id: 7,
            name: "Display only".to_string(),
            category: "Display".to_string(),
            icon: String::new(),
            group_icon: String::new(),
            mode: "host".to_string(),
            duration_ms: 1,
            steps: vec![crate::four_d::controller::HardwareMacroStep {
                at_us: 0,
                kind: "display".to_string(),
                target: None,
                value: None,
                ..Default::default()
            }],
            ..Default::default()
        };
        let preset = controller_macro_effect_preset(&hardware_macro);
        assert_eq!(preset.effect.controller_macro.as_ref().unwrap().id, 7);
        assert!(preset.effect.actions.is_empty());
    }

    #[test]
    fn controller_effect_lane_is_derived_from_the_sequence_commands() {
        use crate::four_d::controller::{HardwareMacro, HardwareMacroStep};
        use crate::four_d::models::ControllerEffectLane;

        let macro_with = |steps| HardwareMacro {
            steps,
            ..Default::default()
        };
        let step = |kind: &str| HardwareMacroStep {
            kind: kind.to_string(),
            ..Default::default()
        };

        assert_eq!(
            controller_macro_lane(&macro_with(vec![step("pwm")])),
            ControllerEffectLane::Pwm
        );
        assert_eq!(
            controller_macro_lane(&macro_with(vec![step("display")])),
            ControllerEffectLane::Display
        );
        assert_eq!(
            controller_macro_lane(&macro_with(vec![step("pwm"), step("display")])),
            ControllerEffectLane::Composite
        );

        let mut motion = step("relay-mask");
        motion.action_ids.push("seat.a.up".to_string());
        assert_eq!(
            controller_macro_lane(&macro_with(vec![motion])),
            ControllerEffectLane::Motion
        );
    }

    #[test]
    fn interactive_estop_release_is_guarded_until_user_confirms() {
        let mut app = PealayerApp::default();
        app.estop_active = true;
        app.confirm_estop_release = true;

        app.request_emergency_stop_change(false);

        assert!(app.estop_active);
        assert!(app.show_estop_release_dialog);
        assert!(!app.skip_estop_release_confirmation_draft);
    }

    #[test]
    fn interactive_estop_release_can_use_persisted_no_confirm_preference() {
        let mut app = PealayerApp::default();
        app.estop_active = true;
        app.confirm_estop_release = false;

        app.request_emergency_stop_change(false);

        assert!(!app.estop_active);
        assert!(!app.show_estop_release_dialog);
    }

    #[test]
    fn web_hardware_details_publish_every_sampled_pwm_channel() {
        let mut capabilities = crate::four_d::controller::HardwareCapabilities::default();
        capabilities.controls = vec![crate::four_d::controller::HardwareControl {
            key: "pwm.3".to_string(),
            kind: "pwm".to_string(),
            name: "PWM 4".to_string(),
            ..Default::default()
        }];
        capabilities.pwm_channels = vec![crate::four_d::controller::HardwareOutput {
            id: 3,
            key: "pwm.3".to_string(),
            name: "PWM 4".to_string(),
            role: String::new(),
            control: "slider".to_string(),
        }];
        capabilities.telemetry.pwm_values = vec![None, None, None, Some(2048)];

        let details =
            web_hardware_details(&capabilities, crate::config::MotionControlMode::default());
        let percent = details["controls"][0]["percent"].as_f64().unwrap();
        assert!((percent - (2048.0 * 100.0 / 4095.0)).abs() < f64::EPSILON);

        capabilities.controls[0].key = "pwm.15".to_string();
        capabilities.pwm_channels[0] = crate::four_d::controller::HardwareOutput {
            id: 15,
            key: "pwm.15".to_string(),
            name: "Status blue".to_string(),
            role: "status-blue".to_string(),
            control: "role-specific".to_string(),
        };
        capabilities.telemetry.pwm_values.resize(16, None);
        capabilities.telemetry.pwm_values[15] = Some(0);
        capabilities.status_led = Some(crate::four_d::controller::HardwareStatusLed {
            blue: 160,
            ..Default::default()
        });
        let details =
            web_hardware_details(&capabilities, crate::config::MotionControlMode::default());
        let percent = details["controls"][0]["percent"].as_f64().unwrap();
        assert!((percent - (160.0 * 100.0 / 255.0)).abs() < f64::EPSILON);
    }
}
