//! Persistent duplex mpv JSON IPC. No property polling or shell commands.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[cfg(unix)]
use std::io::{Read, Write};
use std::{
    collections::{BTreeMap, VecDeque},
    io::{self},
    sync::{
        Arc, Mutex, OnceLock, RwLock,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Internal,
    External,
    Dual,
    Remote,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub mode: Mode,
    pub executable: String,
    pub endpoint: String,
    pub use_mpv_config: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::Internal,
            executable: "mpv".into(),
            endpoint: String::new(),
            use_mpv_config: false,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.executable.trim().is_empty()
            || self.executable.len() > 4096
            || self.executable.contains(['\n', '\r', '\0'])
        {
            return Err(
                "external_mpv.executable must be an executable path, without arguments".into(),
            );
        }
        if self.endpoint.len() > 4096 || self.endpoint.contains(['\n', '\r', '\0']) {
            return Err("external_mpv.endpoint is invalid".into());
        }
        if self.mode == Mode::Remote && self.endpoint.is_empty() {
            return Err("Remote mpv requires its existing IPC endpoint".into());
        }
        if !self.endpoint.is_empty() {
            #[cfg(windows)]
            if !self.endpoint.starts_with(r"\\.\pipe\")
                || self.endpoint.len() <= 9
                || self.endpoint[9..].contains(['\\', '/'])
            {
                return Err(r"Use a local Windows endpoint such as \\.\pipe\mpv".into());
            }
            #[cfg(unix)]
            if !std::path::Path::new(&self.endpoint).is_absolute() {
                return Err("Use an absolute Unix socket path".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Status {
    pub mode: Mode,
    pub endpoint: String,
    pub connected: bool,
    pub owned_process: bool,
    pub revision: u64,
    pub catalog_revision: u64,
    pub seek_revision: u64,
    pub reconnects: u64,
    pub acknowledged_commands: u64,
    pub pending_commands: usize,
    pub error: Option<String>,
    pub preview_error: Option<String>,
    #[serde(skip)]
    pub properties: BTreeMap<String, Value>,
    #[serde(skip)]
    acknowledged_ids: Vec<u64>,
    #[serde(skip)]
    pub clock_at: Option<Instant>,
}
const OBSERVED: &[&str] = &[
    "path",
    "working-directory",
    "time-pos",
    "duration",
    "pause",
    "volume",
    "mute",
    "speed",
    "seeking",
    "eof-reached",
    "idle-active",
    "paused-for-cache",
    "seekable",
    "track-list",
    "chapter-list",
    "chapter",
    "metadata",
    "media-title",
    "container-fps",
    "estimated-vf-fps",
    "video-out-params",
    "video-params",
    "video-dec-params",
    "audio-params",
    "audio-device-list",
    "audio-device",
    "current-ao",
    "current-tracks",
    "vid",
    "aid",
    "sid",
    "secondary-sid",
    "sub-text",
    "sub-visibility",
    "sub-font-size",
    "sub-font",
    "sub-fonts-dir",
    "sub-color",
    "sub-border-color",
    "sub-back-color",
    "sub-border-size",
    "sub-shadow-offset",
    "sub-ass-override",
    "sub-delay",
    "sub-pos",
    "sub-align-x",
    "sub-justify",
    "audio-delay",
    "demuxer-cache-duration",
    "demuxer-cache-time",
    "cache-buffering-state",
    "demuxer-cache-state",
    "file-size",
    "file-format",
    "video-codec",
    "audio-codec-name",
    "video-format",
];
enum Work {
    Configure(Settings),
    Command(u64, Value),
}
struct Bridge {
    tx: SyncSender<Work>,
    state: Arc<RwLock<Status>>,
    generation: Arc<AtomicU64>,
    settings: Mutex<Option<Settings>>,
}
static BRIDGE: OnceLock<Bridge> = OnceLock::new();
pub fn configure(mpv: &'static libmpv2::Mpv, ctx: &eframe::egui::Context, mut settings: Settings) {
    // A Pealayer consumer controls the server's external player, never launches
    // another local playback/hardware authority from mirrored preferences.
    if crate::peer::active() {
        settings.mode = Mode::Internal;
    }
    let bridge = BRIDGE.get_or_init(|| {
        let (tx, rx) = mpsc::sync_channel(128);
        let state = Arc::new(RwLock::new(Status::default()));
        let generation = Arc::new(AtomicU64::new(1));
        let s = state.clone();
        let g = generation.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || run(rx, s, g, mpv, ctx));
        Bridge {
            tx,
            state,
            generation,
            settings: Mutex::new(None),
        }
    });
    let mut previous = bridge.settings.lock().unwrap();
    if previous.as_ref() != Some(&settings) {
        if bridge
            .tx
            .try_send(Work::Configure(settings.clone()))
            .is_ok()
        {
            bridge.generation.fetch_add(1, Ordering::AcqRel);
            bridge.state.write().unwrap().connected = false;
            *previous = Some(settings);
        }
    }
}
pub fn status() -> Status {
    BRIDGE
        .get()
        .and_then(|b| b.state.read().ok().map(|s| s.clone()))
        .unwrap_or_default()
}
pub fn active() -> bool {
    !crate::peer::active()
        && BRIDGE.get().is_some_and(|b| {
            b.settings
                .lock()
                .is_ok_and(|s| s.as_ref().is_some_and(|s| s.mode != Mode::Internal))
        })
}
pub fn preview() -> bool {
    active()
        && BRIDGE
            .get()
            .is_some_and(|b| b.state.read().is_ok_and(|s| s.mode == Mode::Dual))
}
pub fn seek_revision() -> u64 {
    BRIDGE
        .get()
        .and_then(|b| b.state.read().ok().map(|s| s.seek_revision))
        .unwrap_or_default()
}
pub fn set_preview_error(error: String) {
    if let Some(b) = BRIDGE.get() {
        if let Ok(mut s) = b.state.write() {
            s.preview_error = Some(error);
        }
    }
}
pub fn shutdown() {
    if let Some(b) = BRIDGE.get() {
        let s = status();
        if s.owned_process {
            if let Ok(mut c) = Transport::connect(&s.endpoint) {
                let _ = c.write_json(&json!({"command":["quit"]}));
            }
        }
        b.generation.fetch_add(1, Ordering::AcqRel);
    }
}
pub fn send(command: Value) -> Result<(), String> {
    if command.as_array().is_none_or(|a| a.is_empty()) || command.to_string().len() > 65536 {
        return Err("Invalid mpv command".into());
    }
    let b = BRIDGE.get().ok_or("External mpv is not initialized")?;
    if !b.state.read().is_ok_and(|s| s.connected) {
        return Err("External mpv is disconnected; command was not sent".into());
    }
    if b.state.read().is_ok_and(|s| s.pending_commands >= 128) {
        return Err("External mpv is waiting for acknowledgements; command was not sent".into());
    }
    b.tx.try_send(Work::Command(b.generation.load(Ordering::Acquire), command))
        .map_err(|_| "External mpv command queue is full; command was not sent".into())
}
pub fn command(name: &str, args: &[&str]) -> Result<(), String> {
    let mut a = vec![json!(name)];
    a.extend(args.iter().map(|a| json!(a)));
    send(Value::Array(a))
}
pub fn property(name: &str) -> Option<Value> {
    let b = BRIDGE.get()?;
    let s = b.state.read().ok()?;
    if !s.connected {
        return None;
    }
    if name == "paused-for-cache"
        && flag(&s, "pause", true) == false
        && s.clock_at
            .is_some_and(|at| at.elapsed() > Duration::from_secs(1))
    {
        return Some(json!(true));
    }
    if name == "time-pos" {
        let mut t = s.properties.get(name)?.as_f64()?;
        if advancing(&s) {
            t += s.clock_at?.elapsed().as_secs_f64().min(0.25) * number(&s, "speed", 1.0)
        }
        return Some(json!(t.min(number(&s, "duration", f64::MAX))));
    }
    lookup(&s.properties, name)
}
fn lookup(properties: &BTreeMap<String, Value>, name: &str) -> Option<Value> {
    if let Some(v) = properties.get(name) {
        return (!v.is_null()).then(|| v.clone());
    }
    let (root, tail) = name.split_once('/')?;
    let mut v = properties.get(root)?.clone();
    let mut segments = tail.split('/');
    while let Some(segment) = segments.next() {
        if segment == "count" {
            return Some(json!(match v {
                Value::Array(a) => a.len(),
                Value::Object(m) => m.len(),
                _ => return None,
            }));
        }
        if segment == "list" && v.is_object() {
            v = Value::Array(
                v.as_object()?
                    .iter()
                    .map(|(k, v)| json!({"key":k,"value":v}))
                    .collect(),
            );
            continue;
        }
        v = if let Some(a) = v.as_array() {
            a.get(segment.parse::<usize>().ok()?)?.clone()
        } else {
            v.get(segment)?.clone()
        };
    }
    (!v.is_null()).then_some(v)
}
fn number(s: &Status, key: &str, fallback: f64) -> f64 {
    s.properties
        .get(key)
        .and_then(Value::as_f64)
        .filter(|x| x.is_finite())
        .unwrap_or(fallback)
}
fn flag(s: &Status, key: &str, fallback: bool) -> bool {
    s.properties
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}
fn advancing(s: &Status) -> bool {
    s.connected
        && !flag(s, "pause", true)
        && !flag(s, "paused-for-cache", false)
        && !flag(s, "seeking", false)
        && !flag(s, "eof-reached", false)
}

fn apply_event(s: &mut Status, v: &Value) -> bool {
    if v["event"] == "start-file" {
        for key in ["time-pos", "duration", "seekable"] {
            s.properties.remove(key);
        }
        s.clock_at = None;
        s.preview_error = None;
        s.catalog_revision = s.catalog_revision.saturating_add(1);
        return true;
    }
    if v["event"] == "file-loaded" {
        s.catalog_revision = s.catalog_revision.saturating_add(1);
        return true;
    }
    if v["event"] == "end-file" && v["reason"] == "error" {
        s.error = Some("External mpv could not load or decode the media".into());
        return true;
    }
    if v["event"] == "property-change" {
        let Some(name) = v["name"].as_str() else {
            return false;
        };
        let data = v.get("data").cloned().unwrap_or(Value::Null);
        if name == "time-pos" {
            s.clock_at = Some(Instant::now());
        }
        if matches!(name, "path" | "track-list" | "metadata" | "chapter-list")
            && s.properties.get(name) != Some(&data)
        {
            s.catalog_revision = s.catalog_revision.saturating_add(1)
        }
        s.properties.insert(name.into(), data);
        s.revision = s.revision.saturating_add(1);
        return true;
    }
    if v["event"] == "playback-restart" {
        s.seek_revision = s.seek_revision.saturating_add(1);
        s.revision = s.revision.saturating_add(1);
        return true;
    }
    false
}
fn mirror(mpv: &libmpv2::Mpv, s: &Status, last_path: &mut String) {
    if s.mode != Mode::Dual {
        return;
    }
    let path = s
        .properties
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if path != *last_path {
        *last_path = path.into();
        if path.is_empty() {
            let _ = mpv.command("stop", &[]);
        } else {
            let resolved = resolve_path(s, path);
            if mpv.command("loadfile", &[&resolved, "replace"]).is_err() {
                return;
            }
        }
    }
    // Only the external player owns audible output. Raw calls deliberately
    // bypass Player's outbound adapter, so incoming events never echo.
    if mpv.get_property::<bool>("mute").ok() != Some(true) {
        let _ = mpv.set_property("mute", true);
    }
    if mpv.get_property::<f64>("speed").ok() != Some(number(s, "speed", 1.0)) {
        let _ = mpv.set_property("speed", number(s, "speed", 1.0));
    }
    if mpv.get_property::<bool>("pause").ok() != Some(!advancing(s)) {
        let _ = mpv.set_property("pause", !advancing(s));
    }
    for key in [
        "vid",
        "sid",
        "secondary-sid",
        "sub-visibility",
        "sub-delay",
        "sub-pos",
        "sub-font-size",
        "sub-font",
        "sub-fonts-dir",
        "sub-color",
        "sub-border-color",
        "sub-back-color",
        "sub-border-size",
        "sub-shadow-offset",
        "sub-ass-override",
        "audio-delay",
    ] {
        if let Some(value) = s.properties.get(key) {
            set_local(mpv, key, value);
        }
    }
    if let (Some(target), Ok(local)) = (
        s.properties.get("time-pos").and_then(Value::as_f64),
        mpv.get_property::<f64>("time-pos"),
    ) {
        let tolerance = if flag(s, "pause", true) { 0.05 } else { 0.20 };
        if (local - target).abs() > tolerance
            && !mpv.get_property::<bool>("seeking").unwrap_or(true)
        {
            let _ = mpv.command("seek", &[&target.to_string(), "absolute+exact"]);
        }
    }
}
pub fn resolve_path(s: &Status, path: &str) -> String {
    if path.contains("://") || std::path::Path::new(path).is_absolute() {
        return path.into();
    }
    s.properties
        .get("working-directory")
        .and_then(Value::as_str)
        .map(|d| {
            std::path::Path::new(d)
                .join(path)
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| path.into())
}
fn set_local(mpv: &libmpv2::Mpv, key: &str, v: &Value) {
    match v {
        Value::Bool(v) => {
            if mpv.get_property::<bool>(key).ok() != Some(*v) {
                let _ = mpv.set_property(key, *v);
            }
        }
        Value::Number(n) => {
            if let Some(v) = n.as_i64() {
                if mpv.get_property::<i64>(key).ok() != Some(v) {
                    let _ = mpv.set_property(key, v);
                }
            } else if let Some(v) = n.as_f64() {
                if mpv.get_property::<f64>(key).ok() != Some(v) {
                    let _ = mpv.set_property(key, v);
                }
            }
        }
        Value::String(v) => {
            if mpv.get_property::<String>(key).ok().as_deref() != Some(v.as_str()) {
                let _ = mpv.set_property(key, v.as_str());
            }
        }
        _ => {}
    }
}

fn run(
    rx: mpsc::Receiver<Work>,
    state: Arc<RwLock<Status>>,
    generation: Arc<AtomicU64>,
    mpv: &'static libmpv2::Mpv,
    ctx: eframe::egui::Context,
) {
    let latest = Arc::new((Mutex::new(None::<(u64, Status)>), std::sync::Condvar::new()));
    let preview_latest = latest.clone();
    let preview_generation = generation.clone();
    let preview_lock = Arc::new(Mutex::new(()));
    let preview_worker_lock = preview_lock.clone();
    std::thread::spawn(move || {
        let mut last_path = String::new();
        let mut previous_epoch = 0;
        loop {
            let (slot, wake) = &*preview_latest;
            let mut pending = slot.lock().unwrap();
            while pending.is_none() {
                pending = wake.wait(pending).unwrap();
            }
            let (epoch, s) = pending.take().unwrap();
            drop(pending);
            if epoch != preview_generation.load(Ordering::Acquire) {
                continue;
            }
            if epoch != previous_epoch {
                last_path.clear();
                previous_epoch = epoch;
            }
            let _guard = preview_worker_lock.lock().unwrap();
            if epoch == preview_generation.load(Ordering::Acquire) {
                mirror(mpv, &s, &mut last_path);
            }
        }
    });
    let mut settings = Settings::default();
    let mut stream: Option<Transport> = None;
    let mut child: Option<std::process::Child> = None;
    let mut managed = false;
    let mut handoff: Option<(String, f64)> = None;
    #[cfg(unix)]
    let mut _owned_socket: Option<OwnedSocket> = None;
    let mut request = 0u64;
    let mut pending = BTreeMap::<u64, Instant>::new();
    let mut retry = Instant::now();
    let mut readers = VecDeque::<std::thread::JoinHandle<()>>::new();
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Work::Configure(next)) => {
                // Invalidate queued commands and the old reader before reconnect.
                generation.fetch_add(1, Ordering::AcqRel);
                stream = None;
                pending.clear();
                *latest.0.lock().unwrap() = None;
                // Finish any previous raw-decoder update before stopping or
                // handing it back. A stale preview must not overwrite a mode change.
                let _preview_guard = preview_lock.lock().unwrap();
                if let Some(mut owned) = child.take() {
                    if let Some(mut connection) = Transport::connect(&settings.endpoint).ok() {
                        let _ = connection.write_json(&json!({"command":["quit"]}));
                    }
                    let deadline = Instant::now() + Duration::from_secs(2);
                    while owned.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
                        std::thread::sleep(Duration::from_millis(25));
                    }
                    if owned.try_wait().ok().flatten().is_none() {
                        let _ = owned.kill();
                    }
                    let _ = owned.wait();
                }
                #[cfg(unix)]
                {
                    _owned_socket = None;
                }
                let previous_mode = settings.mode;
                let handback = state.read().ok().map(|s| s.clone());
                managed =
                    matches!(next.mode, Mode::External | Mode::Dual) && next.endpoint.is_empty();
                handoff = if managed && previous_mode == Mode::Internal {
                    mpv.get_property::<String>("path")
                        .ok()
                        .map(|path| (path, mpv.get_property::<f64>("time-pos").unwrap_or(0.0)))
                } else {
                    None
                };
                settings = next;
                retry = Instant::now();
                let reconnects = state.read().map(|s| s.reconnects).unwrap_or_default();
                *state.write().unwrap() = Status {
                    mode: settings.mode,
                    reconnects,
                    ..Default::default()
                };
                if matches!(settings.mode, Mode::External | Mode::Remote)
                    || settings.mode == Mode::Internal && previous_mode != Mode::Internal
                {
                    let _ = mpv.command("stop", &[]);
                }
                if settings.mode != Mode::Internal {
                    let _ = mpv.set_property("pause", true);
                }
                if settings.mode == Mode::Internal && previous_mode != Mode::Internal {
                    let cfg = crate::platform::interop::get_live_config();
                    let _ = mpv.set_property("mute", cfg.is_muted);
                    if let Some(s) = handback
                        && let Some(path) = s
                            .properties
                            .get("path")
                            .and_then(Value::as_str)
                            .filter(|p| !p.is_empty())
                    {
                        let resolved = resolve_path(&s, path);
                        let options = format!("start={},pause=yes", number(&s, "time-pos", 0.0));
                        let _ = mpv.command("loadfile", &[&resolved, "replace", "-1", &options]);
                    }
                }
                ctx.request_repaint();
            }
            Ok(Work::Command(epoch, command)) => {
                if epoch == generation.load(Ordering::Acquire)
                    && let Some(connection) = stream.as_mut()
                {
                    request = request.saturating_add(1);
                    if connection
                        .write_json(&json!({"command":command,"request_id":request}))
                        .is_ok()
                    {
                        pending.insert(request, Instant::now());
                    } else {
                        stream = None;
                        generation.fetch_add(1, Ordering::AcqRel);
                        state.write().unwrap().connected = false;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        while readers.front().is_some_and(|r| r.is_finished()) {
            if let Some(r) = readers.pop_front() {
                let _ = r.join();
            }
        }
        if settings.mode == Mode::Internal {
            continue;
        }
        if stream.is_some() && !state.read().is_ok_and(|s| s.connected) {
            stream = None;
            pending.clear();
            generation.fetch_add(1, Ordering::AcqRel);
            retry = Instant::now() + Duration::from_millis(500);
        }
        if let Some(owned) = child.as_mut()
            && owned.try_wait().ok().flatten().is_some()
        {
            child = None;
            stream = None;
            state.write().unwrap().error =
                Some("The launched mpv exited; switch modes to launch it again".into());
            settings.mode = Mode::Remote;
        }
        if stream.is_none() && Instant::now() >= retry {
            retry = Instant::now() + Duration::from_millis(750);
            if settings.endpoint.is_empty() {
                #[cfg(windows)]
                {
                    settings.endpoint = format!(
                        r"\\.\pipe\pealayer-mpv-{}-{}",
                        std::process::id(),
                        uuid::Uuid::new_v4()
                    );
                }
                #[cfg(unix)]
                {
                    match OwnedSocket::create() {
                        Ok(socket) => {
                            settings.endpoint =
                                socket.0.join("ipc.sock").to_string_lossy().into_owned();
                            _owned_socket = Some(socket);
                        }
                        Err(_) => {
                            state.write().unwrap().error =
                                Some("Could not create a private mpv socket directory".into());
                            continue;
                        }
                    }
                }
            }
            if managed && matches!(settings.mode, Mode::External | Mode::Dual) && child.is_none() {
                let cfg = crate::platform::interop::get_live_config();
                let mut c = std::process::Command::new(&settings.executable);
                c.args([
                    "--idle=yes",
                    "--keep-open=yes",
                    "--terminal=no",
                    "--input-media-keys=no",
                ]);
                if !settings.use_mpv_config {
                    c.arg("--no-config");
                }
                c.arg(format!("--input-ipc-server={}", settings.endpoint));
                c.arg("--title=Pealayer · External mpv");
                c.arg(format!("--volume={}", cfg.volume));
                c.arg(format!(
                    "--mute={}",
                    if cfg.is_muted { "yes" } else { "no" }
                ));
                c.arg(format!("--speed={}", cfg.playback_speed));
                c.arg(format!(
                    "--audio-device={}",
                    if cfg.audio_device.is_empty() {
                        "auto"
                    } else {
                        &cfg.audio_device
                    }
                ));
                c.arg(format!("--sub-font-size={}", cfg.subtitle_font_size));
                c.arg(format!("--sub-delay={}", cfg.subtitle_delay_seconds));
                c.arg(format!("--sub-pos={}", cfg.subtitle_position_percent));
                c.arg(format!("--audio-delay={}", cfg.audio_delay_seconds));
                c.arg(format!(
                    "--sub-align-x={}",
                    cfg.subtitle_alignment.mpv_value()
                ));
                for key in ["sub-font", "sub-fonts-dir", "sub-ass-override", "osd-font"] {
                    if let Ok(value) = mpv.get_property::<String>(key)
                        && !value.is_empty()
                    {
                        c.arg(format!("--{key}={value}"));
                    }
                }
                c.arg(format!(
                    "--http-proxy={}",
                    crate::mpv::proxy::playback_proxy(
                        cfg.open_url_use_proxy,
                        cfg.open_url_proxy_url.as_deref().unwrap_or("")
                    )
                    .unwrap_or_default()
                ));
                if !cfg.open_url_use_proxy {
                    c.arg("--stream-lavf-o=http_proxy=");
                }
                c.stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    c.creation_flags(0x08000000);
                }
                match c.spawn() {
                    Ok(p) => child = Some(p),
                    Err(_) => {
                        state.write().unwrap().error =
                            Some("Could not launch mpv; configure its executable path".into());
                        ctx.request_repaint();
                        continue;
                    }
                }
            }
            match Transport::connect(&settings.endpoint) {
                Ok(mut connection) => {
                    let Ok(reader) = connection.try_clone() else {
                        continue;
                    };
                    let epoch = generation.fetch_add(1, Ordering::AcqRel) + 1;
                    {
                        let mut s = state.write().unwrap();
                        s.properties.clear();
                        s.clock_at = None;
                        s.connected = true;
                        s.mode = settings.mode;
                        s.endpoint = settings.endpoint.clone();
                        s.owned_process = child.is_some();
                        s.error = None;
                        s.reconnects += 1;
                    }
                    let mut properties: std::collections::BTreeSet<&str> =
                        OBSERVED.iter().copied().collect();
                    properties.extend(crate::media_info::observed_roots());
                    for (i, name) in properties.into_iter().enumerate() {
                        let _ = connection.write_json(
                            &json!({"command":["observe_property",i+1,name],"request_id":0}),
                        );
                    }
                    let s = state.clone();
                    let g = generation.clone();
                    let reader_ctx = ctx.clone();
                    let latest = latest.clone();
                    readers.push_back(std::thread::spawn(move || {
                        read_events(reader, epoch, s, g, latest, reader_ctx)
                    }));
                    stream = Some(connection);
                    if let Some((path, position)) = handoff.take() {
                        request = request.saturating_add(1);
                        let options = format!("start={position},pause=yes");
                        if stream.as_mut().unwrap().write_json(&json!({"command":["loadfile",path,"replace",-1,options],"request_id":request})).is_ok() {
                            pending.insert(request,Instant::now());
                        }
                    }
                    ctx.request_repaint();
                }
                Err(_) => {
                    let mut s = state.write().unwrap();
                    s.endpoint = settings.endpoint.clone();
                    s.owned_process = child.is_some();
                    s.error=Some("Waiting for mpv IPC; start mpv with the configured --input-ipc-server endpoint".into());
                }
            }
        }
        // Replies are recorded by the reader; acknowledgement timeout retires
        // the connection, never replays non-idempotent commands on a new one.
        {
            let mut s = state.write().unwrap();
            for id in std::mem::take(&mut s.acknowledged_ids) {
                pending.remove(&id);
            }
            s.pending_commands = pending.len();
        }
        if pending
            .values()
            .any(|at| at.elapsed() > Duration::from_secs(3))
        {
            state.write().unwrap().error =
                Some("mpv command acknowledgement timed out; commands were not replayed".into());
            state.write().unwrap().connected = false;
            ctx.request_repaint();
        }
    }
}
fn read_events(
    mut reader: Transport,
    epoch: u64,
    state: Arc<RwLock<Status>>,
    generation: Arc<AtomicU64>,
    latest: Arc<(Mutex<Option<(u64, Status)>>, std::sync::Condvar)>,
    ctx: eframe::egui::Context,
) {
    let mut buffer = Vec::new();
    let mut bytes = [0u8; 16384];
    while generation.load(Ordering::Acquire) == epoch {
        match reader.read_bytes(&mut bytes) {
            Ok(0) => break,
            Ok(n) => {
                buffer.extend_from_slice(&bytes[..n]);
                if buffer.len() > 4 * 1024 * 1024 {
                    break;
                }
                while let Some(end) = buffer.iter().position(|b| *b == b'\n') {
                    let line: Vec<_> = buffer.drain(..=end).collect();
                    let Ok(value) = serde_json::from_slice::<Value>(&line) else {
                        continue;
                    };
                    if generation.load(Ordering::Acquire) != epoch {
                        return;
                    }
                    let snapshot = {
                        let mut s = state.write().unwrap();
                        if let Some(id) = value["request_id"].as_u64().filter(|id| *id > 0) {
                            s.acknowledged_commands += 1;
                            if s.acknowledged_ids.len() < 256 {
                                s.acknowledged_ids.push(id);
                            }
                            if value["error"].as_str().is_some_and(|e| e != "success") {
                                s.error = Some(format!(
                                    "mpv rejected a command: {}",
                                    value["error"].as_str().unwrap_or("unknown error")
                                ));
                            }
                        }
                        if apply_event(&mut s, &value) {
                            Some(s.clone())
                        } else {
                            None
                        }
                    };
                    if let Some(s) = snapshot {
                        let (slot, wake) = &*latest;
                        *slot.lock().unwrap() = Some((epoch, s));
                        wake.notify_one();
                        ctx.request_repaint();
                    }
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) => {}
            Err(_) => break,
        }
    }
    if generation.load(Ordering::Acquire) == epoch {
        let mut s = state.write().unwrap();
        s.connected = false;
        s.error = Some("mpv IPC disconnected; controls are paused until reconnection".into());
        let (slot, wake) = &*latest;
        *slot.lock().unwrap() = Some((epoch, s.clone()));
        wake.notify_one();
        drop(s);
        ctx.request_repaint();
    }
}

#[cfg(unix)]
struct OwnedSocket(std::path::PathBuf);
#[cfg(unix)]
impl OwnedSocket {
    fn create() -> io::Result<Self> {
        use std::os::unix::fs::DirBuilderExt;
        let path = std::env::temp_dir().join(format!("pealayer-mpv-{}", uuid::Uuid::new_v4()));
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700).create(&path)?;
        Ok(Self(path))
    }
}
#[cfg(unix)]
impl Drop for OwnedSocket {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.0.join("ipc.sock"));
        let _ = std::fs::remove_dir(&self.0);
    }
}

#[cfg(unix)]
struct Transport(std::os::unix::net::UnixStream);
#[cfg(unix)]
impl Transport {
    fn connect(path: &str) -> io::Result<Self> {
        let s = std::os::unix::net::UnixStream::connect(path)?;
        s.set_read_timeout(Some(Duration::from_millis(200)))?;
        s.set_write_timeout(Some(Duration::from_secs(2)))?;
        Ok(Self(s))
    }
    fn try_clone(&self) -> io::Result<Self> {
        Ok(Self(self.0.try_clone()?))
    }
    fn read_bytes(&mut self, b: &mut [u8]) -> io::Result<usize> {
        self.0.read(b)
    }
    fn write_json(&mut self, v: &Value) -> io::Result<()> {
        self.0.write_all(format!("{v}\n").as_bytes())
    }
}
#[cfg(windows)]
struct Transport(std::fs::File);
#[cfg(windows)]
impl Transport {
    fn connect(path: &str) -> io::Result<Self> {
        use std::os::windows::fs::OpenOptionsExt;
        Ok(Self(
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(0x40000000)
                .open(path)?,
        ))
    }
    fn try_clone(&self) -> io::Result<Self> {
        Ok(Self(self.0.try_clone()?))
    }
    fn read_bytes(&mut self, b: &mut [u8]) -> io::Result<usize> {
        self.io(b.as_mut_ptr(), b.len(), false, 200)
    }
    fn write_json(&mut self, v: &Value) -> io::Result<()> {
        let b = format!("{v}\n");
        let mut offset = 0;
        while offset < b.len() {
            let n = self.io(
                b.as_ptr().wrapping_add(offset) as *mut u8,
                b.len() - offset,
                true,
                2000,
            )?;
            if n == 0 {
                return Err(io::ErrorKind::WriteZero.into());
            }
            offset += n;
        }
        Ok(())
    }
    fn io(&self, buffer: *mut u8, len: usize, write: bool, timeout: u32) -> io::Result<usize> {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::{
            Foundation::{CloseHandle, HANDLE, WAIT_TIMEOUT},
            Storage::FileSystem::{ReadFile, WriteFile},
            System::{
                IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
                Threading::{CreateEventW, WaitForSingleObject},
            },
        };
        unsafe {
            let handle = HANDLE(self.0.as_raw_handle());
            let event = CreateEventW(None, true, false, None).map_err(io::Error::other)?;
            let mut overlapped = OVERLAPPED {
                hEvent: event,
                ..Default::default()
            };
            let mut count = 0;
            let result = if write {
                WriteFile(
                    handle,
                    Some(std::slice::from_raw_parts(buffer, len)),
                    None,
                    Some(&mut overlapped),
                )
            } else {
                ReadFile(
                    handle,
                    Some(std::slice::from_raw_parts_mut(buffer, len)),
                    None,
                    Some(&mut overlapped),
                )
            };
            if result.is_err()
                && result
                    .as_ref()
                    .err()
                    .is_some_and(|e| e.code().0 as u32 != 0x800703e5)
            {
                let _ = CloseHandle(event);
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "mpv pipe I/O failed",
                ));
            }
            let timed_out = WaitForSingleObject(event, timeout) == WAIT_TIMEOUT;
            if timed_out {
                let _ = CancelIoEx(handle, Some(&overlapped));
            }
            // Complete cancellation before freeing OVERLAPPED or its buffer.
            let done = GetOverlappedResult(handle, &overlapped, &mut count, true);
            let _ = CloseHandle(event);
            if done.is_ok() {
                Ok(count as usize)
            } else if timed_out {
                Err(io::ErrorKind::TimedOut.into())
            } else {
                Err(io::ErrorKind::BrokenPipe.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_config_and_rpc_contract() {
        let mut config = crate::config::AppConfig::default();
        config.external_mpv = Settings {
            mode: Mode::External,
            ..Default::default()
        };
        let serialized = serde_json::to_value(&config).unwrap();
        let decoded: crate::config::AppConfig = serde_json::from_value(serialized).unwrap();
        assert_eq!(decoded.external_mpv, config.external_mpv);
        assert!(decoded.validate().is_ok());
        let controls = crate::preferences_contract::preference_controls(&config);
        assert!(controls.iter().any(|c| c.key == "external_mpv.mode"));
        let request:crate::platform::interop::JsonRpcRequest=serde_json::from_value(json!({"jsonrpc":"2.0","id":1,"method":"pealayer.config.update","params":{"external_mpv":config.external_mpv}})).unwrap();
        let command = crate::platform::interop::command_from_json_rpc(&request)
            .unwrap()
            .unwrap();
        assert!(command.validate().is_ok());
    }
    #[test]
    fn observed_state_and_nested_properties() {
        let mut s = Status {
            connected: true,
            ..Default::default()
        };
        assert!(apply_event(
            &mut s,
            &json!({"event":"property-change","name":"track-list","data":[{"id":2,"type":"audio"}]})
        ));
        assert_eq!(lookup(&s.properties, "track-list/0/id"), Some(json!(2)));
        assert_eq!(lookup(&s.properties, "track-list/count"), Some(json!(1)));
        assert_eq!(s.catalog_revision, 1);
        apply_event(&mut s, &json!({"event":"playback-restart"}));
        assert_eq!(s.seek_revision, 1);
    }
    #[test]
    fn external_settings_reject_unsafe_endpoints() {
        assert!(Settings::default().validate().is_ok());
        assert!(
            Settings {
                mode: Mode::Remote,
                ..Default::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            Settings {
                endpoint: "https://example.test".into(),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    #[ignore = "Starts the explicitly configured real mpv with null outputs; no hardware"]
    fn live_duplex_ipc_and_muted_dual_preview() {
        fn wait(mut ready: impl FnMut() -> bool) {
            let deadline = Instant::now() + Duration::from_secs(8);
            while !ready() {
                assert!(Instant::now() < deadline, "Timed out: {:?}", status());
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        fn rpc(c: &mut Transport, command: Value) -> Value {
            c.write_json(&json!({"command":command,"request_id":987}))
                .unwrap();
            let mut data = Vec::new();
            let mut b = [0u8; 4096];
            loop {
                match c.read_bytes(&mut b) {
                    Ok(n) if n > 0 => {
                        data.extend_from_slice(&b[..n]);
                        while let Some(end) = data.iter().position(|b| *b == b'\n') {
                            let line: Vec<_> = data.drain(..=end).collect();
                            let v: Value = serde_json::from_slice(&line).unwrap();
                            if v["request_id"] == 987 {
                                assert_eq!(v["error"], "success");
                                return v["data"].clone();
                            }
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::TimedOut => {}
                    other => panic!("RPC read: {other:?}"),
                }
            }
        }
        let executable = std::env::var("PEALAYER_TEST_MPV")
            .expect("Set PEALAYER_TEST_MPV to the host's mpv executable");
        #[cfg(windows)]
        let endpoint = format!(r"\\.\pipe\pealayer-test-{}", uuid::Uuid::new_v4());
        #[cfg(unix)]
        let endpoint = std::env::temp_dir()
            .join(format!("pealayer-test-{}.sock", uuid::Uuid::new_v4()))
            .to_string_lossy()
            .into_owned();
        let mut cmd = std::process::Command::new(&executable);
        cmd.args([
            "--no-config",
            "--idle=yes",
            "--keep-open=yes",
            "--pause=yes",
            "--vo=null",
            "--ao=null",
            "--terminal=no",
        ])
        .arg(format!("--input-ipc-server={endpoint}"));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        struct Cleanup(std::process::Child);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                if self.0.try_wait().ok().flatten().is_none() {
                    let _ = self.0.kill();
                    let _ = self.0.wait();
                }
            }
        }
        let mut child = Cleanup(cmd.spawn().unwrap());
        let mpv = Box::leak(Box::new(
            libmpv2::Mpv::with_initializer(|i| {
                i.set_option("vo", "null")?;
                i.set_option("ao", "null")
            })
            .unwrap(),
        ));
        let ctx = eframe::egui::Context::default();
        configure(
            mpv,
            &ctx,
            Settings {
                mode: Mode::Remote,
                endpoint: endpoint.clone(),
                ..Default::default()
            },
        );
        wait(|| status().connected);
        let mut client = Transport::connect(&endpoint).unwrap();
        rpc(
            &mut client,
            json!([
                "loadfile",
                "av://lavfi:testsrc=duration=8:size=64x64:rate=10",
                "replace"
            ]),
        );
        wait(|| property("time-pos").is_some());
        rpc(&mut client, json!(["set_property", "volume", 37]));
        wait(|| property("volume").and_then(|v| v.as_f64()) == Some(37.0));
        crate::mpv::player::Player(mpv)
            .set_property("pause", false)
            .unwrap();
        wait(|| property("pause") == Some(json!(false)));
        wait(|| {
            property("time-pos")
                .and_then(|v| v.as_f64())
                .is_some_and(|t| t > 0.2)
        });
        crate::mpv::player::Player(mpv)
            .set_property("pause", true)
            .unwrap();
        wait(|| property("pause") == Some(json!(true)));
        assert_eq!(
            rpc(&mut client, json!(["get_property", "pause"])),
            json!(true)
        );
        assert!(
            mpv.get_property::<String>("path").is_err(),
            "Remote mode must not load the internal decoder"
        );
        let acknowledgements = status().acknowledged_commands;
        let start = Instant::now();
        for value in 30..62 {
            crate::mpv::player::Player(mpv)
                .set_property("volume", value as f64)
                .unwrap();
        }
        wait(|| {
            status().acknowledged_commands >= acknowledgements + 32
                && property("volume") == Some(json!(61.0))
        });
        eprintln!(
            "32 full-duplex commands acknowledged in {:?}",
            start.elapsed()
        );
        assert!(start.elapsed() < Duration::from_secs(2));
        configure(
            mpv,
            &ctx,
            Settings {
                mode: Mode::Dual,
                endpoint: endpoint.clone(),
                ..Default::default()
            },
        );
        wait(|| {
            status().connected
                && status().mode == Mode::Dual
                && mpv.get_property::<String>("path").is_ok()
        });
        wait(|| mpv.get_property::<bool>("mute").ok() == Some(true));
        wait(|| {
            let local = mpv.get_property::<f64>("time-pos").ok();
            let remote = property("time-pos").and_then(|v| v.as_f64());
            local.zip(remote).is_some_and(|(a, b)| (a - b).abs() < 0.25)
        });
        assert!(
            !status().owned_process,
            "An attached mpv must never be owned/terminated"
        );
        configure(mpv, &ctx, Settings::default());
        wait(|| status().mode == Mode::Internal);
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "Detach must leave an existing mpv running"
        );
        client.write_json(&json!({"command":["quit"]})).unwrap();
        child.0.wait().unwrap();
        configure(
            mpv,
            &ctx,
            Settings {
                mode: Mode::Remote,
                endpoint: endpoint.clone(),
                ..Default::default()
            },
        );
        wait(|| status().mode == Mode::Remote && !status().connected);
        assert!(
            send(json!(["set_property", "volume", 12])).is_err(),
            "Disconnected commands must not replay on reconnect"
        );
        child = Cleanup(cmd.spawn().unwrap());
        wait(|| status().connected && property("volume").is_some());
        let mut client = Transport::connect(&endpoint).unwrap();
        assert_ne!(
            rpc(&mut client, json!(["get_property", "volume"])),
            json!(12.0)
        );
        configure(mpv, &ctx, Settings::default());
        wait(|| status().mode == Mode::Internal);
        client.write_json(&json!({"command":["quit"]})).unwrap();
        child.0.wait().unwrap();
        // A managed idle player has no visible window or media; verify native
        // launch ownership and that switching back closes only this child.
        configure(
            mpv,
            &ctx,
            Settings {
                mode: Mode::External,
                executable,
                ..Default::default()
            },
        );
        wait(|| status().connected && status().owned_process);
        let owned_endpoint = status().endpoint;
        configure(mpv, &ctx, Settings::default());
        wait(|| status().mode == Mode::Internal);
        assert!(
            Transport::connect(&owned_endpoint).is_err(),
            "Managed mpv must close on detach"
        );
        #[cfg(unix)]
        let _ = std::fs::remove_file(&endpoint);
    }
}
