use crate::mpv::render::RenderContextWrapper;
use eframe::egui;
use std::sync::{Arc, Mutex};

const IDLE_WEB_SYNC_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

fn effective_web_sync_interval(
    configured: std::time::Duration,
    playback_or_operation_active: bool,
) -> std::time::Duration {
    if playback_or_operation_active {
        configured
    } else {
        configured.max(IDLE_WEB_SYNC_INTERVAL)
    }
}

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
enum ControllerEffectCatalogKey {
    Macro(u64),
    Strip(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ControllerEffectCatalogEntry {
    key: ControllerEffectCatalogKey,
    name: String,
    icon: String,
    duration_ms: u64,
    mode: String,
    lane: crate::four_d::models::ControllerEffectLane,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug)]
pub struct HardwareEffectAuthoringState {
    pub name: String,
    pub category: String,
    pub color: String,
    /// `automatic` records every acknowledged app/board action, `device-clock`
    /// uses the stricter device timestamp, and `board-retained` leaves the
    /// bounded relay take in board RAM until Pealayer imports it.
    pub capture_mode: String,
    pub active: bool,
    pub preview_active: bool,
    pub preview_reference: Option<String>,
    preview_duration: Option<std::time::Duration>,
    preview_deadline: Option<std::time::Instant>,
    pub anchor_ms: u64,
    pub pending_operation: Option<String>,
    pub pending_saved_macro_id: Option<u64>,
    pub record_after_publish: Option<(String, String)>,
    pub append_target: Option<u64>,
    pub append_discarded: bool,
    /// The most recent effect whose upsert was acknowledged by PCController.
    ///
    /// Catalog refreshes are asynchronous, so the cached capability snapshot
    /// can legitimately lag behind a successful publish. Keep the explicit
    /// acknowledgement as immediate authority for Run/Delete instead of
    /// leaving those controls disabled until the next polling cycle.
    pub acknowledged_effect_reference: Option<String>,
    pub status: String,
    timeline_catalog: Vec<ControllerEffectCatalogEntry>,
}

impl Default for HardwareEffectAuthoringState {
    fn default() -> Self {
        Self {
            name: String::new(),
            category: "Recorded".to_string(),
            color: "violet".to_string(),
            capture_mode: "automatic".to_string(),
            active: false,
            preview_active: false,
            preview_reference: None,
            preview_duration: None,
            preview_deadline: None,
            anchor_ms: 0,
            pending_operation: None,
            pending_saved_macro_id: None,
            record_after_publish: None,
            append_target: None,
            append_discarded: false,
            acknowledged_effect_reference: None,
            status: String::new(),
            timeline_catalog: Vec::new(),
        }
    }
}

impl HardwareEffectAuthoringState {
    fn acknowledge_effect_publish(&mut self, reference: &str) {
        self.acknowledged_effect_reference = canonical_effect_reference(reference);
    }

    fn forget_effect_publish(&mut self, reference: &str) {
        let reference = canonical_effect_reference(reference);
        if self.acknowledged_effect_reference == reference {
            self.acknowledged_effect_reference = None;
        }
    }

    fn effect_publish_is_acknowledged(&self, reference: &str) -> bool {
        canonical_effect_reference(reference).is_some_and(|reference| {
            self.acknowledged_effect_reference.as_ref() == Some(&reference)
        })
    }

    fn begin_effect_preview(
        &mut self,
        reference: &str,
        duration: Option<std::time::Duration>,
    ) {
        self.preview_reference = canonical_effect_reference(reference);
        self.preview_duration = duration;
        self.preview_deadline = None;
        self.preview_active = false;
    }

    fn acknowledge_effect_preview(&mut self, now: std::time::Instant) {
        self.preview_active = true;
        self.preview_deadline = self.preview_duration.map(|duration| now + duration);
    }

    fn finish_effect_preview(&mut self) {
        self.preview_active = false;
        self.preview_reference = None;
        self.preview_duration = None;
        self.preview_deadline = None;
    }

    fn expire_effect_preview(&mut self, now: std::time::Instant) {
        if self.pending_operation.is_none()
            && self
                .preview_deadline
                .is_some_and(|deadline| deadline <= now)
        {
            self.finish_effect_preview();
        }
    }

    fn preview_repaint_after(&self, now: std::time::Instant) -> Option<std::time::Duration> {
        self.preview_deadline
            .and_then(|deadline| deadline.checked_duration_since(now))
    }

