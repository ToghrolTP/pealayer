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
    pub actions: Vec<crate::four_d::models::AtomicAction>,
}

pub struct RttState {
    pub video_texture: Option<eframe::glow::Texture>,
    pub video_fbo: Option<eframe::glow::Framebuffer>,
    pub video_texture_id: Option<eframe::egui::TextureId>,
}

pub struct PealayerApp {
    pub(crate) mpv: &'static Mpv,
    pub(crate) mpv_client: libmpv2::Mpv,
    pub(crate) render_context: Arc<Mutex<RenderContextWrapper>>,

    pub(crate) playback_time: f64,
    pub(crate) duration: f64,
    pub(crate) is_paused: bool,
    pub(crate) volume: f64,
    pub(crate) is_muted: bool,

    pub(crate) seek_pos: Option<f64>,
    pub(crate) last_mouse_activity: std::time::Instant,

    pub(crate) show_error: Option<String>,

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
    pub(crate) timeline: crate::four_d::models::Timeline,
    pub(crate) engine_handle: crate::four_d::engine::EngineHandle,
    
    // Phase 4 & 5 Selection/Override state
    pub(crate) selected_instance_ids: std::collections::HashSet<uuid::Uuid>,
    pub(crate) recording_keys: std::collections::HashMap<eframe::egui::Key, (uuid::Uuid, std::time::Instant)>,
    pub(crate) relay_overrides: [Option<bool>; 9],

