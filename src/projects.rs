//! Portable authoring documents. Loading is data-only: never a hardware command.
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, BTreeSet}, io::{Read, Write}, path::{Path, PathBuf}};
use crate::four_d::models::Timeline;

pub const MAX_DOCUMENT_BYTES: u64 = 32 * 1024 * 1024;
// Exact integer milliseconds, within JavaScript's safe integer range too.
pub const MAX_TIME_MS: u64 = 3_155_760_000_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChannelValue {
    Boolean { on: bool },
    Pwm { percent_milli: u32 },
    Motion { direction: MotionDirection },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionDirection { Stop, Up, Down }

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub name: String,
    pub board_identity: String,
    pub channels: BTreeMap<String, ChannelValue>,
    /// Unknown/unsampled channels are not invented as Off.
    pub unsampled: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TimelineView {
    pub position_ms: u64,
    pub zoom: f32,
    pub scroll: [f32; 2],
    pub selected_cues: BTreeSet<uuid::Uuid>,
    pub selected_curve_points: BTreeSet<(uuid::Uuid, usize)>,
    pub selected_keyframe: Option<uuid::Uuid>,
}
impl Default for TimelineView {
    fn default() -> Self { Self { position_ms: 0, zoom: 100.0, scroll: [0.0; 2], selected_cues: BTreeSet::new(), selected_curve_points: BTreeSet::new(), selected_keyframe: None } }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineDocument {
    pub timeline: Timeline,
    pub view: TimelineView,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Media {
    pub target: String,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub paused: bool,
    pub speed: f64,
    pub video_track: String,
    pub audio_track: String,
    pub subtitle_track: String,
    pub information: crate::media_info::MediaFileInfo,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Components { pub scene: bool, pub timeline: bool, pub media: bool, pub workspace: bool }
impl Default for Components { fn default() -> Self { Self { scene: true, timeline: true, media: true, workspace: true } } }

/// Exactly one embedded value or a sibling JSON document, never two competing sources.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum Part<T> { Embedded { value: T }, File { path: String } }
impl<T> Part<T> { pub fn embedded(value: T) -> Self { Self::Embedded { value } } }

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub title: String,
    pub notes: String,
    pub load: Components,
    pub scene: Option<Part<Scene>>,
    pub timeline: Option<Part<TimelineDocument>>,
    pub media: Option<Part<Media>>,
    pub workspace: Option<Part<crate::config::WorkspaceProfile>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Document {
    #[serde(rename = "pealayer-project")] Project(Project),
    #[serde(rename = "pealayer-scene")] Scene(Scene),
    #[serde(rename = "pealayer-timeline")] Timeline(TimelineDocument),
    #[serde(rename = "pealayer-media")] Media(Media),
    #[serde(rename = "pealayer-workspace")] Workspace(crate::config::WorkspaceProfile),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag="operation", rename_all="snake_case", deny_unknown_fields)]
pub enum Request {
    Snapshot,
    Read { path:String },
    Write { path:String, document:Document },
    Load { path:String, #[serde(default)] components:Option<Components>, #[serde(default)] discard_unsaved:bool },
    Stage { document:Document, #[serde(default)] base_path:String, #[serde(default)] components:Option<Components>, #[serde(default)] discard_unsaved:bool },
    CaptureScene,
    ApplyScene { scene:Scene, confirm:bool },
}

pub(crate) fn rpc_payload(payload:&str,web:bool)->Option<String> {
    let value:serde_json::Value=serde_json::from_str(payload).ok()?;
    let request=if value.get("method").and_then(|v|v.as_str())==Some("pealayer.documents") {value.get("params").cloned().unwrap_or_default()}
        else if value.get("command").and_then(|v|v.as_str())==Some("documents") {value.get("request").cloned().unwrap_or_default()} else {return None};
    let id=value.get("id").cloned();
    let result=(||{
        let config=crate::platform::interop::get_live_config();
        if web && (!config.web_allow_file_access || !config.web_allow_control || !config.web_allow_configuration) {return Err("Document operations require file, configuration and control permissions".into());}
        let request:Request=serde_json::from_value(request).map_err(|e|format!("Invalid document operation: {e}"))?;
        let request=serde_json::to_value(request).map_err(|e|e.to_string())?;
        if let Some(client)=crate::peer::client() {client.post("/api/peer/documents",&request)} else {crate::peer::gui_request("/api/peer/documents",request)}
    })();
    Some(match result {Ok(value)=>crate::platform::interop::format_interop_response(id,&value),Err(error)=>crate::platform::interop::format_interop_error(id,-32000,&error)})
}

fn bounded_text(value: &str, max: usize, name: &str) -> Result<(), String> {
    if value.len() > max || value.contains('\0') { Err(format!("Invalid {name}")) } else { Ok(()) }
}
fn time(value: u64) -> Result<(), String> { if value > MAX_TIME_MS { Err("Time exceeds the exact supported range".into()) } else { Ok(()) } }

impl Scene {
    pub fn validate(&self) -> Result<(), String> {
        bounded_text(&self.name, 1024, "scene name")?;
        bounded_text(&self.board_identity, 512, "board identity")?;
        if self.channels.len() > 1024 || self.unsampled.len() > 1024 { return Err("Scene has too many channels".into()); }
        for (key, value) in &self.channels {
            if key.is_empty() || key.len()>128 || !key.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c)) { return Err("Invalid channel key".into()); }
            if matches!(value, ChannelValue::Pwm { percent_milli } if *percent_milli > 100_000) { return Err(format!("PWM value for {key} must be between 0 and 100 percent")); }
        }
        Ok(())
    }
    pub fn capture(cap: &crate::four_d::controller::HardwareCapabilities) -> Self {
        let mut scene = Self { name: cap.board_name.clone(), board_identity: cap.board_profile.as_ref().map(|p|p.board_identity.clone()).unwrap_or_default(), ..Self::default() };
        for control in &cap.controls {
            let value = if let Some(output) = cap.relays.iter().find(|o|o.key==control.key) {
                cap.board_connected.then_some(ChannelValue::Boolean { on: cap.active_relays.contains(&output.id) })
            } else if let Some(output) = cap.pwm_channels.iter().find(|o|o.key==control.key) {
                cap.telemetry.pwm_logical_percent_milli.get(output.id as usize).copied().flatten().map(|percent_milli|ChannelValue::Pwm { percent_milli })
            } else if control.kind == "motion" {
                cap.motion.as_ref().and_then(|m|match control.key.as_str() {"seat.a"=>Some(&m.left),"seat.b"=>Some(&m.right),_=>None})
                    .and_then(|side|match side.applied.as_str() {"up"=>Some(MotionDirection::Up),"down"=>Some(MotionDirection::Down),"stop"|"stopped"=>Some(MotionDirection::Stop),_=>None}).map(|direction|ChannelValue::Motion {direction})
            } else { None };
            if let Some(value)=value { scene.channels.insert(control.key.clone(),value); } else { scene.unsampled.push(control.key.clone()); }
        }
        scene
    }
}

impl TimelineDocument {
    pub fn validate(&self) -> Result<(), String> {
        let t=&self.timeline;
        time(self.view.position_ms)?;
        if !self.view.zoom.is_finite() || !(1.0..=10000.0).contains(&self.view.zoom) || self.view.scroll.iter().any(|v|!v.is_finite() || !(0.0..=1e12).contains(v)) { return Err("Invalid timeline view".into()); }
        if t.instances.len()>10_000 || t.templates.len()>10_000 || t.keyframes.len()>10_000 || t.analog_tracks.len()>1024 { return Err("Timeline exceeds authoring limits".into()); }
        let mut ids=BTreeSet::new();
        for effect in &t.templates {
            if !ids.insert(effect.id) { return Err("Duplicate effect ID".into()); }
            time(effect.duration_ms)?;
            bounded_text(&effect.name,4096,"effect name")?;
            if effect.actions.len()>100_000 { return Err("Too many effect actions".into()); }
            for action in &effect.actions { time(action.offset_ms)?; }
            if let Some(direct)=&effect.direct_control {
                if direct.value_basis_points>10000 || direct.end_value_basis_points>10000 {return Err("Direct channel value exceeds 100 percent".into());}
            }
        }
        let mut instance_ids=BTreeSet::new();
        for cue in &t.instances {
            time(cue.start_time_ms)?;
            if !instance_ids.insert(cue.id) || !ids.contains(&cue.effect_id) { return Err("Duplicate cue ID or missing effect template".into()); }
            let duration=t.templates.iter().find(|e|e.id==cue.effect_id).unwrap().duration_ms;
            time(cue.start_time_ms.checked_add(duration).ok_or("Cue time overflow")?)?;
        }
        let mut guide_ids=BTreeSet::new();
        for guide in &t.keyframes {time(guide.time_ms)?; if !guide_ids.insert(guide.id) {return Err("Duplicate keyframe ID".into());}}
        let mut curve_ids=BTreeSet::new();
        for curve in &t.analog_tracks {
            if !curve_ids.insert(curve.id) || curve.channel>=16 || curve.keyframes.len()>100_000 { return Err("Invalid analog track".into()); }
            let mut previous=None;
            for point in &curve.keyframes {
                time(point.time_ms)?;
                if !point.value.is_finite() || !(0.0..=1.0).contains(&point.value) || previous.is_some_and(|p|p>=point.time_ms) { return Err("Curve points must be finite, bounded and strictly time ordered".into()); }
                previous=Some(point.time_ms);
            }
        }
        if !self.view.selected_cues.is_subset(&instance_ids) || self.view.selected_keyframe.is_some_and(|id|!guide_ids.contains(&id)) || self.view.selected_curve_points.iter().any(|(id,index)|!t.analog_tracks.iter().any(|c|c.id==*id && *index<c.keyframes.len())) {return Err("Selection references a missing cue or keyframe".into());}
        Ok(())
    }
}
impl Media {
    pub fn validate(&self) -> Result<(), String> {
        bounded_text(&self.target,32768,"media target")?;
        if self.target.trim().is_empty() {return Err("Media target is required".into());}
        time(self.position_ms)?; time(self.duration_ms)?;
        if !self.speed.is_finite() || !(0.01..=100.0).contains(&self.speed) { return Err("Invalid playback speed".into()); }
        for track in [&self.video_track,&self.audio_track,&self.subtitle_track] {
            if !matches!(track.as_str(),"auto"|"no") && track.parse::<u32>().is_err() {return Err("Media track must be auto, no or an mpv track ID".into());}
        }
        Ok(())
    }
}
pub fn validate_workspace(workspace: &crate::config::WorkspaceProfile) -> Result<(), String> {
    if let Some(layout)=&workspace.dock_layout {crate::ui::layout::decode_workspace(layout)?;}
    if let Some(memory)=&workspace.egui_memory {serde_json::from_str::<eframe::egui::Memory>(memory).map_err(|e|format!("Invalid workspace: {e}"))?;}
    Ok(())
}
impl Document {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Scene(s)=>s.validate(), Self::Timeline(t)=>t.validate(), Self::Media(m)=>m.validate(), Self::Workspace(w)=>validate_workspace(w),
            Self::Project(p)=>{
                bounded_text(&p.title,1024,"project title")?; bounded_text(&p.notes,65536,"project notes")?;
                macro_rules! part {($part:expr,$variant:ident)=>{if let Some(part)=$part {match part {Part::Embedded {value}=>Document::$variant(value.clone()).validate()?,Part::File {path}=>{bounded_text(path,32768,"component path")?;if path.trim().is_empty() {return Err("Component path is required".into());}}}}};}
                part!(&p.scene,Scene);part!(&p.timeline,Timeline);part!(&p.media,Media);part!(&p.workspace,Workspace);
                Ok(())
            }
        }
    }
    pub fn read(path: &Path) -> Result<Self,String> {
        let mut bytes=Vec::new();
        std::fs::File::open(path).map_err(|e|format!("Open {}: {e}",path.display()))?.take(MAX_DOCUMENT_BYTES+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
        if bytes.len() as u64>MAX_DOCUMENT_BYTES {return Err("Document exceeds 32 MiB".into());}
        let value:Self=serde_json::from_slice(&bytes).map_err(|e|format!("Invalid Pealayer document: {e}"))?;
        value.validate()?; Ok(value)
    }
    pub fn write(&self,path:&Path)->Result<(),String> {
        self.validate()?;
        let bytes=serde_json::to_vec_pretty(self).map_err(|e|e.to_string())?;
        if bytes.len() as u64>MAX_DOCUMENT_BYTES {return Err("Document exceeds 32 MiB".into());}
        atomic_write(path,&bytes)
    }
}

pub fn atomic_write(path:&Path,bytes:&[u8])->Result<(),String> {
    let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name=path.file_name().ok_or("A file name is required")?.to_string_lossy();
    let temp=parent.join(format!(".{name}.{}.tmp",uuid::Uuid::new_v4()));
    let result=(||{
        let mut file=std::fs::OpenOptions::new().create_new(true).write(true).open(&temp).map_err(|e|e.to_string())?;
        file.write_all(bytes).and_then(|_|file.write_all(b"\n")).and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;
        drop(file);
        crate::config::replace_file(&temp,path)
    })();
    if result.is_err() {let _=std::fs::remove_file(&temp);}
    result
}

pub fn resolve_path(base:&Path,path:&str)->PathBuf {
    let path=PathBuf::from(path);
    if path.is_absolute() {path} else {base.parent().unwrap_or(Path::new(".")).join(path)}
}

/// Resolve and validate every selected file before changing the live session.
pub fn resolve(project:&Project,path:&Path,load:Components)->Result<Project,String> {
    Document::Project(project.clone()).validate()?;
    let mut result=project.clone(); result.load=load;
    macro_rules! resolve {($field:ident,$variant:ident)=>{
        if load.$field {
            if let Some(Part::File {path:relative})=&project.$field {
                let absolute=resolve_path(path,relative);
                result.$field=Some(Part::embedded(match Document::read(&absolute)? {Document::$variant(value)=>value,_=>return Err(format!("Wrong document kind for {}",stringify!($field)))}));
            }
        } else {result.$field=None;}
    };}
    resolve!(scene,Scene); resolve!(timeline,Timeline); resolve!(media,Media); resolve!(workspace,Workspace);
    if let Some(Part::Embedded {value:media})=&mut result.media {
        if !crate::media::is_remote_media_target(&media.target) {
            let base=match &project.media {Some(Part::File {path:p})=>resolve_path(path,p),_=>path.to_path_buf()};
            media.target=resolve_path(&base,&media.target).to_string_lossy().into_owned();
            if !Path::new(&media.target).is_file() {return Err("Project media is missing; choose another path or disable loading media".into());}
        }
    }
    Ok(result)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Discovery {
    pub enabled:bool, pub remembered:bool, pub beside_media:bool, pub search_paths:bool,
    /// One folder per line, portable through the shared Preferences text control.
    pub folders:String,
    pub associations:BTreeMap<String,String>,
}
impl Default for Discovery {fn default()->Self {Self {enabled:true,remembered:true,beside_media:true,search_paths:true,folders:String::new(),associations:BTreeMap::new()}}}
impl Discovery {
    pub fn candidates(&self,media:&str)->Vec<PathBuf> {
        if !self.enabled {return vec![];}
        let mut paths=Vec::new();
        if self.remembered {if let Some(path)=self.associations.get(media) {paths.push(PathBuf::from(path));}}
        let local=!crate::media::is_remote_media_target(media);
        let file=if local {PathBuf::from(media)} else {url::Url::parse(media).ok().and_then(|u|u.path_segments()?.next_back().map(PathBuf::from)).unwrap_or_default()};
        if self.beside_media && local {paths.push(file.with_extension("pealayer.json"));}
        if self.search_paths {if let Some(name)=file.with_extension("pealayer.json").file_name() { for folder in self.folders.lines().map(str::trim).filter(|s|!s.is_empty()).take(32) {paths.push(Path::new(folder).join(name));}}}
        let mut seen=BTreeSet::new(); paths.retain(|p|seen.insert(p.clone()) && p.is_file()); paths
    }
}
