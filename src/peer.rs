//! One authoritative Pealayer session, with memory-only consumer state.
//! Mutations are sent once: a lost acknowledgement must never replay actuation.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant};

const ACTIVE_SESSION_POLL_INTERVAL: Duration = Duration::from_millis(100);
const IDLE_SESSION_POLL_INTERVAL: Duration = Duration::from_secs(1);

fn session_poll_interval(paused: bool) -> Duration {
    if paused {
        IDLE_SESSION_POLL_INTERVAL
    } else {
        ACTIVE_SESSION_POLL_INTERVAL
    }
}

pub const USER_AGENT: &str = concat!("Pealayer/", env!("CARGO_PKG_VERSION"), " peer-client");
static CLIENT: OnceLock<Arc<Client>> = OnceLock::new();
static INSTANCE: OnceLock<String> = OnceLock::new();
static SERVER: OnceLock<ServerResources> = OnceLock::new();
static TIMELINE: Mutex<Option<TimelineState>> = Mutex::new(None);
static MEDIA_VIEW: Mutex<Option<MediaView>> = Mutex::new(None);
static GUI_REQUESTS: Mutex<std::collections::VecDeque<GuiRequest>> =
    Mutex::new(std::collections::VecDeque::new());
static PREVIEW_ERROR: Mutex<Option<String>> = Mutex::new(None);
static CONSUMERS: Mutex<Vec<(String, Instant)>> = Mutex::new(Vec::new());
pub fn consumer_present(id: &str) -> bool {
    CONSUMERS.lock().is_ok_and(|consumers| {
        consumers
            .iter()
            .any(|(known, seen)| known == id && seen.elapsed() < Duration::from_secs(15))
    })
}
pub fn preference_owner_id() -> eframe::egui::Id {
    eframe::egui::Id::new("remote_preference_preview_owner")
}
pub fn observe_consumer(headers: &[(String, String)]) {
    if let Some((_, id)) = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("x-pealayer-client"))
        && id.len() <= 64
        && id
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || value == b'-')
    {
        if let Ok(mut consumers) = CONSUMERS.lock() {
            consumers.retain(|(_, seen)| seen.elapsed() < Duration::from_secs(5));
            if let Some((_, seen)) = consumers.iter_mut().find(|(known, _)| known == id) {
                *seen = Instant::now()
            } else if consumers.len() < 64 {
                consumers.push((id.clone(), Instant::now()));
            }
        }
    }
}
pub fn set_preview_error(error: String) {
    if let Ok(mut slot) = PREVIEW_ERROR.lock() {
        *slot = Some(error);
    }
}
pub fn take_preview_error() -> Option<String> {
    PREVIEW_ERROR.lock().ok().and_then(|mut slot| slot.take())
}
thread_local! { static MIRRORING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

pub fn instance_id() -> &'static str {
    INSTANCE.get_or_init(|| uuid::Uuid::new_v4().to_string())
}
pub fn client() -> Option<&'static Arc<Client>> {
    CLIENT.get()
}
pub fn active() -> bool {
    client().is_some()
}
pub fn diagnostics() -> Value {
    let Some(client) = client() else {
        return serde_json::json!({"role":"authority","instance_id":instance_id()});
    };
    let snapshot = client.snapshot();
    let decoded = SERVER
        .get()
        .and_then(|server| server.mpv.get_property::<f64>("time-pos").ok());
    let position = snapshot.as_ref().map(|value| {
        value.session.position
            + if value.session.paused {
                0.0
            } else {
                (value.received.elapsed().as_secs_f64() + value.round_trip.as_secs_f64() / 2.0)
                    * value.session.speed
            }
    });
    serde_json::json!({
        "role":"client","instance_id":instance_id(),"server":client.origin.as_str(),"local_port":client.local_port,
        "connected":snapshot.as_ref().is_some_and(|value|value.received.elapsed()<Duration::from_secs(2)),
        "sample_age_ms":snapshot.as_ref().map(|value|value.received.elapsed().as_millis() as u64),
        "round_trip_ms":snapshot.as_ref().map(|value|value.round_trip.as_millis() as u64),
        "server_position":position,"decoded_position":decoded,"preview_drift_seconds":decoded.zip(position).map(|(decoded,position)|decoded-position),
        "transport_error":client.error.lock().ok().and_then(|value|value.clone()),
        "command_error":client.command_error.lock().ok().and_then(|value|value.clone()),
        "local_storage":"cache_only","local_hardware_scheduler":false
    })
}
pub fn mirroring() -> bool {
    MIRRORING.with(|value| value.get())
}
pub fn mirror<T>(apply: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            MIRRORING.with(|value| value.set(self.0));
        }
    }
    let _reset = Reset(MIRRORING.with(|value| value.replace(true)));
    apply()
}

