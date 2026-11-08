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
    AudioSource {
        draft_id: uuid::Uuid,
    },
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
    /// Transfers are pinned to the authority that accepted their bytes.
    #[serde(default)]
    pub owner_id: Option<String>,
}
impl ImportRequest {
    pub fn validate(&self) -> Result<(), String> {
        if matches!(self.destination, Destination::AudioSource { .. }) && self.paths.len() != 1 {
            return Err("Choose one audio source file".into());
        }
        if self
            .owner_id
            .as_ref()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_err())
        {
            return Err("Invalid asset owner identity".into());
        }
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
    AudioSource(String),
    Media(PathBuf),
    Subtitle(PathBuf),
    Audio(AudioEffect),
    Timeline(Timeline),
    Scene(crate::projects::Scene),
    Effect(crate::platform::interop::WebControllerEffectDraft),
    EffectCollection(Vec<crate::platform::interop::WebControllerEffectDraft>),
}

/// Adapt the CURRENT PCController EffectDescriptor (also used by effects.json)
/// to the existing editor/save payload. These are two current purpose-specific
/// shapes, not historical schemas. Validation stays in the shared save command.
fn effect_definition(
    mut value: serde_json::Value,
) -> Result<crate::platform::interop::WebControllerEffectDraft, String> {
    if value.get("kind").and_then(|v| v.as_str()) == Some("sequence")
        && value.get("steps").is_some()
    {
        let mut properties = value
            .get("properties")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        if let Some(engine) = value.get("engine").and_then(|v| v.as_str()) {
            if !matches!(engine, "auto" | "host" | "mcu") {
                return Err("Invalid sequence execution mode".into());
            }
            properties
                .as_object_mut()
                .ok_or("Effect properties must be an object")?
                .insert("mode".into(), engine.into());
        }
        let program = serde_json::json!({"steps":value["steps"],"repeat_count":value.get("repeat_count").cloned().unwrap_or(1.into()),"repeat_interval_ms":value.get("repeat_interval_ms").cloned().unwrap_or(0.into()),"properties":properties});
        value
            .as_object_mut()
            .unwrap()
            .insert("program".into(), program);
    }
    let object = value
        .as_object_mut()
        .ok_or("Expected an effect definition")?;
    object.entry("category").or_insert_with(|| "Other".into());
    let effect: crate::platform::interop::WebControllerEffectDraft =
        serde_json::from_value(value).map_err(|e| e.to_string())?;
    crate::platform::interop::InteropCommand::SaveControllerEffect {
        effect: effect.clone(),
    }
    .validate()?;
    Ok(effect)
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
            let prepared = if let Some(effects) = value.get("effects").and_then(|v| v.as_array()) {
                if effects.is_empty() || effects.len() > MAX_FILES {
                    return Err("Choose a catalog containing 1–64 effect definitions".into());
                }
                Prepared::EffectCollection(
                    effects
                        .iter()
                        .cloned()
                        .map(effect_definition)
                        .collect::<Result<Vec<_>, _>>()?,
                )
            } else if value
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
            } else if value.get("kind").is_some()
                && (value.get("program").is_some() || value.get("steps").is_some())
            {
                Prepared::Effect(effect_definition(value)?)
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
    if let Prepared::EffectCollection(effects) = asset {
        return Ok(Prepared::EffectCollection(
            effects
                .into_iter()
                .map(
                    |effect| match resolve_dependencies(Prepared::Effect(effect), path)? {
                        Prepared::Effect(effect) => Ok(effect),
                        _ => unreachable!(),
                    },
                )
                .collect::<Result<Vec<_>, String>>()?,
        ));
    }
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
    for mut curve in copy.analog_tracks {
        if let Some(existing) = next
            .analog_tracks
            .iter_mut()
            .find(|track| track.channel == curve.channel)
        {
            if existing.locked {
                return Err(format!("PWM {} is locked", curve.channel));
            }
            if curve
                .keyframes
                .iter()
                .any(|p| existing.keyframes.iter().any(|q| q.time_ms == p.time_ms))
            {
                return Err(format!(
                    "PWM {} already has a point at an imported time",
                    curve.channel
                ));
            }
            existing.keyframes.append(&mut curve.keyframes);
            existing.keyframes.sort_by_key(|point| point.time_ms);
        } else {
            curve.armed = false;
            next.analog_tracks.push(curve);
        }
    }
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
    subtitles: Vec<(PathBuf, PathBuf)>,
    subtitle_started: Option<std::time::Instant>,
}
impl ImportState {
    pub(crate) fn busy(&self) -> bool {
        self.receiver.is_some()
            || self.audio_active.is_some()
            || !self.audio.is_empty()
            || !self.subtitles.is_empty()
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
fn review_summary(asset: &Prepared) -> Vec<String> {
    match asset {
        Prepared::Scene(scene) => scene
            .channels
            .iter()
            .map(|(key, value)| match value {
                crate::projects::ChannelValue::Boolean { on } => {
                    format!("{key} · {}", if *on { "On" } else { "Off" })
                }
                crate::projects::ChannelValue::Pwm { percent_milli } => {
                    format!("{key} · {:.3}%", f64::from(*percent_milli) / 1000.0)
                }
                crate::projects::ChannelValue::Motion { direction } => {
                    format!("{key} · {direction:?} (requires explicit mapping)")
                }
            })
            .chain(
                scene
                    .unsampled
                    .iter()
                    .map(|key| format!("{key} · Unsampled (unchanged)")),
            )
            .collect(),
        Prepared::Timeline(timeline) => timeline
            .instances
            .iter()
            .filter_map(|cue| {
                timeline
                    .templates
                    .iter()
                    .find(|e| e.id == cue.effect_id)
                    .map(|e| {
                        format!(
                            "{} · {} · {}",
                            crate::duration::format_timeline_time_ms(cue.start_time_ms, true),
                            e.name,
                            crate::duration::format_effect_duration(e.duration_ms)
                        )
                    })
            })
            .chain(timeline.analog_tracks.iter().map(|curve| {
                format!(
                    "{} · PWM {} · {} points",
                    curve.name,
                    curve.channel,
                    curve.keyframes.len()
                )
            }))
            .collect(),
        Prepared::Effect(effect) => vec![
            format!("{} · {}", effect.kind, effect.category),
            format!(
                "{} · {}",
                effect.id,
                crate::duration::format_effect_duration(effect.duration_ms)
            ),
        ],
        _ => vec![],
    }
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
        if request
            .owner_id
            .as_deref()
            .is_some_and(|id| id != crate::peer::instance_id())
        {
            return Err(
                "Asset session owner changed; retry the import in the selected session".into(),
            );
        }
        if self.asset_import.receiver.is_some()
            || self.asset_import.reviews.len() + self.asset_import.audio.len() + request.paths.len()
                > MAX_FILES
        {
            return Err("Finish the current asset import first".into());
        }
        let (tx, rx) = mpsc::channel();
        // Capture the immutable target before starting the worker; never resolve
        // a different session half way through an import.
        let client = crate::peer::client().cloned();
        if client.is_none() {
            crate::server::asset_upload::retain_paths(&request.paths);
        }
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("asset-import".into())
            .spawn(move || {
                let batch = if let Some(client) = client {
                    // A client path belongs to the client, never to its authority.
                    // Transfer bytes through the same bounded upload API as Web.
                    let result = crate::server::asset_upload::forward(&client, &request);
                    vec![(
                        "Remote asset import".into(),
                        result.map(|source| {
                            source.map_or(Prepared::Forwarded, Prepared::AudioSource)
                        }),
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
                                if matches!(request.destination, Destination::AudioSource { .. }) {
                                    if file_kind(Path::new(path)) == FileKind::Audio
                                        && Path::new(path).is_file()
                                    {
                                        Ok(Prepared::AudioSource(path.clone()))
                                    } else {
                                        Err("Choose an audio file for this sound effect".into())
                                    }
                                } else {
                                    prepare(Path::new(path))
                                },
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
        let mut changed = false;
        let result = self.asset_import.receiver.as_ref().map(|rx| rx.try_recv());
        if let Some(Ok(batch)) = result {
            changed = true;
            self.asset_import.receiver = None;
            // A mixed drop loads media before attaching its subtitle tracks.
            let total = batch
                .iter()
                .map(|(_, asset, _)| match asset {
                    Ok(Prepared::EffectCollection(effects)) => effects.len(),
                    _ => 1,
                })
                .sum::<usize>();
            if total + self.asset_import.reviews.len() > MAX_FILES {
                self.asset_error(
                    "This drop exceeds 64 pending asset definitions; import a smaller selection"
                        .into(),
                );
                self.publish_asset_snapshot();
                return;
            }
            let mut batch = batch
                .into_iter()
                .flat_map(|(name, asset, destination)| match asset {
                    Ok(Prepared::EffectCollection(effects)) => effects
                        .into_iter()
                        .map(|effect| {
                            (
                                format!("{name} · {}", effect.name),
                                Ok(Prepared::Effect(effect)),
                                destination.clone(),
                            )
                        })
                        .collect::<Vec<_>>(),
                    _ => vec![(name, asset, destination)],
                })
                .collect::<Vec<_>>();
            batch.sort_by_key(|(_, asset, destination)| {
                !(matches!(asset, Ok(Prepared::Media(_)))
                    || matches!(asset, Ok(Prepared::Audio(_)))
                        && *destination == Destination::Player)
            });
            let mut media_loaded = false;
            let mut requested_media = None;
            for (name, asset, mut destination) in batch {
                if matches!(&asset, Ok(Prepared::Scene(_) | Prepared::Timeline(_)))
                    && !matches!(destination, Destination::Timeline { .. })
                {
                    destination = Destination::Timeline {
                        start_time_ms: (self.playback_time.max(0.0) * 1000.0)
                            .round()
                            .min(MAX_TIME_MS as f64) as u64,
                        track_key: None,
                    };
                }
                match asset {
                    Ok(Prepared::Forwarded) => {}
                    Ok(Prepared::AudioSource(source)) => {
                        if let Destination::AudioSource { draft_id } = destination {
                            if self.show_effect_library_editor
                                && self.effect_library_draft.kind == "audio"
                                && self.effect_library_draft.id == draft_id.to_string()
                            {
                                match serde_json::from_str::<crate::mpv::sfx::AudioProgram>(
                                    &self.effect_library_draft.program_json,
                                ) {
                                    Ok(mut program) => {
                                        program.source = source;
                                        self.effect_library_draft.program_json =
                                            serde_json::to_string(&program).unwrap();
                                        self.effect_library_draft.duration_ms = 0;
                                    }
                                    Err(error) => self.asset_error(error.to_string()),
                                }
                            } else {
                                self.asset_error("The sound effect draft changed before the file was ready; drop it again".into());
                            }
                        }
                    }
                    Ok(Prepared::Audio(audio)) if destination != Destination::Player => {
                        self.asset_import.audio.push_back((audio, destination))
                    }
                    Ok(Prepared::Audio(audio)) => {
                        if requested_media.is_some() {
                            if self.pending_timeline_close_action.is_some() {
                                self.asset_error("Confirm the media change before appending additional playlist files".into());
                            } else if let Err(error) = self
                                .mpv
                                .command("loadfile", &[&audio.program.source, "append-play"])
                            {
                                self.asset_error(error.to_string());
                            }
                        } else {
                            requested_media = Some(PathBuf::from(&audio.program.source));
                            self.load_media_target(&audio.program.source);
                        }
                    }
                    Ok(Prepared::Effect(mut effect)) => {
                        // External definitions are copies. Never upsert over an
                        // unrelated live controller program by its exported ID.
                        effect.id = if effect.kind == "sequence" {
                            let mut used = self
                                .advertised_hardware()
                                .map(|caps| {
                                    caps.macros
                                        .iter()
                                        .map(|m| m.id)
                                        .collect::<std::collections::BTreeSet<_>>()
                                })
                                .unwrap_or_default();
                            for review in &self.asset_import.reviews {
                                if let Prepared::Effect(e) = &review.asset {
                                    if e.kind == "sequence" {
                                        if let Ok(id) = e.id.parse::<u64>() {
                                            used.insert(id);
                                        }
                                    }
                                }
                            }
                            if self.show_effect_library_editor {
                                if let Ok(id) = self.effect_library_draft.id.parse::<u64>() {
                                    used.insert(id);
                                }
                            }
                            let Some(id) = (0..=255u64).find(|id| !used.contains(id)) else {
                                self.asset_error("No free sequence slots; remove an unused effect before importing".into());
                                continue;
                            };
                            id.to_string()
                        } else {
                            uuid::Uuid::new_v4().to_string()
                        };
                        effect.reference.clear();
                        effect.is_new = true;
                        self.asset_import.reviews.push_back(Review {
                            id: uuid::Uuid::new_v4(),
                            name,
                            asset: Prepared::Effect(effect),
                            destination,
                        });
                    }
                    Ok(Prepared::Media(path)) => {
                        if matches!(destination, Destination::Library) {
                            self.asset_error(
                                "Video files belong in the player, not the Effects Library".into(),
                            );
                            continue;
                        }
                        if !media_loaded {
                            requested_media = Some(path.clone());
                            self.load_video_file(path);
                            media_loaded = true;
                        } else if self.pending_timeline_close_action.is_some() {
                            self.asset_error("Confirm the media change before appending additional playlist files".into());
                        } else if let Some(path) = path.to_str() {
                            let _ = self.mpv.command("loadfile", &[path, "append-play"]);
                        }
                    }
                    Ok(Prepared::Subtitle(path)) => {
                        if let Some(media) = requested_media
                            .clone()
                            .or_else(|| self.current_video_path.clone())
                        {
                            // loadfile is asynchronous. sub-add before FileLoaded
                            // would attach to the old movie or be discarded.
                            if requested_media.is_some() || !self.media_metadata_loaded {
                                self.asset_import.subtitles.push((media, path));
                                if self.pending_timeline_close_action.is_none() {
                                    self.asset_import
                                        .subtitle_started
                                        .get_or_insert_with(std::time::Instant::now);
                                }
                            } else {
                                self.attach_asset_subtitle(path);
                            }
                        } else {
                            self.asset_error("Load media before attaching a subtitle".into());
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
            changed = true;
            self.asset_import.receiver = None;
            self.asset_error("Asset import worker stopped".into());
        }
        if self.asset_import.audio_active.is_none() && !crate::mpv::sfx::importing() {
            if let Some((audio, destination)) = self.asset_import.audio.pop_front() {
                changed = true;
                let id = audio.id;
                match crate::mpv::sfx::import(audio) {
                    Ok(()) => self.asset_import.audio_active = Some((id, destination)),
                    Err(error) => self.asset_error(error),
                }
            }
        }
        if !self.asset_import.subtitles.is_empty() && self.pending_timeline_close_action.is_none() {
            self.asset_import
                .subtitle_started
                .get_or_insert_with(std::time::Instant::now);
        }
        if let Some(started) = self.asset_import.subtitle_started {
            let timeout = std::time::Duration::from_secs(60);
            if started.elapsed() >= timeout {
                self.asset_import.subtitles.clear();
                self.asset_import.subtitle_started = None;
                changed = true;
                self.asset_error(
                    "The movie did not finish loading; dropped subtitles were not attached".into(),
                );
            } else {
                ctx.request_repaint_after(timeout.saturating_sub(started.elapsed()));
            }
        }
        if changed {
            self.publish_asset_snapshot();
        }
        if self.asset_import.receiver.is_some() || !self.asset_import.audio.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
    fn attach_asset_subtitle(&mut self, path: PathBuf) {
        if let Err(error) = self
            .mpv
            .command("sub-add", &[&path.to_string_lossy(), "select"])
        {
            self.asset_error(format!("Could not attach subtitle: {error}"));
        } else {
            self.refresh_media_tracks();
        }
    }
    pub(crate) fn attach_loaded_asset_subtitles(&mut self) {
        let loaded = self
            .mpv
            .get_property::<String>("path")
            .ok()
            .map(PathBuf::from);
        if loaded.as_ref() != self.current_video_path.as_ref() {
            return;
        }
        self.asset_import.subtitle_started = None;
        let mut pending = Vec::new();
        for (media, path) in std::mem::take(&mut self.asset_import.subtitles) {
            if self.current_video_path.as_ref() == Some(&media) {
                self.attach_asset_subtitle(path);
            } else {
                pending.push((media, path));
            }
        }
        self.asset_import.subtitles = pending;
        self.publish_asset_snapshot();
    }
    pub(crate) fn cancel_asset_media_change(&mut self, path: &Path) {
        self.asset_import
            .subtitles
            .retain(|(media, _)| media != path);
        if self.asset_import.subtitles.is_empty() {
            self.asset_import.subtitle_started = None;
        }
        self.publish_asset_snapshot();
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
        let reviews=self.asset_import.reviews.iter().map(|r|serde_json::json!({"id":r.id,"name":r.name,"kind":match r.asset {Prepared::Timeline(_)=>"timeline",Prepared::Scene(_)=>"scene",Prepared::Effect(_)=>"effect",_=>"asset"},"destination":r.destination,"summary":review_summary(&r.asset),"effect":match &r.asset{Prepared::Effect(effect)=>serde_json::to_value(effect).unwrap_or_default(),_=>serde_json::Value::Null}})).collect::<Vec<_>>();
        let value = serde_json::json!({"busy":self.asset_import.busy(),"reviews":reviews});
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
        self.publish_asset_snapshot();
    }
    pub(crate) fn append_asset_fragment(&mut self, fragment: &Timeline, time: u64) -> Result<(), String> {
        if self.current_video_path.is_some() && !self.is_paused {
            return Err("Pause playback before adding an asset to the timeline".into());
        }
        let mut next = self.timeline.clone();
        let ids = append_timeline(&mut next, fragment, time)?;
        self.validate_asset_fragment_locks(fragment)?;
        self.undo_stack.push(self.snapshot_timeline());
        self.timeline = next;
        self.selected_instance_ids = ids.into_iter().collect();
        self.commit_timeline_edit();
        Ok(())
    }
    pub(crate) fn validate_asset_fragment_locks(&self, fragment: &Timeline) -> Result<(), String> {
        if fragment.analog_tracks.iter().any(|curve| {
            self.timeline
                .analog_tracks
                .iter()
                .any(|track| track.channel == curve.channel && track.locked)
        }) || fragment.templates.iter().any(|effect| {
            effect
                .target
                .primary_relay_id()
                .is_some_and(|id| self.track_locked.contains(&id))
                || effect.direct_control.as_ref().is_some_and(|direct| {
                    self.advertised_hardware().is_some_and(|hardware| {
                        hardware
                            .controls
                            .iter()
                            .any(|control| control.key == direct.control_key && control.locked)
                    })
                })
                || effect
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
                if self.current_video_path.is_some() && !self.is_paused {
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
        if self.pending_timeline_close_action.is_some()
            || crate::peer::client().is_some_and(|c| c.pending_timeline_change())
        {
            return;
        }
        let remote = crate::peer::client().and_then(|c| c.asset_review());
        let value = remote.clone().or_else(|| {
            SNAPSHOT
                .lock()
                .ok()?
                .as_ref()?
                .get("reviews")?
                .as_array()?
                .first()
                .cloned()
        });
        let Some(value) = value else {
            return;
        };
        let Some(id) = value
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|v| uuid::Uuid::parse_str(v).ok())
        else {
            return;
        };
        let kind = value["kind"].as_str().unwrap_or_default();
        let time = value["destination"]["start_time_ms"].as_u64().unwrap_or(0);
        let mut action = None;
        let modal=eframe::egui::Modal::new(eframe::egui::Id::new("external-asset-review")).show(ctx,|ui|{
            ui.set_width((ctx.content_rect().width()-64.0).clamp(200.0,520.0));
            ui.horizontal(|ui|{
                ui.heading(format!("{} {}",crate::ui::icons::FOLDER_OPEN,self.tr("Import asset")));
                ui.with_layout(eframe::egui::Layout::right_to_left(eframe::egui::Align::Center),|ui|{if crate::ui::dialog::close_button(ui,&self.tr("Cancel")).clicked(){action=Some(false);}});
            });
            ui.label(value["name"].as_str().unwrap_or_default());ui.separator();
            if kind=="effect" {ui.label(self.tr("Review in Effect properties, then Save or Publish. Drag the saved effect from the library to reuse it."));}
            else {ui.label(format!("{} · {}",self.tr("Add to timeline"),crate::duration::format_timeline_time_ms(time,true)));ui.weak(self.tr("Existing cues and playback position are preserved. No live scene is run."));}
            eframe::egui::ScrollArea::vertical().max_height((ctx.content_rect().height()*0.45).max(80.0)).show(ui,|ui|{
                for line in value["summary"].as_array().into_iter().flatten().filter_map(|v|v.as_str()){ui.label(line);}
            });
            ui.add_space(12.0);ui.horizontal(|ui|{if ui.button(self.tr("Cancel")).clicked(){action=Some(false);}if ui.button(self.tr(if kind=="effect"{"Open properties"}else{"Add to timeline"})).clicked(){action=Some(true);}});
        });
        if modal.should_close() {
            action = Some(false);
        }
        if let Some(accept) = action {
            if remote.is_some() {
                if accept && kind == "effect" {
                    if let (Ok(effect), Ok(destination)) = (
                        serde_json::from_value(value["effect"].clone()),
                        serde_json::from_value(value["destination"].clone()),
                    ) {
                        self.asset_import.reviews.push_back(Review {
                            id,
                            name: value["name"].as_str().unwrap_or_default().into(),
                            asset: Prepared::Effect(effect),
                            destination,
                        });
                        if let Err(error) = self.review_asset(ctx, id, true) {
                            self.asset_import.reviews.retain(|r| r.id != id);
                            self.asset_error(error);
                            return;
                        }
                    } else {
                        self.asset_error("Invalid imported effect from the authority".into());
                        return;
                    }
                }
                self.apply_interop_command(
                    ctx,
                    crate::platform::interop::InteropCommand::ReviewAsset {
                        id,
                        accept: accept && kind != "effect",
                    },
                    "assets",
                );
            } else if let Err(error) = self.review_asset(ctx, id, accept) {
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
pub fn paint_drop_target(ui: &eframe::egui::Ui, rect: eframe::egui::Rect, label: &str) {
    use eframe::egui;
    let hovering = ui.input(|input| {
        !input.raw.hovered_files.is_empty()
            && input.pointer.latest_pos().is_some_and(|p| rect.contains(p))
    });
    if hovering {
        let color = ui.visuals().selection.stroke.color;
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        painter.rect_filled(rect, 4.0, color.linear_multiply(0.12));
        painter.rect_stroke(
            rect.shrink(1.0),
            4.0,
            egui::Stroke::new(2.0, color),
            egui::StrokeKind::Inside,
        );
        let paths=ui.input(|i|i.raw.hovered_files.iter().filter_map(|f|f.path.clone()).collect::<Vec<_>>());
        let feedback=drop_feedback(&paths,&native_destination(ui.ctx()));
        let label=if paths.is_empty(){label.to_string()}else{format!("{} {}",crate::ui::icons::named_control_icon(&feedback.icon).unwrap_or(crate::ui::icons::FOLDER_OPEN),feedback.message)};
        if !label.is_empty() {
            painter.text(
                rect.left_top() + egui::vec2(12.0, 12.0),
                egui::Align2::LEFT_TOP,
                &label,
                egui::FontId::proportional(14.0),
                ui.visuals().text_color(),
            );
        }
    }
}
pub fn native_destination(ctx: &eframe::egui::Context) -> Destination {
    let Some(pointer) = ctx.pointer_latest_pos() else {
        return Destination::Player;
    };
    native_destination_at(ctx,pointer)
}
pub fn native_destination_at(ctx:&eframe::egui::Context,pointer:eframe::egui::Pos2)->Destination {
    if let Some((rect, draft_id)) = ctx.data(|d| {
        d.get_temp::<(eframe::egui::Rect, uuid::Uuid)>(eframe::egui::Id::new(
            "external-assets-audio-source",
        ))
    }) && rect.contains(pointer)
    {
        return Destination::AudioSource { draft_id };
    }
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

#[derive(Clone,Debug,PartialEq)]
pub struct DropFeedback {pub icon:String,pub message:String}
pub fn drop_feedback(paths:&[PathBuf],destination:&Destination)->DropFeedback {
    if paths.len()!=1 {return DropFeedback{icon:"files".into(),message:format!("Import {} assets",paths.len())};}
    let path=&paths[0];
    let name=path.file_name().unwrap_or_default().to_string_lossy().chars().filter(|c|!c.is_control()).take(120).collect::<String>();
    let (icon,action)=match (destination,file_kind(path)) {
        (Destination::AudioSource{..},FileKind::Audio)=>("speaker-high","Use audio source"),
        (Destination::Player,FileKind::Audio)=>("play","Play"),
        (_,FileKind::Subtitle)=>("subtitles","Load subtitle"),
        (_,FileKind::Audio)=>("speaker-high","Add SFX"),
        (_,FileKind::Document)=>("folder-open","Review asset"),
        (_,FileKind::Media)=>("play","Play"),
    };
    DropFeedback{icon:icon.into(),message:format!("{action} {name}")}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn drop_feedback_matches_import_action_and_does_not_expose_parent_paths(){
        for (path,action,icon) in [("C:/private/movie.mkv","Play movie.mkv","play"),("C:/private/sub.srt","Load subtitle sub.srt","subtitles"),("C:/private/tone.wav","Play tone.wav","play"),("C:/private/effect.json","Review asset effect.json","folder-open")] {
            let feedback=drop_feedback(&[path.into()],&Destination::Player);assert_eq!(feedback.message,action);assert_eq!(feedback.icon,icon);
        }
        assert_eq!(drop_feedback(&["one.mkv".into(),"two.srt".into()],&Destination::Player).message,"Import 2 assets");
        assert_eq!(drop_feedback(&["tone.wav".into()],&Destination::Library).message,"Add SFX tone.wav");
    }
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
                destination: Default::default(),
                owner_id: None,
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
    #[test]
    fn curve_import_merges_channel_without_duplicate_outputs_or_partial_mutation() {
        use crate::four_d::curve::{AnalogTrack, Interpolation, Keyframe};
        let mut curve = AnalogTrack::new("Wind", 0);
        curve.keyframes = vec![Keyframe::new(429, 0.5, Interpolation::Linear)];
        let fragment = Timeline {
            analog_tracks: vec![curve.clone()],
            ..Default::default()
        };
        let mut target = fragment.clone();
        append_timeline(&mut target, &fragment, 10000).unwrap();
        assert_eq!(target.analog_tracks.len(), 1);
        assert_eq!(
            target.analog_tracks[0]
                .keyframes
                .iter()
                .map(|p| p.time_ms)
                .collect::<Vec<_>>(),
            vec![429, 10429]
        );
        let before = target.clone();
        assert!(append_timeline(&mut target, &fragment, 10000).is_err());
        assert_eq!(target, before);
        target.analog_tracks[0].locked = true;
        assert!(append_timeline(&mut target, &fragment, 20000).is_err());
    }
    #[test]
    fn actual_native_drop_geometry_accounts_for_pan_zoom_and_header() {
        use eframe::egui;
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 600.0),
                )),
                events: vec![egui::Event::PointerMoved(egui::pos2(200.0, 160.0))],
                ..Default::default()
            },
            |_| {
                ctx.data_mut(|d| {
                    d.insert_temp(
                        egui::Id::new("external-assets-timeline"),
                        TimelineDropArea {
                            viewport: egui::Rect::from_min_max(
                                egui::pos2(100.0, 100.0),
                                egui::pos2(800.0, 500.0),
                            ),
                            content_origin: egui::pos2(-842.9, 100.0),
                            zoom: 100.0,
                            row_height: 40.0,
                            keys: vec!["hardware:relay.1".into(), "controller-effect:audio".into()],
                        },
                    )
                });
                assert_eq!(
                    native_destination(&ctx),
                    Destination::Timeline {
                        start_time_ms: 10429,
                        track_key: Some("controller-effect:audio".into())
                    }
                );
            },
        );
        output.textures_delta.clear();
    }
    #[test]
    fn rejected_scene_does_not_partially_append_or_seek() {
        let ctx = eframe::egui::Context::default();
        let mut app = PealayerApp::default();
        app.is_paused = true;
        app.playback_time = 14.429;
        let original = app.timeline.clone();
        let id = uuid::Uuid::new_v4();
        app.asset_import.reviews.push_back(Review {
            id,
            name: "Scene".into(),
            asset: Prepared::Scene(crate::projects::Scene {
                name: "Scene".into(),
                board_identity: "another-board".into(),
                ..Default::default()
            }),
            destination: Destination::Timeline {
                start_time_ms: 10429,
                track_key: None,
            },
        });
        assert!(app.review_asset(&ctx, id, true).is_err());
        assert_eq!(app.timeline, original);
        assert_eq!(app.playback_time, 14.429);
        assert!(app.asset_import.awaiting_review());
        app.review_asset(&ctx, id, false).unwrap();
        assert!(!app.asset_import.awaiting_review());
    }
    #[test]
    fn document_reader_reuses_scene_contract_and_rejects_unknown_or_truncated_json() {
        let path =
            std::env::temp_dir().join(format!("pealayer-asset-{}.json", uuid::Uuid::new_v4()));
        let scene = crate::projects::Scene {
            name: "Preset".into(),
            channels: std::collections::BTreeMap::from([(
                "relay.1".into(),
                crate::projects::ChannelValue::Boolean { on: true },
            )]),
            ..Default::default()
        };
        let doc = crate::projects::Document::Scene(scene.clone());
        std::fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
        match prepare(&path).unwrap() {
            Prepared::Scene(actual) => assert_eq!(actual, scene),
            _ => panic!("expected existing scene format"),
        }
        std::fs::write(&path, b"{\"kind\":\"unknown\"}").unwrap();
        assert!(prepare(&path).is_err());
        std::fs::write(&path, b"{").unwrap();
        assert!(prepare(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn asset_rpc_uses_the_same_typed_destination_and_rejects_invalid_commands() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"pealayer.assets.import","params":{"paths":["sound.wav"],"destination":{"surface":"timeline","start_time_ms":10429,"track_key":"controller-effect:audio"}}}"#;
        let (_, command) = crate::platform::interop::parse_interop_request(json).unwrap();
        let crate::platform::interop::InteropCommand::ImportAssets { request } = command else {
            panic!("wrong command")
        };
        assert_eq!(request.destination.time(), 10429);
        assert_eq!(request.paths, vec!["sound.wav"]);
        assert!(
            crate::platform::interop::parse_interop_request(&json.replace("10429", "-1")).is_err()
        );
        assert!(
            crate::platform::interop::parse_interop_request(&json.replace("10429", "\"bad\""))
                .is_err()
        );
        for decision in ["save", "discard", "cancel"] {
            assert!(crate::platform::interop::parse_interop_request(&format!(r#"{{"method":"pealayer.timeline.resolve_pending","params":{{"decision":"{decision}"}}}}"#)).is_ok());
        }
        assert!(crate::platform::interop::parse_interop_request(r#"{"method":"pealayer.timeline.resolve_pending","params":{"decision":"overwrite"}}"#).is_err());
    }
    #[test]
    fn current_controller_descriptor_preserves_steps_repeats_and_properties() {
        let value = serde_json::json!({"id":"7","reference":"effect:7","name":"Light / dark","category":"Lighting","kind":"sequence","engine":"host","duration_ms":1000,"steps":[{"at_us":0,"kind":"relay","target":5,"value":1}],"repeat_count":3,"repeat_interval_ms":500,"properties":{"color":"#FFAA00","label":"Lamp"}});
        let effect = effect_definition(value).unwrap();
        assert_eq!(effect.program["repeat_count"], 3);
        assert_eq!(effect.program["repeat_interval_ms"], 500);
        assert_eq!(effect.program["properties"]["mode"], "host");
        assert_eq!(effect.program["properties"]["color"], "#FFAA00");
        assert_eq!(effect.program["steps"][0]["target"], 5);
        let path =
            std::env::temp_dir().join(format!("pealayer-catalog-{}.json", uuid::Uuid::new_v4()));
        std::fs::write(
            &path,
            serde_json::to_vec(
                &serde_json::json!({"effects":[effect],"groups":{"Lighting":{"icon":"lightbulb"}}}),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(
            matches!(prepare(&path).unwrap(),Prepared::EffectCollection(effects) if effects.len()==1)
        );
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn no_media_import_is_offline_authoring_not_playing() {
        let mut app = PealayerApp::default();
        app.is_paused = false;
        let original_time = app.playback_time;
        let effect = Effect::new("Marker".into(), "sparkle".into(), 1000, vec![]);
        let fragment = Timeline {
            instances: vec![EffectInstance::new(effect.id, 0)],
            templates: vec![effect],
            ..Default::default()
        };
        app.append_asset_fragment(&fragment, 10429).unwrap();
        assert_eq!(app.timeline.instances[0].start_time_ms, 10429);
        assert_eq!(app.playback_time, original_time);
    }
    #[test]
    fn native_review_renders_actual_values_and_never_mutates_on_open() {
        use eframe::egui;
        let mut app = PealayerApp::default();
        app.is_paused = true;
        app.playback_time = 14.429;
        let original = app.timeline.clone();
        let review = Review {
            id: uuid::Uuid::new_v4(),
            name: "Scene.json".into(),
            asset: Prepared::Scene(crate::projects::Scene {
                name: "Preset".into(),
                channels: std::collections::BTreeMap::from([
                    (
                        "relay.5".into(),
                        crate::projects::ChannelValue::Boolean { on: true },
                    ),
                    (
                        "pwm.0".into(),
                        crate::projects::ChannelValue::Pwm {
                            percent_milli: 35000,
                        },
                    ),
                ]),
                ..Default::default()
            }),
            destination: Destination::Timeline {
                start_time_ms: 10429,
                track_key: None,
            },
        };
        app.asset_import.reviews.push_back(review);
        app.publish_asset_snapshot();
        for width in [390.0, 768.0, 1440.0] {
            let ctx = egui::Context::default();
            // Centered popup/scroll geometry settles after the sizing pass.
            for _ in 0..2 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 850.0),
                        )),
                        ..Default::default()
                    },
                    |_| app.draw_asset_review(&ctx),
                );
                output.textures_delta.clear();
            }
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 850.0),
                    )),
                    ..Default::default()
                },
                |_| app.draw_asset_review(&ctx),
            );
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| {
                    if let egui::epaint::Shape::Text(t) = &shape.shape {
                        Some(t.galley.job.text.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert!(
                text.iter().any(|t| t.contains("pwm.0 · 35.000%")),
                "Missing actual scene values: {text:?}"
            );
            assert_eq!(app.timeline, original);
            assert_eq!(app.playback_time, 14.429);
            assert!(app.asset_import.awaiting_review());
            output.textures_delta.clear();
        }
        app.asset_import.reviews.clear();
        app.publish_asset_snapshot();
    }
    #[test]
    fn mixed_drop_waits_for_unsaved_decision_and_cancel_preserves_work() {
        let directory =
            std::env::temp_dir().join(format!("pealayer-mixed-assets-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let movie = directory.join("new.mkv");
        let subtitle = directory.join("captions.srt");
        std::fs::write(&movie, b"not decoded: dirty timeline must block loadfile").unwrap();
        std::fs::write(&subtitle, b"1\n00:00:00,000 --> 00:00:01,000\nTest\n").unwrap();
        let mut app = PealayerApp::default();
        let ctx = eframe::egui::Context::default();
        let effect = Effect::new("Work".into(), "sparkle".into(), 1000, vec![]);
        app.timeline
            .instances
            .push(EffectInstance::new(effect.id, 0));
        app.timeline.templates.push(effect);
        app.playback_time = 14.429;
        let before = app.timeline.clone();
        app.import_assets(
            ImportRequest {
                paths: vec![
                    subtitle.to_string_lossy().into(),
                    movie.to_string_lossy().into(),
                ],
                destination: Destination::Player,
                owner_id: None,
            },
            &ctx,
        )
        .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while app.asset_import.receiver.is_some() && std::time::Instant::now() < deadline {
            app.process_asset_imports(&ctx);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(
            app.pending_timeline_close_action,
            Some(crate::app::PendingTimelineCloseAction::SwitchMedia(
                movie.clone()
            ))
        );
        assert_eq!(app.asset_import.subtitles, vec![(movie.clone(), subtitle)]);
        assert!(app.asset_import.subtitle_started.is_none());
        app.resolve_timeline_close(&ctx, crate::app::TimelineCloseDecision::Cancel)
            .unwrap();
        assert_eq!(app.timeline, before);
        assert_eq!(app.playback_time, 14.429);
        assert!(!app.asset_import.busy());
        assert!(app.pending_timeline_close_action.is_none());
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn acknowledged_asset_replica_is_not_an_unsaved_consumer_sidecar() {
        let mut app=PealayerApp::default();app.current_video_path=Some("fixture.mkv".into());
        let effect=Effect::new("Imported".into(),"sparkle".into(),1000,vec![]);app.timeline.instances.push(EffectInstance::new(effect.id,10429));app.timeline.templates.push(effect);
        app.acknowledge_peer_authoring_snapshot(true,true);assert!(!app.is_timeline_dirty());
        app.timeline.instances[0].start_time_ms=11000;
        app.acknowledge_peer_authoring_snapshot(false,false);assert!(app.is_timeline_dirty(),"unchanged server snapshots must not erase pending local edits");
        app.acknowledge_peer_authoring_snapshot(true,false);assert!(!app.is_timeline_dirty());
    }
}