    // Phase 6 Preset Library state
    pub(crate) preset_library: Vec<EffectPreset>,
    pub(crate) effects_search_query: String,
    pub(crate) track_muted: [bool; 9],
    pub(crate) track_soloed: [bool; 9],
    pub(crate) track_locked: [bool; 9],
    pub(crate) active_drag: Option<ActiveDragState>,
    pub(crate) estop_active: bool,
    pub(crate) serial_port: String,
    pub(crate) is_connected: bool,
    pub(crate) lasso_origin: Option<egui::Pos2>,
    pub(crate) lasso_rect: Option<egui::Rect>,
    pub(crate) rtt_state: Arc<Mutex<RttState>>,
    pub(crate) current_video_path: Option<std::path::PathBuf>,
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
            if let Some(gl) = _frame.gl() {
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
                    let texture_id = _frame.register_native_glow_texture(tex);
                    
                    rtt_data = Some((tex, fbo, texture_id));
                }
            }
        }
        
        if let Some((tex, fbo, texture_id)) = rtt_data {
            let mut rtt = self.rtt_state.lock().unwrap();
            rtt.video_texture = Some(tex);
            rtt.video_fbo = Some(fbo);
            rtt.video_texture_id = Some(texture_id);
        }

        use libmpv2::events::{Event, PropertyData};

        let ctx = ui.ctx().clone();

        // Read MPV events
        loop {
            match self.mpv_client.wait_event(0.0) {
                Some(Ok(Event::PropertyChange {
                    reply_userdata,
                    change,
                    ..
                })) => match (reply_userdata, change) {
                    (1, PropertyData::Double(v)) => {
                        self.playback_time = v;
                        self.engine_handle
                            .playback_time_ms
                            .store((v * 1000.0) as u64, std::sync::atomic::Ordering::Relaxed);
                    }
                    (2, PropertyData::Double(v)) => self.duration = v,
                    (3, PropertyData::Flag(v)) => {
                        self.is_paused = v;
                        self.engine_handle
                            .is_playing
                            .store(!v, std::sync::atomic::Ordering::Relaxed);
                    }
                    (4, PropertyData::Double(v)) => self.volume = v,
                    (5, PropertyData::Flag(v)) => self.is_muted = v,
                    (6, PropertyData::Flag(v)) => self.sub_visibility = v,
                    (7, PropertyData::Double(v)) => self.sub_font_size = v,
                    (8, PropertyData::Double(v)) => self.sub_delay = v,
                    (9, PropertyData::Str(v)) => self.current_sid = v.to_string(),
                    (9, PropertyData::OsdStr(v)) => self.current_sid = v.to_string(),
                    (10, PropertyData::Double(v)) => self.audio_delay = v,
                    (11, PropertyData::Str(v)) => self.current_aid = v.to_string(),
                    (11, PropertyData::OsdStr(v)) => self.current_aid = v.to_string(),
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
                    let _ =
                        self.engine_handle
                            .sender
                            .send(crate::four_d::engine::EngineMessage::Seek(
                                (self.playback_time * 1000.0) as u64,
                            ));
                }
                Some(Ok(Event::StartFile)) => {
                    self.show_error = None;
                    self.refresh_sub_tracks();
                    self.refresh_audio_tracks();
                }
                Some(Ok(_)) => {}
                _ => break,
            }
        }

        // Sync connection state and check for errors from background thread
        if let Ok(mut err_guard) = self.engine_handle.connection_error.try_lock() {
            if let Some(err) = err_guard.take() {
                self.show_error = Some(err);
            }
        }
        self.is_connected = self.engine_handle.is_connected.load(std::sync::atomic::Ordering::Relaxed);

        let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        if !is_fullscreen {
            crate::ui::menu::draw(self, ui);
            crate::ui::status_bar::draw(self, ui);
        }

        // Track mouse movement
        if ctx.input(|i| {
            i.pointer.delta().length() > 0.0 || i.pointer.any_click() || i.pointer.any_pressed()
        }) {
            self.last_mouse_activity = std::time::Instant::now();
        } else if self.last_mouse_activity.elapsed().as_secs_f32() > 3.0 {
            // Hide mouse cursor when inactive
            ctx.set_cursor_icon(egui::CursorIcon::None);
        }

        // Handle Keyboard Shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            let _ = self.mpv.command("cycle", &["pause"]);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::M)) {
            let _ = self.mpv.command("cycle", &["mute"]);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            let _ = self.mpv.command("seek", &["-5", "relative"]);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            let _ = self.mpv.command("seek", &["5", "relative"]);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            let _ = self.mpv.command("add", &["volume", "5"]);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            let _ = self.mpv.command("add", &["volume", "-5"]);
        }

        const MACRO_KEYS: [(egui::Key, u8); 8] = [
            (egui::Key::F1, 1),
            (egui::Key::F2, 2),
            (egui::Key::F3, 3),
            (egui::Key::F4, 4),
            (egui::Key::F5, 5),
            (egui::Key::F6, 6),
            (egui::Key::F7, 7),
            (egui::Key::F8, 8),
        ];

        let mut timeline_dirty = false;
        
        for (key, relay_id) in MACRO_KEYS {
            if ctx.input(|i| i.key_pressed(key)) && !self.recording_keys.contains_key(&key) {
                let start_time = (self.playback_time * 1000.0) as u64;
                
                let actions = crate::four_d::patterns::generate_constant(relay_id, true, 100);
                let template = crate::four_d::models::Effect::new(
                    format!("Recorded Relay {}", relay_id),
                    "🔴".to_string(),
                    100,
                    actions
                );
                let template_id = template.id;
                self.timeline.templates.push(template);
                
                let instance = crate::four_d::models::EffectInstance::new(template_id, start_time);
                let instance_id = instance.id;
                self.timeline.instances.push(instance);
                
                self.recording_keys.insert(key, (instance_id, std::time::Instant::now()));
                timeline_dirty = true;
            }
            
            if ctx.input(|i| i.key_released(key)) {
                if let Some((instance_id, start_instant)) = self.recording_keys.remove(&key) {
                    if let Some(instance) = self.timeline.instances.iter().find(|i| i.id == instance_id) {
                        let mut duration = start_instant.elapsed().as_millis() as u64;
                        if duration < 100 {
                            duration = 100; // minimum duration
                        }
                        
                        if let Some(template) = self.timeline.templates.iter_mut().find(|t| t.id == instance.effect_id) {
                            template.duration_ms = duration;
                            template.actions = crate::four_d::patterns::generate_constant(relay_id, true, duration);
                        }
                        timeline_dirty = true;
                    }
                }
            }
        }
        
        if timeline_dirty {
            let compiled = crate::four_d::engine::compile_timeline(&self.timeline, &self.track_muted, &self.track_soloed);
            let _ = self.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
        }

        let mut frame = egui::Frame::central_panel(&ui.style());
        frame.inner_margin = egui::Margin::same(0);

        egui::CentralPanel::default()
            .frame(frame)
            .show_inside(ui, |ui| {
                if self.show_four_d_editor {
                    let mut dock_state = std::mem::replace(&mut self.dock_state, egui_dock::DockState::new(vec![]));
                    let mut tab_viewer = crate::ui::layout::PealayerTabViewer { app: self };
                    egui_dock::DockArea::new(&mut dock_state)
                        .show_inside(ui, &mut tab_viewer);
                    self.dock_state = dock_state;
                } else {
                    crate::ui::video::draw(self, ui);
                    crate::ui::controls::draw(self, ui);
                    crate::ui::four_d::draw_editor(self, ui);
                }
                crate::ui::error::draw(self, ui);
                crate::ui::subtitles::draw_settings_dialog(self, ui);
                crate::ui::audio::draw_settings_dialog(self, ui);
            });
    }
}

impl PealayerApp {
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
}