pub fn endpoint(value: &str) -> Result<url::Url, String> {
    let normalized = value
        .strip_prefix("pealayer://")
        .map(|rest| format!("http://{rest}"))
        .unwrap_or_else(|| value.to_string());
    let mut url =
        url::Url::parse(&normalized).map_err(|_| "Invalid Pealayer endpoint".to_string())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.path(), "" | "/")
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "Use pealayer://host:port or an HTTP(S) origin without a path or credentials".into(),
        );
    }
    if url.port().is_none() {
        url.set_port(Some(8080))
            .map_err(|_| "Invalid Pealayer port")?;
    }
    Ok(url)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    pub instance_id: String,
    pub config: crate::config::AppConfig,
    pub status: Value,
    pub hardware: Option<crate::four_d::controller::HardwareCapabilities>,
    pub media: Option<String>,
    #[serde(default)]
    pub media_view: Option<MediaView>,
    pub position: f64,
    pub paused: bool,
    pub speed: f64,
    pub sampled_unix_ms: u64,
    pub timeline: Option<TimelineState>,
    pub config_path: String,
    pub consumers: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaView {
    pub tracks: Vec<crate::app::MediaTrackInfo>,
    pub file: crate::media_info::MediaFileInfo,
    pub vid: String,
    pub aid: String,
    pub sid: String,
}
pub fn publish_media_view(view: MediaView) {
    if !active()
        && let Ok(mut slot) = MEDIA_VIEW.lock()
    {
        *slot = Some(view);
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineState {
    pub timeline: crate::four_d::models::Timeline,
    pub muted: std::collections::BTreeSet<u8>,
    pub soloed: std::collections::BTreeSet<u8>,
}
pub fn publish_timeline(state: TimelineState) {
    if !active()
        && let Ok(mut slot) = TIMELINE.lock()
    {
        *slot = Some(state);
    }
}
pub struct GuiRequest {
    pub path: String,
    pub value: Value,
    pub reply: mpsc::Sender<Result<Value, String>>,
    pub deadline: Instant,
}
pub fn gui_request(path: &str, value: Value) -> Result<Value, String> {
    let (reply, result) = mpsc::channel();
    let mut pending = GUI_REQUESTS
        .lock()
        .map_err(|_| "Session dispatcher is unavailable")?;
    if pending.len() >= 64 {
        return Err("Session dispatcher is busy".into());
    }
    pending.push_back(GuiRequest {
        path: path.into(),
        value,
        reply,
        deadline: Instant::now() + Duration::from_secs(3),
    });
    drop(pending);
    if let Some(server) = SERVER.get() {
        server.context.request_repaint();
    }
    result
        .recv_timeout(Duration::from_secs(4))
        .map_err(|_| "Server did not acknowledge the change; it was not retried".to_string())?
}
pub fn take_gui_requests() -> Vec<GuiRequest> {
    GUI_REQUESTS
        .lock()
        .map(|mut queue| queue.drain(..).collect())
        .unwrap_or_default()
}

struct ServerResources {
    engine: mpsc::Sender<crate::four_d::engine::EngineMessage>,
    capabilities: Arc<Mutex<Option<crate::four_d::controller::HardwareCapabilities>>>,
    mpv: &'static libmpv2::Mpv,
    context: eframe::egui::Context,
}
pub fn register_server(
    engine: &crate::four_d::engine::EngineHandle,
    mpv: &'static libmpv2::Mpv,
    context: eframe::egui::Context,
) {
    let _ = SERVER.set(ServerResources {
        engine: engine.sender.clone(),
        capabilities: engine.hardware_capabilities.clone(),
        mpv,
        context,
    });
}
pub fn server_session() -> Result<Session, String> {
    let server = SERVER.get().ok_or("Pealayer session is initializing")?;
    let media = server
        .mpv
        .get_property::<String>("path")
        .ok()
        .filter(|value| {
            !value.is_empty() && crate::platform::interop::get_live_config().web_allow_file_access
        });
    Ok(Session {
        instance_id: instance_id().into(),
        config: crate::platform::interop::get_live_config(),
        status: serde_json::to_value(crate::platform::interop::get_live_status())
            .unwrap_or(Value::Null),
        hardware: server
            .capabilities
            .lock()
            .ok()
            .and_then(|value| value.clone()),
        media,
        media_view: MEDIA_VIEW.lock().ok().and_then(|view| view.clone()),
        position: server.mpv.get_property::<f64>("time-pos").unwrap_or(0.0),
        paused: server.mpv.get_property::<bool>("pause").unwrap_or(true),
        speed: server.mpv.get_property::<f64>("speed").unwrap_or(1.0),
        sampled_unix_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        timeline: TIMELINE.lock().ok().and_then(|value| value.clone()),
        config_path: crate::config::AppConfig::get_config_path()
            .to_string_lossy()
            .into(),
        consumers: CONSUMERS
            .lock()
            .map(|values| {
                values
                    .iter()
                    .filter(|(_, seen)| seen.elapsed() < Duration::from_secs(5))
                    .map(|(id, _)| id.clone())
                    .collect()
            })
            .unwrap_or_default(),
    })
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum MediaOperation {
    Command { name: String, args: Vec<String> },
    SetProperty { name: String, value: Value },
}
pub fn apply_media_operation(operation: MediaOperation) -> Result<(), String> {
    let mpv = SERVER.get().ok_or("Pealayer session is initializing")?.mpv;
    match operation {
        MediaOperation::Command { name, args } => {
            if !matches!(
                name.as_str(),
                "loadfile"
                    | "stop"
                    | "seek"
                    | "frame-step"
                    | "frame-back-step"
                    | "playlist-next"
                    | "playlist-prev"
                    | "cycle"
                    | "sub-add"
                    | "audio-add"
                    | "video-add"
            ) {
                return Err("This MPV command is not exposed to peers".into());
            }
            if args.len() > 16 || args.iter().any(|value| value.len() > 32768) {
                return Err("Invalid media command arguments".into());
            }
            if name == "cycle"
                && !args.first().is_some_and(|value| {
                    matches!(value.as_str(), "pause" | "mute" | "sid" | "aid" | "vid")
                })
            {
                return Err("Unsupported cycle property".into());
            }
            if name == "loadfile" {
                // Opening must use the application path (session, cues, history,
                // proxy configuration), not a raw libmpv command.
                return Err("Use the unified Open command to load media".into());
            }
            if matches!(name.as_str(), "sub-add" | "audio-add" | "video-add")
                && !crate::platform::interop::get_live_config().web_allow_file_access
            {
                return Err("Web host file access permission is disabled".into());
            }
            mpv.command(&name, &args.iter().map(String::as_str).collect::<Vec<_>>())
                .map_err(|error| error.to_string())
        }
        MediaOperation::SetProperty { name, value } => {
            if !matches!(
                name.as_str(),
                "pause"
                    | "volume"
                    | "mute"
                    | "speed"
                    | "aid"
                    | "vid"
                    | "sid"
                    | "sub-visibility"
                    | "sub-delay"
                    | "sub-pos"
                    | "sub-font-size"
                    | "sub-align-x"
                    | "sub-justify"
                    | "audio-delay"
                    | "chapter"
                    | "ab-loop-a"
                    | "ab-loop-b"
                    | "loop-file"
            ) {
                return Err("This MPV property is not exposed to peers".into());
            }
            match value {
                Value::Bool(value) => mpv.set_property(&name, value),
                Value::Number(value) if value.is_i64() => {
                    mpv.set_property(&name, value.as_i64().unwrap())
                }
                Value::Number(value) => {
                    mpv.set_property(&name, value.as_f64().ok_or("Invalid numeric property")?)
                }
                Value::String(value) if value.len() <= 32768 => mpv.set_property(&name, value),
                _ => return Err("Unsupported media property value".into()),
            }
            .map_err(|error| error.to_string())
        }
    }
}
pub fn controller_call(method: String, params: Value) -> Result<Value, String> {
    if !method.starts_with("controller.") || method.len() > 128 {
        return Err("Invalid controller method".into());
    }
    let server = SERVER.get().ok_or("Pealayer session is initializing")?;
    let (tx, rx) = mpsc::channel();
    server
        .engine
        .send(crate::four_d::engine::EngineMessage::ReplyControllerCall {
            method,
            params,
            reply: tx,
        })
        .map_err(|_| "Hardware dispatcher is unavailable")?;
    rx.recv_timeout(Duration::from_secs(8))
        .map_err(|_| "Hardware acknowledgement timed out; command was not retried".to_string())?
}
pub fn hardware_command(command: crate::four_d::protocol::Command) -> Result<(), String> {
    let server = SERVER.get().ok_or("Pealayer session is initializing")?;
    server
        .engine
        .send(crate::four_d::engine::EngineMessage::SendCommand(command))
        .map_err(|_| "Hardware dispatcher is unavailable".to_string())
}

pub struct ReceivedSession {
    pub session: Session,
    pub received: Instant,
    pub round_trip: Duration,
    pub revision: u64,
}
pub struct Client {
    pub origin: url::Url,
    pub local_port: u16,
    pub http: reqwest::blocking::Client,
    latest: Mutex<ReceivedSession>,
    pending: mpsc::SyncSender<(String, Value, Instant)>,
    pub error: Mutex<Option<String>>,
    pub command_error: Mutex<Option<String>>,
    context: Mutex<Option<eframe::egui::Context>>,
}
impl Client {
    pub fn url(&self, path: &str) -> Result<url::Url, String> {
        if !path.starts_with('/') || path.starts_with("//") || path.contains('\\') {
            return Err("Invalid peer API path".into());
        }
        let url = self.origin.join(path).map_err(|error| error.to_string())?;
        if url.origin() != self.origin.origin() {
            return Err("Peer request must stay on the selected origin".into());
        }
        Ok(url)
    }
    pub fn request(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> Result<reqwest::blocking::RequestBuilder, String> {
        Ok(self
            .http
            .request(method, self.url(path)?)
            .header("X-Pealayer-Route", instance_id())
            .header("X-Pealayer-Client", instance_id()))
    }
    pub fn post(&self, path: &str, value: &Value) -> Result<Value, String> {
        if self
            .snapshot()
            .is_none_or(|value| value.received.elapsed() > Duration::from_secs(2))
        {
            return Err("Remote Pealayer disconnected; command was not sent".into());
        }
        let response = self
            .request(reqwest::Method::POST, path)?
            .json(value)
            .send()
            .map_err(|error| format!("Remote Pealayer request failed: {error}"))?;
        let status = response.status();
        let body = response
            .json::<Value>()
            .map_err(|_| "Invalid remote Pealayer response".to_string())?;
        if !status.is_success() {
            return Err(body
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("Remote Pealayer rejected the request")
                .into());
        }
        Ok(body)
    }
    pub fn queue(&self, path: &str, value: Value) -> Result<(), String> {
        if self
            .latest
            .lock()
            .is_ok_and(|value| value.received.elapsed() > Duration::from_secs(2))
        {
            return Err(
                "Remote Pealayer is disconnected; no local fallback or queued replay was performed"
                    .into(),
            );
        }
        self.pending
            .try_send((path.into(), value, Instant::now()))
            .map_err(|_| {
                "Remote command queue is full or disconnected; command was not sent".into()
            })
    }
    pub fn snapshot(&self) -> Option<ReceivedSession> {
        self.latest.lock().ok().map(|value| ReceivedSession {
            session: value.session.clone(),
            received: value.received,
            round_trip: value.round_trip,
            revision: value.revision,
        })
    }
    pub fn config(&self) -> crate::config::AppConfig {
        self.latest
            .lock()
            .map(|value| value.session.config.clone())
            .unwrap_or_default()
    }
    pub fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        self.request(reqwest::Method::GET, path)?
            .send()
            .and_then(|response| response.error_for_status())
            .and_then(|response| response.json())
            .map_err(|error| error.to_string())
    }
    pub fn media_url(&self, target: &str) -> Result<String, String> {
        if let Ok(url) = url::Url::parse(target)
            && matches!(url.scheme(), "http" | "https" | "rtsp" | "rtmp")
        {
            return Ok(target.into());
        }
        let mut url = self.url("/api/fs/file")?;
        url.query_pairs_mut().append_pair("path", target);
        Ok(url.to_string())
    }
    /// A cached image is the only filesystem output of remote folder browsing.
    /// The authority performs discovery/FFmpeg work and owns proxy credentials.
    pub fn folder_thumbnail(&self, target: &str) -> Result<std::path::PathBuf, String> {
        use sha2::{Digest, Sha256};
        use std::io::Read;
        let directory = crate::server::thumbnails::get_thumbnail_cache_dir().join("peers");
        let key = format!(
            "{:x}",
            Sha256::digest(format!("{}\n{target}", self.origin).as_bytes())
        );
        let path = directory.join(format!("{key}.jpg"));
        if path.is_file() {
            return Ok(path);
        }
        let mut url = self.url("/api/remote/thumbnail")?;
        url.query_pairs_mut().append_pair("url", target);
        let request_target = format!("{}?{}", url.path(), url.query().unwrap_or_default());
        for _ in 0..20 {
            let response = self
                .request(reqwest::Method::GET, &request_target)?
                .send()
                .map_err(|error| error.to_string())?;
            if response.status() == reqwest::StatusCode::ACCEPTED {
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
            let response = response
                .error_for_status()
                .map_err(|error| error.to_string())?;
            let mut bytes = Vec::new();
            response
                .take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())?;
            if bytes.len() > 8 * 1024 * 1024 {
                return Err("Peer thumbnail exceeds 8 MiB".into());
            }
            image::load_from_memory(&bytes).map_err(|error| error.to_string())?;
            std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
            // The caller caps in-memory entries; bound the on-disk peer cache too.
            let mut cached = std::fs::read_dir(&directory)
                .map_err(|error| error.to_string())?
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .is_some_and(|extension| extension == "jpg")
                })
                .collect::<Vec<_>>();
            cached.sort_by_key(|entry| entry.metadata().and_then(|meta| meta.modified()).ok());
            let excess = cached.len().saturating_sub(255);
            for entry in cached.into_iter().take(excess) {
                let _ = std::fs::remove_file(entry.path());
            }
            let staged = directory.join(format!(".{key}.{}.tmp", uuid::Uuid::new_v4()));
            std::fs::write(&staged, bytes).map_err(|error| error.to_string())?;
            if let Err(error) = std::fs::rename(&staged, &path) {
                let _ = std::fs::remove_file(&staged);
                if !path.is_file() {
                    return Err(error.to_string());
                }
            }
            return Ok(path);
        }
        Err("Peer thumbnail is still pending; refresh the folder to retry".into())
    }
    pub fn save_config(&self, config: &crate::config::AppConfig) -> Result<(), String> {
        if mirroring() {
            return Ok(());
        }
        let old = serde_json::to_value(self.config()).map_err(|error| error.to_string())?;
        let new = serde_json::to_value(config).map_err(|error| error.to_string())?;
        let mut patch = serde_json::Map::new();
        for (key, value) in new.as_object().ok_or("Invalid configuration")? {
            if matches!(
                key.as_str(),
                "window_geometry"
                    | "egui_memory"
                    | "last_media_target"
                    | "last_media_paused"
                    | "playback_positions"
                    | "hardware_endpoint"
                    | "effect_cue_session"
            ) {
                continue;
            }
            if old.get(key) != Some(value) {
                patch.insert(key.clone(), value.clone());
            }
        }
        // Preserve screen-dependent server geometry while sharing workspace contents.
        for key in ["workspace_session", "workspace_profiles"] {
            if let Some(value) = patch.get_mut(key) {
                if key == "workspace_session" {
                    preserve_geometry(value, old.get(key));
                } else if let Some(profiles) = value.as_object_mut() {
                    for (id, profile) in profiles {
                        preserve_geometry(profile, old.get(key).and_then(|value| value.get(id)));
                    }
                }
            }
        }
        self.post(
            "/api/peer/config",
            &serde_json::json!({"operation":"save","expected":patch.keys().map(|key|(key.clone(),old.get(key).cloned().unwrap_or(Value::Null))).collect::<serde_json::Map<String,Value>>(),"values":patch}),
        )
        .map(|_| ())
    }
    pub fn register_context(&self, context: &eframe::egui::Context) {
        if let Ok(mut slot) = self.context.lock() {
            *slot = Some(context.clone());
        }
    }
}
pub fn validate_config_expectations(
    config: &crate::config::AppConfig,
    expected: Option<&Value>,
) -> Result<(), String> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let expected = expected
        .as_object()
        .ok_or("Expected configuration must be an object")?;
    let current = serde_json::to_value(config).map_err(|error| error.to_string())?;
    for (key, value) in expected {
        if current.get(key) != Some(value) {
            return Err(format!(
                "Setting {key} changed on another peer; refresh before saving"
            ));
        }
    }
    Ok(())
}
pub(crate) fn probe_session(value: &str) -> Result<(url::Url, reqwest::blocking::Client, Session, Duration), String> {
    let origin = endpoint(value)?;
    let http = reqwest::blocking::Client::builder()
        .no_proxy()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| error.to_string())?;
    let started = Instant::now();
    let response = http
        .get(
            origin
                .join("/api/peer/session")
                .map_err(|error| error.to_string())?,
        )
        .header("X-Pealayer-Route", instance_id())
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(
            "Server does not expose the Pealayer peer session API, or access is disabled".into(),
        );
    }
    let session = response
        .json::<Session>()
        .map_err(|_| "Invalid Pealayer peer session".to_string())?;
    if session.instance_id == instance_id() {
        return Err("A Pealayer instance cannot connect to itself".into());
    }
    session.config.validate()?;
    Ok((origin, http, session, started.elapsed()))
}

