use crate::mpv::render::RenderContextWrapper;
use eframe::egui;
use libmpv2::Mpv;
use std::sync::{Arc, Mutex};

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
    pub(crate) timeline: crate::four_d::models::Timeline,
    pub(crate) engine_handle: crate::four_d::engine::EngineHandle,
    pub(crate) recording_keys: std::collections::HashMap<eframe::egui::Key, (uuid::Uuid, std::time::Instant)>,
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

        let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        if !is_fullscreen {
            crate::ui::menu::draw(self, ui);
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
            let compiled = crate::four_d::engine::compile_timeline(&self.timeline);
            let _ = self.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
        }

        let mut frame = egui::Frame::central_panel(&ui.style());
        frame.inner_margin = egui::Margin::same(0);

        egui::CentralPanel::default()
            .frame(frame)
            .show_inside(ui, |ui| {
                crate::ui::video::draw(self, ui);
                crate::ui::controls::draw(self, ui);
                crate::ui::error::draw(self, ui);
                crate::ui::subtitles::draw_settings_dialog(self, ui);
                crate::ui::audio::draw_settings_dialog(self, ui);
                crate::ui::four_d::draw_editor(self, ui);
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
