//! Hardware/player-independent download services. No libmpv, window or shell dependency.
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

pub mod aria2;
pub mod cache;
mod engines;
pub mod tools;
#[cfg(feature = "ui")]
pub mod ui;

const MAX_JOBS: usize = 512;
const USER_AGENT: &str = concat!("Pealayer-Downloader/", env!("CARGO_PKG_VERSION"));

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Queued,
    Connecting,
    Downloading,
    Paused,
    Complete,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    #[default]
    Native,
    Aria2,
    YtDlp,
    Ffmpeg,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaOperation {
    RemuxMp4,
    ExtractAudio,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Processing {
    source_id: String,
    operation: MediaOperation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddRequest {
    pub url: String,
    #[serde(default)]
    pub use_proxy: bool,
    #[serde(default)]
    pub proxy_url: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub engine: Engine,
    #[serde(default = "one")]
    pub connections: usize,
}

fn one() -> usize {
    1
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineSettings {
    pub aria2_endpoint: Option<String>,
    pub aria2_secret: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub source: String,
    pub filename: String,
    pub state: State,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed: u64,
    pub eta_seconds: Option<u64>,
    pub error: Option<String>,
    pub output: Option<String>,
    pub samples: VecDeque<u64>,
    #[serde(skip)]
    active: bool,
    #[serde(skip)]
    last_sample: Option<Instant>,
    #[serde(skip)]
    sample_bytes: u64,
    #[serde(skip)]
    resume_requested: bool,
    request: AddRequest,
    validator: Option<String>,
    #[serde(default)]
    engine_id: Option<String>,
    #[serde(default)]
    processing: Option<Processing>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicJob {
    pub id: String,
    pub source: String,
    pub filename: String,
    pub state: State,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed: u64,
    pub eta_seconds: Option<u64>,
    pub error: Option<String>,
    pub output: Option<String>,
    pub samples: VecDeque<u64>,
    pub actions: Vec<&'static str>,
    pub engine: Engine,
    pub connections: usize,
    pub streamable: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub jobs: Vec<PublicJob>,
    pub max_concurrent: usize,
    pub bytes_per_second: u64,
    pub active: usize,
    pub total_speed: u64,
    pub revision: u64,
    pub engines: Vec<Engine>,
    pub aria2_endpoint: Option<String>,
}

struct Inner {
    jobs: Vec<Job>,
    root: PathBuf,
    max_concurrent: usize,
    bytes_per_second: u64,
    revision: u64,
    _queue_lock: File,
    engines: EngineSettings,
    shutting_down: bool,
}

#[derive(Serialize, Deserialize)]
struct StoredQueue {
    jobs: Vec<Job>,
    max_concurrent: usize,
    bytes_per_second: u64,
    #[serde(default)]
    engines: EngineSettings,
}

/// Clones share one queue. One scheduler and at most eight bounded HTTP workers.
#[derive(Clone)]
pub struct Manager(Arc<Mutex<Inner>>);

pub fn default_root() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        return std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("Programs/Pealayer/downloads"))
            .ok_or_else(|| "LOCALAPPDATA is unavailable".into());
    }
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME")
            .map(|p| PathBuf::from(p).join("Library/Caches/Pealayer/downloads"))
            .ok_or_else(|| "HOME is unavailable".into());
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".cache")))
            .map(|p| p.join("pealayer/downloads"))
            .ok_or_else(|| "Cache directory is unavailable".into())
    }
}