pub fn connect(value: &str, local_port: u16) -> Result<(), String> {
    if local_port == 0 {
        return Err("Client Web port must be nonzero".into());
    }
    let (origin, http, session, round_trip) = probe_session(value)?;
    let (tx, rx) = mpsc::sync_channel::<(String, Value, Instant)>(64);
    let client = Arc::new(Client {
        origin,
        local_port,
        http,
        latest: Mutex::new(ReceivedSession {
            session,
            received: Instant::now(),
            round_trip,
            revision: 1,
        }),
        pending: tx,
        error: Mutex::new(None),
        command_error: Mutex::new(None),
        context: Mutex::new(None),
    });
    CLIENT
        .set(client.clone())
        .map_err(|_| "A remote session is already selected")?;
    let commands = client.clone();
    std::thread::spawn(move || {
        while let Ok((path, value, created)) = rx.recv() {
            let result = if created.elapsed() > Duration::from_millis(500) {
                Err("Expired remote command was discarded; it was not replayed".into())
            } else {
                commands.post(&path, &value)
            };
            if let Ok(mut slot) = commands.command_error.lock() {
                *slot = result.err();
            }
            if let Some(context) = commands.context.lock().ok().and_then(|value| value.clone()) {
                context.request_repaint();
            }
        }
    });
    std::thread::spawn(move || {
        loop {
            // Keep precise position synchronization while playing, but do not
            // serialize and transfer the full session/config ten times per
            // second while paused. The one-second idle snapshot also bounds
            // remote health detection without a permanent high-rate poll.
            let poll_interval = client
                .snapshot()
                .map(|snapshot| session_poll_interval(snapshot.session.paused))
                .unwrap_or(ACTIVE_SESSION_POLL_INTERVAL);
            let started = Instant::now();
            let next = client
                .request(reqwest::Method::GET, "/api/peer/session")
                .and_then(|request| request.send().map_err(|error| error.to_string()))
                .and_then(|response| {
                    response
                        .error_for_status()
                        .map_err(|error| error.to_string())
                })
                .and_then(|response| {
                    response
                        .json::<Session>()
                        .map_err(|error| error.to_string())
                });
            match next {
                Ok(session) => {
                    let mut changed = false;
                    if let Ok(mut latest) = client.latest.lock() {
                        changed = latest.session.status != session.status
                            || latest.session.config != session.config
                            || latest.session.media != session.media
                            || latest.session.media_view != session.media_view
                            || latest.session.timeline != session.timeline;
                        latest.session = session;
                        latest.received = Instant::now();
                        latest.round_trip = started.elapsed();
                        latest.revision = latest.revision.wrapping_add(1);
                    }
                    if let Ok(mut error) = client.error.lock() {
                        *error = None;
                    }
                    if changed
                        && let Some(context) =
                            client.context.lock().ok().and_then(|value| value.clone())
                    {
                        context.request_repaint();
                    }
                }
                Err(error) => {
                    if let Ok(mut slot) = client.error.lock() {
                        *slot = Some(error)
                    }
                }
            }
            std::thread::sleep(poll_interval);
        }
    });
    Ok(())
}

