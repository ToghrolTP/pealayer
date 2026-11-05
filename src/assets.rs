//! One external-asset ingestion path for native drops, the browser and IPC.
//! Parsing/transfer is off the UI thread. Structured imports are reviewed before
//! mutation; a scene is authored as cues, never applied to live outputs here.
use crate::{
    app::PealayerApp,
    four_d::models::{Effect, EffectInstance, Timeline},
    mpv::sfx::AudioEffect,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, mpsc},
};

pub const MAX_FILES: usize = 64;
pub const MAX_TIME_MS: u64 = crate::projects::MAX_TIME_MS;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "surface", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Destination {
    #[default]
    Player,
    Library,
    Timeline {
        start_time_ms: u64,
        #[serde(default)]
        track_key: Option<String>,
    },
}
impl Destination {
    pub fn validate(&self) -> Result<(), String> {
        if let Self::Timeline {
            start_time_ms,
            track_key,
        } = self
        {
            if *start_time_ms > MAX_TIME_MS
                || track_key.as_ref().is_some_and(|key| {
                    key.is_empty()
                        || key.len() > 192
                        || !key
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b":._-".contains(&b))
                })
            {
                return Err("Invalid asset placement".into());
            }
        }
        Ok(())
    }
    fn time(&self) -> u64 {
        match self {
            Self::Timeline { start_time_ms, .. } => *start_time_ms,
            _ => 0,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportRequest {
    pub paths: Vec<String>,
    #[serde(default)]
    pub destination: Destination,
}
impl ImportRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.paths.is_empty()
            || self.paths.len() > MAX_FILES
            || self
                .paths
                .iter()
                .any(|p| p.is_empty() || p.len() > 32768 || p.chars().any(char::is_control))
        {
            return Err("Choose 1–64 asset files with valid paths".into());
        }
        self.destination.validate()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Audio,
    Subtitle,
    Document,
    Media,
}
pub fn file_kind(path: &Path) -> FileKind {
    match path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "wav" | "mp3" | "flac" | "ogg" | "opus" | "m4a" | "aac" | "aiff" | "aif" | "wma" => {
            FileKind::Audio
        }
        "srt" | "vtt" | "ass" | "ssa" | "sub" | "idx" | "sup" => FileKind::Subtitle,
        "json" | "pealayer" => FileKind::Document,
        _ => FileKind::Media,
    }
}
#[derive(Clone, Debug)]
pub enum Prepared {
    Forwarded,
    Media(PathBuf),
    Subtitle(PathBuf),
    Audio(AudioEffect),
    Timeline(Timeline),
    Scene(crate::projects::Scene),
    Effect(crate::platform::interop::WebControllerEffectDraft),
}