    fn effect_preview_phase(
        &mut self,
        reference: &str,
        now: std::time::Instant,
    ) -> ControllerEffectPreviewPhase {
        self.expire_effect_preview(now);
        let same_effect = canonical_effect_reference(reference).is_some_and(|reference| {
            self.preview_reference.as_ref() == Some(&reference)
        });
        match self.pending_operation.as_deref() {
            Some("effect-play" | "effect-preview") if same_effect => {
                ControllerEffectPreviewPhase::Starting
            }
            Some("effect-stop") if same_effect => ControllerEffectPreviewPhase::Stopping,
            _ if same_effect && self.preview_active => ControllerEffectPreviewPhase::Stop,
            _ => ControllerEffectPreviewPhase::Run,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ControllerEffectPreviewPhase {
    Run,
    Starting,
    Stop,
    Stopping,
}

fn canonical_effect_reference(reference: &str) -> Option<String> {
    let reference = reference.trim();
    let id = ["effect:", "strip:"]
        .into_iter()
        .find_map(|prefix| {
            reference
                .get(..prefix.len())
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
                .then(|| &reference[prefix.len()..])
        })
        .unwrap_or(reference)
        .trim();
    (!id.is_empty()).then(|| format!("effect:{}", id.to_ascii_lowercase()))
}

pub struct RttState {
    pub video_texture: Option<eframe::glow::Texture>,
    pub video_fbo: Option<eframe::glow::Framebuffer>,
    pub video_texture_id: Option<eframe::egui::TextureId>,
    pub texture_width: u32,
    pub texture_height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PendingScrubCommit {
    request_id: u64,
    target_time: f64,
    dispatched: bool,
    playback_restarted: bool,
}

fn seek_target_reached(actual_time: f64, target_time: f64, media_fps: f64) -> bool {
    if !actual_time.is_finite() || !target_time.is_finite() {
        return false;
    }
    let frame_tolerance = if media_fps.is_finite() && media_fps > 0.0 {
        2.0 / media_fps
    } else {
        0.1
    };
    (actual_time - target_time).abs() <= frame_tolerance.max(0.05)
}

/// Resolve the logical timeline position after mpv has decoded a seek.
///
/// A requested timestamp can fall between video frames. In that case mpv's
/// `time-pos` reports the PTS of the real frame it can display (usually the
/// following frame), but an editor must retain the user's sub-frame timeline
/// position while paused so hardware cues and keyframes can still be authored
/// at millisecond precision. Playing media follows the decoded clock instead.
fn settled_seek_position(
    actual_time: f64,
    target_time: f64,
    was_playing_before_scrub: bool,
) -> f64 {
    if was_playing_before_scrub {
        actual_time
    } else {
        target_time
    }
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
    pub(crate) web_only: bool,
    pub(crate) app_name: String,
    pub(crate) app_publisher: Option<String>,
    pub(crate) app_copyright: Option<String>,
    pub(crate) last_window_title: String,
    pub(crate) window_geometry: Option<crate::config::WindowGeometry>,
    pub(crate) language_preference: crate::config::AppLanguage,
    pub(crate) language: crate::config::AppLanguage,
    pub(crate) direction_preference: crate::config::AppDirection,
    pub(crate) theme_preference: crate::config::AppTheme,
    pub(crate) color_palette: crate::config::ColorPalette,
    pub(crate) rtl: bool,
    pub(crate) mpv: crate::mpv::player::Player,
    pub(crate) mpv_client: libmpv2::Mpv,
    pub(crate) render_context: Arc<Mutex<Option<RenderContextWrapper>>>,

    pub playback_time: f64,
    pub duration: f64,
    pub is_seekable: bool,
    pub(crate) media_metadata_loaded: bool,
    pub(crate) cache_duration: Option<f64>,
    pub(crate) cache_buffering_percent: Option<f64>,
    pub(crate) media_fps: f64,
    pub(crate) video_aspect_ratio: f64,
    pub(crate) consistent_video_aspect_ratio: bool,
    pub(crate) always_on_top: crate::config::AlwaysOnTopMode,
    pub(crate) applied_always_on_top: Option<bool>,
    pub(crate) pending_video_aspect_resize: bool,
    pub is_paused: bool,
    pub is_eof: bool,
    pub(crate) volume: f64,
    pub(crate) is_muted: bool,
    pub(crate) playback_rate: f64,
    pub(crate) configured_playback_speed: f64,
    pub(crate) temporary_fast_forward_speed: f64,
    pub(crate) video_surface_gesture: Option<crate::ui::video::VideoSurfaceGesture>,

    pub seek_pos: Option<f64>,
    pub(crate) seek_controller: crate::mpv::seek::SeekController,
    pub(crate) was_playing_before_scrub: bool,
    pub(crate) is_scrubbing: bool,
    pub(crate) pending_scrub_commit: Option<PendingScrubCommit>,
    pub(crate) last_mouse_activity: std::time::Instant,
    pub(crate) pin_controls: bool,

    pub show_error: Option<String>,

    // Subtitle state
    pub(crate) show_sub_settings: bool,
    pub(crate) sub_visibility: bool,
    pub(crate) sub_font_size: f64,
    pub(crate) numeric_input_steps: std::collections::BTreeMap<String, crate::config::NumericInputSteps>,
    pub(crate) sub_delay: f64,
    pub(crate) sub_position_percent: f64,
    pub(crate) current_sid: String,
    pub(crate) sub_tracks: Vec<SubtitleTrack>,
    pub(crate) subtitle_direction: crate::subtitle::SubtitleDirection,
    pub(crate) subtitle_alignment: crate::subtitle::SubtitleAlignment,
    pub(crate) subtitle_text_replacements: Vec<crate::subtitle::SubtitleReplacement>,
    pub(crate) subtitle_text: String,

    // Video state
    pub(crate) current_vid: String,
    pub(crate) video_tracks: Vec<VideoTrack>,

    // Audio state
    pub(crate) show_audio_settings: bool,
    pub(crate) audio_delay: f64,
    pub(crate) current_aid: String,
    pub(crate) audio_tracks: Vec<AudioTrack>,
    pub(crate) media_tracks: Vec<MediaTrackInfo>,
    pub(crate) media_file_info: crate::media_info::MediaFileInfo,
    pub(crate) media_track_properties: Option<MediaTrackKey>,
    pub(crate) selected_timeline_track: Option<String>,

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
    pub(crate) osd_display_options: Option<crate::platform::interop::OsdOptions>,
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
    pub(crate) hardware_control_color_draft: String,
    pub(crate) hardware_control_up_color_draft: String,
    pub(crate) hardware_control_down_color_draft: String,
    pub(crate) hardware_control_pwm_percent: f64,
    pub(crate) keyboard_shortcuts_enabled: bool,
    pub(crate) media_keys_enabled: bool,
    pub(crate) application_shortcuts: crate::application_shortcuts::ApplicationShortcuts,
    pub(crate) global_hardware_hotkeys_enabled: bool,
    pub(crate) hardware_key_bindings: Vec<crate::config::HardwareKeyBinding>,
    pub(crate) hardware_binding_dialog_channel: Option<String>,
    pub(crate) hardware_binding_draft: Option<crate::config::HardwareKeyBinding>,
    pub(crate) hardware_binding_capturing: bool,
    pub(crate) hardware_hotkey_runtime: crate::hardware_shortcuts::GlobalHardwareShortcutRuntime,
    pub(crate) active_hardware_bindings: std::collections::BTreeSet<String>,
    pub(crate) board_operation: Option<String>,
    pub(crate) board_operation_status: String,
    pub(crate) rf: crate::ui::rf::RfState,
    pub(crate) board_settings_draft: Option<crate::four_d::controller::HardwareBoardSettings>,
    pub(crate) board_settings_dirty: bool,
    pub(crate) board_reboot_armed: bool,
    pub(crate) front_panel_refresh_attempted: bool,
    pub(crate) front_panel_pending_key: Option<String>,
    pub(crate) pause_on_hardware_disconnect: bool,
    pub(crate) auto_connect_hardware: bool,
    pub(crate) click_player_to_toggle: bool,
    pub(crate) show_subseconds: bool,
    pub(crate) human_readable_time_units: bool,
    pub(crate) seekbar_hover_thumbnails: bool,
    pub(crate) nle_seekbar_hover_thumbnails: bool,
    pub(crate) seekbar_thumbnail_preview: crate::ui::seek_preview::SeekbarThumbnailPreview,
    pub(crate) quick_seek_seconds: f64,
    pub(crate) frame_step_count: u32,
    pub(crate) wheel_seek_seconds: f64,
    pub(crate) osd_position: crate::config::OsdPosition,
    pub(crate) osd_timeout_seconds: f32,
    pub(crate) paused_drag_action: crate::config::PlayerDragAction,
    pub(crate) playing_drag_action: crate::config::PlayerDragAction,
    pub(crate) middle_click_action: crate::config::PlayerClickAction,
    pub(crate) middle_hold_action: crate::config::PlayerDragAction,
    pub(crate) right_click_action: crate::config::PlayerClickAction,
    pub(crate) right_hold_action: crate::config::PlayerDragAction,
    pub(crate) fullscreen_video_background: crate::config::VideoBackground,
    pub(crate) motion_control_mode: crate::config::MotionControlMode,
    pub(crate) held_motion_action: Option<(String, String, String)>,
    pub(crate) compact_hardware_controls: bool,
    pub(crate) compact_timeline_tracks: bool,
    pub(crate) timeline_header_wheel_vertical_scroll: bool,
    pub(crate) timeline_plain_wheel_action: crate::config::TimelineWheelBehavior,
    pub(crate) timeline_ctrl_wheel_action: crate::config::TimelineWheelBehavior,
    pub(crate) timeline_shift_wheel_action: crate::config::TimelineWheelBehavior,
    pub(crate) timeline_alt_wheel_action: crate::config::TimelineWheelBehavior,
    pub(crate) timeline_middle_button_pan: bool,
    pub(crate) timeline_middle_axis_lock_modifiers: bool,
    pub(crate) timeline_animated_navigation: bool,
    pub(crate) timeline_navigation_transition_ms: u32,
    pub(crate) non_user_control_visibility: crate::config::NonUserControlVisibility,
    pub(crate) prefix_relay_identifiers: bool,
    pub(crate) live_pwm_updates: bool,
    pub(crate) hardware_actions_on_press: bool,
    pub(crate) single_instance: bool,
    pub(crate) window_magnetic_snap: bool,
    pub(crate) window_magnetic_snap_distance: u32,
    pub(crate) windows_mica_backdrop: bool,
    pub(crate) windows_dwm_theming: bool,
    pub(crate) windows_video_taskbar_thumbnail: bool,
    pub(crate) windows_thumbnail_toolbar: bool,
    pub(crate) windows_jump_list_quick_actions: bool,
    pub(crate) opengl_vsync: bool,
    pub(crate) live_video_during_window_move: bool,
    pub(crate) compositor_paced_window_move: bool,
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
    pub(crate) media_cmd_tx: std::sync::mpsc::Sender<souvlaki::MediaControlEvent>,
    pub(crate) media_cmd_rx: std::sync::mpsc::Receiver<souvlaki::MediaControlEvent>,
    pub window_handle: Option<isize>,
    pub shell_initialized: bool,
    pub(crate) last_taskbar_state: Option<crate::platform::windows::TaskbarState>,
    pub(crate) last_thumbnail_button_state: Option<(bool, bool, bool, bool, bool, (u32, bool))>,
    pub(crate) last_update_notice_state: Option<String>,
}

fn should_throttle_video_render(
    pointer_down: bool,
    previous_window_operation: bool,
    timeline_drag_active: bool,
) -> bool {
    pointer_down && (previous_window_operation || timeline_drag_active)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum MediaTrackType {
    Video,
    Audio,
    Subtitle,
}

impl MediaTrackType {
    pub const fn mpv_name(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Subtitle => "sub",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Video => "Video",
            Self::Audio => "Audio",
            Self::Subtitle => "Subtitles",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaTrackKey {
    pub kind: MediaTrackType,
    pub id: i64,
}

/// A factual snapshot of one `track-list/N` entry exposed by libmpv.
///
/// Fields stay optional because containers, demuxers, and stream types expose
/// different subsets. The properties UI omits unavailable values instead of
/// inventing placeholders or inferring technical details.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MediaTrackInfo {
    pub list_index: i64,
    pub kind: MediaTrackType,
    pub id: i64,
    pub source_id: Option<i64>,
    pub title: Option<String>,
    pub language: Option<String>,
    pub image: Option<bool>,
    pub album_art: Option<bool>,
    pub is_default: Option<bool>,
    pub forced: Option<bool>,
    pub dependent: Option<bool>,
    pub visual_impaired: Option<bool>,
    pub hearing_impaired: Option<bool>,
    pub hls_bitrate: Option<i64>,
    pub program_id: Option<i64>,
    pub codec: Option<String>,
    pub codec_description: Option<String>,
    pub codec_profile: Option<String>,
    pub external: Option<bool>,
    pub external_filename: Option<String>,
    pub selected: Option<bool>,
    pub main_selection: Option<i64>,
    pub ffmpeg_index: Option<i64>,
    pub decoder: Option<String>,
    pub decoder_description: Option<String>,
    pub demux_width: Option<i64>,
    pub demux_height: Option<i64>,
    pub crop_x: Option<i64>,
    pub crop_y: Option<i64>,
    pub crop_width: Option<i64>,
    pub crop_height: Option<i64>,
    pub channel_count: Option<i64>,
    pub channel_layout: Option<String>,
    pub sample_rate: Option<i64>,
    pub fps: Option<f64>,
    pub bitrate: Option<f64>,
    pub rotation: Option<i64>,
    pub pixel_aspect_ratio: Option<f64>,
    pub format_name: Option<String>,
    pub replaygain_track_peak: Option<f64>,
    pub replaygain_track_gain: Option<f64>,
    pub replaygain_album_peak: Option<f64>,
    pub replaygain_album_gain: Option<f64>,
    pub dolby_vision_profile: Option<i64>,
    pub dolby_vision_level: Option<i64>,
    pub metadata: std::collections::BTreeMap<String, String>,
}

impl eframe::App for PealayerApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        for request in crate::peer::take_gui_requests() {
            let result=if request.deadline< std::time::Instant::now(){Err("Request expired before application; change was not applied".into())}else{self.apply_peer_request(ui.ctx(),&request.path,request.value)};
            let _=request.reply.send(result);
        }
        if self.web_only {
            // Keep the native event/render loop alive for libmpv and the
            // shared command engine while exposing only the Web/PWA surface.
            // Reasserting visibility prevents a focus/restore command from
            // accidentally presenting the implementation viewport.
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
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
            if self.web_only {
                crate::platform::windows::hide_native_window(
                    crate::platform::windows::get_registered_hwnd(),
                );
                crate::platform::windows::hide_current_process_windows();
            }
        }

        if !self.web_only {
            crate::platform::taskbar_preview::register_repaint(ui.ctx());
            self.ensure_shell_initialized();
            self.process_shell_commands(ui.ctx());
        }
        self.process_controller_call_results();
        let preview_now = std::time::Instant::now();
        self.hardware_effect_authoring
            .expire_effect_preview(preview_now);
        if let Some(remaining) = self
            .hardware_effect_authoring
            .preview_repaint_after(preview_now)
        {
            ui.ctx().request_repaint_after(remaining);
        }
        self.refresh_controller_effect_timeline_metadata();
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
            && let Some((_, stop_action, control_key)) = self.held_motion_action.take()
        {
            crate::ui::layout::invoke_held_motion_action(
                self,
                &control_key,
                &stop_action,
            );
        }

        if !self.media_keys_enabled {
            // Drop unregisters the native session and invalidates old callbacks.
            self.media_controls = None;
        } else if self.media_controls.is_none() {
            let hwnd = self
                .window_handle
                .unwrap_or_else(crate::platform::windows::get_registered_hwnd);
            // Windows requires a real HWND; initialization before registration
            // would panic inside the OS-media-controls library.
            if !cfg!(target_os = "windows") || hwnd != 0 {
                let mut controls = crate::platform::media_controls::MediaControlsManager::new(
                    hwnd,
                    self.media_cmd_tx.clone(),
                    ui.ctx().clone(),
                );
                let title = self.current_video_path.as_deref()
                    .map(|path| crate::media::media_target_label(&path.to_string_lossy()));
                controls.update_metadata(title.as_deref());
                self.media_controls = Some(controls);
            }
        }

        // Track active window/panel drag operations safely without lock nesting
        let is_pointer_down = ui.input(|i| i.pointer.any_down());
        let native_window_operating = crate::platform::windows::native_window_operation_active();
        if crate::platform::windows::take_native_window_operation_ended() {
            // Always paint once on release: live mode consumes any final
            // geometry/property change, while compatibility mode replaces the
            // deliberately retained frame without waiting for another wakeup.
            ui.ctx().request_repaint();
        }
        // Only throttle the MPV render pass for a real window/timeline drag.
        // `egui_is_using_pointer()` is also true while seeking or holding the
        // video surface, where suppressing paint freezes the very preview the
        // gesture is meant to control.
        self.is_window_operating = (native_window_operating && !self.live_video_during_window_move)
            || should_throttle_video_render(
                is_pointer_down,
                self.is_window_operating,
                self.active_drag.is_some(),
            );

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
        while let Ok(event) = self.media_cmd_rx.try_recv() {
            if self.media_keys_enabled
                && let Some(command) = crate::platform::media_controls::map_media_control_event(
                    event,
                    self.quick_seek_seconds,
                )
            {
                inbound_commands.push(("Media controls", command));
            }
        }
        while let Ok(delivery) = self.controller_cmd_rx.try_recv() {
            self.apply_interop_command(ui.ctx(), delivery.command.clone(), "PCController");
            delivery.acknowledge_applied();
        }
        for (source, command) in inbound_commands {
            // A preceding IPC/config command can disable media keys during
            // this same frame. Do not apply already-queued OS actions then.
            if source == "Media controls" && !self.media_keys_enabled {
                continue;
            }
            self.apply_interop_command(ui.ctx(), command, source);
        }

        // Reconcile the workspace with the viewport before publishing status.
        crate::remote_location::install_context(ui.ctx());
        if let Some(playback) = crate::remote_location::take_playback() { self.play_remote_location(playback); }
        crate::remote_location::set_current(self.current_video_path.as_ref().and_then(|path|path.to_str()));
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
        if !self.consistent_video_aspect_ratio || self.show_four_d_editor || is_fullscreen {
            crate::platform::windows::set_simple_video_aspect_constraint(
                false,
                self.video_aspect_ratio,
                0,
                0,
            );
        }

        // Keep the read-only status snapshot current at the configured cadence;
        // WebSocket delivery itself can be disabled independently.
        let web_config = crate::platform::interop::get_live_config();
        crate::peer::publish_timeline(crate::peer::TimelineState{timeline:self.timeline.clone(),muted:self.track_muted.clone(),soloed:self.track_soloed.clone()});
        let appearance = crate::platform::interop::AppearanceState::new(
            &web_config,
            ui.ctx().theme() == egui::Theme::Dark,
        );
        let configured_web_sync_interval =
            std::time::Duration::from_millis(u64::from(web_config.web_sync_interval_ms));
        let web_sync_active = (self.current_video_path.is_some() && !self.is_paused)
            || self.is_scrubbing
            || self.pending_scrub_commit.is_some()
            || self.hardware_effect_authoring.pending_operation.is_some()
            || self.board_operation.is_some();
        // Real controller, WebSocket, media and input changes request their own
        // repaint. Keep the configured fast cadence only while work is active;
        // an idle/paused player needs a low-rate status backstop, not a full NLE
        // recomposition ten times per second.
        let web_sync_interval =
            effective_web_sync_interval(configured_web_sync_interval, web_sync_active);
        let now = std::time::Instant::now();
        let appearance_changed =
            crate::platform::interop::get_live_appearance().as_ref() != Some(&appearance);
        let messages = crate::messaging::snapshot();
        crate::remote_location::sync_options(web_config.remote_folder_auto_next, web_config.remote_folder_thumbnails);
        let remote_changed = crate::platform::interop::get_live_remote_revision() != crate::remote_location::revision();
        let messages_changed = crate::platform::interop::get_live_message_snapshot() != messages;
        let should_broadcast = appearance_changed || messages_changed || remote_changed
            || match self.last_web_broadcast {
                Some(last) => now.duration_since(last) >= web_sync_interval,
                None => true,
            };

        if !should_broadcast {
            // A push can wake the last idle frame inside the Web rate limit.
            // Keep one deadline repaint to publish that final state rather
            // than leaving the cached API/Web preview one event behind. The
            // deadline frame broadcasts and schedules no further wakeup.
            if let Some(last) = self.last_web_broadcast {
                ui.ctx().request_repaint_after(
                    web_sync_interval.saturating_sub(now.duration_since(last)),
                );
            }
        }

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
            let chapters = self.media_chapters();
            let current_chapter_index = self.active_media_chapter().map(|chapter| chapter.index);
            let status_resp = crate::platform::interop::PlayerStatusResponse {
                application: crate::platform::interop::ApplicationIdentity::current(&self.app_name),
                runtime: crate::platform::interop::runtime_identity(),
                rf: self.rf.snapshot(),
                remote_browser: crate::remote_location::snapshot(),
                messages,
                appearance: Some(appearance),
                timeline_wheel_preferences: Some(crate::config::TimelineWheelPreferences {
                    plain: self.timeline_plain_wheel_action,
                    ctrl: self.timeline_ctrl_wheel_action,
                    shift: self.timeline_shift_wheel_action,
                    alt: self.timeline_alt_wheel_action,
                }),
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
                chapters: chapters
                    .into_iter()
                    .map(|chapter| crate::platform::interop::WebMediaChapter {
                        index: chapter.index,
                        title: chapter.title,
                        time_seconds: chapter.time_seconds,
                    })
                    .collect(),
                current_chapter_index,
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
                hardware_sync: self.engine_handle.prepared_timeline.try_lock().ok()
                    .filter(|plan|plan.has_items()).map(|plan|serde_json::json!({
                        "revision":plan.revision,"prepared_revision":plan.acknowledged_revision,
                        "clock_ack_revision":plan.clock_ack_revision,"clock_ack_epoch":plan.clock_ack_epoch,
                        "ack_age_ms":plan.last_ack.map(|ack|ack.elapsed().as_millis() as u64),
                        "error":plan.error,"timeline":plan.feedback
                    })).unwrap_or(serde_json::Value::Null),
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
                effect_recording: hardware
                    .as_ref()
                    .map(|capabilities| {
                        let recording = &capabilities.effect_recording;
                        crate::platform::interop::WebEffectRecording {
                            active: recording.active,
                            id: recording.id,
                            name: recording.name.clone(),
                            mode: recording.mode.clone(),
                            category: recording.category.clone(),
                            color: recording.color.clone(),
                            steps: recording.steps,
                            preview: recording.preview.clone(),
                            device_retained: recording.device_retained,
                            overwritten: recording.overwritten,
                            started_at: recording.started_at.clone(),
                            last_error: recording.last_error.clone(),
                            pending: self.hardware_effect_authoring.pending_operation.is_some() || self.hardware_effect_authoring.pending_saved_macro_id.is_some(),
                        }
                    })
                    .unwrap_or_else(|| crate::platform::interop::WebEffectRecording {
                        active: self.hardware_effect_authoring.active,
                        name: self.hardware_effect_authoring.name.clone(),
                        mode: self.hardware_effect_authoring.capture_mode.clone(),
                        category: self.hardware_effect_authoring.category.clone(),
                        color: self.hardware_effect_authoring.color.clone(),
                        pending: self.hardware_effect_authoring.pending_operation.is_some() || self.hardware_effect_authoring.pending_saved_macro_id.is_some(),
                        ..Default::default()
                    }),
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
                            .map(str::to_owned)
                            .or_else(|| {
                                effect.direct_control.as_ref().map(|cue| cue.control_key.clone())
                            })
                            .unwrap_or_else(|| "sequence".to_string()),
                    })
                    .collect(),
                controller_effects,
                controller_effect_groups: hardware
                    .as_ref()
                    .map(|hardware| hardware.effect_groups.clone())
                    .unwrap_or_default(),
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
                                resizable: effect.duration_resizable(),
                                control_key: effect
                                    .direct_control
                                    .as_ref()
                                    .map(|cue| cue.control_key.clone()),
                                value_basis_points: effect
                                    .direct_control
                                    .as_ref()
                                    .map(|cue| cue.value_basis_points),
                            })
                    })
                    .collect(),
                timeline_tracks: crate::ui::layout::web_timeline_tracks(self),
                osd: self.osd_message.as_ref().and_then(|(message, started)| {
                    let options = self.osd_display_options.clone().unwrap_or_default();
                    let timeout = options
                        .timeout_seconds
                        .unwrap_or(self.osd_timeout_seconds)
                        .max(0.25);
                    let remaining = std::time::Duration::from_secs_f32(timeout)
                        .checked_sub(started.elapsed())?;
                    Some(crate::platform::interop::WebOsdState {
                        message: message.clone(),
                        remaining_ms: remaining.as_millis().min(u64::MAX as u128) as u64,
                        default_position: match self.osd_position {
                            crate::config::OsdPosition::TopLeft => {
                                crate::platform::interop::OsdAnchor::TopLeft
                            }
                            crate::config::OsdPosition::Center => {
                                crate::platform::interop::OsdAnchor::Center
                            }
                        },
                        options,
                    })
                }),
                update: crate::update::manager().status(),
            };
            crate::peer::publish_media_view(crate::peer::MediaView {
                tracks:self.media_tracks.clone(), file:self.media_file_info.clone(),
                vid:self.current_vid.clone(), aid:self.current_aid.clone(), sid:self.current_sid.clone(),
            });
            crate::platform::interop::set_live_status(status_resp.clone());
            if let Ok(json) = serde_json::to_string(&status_resp) {
                let authoritative=crate::peer::client().and_then(|client|client.snapshot()).map(|value|value.session.status.to_string()).unwrap_or(json);
                let _ = self.web_state_tx.send(authoritative);
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

        if crate::peer::active(){crate::peer::mirror(||self.process_events());}else{self.process_events();}
        if crate::peer::active(){self.poll_peer_session(&ctx);}
        self.sync_window_level(ui.ctx());
        if self.is_scrubbing || self.pending_scrub_commit.is_some() {
            // A paused libmpv surface still needs paint opportunities while a
            // coalesced preview or exact commit is decoding. This timer exists
            // only for the active gesture/commit and therefore does not revive
            // the old permanent idle repaint loop.
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        self.update_shell_state();
        if let Some(ref mut mc) = self.media_controls {
            mc.update_playback(
                self.current_video_path.is_some(),
                self.is_paused,
                self.playback_time,
                self.duration,
            );
            mc.update_volume(self.volume as f64 / 100.0);
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
        let update = crate::update::manager().status();
        if update.state != "idle"
            && self.last_update_notice_state.as_deref() != Some(update.state.as_str())
        {
            self.set_osd(update.message.clone());
            if matches!(update.state.as_str(), "restarting" | "failed")
                && let Some(hwnd) = self.window_handle
            {
                let _ = crate::platform::windows::show_system_notification(
                    hwnd,
                    &self.app_name,
                    &update.message,
                );
            }
            self.last_update_notice_state = Some(update.state.clone());
        }
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
        if self.hardware_effect_authoring.pending_operation.is_some()
            || self.board_operation.is_some()
        {
            // Tracked RPC calls complete off the UI thread. Keep a short
            // repaint lease while one is pending so its result cannot remain
            // hidden until another mouse or media event happens.
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
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
        crate::ui::sync_native_window_appearance(ui.ctx(), self.color_palette);

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
        self.process_application_shortcuts(&ctx);
        let timeline_keyboard_active =
            ctx.memory(|memory| memory.has_focus(crate::ui::layout::timeline_keyboard_focus_id()));
        let transport_shortcuts_enabled = self.keyboard_shortcuts_enabled
            && !ctx.egui_wants_keyboard_input()
            && !timeline_keyboard_active;
        self.process_hardware_key_bindings(
            &ctx,
            transport_shortcuts_enabled && self.hardware_binding_dialog_channel.is_none(),
        );
        if let Some(direction) = crate::application_shortcuts::take_frame_step_shortcut(
            &ctx,
            self.keyboard_shortcuts_enabled,
        ) {
            self.step_frames(direction);
        }
        if transport_shortcuts_enabled && ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.toggle_playback();
        }
        if transport_shortcuts_enabled && ctx.input(|i| i.key_pressed(egui::Key::F)) {
            self.toggle_fullscreen(&ctx);
        }
        if transport_shortcuts_enabled && ctx.input(|i| i.key_pressed(egui::Key::M)) {
            self.toggle_audio_muted();
        }
        if transport_shortcuts_enabled
            && ctx.input(|i| i.modifiers.is_none() && i.key_pressed(egui::Key::ArrowLeft))
        {
            self.seek_relative(-self.quick_seek_seconds);
        }
        if transport_shortcuts_enabled
            && ctx.input(|i| i.modifiers.is_none() && i.key_pressed(egui::Key::ArrowRight))
        {
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
            if transport_shortcuts_enabled
                && ctx.input(|i| i.key_pressed(key))
                && !self.recording_keys.contains_key(&key)
            {
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
                crate::ui::peer_browser::draw(self,ui);
                crate::ui::peer_browser::draw_connection(ui);
                crate::ui::effects_library::draw_editor(self, ui);
                crate::ui::board_info::draw(self, ui);
                crate::ui::rf::draw(self, ui);
                crate::ui::hardware_control::draw(self, ui);
                crate::ui::media_track_properties::draw(self, ui);
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
                                for (icon, shortcut, action) in [
                                    (crate::ui::icons::ARROWS_OUT, self.application_shortcuts.fullscreen.as_str(), "Toggle Fullscreen mode"),
                                    (crate::ui::icons::INFO, self.application_shortcuts.media_information.as_str(), "Media information"),
                                    (crate::ui::icons::FOLDER_OPEN, self.application_shortcuts.media_folder.as_str(), "Open containing folder"),
                                    (crate::ui::icons::GEAR, self.application_shortcuts.preferences.as_str(), "Preferences..."),
                                    (crate::ui::icons::PENCIL_SIMPLE, self.application_shortcuts.edit_config.as_str(), "Edit configuration file"),
                                ] {
                                    ui.label(icon);
                                    ui.label(if shortcut.is_empty() { "—" } else { shortcut });
                                    ui.label(crate::ui::i18n::tr(language, action));
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
        crate::ui::toasts::draw(ui.ctx());
        crate::ui::remote_location::draw(self, ui.ctx());
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // Consumer lifecycle/placement is not a mutation of the authority.
        if !crate::peer::active() { self.save_config(); }
    }
    fn persist_egui_memory(&self)->bool { !crate::peer::active() }

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
        if !crate::peer::active() { self.save_config(); }
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
    fn process_application_shortcuts(&mut self, ctx: &egui::Context) {
        if !self.keyboard_shortcuts_enabled || self.hardware_binding_capturing {
            return;
        }
        let text_editing = ctx.egui_wants_keyboard_input();
        let mut actions = Vec::new();
        ctx.input_mut(|input| {
            input.events.retain(|event| {
                if let Some(action) = self
                    .application_shortcuts
                    .action_for_event(event, text_editing)
                {
                    actions.push(action);
                    false // Do not also trigger transport/hardware shortcuts.
                } else {
                    true
                }
            })
        });
        for action in actions {
            use crate::application_shortcuts::ApplicationAction;
            use crate::platform::interop::InteropCommand;
            let command = match action {
                ApplicationAction::Fullscreen => InteropCommand::ToggleFullscreen,
                ApplicationAction::MediaInformation => InteropCommand::OpenMediaInformation,
                ApplicationAction::Preferences => InteropCommand::OpenPreferences,
                ApplicationAction::EditConfig => InteropCommand::EditConfiguration,
                ApplicationAction::MediaFolder => InteropCommand::OpenMediaFolder,
            };
            self.apply_interop_command(ctx, command, "keyboard");
        }
    }

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
        let global_bindings =
            if self.keyboard_shortcuts_enabled && self.global_hardware_hotkeys_enabled {
                self.hardware_key_bindings.as_slice()
            } else {
                &[]
            };
        self.hardware_hotkey_runtime.sync(global_bindings);
        let global_events = self.hardware_hotkey_runtime.drain_events();
        for event in global_events {
            // Recording a new chord must never trigger another binding, but a
            // key-up from a previously active hold binding is safety-critical:
            // it still has to send the configured release/stop action.
            if !event.pressed || !self.hardware_binding_capturing {
                self.dispatch_hardware_binding(&event.binding_id, event.pressed);
            }
        }

        if !self.keyboard_shortcuts_enabled {
            return;
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
        if crate::platform::windows::take_shell_reinitialize_request() {
            self.shell_initialized = false;
            self.last_thumbnail_button_state = None;
        }
        if !self.shell_initialized {
            let hwnd = self
                .window_handle
                .unwrap_or_else(crate::platform::windows::get_registered_hwnd);
            if hwnd != 0 {
                let result = crate::platform::windows::install_shell_message_hook(hwnd)
                    .and_then(|_| {
                        crate::platform::windows::init_taskbar_thumbnail_toolbar(
                            hwnd,
                            self.windows_thumbnail_toolbar,
                        )
                    })
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
                crate::platform::windows::THUMB_BUTTON_MUTE => {
                    self.toggle_audio_muted();
                }
                crate::platform::windows::THUMB_BUTTON_FULLSCREEN => {
                    self.toggle_fullscreen(ctx);
                }
                crate::platform::windows::TRAY_CMD_MUTE => {
                    self.toggle_audio_muted();
                }
                crate::platform::windows::TRAY_CMD_OPEN => {
                    self.open_media_file_dialog(ctx);
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

    /// Change the native stacking level only on policy or playback transitions.
    /// Target the main window explicitly, never a preferences/dialog viewport.
    fn sync_window_level(&mut self, ctx: &egui::Context) {
        let video_loaded = self.current_video_path.is_some()
            && self.media_metadata_loaded
            && self.media_tracks.iter().any(|track| {
                track.kind == MediaTrackType::Video
                    && self.current_vid == track.id.to_string()
                    && track.image != Some(true)
                    && track.album_art != Some(true)
            });
        let active = !self.web_only
            && self.always_on_top.is_active(video_loaded, self.is_paused, self.is_eof);
        if self.applied_always_on_top != Some(active) {
            ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::WindowLevel(
                if active { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal },
            ));
            self.applied_always_on_top = Some(active);
        }
    }

    /// Synchronizes playback time, duration, pause state, and error condition
    /// with the Windows taskbar progress state and thumbnail toolbar buttons.
    pub fn update_shell_state(&mut self) {
        // All native/Web/API commands settle through this actual MPV state.
        // No hardware RPC or serial I/O runs on the GUI/render thread.
        if let Ok(mut sample) = self.engine_handle.media_playback.lock() {
            sample.name.clone_from(&self.app_name);
            if !self.engine_handle.media_clock_owned.load(std::sync::atomic::Ordering::Acquire) {
            sample.duration_ms = (self.duration.is_finite() && self.duration > 0.0)
                .then_some((self.duration * 1000.0).round() as u64);
            let playing = self.current_video_path.is_some() && !self.is_paused && !self.is_eof
                && !self.is_scrubbing && !self.estop_active && !sample.buffering;
            if sample.playing != playing {
                sample.sampled_at = std::time::Instant::now();
            }
            sample.playing = playing;
            sample.loaded = self.current_video_path.is_some();
            sample.rate = self.playback_rate.clamp(0.25, 4.0);
            }
        }
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
        if hwnd != 0 && (!self.windows_video_taskbar_thumbnail || self.current_video_path.is_none())
        {
            let _ = crate::platform::windows::update_video_taskbar_thumbnail(hwnd, None);
        }
        let thumbnail_state = (
            self.is_paused,
            self.is_muted,
            self.was_fullscreen,
            self.current_video_path.is_some(),
            self.windows_thumbnail_toolbar,
            crate::platform::windows::thumbnail_toolbar_metrics(hwnd),
        );
        if self.shell_initialized
            && hwnd != 0
            && self.last_thumbnail_button_state != Some(thumbnail_state)
            && crate::platform::windows::update_taskbar_thumbnail_buttons(
                hwnd,
                thumbnail_state.0,
                thumbnail_state.1,
                thumbnail_state.2,
                thumbnail_state.3,
                thumbnail_state.4,
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

    /// Refreshes every placed controller-owned cue from PCController's latest
    /// effect catalog while preserving cue identity and timeline position.
    ///
    /// The catalog snapshot prevents a deliberately resized cue from being
    /// overwritten on every frame. A real catalog edit, reconnect, or rename
    /// advances the snapshot and updates every template with the same durable
    /// controller reference, including isolated copies used by multiple cues.
    fn refresh_controller_effect_timeline_metadata(&mut self) {
        let Some(capabilities) = self.advertised_hardware() else {
            return;
        };
        let catalog = controller_effect_catalog(&capabilities);
        if catalog == self.hardware_effect_authoring.timeline_catalog {
            return;
        }
        self.hardware_effect_authoring.timeline_catalog = catalog.clone();
        if reconcile_controller_effect_templates(&mut self.timeline, &catalog) {
            self.persist_timeline_track_preferences();
            self.sync_timeline_engine();
            self.save_config();
        }
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
        if self.hardware_effect_authoring.pending_operation.is_some() || self.hardware_effect_authoring.pending_saved_macro_id.is_some() {
            return Err("another hardware effect operation is still running".to_string());
        }
        if !self
            .engine_handle
            .is_connected
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            return Err(
                "PCController is reconnecting. Wait for the hardware status to become ready and try again."
                    .to_string(),
            );
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
        if self.effect_library_draft.kind != "sequence" {
            return Err("Select a sequence to capture into".into());
        }
        if self.hardware_effect_authoring.active {
            return Err("Finish or discard the current capture first".into());
        }
        self.save_controller_effect()?;
        self.hardware_effect_authoring.record_after_publish = Some((
            format!("effect:{}", self.effect_library_draft.id),
            self.hardware_effect_authoring.capture_mode.clone(),
        ));
        Ok(())
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
        self.request_hardware_effect_command("effect-preview", format!("effect play {id}"))?;
        self.hardware_effect_authoring
            .begin_effect_preview(&format!("effect:{id}"), None);
        Ok(())
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
        if self.hardware_effect_authoring.active || self.advertised_hardware().is_some_and(|hardware| hardware.effect_recording.active) {
            return Err("Finish or discard capture before publishing edits".into());
        }
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
        let category = checked_text(&draft.category, "Effect group", true)?;
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
        let command = if draft.original_name.trim().is_empty() {
            format!("effect group create {name} {icon}")
        } else {
            let original = Self::controller_command_argument(&draft.original_name)
                .ok_or_else(|| "The original effect group name is invalid".to_string())?;
            format!("effect group update {original} {name} {icon}")
        };
        self.request_hardware_effect_command("effect-group-save", command)
    }

    pub(crate) fn play_controller_effect(&mut self, reference: &str) -> Result<(), String> {
        if reference.trim().is_empty() {
            return Err("Select a PCController effect first".to_string());
        }
        if !self.controller_effect_is_advertised(reference) {
            return Err(
                "This effect is not published in PCController. Publish it before running it."
                    .to_string(),
            );
        }
        let canonical = canonical_effect_reference(reference)
            .ok_or_else(|| "Select a PCController effect first".to_string())?;
        let duration = self.controller_effect_preview_duration(&canonical);
        self.request_hardware_effect_command(
            "effect-play",
            format!("effect play {}", reference.trim()),
        )?;
        self.hardware_effect_authoring
            .begin_effect_preview(&canonical, duration);
        Ok(())
    }

    fn controller_effect_preview_duration(
        &self,
        reference: &str,
    ) -> Option<std::time::Duration> {
        let reference = canonical_effect_reference(reference)?;
        let id = reference.strip_prefix("effect:")?;
        let duration_ms = self
            .advertised_hardware()?
            .macros
            .iter()
            .find(|effect| {
                effect.id.to_string() == id || effect.name.eq_ignore_ascii_case(id)
            })?
            .duration_ms;
        (duration_ms > 0).then(|| std::time::Duration::from_millis(duration_ms))
    }

    pub(crate) fn controller_effect_preview_phase(
        &mut self,
        reference: &str,
    ) -> ControllerEffectPreviewPhase {
        self.hardware_effect_authoring
            .effect_preview_phase(reference, std::time::Instant::now())
    }

    pub(crate) fn controller_effect_preview_action_enabled(
        &self,
        phase: ControllerEffectPreviewPhase,
    ) -> bool {
        self.hardware_effect_authoring.pending_operation.is_none()
            && matches!(
                phase,
                ControllerEffectPreviewPhase::Run | ControllerEffectPreviewPhase::Stop
            )
    }

    pub(crate) fn invoke_controller_effect_preview_action(
        &mut self,
        reference: &str,
    ) -> Result<(), String> {
        match self.controller_effect_preview_phase(reference) {
            ControllerEffectPreviewPhase::Run => self.play_controller_effect(reference),
            ControllerEffectPreviewPhase::Stop => self.stop_controller_effect(reference),
            ControllerEffectPreviewPhase::Starting => {
                Err("This effect is still starting".to_string())
            }
            ControllerEffectPreviewPhase::Stopping => {
                Err("This effect is still stopping".to_string())
            }
        }
    }

    pub(crate) fn controller_effect_is_advertised(&self, reference: &str) -> bool {
        if self
            .hardware_effect_authoring
            .effect_publish_is_acknowledged(reference)
        {
            return true;
        }
        let reference = reference.trim();
        let id = reference.strip_prefix("effect:").unwrap_or(reference);
        self.advertised_hardware().is_some_and(|capabilities| {
            capabilities
                .macros
                .iter()
                .any(|effect| effect.id.to_string() == id || effect.name.eq_ignore_ascii_case(id))
                || capabilities.strip_effects.iter().any(|effect| {
                    effect.id.eq_ignore_ascii_case(id) || effect.name.eq_ignore_ascii_case(id)
                })
        })
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
            if result.operation.starts_with("rf-") {
                if self.rf.complete(&result.operation, result.result) {
                    if let Err(error) = self.request_rf("catalog", serde_json::json!({"read_board":true})) { self.rf.error = error; }
                }
                continue;
            }
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
                            self.hardware_effect_authoring.append_discarded = false;
                            self.hardware_effect_authoring.pending_saved_macro_id =
                                saved_effect_id(&output);
                            self.engine_handle.request_catalog_refresh();
                        }
                        "macro-discard" => {
                            self.hardware_effect_authoring.active = false;
                            self.hardware_effect_authoring.append_discarded = true;
                            self.hardware_effect_authoring.pending_saved_macro_id = self.hardware_effect_authoring.append_target;
                            self.engine_handle.request_catalog_refresh();
                        }
                        "effect-play" | "effect-preview" | "strip-rainbow" => {
                            self.hardware_effect_authoring
                                .acknowledge_effect_preview(std::time::Instant::now());
                            self.engine_handle.request_catalog_refresh();
                        }
                        "effect-stop" | "strip-stop" | "strip-clear" => {
                            self.hardware_effect_authoring.finish_effect_preview();
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
                        "effect-save" => {
                            let reference = self.hardware_effect_authoring.record_after_publish.as_ref()
                                .map(|(target, _)| target.clone())
                                .unwrap_or_else(|| format!("effect:{}", self.effect_library_draft.id.trim()));
                            self.effect_library_draft.reference = reference.clone();
                            self.effect_library_draft.is_new = false;
                            self.hardware_effect_authoring
                                .acknowledge_effect_publish(&reference);
                            self.save_config();
                            self.engine_handle.request_catalog_refresh();
                            if let Some((target, mode)) = self.hardware_effect_authoring.record_after_publish.take() {
                                self.hardware_effect_authoring.append_discarded = false;
                                self.hardware_effect_authoring.append_target = target.strip_prefix("effect:").and_then(|id|id.parse().ok());
                                if let Err(error) = self.request_hardware_effect_command("macro-start", format!("effect record append {target} {mode}")) {
                                    self.hardware_effect_authoring.append_target = None;
                                    self.set_osd(error);
                                }
                            }
                        }
                        "effect-delete" => {
                            self.hardware_effect_authoring
                                .forget_effect_publish(&self.effect_library_draft.reference);
                            self.engine_handle.request_catalog_refresh();
                        }
                        "strip-config" | "strip-fill" | "strip-frame" | "strip-pixel"
                        | "strip-status" => {
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
                    if result.operation == "effect-save" { self.hardware_effect_authoring.record_after_publish = None; }
                    if result.operation == "macro-start" { self.hardware_effect_authoring.append_target = None; }
                    if matches!(result.operation.as_str(), "effect-play" | "effect-preview") {
                        self.hardware_effect_authoring.finish_effect_preview();
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
        let Some(capabilities) = self.advertised_hardware() else { return; };
        let Some(hardware_macro) = capabilities.macros.iter().find(|item| item.id == id).cloned()
        else {
            return;
        };
        let effect = controller_macro_effect_preset(&hardware_macro).effect;
        if self.hardware_effect_authoring.append_target == Some(id) {
            // A save ACK may arrive before the refreshed catalog. Do not reload
            // the old prefix into the editor or its existing timeline cues.
            let recording = &capabilities.effect_recording;
            if recording.active || u64::from(recording.id) != id { return; }
            if !self.hardware_effect_authoring.append_discarded
                && serde_json::to_value(&hardware_macro.steps).ok() != serde_json::to_value(&recording.preview).ok() { return; }
            let catalog = controller_effect_catalog(&capabilities);
            reconcile_controller_effect_templates(&mut self.timeline, &catalog);
            self.hardware_effect_authoring.timeline_catalog = catalog;
            crate::ui::effects_library::select_sequence(self, &hardware_macro);
            self.hardware_effect_authoring.pending_saved_macro_id = None;
            self.hardware_effect_authoring.append_target = None;
            self.hardware_effect_authoring.status.clear();
            self.sync_timeline_engine();
            return;
        }
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
        if crate::peer::active() && let InteropCommand::Launch{request}=&command {
            if let Some(target)=&request.target{self.load_media_target(target);}
            if let Some(value)=request.volume{self.apply_interop_command(ctx,InteropCommand::SetVolume{value},source);}
            if request.fullscreen{self.set_fullscreen(ctx,true);}
            if request.activate{ctx.send_viewport_cmd(egui::ViewportCommand::Focus);}
            for command in request.commands.clone(){self.apply_interop_command(ctx,command,source);}
            return;
        }

        // Dialogs and physical client-window placement remain local. Everything
        // that operates the session goes through the authority's unified engine.
        if let Some(client)=crate::peer::client() && !crate::peer::mirroring()
            && !matches!(&command,InteropCommand::OpenPreferences|InteropCommand::OpenMediaInformation|InteropCommand::OpenBoardInformation{..}|InteropCommand::OpenRfManager|InteropCommand::Activate|InteropCommand::Minimize|InteropCommand::Maximize|InteropCommand::Restore|InteropCommand::SetFullscreen{..}|InteropCommand::ToggleFullscreen|InteropCommand::GetStatus) {
            if matches!(&command,InteropCommand::OpenMediaFolder){crate::ui::peer_browser::open(ctx,crate::ui::peer_browser::Purpose::Media,None);return;}
            if let Err(error)=serde_json::to_value(&command).map_err(|error|error.to_string()).and_then(|value|client.queue("/api/player/command",value)){self.show_error=Some(error);}
            return;
        }

        match command {
            InteropCommand::OpenRfManager => { self.rf.open = true; let _ = self.request_rf("catalog", serde_json::json!({"read_board":true})); },
            InteropCommand::RfControl { operation, params } => { if let Err(error) = self.request_rf(&operation, params) { self.rf.error = error; } },
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
                if let Some(next) = self.remote_neighbor(1, false) { self.play_remote_location(next); }
                else if !self.has_remote_playlist() { let _ = self.mpv.command("playlist-next", &["force"]); }
            }
            InteropCommand::Previous => {
                if let Some(previous) = self.remote_neighbor(-1, false) { self.play_remote_location(previous); }
                else if !self.has_remote_playlist() { let _ = self.mpv.command("playlist-prev", &["force"]); }
            }
            InteropCommand::PreviousChapter => self.previous_media_chapter(),
            InteropCommand::NextChapter => self.next_media_chapter(),
            InteropCommand::SetChapter { index } => self.jump_to_media_chapter(index),
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
                self.set_audio_muted(muted);
            }
            InteropCommand::ToggleMute => {
                self.toggle_audio_muted();
            }
            InteropCommand::SetRate { rate } => {
                self.set_playback_speed(rate, true);
            }
            InteropCommand::Open { target } => self.load_media_target(&target),
            InteropCommand::BrowseRemote { target, use_proxy } => { if let Err(error) = crate::remote_location::request(&target, use_proxy, false, ctx) { self.set_osd(error); } },
            InteropCommand::SelectRemote { target, play } => { if let Err(error) = crate::remote_location::select(&target, play) { self.set_osd(error); } },
            InteropCommand::SortRemote { by, descending } => crate::remote_location::sort(by, descending),
            InteropCommand::CloseRemoteBrowser => crate::remote_location::close(),
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
            InteropCommand::OpenMediaInformation => {
                self.open_or_focus_tab(crate::ui::layout::PealayerTab::MediaInspector);
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            InteropCommand::OpenMediaFolder => {
                let result = self
                    .current_video_path
                    .as_deref()
                    .ok_or_else(|| "No media is loaded".to_owned())
                    .and_then(crate::application_shortcuts::containing_media_folder)
                    .and_then(|folder| {
                        open::that_detached(folder).map_err(|error| error.to_string())
                    });
                if let Err(error) = result {
                    self.set_osd(error);
                    return;
                }
            }
            InteropCommand::EditConfiguration => {
                let config = self.runtime_config_snapshot();
                if let Err(error) = crate::ui::preferences::perform_config_path_action(
                    crate::ui::preferences::ConfigPathAction::Edit,
                    &config,
                ) {
                    self.set_osd(error);
                    return;
                }
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
            InteropCommand::ShowOsd { message, options } => {
                self.set_osd_with_options(message, options);
                return;
            }
            InteropCommand::HideOsd => {
                self.clear_osd();
                return;
            }
            InteropCommand::PublishToast { toast } => {
                if let Err(error) = crate::messaging::publish(toast, source) { self.set_osd(error); }
                ctx.request_repaint(); return;
            }
            InteropCommand::DismissToast { id } => {
                crate::messaging::dismiss(&id); ctx.request_repaint(); return;
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
            InteropCommand::UpdateTimelineTrack {
                key,
                linked,
                visible,
                muted,
                soloed,
                locked,
                selected,
            } => {
                let patch = crate::ui::layout::TimelineTrackPatch {
                    linked,
                    visible,
                    muted,
                    soloed,
                    locked,
                    selected,
                };
                if let Err(error) = crate::ui::layout::update_timeline_track(self, &key, patch) {
                    self.set_osd(error);
                }
            }
            InteropCommand::ManageTimelineTrack { key } => {
                if let Err(error) = crate::ui::layout::manage_timeline_track_by_key(self, &key) {
                    self.set_osd(error);
                }
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
            InteropCommand::UpdateEffectCue {
                instance_id,
                start_time_ms,
                duration_ms,
            } => {
                let Ok(instance_id) = uuid::Uuid::parse_str(&instance_id) else {
                    self.set_osd(self.tr("Cue is no longer available"));
                    return;
                };
                let Some(instance_index) = self
                    .timeline
                    .instances
                    .iter()
                    .position(|instance| instance.id == instance_id)
                else {
                    self.set_osd(self.tr("Cue is no longer available"));
                    return;
                };

                let effect_id = self.timeline.instances[instance_index].effect_id;
                let Some(template) = self
                    .timeline
                    .templates
                    .iter()
                    .find(|template| template.id == effect_id)
                    .cloned()
                else {
                    self.set_osd(self.tr("Effect is no longer available"));
                    return;
                };

                // Duration is placement-specific. Give a resized cue its own
                // template so another placement of the reusable effect never
                // changes underneath the user.
                let resolved_effect_id = if template.duration_resizable()
                    && template.duration_ms != duration_ms
                {
                    let mut placement_template = template;
                    placement_template.id = uuid::Uuid::new_v4();
                    update_effect_duration(&mut placement_template, duration_ms);
                    let id = placement_template.id;
                    self.timeline.templates.push(placement_template);
                    id
                } else {
                    effect_id
                };

                let instance = &mut self.timeline.instances[instance_index];
                instance.start_time_ms = start_time_ms;
                instance.effect_id = resolved_effect_id;
                self.selected_instance_ids.clear();
                self.selected_instance_ids.insert(instance_id);
                self.sync_timeline_engine();
            }
            InteropCommand::AddDirectControlCue {
                control_key,
                value_basis_points,
                start_time_ms,
                duration_ms,
            } => {
                if let Err(error) = self.add_direct_control_cue(
                    &control_key,
                    value_basis_points,
                    start_time_ms,
                    duration_ms,
                ) {
                    self.set_osd(error);
                }
            }
            InteropCommand::UpdateDirectControlCueValue {
                instance_id,
                value_basis_points,
            } => {
                let result = uuid::Uuid::parse_str(&instance_id)
                    .map_err(|_| "Cue is no longer available".to_string())
                    .and_then(|id| self.update_direct_control_cue_value(id, value_basis_points));
                if let Err(error) = result {
                    self.set_osd(error);
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
            InteropCommand::CreateControllerEffectGroup { name, icon } => {
                if let Err(error) = self.save_controller_effect_group(ControllerEffectGroupDraft {
                    original_name: String::new(),
                    name,
                    icon,
                }) {
                    self.set_osd(error);
                }
            }
            InteropCommand::SaveControllerEffect { effect } => {
                if self.hardware_effect_authoring.active || self.hardware_effect_authoring.pending_operation.is_some() {
                    self.set_osd("Finish the current hardware effect operation first".into());
                    return;
                }
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
                        .filter(|mode| matches!(*mode, "auto" | "host" | "mcu"))
                        .unwrap_or("auto")
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
            InteropCommand::StartControllerEffectRecording {
                name,
                category,
                color,
                mode,
                effect,
            } => {
                if let Some(effect) = effect {
                    if self.hardware_effect_authoring.active || self.hardware_effect_authoring.pending_operation.is_some() {
                        self.set_osd("Finish the current hardware effect operation first".into());
                        return;
                    }
                    self.hardware_effect_authoring.capture_mode = mode.clone();
                    self.apply_interop_command(ctx, InteropCommand::SaveControllerEffect { effect }, source);
                    if self.hardware_effect_authoring.pending_operation.as_deref() == Some("effect-save") {
                        let target = format!("effect:{}", self.effect_library_draft.id);
                        self.hardware_effect_authoring.record_after_publish = Some((target, mode));
                    }
                    return;
                }
                self.hardware_effect_authoring.name = name;
                self.hardware_effect_authoring.category = category;
                self.hardware_effect_authoring.color = color;
                self.hardware_effect_authoring.capture_mode = mode;
                self.effect_library_draft.name = self.hardware_effect_authoring.name.clone();
                self.effect_library_draft.category = self.hardware_effect_authoring.category.clone();
                self.effect_library_draft.color = self.hardware_effect_authoring.color.clone();
                if let Err(error) = self.start_hardware_effect_recording() {
                    self.set_osd(error);
                    return;
                }
            }
            InteropCommand::RefreshControllerEffectRecording => {
                if let Err(error) = self.refresh_hardware_effect_recording() {
                    self.set_osd(error);
                    return;
                }
            }
            InteropCommand::SaveControllerEffectRecording => {
                if let Err(error) = self.save_hardware_effect_recording() {
                    self.set_osd(error);
                    return;
                }
            }
            InteropCommand::DiscardControllerEffectRecording => {
                if let Err(error) = self.discard_hardware_effect_recording() {
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
                let _ = self.engine_handle.queue_controller_intent(
                    format!("pwm.{channel}"),
                    "controller.pwm.set",
                    serde_json::json!({"channel": channel, "value": value}),
                    false,
                );
            }
            InteropCommand::RefreshHardwareCatalog => {
                self.engine_handle.request_catalog_refresh();
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
        if !matches!(source, "keyboard" | "menu") {
            self.set_osd(format!("{source}: command applied"));
        }
    }

    fn update_seek_completion_state(&mut self) {
        for completion in self.seek_controller.take_completed() {
            if completion.mode == crate::mpv::seek::SeekMode::Commit
                && self
                    .pending_scrub_commit
                    .as_ref()
                    .is_some_and(|pending| pending.request_id == completion.request_id)
                && let Some(pending) = self.pending_scrub_commit.as_mut()
            {
                pending.dispatched = true;
            }
        }
    }

    fn settle_scrub_commit_if_ready(&mut self) {
        let Some(pending) = self.pending_scrub_commit else {
            return;
        };
        if !pending.dispatched || !pending.playback_restarted {
            return;
        }

        let actual_time = self
            .mpv
            .get_property::<f64>("time-pos")
            .unwrap_or(pending.target_time);
        if !seek_target_reached(actual_time, pending.target_time, self.media_fps) {
            // A PlaybackRestart from an older preview seek can arrive after the
            // final commit was queued. Require the restart for the committed
            // target instead of allowing that stale event to release playback.
            if let Some(current) = self.pending_scrub_commit.as_mut() {
                current.playback_restarted = false;
            }
            return;
        }

        let retain_exact_target = !self.was_playing_before_scrub;
        let settled_time = settled_seek_position(
            actual_time,
            pending.target_time,
            self.was_playing_before_scrub,
        );
        self.playback_time = settled_time;
        self.seek_pos = retain_exact_target.then_some(pending.target_time);
        self.pending_scrub_commit = None;
        self.engine_handle.playback_time_ms.store(
            (settled_time * 1_000.0).max(0.0) as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
        let _ = self
            .engine_handle
            .sender
            .send(crate::four_d::engine::EngineMessage::Seek(
                (settled_time * 1_000.0).max(0.0) as u64,
            ));

        if self.was_playing_before_scrub {
            let _ = self.mpv.set_property("pause", false);
            self.is_paused = false;
            self.engine_handle
                .is_playing
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.was_playing_before_scrub = false;
    }

    fn reset_scrub_state(&mut self) {
        self.seek_pos = None;
        self.is_scrubbing = false;
        self.was_playing_before_scrub = false;
        self.pending_scrub_commit = None;
        let _ = self.seek_controller.take_completed();
    }

    /// Polls and processes all pending MPV events and updates application state.
    pub fn process_events(&mut self) {
        use libmpv2::events::{Event, PropertyData};

        self.update_seek_completion_state();
        loop {
            match self.mpv_client.wait_event(0.0) {
                Some(Ok(Event::PropertyChange {
                    reply_userdata,
                    change,
                    ..
                })) => match (reply_userdata, change) {
                    (1, PropertyData::Double(v)) => {
                        // Publish decoded MPV time even when the UI retains an
                        // exact logical seek target or a scrub preview position.
                        if !self.engine_handle.media_clock_owned.load(std::sync::atomic::Ordering::Acquire)
                            && let Ok(mut sample) = self.engine_handle.media_playback.lock() {
                            sample.position_ms = (v.max(0.0) * 1000.0).round() as u64;
                            sample.sampled_at = std::time::Instant::now();
                        }
                        if !self.is_scrubbing
                            && self.pending_scrub_commit.is_none()
                            && self.seek_pos.is_none()
                        {
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
                    (6, PropertyData::Flag(v)) => {
                        if !self.uses_processed_subtitle_overlay() {
                            self.sub_visibility = v;
                        }
                    }
                    (7, PropertyData::Double(v)) => {
                        self.sub_font_size = v;
                        self.sync_subtitle_rendering();
                    }
                    (8, PropertyData::Double(v)) => self.sub_delay = v,
                    (9, PropertyData::Str(v)) => {
                        self.current_sid = v.to_string();
                        self.sync_subtitle_rendering();
                    }
                    (9, PropertyData::OsdStr(v)) => {
                        self.current_sid = v.to_string();
                        self.sync_subtitle_rendering();
                    }
                    (10, PropertyData::Double(v)) => self.audio_delay = v,
                    (11, PropertyData::Str(v)) => self.current_aid = v.to_string(),
                    (11, PropertyData::OsdStr(v)) => self.current_aid = v.to_string(),
                    (12, PropertyData::Flag(v)) => {
                        let advance = v && !self.is_eof;
                        self.is_eof = v;
                        if advance && !crate::peer::active() { if let Some(next) = self.remote_neighbor(1, true) { self.play_remote_location(next); } }
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
                    (19, PropertyData::Str(v)) => {
                        self.subtitle_text = v.to_string();
                        self.sync_subtitle_rendering();
                    }
                    (19, PropertyData::OsdStr(v)) => {
                        self.subtitle_text = v.to_string();
                        self.sync_subtitle_rendering();
                    }
                    (20, PropertyData::Double(v)) => {
                        self.sub_position_percent = v.clamp(0.0, 100.0);
                        self.sync_subtitle_rendering();
                    }
                    (21, PropertyData::Double(v)) => {
                        if v.is_finite() && (0.05..=20.0).contains(&v) {
                            if (self.video_aspect_ratio - v).abs() > f64::EPSILON {
                                self.pending_video_aspect_resize = true;
                            }
                            self.video_aspect_ratio = v;
                        }
                    }
                    (22, PropertyData::Flag(v)) => {
                        if !self.engine_handle.media_clock_owned.load(std::sync::atomic::Ordering::Acquire)
                            && let Ok(mut sample) = self.engine_handle.media_playback.lock() {
                            sample.buffering = v;
                        }
                    }
                    _ => {}
                },
                Some(Ok(Event::EndFile(reason))) => {
                    if reason == 4 {
                        if crate::peer::active(){
                            // Decoder failure is only a preview failure. Do not
                            // stop or advance the server or its hardware cues.
                            crate::peer::set_preview_error("Video preview could not be decoded; remote controls remain available".into());
                        } else {
                        // MPV_END_FILE_REASON_ERROR
                        self.show_error =
                            Some("Error: Failed to play the selected file.".to_string());
                        }
                    }
                }
                Some(Ok(Event::Seek)) => {
                    if !self.is_scrubbing && self.pending_scrub_commit.is_none() {
                        self.seek_pos = None;
                    }
                    let current_pos_ms =
                        (self.seek_pos.unwrap_or(self.playback_time) * 1000.0) as u64;
                    let _ = self
                        .engine_handle
                        .sender
                        .send(crate::four_d::engine::EngineMessage::Seek(current_pos_ms));
                }
                Some(Ok(Event::PlaybackRestart)) => {
                    if let Some(pending) = self.pending_scrub_commit.as_mut() {
                        pending.playback_restarted = true;
                    }
                }
                Some(Ok(Event::StartFile)) => {
                    if !self.engine_handle.media_clock_owned.load(std::sync::atomic::Ordering::Acquire)
                        && let Ok(mut sample) = self.engine_handle.media_playback.lock() {
                        sample.position_ms = 0;
                        sample.buffering = false;
                        sample.sampled_at = std::time::Instant::now();
                    }
                    self.show_error = None;
                    self.is_eof = false;
                    self.playback_time = 0.0;
                    self.duration = 0.0;
                    self.is_seekable = false;
                    self.media_metadata_loaded = false;
                    self.media_file_info = crate::media_info::MediaFileInfo::default();
                    self.cache_duration = None;
                    self.cache_buffering_percent = None;
                    self.video_aspect_ratio = 16.0 / 9.0;
                    self.subtitle_text.clear();
                    self.clear_subtitle_overlay();
                    self.reset_scrub_state();
                    self.refresh_media_tracks();
                    self.sync_subtitle_rendering();
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
                    if let Ok(aspect_ratio) =
                        self.mpv.get_property::<f64>("video-out-params/aspect")
                        && aspect_ratio.is_finite()
                        && (0.05..=20.0).contains(&aspect_ratio)
                    {
                        self.video_aspect_ratio = aspect_ratio;
                        // A different file or selected video stream can have the
                        // same numerical aspect as its predecessor. FileLoaded is
                        // still a semantic video change and must re-apply the
                        // configured Simple-workspace window geometry.
                        self.pending_video_aspect_resize = true;
                    }
                    if let Some(position) = self.pending_resume_position.take().filter(|_|!crate::peer::active()) {
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
                    self.refresh_media_tracks();
                }
                Some(Ok(_)) => {}
                _ => break,
            }
        }
        // The backend acknowledgement and PlaybackRestart can arrive in either
        // order within this frame. Drain once more, then settle only when both
        // halves of the exact commit have been observed.
        self.update_seek_completion_state();
        self.settle_scrub_commit_if_ready();
        if !crate::peer::active() && self.last_playback_position_checkpoint.elapsed() >= std::time::Duration::from_secs(5) {
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
        self.reset_scrub_state();
        let _ = self.mpv.command("seek", &["0", "absolute+exact"]);
        let pending_hardware = self.engine_handle.request_prepared_play();
        let _ = self.mpv.set_property("pause", pending_hardware);
        self.is_paused = false;
        self.is_eof = false;
        self.playback_time = 0.0;
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
        if self.is_playback_finished() { self.replay(); return; }
        if self.engine_handle.request_prepared_play() {
            let _=self.mpv.set_property("pause",true);
            return;
        }
        if self.is_playback_finished() {
            self.replay();
        } else {
            // A paused exact seek may intentionally retain a sub-frame logical
            // playhead. Once playback starts, mpv's decoded clock is again the
            // authoritative position.
            self.seek_pos = None;
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
        if let Ok(mut plan)=self.engine_handle.prepared_timeline.lock(){plan.play_requested=false;}
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
        if seconds < 0.0 {
            self.is_eof = false;
        }
        let target = (self.playback_time + seconds).clamp(0.0, self.duration.max(0.0));
        // Use the same exact-commit lifecycle as absolute/timeline seeks so a
        // paused relative seek also retains a sub-frame authoring position.
        self.scrub_to(target);
        self.finish_scrub(target);
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
        // Establish the same pause/preview state used by a drag before queuing
        // the exact commit. Otherwise a click-to-seek can resume from the old
        // frame while the background worker has not executed the seek yet.
        self.scrub_to(target);
        self.finish_scrub(target);
        self.set_osd(format!(
            "{}: {}",
            self.tr("Seek"),
            crate::ui::controls::format_player_time(target, self.duration >= 3600.0, true)
        ));
    }

    pub(crate) fn media_chapters(&self) -> Vec<crate::media_info::MediaChapter> {
        crate::media_info::chapters(&self.media_file_info)
    }

    pub(crate) fn active_media_chapter(&self) -> Option<crate::media_info::MediaChapter> {
        let position = self.seek_pos.unwrap_or(self.playback_time);
        self.media_chapters()
            .into_iter()
            .rev()
            .find(|chapter| chapter.time_seconds <= position + 0.001)
    }

    pub(crate) fn jump_to_media_chapter(&mut self, index: i64) {
        let Some(chapter) = self
            .media_chapters()
            .into_iter()
            .find(|chapter| chapter.index == index)
        else {
            return;
        };
        self.seek_absolute(chapter.time_seconds);
        self.set_osd(format!("{}: {}", self.tr("Chapter"), chapter.title));
    }

    pub(crate) fn next_media_chapter(&mut self) {
        let position = self.seek_pos.unwrap_or(self.playback_time);
        if let Some(chapter) = self
            .media_chapters()
            .into_iter()
            .find(|chapter| chapter.time_seconds > position + 0.05)
        {
            self.jump_to_media_chapter(chapter.index);
        }
    }

    pub(crate) fn previous_media_chapter(&mut self) {
        let position = self.seek_pos.unwrap_or(self.playback_time);
        let chapters = self.media_chapters();
        let Some(active_position) = chapters
            .iter()
            .rposition(|chapter| chapter.time_seconds <= position + 0.001)
        else {
            return;
        };
        let active = &chapters[active_position];
        let target_position = if position - active.time_seconds > 3.0 {
            active_position
        } else {
            active_position.saturating_sub(1)
        };
        self.jump_to_media_chapter(chapters[target_position].index);
    }

    /// Advances or reverses playback by the configured number of frames.
    pub fn step_frames(&mut self, direction: i32) {
        if self.current_video_path.is_none() || direction == 0 {
            return;
        }
        // Frame stepping explicitly returns control to mpv's decoded-frame
        // clock, replacing any paused sub-frame timeline position.
        self.seek_pos = None;
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
            if self.pending_scrub_commit.take().is_none() {
                self.was_playing_before_scrub = !self.is_paused
                    && self.current_video_path.is_some()
                    && !self.is_playback_finished();
                if self.was_playing_before_scrub {
                    let _ = self.mpv.set_property("pause", true);
                    self.is_paused = true;
                    self.engine_handle
                        .is_playing
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                }
                self.commit_recorded_samples();
            }
        }

        if clamped < self.duration {
            self.is_eof = false;
        }

        self.seek_pos = Some(clamped);
        let _ = self.seek_controller.request_scrub(clamped);
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
        let request_id = self.seek_controller.request_commit(clamped);
        self.pending_scrub_commit = Some(PendingScrubCommit {
            request_id,
            target_time: clamped,
            dispatched: false,
            playback_restarted: false,
        });
        self.commit_recorded_samples();
        // The gesture is over, but keep seek_pos authoritative until mpv emits
        // PlaybackRestart for this exact committed target. Playback is resumed
        // by settle_scrub_commit_if_ready, never against the stale old frame.
        self.is_scrubbing = false;
    }

    fn selected_subtitle_is_bitmap(&self) -> bool {
        self.current_sid != "no"
            && self
                .mpv
                .get_property::<bool>("current-tracks/sub/image")
                .unwrap_or(false)
    }

    fn uses_processed_subtitle_overlay(&self) -> bool {
        if !self.sub_visibility || self.current_sid == "no" || self.selected_subtitle_is_bitmap() {
            return false;
        }
        crate::subtitle::requires_processed_overlay(
            self.subtitle_direction,
            self.subtitle_alignment,
            &self.subtitle_text_replacements,
        )
    }

    fn clear_subtitle_overlay(&self) {
        let _ = self
            .mpv_client
            .command("osd-overlay", &["7301", "none", "", "0", "720", "10"]);
    }

    /// Synchronize logical subtitle visibility with either mpv's native
    /// renderer (bitmap/unmodified subtitles) or Pealayer's processed overlay.
    pub(crate) fn sync_subtitle_rendering(&mut self) {
        // Native plain-text/bitmap fallback must not retain a previous choice
        // when switching renderer or tracks. Bitmap alignment is mpv-owned.
        let _ = self.mpv.set_property("sub-align-x", self.subtitle_alignment.mpv_value());
        let _ = self.mpv.set_property("sub-justify", self.subtitle_alignment.mpv_value());
        if !self.uses_processed_subtitle_overlay() {
            self.clear_subtitle_overlay();
            let _ = self.mpv.set_property("sub-visibility", self.sub_visibility);
            return;
        }

        // Keep decoding active so `sub-text` continues to advance, but suppress
        // the unprocessed renderer to avoid a doubled caption.
        let _ = self.mpv.set_property("sub-visibility", false);
        let replaced = crate::subtitle::apply_text_replacements(
            &self.subtitle_text,
            &self.subtitle_text_replacements,
        );
        if replaced.is_empty() {
            self.clear_subtitle_overlay();
            return;
        }
        let event = crate::subtitle::overlay_ass_event(
            &replaced,
            self.subtitle_direction,
            self.subtitle_alignment,
            self.sub_font_size,
            self.sub_position_percent,
        );
        let _ = self.mpv_client.command(
            "osd-overlay",
            &["7301", "ass-events", &event, "1280", "720", "10"],
        );
    }

    pub(crate) fn set_subtitle_visibility(&mut self, visible: bool) {
        self.sub_visibility = visible;
        self.sync_subtitle_rendering();
    }

    pub(crate) fn refresh_media_tracks(&mut self) {
        if let Some(view)=crate::peer::client().and_then(|client|client.snapshot()).and_then(|snapshot|snapshot.session.media_view) {
            self.apply_peer_media_view(view);
            return;
        }
        let mut media_tracks = Vec::new();
        let count = self
            .mpv
            .get_property::<i64>("track-list/count")
            .unwrap_or(0);
        for list_index in 0..count {
            let prefix = format!("track-list/{list_index}");
            let Ok(kind_name) = self.mpv.get_property::<String>(&format!("{prefix}/type")) else {
                continue;
            };
            let kind = match kind_name.as_str() {
                "video" => MediaTrackType::Video,
                "audio" => MediaTrackType::Audio,
                "sub" => MediaTrackType::Subtitle,
                _ => continue,
            };
            let Ok(id) = self.mpv.get_property::<i64>(&format!("{prefix}/id")) else {
                continue;
            };

            let property = |name: &str| format!("{prefix}/{name}");
            let string = |name: &str| self.mpv.get_property::<String>(&property(name)).ok();
            let integer = |name: &str| self.mpv.get_property::<i64>(&property(name)).ok();
            let number = |name: &str| {
                self.mpv
                    .get_property::<f64>(&property(name))
                    .ok()
                    .or_else(|| integer(name).map(|value| value as f64))
            };
            let boolean = |name: &str| self.mpv.get_property::<bool>(&property(name)).ok();

            let mut metadata = std::collections::BTreeMap::new();
            let metadata_count = self
                .mpv
                .get_property::<i64>(&format!("{prefix}/metadata/list/count"))
                .unwrap_or(0);
            for metadata_index in 0..metadata_count {
                let metadata_prefix = format!("{prefix}/metadata/list/{metadata_index}");
                if let (Ok(key), Ok(value)) = (
                    self.mpv
                        .get_property::<String>(&format!("{metadata_prefix}/key")),
                    self.mpv
                        .get_property::<String>(&format!("{metadata_prefix}/value")),
                ) {
                    metadata.insert(key, value);
                }
            }

            media_tracks.push(MediaTrackInfo {
                list_index,
                kind,
                id,
                source_id: integer("src-id"),
                title: string("title"),
                language: string("lang"),
                image: boolean("image"),
                album_art: boolean("albumart"),
                is_default: boolean("default"),
                forced: boolean("forced"),
                dependent: boolean("dependent"),
                visual_impaired: boolean("visual-impaired"),
                hearing_impaired: boolean("hearing-impaired"),
                hls_bitrate: integer("hls-bitrate"),
                program_id: integer("program-id"),
                codec: string("codec"),
                codec_description: string("codec-desc"),
                codec_profile: string("codec-profile"),
                external: boolean("external"),
                external_filename: string("external-filename"),
                selected: boolean("selected"),
                main_selection: integer("main-selection"),
                ffmpeg_index: integer("ff-index"),
                decoder: string("decoder"),
                decoder_description: string("decoder-desc"),
                demux_width: integer("demux-w"),
                demux_height: integer("demux-h"),
                crop_x: integer("demux-crop-x"),
                crop_y: integer("demux-crop-y"),
                crop_width: integer("demux-crop-w"),
                crop_height: integer("demux-crop-h"),
                channel_count: integer("demux-channel-count"),
                channel_layout: string("demux-channels"),
                sample_rate: integer("demux-samplerate"),
                fps: number("demux-fps"),
                bitrate: number("demux-bitrate"),
                rotation: integer("demux-rotation"),
                pixel_aspect_ratio: number("demux-par"),
                format_name: string("format-name"),
                replaygain_track_peak: number("replaygain-track-peak"),
                replaygain_track_gain: number("replaygain-track-gain"),
                replaygain_album_peak: number("replaygain-album-peak"),
                replaygain_album_gain: number("replaygain-album-gain"),
                dolby_vision_profile: integer("dolby-vision-profile"),
                dolby_vision_level: integer("dolby-vision-level"),
                metadata,
            });
        }

        self.video_tracks = media_tracks
            .iter()
            .filter(|track| track.kind == MediaTrackType::Video)
            .map(|track| VideoTrack {
                id: track.id,
                title: track.title.clone(),
                lang: track.language.clone(),
            })
            .collect();
        self.audio_tracks = media_tracks
            .iter()
            .filter(|track| track.kind == MediaTrackType::Audio)
            .map(|track| AudioTrack {
                id: track.id,
                title: track.title.clone(),
                lang: track.language.clone(),
            })
            .collect();
        self.sub_tracks = media_tracks
            .iter()
            .filter(|track| track.kind == MediaTrackType::Subtitle)
            .map(|track| SubtitleTrack {
                id: track.id,
                title: track.title.clone(),
                lang: track.language.clone(),
            })
            .collect();
        self.media_tracks = media_tracks;
        self.media_file_info = crate::media_info::capture(&self.mpv);
        if self.media_track_properties.is_some_and(|selection| {
            !self
                .media_tracks
                .iter()
                .any(|track| track.kind == selection.kind && track.id == selection.id)
        }) {
            self.media_track_properties = None;
        }
    }

    pub(crate) fn media_track(&self, selection: MediaTrackKey) -> Option<&MediaTrackInfo> {
        self.media_tracks
            .iter()
            .find(|track| track.kind == selection.kind && track.id == selection.id)
    }

    pub(crate) fn select_media_track(&mut self, selection: MediaTrackKey) {
        let id = selection.id.to_string();
        let applied = match selection.kind {
            MediaTrackType::Video => {
                self.current_vid = id.clone();
                self.mpv.set_property("vid", id)
            }
            MediaTrackType::Audio => {
                self.current_aid = id.clone();
                self.mpv.set_property("aid", id)
            }
            MediaTrackType::Subtitle => {
                self.current_sid = id.clone();
                let result = self.mpv.set_property("sid", id);
                if result.is_ok() {
                    self.set_subtitle_visibility(true);
                }
                result
            }
        };
        if applied.is_ok() {
            if selection.kind == MediaTrackType::Video {
                // Switching video streams is a video change even when the new
                // stream happens to share the previous stream's aspect value.
                self.pending_video_aspect_resize = true;
            }
            for track in &mut self.media_tracks {
                if track.kind == selection.kind {
                    track.selected = Some(track.id == selection.id);
                }
            }
        }
    }

    pub(crate) fn current_media_track_id(&self, kind: MediaTrackType) -> &str {
        match kind {
            MediaTrackType::Video => &self.current_vid,
            MediaTrackType::Audio => &self.current_aid,
            MediaTrackType::Subtitle => &self.current_sid,
        }
    }

    pub(crate) fn disable_media_track(&mut self, kind: MediaTrackType) {
        let property = match kind {
            MediaTrackType::Video => {
                self.current_vid = "no".to_string();
                "vid"
            }
            MediaTrackType::Audio => {
                self.current_aid = "no".to_string();
                "aid"
            }
            MediaTrackType::Subtitle => {
                self.current_sid = "no".to_string();
                self.sub_visibility = false;
                "sid"
            }
        };
        if self.mpv.set_property(property, "no").is_ok() {
            for track in &mut self.media_tracks {
                if track.kind == kind {
                    track.selected = Some(false);
                }
            }
        }
    }

    pub(crate) fn set_audio_muted(&mut self, muted: bool) {
        if self.mpv.set_property("mute", muted).is_ok() {
            self.is_muted = muted;
            self.set_osd(if muted {
                self.tr("Mute")
            } else {
                self.tr("Unmute")
            });
            self.save_config();
        }
    }

    pub(crate) fn toggle_audio_muted(&mut self) {
        self.set_audio_muted(!self.is_muted);
    }

    /// One picker for the menu, video surface and tray. Retain PathBuf rather
    /// than losing non-UTF-8 local names; peer clients browse the master only.
    pub(crate) fn open_media_file_dialog(&mut self, ctx: &egui::Context) {
        if crate::peer::active() {
            crate::ui::peer_browser::open(ctx, crate::ui::peer_browser::Purpose::Media, None);
        } else if let Some(path) = rfd::FileDialog::new()
            .add_filter(self.tr("Video Files"), &["mp4", "mkv", "avi", "webm", "mov", "flv"])
            .pick_file()
        {
            self.load_video_file(path);
        }
    }

    pub fn load_video_file(&mut self, path: std::path::PathBuf) {
        if crate::peer::active(){self.load_media_target(&path.to_string_lossy());return;}
        let path_str = path.to_str().unwrap_or("");
        if !path_str.is_empty() {
            self.reset_scrub_state();
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
            self.refresh_media_tracks();
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
        if crate::peer::active() && !crate::peer::mirroring() { self.load_media_target(url); return; }
        let trimmed = url.trim();
        if !trimmed.is_empty() {
            self.reset_scrub_state();
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
            &self.mpv,
            self.open_url_use_proxy,
            &self.open_url_proxy_url,
        )
    }

    pub fn load_media_target(&mut self, target: &str) {
        if let Some(client)=crate::peer::client() && !crate::peer::mirroring() {
            if let Err(error)=client.queue("/api/player/command",serde_json::json!({"command":"open","target":target})) {self.set_osd(error)}
            return;
        }
        if crate::remote_location::normalize(target).is_ok() {
            if let Some(ctx) = crate::remote_location::context() {
                self.load_remote_target(target, self.open_url_use_proxy, ctx);
            } else { self.load_url(target); }
        } else if crate::media::is_remote_media_target(target) {
            self.load_url(target);
        } else {
            self.load_video_file(std::path::PathBuf::from(target));
        }
    }

    fn remote_neighbor(&self, direction: i32, automatic: bool) -> Option<crate::remote_location::Playback> {
        crate::remote_location::step(self.current_video_path.as_ref()?.to_str()?, direction, automatic)
    }
    fn has_remote_playlist(&self) -> bool {
        self.current_video_path.as_ref().and_then(|path|path.to_str()).and_then(crate::remote_location::playback_proxy_for).is_some()
    }

    pub fn load_remote_target(&mut self, target: &str, use_proxy: bool, ctx: &egui::Context) {
        if let Some(client)=crate::peer::client() {
            if let Err(error)=client.queue("/api/peer/open",serde_json::json!({"target":target,"use_proxy":use_proxy})){self.set_osd(error)}
            return;
        }
        if crate::remote_location::normalize(target).is_ok_and(|url|crate::remote_location::playable(&url)) {
            self.play_remote_location(crate::remote_location::Playback {target:target.into(),use_proxy});
            let _ = crate::remote_location::prefetch(target,use_proxy,ctx);
        } else if let Err(error) = crate::remote_location::request(target,Some(use_proxy),true,ctx) { self.set_osd(error); }
    }

    pub fn play_remote_location(&mut self, playback: crate::remote_location::Playback) {
        if let Err(error) = crate::mpv::proxy::apply_runtime(&self.mpv, playback.use_proxy, &self.open_url_proxy_url) { self.set_osd(error.to_string()); return; }
        let _ = self.mpv.set_property("options/user-agent", crate::remote_location::USER_AGENT);
        self.load_url(&playback.target);
        // loadfile inherits MPV's pause flag. A browser Play/Next command must
        // start the selected file even when the previous file was paused or
        // kept open at EOF. Startup restoration applies its saved pause later.
        let _ = self.mpv.set_property("pause", false);
        self.is_paused = false;
        self.engine_handle.is_playing.store(true, std::sync::atomic::Ordering::Relaxed);
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
        self.reset_scrub_state();
        self.capture_current_playback_position();
        let _ = self.mpv.command("stop", &[]);
        self.current_video_path = None;
        self.last_media_target = None;
        self.playback_time = 0.0;
        self.duration = 0.0;
        self.is_seekable = false;
        self.media_metadata_loaded = false;
        self.media_file_info = crate::media_info::MediaFileInfo::default();
        self.cache_duration = None;
        self.cache_buffering_percent = None;
        self.video_aspect_ratio = 16.0 / 9.0;
        self.is_eof = false;
        self.is_paused = false;
        if let Some(ref mut mc) = self.media_controls {
            mc.update_metadata(None);
            mc.update_playback(false, false, 0.0, 0.0);
        }
        self.set_osd("Video Closed".to_string());
        self.save_config();
    }

    pub(crate) fn runtime_config_snapshot(&self) -> crate::config::AppConfig {
        // Preserve deployment-owned branding while saving mutable player
        // preferences through one typed configuration contract.
        // Start with the live contract, including unsaved appearance previews.
        // Reloading disk here silently reverted the accent during autosaves or
        // an unrelated API patch, even while the native UI still used it.
        let mut cfg = crate::platform::interop::get_live_config();
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
        cfg.color_palette = self.color_palette;
        cfg.hardware_endpoint =
            (!self.serial_port.trim().is_empty()).then(|| self.serial_port.clone());
        cfg.auto_connect_hardware = self.auto_connect_hardware;
        cfg.pause_on_hardware_disconnect = self.pause_on_hardware_disconnect;
        cfg.click_player_to_toggle = self.click_player_to_toggle;
        cfg.playback_speed = self.configured_playback_speed;
        cfg.temporary_fast_forward_speed = self.temporary_fast_forward_speed;
        cfg.subtitle_font_size = self.sub_font_size;
        cfg.numeric_input_steps = self.numeric_input_steps.clone();
        cfg.subtitle_delay_seconds = self.sub_delay;
        cfg.subtitle_position_percent = self.sub_position_percent;
        cfg.subtitle_direction = self.subtitle_direction;
        cfg.subtitle_alignment = self.subtitle_alignment;
        cfg.subtitle_text_replacements = self.subtitle_text_replacements.clone();
        cfg.audio_delay_seconds = self.audio_delay;
        cfg.show_subseconds = self.show_subseconds;
        cfg.human_readable_time_units = self.human_readable_time_units;
        cfg.seekbar_hover_thumbnails = self.seekbar_hover_thumbnails;
        cfg.nle_seekbar_hover_thumbnails = self.nle_seekbar_hover_thumbnails;
        cfg.consistent_video_aspect_ratio = self.consistent_video_aspect_ratio;
        cfg.always_on_top = self.always_on_top;
        cfg.quick_seek_seconds = self.quick_seek_seconds;
        cfg.frame_step_count = self.frame_step_count;
        cfg.wheel_seek_seconds = self.wheel_seek_seconds;
        cfg.osd_position = self.osd_position;
        cfg.osd_timeout_seconds = self.osd_timeout_seconds;
        cfg.paused_drag_action = self.paused_drag_action;
        cfg.playing_drag_action = self.playing_drag_action;
        cfg.middle_click_action = self.middle_click_action;
        cfg.middle_hold_action = self.middle_hold_action;
        cfg.right_click_action = self.right_click_action;
        cfg.right_hold_action = self.right_hold_action;
        cfg.fullscreen_video_background = self.fullscreen_video_background;
        cfg.motion_control_mode = self.motion_control_mode;
        cfg.compact_hardware_controls = self.compact_hardware_controls;
        cfg.compact_timeline_tracks = self.compact_timeline_tracks;
        cfg.timeline_header_wheel_vertical_scroll = self.timeline_header_wheel_vertical_scroll;
        cfg.timeline_plain_wheel_action = self.timeline_plain_wheel_action;
        cfg.timeline_ctrl_wheel_action = self.timeline_ctrl_wheel_action;
        cfg.timeline_shift_wheel_action = self.timeline_shift_wheel_action;
        cfg.timeline_alt_wheel_action = self.timeline_alt_wheel_action;
        cfg.timeline_middle_button_pan = self.timeline_middle_button_pan;
        cfg.timeline_middle_axis_lock_modifiers = self.timeline_middle_axis_lock_modifiers;
        cfg.timeline_animated_navigation = self.timeline_animated_navigation;
        cfg.timeline_navigation_transition_ms = self.timeline_navigation_transition_ms;
        cfg.non_user_control_visibility = self.non_user_control_visibility;
        cfg.prefix_relay_identifiers = self.prefix_relay_identifiers;
        cfg.live_pwm_updates = self.live_pwm_updates;
        cfg.hardware_actions_on_press = self.hardware_actions_on_press;
        cfg.keyboard_shortcuts_enabled = self.keyboard_shortcuts_enabled;
        cfg.media_keys_enabled = self.media_keys_enabled;
        cfg.application_shortcuts = self.application_shortcuts.clone();
        cfg.global_hardware_hotkeys_enabled = self.global_hardware_hotkeys_enabled;
        cfg.hardware_key_bindings = self.hardware_key_bindings.clone();
        cfg.show_estop_control = self.show_estop_control;
        cfg.confirm_estop_release = self.confirm_estop_release;
        cfg.single_instance = self.single_instance;
        cfg.window_magnetic_snap = self.window_magnetic_snap;
        cfg.window_magnetic_snap_distance = self.window_magnetic_snap_distance;
        cfg.windows_mica_backdrop = self.windows_mica_backdrop;
        cfg.windows_dwm_theming = self.windows_dwm_theming;
        cfg.windows_video_taskbar_thumbnail = self.windows_video_taskbar_thumbnail;
        cfg.windows_thumbnail_toolbar = self.windows_thumbnail_toolbar;
        cfg.windows_jump_list_quick_actions = self.windows_jump_list_quick_actions;
        cfg.opengl_vsync = self.opengl_vsync;
        cfg.live_video_during_window_move = self.live_video_during_window_move;
        cfg.compositor_paced_window_move = self.compositor_paced_window_move;
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
        cfg.effect_working_draft = (!self.effect_library_draft.name.trim().is_empty()
            || !self.effect_library_draft.steps.is_empty()
            || !self.effect_library_draft.program_json.trim().is_empty())
        .then(|| self.effect_library_draft.clone());
        let cue_timeline = self.timeline.controller_cue_session();
        cfg.effect_cue_session = self
            .current_video_path
            .as_ref()
            .filter(|_| cue_timeline.has_controller_cues())
            .map(|target| crate::config::EffectCueSession {
                media_target: crate::media::playback_history_key(&target.to_string_lossy()),
                timeline: cue_timeline,
            });
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
        self.color_palette = config.color_palette;
        self.serial_port = endpoint.clone();
        self.auto_connect_hardware = config.auto_connect_hardware;
        self.pause_on_hardware_disconnect = config.pause_on_hardware_disconnect;
        self.click_player_to_toggle = config.click_player_to_toggle;
        self.configured_playback_speed = config.playback_speed;
        self.temporary_fast_forward_speed = config.temporary_fast_forward_speed;
        if self.video_surface_gesture.is_none() {
            let _ = self
                .mpv
                .set_property("speed", self.configured_playback_speed);
            self.playback_rate = self.configured_playback_speed;
        }
        self.sub_font_size = config.subtitle_font_size;
        self.numeric_input_steps = config.numeric_input_steps.clone();
        self.sub_delay = config.subtitle_delay_seconds;
        self.sub_position_percent = config.subtitle_position_percent;
        self.audio_delay = config.audio_delay_seconds;
        self.subtitle_direction = config.subtitle_direction;
        self.subtitle_alignment = config.subtitle_alignment;
        self.subtitle_text_replacements = config.subtitle_text_replacements.clone();
        self.show_subseconds = config.show_subseconds;
        self.human_readable_time_units = config.human_readable_time_units;
        self.seekbar_hover_thumbnails = config.seekbar_hover_thumbnails;
        self.nle_seekbar_hover_thumbnails = config.nle_seekbar_hover_thumbnails;
        let aspect_lock_enabled =
            !self.consistent_video_aspect_ratio && config.consistent_video_aspect_ratio;
        self.consistent_video_aspect_ratio = config.consistent_video_aspect_ratio;
        self.always_on_top = config.always_on_top;
        self.sync_window_level(ctx);
        if aspect_lock_enabled && self.current_video_path.is_some() {
            self.pending_video_aspect_resize = true;
        } else if !self.consistent_video_aspect_ratio {
            self.pending_video_aspect_resize = false;
        }
        self.quick_seek_seconds = config.quick_seek_seconds;
        self.frame_step_count = config.frame_step_count;
        self.wheel_seek_seconds = config.wheel_seek_seconds;
        self.osd_position = config.osd_position;
        self.osd_timeout_seconds = config.osd_timeout_seconds;
        self.paused_drag_action = config.paused_drag_action;
        self.playing_drag_action = config.playing_drag_action;
        self.middle_click_action = config.middle_click_action;
        self.middle_hold_action = config.middle_hold_action;
        self.right_click_action = config.right_click_action;
        self.right_hold_action = config.right_hold_action;
        self.fullscreen_video_background = config.fullscreen_video_background;
        self.motion_control_mode = config.motion_control_mode;
        self.compact_hardware_controls = config.compact_hardware_controls;
        self.compact_timeline_tracks = config.compact_timeline_tracks;
        self.timeline_header_wheel_vertical_scroll = config.timeline_header_wheel_vertical_scroll;
        self.timeline_plain_wheel_action = config.timeline_plain_wheel_action;
        self.timeline_ctrl_wheel_action = config.timeline_ctrl_wheel_action;
        self.timeline_shift_wheel_action = config.timeline_shift_wheel_action;
        self.timeline_alt_wheel_action = config.timeline_alt_wheel_action;
        self.timeline_middle_button_pan = config.timeline_middle_button_pan;
        self.timeline_middle_axis_lock_modifiers = config.timeline_middle_axis_lock_modifiers;
        self.timeline_animated_navigation = config.timeline_animated_navigation;
        self.timeline_navigation_transition_ms = config.timeline_navigation_transition_ms;
        self.non_user_control_visibility = config.non_user_control_visibility;
        self.prefix_relay_identifiers = config.prefix_relay_identifiers;
        self.live_pwm_updates = config.live_pwm_updates;
        self.hardware_actions_on_press = config.hardware_actions_on_press;
        if self.hardware_key_bindings != config.hardware_key_bindings
            || self.keyboard_shortcuts_enabled != config.keyboard_shortcuts_enabled
            || self.global_hardware_hotkeys_enabled != config.global_hardware_hotkeys_enabled
        {
            // A file-watcher/API update may unregister or alter a held global
            // shortcut. Release against the old contract before replacing it.
            self.release_active_hardware_bindings();
        }
        self.keyboard_shortcuts_enabled = config.keyboard_shortcuts_enabled;
        self.media_keys_enabled = config.media_keys_enabled;
        if !self.media_keys_enabled {
            self.media_controls = None;
        }
        self.application_shortcuts = config.application_shortcuts.clone();
        self.global_hardware_hotkeys_enabled = config.global_hardware_hotkeys_enabled;
        self.hardware_key_bindings = config.hardware_key_bindings.clone();
        self.show_estop_control = config.show_estop_control;
        self.confirm_estop_release = config.confirm_estop_release;
        self.single_instance = config.single_instance;
        self.window_magnetic_snap = config.window_magnetic_snap;
        self.window_magnetic_snap_distance = config.window_magnetic_snap_distance;
        self.windows_mica_backdrop = config.windows_mica_backdrop;
        self.windows_dwm_theming = config.windows_dwm_theming;
        self.windows_video_taskbar_thumbnail = config.windows_video_taskbar_thumbnail;
        self.windows_thumbnail_toolbar = config.windows_thumbnail_toolbar;
        let jump_list_changed =
            self.windows_jump_list_quick_actions != config.windows_jump_list_quick_actions;
        self.windows_jump_list_quick_actions = config.windows_jump_list_quick_actions;
        self.opengl_vsync = config.opengl_vsync;
        self.live_video_during_window_move = config.live_video_during_window_move;
        self.compositor_paced_window_move = config.compositor_paced_window_move;
        self.native_dialog_windows = config.native_dialog_windows;
        self.auto_reload_config = config.auto_reload_config;
        self.status_bar = config.status_bar;
        self.workspace_profiles = config.workspace_profiles.clone();
        self.active_workspace_profile = config.active_workspace_profile.clone();
        self.effect_library_draft = config.effect_working_draft.clone().unwrap_or_default();
        crate::platform::windows::configure_window_composition(
            self.windows_dwm_theming,
            self.windows_mica_backdrop,
        );
        crate::platform::windows::configure_window_magnetic_snap(
            self.window_magnetic_snap,
            self.window_magnetic_snap_distance as i32,
        );
        crate::platform::windows::configure_live_video_during_window_move(
            self.live_video_during_window_move,
        );
        crate::platform::windows::configure_compositor_paced_window_move(
            self.compositor_paced_window_move,
        );
        if jump_list_changed && !crate::peer::active() {
            crate::platform::windows::sync_windows_jump_list_with_options(
                &self.recent_media,
                self.windows_jump_list_quick_actions,
            );
        }
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
        let _ = self.mpv.set_property("sub-font-size", self.sub_font_size);
        let _ = self.mpv.set_property("sub-delay", self.sub_delay);
        let _ = self.mpv.set_property("sub-pos", self.sub_position_percent);
        let _ = self.mpv.set_property("audio-delay", self.audio_delay);
        self.sync_subtitle_rendering();
        crate::mpv::proxy::apply_runtime(
            &self.mpv,
            self.open_url_use_proxy,
            &self.open_url_proxy_url,
        )?;
        if !crate::peer::active(){crate::platform::windows::sync_windows_jump_list_with_options(
            &self.recent_media,
            self.windows_jump_list_quick_actions,
        );}
        self.prune_recent_remote_thumbnail_cache();
        crate::ui::i18n::configure_ui_fonts(
            ctx,
            self.language == crate::config::AppLanguage::Persian,
        );
        crate::ui::configure_native_appearance(ctx, &config);

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
        if let Some(client)=crate::peer::client() && !crate::peer::mirroring(){
            let old=serde_json::to_value(client.config()).map_err(|error|error.to_string())?;
            let mut values=serde_json::to_value(&config).map_err(|error|error.to_string())?;
            if let Some(values)=values.as_object_mut(){values.retain(|key,value|!matches!(key.as_str(),"window_geometry"|"egui_memory"|"workspace_session"|"workspace_dock_layout"|"last_media_target"|"last_media_paused"|"recent_media"|"playback_positions"|"hardware_endpoint") && old.get(key)!=Some(value));}
            let expected=values.as_object().ok_or("Invalid configuration")?.keys().map(|key|(key.clone(),old.get(key).cloned().unwrap_or_default())).collect::<serde_json::Map<String,serde_json::Value>>();
            client.queue("/api/peer/config",serde_json::json!({"operation":"preview","expected":expected,"values":values}))?;
        }
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
        if self.preference_preview_original.is_none() {return Ok(())}
        if let Some(client)=crate::peer::client(){client.post("/api/peer/config",&serde_json::json!({"operation":"discard"}))?;}
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

    fn apply_peer_request(&mut self,ctx:&egui::Context,path:&str,value:serde_json::Value)->Result<serde_json::Value,String>{
        if crate::peer::active(){return Err("Only the authoritative instance can apply session changes".into())}
        match path {
            "/api/peer/open"=>{
                let target=value.get("target").and_then(serde_json::Value::as_str).ok_or("Media target is required")?;
                if target.is_empty() || target.len()>32768{return Err("Invalid media target".into())}
                crate::remote_location::normalize(target)?;
                let use_proxy=value.get("use_proxy").and_then(serde_json::Value::as_bool).unwrap_or(self.open_url_use_proxy);
                self.load_remote_target(target,use_proxy,ctx);
            },
            "/api/peer/config"=>{
                let operation=value.get("operation").and_then(serde_json::Value::as_str).unwrap_or("save");
                let owner_id=crate::peer::preference_owner_id();
                let owner=ctx.data_mut(|data|data.get_temp::<String>(owner_id));
                let consumer=value.get("_consumer").and_then(serde_json::Value::as_str).unwrap_or("");
                // A failed/expired preview has nothing to discard. It must
                // still be possible to close the consumer's editor safely.
                if operation=="discard" && owner.is_none(){return Ok(serde_json::json!({"applied":false,"preview":false}))}
                if self.show_preferences_dialog {
                    return Err("Server Preferences is open; finish that edit before changing configuration from a peer".into());
                }
                if owner.as_deref().is_some_and(|owner|owner!=consumer) {
                    return Err("Preferences is being previewed by another peer; finish that edit first".into());
                }
                match operation {
                    "save"=>{
                        crate::peer::validate_config_expectations(&crate::platform::interop::get_live_config(), value.get("expected"))?;
                        let config=crate::platform::interop::get_live_config().apply_patch(value.get("values").ok_or("Configuration values are required")?)?;
                        config.save()?;
                        if value.get("values").is_some_and(|values|values.get("workspace_session").is_some()){
                            let mut profile=config.workspace_session.clone();profile.window_geometry=None;profile.egui_memory=None;self.apply_workspace_profile(ctx,&profile)?;
                        }
                        self.commit_preference_preview(ctx,config)?;
                        ctx.data_mut(|data|data.remove::<String>(owner_id));
                    },
                    "preview"=>{
                        if consumer.is_empty() || !crate::peer::consumer_present(consumer){return Err("A live Pealayer consumer identity is required for preference previews".into())}
                        crate::peer::validate_config_expectations(&crate::platform::interop::get_live_config(), value.get("expected"))?;
                        let config=crate::platform::interop::get_live_config().apply_patch(value.get("values").ok_or("Configuration values are required")?)?;
                        self.preview_runtime_config(ctx,config)?;
                        ctx.data_mut(|data|data.insert_temp(owner_id,consumer.to_string()));
                    },
                    "discard"=>{
                        if owner.is_none(){return Err("This peer does not own a preference preview".into())}
                        self.cancel_preference_preview(ctx)?;
                        ctx.data_mut(|data|data.remove::<String>(owner_id));
                    },
                    _=>return Err("Unknown configuration operation".into()),
                }
            },
            "/api/peer/timeline"=>{
                let expected:Option<crate::peer::TimelineState>=serde_json::from_value(value.get("expected").cloned().unwrap_or_default()).map_err(|error|error.to_string())?;
                let current=crate::peer::TimelineState{timeline:self.timeline.clone(),muted:self.track_muted.clone(),soloed:self.track_soloed.clone()};
                if expected.as_ref()!=Some(&current){return Err("Timeline changed on another peer; refresh before editing again".into())}
                let state:crate::peer::TimelineState=serde_json::from_value(value.get("state").cloned().ok_or("Timeline state is required")?).map_err(|error|error.to_string())?;
                if state.timeline.instances.len()>10000 || state.timeline.templates.len()>10000 || state.timeline.keyframes.len()>10000{return Err("Timeline exceeds session limits".into())}
                self.undo_stack.push(self.snapshot_timeline());
                self.timeline=state.timeline;self.track_muted=state.muted;self.track_soloed=state.soloed;
                self.sync_timeline_engine();self.persist_timeline_track_preferences();self.save_config();
            },
            "/api/peer/files"=>{
                let path=std::path::PathBuf::from(value.get("path").and_then(serde_json::Value::as_str).ok_or("Server path is required")?);
                if !path.extension().is_some_and(|extension|extension.eq_ignore_ascii_case("json")) {
                    return Err("Configuration and timeline files must use the .json extension".into());
                }
                match value.get("operation").and_then(serde_json::Value::as_str){
                    Some("export_config")=>{let config:crate::config::AppConfig=serde_json::from_value(value.get("config").cloned().ok_or("Config is required")?).map_err(|error|error.to_string())?;config.validate()?;config.save_to_path(&path)?;},
                    Some("open_timeline")=>{self.timeline=crate::four_d::models::Timeline::load_from_file(&path).map_err(|error|error.to_string())?;self.sync_timeline_engine();self.save_config();},
                    Some("save_timeline")=>self.timeline.save_to_file(&path).map_err(|error|error.to_string())?,
                    _=>return Err("Unsupported server file operation".into()),
                }
            },
            _=>return Err("Unsupported session operation".into()),
        }
        Ok(serde_json::json!({"applied":true}))
    }

    fn poll_peer_session(&mut self,ctx:&egui::Context) {
        let Some(client)=crate::peer::client() else{return};
        client.register_context(ctx);
        let Some(snapshot)=client.snapshot() else{return};
        let id=egui::Id::new("pealayer_remote_authority_ui");
        let mut state=ctx.data_mut(|data|data.get_temp::<crate::peer::PeerUiState>(id).unwrap_or_default());
        if state.config.as_ref()!=Some(&snapshot.session.config) {
            let config=snapshot.session.config.clone();
            let workspace_changed=state.config.as_ref().is_none_or(|old|old.active_workspace_profile!=config.active_workspace_profile || old.workspace_session.nle!=config.workspace_session.nle || old.workspace_dock_layout!=config.workspace_dock_layout);
            if let Err(error)=crate::peer::mirror(||self.apply_runtime_config(ctx,config.clone())){self.config_status=error;}
            if workspace_changed{
                let mut profile=config.workspace_session.clone();profile.window_geometry=None;profile.egui_memory=None;
                if let Err(error)=crate::peer::mirror(||self.apply_workspace_profile(ctx,&profile)){self.config_status=error;}
            }
            state.config=Some(config);
        }
        if state.timeline!=snapshot.session.timeline {
            if let Some(timeline)=&snapshot.session.timeline {
                self.timeline=timeline.timeline.clone();self.track_muted=timeline.muted.clone();self.track_soloed=timeline.soloed.clone();
            }
            state.timeline=snapshot.session.timeline.clone();
        }
        let fresh=snapshot.received.elapsed()<std::time::Duration::from_secs(2);
        self.is_connected=fresh;
        self.serial_port=client.origin.as_str().replacen("http://","pealayer://",1).trim_end_matches('/').into();
        if !fresh {
            let _=self.mpv.0.set_property("pause",true);
            self.connection_notice=Some("Remote Pealayer disconnected; controls are not redirected to local hardware".into());
            return;
        }
        self.connection_notice=client.error.lock().ok().and_then(|value|value.clone()).or_else(||client.command_error.lock().ok().and_then(|value|value.clone()));
        if state.loaded_media!=snapshot.session.media {
            let _=self.mpv.0.command("stop",&[]);
            state.loaded_media=snapshot.session.media.clone();
            state.media_error=None;
            state.attached_external.clear();
            if let Some(target)=&snapshot.session.media {
                match client.media_url(target).and_then(|url|{
                    self.mpv.0.set_property("user-agent",crate::peer::USER_AGENT).map_err(|error|error.to_string())?;
                    if url.starts_with(client.origin.as_str()){let _=self.mpv.0.set_property("http-proxy","");}
                    self.mpv.0.command("loadfile",&[&url,"replace","-1",&format!("start={},pause=yes",snapshot.session.position)]).map_err(|error|error.to_string())
                }) {Ok(())=>{},Err(error)=>state.media_error=Some(format!("Remote video preview unavailable: {error}"))}
            }
        }
        if let Some(error)=crate::peer::take_preview_error(){state.media_error=Some(error);}
        self.current_video_path=snapshot.session.media.clone().map(std::path::PathBuf::from);
        self.volume=snapshot.session.status.get("volume").and_then(serde_json::Value::as_f64).unwrap_or(self.volume);
        self.is_muted=snapshot.session.status.get("muted").and_then(serde_json::Value::as_bool).unwrap_or(self.is_muted);
        self.is_paused=snapshot.session.paused;
        self.playback_rate=snapshot.session.speed;
        self.duration=snapshot.session.status.get("duration").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
        self.is_seekable=snapshot.session.status.get("seekable").and_then(serde_json::Value::as_bool).unwrap_or(false);
        if let Some(view)=snapshot.session.media_view {self.apply_peer_media_view(view);}
        let expected=snapshot.session.position+if snapshot.session.paused {0.0}else{(snapshot.received.elapsed().as_secs_f64()+snapshot.round_trip.as_secs_f64()/2.0)*snapshot.session.speed};
        self.playback_time=expected;
        if state.loaded_media.is_some() && state.media_error.is_none() {
            let actual=self.mpv.0.get_property::<f64>("time-pos").ok();
            if actual.is_some() {
                for (name,value) in [("volume",self.volume),("sub-delay",self.sub_delay),("audio-delay",self.audio_delay)] {
                    if self.mpv.0.get_property::<f64>(name).ok()!=Some(value){let _=self.mpv.0.set_property(name,value);}
                }
                if self.mpv.0.get_property::<bool>("mute").ok()!=Some(self.is_muted){let _=self.mpv.0.set_property("mute",self.is_muted);}
                self.synchronize_peer_tracks(&mut state.attached_external);
            }
            if actual.is_some_and(|actual|(actual-expected).abs()>0.08)
                && state.last_correction.is_none_or(|last|last.elapsed()>=std::time::Duration::from_millis(500)){
                let _=self.mpv.0.command("seek",&[&expected.to_string(),"absolute+exact"]);state.last_correction=Some(std::time::Instant::now());
            }
            if self.mpv.0.get_property::<f64>("speed").ok()!=Some(snapshot.session.speed){let _=self.mpv.0.set_property("speed",snapshot.session.speed);}
            if self.mpv.0.get_property::<bool>("pause").ok()!=Some(snapshot.session.paused){let _=self.mpv.0.set_property("pause",snapshot.session.paused);}
        }
        if let Some(error)=&state.media_error{self.connection_notice=Some(error.clone());}
        ctx.data_mut(|data|data.insert_temp(id,state));
    }

    fn synchronize_peer_tracks(&self,attached:&mut std::collections::BTreeSet<String>) {
        let Some(client)=crate::peer::client() else{return};
        for (kind,property,selected,command) in [(MediaTrackType::Video,"vid",&self.current_vid,"video-add"),(MediaTrackType::Audio,"aid",&self.current_aid,"audio-add"),(MediaTrackType::Subtitle,"sid",&self.current_sid,"sub-add")] {
            let mut desired=selected.clone();
            if let Some(track)=self.media_tracks.iter().find(|track|track.kind==kind && track.id.to_string()==*selected)
                && track.external==Some(true)
                && let Some(source)=track.external_filename.as_deref()
                && let Ok(url)=client.media_url(source)
            {
                let key=format!("{property}:{url}");
                if attached.insert(key){let _=self.mpv.0.command(command,&[&url,"auto"]);}
                let count=self.mpv.0.get_property::<i64>("track-list/count").unwrap_or(0).clamp(0,1000);
                let found=(0..count).find_map(|index|{
                    let prefix=format!("track-list/{index}");
                    if self.mpv.0.get_property::<String>(&format!("{prefix}/type")).ok().as_deref()!=Some(kind.mpv_name()) || self.mpv.0.get_property::<String>(&format!("{prefix}/external-filename")).ok().as_deref()!=Some(url.as_str()){return None}
                    self.mpv.0.get_property::<i64>(&format!("{prefix}/id")).ok().map(|id|id.to_string())
                });
                let Some(found)=found else{continue};
                desired=found;
            }
            if self.mpv.0.get_property::<String>(property).ok()!=Some(desired.clone()){let _=self.mpv.0.set_property(property,desired);}
        }
    }

    fn apply_peer_media_view(&mut self,view:crate::peer::MediaView) {
        self.video_tracks=view.tracks.iter().filter(|track|track.kind==MediaTrackType::Video).map(|track|VideoTrack{id:track.id,title:track.title.clone(),lang:track.language.clone()}).collect();
        self.audio_tracks=view.tracks.iter().filter(|track|track.kind==MediaTrackType::Audio).map(|track|AudioTrack{id:track.id,title:track.title.clone(),lang:track.language.clone()}).collect();
        self.sub_tracks=view.tracks.iter().filter(|track|track.kind==MediaTrackType::Subtitle).map(|track|SubtitleTrack{id:track.id,title:track.title.clone(),lang:track.language.clone()}).collect();
        self.media_tracks=view.tracks;self.media_file_info=view.file;
        self.current_vid=view.vid;self.current_aid=view.aid;self.current_sid=view.sid;
    }

    fn poll_external_config(&mut self, ctx: &egui::Context) {
        if crate::peer::active(){ self.poll_peer_session(ctx);return; }
        if let Some(owner)=ctx.data_mut(|data|data.get_temp::<String>(crate::peer::preference_owner_id())) {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
            if !crate::peer::consumer_present(&owner) {
                if let Err(error)=self.cancel_preference_preview(ctx){self.config_status=error;}
                ctx.data_mut(|data|data.remove::<String>(crate::peer::preference_owner_id()));
            }
        }
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
        if ctx.data_mut(|data|data.get_temp::<String>(crate::peer::preference_owner_id())).is_some() {
            return Err("Preferences is being previewed by a peer; finish that edit first".into());
        }
        let updated = self.runtime_config_snapshot().apply_patch(values)?;
        updated.save()?;
        self.apply_runtime_config(ctx, updated.clone())?;
        if [
            "theme",
            "color_palette",
            "accent_color",
            "custom_accent_color",
        ]
        .iter()
        .any(|key| values.get(key).is_some())
        {
            if let Some(draft) = self.preferences_draft.as_mut() {
                draft.sync_external_appearance(&updated);
            }
            // Discarding an older native draft must not undo a newer committed
            // appearance change from another client.
            if let Some(original) = self.preference_preview_original.as_mut() {
                original.copy_appearance_from(&updated);
            }
        }
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
        crate::ui::sync_native_window_appearance(ctx, self.color_palette);
        self.save_config();
    }

    pub fn add_recent_media(&mut self, path: std::path::PathBuf) {
        self.recent_media.retain(|p| p != &path);
        self.recent_media.insert(0, path);
        if self.recent_media.len() > 10 {
            self.recent_media.truncate(10);
        }
        crate::platform::windows::sync_windows_jump_list_with_options(
            &self.recent_media,
            self.windows_jump_list_quick_actions,
        );
        self.prune_recent_remote_thumbnail_cache();
        self.save_config();
    }

    pub fn clear_recent_media(&mut self) {
        self.recent_media.clear();
        crate::platform::windows::sync_windows_jump_list_with_options(
            &[],
            self.windows_jump_list_quick_actions,
        );
        self.prune_recent_remote_thumbnail_cache();
        self.save_config();
    }

    pub fn remove_recent_media(&mut self, target: &str) {
        self.recent_media
            .retain(|path| path.to_string_lossy() != target);
        crate::platform::windows::sync_windows_jump_list_with_options(
            &self.recent_media,
            self.windows_jump_list_quick_actions,
        );
        self.prune_recent_remote_thumbnail_cache();
        self.save_config();
    }

    pub fn clear_recent_remote_media(&mut self) {
        self.recent_media
            .retain(|path| !crate::media::is_remote_media_target(&path.to_string_lossy()));
        crate::platform::windows::sync_windows_jump_list_with_options(
            &self.recent_media,
            self.windows_jump_list_quick_actions,
        );
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
        self.osd_display_options = None;
        if msg.trim().is_empty() {
            self.osd_message = None;
        } else {
            self.osd_message = Some((msg, std::time::Instant::now()));
        }
    }

    pub fn set_osd_with_options(
        &mut self,
        msg: String,
        options: crate::platform::interop::OsdOptions,
    ) {
        if msg.trim().is_empty() {
            self.clear_osd();
        } else {
            self.osd_message = Some((msg, std::time::Instant::now()));
            self.osd_display_options = Some(options);
        }
    }

    pub fn clear_osd(&mut self) {
        self.osd_message = None;
        self.osd_display_options = None;
    }

    pub(crate) fn set_playback_speed(&mut self, speed: f64, persist: bool) {
        if !speed.is_finite() {
            return;
        }
        let speed = speed.clamp(0.25, 4.0);
        self.configured_playback_speed = speed;
        if self.video_surface_gesture.is_none() {
            let _ = self.mpv.set_property("speed", speed);
            self.playback_rate = speed;
        }
        self.set_osd(format!("{}: {speed:.2}×", self.tr("Playback speed")));
        if persist {
            self.save_config();
        }
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
        if crate::peer::active(){self.sync_timeline_engine();return;}
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

    /// Insert an exact, timeline-wide magnetic guide and make the edit durable.
    ///
    /// All UI entry points use this method so toolbar, ruler-menu, and keyboard
    /// insertion share undo, selection, persistence, and user feedback.
    pub(crate) fn insert_timeline_keyframe(&mut self, time_ms: u64) -> (uuid::Uuid, bool) {
        if let Some(existing) = self
            .timeline
            .keyframes
            .iter()
            .find(|keyframe| keyframe.time_ms == time_ms)
            .map(|keyframe| keyframe.id)
        {
            self.selected_timeline_keyframe = Some(existing);
            self.selected_instance_ids.clear();
            self.selected_keyframes.clear();
            self.set_osd(format!(
                "{} {}",
                self.tr("Keyframe already exists at"),
                crate::duration::format_time_value_ms(time_ms)
            ));
            return (existing, false);
        }

        self.undo_stack.push(self.snapshot_timeline());
        let id = self.timeline.add_keyframe(time_ms);
        self.selected_timeline_keyframe = Some(id);
        self.selected_instance_ids.clear();
        self.selected_keyframes.clear();
        self.set_osd(format!(
            "{} {}",
            self.tr("Keyframe inserted at"),
            crate::duration::format_time_value_ms(time_ms)
        ));
        // Persist last so an I/O failure remains the visible status message
        // instead of being overwritten by a success notification.
        self.persist_timeline_track_preferences();
        (id, true)
    }

    /// Persist and publish a timeline edit that has already mutated the model.
    pub(crate) fn commit_timeline_edit(&mut self) {
        self.persist_timeline_track_preferences();
        self.sync_timeline_engine();
    }

    /// Deletes every selected cue as one undoable timeline edit.
    ///
    /// Selection may outlive the timeline's synthetic keyboard focus (for
    /// example after clicking another non-text control), so keyboard and menu
    /// entry points share this model-level operation instead of duplicating a
    /// focus-dependent retain call.
    pub(crate) fn delete_selected_timeline_cues(&mut self) -> usize {
        let selected = self.selected_instance_ids.clone();
        let removed = self
            .timeline
            .instances
            .iter()
            .filter(|instance| selected.contains(&instance.id))
            .count();
        if removed == 0 {
            self.selected_instance_ids.clear();
            return 0;
        }

        self.undo_stack.push(self.snapshot_timeline());
        if self
            .active_drag
            .as_ref()
            .is_some_and(|drag| selected.contains(&drag.instance_id))
        {
            self.active_drag = None;
        }
        self.timeline
            .instances
            .retain(|instance| !selected.contains(&instance.id));
        self.selected_instance_ids.clear();
        self.commit_timeline_edit();
        removed
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
        let state=crate::peer::TimelineState{timeline:self.timeline.clone(),muted:self.track_muted.clone(),soloed:self.track_soloed.clone()};
        if let Some(client)=crate::peer::client(){
            if !crate::peer::mirroring() && let Some(snapshot)=client.snapshot() && snapshot.session.timeline.as_ref()!=Some(&state){
                let _=client.queue("/api/peer/timeline",serde_json::json!({"expected":snapshot.session.timeline,"state":state}));
            }
            return;
        }
        crate::peer::publish_timeline(state);
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
        let direct_pwm = crate::four_d::engine::compile_direct_pwm_cues(&self.timeline);
        let analog=self.linked_analog_tracks();
        let payload=crate::four_d::media_timeline::compile_plan(&self.timeline,&relays,&analog);
        if let Ok(mut plan)=self.engine_handle.prepared_timeline.lock(){plan.replace(payload);}
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
        let _ = self.engine_handle.sender.send(
            crate::four_d::engine::EngineMessage::UpdateDirectPwmCues(direct_pwm),
        );
    }

    /// Adds a directly-authored state/value interval to an advertised channel.
    /// These placements are intentionally distinct from recorded effects: the
    /// user owns their duration and PCController still owns the physical key.
    pub fn add_direct_control_cue(
        &mut self,
        control_key: &str,
        value_basis_points: u16,
        start_time_ms: u64,
        duration_ms: u64,
    ) -> Result<uuid::Uuid, String> {
        let control = self
            .engine_handle
            .hardware_capabilities
            .lock()
            .ok()
            .and_then(|catalog| catalog.as_ref()?.controls.iter().find(|item| item.key == control_key).cloned())
            .ok_or_else(|| format!("Channel '{control_key}' is not advertised by PCController"))?;
        let relay_id = control_key
            .strip_prefix("relay.")
            .and_then(|value| value.parse::<u8>().ok())
            .filter(|id| *id != 0);
        let is_pwm = matches!(control.kind.as_str(), "pwm" | "mosfet")
            || control_key.starts_with("pwm.");
        if relay_id.is_none() && !is_pwm {
            return Err(format!("Channel '{}' does not support direct value cues", control.name));
        }
        let value = value_basis_points.min(10_000);
        let value_label = if relay_id.is_some() {
            if value >= 5_000 { "On".to_string() } else { "Off".to_string() }
        } else {
            format!("{:.2}%", f32::from(value) / 100.0)
        };
        let template = crate::four_d::models::Effect::direct_control(
            format!("{} · {}", control.name, value_label),
            control.icon.clone(),
            duration_ms,
            control.key.clone(),
            value,
            relay_id,
        );
        let template_id = template.id;
        let instance = crate::four_d::models::EffectInstance::new(template_id, start_time_ms);
        let instance_id = instance.id;
        self.undo_stack.push(self.snapshot_timeline());
        self.timeline.templates.push(template);
        self.timeline.instances.push(instance);
        self.timeline
            .track_states
            .entry(crate::four_d::models::hardware_timeline_track_key(control_key))
            .or_default();
        self.selected_instance_ids.clear();
        self.selected_instance_ids.insert(instance_id);
        self.selected_timeline_track = Some(crate::four_d::models::hardware_timeline_track_key(control_key));
        self.sync_timeline_engine();
        Ok(instance_id)
    }

    pub fn update_direct_control_cue_value(
        &mut self,
        instance_id: uuid::Uuid,
        value_basis_points: u16,
    ) -> Result<(), String> {
        let effect_id = self
            .timeline
            .instances
            .iter()
            .find(|instance| instance.id == instance_id)
            .map(|instance| instance.effect_id)
            .ok_or_else(|| "Cue is no longer available".to_string())?;
        self.undo_stack.push(self.snapshot_timeline());
        let isolated = self.isolate_template_for_instance(instance_id).unwrap_or(effect_id);
        let effect = self
            .timeline
            .templates
            .iter_mut()
            .find(|template| template.id == isolated)
            .ok_or_else(|| "Cue effect is no longer available".to_string())?;
        let direct = effect
            .direct_control
            .as_mut()
            .ok_or_else(|| "This recorded effect has no direct value".to_string())?;
        let value = value_basis_points.min(10_000);
        direct.value_basis_points = value;
        let base_name = effect
            .name
            .split_once(" · ")
            .map_or(effect.name.as_str(), |(base, _)| base)
            .to_string();
        let value_label = if direct.control_key.starts_with("relay.") {
            if value >= 5_000 { "On".to_string() } else { "Off".to_string() }
        } else {
            format!("{:.2}%", f32::from(value) / 100.0)
        };
        effect.name = format!("{base_name} · {value_label}");
        for action in &mut effect.actions {
            action.state = value >= 5_000;
        }
        self.sync_timeline_engine();
        Ok(())
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

fn controller_effect_catalog(
    capabilities: &crate::four_d::controller::HardwareCapabilities,
) -> Vec<ControllerEffectCatalogEntry> {
    capabilities
        .macros
        .iter()
        .map(|effect| ControllerEffectCatalogEntry {
            key: ControllerEffectCatalogKey::Macro(effect.id),
            name: effect.name.clone(),
            icon: crate::ui::icons::named_control_icon(&effect.icon)
                .unwrap_or(crate::ui::icons::SPARKLE)
                .to_string(),
            duration_ms: effect.duration_ms.max(1),
            mode: effect.mode.clone(),
            lane: controller_macro_lane(effect),
        })
        .chain(capabilities.strip_effects.iter().map(|effect| {
            ControllerEffectCatalogEntry {
                key: ControllerEffectCatalogKey::Strip(effect.id.clone()),
                name: effect.name.clone(),
                icon: crate::ui::icons::named_control_icon(&effect.icon)
                    .unwrap_or(crate::ui::icons::SPARKLE)
                    .to_string(),
                duration_ms: effect.default_duration_ms.unwrap_or(5_000).max(1),
                mode: effect.engine.clone(),
                lane: crate::four_d::models::ControllerEffectLane::Lighting,
            }
        }))
        .collect()
}

fn reconcile_controller_effect_templates(
    timeline: &mut crate::four_d::models::Timeline,
    catalog: &[ControllerEffectCatalogEntry],
) -> bool {
    let mut changed = false;
    for template in &mut timeline.templates {
        let catalog_entry = if let Some(reference) = template.controller_macro.as_ref() {
            catalog.iter().find(|entry| {
                matches!(entry.key, ControllerEffectCatalogKey::Macro(id) if id == reference.id)
            })
        } else if let Some(reference) = template.controller_strip_effect.as_ref() {
            catalog.iter().find(|entry| {
                matches!(&entry.key, ControllerEffectCatalogKey::Strip(id) if id == &reference.id)
            })
        } else {
            None
        };
        let Some(catalog_entry) = catalog_entry else {
            continue;
        };

        let mut refreshed = match &catalog_entry.key {
            ControllerEffectCatalogKey::Macro(id) => {
                crate::four_d::models::Effect::controller_macro(
                    catalog_entry.name.clone(),
                    catalog_entry.icon.clone(),
                    catalog_entry.duration_ms,
                    *id,
                    catalog_entry.mode.clone(),
                )
            }
            ControllerEffectCatalogKey::Strip(id) => {
                crate::four_d::models::Effect::controller_strip_effect(
                    catalog_entry.name.clone(),
                    catalog_entry.duration_ms,
                    id.clone(),
                )
            }
        };
        refreshed.id = template.id;
        refreshed.icon.clone_from(&catalog_entry.icon);
        refreshed.controller_lane = Some(catalog_entry.lane);
        if *template != refreshed {
            *template = refreshed;
            changed = true;
        }
    }
    changed
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
        "melodies": capabilities.melodies.iter().map(|melody| serde_json::json!({
            "name": melody.name,
            "duration_ms": melody.duration_ms(),
            "notes": melody.notes,
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

fn saved_effect_id(output: &str) -> Option<u64> {
    output
        .strip_prefix("effect ")
        .or_else(|| output.strip_prefix("macro "))?
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
        let _ = mpv_client.observe_property("sub-text", libmpv2::Format::String, 19);
        let _ = mpv_client.observe_property("sub-pos", libmpv2::Format::Double, 20);
        let _ = mpv_client.observe_property("video-out-params/aspect", libmpv2::Format::Double, 21);
        let _ = mpv_client.observe_property("paused-for-cache", libmpv2::Format::Flag, 22);
        let (_interop_tx, interop_rx) = std::sync::mpsc::channel();
        let (_controller_cmd_tx, controller_cmd_rx) =
            std::sync::mpsc::channel::<crate::platform::interop::ControllerDelivery>();
        let (web_state_tx, _web_state_rx) = std::sync::mpsc::channel();
        let (_web_cmd_tx, web_cmd_rx) = std::sync::mpsc::channel();
        let (media_cmd_tx, media_cmd_rx) = std::sync::mpsc::channel();
        let engine_handle = crate::four_d::engine::spawn_engine();
        engine_handle.attach_playback_clock(mpv);
        Self {
            web_only: false,
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
            color_palette: crate::config::ColorPalette::Native,
            rtl: crate::config::resolve_rtl(
                crate::config::AppDirection::Auto,
                crate::config::resolve_language(crate::config::AppLanguage::System),
            ),
            mpv: crate::mpv::player::Player(mpv),
            mpv_client,
            render_context: Arc::new(Mutex::new(None)),
            playback_time: 0.0,
            duration: 0.0,
            is_seekable: false,
            media_metadata_loaded: false,
            cache_duration: None,
            cache_buffering_percent: None,
            media_fps: 0.0,
            video_aspect_ratio: 16.0 / 9.0,
            consistent_video_aspect_ratio: true,
            always_on_top: crate::config::AlwaysOnTopMode::Never,
            applied_always_on_top: None,
            pending_video_aspect_resize: false,
            is_paused: false,
            is_eof: false,
            volume: 100.0,
            is_muted: false,
            playback_rate: 1.0,
            configured_playback_speed: 1.0,
            temporary_fast_forward_speed: 2.0,
            video_surface_gesture: None,
            seek_pos: None,
            seek_controller: crate::mpv::seek::SeekController::new(
                crate::mpv::seek::MpvSeekBackend::new(mpv),
            ),
            was_playing_before_scrub: false,
            is_scrubbing: false,
            pending_scrub_commit: None,
            last_mouse_activity: std::time::Instant::now(),
            pin_controls: false,
            show_error: None,
            show_sub_settings: false,
            sub_visibility: true,
            sub_font_size: 55.0,
            numeric_input_steps: Default::default(),
            sub_delay: 0.0,
            sub_position_percent: 100.0,
            current_sid: "no".to_string(),
            sub_tracks: Vec::new(),
            subtitle_direction: crate::subtitle::SubtitleDirection::Auto,
            subtitle_alignment: crate::subtitle::SubtitleAlignment::Center,
            subtitle_text_replacements: crate::subtitle::default_text_replacements(),
            subtitle_text: String::new(),
            current_vid: "no".to_string(),
            video_tracks: Vec::new(),
            show_audio_settings: false,
            audio_delay: 0.0,
            current_aid: "no".to_string(),
            audio_tracks: Vec::new(),
            media_tracks: Vec::new(),
            media_file_info: crate::media_info::MediaFileInfo::default(),
            media_track_properties: None,
            selected_timeline_track: None,
            show_four_d_editor: true,
            dock_state: crate::ui::layout::create_initial_layout(),
            timeline: crate::four_d::models::Timeline::new(),
            engine_handle,
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
            osd_display_options: None,
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
            hardware_control_color_draft: String::new(),
            hardware_control_up_color_draft: String::new(),
            hardware_control_down_color_draft: String::new(),
            hardware_control_pwm_percent: 0.0,
            keyboard_shortcuts_enabled: true,
            media_keys_enabled: true,
            application_shortcuts: crate::application_shortcuts::ApplicationShortcuts::default(),
            global_hardware_hotkeys_enabled: true,
            hardware_key_bindings: Vec::new(),
            hardware_binding_dialog_channel: None,
            hardware_binding_draft: None,
            hardware_binding_capturing: false,
            hardware_hotkey_runtime:
                crate::hardware_shortcuts::GlobalHardwareShortcutRuntime::default(),
            active_hardware_bindings: std::collections::BTreeSet::new(),
            board_operation: None,
            board_operation_status: String::new(),
            rf: crate::ui::rf::RfState::default(),
            board_settings_draft: None,
            board_settings_dirty: false,
            board_reboot_armed: false,
            front_panel_refresh_attempted: false,
            front_panel_pending_key: None,
            pause_on_hardware_disconnect: true,
            auto_connect_hardware: true,
            click_player_to_toggle: true,
            show_subseconds: true,
            human_readable_time_units: true,
            seekbar_hover_thumbnails: false,
            nle_seekbar_hover_thumbnails: false,
            seekbar_thumbnail_preview: crate::ui::seek_preview::SeekbarThumbnailPreview::default(),
            quick_seek_seconds: 10.0,
            frame_step_count: 1,
            wheel_seek_seconds: 5.0,
            osd_position: crate::config::OsdPosition::TopLeft,
            osd_timeout_seconds: 3.5,
            paused_drag_action: crate::config::PlayerDragAction::MoveWindow,
            playing_drag_action: crate::config::PlayerDragAction::TemporaryFastForward,
            middle_click_action: crate::config::PlayerClickAction::None,
            middle_hold_action: crate::config::PlayerDragAction::None,
            right_click_action: crate::config::PlayerClickAction::ContextMenu,
            right_hold_action: crate::config::PlayerDragAction::None,
            fullscreen_video_background: crate::config::VideoBackground::Black,
            motion_control_mode: crate::config::MotionControlMode::Hold,
            held_motion_action: None,
            compact_hardware_controls: false,
            compact_timeline_tracks: true,
            timeline_header_wheel_vertical_scroll: true,
            timeline_plain_wheel_action: crate::config::TimelineWheelBehavior::Zoom,
            timeline_ctrl_wheel_action: crate::config::TimelineWheelBehavior::VerticalScroll,
            timeline_shift_wheel_action: crate::config::TimelineWheelBehavior::HorizontalScroll,
            timeline_alt_wheel_action: crate::config::TimelineWheelBehavior::Zoom,
            timeline_middle_button_pan: true,
            timeline_middle_axis_lock_modifiers: true,
            timeline_animated_navigation: true,
            timeline_navigation_transition_ms: 220,
            non_user_control_visibility: crate::config::NonUserControlVisibility::Dimmed,
            prefix_relay_identifiers: true,
            live_pwm_updates: true,
            hardware_actions_on_press: true,
            single_instance: true,
            window_magnetic_snap: false,
            window_magnetic_snap_distance: 16,
            windows_mica_backdrop: false,
            windows_dwm_theming: true,
            windows_video_taskbar_thumbnail: true,
            windows_thumbnail_toolbar: true,
            windows_jump_list_quick_actions: true,
            opengl_vsync: false,
            live_video_during_window_move: true,
            compositor_paced_window_move: true,
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
            media_cmd_tx,
            media_cmd_rx,
            window_handle: None,
            shell_initialized: false,
            last_taskbar_state: None,
            last_thumbnail_button_state: None,
            last_update_notice_state: None,
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

    static APP_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn lock_app_tests() -> std::sync::MutexGuard<'static, ()> {
        APP_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn web_sync_uses_fast_cadence_only_while_playback_or_operations_are_active() {
        let configured = std::time::Duration::from_millis(100);
        assert_eq!(effective_web_sync_interval(configured, true), configured);
        assert_eq!(
            effective_web_sync_interval(configured, false),
            std::time::Duration::from_secs(1)
        );
        assert_eq!(
            effective_web_sync_interval(std::time::Duration::from_secs(2), false),
            std::time::Duration::from_secs(2)
        );
    }

    #[test]
    fn application_shortcuts_dispatch_fullscreen_and_preferences_without_retriggering_transport() {
        let _lock = lock_app_tests();
        let mut app = PealayerApp::default();
        let ctx = egui::Context::default();
        let frame = |app: &mut PealayerApp, key, modifiers| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed: true,
                        repeat: false,
                        modifiers,
                    }],
                    ..Default::default()
                },
                |_| {
                    app.process_application_shortcuts(&ctx);
                },
            );
            output.textures_delta.clear();
        };
        frame(&mut app, egui::Key::F11, egui::Modifiers::NONE);
        assert!(app.fullscreen_intent(&ctx));
        assert!(!ctx.input(|input| input.key_pressed(egui::Key::F11)));
        app.keyboard_shortcuts_enabled = false;
        frame(&mut app, egui::Key::F11, egui::Modifiers::NONE);
        assert!(app.fullscreen_intent(&ctx));
        app.keyboard_shortcuts_enabled = true;
        let mut modifiers = egui::Modifiers::NONE;
        if cfg!(target_os = "macos") {
            modifiers.mac_cmd = true;
            modifiers.command = true;
        } else {
            modifiers.ctrl = true;
            modifiers.command = true;
        }
        frame(&mut app, egui::Key::Comma, modifiers);
        assert!(app.show_preferences_dialog);
        assert!(
            !ctx.input(|input| input.key_pressed(egui::Key::Comma)),
            "preferences accelerator must not also frame-step backward"
        );
    }

    #[test]
    fn shared_media_track_identity_covers_video_audio_and_subtitles() {
        let _lock = lock_app_tests();
        let mut app = PealayerApp::default();
        app.current_vid = "2".to_string();
        app.current_aid = "5".to_string();
        app.current_sid = "9".to_string();

        assert_eq!(app.current_media_track_id(MediaTrackType::Video), "2");
        assert_eq!(app.current_media_track_id(MediaTrackType::Audio), "5");
        assert_eq!(app.current_media_track_id(MediaTrackType::Subtitle), "9");
    }

    #[test]
    fn exact_seek_settlement_rejects_stale_positions() {
        assert!(!seek_target_reached(12.0, 48.0, 24.0));
        assert!(!seek_target_reached(47.5, 48.0, 24.0));
        assert!(!seek_target_reached(f64::NAN, 48.0, 24.0));
    }

    #[test]
    fn exact_seek_settlement_accepts_the_committed_frame() {
        assert!(seek_target_reached(48.0, 48.0, 24.0));
        assert!(seek_target_reached(48.04, 48.0, 24.0));
        assert!(seek_target_reached(48.08, 48.0, 24.0));
    }

    #[test]
    fn paused_exact_seek_retains_sub_frame_timeline_position() {
        assert_eq!(settled_seek_position(3.462, 3.429, false), 3.429);
    }

    #[test]
    fn playing_exact_seek_follows_decoded_media_clock() {
        assert_eq!(settled_seek_position(3.462, 3.429, true), 3.462);
    }

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

    #[test]
    fn deleting_selected_cues_is_grouped_as_one_undoable_timeline_edit() {
        let _lock = lock_app_tests();
        let mut app = PealayerApp::default();
        let effect = crate::four_d::models::Effect::new(
            "Selected cue".to_string(),
            String::new(),
            1_000,
            Vec::new(),
        );
        let effect_id = effect.id;
        app.timeline.templates.push(effect);
        let first = crate::four_d::models::EffectInstance::new(effect_id, 1_000);
        let second = crate::four_d::models::EffectInstance::new(effect_id, 2_000);
        let retained = crate::four_d::models::EffectInstance::new(effect_id, 3_000);
        app.selected_instance_ids.insert(first.id);
        app.selected_instance_ids.insert(second.id);
        app.timeline.instances = vec![first, second, retained.clone()];

        assert_eq!(app.delete_selected_timeline_cues(), 2);
        assert_eq!(app.timeline.instances, vec![retained]);
        assert!(app.selected_instance_ids.is_empty());

        let deleted_state = app.snapshot_timeline();
        let restored = app
            .undo_stack
            .undo(deleted_state)
            .expect("cue deletion should create one undo checkpoint");
        assert_eq!(restored.instances.len(), 3);
    }

    #[test]
    fn edited_controller_effect_metadata_refreshes_every_placed_timeline_copy() {
        let first = crate::four_d::models::Effect::controller_macro(
            "Old name".to_string(),
            "old-icon".to_string(),
            1_000,
            7,
            "host".to_string(),
        );
        let first_id = first.id;
        let mut isolated = first.clone();
        isolated.id = uuid::Uuid::new_v4();
        let isolated_id = isolated.id;
        let strip = crate::four_d::models::Effect::controller_strip_effect(
            "Old lighting".to_string(),
            5_000,
            "aurora".to_string(),
        );
        let strip_id = strip.id;
        let local = crate::four_d::models::Effect::new(
            "Local effect".to_string(),
            "local-icon".to_string(),
            750,
            Vec::new(),
        );
        let local_id = local.id;
        let mut timeline = crate::four_d::models::Timeline {
            templates: vec![first, isolated, strip, local],
            ..Default::default()
        };
        timeline
            .instances
            .push(crate::four_d::models::EffectInstance::new(first_id, 1_000));
        timeline.instances.push(crate::four_d::models::EffectInstance::new(
            isolated_id,
            2_000,
        ));
        timeline
            .instances
            .push(crate::four_d::models::EffectInstance::new(strip_id, 3_000));
        let original_instances = timeline.instances.clone();

        let capabilities = crate::four_d::controller::HardwareCapabilities {
            macros: vec![crate::four_d::controller::HardwareMacro {
                id: 7,
                name: "Renamed display cue".to_string(),
                icon: "monitor".to_string(),
                mode: "mcu".to_string(),
                duration_ms: 2_750,
                steps: vec![crate::four_d::controller::HardwareMacroStep {
                    kind: "display".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            strip_effects: vec![crate::four_d::controller::HardwareStripEffect {
                id: "aurora".to_string(),
                name: "Renamed aurora".to_string(),
                icon: "sparkle".to_string(),
                engine: "host".to_string(),
                default_duration_ms: Some(8_000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let catalog = super::controller_effect_catalog(&capabilities);

        assert!(super::reconcile_controller_effect_templates(
            &mut timeline,
            &catalog
        ));
        assert_eq!(timeline.instances, original_instances);
        for id in [first_id, isolated_id] {
            let effect = timeline
                .templates
                .iter()
                .find(|effect| effect.id == id)
                .unwrap();
            assert_eq!(effect.name, "Renamed display cue");
            assert_eq!(effect.duration_ms, 2_750);
            assert_eq!(
                effect.controller_lane,
                Some(crate::four_d::models::ControllerEffectLane::Display)
            );
            assert_eq!(effect.controller_macro.as_ref().unwrap().mode, "mcu");
        }
        let strip = timeline
            .templates
            .iter()
            .find(|effect| effect.id == strip_id)
            .unwrap();
        assert_eq!(strip.name, "Renamed aurora");
        assert_eq!(strip.duration_ms, 8_000);
        let local = timeline
            .templates
            .iter()
            .find(|effect| effect.id == local_id)
            .unwrap();
        assert_eq!(local.name, "Local effect");
        assert_eq!(local.duration_ms, 750);
        assert!(!super::reconcile_controller_effect_templates(
            &mut timeline,
            &catalog
        ));
    }
    use super::{
        DroppedFileKind, contextual_window_title, controller_macro_effect_preset, dropped_file_kind,
    };

    #[test]
    fn extracts_saved_controller_macro_id_for_catalog_reconciliation() {
        assert_eq!(
            super::saved_effect_id(
                "effect 6/seat-motion saved with 7 steps and auto execution policy"
            ),
            Some(6)
        );
        assert_eq!(
            super::saved_effect_id("macro 7/legacy saved with 2 mcu-timed steps"),
            Some(7)
        );
        assert_eq!(super::saved_effect_id("recording is empty"), None);
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
        let _lock = lock_app_tests();
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
        assert!(should_throttle_video_render(true, false, true));
        assert!(should_throttle_video_render(true, true, false));
        assert!(!should_throttle_video_render(true, false, false));
        assert!(!should_throttle_video_render(false, true, true));
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
        let _lock = lock_app_tests();
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
        let _lock = lock_app_tests();
        let app = PealayerApp::default();
        assert!(!app.was_fullscreen);
        assert!(app.show_four_d_editor);
    }

    #[test]
    fn rpc_fullscreen_round_trip_restores_the_staged_nle_workspace() {
        let _lock = lock_app_tests();
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
        let _lock = lock_app_tests();
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
        let _lock = lock_app_tests();
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
        let _lock = lock_app_tests();
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
        let _lock = lock_app_tests();
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
        let _lock = lock_app_tests();
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
        let _lock = lock_app_tests();
        let mut app = PealayerApp::default();
        app.estop_active = true;
        app.confirm_estop_release = false;

        app.request_emergency_stop_change(false);

        assert!(!app.estop_active);
        assert!(!app.show_estop_release_dialog);
    }

    #[test]
    fn successful_effect_publish_is_immediately_runnable_before_catalog_refresh() {
        let _lock = lock_app_tests();
        let mut app = PealayerApp::default();
        assert!(!app.controller_effect_is_advertised("effect:17"));

        app.hardware_effect_authoring
            .acknowledge_effect_publish("17");

        assert!(app.controller_effect_is_advertised("effect:17"));
        assert!(app.controller_effect_is_advertised("EFFECT:17"));
        assert!(!app.controller_effect_is_advertised("effect:18"));
    }

    #[test]
    fn deleting_an_acknowledged_effect_revokes_the_immediate_publish_state() {
        let mut authoring = HardwareEffectAuthoringState::default();
        authoring.acknowledge_effect_publish("effect:rainbow-wave");

        authoring.forget_effect_publish("RAINBOW-WAVE");

        assert!(!authoring.effect_publish_is_acknowledged("effect:rainbow-wave"));
    }

    #[test]
    fn effect_preview_phase_is_contextual_per_effect_and_expires() {
        let mut authoring = HardwareEffectAuthoringState::default();
        let started_at = std::time::Instant::now();
        authoring.begin_effect_preview(
            "effect:7",
            Some(std::time::Duration::from_millis(750)),
        );
        authoring.pending_operation = Some("effect-play".to_string());

        assert_eq!(
            authoring.effect_preview_phase("effect:7", started_at),
            ControllerEffectPreviewPhase::Starting
        );
        assert_eq!(
            authoring.effect_preview_phase("effect:8", started_at),
            ControllerEffectPreviewPhase::Run
        );

        authoring.pending_operation = None;
        authoring.acknowledge_effect_preview(started_at);
        assert_eq!(
            authoring.effect_preview_phase(
                "effect:7",
                started_at + std::time::Duration::from_millis(749),
            ),
            ControllerEffectPreviewPhase::Stop
        );

        assert_eq!(
            authoring.effect_preview_phase(
                "effect:7",
                started_at + std::time::Duration::from_millis(750),
            ),
            ControllerEffectPreviewPhase::Run
        );
        assert!(!authoring.preview_active);
        assert_eq!(authoring.preview_reference, None);
    }

    #[test]
    fn effect_preview_phase_reports_stopping_only_for_the_running_card() {
        let mut authoring = HardwareEffectAuthoringState::default();
        let now = std::time::Instant::now();
        authoring.begin_effect_preview("strip:Aurora", None);
        authoring.acknowledge_effect_preview(now);
        authoring.pending_operation = Some("effect-stop".to_string());

        assert_eq!(
            authoring.effect_preview_phase("effect:aurora", now),
            ControllerEffectPreviewPhase::Stopping
        );
        assert_eq!(
            authoring.effect_preview_phase("effect:other", now),
            ControllerEffectPreviewPhase::Run
        );
        assert_eq!(
            canonical_effect_reference("STRIP:Aurora").as_deref(),
            Some("effect:aurora")
        );
    }

    #[test]
    fn web_hardware_details_publish_every_sampled_pwm_channel() {
        let mut capabilities = crate::four_d::controller::HardwareCapabilities::default();
        capabilities.melodies = vec![crate::four_d::controller::HardwareMelody {
            name: "attention".to_string(),
            notes: vec![crate::four_d::controller::HardwareMelodyNote {
                frequency_hz: 880,
                duration_ms: 100,
                gap_ms: 25,
            }],
        }];
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
        assert_eq!(details["melodies"][0]["name"], "attention");
        assert_eq!(details["melodies"][0]["duration_ms"], 125);

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
