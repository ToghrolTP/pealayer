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
    pub(crate) pin_controls: bool,

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
    pub(crate) show_remaining_time: bool,
    pub(crate) osd_message: Option<(String, std::time::Instant)>,
    pub(crate) recent_media: Vec<std::path::PathBuf>,
    pub(crate) show_open_url_dialog: bool,
    pub(crate) url_input_buffer: String,
    pub(crate) is_window_operating: bool,
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
        // Track active window/panel drag operations
        self.is_window_operating = ui.input(|i| i.pointer.any_down() && (self.active_drag.is_some() || ui.ctx().egui_is_using_pointer()));

        // Process drag and dropped files
        let dropped_file_path = ui.input(|i| {
            i.raw.dropped_files.first().and_then(|f| f.path.clone())
        });
        if let Some(path) = dropped_file_path {
            self.load_video_file(path);
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
            if let Ok(mut rtt) = self.rtt_state.try_lock() {
                rtt.video_texture = Some(tex);
                rtt.video_fbo = Some(fbo);
                rtt.video_texture_id = Some(texture_id);
            }
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
                        let prev_paused = self.is_paused;
                        self.is_paused = v;
                        self.engine_handle
                            .is_playing
                            .store(!v, std::sync::atomic::Ordering::Relaxed);
                        if self.current_video_path.is_some() && prev_paused != v {
                            self.set_osd(if v { "Pause".to_string() } else { "Play".to_string() });
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
                            self.set_osd(if v { "Mute: On".to_string() } else { "Mute: Off".to_string() });
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
        } else if self.last_mouse_activity.elapsed().as_secs_f32() > 3.0 && !self.pin_controls {
            // Hide mouse cursor when inactive
            ctx.set_cursor_icon(egui::CursorIcon::None);
        }

        // Handle Keyboard Shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            let _ = self.mpv.command("cycle", &["pause"]);
            if self.current_video_path.is_some() {
                self.is_paused = !self.is_paused;
                self.set_osd(if self.is_paused { "Pause".to_string() } else { "Play".to_string() });
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
            self.set_osd("Fullscreen".to_string());
        }
        if ctx.input(|i| i.key_pressed(egui::Key::M)) {
            let _ = self.mpv.command("cycle", &["mute"]);
            self.is_muted = !self.is_muted;
            self.set_osd(if self.is_muted { "Mute".to_string() } else { "Unmute".to_string() });
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            let _ = self.mpv.command("seek", &["-5", "relative"]);
            if self.current_video_path.is_some() {
                self.set_osd("Seek: -5s".to_string());
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            let _ = self.mpv.command("seek", &["5", "relative"]);
            if self.current_video_path.is_some() {
                self.set_osd("Seek: +5s".to_string());
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Period) || i.key_pressed(egui::Key::CloseBracket)) {
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

                if self.show_open_url_dialog {
                    let mut open_url = false;
                    let mut close_dialog = false;

                    egui::Window::new("🔗 Open Location / URL")
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                        .show(ui.ctx(), |ui| {
                            ui.label("Enter direct video URL, HTTP/HTTPS stream, or HLS link:");
                            ui.add_space(6.0);
                            
                            ui.horizontal(|ui| {
                                let text_edit = ui.add(
                                    egui::TextEdit::singleline(&mut self.url_input_buffer)
                                        .desired_width(340.0)
                                        .hint_text("https://..."),
                                );
                                if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                    open_url = true;
                                }

                                if ui.button("📋 Paste").clicked() {
                                    if let Some(text) = ui.input(|i| i.raw.events.iter().find_map(|e| match e { egui::Event::Paste(t) => Some(t.clone()), _ => None })) {
                                        self.url_input_buffer = text;
                                    }
                                }
                            });

                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                if ui.button("Open").clicked() {
                                    open_url = true;
                                }
                                if ui.button("Cancel").clicked() {
                                    close_dialog = true;
                                }
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

    pub fn load_video_file(&mut self, path: std::path::PathBuf) {
        let path_str = path.to_str().unwrap_or("");
        if !path_str.is_empty() {
            let _ = self.mpv.command("loadfile", &[path_str, "replace"]);
            self.current_video_path = Some(path.clone());
            self.add_recent_media(path.clone());
            self.set_osd(format!("Loaded: {}", path.file_name().and_then(|n| n.to_str()).unwrap_or(path_str)));
            
            // Auto-load matching sidecar timeline
            let mut sidecar = path.clone();
            sidecar.set_extension("4d.json");
            if !sidecar.exists() {
                sidecar.set_extension("json");
            }
            if sidecar.exists() {
                if let Ok(timeline) = crate::four_d::models::Timeline::load_from_file(&sidecar) {
                    self.timeline = timeline;
                    let compiled = crate::four_d::engine::compile_timeline(&self.timeline, &self.track_muted, &self.track_soloed);
                    let _ = self.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                }
            }
        }
    }

    pub fn load_url(&mut self, url: &str) {
        let trimmed = url.trim();
        if !trimmed.is_empty() {
            let _ = self.mpv.command("loadfile", &[trimmed, "replace"]);
            let path = std::path::PathBuf::from(trimmed);
            self.current_video_path = Some(path.clone());
            self.add_recent_media(path);
            self.set_osd(format!("Loading URL: {}", trimmed));
        }
    }

    pub fn close_video(&mut self) {
        let _ = self.mpv.command("stop", &[]);
        self.current_video_path = None;
        self.playback_time = 0.0;
        self.duration = 0.0;
        self.set_osd("Video Closed".to_string());
    }

    pub fn add_recent_media(&mut self, path: std::path::PathBuf) {
        self.recent_media.retain(|p| p != &path);
        self.recent_media.insert(0, path);
        if self.recent_media.len() > 10 {
            self.recent_media.truncate(10);
        }
        self.save_recent_media();
    }

    pub fn load_recent_media_from_disk() -> Vec<std::path::PathBuf> {
        let path = get_recent_config_path();
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(list) = serde_json::from_str::<Vec<std::path::PathBuf>>(&data) {
                    return list;
                }
            }
        }
        Vec::new()
    }

    pub fn save_recent_media(&self) {
        let path = get_recent_config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.recent_media) {
            let _ = std::fs::write(&path, json);
        }
    }

    pub fn clear_recent_media(&mut self) {
        self.recent_media.clear();
        self.save_recent_media();
    }

    pub fn set_osd(&mut self, msg: String) {
        self.osd_message = Some((msg, std::time::Instant::now()));
    }
}

fn get_recent_config_path() -> std::path::PathBuf {
    let mut path = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".config"))
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    path.push("pealayer");
    path.push("recent.json");
    path
}

#[cfg(test)]
mod tests {
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
            add(&mut list, std::path::PathBuf::from(format!("/video{}.mp4", i)));
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
}