pub fn prepare(path: &Path) -> Result<Prepared, String> {
    if !path.is_file() {
        return Err("Drop a regular asset file, not a directory or missing path".into());
    }
    match file_kind(path) {
        FileKind::Media => Ok(Prepared::Media(path.into())),
        FileKind::Subtitle => Ok(Prepared::Subtitle(path.into())),
        FileKind::Audio => Ok(Prepared::Audio(AudioEffect {
            id: uuid::Uuid::new_v4(),
            name: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .chars()
                .filter(|c| !c.is_control())
                .take(64)
                .collect(),
            group: "Audio".into(),
            icon: "speaker-high".into(),
            description: String::new(),
            duration_ms: 0,
            program: crate::mpv::sfx::AudioProgram {
                source: path.to_string_lossy().into_owned(),
                ..Default::default()
            },
        })),
        FileKind::Document => {
            let mut bytes = Vec::new();
            std::fs::File::open(path)
                .map_err(|e| e.to_string())?
                .take(crate::projects::MAX_DOCUMENT_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() as u64 > crate::projects::MAX_DOCUMENT_BYTES {
                return Err("Asset document exceeds 32 MiB".into());
            }
            let value: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|e| format!("Invalid asset JSON: {e}"))?;
            let prepared = if value
                .get("kind")
                .and_then(|v| v.as_str())
                .is_some_and(|v| v.starts_with("pealayer-"))
            {
                let document: crate::projects::Document =
                    serde_json::from_value(value).map_err(|e| e.to_string())?;
                document.validate()?;
                match document {
                    crate::projects::Document::Scene(scene)=>Prepared::Scene(scene),
                    crate::projects::Document::Timeline(document)=>Prepared::Timeline(document.timeline),
                    _=>return Err("Project/media/workspace restoration is not part of asset placement. Open that document through its authoring workflow; a drop will not replace this session".into()),
                }
            } else if value.get("instances").is_some() && value.get("templates").is_some() {
                // This is the current Timeline::save_to_file format, not a
                // renamed legacy contract. A drop APPENDS it; Open replaces it.
                let timeline: Timeline =
                    serde_json::from_value(value).map_err(|e| e.to_string())?;
                crate::projects::TimelineDocument {
                    timeline: timeline.clone(),
                    view: Default::default(),
                }
                .validate()?;
                Prepared::Timeline(timeline)
            } else if value.get("kind").is_some() && value.get("program").is_some() {
                let effect: crate::platform::interop::WebControllerEffectDraft =
                    serde_json::from_value(value).map_err(|e| e.to_string())?;
                crate::platform::interop::InteropCommand::SaveControllerEffect {
                    effect: effect.clone(),
                }
                .validate()?;
                Prepared::Effect(effect)
            } else {
                return Err(
                    "Unrecognized asset document; expected a scene, timeline or effect definition"
                        .into(),
                );
            };
            resolve_dependencies(prepared, path)
        }
    }
}
fn resolve_dependencies(mut asset: Prepared, path: &Path) -> Result<Prepared, String> {
    let resolve = |audio: &mut AudioEffect| -> Result<(), String> {
        if !crate::media::is_remote_media_target(&audio.program.source) {
            let source = crate::projects::resolve_path(path, &audio.program.source);
            if !source.is_file() {
                return Err(format!("Missing audio dependency for {}", audio.name));
            }
            audio.program.source = source.to_string_lossy().into_owned();
        }
        audio.validate()
    };
    if let Prepared::Timeline(timeline) = &mut asset {
        for effect in &mut timeline.templates {
            if let Some(audio) = &mut effect.audio_effect {
                resolve(audio)?;
            }
        }
    }
    if let Prepared::Effect(effect) = &mut asset {
        if effect.kind == "audio" {
            let mut audio = AudioEffect {
                id: uuid::Uuid::parse_str(&effect.id).map_err(|e| e.to_string())?,
                name: effect.name.clone(),
                group: effect.category.clone(),
                icon: effect.icon.clone(),
                description: effect.description.clone(),
                duration_ms: 0,
                program: serde_json::from_value(effect.program.clone())
                    .map_err(|e| e.to_string())?,
            };
            resolve(&mut audio)?;
            effect.program = serde_json::to_value(audio.program).map_err(|e| e.to_string())?;
        }
    }
    Ok(asset)
}