impl Manager {
    pub fn open(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|_| "Cannot create download storage")?;
        let queue_lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("queue.lock"))
            .map_err(|_| "Cannot open queue lock")?;
        queue_lock.try_lock().map_err(|_| "Download queue is already owned by another process; connect to that instance instead")?;
        let queue = root.join("queue.json");
        let stored: StoredQueue = if queue.exists() {
            if fs::metadata(&queue).map_err(|_| "Cannot read queue")?.len() > 4 * 1024 * 1024 {
                return Err("Download queue exceeds storage limit".into());
            }
            serde_json::from_slice(&fs::read(queue).map_err(|_| "Cannot read queue")?)
                .map_err(|_| "Download queue is invalid; preserve it before recovery")?
        } else {
            StoredQueue {
                jobs: Vec::new(),
                max_concurrent: 2,
                bytes_per_second: 0,
                engines: EngineSettings::default(),
            }
        };
        if !(1..=8).contains(&stored.max_concurrent) {
            return Err("Invalid saved concurrency".into());
        }
        let mut jobs = stored.jobs;
        if jobs.len() > MAX_JOBS {
            return Err("Download queue exceeds job limit".into());
        }
        for job in &mut jobs {
            validate_request(&job.request)?;
            if uuid::Uuid::parse_str(&job.id).is_err()
                || safe_filename(&job.filename)? != job.filename
            {
                return Err("Invalid persisted download identity".into());
            }
            // Never silently start network activity after relaunch.
            if matches!(
                job.state,
                State::Queued | State::Connecting | State::Downloading
            ) {
                job.state = State::Paused;
            }
            job.active = false;
            job.speed = 0;
            job.eta_seconds = None;
            job.last_sample = None;
            job.resume_requested = false;
        }
        let manager = Self(Arc::new(Mutex::new(Inner {
            jobs,
            root,
            max_concurrent: stored.max_concurrent,
            bytes_per_second: stored.bytes_per_second,
            revision: 0,
            _queue_lock: queue_lock,
            engines: stored.engines,
            shutting_down: false,
        })));
        let weak = Arc::downgrade(&manager.0);
        thread::Builder::new()
            .name("download-queue".into())
            .spawn(move || {
                while let Some(shared) = weak.upgrade() {
                    let next = {
                        let mut inner = shared.lock().unwrap_or_else(|p| p.into_inner());
                        if !inner.shutting_down
                            && inner.jobs.iter().filter(|j| j.active).count() < inner.max_concurrent
                        {
                            inner
                                .jobs
                                .iter_mut()
                                .find(|j| j.state == State::Queued && !j.active)
                                .map(|job| {
                                    job.active = true;
                                    job.state = State::Connecting;
                                    job.error = None;
                                    job.id.clone()
                                })
                        } else {
                            None
                        }
                    };
                    if let Some(id) = next {
                        let manager = Self(shared);
                        let worker_manager = manager.clone();
                        let worker_id = id.clone();
                        if thread::Builder::new()
                            .name("download-http".into())
                            .spawn(move || {
                                let result = worker_manager.transfer(&worker_id);
                                worker_manager.finish(&worker_id, result);
                            })
                            .is_err()
                        {
                            manager.finish(&id, Err("Could not start transfer worker".into()));
                        }
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            })
            .map_err(|_| "Could not start download scheduler")?;
        let interrupted = {
            let mut inner = manager.0.lock().unwrap_or_else(|p| p.into_inner());
            inner
                .jobs
                .iter_mut()
                .filter(|job| {
                    job.request.engine == Engine::Aria2
                        && job.engine_id.is_some()
                        && !matches!(job.state, State::Complete | State::Cancelled)
                })
                .map(|job| {
                    job.state = State::Paused;
                    job.active = true;
                    job.id.clone()
                })
                .collect::<Vec<_>>()
        };
        for id in interrupted {
            let runner = manager.clone();
            thread::spawn(move || {
                let result = runner.transfer_aria2(&id);
                runner.finish(&id, result);
            });
        }
        Ok(manager)
    }

    pub fn snapshot(&self) -> Snapshot {
        let ffmpeg = tools::find("ffmpeg").is_some();
        let yt_dlp = tools::find("yt-dlp").is_some();
        let inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
        let jobs = inner
            .jobs
            .iter()
            .enumerate()
            .map(|(index, job)| {
                let mut actions = Vec::new();
                if matches!(
                    job.state,
                    State::Queued | State::Connecting | State::Downloading
                ) {
                    actions.push("pause");
                }
                if matches!(job.state, State::Paused | State::Failed) {
                    actions.push("resume");
                }
                if !matches!(job.state, State::Complete | State::Cancelled) {
                    actions.push("cancel");
                }
                if !job.active
                    && !(job.request.engine == Engine::Aria2
                        && job.engine_id.is_some()
                        && matches!(job.state, State::Paused | State::Failed))
                    && !inner.jobs.iter().any(|other| {
                        other
                            .processing
                            .as_ref()
                            .is_some_and(|p| p.source_id == job.id)
                            && !matches!(other.state, State::Complete | State::Cancelled)
                    })
                    && matches!(
                        job.state,
                        State::Paused | State::Failed | State::Complete | State::Cancelled
                    )
                {
                    actions.push("remove");
                }
                if index > 0 {
                    actions.push("move_up");
                }
                if index + 1 < inner.jobs.len() {
                    actions.push("move_down");
                }
                if job.state == State::Complete && ffmpeg {
                    actions.extend(["remux_mp4", "extract_audio"]);
                }
                PublicJob {
                    id: job.id.clone(),
                    source: job.source.clone(),
                    filename: job.filename.clone(),
                    state: job.state,
                    downloaded: job.downloaded,
                    total: job.total,
                    speed: job.speed,
                    eta_seconds: job.eta_seconds,
                    error: job.error.clone(),
                    output: job.output.clone(),
                    samples: job.samples.clone(),
                    actions,
                    engine: job.request.engine,
                    connections: job.request.connections,
                    streamable: job.total.is_some_and(|n| n > 0)
                        && job.downloaded > 0
                        && (job.state == State::Complete
                            || (job.request.engine == Engine::Native
                                && job.validator.is_some()
                                && job.state != State::Cancelled)),
                }
            })
            .collect();
        Snapshot {
            jobs,
            max_concurrent: inner.max_concurrent,
            bytes_per_second: inner.bytes_per_second,
            active: inner.jobs.iter().filter(|j| j.active).count(),
            total_speed: inner.jobs.iter().map(|j| j.speed).sum(),
            revision: inner.revision,
            engines: {
                let mut engines = vec![Engine::Native];
                if inner.engines.aria2_endpoint.is_some() {
                    engines.push(Engine::Aria2);
                }
                if yt_dlp {
                    engines.push(Engine::YtDlp);
                }
                if ffmpeg {
                    engines.push(Engine::Ffmpeg);
                }
                engines
            },
            aria2_endpoint: inner.engines.aria2_endpoint.clone(),
        }
    }

    pub fn add(&self, request: AddRequest) -> Result<String, String> {
        if request.engine == Engine::Ffmpeg {
            return Err("Select an owned completed job for media processing".into());
        }
        let parsed = validate_request(&request)?;
        let name = request.filename.clone().unwrap_or_else(|| {
            let last = parsed
                .path_segments()
                .and_then(|mut s| s.next_back())
                .filter(|s| !s.is_empty())
                .unwrap_or("download");
            percent_encoding::percent_decode_str(last)
                .decode_utf8_lossy()
                .into_owned()
        });
        let filename = safe_filename(&name)?;
        let source = format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""));
        let mut inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if inner.shutting_down {
            return Err("Download service is shutting down".into());
        }
        if request.engine == Engine::Aria2 && inner.engines.aria2_endpoint.is_none() {
            return Err("Configure an aria2 RPC endpoint first".into());
        }
        if request.engine == Engine::YtDlp && tools::find("yt-dlp").is_none() {
            return Err("yt-dlp is unavailable; install it in Pealayer/tools or PATH".into());
        }
        if inner.jobs.len() >= MAX_JOBS {
            return Err("Queue is full; remove completed jobs first".into());
        }
        let id = uuid::Uuid::new_v4().to_string();
        inner.jobs.push(Job {
            id: id.clone(),
            source,
            filename,
            state: State::Queued,
            downloaded: 0,
            total: None,
            speed: 0,
            eta_seconds: None,
            error: None,
            output: None,
            samples: VecDeque::new(),
            active: false,
            last_sample: None,
            sample_bytes: 0,
            resume_requested: false,
            request,
            validator: None,
            engine_id: None,
            processing: None,
        });
        inner.revision += 1;
        if let Err(error) = persist(&inner) {
            inner.jobs.pop();
            return Err(error);
        }
        Ok(id)
    }

    pub fn action(&self, id: &str, action: &str) -> Result<(), String> {
        if matches!(action, "remux_mp4" | "extract_audio") {
            self.process(
                id,
                if action == "remux_mp4" {
                    MediaOperation::RemuxMp4
                } else {
                    MediaOperation::ExtractAudio
                },
            )?;
            return Ok(());
        }
        let mut inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if inner.shutting_down {
            return Err("Download service is shutting down".into());
        }
        let index = inner
            .jobs
            .iter()
            .position(|j| j.id == id)
            .ok_or("Download does not exist")?;
        let saved = inner.jobs.clone();
        if matches!(action, "move_up" | "move_down") {
            let target = if action == "move_up" {
                index.checked_sub(1)
            } else {
                (index + 1 < inner.jobs.len()).then_some(index + 1)
            };
            if let Some(target) = target {
                inner.jobs.swap(index, target);
            } else {
                return Err("Already at queue boundary".into());
            }
            inner.revision += 1;
            if let Err(error) = persist(&inner) {
                inner.jobs = saved;
                return Err(error);
            }
            return Ok(());
        }
        let job = &mut inner.jobs[index];
        match action {
            "pause"
                if matches!(
                    job.state,
                    State::Queued | State::Connecting | State::Downloading
                ) =>
            {
                job.state = State::Paused
            }
            "resume" if matches!(job.state, State::Paused | State::Failed) => {
                if job.active {
                    job.resume_requested = true;
                } else {
                    job.state = State::Queued;
                }
            }
            "cancel" if !matches!(job.state, State::Complete | State::Cancelled) => {
                job.resume_requested = false;
                job.state = State::Cancelled;
            }
            "remove"
                if !job.active
                    && !(job.request.engine == Engine::Aria2
                        && job.engine_id.is_some()
                        && matches!(job.state, State::Paused | State::Failed))
                    && !saved.iter().any(|other| {
                        other.processing.as_ref().is_some_and(|p| p.source_id == id)
                            && !matches!(other.state, State::Complete | State::Cancelled)
                    })
                    && matches!(
                        job.state,
                        State::Cancelled | State::Complete | State::Failed | State::Paused
                    ) =>
            {
                inner.jobs.remove(index);
            }
            _ => return Err("Action is unavailable in the current download state".into()),
        }
        inner.revision += 1;
        if let Err(error) = persist(&inner) {
            inner.jobs = saved;
            return Err(error);
        }
        let reclaim_cancelled = inner
            .jobs
            .iter_mut()
            .find(|j| j.id == id)
            .filter(|job| {
                job.request.engine == Engine::Aria2
                    && job.state == State::Cancelled
                    && !job.active
                    && job.engine_id.is_some()
            })
            .map(|job| {
                job.active = true;
                job.id.clone()
            });
        drop(inner);
        if let Some(id) = reclaim_cancelled {
            let manager = self.clone();
            thread::spawn(move || {
                let result = manager.transfer_aria2(&id);
                manager.finish(&id, result);
            });
        }
        Ok(())
    }

    pub fn process(&self, source_id: &str, operation: MediaOperation) -> Result<String, String> {
        if tools::find("ffmpeg").is_none() {
            return Err("FFmpeg is unavailable; install it in Pealayer/tools or PATH".into());
        }
        let mut inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if inner.jobs.len() >= MAX_JOBS {
            return Err("Queue is full".into());
        }
        let source = inner
            .jobs
            .iter()
            .find(|j| j.id == source_id && j.state == State::Complete && j.output.is_some())
            .ok_or("Select a completed owned media file")?
            .clone();
        let id = uuid::Uuid::new_v4().to_string();
        let stem = std::path::Path::new(&source.filename)
            .file_stem()
            .and_then(|p| p.to_str())
            .unwrap_or("media");
        let extension = match operation {
            MediaOperation::RemuxMp4 => "mp4",
            MediaOperation::ExtractAudio => "m4a",
        };
        let mut request = source.request;
        request.engine = Engine::Ffmpeg;
        request.connections = 1;
        inner.jobs.push(Job {
            id: id.clone(),
            source: "Local media processing".into(),
            filename: safe_filename(&format!("{stem}.{extension}"))?,
            state: State::Queued,
            downloaded: 0,
            total: None,
            speed: 0,
            eta_seconds: None,
            error: None,
            output: None,
            samples: VecDeque::new(),
            active: false,
            last_sample: None,
            sample_bytes: 0,
            resume_requested: false,
            request,
            validator: None,
            engine_id: None,
            processing: Some(Processing {
                source_id: source_id.into(),
                operation,
            }),
        });
        if let Err(error) = persist(&inner) {
            inner.jobs.pop();
            return Err(error);
        }
        inner.revision += 1;
        Ok(id)
    }

    pub fn configure(&self, concurrent: usize, bytes_per_second: u64) -> Result<(), String> {
        if !(1..=8).contains(&concurrent) {
            return Err("Concurrent downloads must be 1..8".into());
        }
        let mut inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if (concurrent, bytes_per_second) != (inner.max_concurrent, inner.bytes_per_second)
            && inner
                .jobs
                .iter()
                .any(|j| j.active && j.request.engine == Engine::YtDlp)
        {
            return Err("Pause yt-dlp jobs before changing their bandwidth allocation; resume applies the new limit".into());
        }
        let previous = (inner.max_concurrent, inner.bytes_per_second);
        inner.max_concurrent = concurrent;
        inner.bytes_per_second = bytes_per_second;
        if let Err(error) = persist(&inner) {
            (inner.max_concurrent, inner.bytes_per_second) = previous;
            return Err(error);
        }
        inner.revision += 1;
        Ok(())
    }

    pub fn configure_engines(&self, settings: EngineSettings) -> Result<(), String> {
        if let Some(endpoint) = &settings.aria2_endpoint {
            let client = aria2::Client::new(endpoint, settings.aria2_secret.clone())?;
            client.version()?;
        } else if settings.aria2_secret.is_some() {
            return Err("aria2 secret requires an endpoint".into());
        }
        let mut inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if inner.jobs.iter().any(|j| {
            j.active
                || (j.request.engine == Engine::Aria2
                    && j.engine_id.is_some()
                    && !matches!(j.state, State::Complete | State::Cancelled))
        }) {
            return Err(
                "Cancel outstanding aria2 transfers before changing engine ownership".into(),
            );
        }
        let previous = inner.engines.clone();
        inner.engines = settings;
        if let Err(error) = persist(&inner) {
            inner.engines = previous;
            return Err(error);
        }
        inner.revision += 1;
        Ok(())
    }

    pub fn shutdown(&self, timeout: Duration) -> Result<(), String> {
        {
            let mut inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
            inner.shutting_down = true;
            for job in &mut inner.jobs {
                job.resume_requested = false;
                if matches!(
                    job.state,
                    State::Queued | State::Connecting | State::Downloading
                ) {
                    job.state = State::Paused;
                }
            }
            persist(&inner)?;
        }
        let deadline = Instant::now() + timeout;
        loop {
            let inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
            if !inner.jobs.iter().any(|job| job.active) {
                return Ok(());
            }
            drop(inner);
            if Instant::now() >= deadline {
                return Err("Some external engine workers could not acknowledge shutdown before the deadline".into());
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn finish(&self, id: &str, result: Result<(), String>) {
        let mut inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(job) = inner.jobs.iter_mut().find(|j| j.id == id) {
            job.active = false;
            job.speed = 0;
            job.eta_seconds = None;
            if job.state == State::Paused && job.resume_requested {
                job.resume_requested = false;
                job.state = State::Queued;
            } else if !matches!(job.state, State::Paused | State::Cancelled) {
                if let Err(error) = result {
                    job.state = State::Failed;
                    job.error = Some(error);
                } else {
                    job.state = State::Complete;
                }
            } else if let Err(error) = result {
                job.error = Some(error);
                if job.state == State::Cancelled {
                    job.state = State::Failed;
                }
            }
        }
        inner.revision += 1;
        if let Err(error) = persist(&inner) {
            if let Some(job) = inner.jobs.iter_mut().find(|j| j.id == id) {
                job.error = Some(error);
            }
        }
    }

    fn transfer(&self, id: &str) -> Result<(), String> {
        let engine = {
            let inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
            inner
                .jobs
                .iter()
                .find(|j| j.id == id)
                .ok_or("Download removed")?
                .request
                .engine
        };
        match engine {
            Engine::Aria2 => return self.transfer_aria2(id),
            Engine::YtDlp => return self.transfer_media(id),
            Engine::Ffmpeg => return self.transfer_ffmpeg(id),
            Engine::Native => {}
        }
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "Cannot initialize transfer runtime")?
            .block_on(self.transfer_async(id))
    }

    async fn transfer_async(&self, id: &str) -> Result<(), String> {
        let (request, directory, filename, validator) = {
            let inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
            let job = inner
                .jobs
                .iter()
                .find(|j| j.id == id)
                .ok_or("Download removed")?;
            (
                job.request.clone(),
                inner.root.join(&job.id),
                job.filename.clone(),
                job.validator.clone(),
            )
        };
        fs::create_dir_all(&directory).map_err(|_| "Cannot create download directory")?;
        let partial = directory.join("payload.part");
        let mut offset = fs::metadata(&partial).map_or(0, |m| m.len());
        // Resume requires a source identity, not just the previous length.
        if validator.is_none() {
            offset = 0;
        }
        let mut builder = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30));
        if !request.use_proxy {
            builder = builder.no_proxy();
        } else if let Some(proxy) = &request.proxy_url {
            builder =
                builder.proxy(reqwest::Proxy::all(proxy).map_err(|_| "Invalid proxy address")?);
        }
        let client = builder
            .build()
            .map_err(|_| "Cannot initialize HTTP client")?;
        let mut get = client
            .get(&request.url)
            .header(reqwest::header::ACCEPT_ENCODING, "identity");
        if offset > 0 {
            get = get
                .header(reqwest::header::RANGE, format!("bytes={offset}-"))
                .header(reqwest::header::IF_RANGE, validator.as_deref().unwrap());
        }
        let pending = get.send();
        tokio::pin!(pending);
        let mut response = loop {
            if matches!(self.job_state(id)?, State::Paused | State::Cancelled) {
                return Ok(());
            }
            match tokio::time::timeout(Duration::from_millis(250), &mut pending).await {
                Err(_) => continue,
                Ok(result) => {
                    break result
                        .map_err(|e| format!("HTTP connection failed: {}", e.without_url()))?;
                }
            }
        };
        if !response.status().is_success() {
            return Err(format!(
                "Server returned HTTP {}",
                response.status().as_u16()
            ));
        }
        let mut total = response.content_length();
        if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
            let range = response
                .headers()
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|h| h.to_str().ok())
                .ok_or("Missing Content-Range")?;
            let (start, end, size) = parse_content_range(range)?;
            if start != offset
                || response
                    .content_length()
                    .is_some_and(|n| n != end - start + 1)
            {
                return Err("Server returned an inconsistent byte range".into());
            }
            total = Some(size);
        } else {
            offset = 0;
        }
        let current_validator = response
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|h| h.to_str().ok())
            .filter(|v| !v.starts_with("W/"))
            .or_else(|| {
                response
                    .headers()
                    .get(reqwest::header::LAST_MODIFIED)
                    .and_then(|h| h.to_str().ok())
            })
            .map(str::to_owned);
        if offset > 0 && current_validator.as_ref() != validator.as_ref() {
            return Err("Source changed; resume refused".into());
        }
        let opened_path = if offset == 0 {
            directory.join("payload.restart")
        } else {
            partial.clone()
        };
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(offset > 0)
            .truncate(offset == 0)
            .open(&opened_path)
            .map_err(|_| "Cannot open partial download")?;
        // Existing streaming readers keep their old inode when a full response restarts this job.
        if offset == 0 {
            fs::rename(&opened_path, &partial)
                .map_err(|_| "Cannot commit fresh partial download")?;
        }
        {
            let mut inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
            let job = inner
                .jobs
                .iter_mut()
                .find(|j| j.id == id)
                .ok_or("Download removed")?;
            if matches!(job.state, State::Paused | State::Cancelled) {
                return Ok(());
            }
            job.downloaded = offset;
            job.total = total;
            job.validator = current_validator;
            job.state = State::Downloading;
            job.last_sample = Some(Instant::now());
            job.sample_bytes = offset;
            persist(&inner)?;
        }
        let mut transferred = 0_u64;
        let mut next_read = Instant::now();
        let mut previous_limit = 0;
        let mut last_received = Instant::now();
        loop {
            let limit = {
                let inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
                let job = inner
                    .jobs
                    .iter()
                    .find(|j| j.id == id)
                    .ok_or("Download removed")?;
                if matches!(job.state, State::Paused | State::Cancelled) {
                    file.sync_all()
                        .map_err(|_| "Could not flush partial download")?;
                    return Ok(());
                }
                if inner.bytes_per_second == 0 {
                    0
                } else {
                    (inner.bytes_per_second / inner.max_concurrent as u64).max(1)
                }
            };
            if limit != previous_limit {
                next_read = Instant::now();
                previous_limit = limit;
            }
            if limit > 0 && next_read > Instant::now() {
                // Intentional bandwidth pacing is not a server inactivity fault.
                last_received = Instant::now();
                tokio::time::sleep(Duration::from_millis(20)).await;
                continue;
            }
            if last_received.elapsed() > Duration::from_secs(30) {
                return Err("Server stopped sending data; partial download retained".into());
            }
            let chunk = match tokio::time::timeout(Duration::from_millis(250), response.chunk())
                .await
            {
                Err(_) => continue,
                Ok(Err(_)) => {
                    return Err(
                        "Transfer interrupted; partial data retained for validated resume".into(),
                    );
                }
                Ok(Ok(None)) => break,
                Ok(Ok(Some(chunk))) => chunk,
            };
            file.write_all(&chunk)
                .map_err(|_| "Download storage write failed")?;
            last_received = Instant::now();
            transferred += chunk.len() as u64;
            if limit > 0 {
                next_read =
                    Instant::now() + Duration::from_secs_f64(chunk.len() as f64 / limit as f64);
            }
            let mut inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
            let job = inner
                .jobs
                .iter_mut()
                .find(|j| j.id == id)
                .ok_or("Download removed")?;
            job.downloaded = offset + transferred;
            if total.is_some_and(|n| job.downloaded > n) {
                return Err("Server exceeded declared file size".into());
            }
            let elapsed = job.last_sample.unwrap().elapsed();
            if elapsed >= Duration::from_millis(250) {
                job.speed =
                    ((job.downloaded - job.sample_bytes) as f64 / elapsed.as_secs_f64()) as u64;
                job.eta_seconds = total.and_then(|n| {
                    (job.speed > 0).then(|| n.saturating_sub(job.downloaded) / job.speed)
                });
                if job.samples.len() == 120 {
                    job.samples.pop_front();
                }
                job.samples.push_back(job.speed);
                job.sample_bytes = job.downloaded;
                job.last_sample = Some(Instant::now());
                inner.revision += 1;
            }
        }
        if total.is_some_and(|n| n != offset + transferred) {
            return Err("Download ended before expected file size".into());
        }
        file.sync_all()
            .map_err(|_| "Could not flush completed download")?;
        drop(file);
        let destination = directory.join(filename);
        let mut inner = self.0.lock().unwrap_or_else(|p| p.into_inner());
        let job = inner
            .jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or("Download removed")?;
        if matches!(job.state, State::Paused | State::Cancelled) {
            return Ok(());
        }
        if destination.exists() {
            return Err("Completed destination already exists; refusing overwrite".into());
        }
        // link is atomic and fails if another process creates the target; unlike rename it never overwrites.
        fs::hard_link(&partial, &destination)
            .map_err(|_| "Cannot commit completed download without overwriting")?;
        fs::remove_file(&partial)
            .map_err(|_| "Completed download committed but partial cleanup failed")?;
        job.output = Some(destination.to_string_lossy().into_owned());
        job.state = State::Complete;
        Ok(())
    }
}

