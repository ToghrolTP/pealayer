//! Host audio effects. A bounded worker owns independent audio-only libmpv
//! voices; neither importing nor decoder/device work runs inside egui.
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioProgram {
    pub source: String,
    /// Empty follows Preferences → SFX output; that in turn can follow media.
    pub output_device: String,
    pub volume: u16,
}
impl Default for AudioProgram {
    fn default() -> Self {
        Self {
            source: String::new(),
            output_device: String::new(),
            volume: 100,
        }
    }
}
impl AudioProgram {
    pub fn validate(&self) -> Result<(), String> {
        if self.source.trim().is_empty()
            || self.source.len() > 8192
            || self.source.chars().any(char::is_control)
        {
            return Err("Choose an audio file or HTTP(S) URL".into());
        }
        if self.source.contains("://")
            && !self.source.starts_with("https://")
            && !self.source.starts_with("http://")
        {
            return Err("SFX URLs must use HTTP or HTTPS".into());
        }
        if self.volume > 100
            || self.output_device.len() > 512
            || self.output_device.chars().any(char::is_control)
        {
            return Err("Invalid SFX volume or output device".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioEffect {
    pub id: Uuid,
    pub name: String,
    pub group: String,
    pub icon: String,
    pub description: String,
    pub duration_ms: u64,
    pub program: AudioProgram,
}
impl AudioEffect {
    pub fn reference(&self) -> String {
        format!("sfx:{}", self.id)
    }
    pub fn template(&self) -> crate::four_d::models::Effect {
        let mut effect = crate::four_d::models::Effect::new(
            self.name.clone(),
            self.icon.clone(),
            self.duration_ms,
            vec![],
        );
        effect.id = self.id;
        effect.controller_lane = Some(crate::four_d::models::ControllerEffectLane::Audio);
        effect.audio_effect = Some(self.clone());
        effect.duration_policy = crate::four_d::models::CueDurationPolicy::Intrinsic;
        effect
    }
    pub fn validate(&self) -> Result<(), String> {
        self.program.validate()?;
        for text in [&self.name, &self.group, &self.icon] {
            if text.trim().is_empty()
                || text.chars().count() > 64
                || text.chars().any(char::is_control)
            {
                return Err(
                    "SFX name, group and icon must contain 1–64 printable characters".into(),
                );
            }
        }
        if self.description.len() > 4096 {
            return Err("SFX description is too long".into());
        }
        Ok(())
    }
}

pub fn resolved_device(program: &AudioProgram, media: &str, sfx: &str) -> String {
    [program.output_device.as_str(), sfx, media, "auto"]
        .into_iter()
        .find(|v| !v.trim().is_empty())
        .unwrap()
        .to_string()
}

#[derive(Clone)]
struct Placement {
    id: Uuid,
    start_ms: u64,
    effect: AudioEffect,
}
enum Command {
    Preview(AudioEffect),
    StopPreview,
    Outputs(String, String),
}
pub enum Completion {
    Imported(AudioEffect),
    Error(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioVoiceState {
    pub cue_id: Uuid,
    pub preview: bool,
    pub paused: bool,
    pub position_seconds: Option<f64>,
    pub backend: Option<String>,
}
struct Engine {
    commands: mpsc::SyncSender<Command>,
    plan: Arc<Mutex<Vec<Placement>>>,
    results: Arc<Mutex<Vec<Completion>>>,
    previews: Arc<Mutex<Vec<Uuid>>>,
    importing: Arc<AtomicBool>,
    voices: Arc<Mutex<Vec<AudioVoiceState>>>,
    ctx: eframe::egui::Context,
}
static ENGINE: OnceLock<Engine> = OnceLock::new();
pub fn initialize(
    clock: Arc<Mutex<crate::four_d::media_sync::PlaybackSample>>,
    estop: Arc<AtomicBool>,
    ctx: &eframe::egui::Context,
) -> bool {
    let fresh = ENGINE.get().is_none();
    ENGINE.get_or_init(|| {
        let (tx, rx) = mpsc::sync_channel(32);
        let plan = Arc::new(Mutex::new(Vec::<Placement>::new()));
        let results = Arc::new(Mutex::new(Vec::new()));
        let previews = Arc::new(Mutex::new(Vec::new()));
        let voice_states = Arc::new(Mutex::new(Vec::new()));
        let worker_states = voice_states.clone();
        let config = crate::platform::interop::get_live_config();
        let (worker_plan, worker_results, worker_previews, worker_ctx) = (plan.clone(), results.clone(), previews.clone(), ctx.clone());
        std::thread::Builder::new().name("sfx-playback".into()).spawn(move || {
            let mut voices = HashMap::<Uuid, Voice>::new();
            let mut outputs = (config.audio_device, config.sfx_audio_device);
            let mut epoch = 0;
            let mut device_refresh = Instant::now();
            let mut capacity_reported = false;
            let mut last_status = Instant::now();
            if !crate::peer::active() { std::thread::spawn(crate::mpv::audio_output::refresh_devices); }
            loop {
                while let Ok(command) = rx.try_recv() {
                    match command {
                        Command::Outputs(media, sfx) => {
                            if outputs != (media.clone(), sfx.clone()) { outputs = (media, sfx); voices.clear(); }
                        }
                        Command::StopPreview => voices.retain(|_, voice| !voice.preview),
                        Command::Preview(effect) if !estop.load(Ordering::SeqCst) && !crate::peer::active() => {
                            voices.retain(|_, voice| !voice.preview);
                            match Voice::new(&effect, &outputs, 0, 1.0, false, true) {
                                Ok(voice) => { voices.insert(effect.id, voice); }
                                Err(error) => push_error(&worker_results, &worker_ctx, error),
                            }
                        }
                        _ => {}
                    }
                }
                let sample = clock.lock().map(|v| v.clone()).unwrap_or_default();
                if epoch != sample.epoch { voices.retain(|_, v| v.preview); epoch = sample.epoch; }
                if estop.load(Ordering::SeqCst) || crate::peer::active() { voices.clear(); }
                else {
                    let position = sample.position_now();
                    let plan = worker_plan.lock().map(|v| v.clone()).unwrap_or_default();
                    let active_count = plan.iter().filter(|p| sample.loaded && position >= p.start_ms.saturating_sub(400) && position < p.start_ms.saturating_add(p.effect.duration_ms)).count();
                    if active_count > 16 && !capacity_reported {
                        push_error(&worker_results, &worker_ctx, "SFX playback supports at most 16 simultaneous sounds; reduce overlapping cues".into());
                    }
                    capacity_reported = active_count > 16;
                    voices.retain(|id, voice| voice.preview || plan.iter().any(|p| p.id == *id && sample.loaded && position >= p.start_ms.saturating_sub(400) && position < p.start_ms.saturating_add(p.effect.duration_ms)));
                    for placement in plan.iter().filter(|p| sample.loaded && position >= p.start_ms.saturating_sub(400) && position < p.start_ms.saturating_add(p.effect.duration_ms)) {
                        if voices.get(&placement.id).is_some_and(|voice| voice.program.as_ref().is_some_and(|program| program != &placement.effect.program)) {
                            voices.remove(&placement.id);
                        }
                        let paused = !sample.playing || sample.buffering || position < placement.start_ms;
                        let offset = position.saturating_sub(placement.start_ms);
                        if !voices.contains_key(&placement.id) && voices.len() < 16 {
                            match Voice::new(&placement.effect, &outputs, offset, sample.rate, paused, false) {
                                Ok(voice) => { voices.insert(placement.id, voice); }
                                Err(error) => { push_error(&worker_results, &worker_ctx, error); /* do not retry on every tick */ voices.insert(placement.id, Voice::failed()); }
                            }
                        }
                        if let Some(voice) = voices.get_mut(&placement.id) { voice.align(offset, sample.rate, paused); }
                    }
                    for voice in voices.values_mut() {
                        if let Some(error) = voice.poll_error() { push_error(&worker_results, &worker_ctx, error); }
                    }
                    voices.retain(|_, v| !v.preview || v.mpv.as_ref().is_some_and(|mpv| !mpv.get_property::<bool>("eof-reached").unwrap_or(false)));
                }
                let current = voices.iter().filter_map(|(id, v)| (v.preview && v.mpv.is_some()).then_some(*id)).collect::<Vec<_>>();
                if let Ok(mut old) = worker_previews.lock() { if *old != current { *old = current; worker_ctx.request_repaint(); } }
                if last_status.elapsed() >= Duration::from_millis(100) {
                    let current = voices.iter().filter_map(|(id, voice)| {
                        let mpv = voice.mpv.as_ref()?;
                        Some(AudioVoiceState { cue_id: *id, preview: voice.preview,
                            paused: mpv.get_property::<bool>("pause").unwrap_or(true),
                            position_seconds: mpv.get_property::<f64>("time-pos").ok(),
                            backend: mpv.get_property::<String>("current-ao").ok() })
                    }).collect();
                    if let Ok(mut old) = worker_states.lock() { *old = current; }
                    last_status = Instant::now();
                }
                if device_refresh.elapsed() > Duration::from_secs(10) {
                    if !crate::peer::active() {
                        std::thread::spawn(crate::mpv::audio_output::refresh_devices);
                    }
                    device_refresh = Instant::now();
                }
                std::thread::sleep(Duration::from_millis(if voices.is_empty() { 50 } else { 20 }));
            }
        }).expect("start SFX playback worker");
        Engine { commands: tx, plan, results, previews, voices: voice_states, importing: Arc::new(AtomicBool::new(false)), ctx: ctx.clone() }
    });
    fresh
}
fn push_error(results: &Mutex<Vec<Completion>>, ctx: &eframe::egui::Context, error: String) {
    if let Ok(mut results) = results.lock() {
        if results.len() < 32 {
            results.push(Completion::Error(error));
        }
    }
    ctx.request_repaint();
}
pub fn importing() -> bool {
    ENGINE
        .get()
        .is_some_and(|e| e.importing.load(Ordering::Relaxed))
}
pub fn active_voices() -> Vec<AudioVoiceState> {
    ENGINE
        .get()
        .and_then(|engine| engine.voices.lock().ok().map(|voices| voices.clone()))
        .unwrap_or_default()
}
pub fn take_completions() -> Vec<Completion> {
    ENGINE
        .get()
        .and_then(|e| e.results.lock().ok().map(|mut v| std::mem::take(&mut *v)))
        .unwrap_or_default()
}
pub fn previewing(id: Uuid) -> bool {
    ENGINE
        .get()
        .is_some_and(|e| e.previews.lock().is_ok_and(|v| v.contains(&id)))
}
pub fn preview(effect: AudioEffect) -> Result<(), String> {
    effect.validate()?;
    send(Command::Preview(effect))
}
pub fn stop_preview() -> Result<(), String> {
    send(Command::StopPreview)
}
fn send(command: Command) -> Result<(), String> {
    ENGINE
        .get()
        .ok_or("SFX engine is initializing")?
        .commands
        .try_send(command)
        .map_err(|_| "SFX command queue is full".into())
}
pub fn outputs(media: &str, sfx: &str) {
    let _ = send(Command::Outputs(media.into(), sfx.into()));
}
pub fn update_plan(timeline: &crate::four_d::models::Timeline) {
    if let Some(engine) = ENGINE.get()
        && let Ok(mut plan) = engine.plan.lock()
    {
        *plan = if !timeline.track_state("controller-effect:audio").linked {
            vec![]
        } else {
            timeline
                .instances
                .iter()
                .filter_map(|instance| {
                    let effect = timeline
                        .templates
                        .iter()
                        .find(|t| t.id == instance.effect_id)?
                        .audio_effect
                        .clone()?;
                    Some(Placement {
                        id: instance.id,
                        start_ms: instance.start_time_ms,
                        effect,
                    })
                })
                .collect()
        };
    }
}
pub fn import(mut effect: AudioEffect) -> Result<(), String> {
    effect.validate()?;
    let engine = ENGINE.get().ok_or("SFX engine is initializing")?;
    if engine.importing.swap(true, Ordering::SeqCst) {
        return Err("An audio import is already in progress".into());
    }
    let (results, importing, ctx) = (
        engine.results.clone(),
        engine.importing.clone(),
        engine.ctx.clone(),
    );
    std::thread::Builder::new()
        .name("sfx-import".into())
        .spawn(move || {
            let result = probe_duration(&effect.program).map(|duration| {
                effect.duration_ms = duration;
                effect
            });
            if let Ok(mut events) = results.lock() {
                events.push(match result {
                    Ok(effect) => Completion::Imported(effect),
                    Err(error) => Completion::Error(error),
                });
            }
            importing.store(false, Ordering::SeqCst);
            ctx.request_repaint();
        })
        .map_err(|error| {
            engine.importing.store(false, Ordering::SeqCst);
            format!("Start audio import: {error}")
        })?;
    Ok(())
}
fn probe_duration(program: &AudioProgram) -> Result<u64, String> {
    let config = crate::platform::interop::get_live_config();
    let mpv = libmpv2::Mpv::with_initializer(|init| {
        crate::mpv::proxy::apply_before_initialize(
            &init,
            config.open_url_use_proxy,
            config.open_url_proxy_url.as_deref().unwrap_or_default(),
        )?;
        init.set_option("vo", "null")?;
        init.set_option("ao", "null")?;
        init.set_option("vid", "no")?;
        init.set_option("pause", true)?;
        init.set_option("network-timeout", 5)?;
        Ok(())
    })
    .map_err(|error| format!("Initialize audio import: {error}"))?;
    mpv.command("loadfile", &[program.source.as_str(), "replace"])
        .map_err(|error| format!("Load audio: {error}"))?;
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        while let Some(event) = mpv.wait_event(0.0) {
            if let Err(error) = event {
                return Err(format!("Read audio: {error}"));
            }
        }
        if mpv.get_property::<i64>("track-list/count").unwrap_or(0) > 0 {
            let audio_count = mpv.get_property::<i64>("track-list/count").unwrap_or(0);
            let has_audio = (0..audio_count).any(|i| {
                mpv.get_property::<String>(&format!("track-list/{i}/type"))
                    .is_ok_and(|v| v == "audio")
            });
            if !has_audio {
                return Err("The selected media has no audio track".into());
            }
            if let Ok(duration) = mpv.get_property::<f64>("duration")
                && duration.is_finite()
                && duration > 0.0
                && duration <= 86400.0
            {
                return Ok((duration * 1000.0).round().max(1.0) as u64);
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    Err("Audio import failed: a readable audio track with a finite duration is required".into())
}
struct Voice {
    mpv: Option<libmpv2::Mpv>,
    preview: bool,
    paused: bool,
    rate: f64,
    last_seek: Instant,
    program: Option<AudioProgram>,
}
impl Voice {
    fn failed() -> Self {
        Self {
            mpv: None,
            preview: false,
            paused: true,
            rate: 1.0,
            last_seek: Instant::now(),
            program: None,
        }
    }
    fn new(
        effect: &AudioEffect,
        outputs: &(String, String),
        offset: u64,
        rate: f64,
        paused: bool,
        preview: bool,
    ) -> Result<Self, String> {
        let device = resolved_device(&effect.program, &outputs.0, &outputs.1);
        let config = crate::platform::interop::get_live_config();
        let mpv = libmpv2::Mpv::with_initializer(|init| {
            crate::mpv::proxy::apply_before_initialize(
                &init,
                config.open_url_use_proxy,
                config.open_url_proxy_url.as_deref().unwrap_or_default(),
            )?;
            init.set_option("vo", "null")?;
            init.set_option("vid", "no")?;
            init.set_option("idle", true)?;
            init.set_option("audio-device", device.as_str())?;
            init.set_option("volume", i64::from(effect.program.volume))?;
            init.set_option("pause", paused)?;
            init.set_option("speed", rate)?;
            init.set_option("start", format!("{:.3}", offset as f64 / 1000.0).as_str())?;
            init.set_option("network-timeout", 5)?;
            Ok(())
        })
        .map_err(|error| format!("Initialize SFX output: {error}"))?;
        mpv.command("loadfile", &[effect.program.source.as_str(), "replace"])
            .map_err(|error| format!("Load SFX: {error}"))?;
        Ok(Self {
            mpv: Some(mpv),
            preview,
            paused,
            rate,
            last_seek: Instant::now(),
            program: Some(effect.program.clone()),
        })
    }
    fn align(&mut self, offset: u64, rate: f64, paused: bool) {
        let Some(mpv) = &self.mpv else {
            return;
        };
        if self.paused != paused {
            let _ = mpv.set_property("pause", paused);
            self.paused = paused;
        }
        if self.rate != rate {
            let _ = mpv.set_property("speed", rate);
            self.rate = rate;
        }
        if self.last_seek.elapsed() >= Duration::from_millis(150)
            && let Ok(position) = mpv.get_property::<f64>("time-pos")
            && (position * 1000.0 - offset as f64).abs() > 120.0
        {
            let _ = mpv.command(
                "seek",
                &[
                    format!("{:.3}", offset as f64 / 1000.0).as_str(),
                    "absolute+exact",
                ],
            );
            self.last_seek = Instant::now();
        }
    }
    fn poll_error(&mut self) -> Option<String> {
        let mpv = self.mpv.as_ref()?;
        let mut failure = None;
        let mut ended = false;
        for _ in 0..64 {
            let Some(event) = mpv.wait_event(0.0) else {
                break;
            };
            match event {
                Err(error) => {
                    failure = Some(format!("SFX decoder/output failed: {error}"));
                    break;
                }
                Ok(libmpv2::events::Event::EndFile(_)) => {
                    ended = true;
                    break;
                }
                _ => {}
            }
        }
        if failure.is_some() || ended {
            self.mpv = None;
        }
        failure
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_precedence_and_default() {
        let mut p = AudioProgram::default();
        assert_eq!(resolved_device(&p, "", ""), "auto");
        assert_eq!(resolved_device(&p, "wasapi/media", ""), "wasapi/media");
        assert_eq!(
            resolved_device(&p, "wasapi/media", "wasapi/sfx"),
            "wasapi/sfx"
        );
        p.output_device = "wasapi/effect".into();
        assert_eq!(resolved_device(&p, "auto", "auto"), "wasapi/effect");
    }
    #[test]
    fn reject_invalid_sources_and_gain() {
        let mut p = AudioProgram::default();
        assert!(p.validate().is_err());
        p.source = "https://example.invalid/sound.wav".into();
        assert!(p.validate().is_ok());
        p.volume = 101;
        assert!(p.validate().is_err());
    }
    #[test]
    fn audio_templates_are_move_only_and_survive_session_roundtrip() {
        let sound = AudioEffect {
            id: Uuid::new_v4(),
            name: "Chime".into(),
            group: "Audio".into(),
            icon: "speaker-high".into(),
            description: String::new(),
            duration_ms: 1200,
            program: AudioProgram {
                source: "chime.wav".into(),
                ..Default::default()
            },
        };
        let effect = sound.template();
        assert!(!effect.duration_resizable());
        let mut timeline = crate::four_d::models::Timeline::new();
        timeline
            .instances
            .push(crate::four_d::models::EffectInstance::new(
                effect.id, 10_000,
            ));
        timeline.templates.push(effect);
        let session = timeline.controller_cue_session();
        let restored: crate::four_d::models::Timeline =
            serde_json::from_value(serde_json::to_value(session).unwrap()).unwrap();
        assert_eq!(restored.instances[0].start_time_ms, 10_000);
        assert_eq!(restored.templates[0].audio_effect, Some(sound));
        assert!(!restored.has_controller_cues());
    }
}