/// Append a fragment transactionally, remapping every identity and retaining
/// exact relative timing. Session view, media, locks and mute/solo are untouched.
pub fn append_timeline(
    target: &mut Timeline,
    source: &Timeline,
    start_ms: u64,
) -> Result<Vec<uuid::Uuid>, String> {
    let mut copy = source.clone();
    let mut ids = std::collections::BTreeMap::new();
    for effect in &mut copy.templates {
        let id = uuid::Uuid::new_v4();
        ids.insert(effect.id, id);
        effect.id = id;
    }
    let offset = |time: u64| {
        time.checked_add(start_ms)
            .filter(|t| *t <= MAX_TIME_MS)
            .ok_or_else(|| "Asset placement exceeds supported time range".to_string())
    };
    for cue in &mut copy.instances {
        cue.id = uuid::Uuid::new_v4();
        cue.effect_id = *ids.get(&cue.effect_id).ok_or("Missing effect template")?;
        cue.start_time_ms = offset(cue.start_time_ms)?;
    }
    for frame in &mut copy.keyframes {
        frame.id = uuid::Uuid::new_v4();
        frame.time_ms = offset(frame.time_ms)?;
    }
    for curve in &mut copy.analog_tracks {
        curve.id = uuid::Uuid::new_v4();
        for point in &mut curve.keyframes {
            point.time_ms = offset(point.time_ms)?;
        }
    }
    let selected = copy.instances.iter().map(|cue| cue.id).collect();
    let mut next = target.clone();
    next.templates.extend(copy.templates);
    next.instances.extend(copy.instances);
    next.keyframes.extend(copy.keyframes);
    next.analog_tracks.extend(copy.analog_tracks);
    crate::projects::TimelineDocument {
        timeline: next.clone(),
        view: Default::default(),
    }
    .validate()?;
    *target = next;
    Ok(selected)
}

type Batch = Vec<(String, Result<Prepared, String>, Destination)>;
#[derive(Default)]
pub struct ImportState {
    receiver: Option<mpsc::Receiver<Batch>>,
    reviews: VecDeque<Review>,
    audio: VecDeque<(AudioEffect, Destination)>,
    audio_active: Option<(uuid::Uuid, Destination)>,
}
impl ImportState {
    pub(crate) fn busy(&self) -> bool {
        self.receiver.is_some() || self.audio_active.is_some() || !self.audio.is_empty()
    }
    pub(crate) fn awaiting_review(&self) -> bool {
        !self.reviews.is_empty()
    }
}
#[derive(Clone, Debug)]
struct Review {
    id: uuid::Uuid,
    name: String,
    asset: Prepared,
    destination: Destination,
}
static SNAPSHOT: Mutex<Option<serde_json::Value>> = Mutex::new(None);
pub fn snapshot() -> serde_json::Value {
    SNAPSHOT
        .lock()
        .ok()
        .and_then(|v| v.clone())
        .unwrap_or_else(|| serde_json::json!({"busy":false,"reviews":[]}))
}