fn persist(inner: &Inner) -> Result<(), String> {
    let staged = inner.root.join("queue.pending.json");
    let bytes = serde_json::to_vec(&StoredQueue {
        jobs: inner.jobs.clone(),
        max_concurrent: inner.max_concurrent,
        bytes_per_second: inner.bytes_per_second,
        engines: inner.engines.clone(),
    })
    .map_err(|_| "Cannot serialize queue")?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Download queue exceeds storage limit".into());
    }
    let mut file = File::create(&staged).map_err(|_| "Cannot stage queue")?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "Cannot persist queue")?;
    drop(file);
    fs::rename(staged, inner.root.join("queue.json")).map_err(|_| "Cannot commit queue".to_string())
}

fn validate_request(request: &AddRequest) -> Result<url::Url, String> {
    if !(1..=16).contains(&request.connections) {
        return Err("Connections must be 1..16".into());
    }
    if request.engine == Engine::Native && request.connections != 1 {
        return Err(
            "Native segmented downloads are not enabled; select aria2 for multiple connections"
                .into(),
        );
    }
    if request.engine == Engine::YtDlp && request.connections != 1 {
        return Err("yt-dlp jobs use the extractor's connection policy".into());
    }
    if request.url.len() > 16_384 {
        return Err("URL is too long".into());
    }
    let parsed = url::Url::parse(&request.url).map_err(|_| "Invalid download URL")?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err("Use an HTTP(S) URL without embedded credentials".into());
    }
    Ok(parsed)
}