#[derive(Clone, Default)]
pub(crate) struct PeerUiState {
    pub link_connected: Option<bool>,
    pub config: Option<crate::config::AppConfig>,
    pub loaded_media: Option<String>,
    pub media_error: Option<String>,
    pub timeline: Option<TimelineState>,
    pub last_correction: Option<Instant>,
    pub attached_external: std::collections::BTreeSet<String>,
}

pub(crate) fn peer_link_was_lost(previous: Option<bool>, fresh: bool) -> bool {
    previous == Some(true) && !fresh
}

fn preserve_geometry(new: &mut Value, old: Option<&Value>) {
    if let Some(new) = new.as_object_mut() {
        for key in ["window_geometry", "egui_memory"] {
            new.insert(
                key.into(),
                old.and_then(|value| value.get(key))
                    .cloned()
                    .unwrap_or(Value::Null),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn process_probe_requires_a_valid_foreign_session_and_sends_loop_identity() {
        use std::io::{Read, Write};
        for (id, expected) in [("foreign-instance", true), (instance_id(), false)] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address=listener.local_addr().unwrap();
            let session=Session {
                instance_id:id.into(), config:crate::config::AppConfig::default(),status:serde_json::json!({}),
                hardware:None,media:None,media_view:None,position:0.0,paused:true,speed:1.0,
                sampled_unix_ms:0,timeline:None,config_path:String::new(),consumers:vec![],
            };
            let body=serde_json::to_string(&session).unwrap();
            let worker=std::thread::spawn(move || {
                let (mut stream, _)=listener.accept().unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut bytes=Vec::new();
                loop {
                    let mut chunk=[0u8;1024]; let count=stream.read(&mut chunk).unwrap();
                    if count==0 { break; } bytes.extend_from_slice(&chunk[..count]);
                    if bytes.windows(4).any(|window|window==b"\r\n\r\n") {break;}
                }
                let request=String::from_utf8(bytes).unwrap().to_ascii_lowercase();
                assert!(request.starts_with("get /api/peer/session "));
                assert!(request.contains(&format!("x-pealayer-route: {}",instance_id())));
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
            });
            assert_eq!(probe_session(&format!("pealayer://{address}")).is_ok(), expected);
            worker.join().unwrap();
        }
    }
    use super::*;
    #[test]
    fn stale_peer_link_is_not_a_board_disconnect_or_initial_connection_failure() {
        assert!(peer_link_was_lost(Some(true), false));
        assert!(!peer_link_was_lost(None, false));
        assert!(!peer_link_was_lost(Some(false), false));
        assert!(!peer_link_was_lost(Some(false), true));
    }
    #[test]
    fn peer_session_polling_slows_only_while_paused() {
        assert_eq!(
            session_poll_interval(false),
            Duration::from_millis(100)
        );
        assert_eq!(
            session_poll_interval(true),
            Duration::from_secs(1)
        );
        assert!(session_poll_interval(true) <= Duration::from_secs(1));
    }

    #[test]
    fn endpoint_validation() {
        assert_eq!(
            endpoint("pealayer://[::1]:8181").unwrap().as_str(),
            "http://[::1]:8181/"
        );
        assert_eq!(endpoint("pealayer://cafe-pc").unwrap().port(), Some(8080));
        for value in [
            "pccontroller://host:8787",
            "http://a:1/path",
            "http://user:pass@host:8080",
            "http://host/?x=1",
        ] {
            assert!(endpoint(value).is_err());
        }
    }
    #[test]
    fn configuration_conflicts_do_not_overwrite_newer_values() {
        let config = crate::config::AppConfig::default();
        assert!(
            validate_config_expectations(
                &config,
                Some(&serde_json::json!({"volume":config.volume}))
            )
            .is_ok()
        );
        assert!(
            validate_config_expectations(
                &config,
                Some(&serde_json::json!({"volume":config.volume+1.0}))
            )
            .is_err()
        );
        assert!(validate_config_expectations(&config, Some(&serde_json::json!([]))).is_err());
    }
    #[test]
    fn workspace_sharing_preserves_screen_specific_geometry() {
        let old = serde_json::json!({"window_geometry":{"x":12},"egui_memory":"server"});
        let mut new =
            serde_json::json!({"name":"Shared","window_geometry":{"x":999},"egui_memory":"client"});
        preserve_geometry(&mut new, Some(&old));
        assert_eq!(new["window_geometry"], old["window_geometry"]);
        assert_eq!(new["egui_memory"], old["egui_memory"]);
        assert_eq!(new["name"], "Shared");
    }
}