impl PealayerApp {
    pub(crate) fn import_assets(
        &mut self,
        request: ImportRequest,
        ctx: &eframe::egui::Context,
    ) -> Result<(), String> {
        request.validate()?;
        if self.asset_import.receiver.is_some()
            || self.asset_import.reviews.len() + self.asset_import.audio.len() + request.paths.len()
                > MAX_FILES
        {
            return Err("Finish the current asset import first".into());
        }
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("asset-import".into())
            .spawn(move || {
                let batch = if let Some(client) = crate::peer::client() {
                    // A client path belongs to the client, never to its authority.
                    // Transfer bytes through the same bounded upload API as Web.
                    let result = crate::server::asset_upload::forward(&client, &request);
                    vec![(
                        "Remote asset import".into(),
                        result.map(|_| Prepared::Forwarded),
                        request.destination,
                    )]
                } else {
                    request
                        .paths
                        .iter()
                        .map(|path| {
                            (
                                Path::new(path)
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .into_owned(),
                                prepare(Path::new(path)),
                                request.destination.clone(),
                            )
                        })
                        .collect()
                };
                let _ = tx.send(batch);
                ctx.request_repaint();
            })
            .map_err(|e| e.to_string())?;
        self.asset_import.receiver = Some(rx);
        self.publish_asset_snapshot();
        Ok(())
    }
    pub(crate) fn process_asset_imports(&mut self, ctx: &eframe::egui::Context) {
        let result = self.asset_import.receiver.as_ref().map(|rx| rx.try_recv());
        if let Some(Ok(batch)) = result {
            self.asset_import.receiver = None;
            // A mixed drop loads media before attaching its subtitle tracks.
            let mut batch = batch;
            batch.sort_by_key(|(_, asset, _)| !matches!(asset, Ok(Prepared::Media(_))));
            let mut media_loaded = false;
            for (name, asset, destination) in batch {
                match asset {
                    Ok(Prepared::Forwarded) => {}
                    Ok(Prepared::Audio(audio)) if destination != Destination::Player => {
                        self.asset_import.audio.push_back((audio, destination))
                    }
                    Ok(Prepared::Audio(audio)) => self.load_media_target(&audio.program.source),
                    Ok(Prepared::Media(path)) => {
                        if matches!(destination, Destination::Library) {
                            self.asset_error(
                                "Video files belong in the player, not the Effects Library".into(),
                            );
                            continue;
                        }
                        if !media_loaded {
                            self.load_video_file(path);
                            media_loaded = true;
                        } else if let Some(path) = path.to_str() {
                            let _ = self.mpv.command("loadfile", &[path, "append-play"]);
                        }
                    }
                    Ok(Prepared::Subtitle(path)) => {
                        if self.current_video_path.is_none() {
                            self.asset_error("Load media before attaching a subtitle".into());
                        } else if let Err(error) =
                            self.mpv.command("sub-add", &[&path.to_string_lossy()])
                        {
                            self.asset_error(format!("Could not attach subtitle: {error}"));
                        } else {
                            self.refresh_media_tracks();
                        }
                    }
                    Ok(asset) => self.asset_import.reviews.push_back(Review {
                        id: uuid::Uuid::new_v4(),
                        name,
                        asset,
                        destination,
                    }),
                    Err(error) => self.asset_error(format!("{name}: {error}")),
                }
            }
        } else if let Some(Err(mpsc::TryRecvError::Disconnected)) = result {
            self.asset_import.receiver = None;
            self.asset_error("Asset import worker stopped".into());
        }
        if self.asset_import.audio_active.is_none() && !crate::mpv::sfx::importing() {
            if let Some((audio, destination)) = self.asset_import.audio.pop_front() {
                let id = audio.id;
                match crate::mpv::sfx::import(audio) {
                    Ok(()) => self.asset_import.audio_active = Some((id, destination)),
                    Err(error) => self.asset_error(error),
                }
            }
        }
        self.publish_asset_snapshot();
        if self.asset_import.receiver.is_some() || !self.asset_import.audio.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
    pub(crate) fn asset_error(&mut self, error: String) {
        let _ = crate::messaging::publish(
            crate::messaging::ToastRequest {
                id: Some("asset-import.error".into()),
                title: "Asset import".into(),
                message: error.clone(),
                severity: crate::messaging::Severity::Error,
                timeout_ms: 8000,
                actions: vec![],
                default_action: None,
                details: None,
            },
            "assets",
        );
        self.show_error = Some(error);
    }
    fn publish_asset_snapshot(&mut self) {
        let reviews=self.asset_import.reviews.iter().map(|r|serde_json::json!({"id":r.id,"name":r.name,"kind":match r.asset {Prepared::Timeline(_)=>"timeline",Prepared::Scene(_)=>"scene",Prepared::Effect(_)=>"effect",_=>"asset"},"destination":r.destination,"effect":match &r.asset{Prepared::Effect(effect)=>serde_json::to_value(effect).unwrap_or_default(),_=>serde_json::Value::Null}})).collect::<Vec<_>>();
        let value = serde_json::json!({"busy":self.asset_import.receiver.is_some() || self.asset_import.audio_active.is_some() || !self.asset_import.audio.is_empty(),"reviews":reviews});
        if let Ok(mut snapshot) = SNAPSHOT.lock() {
            if snapshot.as_ref() != Some(&value) {
                *snapshot = Some(value);
                self.last_web_broadcast = None;
            }
        }
    }
    pub(crate) fn finish_asset_audio(&mut self, id: uuid::Uuid, effect: Option<&AudioEffect>) {
        if !self
            .asset_import
            .audio_active
            .as_ref()
            .is_some_and(|(active, _)| *active == id)
        {
            return;
        }
        let (_, destination) = self.asset_import.audio_active.take().unwrap();
        if let (
            Some(effect),
            Destination::Timeline {
                start_time_ms,
                track_key: _,
            },
        ) = (effect, destination)
        {
            let template = effect.template();
            let mut fragment = Timeline::default();
            let id = template.id;
            fragment.templates.push(template);
            fragment.instances.push(EffectInstance::new(id, 0));
            if let Err(error) = self.append_asset_fragment(&fragment, start_time_ms) {
                self.asset_error(format!("SFX saved to Effects Library. {error}"));
            }
        }
    }
    fn append_asset_fragment(&mut self, fragment: &Timeline, time: u64) -> Result<(), String> {
        if !self.is_paused {
            return Err("Pause playback before adding an asset to the timeline".into());
        }
        let mut next = self.timeline.clone();
        let ids = append_timeline(&mut next, fragment, time)?;
        if fragment.templates.iter().any(|effect| {
            effect
                .actions
                .iter()
                .any(|action| self.track_locked.contains(&action.relay_id))
                || effect
                    .direct_control
                    .as_ref()
                    .and_then(|control| control.control_key.strip_prefix("pwm."))
                    .and_then(|id| id.parse::<u8>().ok())
                    .is_some_and(|id| {
                        self.timeline
                            .analog_tracks
                            .iter()
                            .any(|track| track.channel == id && track.locked)
                    })
        }) {
            return Err("Unlock the destination tracks before importing cues".into());
        }
        self.undo_stack.push(self.snapshot_timeline());
        self.timeline = next;
        self.selected_instance_ids = ids.into_iter().collect();
        self.commit_timeline_edit();
        Ok(())
    }
    pub(crate) fn review_asset(
        &mut self,
        ctx: &eframe::egui::Context,
        id: uuid::Uuid,
        accept: bool,
    ) -> Result<(), String> {
        let index = self
            .asset_import
            .reviews
            .iter()
            .position(|r| r.id == id)
            .ok_or("Asset review is no longer available")?;
        if !accept {
            self.asset_import.reviews.remove(index);
            self.publish_asset_snapshot();
            return Ok(());
        }
        let review = self.asset_import.reviews[index].clone();
        match review.asset {
            Prepared::Timeline(timeline) => {
                self.append_asset_fragment(&timeline, review.destination.time())?
            }
            Prepared::Scene(scene) => {
                if !self.is_paused {
                    return Err("Pause playback before adding scene cues".into());
                }
                let hardware = self.advertised_hardware().ok_or(
                    "A channel catalog is needed to map this scene; connect PCController first",
                )?;
                if !scene.board_identity.is_empty()
                    && hardware
                        .board_profile
                        .as_ref()
                        .is_none_or(|p| p.board_identity != scene.board_identity)
                {
                    return Err(
                        "Scene belongs to another board; remap it explicitly before importing"
                            .into(),
                    );
                }
                let mut fragment = Timeline::default();
                for (key, value) in scene.channels {
                    let control = hardware
                        .controls
                        .iter()
                        .find(|c| c.key == key)
                        .ok_or_else(|| format!("Scene channel {key} is not advertised"))?;
                    let basis = match value {
                        crate::projects::ChannelValue::Boolean { on }
                            if control.kind == "relay" =>
                        {
                            if on {
                                10000
                            } else {
                                0
                            }
                        }
                        crate::projects::ChannelValue::Pwm { percent_milli }
                            if matches!(control.kind.as_str(), "pwm" | "mosfet") =>
                        {
                            ((percent_milli + 5) / 10) as u16
                        }
                        _ => {
                            return Err(format!(
                                "Scene channel {key} requires an explicit motion or unsupported-channel mapping"
                            ));
                        }
                    };
                    let relay = hardware.relays.iter().find(|r| r.key == key).map(|r| r.id);
                    let mut effect = Effect::direct_control(
                        format!("{} · {}", scene.name, control.name),
                        control.icon.clone(),
                        100,
                        key,
                        basis,
                        relay,
                    );
                    effect.direct_control.as_mut().unwrap().behavior =
                        crate::four_d::models::DirectCueBehavior::SetKeep;
                    fragment.instances.push(EffectInstance::new(effect.id, 0));
                    fragment.templates.push(effect);
                }
                if fragment.instances.is_empty() {
                    return Err("Scene contains no sampled channels".into());
                }
                self.append_asset_fragment(&fragment, review.destination.time())?;
            }
            Prepared::Effect(effect) => {
                if self.show_effect_library_editor
                    || crate::ui::effects_library::unsaved_designer(self, ctx).unwrap_or(true)
                {
                    return Err("Finish or discard the current effect draft first".into());
                }
                // Import opens the EXISTING editor. Publish/Save retains its
                // existing PCController ownership/validation/acknowledgement.
                let steps = effect.program.get("steps").unwrap_or(&effect.program);
                let properties = effect.program.get("properties");
                let property_string = |key: &str| {
                    properties
                        .and_then(|p| p.get(key))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string()
                };
                self.effect_library_draft = crate::app::ControllerEffectDraft {
                    id: effect.id,
                    name: effect.name,
                    icon: effect.icon,
                    category: effect.category,
                    description: effect.description,
                    kind: effect.kind.clone(),
                    duration_ms: effect.duration_ms,
                    program_json: serde_json::to_string_pretty(&effect.program)
                        .map_err(|e| e.to_string())?,
                    steps: if effect.kind == "sequence" {
                        serde_json::from_value(steps.clone()).map_err(|e| e.to_string())?
                    } else {
                        vec![]
                    },
                    repeat_count: effect
                        .program
                        .get("repeat_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1) as u16,
                    repeat_interval_ms: effect
                        .program
                        .get("repeat_interval_ms")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    default_pixels: effect.default_pixels,
                    default_fps: effect.default_fps,
                    color: effect.color,
                    engine: properties
                        .and_then(|p| p.get("mode"))
                        .and_then(|v| v.as_str())
                        .filter(|v| matches!(*v, "auto" | "host" | "mcu"))
                        .unwrap_or("auto")
                        .into(),
                    label: property_string("label"),
                    lcd_message: property_string("lcd_message"),
                    board_profile_key: property_string("board_profile_key"),
                    board_profile_mode: property_string("board_profile_mode"),
                    keep_outputs_on_cancel: properties
                        .and_then(|p| p.get("keep_outputs_on_cancel"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    timing_tolerance_us: properties
                        .and_then(|p| p.get("timing_tolerance_us"))
                        .and_then(|v| v.as_u64())
                        .and_then(|v| u32::try_from(v).ok())
                        .unwrap_or(0),
                    is_new: true,
                    ..Default::default()
                };
                self.show_effect_library_editor = true;
            }
            _ => return Err("Asset does not require review".into()),
        }
        self.asset_import.reviews.remove(index);
        self.publish_asset_snapshot();
        Ok(())
    }
    pub(crate) fn draw_asset_review(&mut self, ctx: &eframe::egui::Context) {
        let Some(review) = self.asset_import.reviews.front().cloned() else {
            return;
        };
        let mut action = None;
        eframe::egui::Modal::new(eframe::egui::Id::new("external-asset-review")).show(ctx,|ui|{
            ui.set_max_width(440.0);ui.heading("Import asset");ui.label(&review.name);ui.separator();
            let (description,button)=match &review.asset {
                Prepared::Scene(scene)=>(format!("{} sampled channels. Add set-and-keep cues at {}. Unsampled channels are not changed. This does not run the scene.",scene.channels.len(),crate::duration::format_timeline_time_ms(review.destination.time(),true)),"Add scene cues"),
                Prepared::Timeline(timeline)=>(format!("Append {} cues, {} keyframes and {} curves at {}. Existing cues, playback position and workspace are preserved.",timeline.instances.len(),timeline.keyframes.len(),timeline.analog_tracks.len(),crate::duration::format_timeline_time_ms(review.destination.time(),true)),"Add to timeline"),
                Prepared::Effect(effect)=>(format!("Open {} in Effect properties. Review it, then use the existing Save/Publish controls. Hardware programs stay owned by PCController.",effect.name),"Open properties"),
                _=>(String::new(),"Import"),
            };ui.label(description);ui.add_space(12.0);ui.horizontal(|ui|{if ui.button("Cancel").clicked(){action=Some(false);}if ui.button(button).clicked(){action=Some(true);}});
        });
        if let Some(accept) = action {
            if let Err(error) = self.review_asset(ctx, review.id, accept) {
                self.asset_error(error);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TimelineDropArea {
    pub viewport: eframe::egui::Rect,
    pub content_origin: eframe::egui::Pos2,
    pub zoom: f32,
    pub row_height: f32,
    pub keys: Vec<String>,
}
pub fn native_destination(ctx: &eframe::egui::Context) -> Destination {
    let Some(pointer) = ctx.pointer_latest_pos() else {
        return Destination::Player;
    };
    let area = ctx.data(|d| {
        d.get_temp::<TimelineDropArea>(eframe::egui::Id::new("external-assets-timeline"))
    });
    if let Some(area) = area
        .filter(|area| area.viewport.contains(pointer) && area.zoom > 0.0 && area.row_height > 0.0)
    {
        let row = ((pointer.y - area.content_origin.y) / area.row_height).floor() as usize;
        return Destination::Timeline {
            start_time_ms: (((pointer.x - area.content_origin.x) / area.zoom).max(0.0) as f64
                * 1000.0)
                .round() as u64,
            track_key: area.keys.get(row).cloned(),
        };
    }
    if ctx
        .data(|d| {
            d.get_temp::<eframe::egui::Rect>(eframe::egui::Id::new("external-assets-library"))
        })
        .is_some_and(|r| r.contains(pointer))
    {
        Destination::Library
    } else {
        Destination::Player
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classifies_assets_without_treating_json_as_media() {
        assert_eq!(file_kind(Path::new("a.WAV")), FileKind::Audio);
        assert_eq!(file_kind(Path::new("a.SRT")), FileKind::Subtitle);
        assert_eq!(file_kind(Path::new("a.json")), FileKind::Document);
        assert_eq!(file_kind(Path::new("a.mkv")), FileKind::Media);
    }
    #[test]
    fn append_preserves_position_relative_timing_and_rekeys() {
        let audio = AudioEffect {
            id: uuid::Uuid::new_v4(),
            name: "a".into(),
            group: "Audio".into(),
            icon: "speaker-high".into(),
            description: String::new(),
            duration_ms: 200,
            program: crate::mpv::sfx::AudioProgram {
                source: "https://example.invalid/a.wav".into(),
                ..Default::default()
            },
        };
        let mut source = Timeline::default();
        source.templates.push(audio.template());
        source.instances.push(EffectInstance::new(audio.id, 429));
        let mut target = source.clone();
        let ids = append_timeline(&mut target, &source, 10000).unwrap();
        assert_eq!(target.instances.len(), 2);
        assert_eq!(target.instances[1].start_time_ms, 10429);
        assert_ne!(target.instances[1].effect_id, source.templates[0].id);
        assert_eq!(ids, vec![target.instances[1].id]);
        let before = target.clone();
        assert!(append_timeline(&mut target, &source, MAX_TIME_MS).is_err());
        assert_eq!(target, before);
    }
    #[test]
    fn rejects_invalid_placement_and_empty_batch() {
        assert!(
            ImportRequest {
                paths: vec![],
                destination: Default::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            Destination::Timeline {
                start_time_ms: MAX_TIME_MS + 1,
                track_key: None
            }
            .validate()
            .is_err()
        );
    }
}