fn safe_filename(value: &str) -> Result<String, String> {
    if value.is_empty()
        || value.len() > 200
        || value
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || value.ends_with(['.', ' '])
        || matches!(value, "." | "..")
    {
        return Err("Filename must be a safe single path component".into());
    }
    let stem = value.split('.').next().unwrap().to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem[3..].parse::<u8>().is_ok_and(|n| (1..=9).contains(&n))
    {
        return Err("Filename is reserved by the operating system".into());
    }
    Ok(value.into())
}

fn parse_content_range(value: &str) -> Result<(u64, u64, u64), String> {
    let (range, total) = value
        .strip_prefix("bytes ")
        .and_then(|v| v.split_once('/'))
        .ok_or("Invalid Content-Range")?;
    let (start, end) = range.split_once('-').ok_or("Invalid Content-Range")?;
    let parse = |v: &str| {
        v.parse::<u64>()
            .map_err(|_| "Invalid Content-Range".to_string())
    };
    let (start, end, total) = (parse(start)?, parse(end)?, parse(total)?);
    if start > end || end >= total {
        return Err("Invalid Content-Range bounds".into());
    }
    Ok((start, end, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn request(url: &str) -> AddRequest {
        AddRequest {
            url: url.into(),
            use_proxy: false,
            proxy_url: None,
            filename: None,
            engine: Engine::Native,
            connections: 1,
        }
    }

    #[test]
    fn only_http_sources_without_embedded_credentials_are_accepted() {
        for value in [
            "file:///etc/passwd",
            "ftp://example.test/file",
            "https://user:secret@example.test/file",
            "not a URL",
        ] {
            assert!(validate_request(&request(value)).is_err(), "{value}");
        }
        assert!(validate_request(&request("https://example.test/file?signature=private")).is_ok());
    }

    #[test]
    fn filename_is_one_portable_component() {
        for value in [
            "",
            "..",
            "../media",
            "folder\\media",
            "file:stream",
            "NUL.txt",
            "COM9.mkv",
            "file.",
            "file ",
            "line\nbreak",
        ] {
            assert!(safe_filename(value).is_err(), "{value}");
        }
        assert_eq!(safe_filename("فیلم.mp4").unwrap(), "فیلم.mp4");
        assert!(safe_filename("COM10.mkv").is_ok());
    }

    #[test]
    fn range_validation_rejects_missing_unknown_or_inconsistent_bounds() {
        assert_eq!(parse_content_range("bytes 12-23/24").unwrap(), (12, 23, 24));
        for value in [
            "bytes */24",
            "bytes 0-9/*",
            "bytes 9-2/24",
            "bytes 0-24/24",
            "bytes 0-0/0",
            "items 0-3/4",
        ] {
            assert!(parse_content_range(value).is_err(), "{value}");
        }
    }

    #[test]
    fn snapshots_never_disclose_source_queries_or_proxy_secrets() {
        let job = Job {
            id: uuid::Uuid::new_v4().to_string(),
            source: "https://example.test".into(),
            filename: "media.mkv".into(),
            state: State::Paused,
            downloaded: 0,
            total: None,
            speed: 0,
            eta_seconds: None,
            error: None,
            output: None,
            samples: VecDeque::new(),
            active: false,
            last_sample: None,
            sample_bytes: 0,
            resume_requested: false,
            request: AddRequest {
                proxy_url: Some("https://proxy-user:proxy-secret@proxy.test".into()),
                ..request("https://example.test/media?signature=private")
            },
            validator: None,
            engine_id: None,
            processing: None,
        };
        let manager = Manager(Arc::new(Mutex::new(Inner {
            jobs: vec![job],
            root: PathBuf::new(),
            max_concurrent: 2,
            bytes_per_second: 0,
            revision: 0,
            _queue_lock: File::open(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
                .unwrap(),
            engines: EngineSettings::default(),
            shutting_down: false,
        })));
        let json = serde_json::to_string(&manager.snapshot()).unwrap();
        assert!(!json.contains("signature"));
        assert!(!json.contains("proxy-secret"));
        assert!(!json.contains("proxy-user"));
        assert!(json.contains("example.test"));
    }

    #[test]
    fn media_tool_progress_is_structured_not_inferred_from_human_text() {
        assert_eq!(
            tools::yt_dlp_progress("download:{\"downloaded_bytes\":12}").unwrap()["downloaded_bytes"],
            12
        );
        assert!(tools::yt_dlp_progress("[download] 50%").is_none());
        assert_eq!(
            tools::ffmpeg_progress("out_time_us=500000"),
            Some(("out_time_us", "500000"))
        );
        assert!(tools::ffmpeg_progress("frame=300").is_none());
    }

    #[test]
    fn local_http_transfer_commits_the_exact_payload() {
        let payload = b"Pealayer deterministic download fixture";
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 2048];
            let read = stream.read(&mut request).unwrap();
            assert!(String::from_utf8_lossy(&request[..read]).starts_with("GET /payload.bin "));
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nETag: \"fixture-v1\"\r\nConnection: close\r\n\r\n",
                payload.len()
            )
            .unwrap();
            stream.write_all(payload).unwrap();
        });

        let root =
            std::env::temp_dir().join(format!("pealayer-download-test-{}", uuid::Uuid::new_v4()));
        let manager = Manager::open(root.clone()).unwrap();
        let id = manager
            .add(request(&format!("http://{address}/payload.bin")))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let output = loop {
            let job = manager
                .snapshot()
                .jobs
                .into_iter()
                .find(|job| job.id == id)
                .unwrap();
            match job.state {
                State::Complete => break PathBuf::from(job.output.unwrap()),
                State::Failed => panic!("download failed: {:?}", job.error),
                _ if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
                _ => panic!("download did not finish before the fixture deadline"),
            }
        };
        server.join().unwrap();
        assert_eq!(fs::read(output).unwrap(), payload);
        drop(manager);
        fs::remove_dir_all(root).unwrap();
    }
}
