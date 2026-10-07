use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_UPDATE_BYTES: u64 = 512 * 1024 * 1024;
const UPDATE_CHUNK_BYTES: usize = 768 * 1024;
const HELPER_ARGUMENT: &str = "--pealayer-update-helper";
const HEALTH_PATH_ENV: &str = "PEALAYER_UPDATE_HEALTH_PATH";
const HEALTH_TOKEN_ENV: &str = "PEALAYER_UPDATE_HEALTH_TOKEN";
const HELPER_PATH_ENV: &str = "PEALAYER_UPDATE_HELPER_PATH";
const JOURNAL_PATH_ENV: &str = "PEALAYER_UPDATE_JOURNAL_PATH";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateStatus {
    pub operation_id: Option<String>,
    pub state: String,
    pub source: Option<String>,
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
    pub sha256: Option<String>,
    pub version: Option<String>,
    pub message: String,
    pub error: Option<String>,
}

impl Default for UpdateStatus {
    fn default() -> Self {
        Self {
            operation_id: None,
            state: "idle".to_string(),
            source: None,
            bytes_done: 0,
            bytes_total: None,
            sha256: None,
            version: None,
            message: "No update in progress".to_string(),
            error: None,
        }
    }
}

impl UpdateStatus {
    pub fn active(&self) -> bool {
        matches!(
            self.state.as_str(),
            "receiving" | "downloading" | "verifying" | "staged" | "restarting"
        )
    }

    pub fn progress_percent(&self) -> Option<f64> {
        self.bytes_total
            .filter(|total| *total > 0)
            .map(|total| (self.bytes_done as f64 * 100.0 / total as f64).clamp(0.0, 100.0))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub format: String,
    pub service: String,
    pub version: String,
    pub git_commit: String,
    pub git_dirty: bool,
    pub platform: String,
    pub arch: String,
    pub sha256: String,
    pub size: u64,
    pub artifact_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub libmpv_runtime: Option<LibmpvRuntimeIdentity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LibmpvRuntimeIdentity {
    pub file_name: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeginUploadRequest {
    pub sha256: String,
    pub size: u64,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
    #[serde(default)]
    pub libmpv_runtime: Option<LibmpvRuntimeIdentity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishUploadRequest {
    pub operation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchUpdateRequest {
    pub url: String,
    #[serde(default)]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateJournal {
    format: String,
    operation_id: String,
    parent_pid: u32,
    current_path: PathBuf,
    staged_path: PathBuf,
    backup_path: PathBuf,
    helper_path: PathBuf,
    journal_path: PathBuf,
    health_path: PathBuf,
    health_token: String,
    current_sha256: String,
    replacement_sha256: String,
    arguments: Vec<String>,
    working_directory: PathBuf,
}

struct ActiveUpload {
    operation_id: String,
    path: PathBuf,
    file: File,
    hasher: Sha256,
    expected_sha256: String,
    expected_size: u64,
    received: u64,
    version: Option<String>,
}

#[derive(Default)]
struct UpdateInner {
    status: UpdateStatus,
    upload: Option<ActiveUpload>,
}

#[derive(Clone, Default)]
pub struct UpdateManager {
    inner: Arc<Mutex<UpdateInner>>,
    gui_context: Arc<Mutex<Option<eframe::egui::Context>>>,
}

static GLOBAL_MANAGER: OnceLock<UpdateManager> = OnceLock::new();

pub fn manager() -> UpdateManager {
    GLOBAL_MANAGER.get_or_init(UpdateManager::default).clone()
}

impl UpdateManager {
    pub fn register_gui_context(&self, context: eframe::egui::Context) {
        if let Ok(mut current) = self.gui_context.lock() {
            *current = Some(context);
        }
    }

    fn wake_gui(&self) {
        let context = self
            .gui_context
            .lock()
            .ok()
            .and_then(|current| current.clone());
        if let Some(context) = context {
            context.request_repaint();
        }
    }

    fn dispatch_quit(
        &self,
        command_tx: &std::sync::mpsc::Sender<crate::platform::interop::InteropCommand>,
    ) {
        if command_tx
            .send(crate::platform::interop::InteropCommand::Quit)
            .is_ok()
        {
            // An idle/paused eframe window may have no more frames scheduled.
            // Wake AFTER enqueueing Quit, not before the 350 ms response delay.
            self.wake_gui();
        } else {
            let status = self.status();
            self.fail(
                status.operation_id,
                status.source,
                "application dispatcher is unavailable",
            );
        }
    }

    pub fn status(&self) -> UpdateStatus {
        self.inner
            .lock()
            .map(|inner| inner.status.clone())
            .unwrap_or_else(|_| UpdateStatus {
                state: "error".to_string(),
                message: "Update coordinator unavailable".to_string(),
                error: Some("update state lock is poisoned".to_string()),
                ..UpdateStatus::default()
            })
    }

    fn replace_status(&self, status: UpdateStatus) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.status = status;
        }
        self.wake_gui();
    }

    fn fail(&self, operation_id: Option<String>, source: Option<String>, error: impl ToString) {
        let error = error.to_string();
        self.replace_status(UpdateStatus {
            operation_id,
            state: "failed".to_string(),
            source,
            message: "Update failed".to_string(),
            error: Some(error),
            ..UpdateStatus::default()
        });
    }

    pub fn begin_upload(&self, request: BeginUploadRequest) -> Result<UpdateStatus, String> {
        let expected_sha256 = normalize_sha256(&request.sha256)?;
        validate_update_size(request.size)?;
        if let Some(platform) = request.platform.as_deref()
            && !platform.eq_ignore_ascii_case(std::env::consts::OS)
        {
            return Err(format!(
                "update platform {platform:?} does not match {:?}",
                std::env::consts::OS
            ));
        }
        if let Some(arch) = request.arch.as_deref()
            && !arch.eq_ignore_ascii_case(std::env::consts::ARCH)
        {
            return Err(format!(
                "update architecture {arch:?} does not match {:?}",
                std::env::consts::ARCH
            ));
        }
        validate_libmpv_compatibility(
            request.libmpv_runtime.as_ref(),
            adjacent_libmpv_runtime_identity()?.as_ref(),
        )?;

        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "update state lock is poisoned".to_string())?;
        if inner.status.active() || inner.upload.is_some() {
            return Err("another update operation is already active".to_string());
        }
        let operation_id = format!("update-{}", uuid::Uuid::new_v4());
        let path = update_scratch_path(&operation_id, "download")?;
        let file = create_update_file(&path, true)
            .map_err(|error| format!("create staged update: {error}"))?;
        let status = UpdateStatus {
            operation_id: Some(operation_id.clone()),
            state: "receiving".to_string(),
            source: Some("peer-upload".to_string()),
            bytes_done: 0,
            bytes_total: Some(request.size),
            sha256: Some(expected_sha256.clone()),
            version: request.version.clone(),
            message: "Receiving verified update from peer".to_string(),
            error: None,
        };
        inner.upload = Some(ActiveUpload {
            operation_id,
            path,
            file,
            hasher: Sha256::new(),
            expected_sha256,
            expected_size: request.size,
            received: 0,
            version: request.version,
        });
        inner.status = status.clone();
        drop(inner);
        self.wake_gui();
        Ok(status)
    }

    pub fn append_chunk(
        &self,
        operation_id: &str,
        offset: u64,
        bytes: &[u8],
    ) -> Result<UpdateStatus, String> {
        if bytes.is_empty() || bytes.len() > UPDATE_CHUNK_BYTES {
            return Err(format!(
                "update chunk must contain 1 to {UPDATE_CHUNK_BYTES} bytes"
            ));
        }
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "update state lock is poisoned".to_string())?;
        let upload = inner
            .upload
            .as_mut()
            .ok_or_else(|| "no peer update upload is active".to_string())?;
        if upload.operation_id != operation_id {
            return Err("update operation id does not match the active upload".to_string());
        }
        if upload.received != offset {
            return Err(format!(
                "update chunk offset {offset} does not match next byte {}",
                upload.received
            ));
        }
        let next = upload.received.saturating_add(bytes.len() as u64);
        if next > upload.expected_size {
            return Err("update upload exceeds the declared size".to_string());
        }
        upload
            .file
            .write_all(bytes)
            .map_err(|error| format!("write update chunk: {error}"))?;
        upload.hasher.update(bytes);
        upload.received = next;
        let expected_size = upload.expected_size;
        inner.status.bytes_done = next;
        inner.status.message = format!(
            "Received {} of {}",
            format_bytes(next),
            format_bytes(expected_size)
        );
        let status = inner.status.clone();
        drop(inner);
        self.wake_gui();
        Ok(status)
    }

    pub fn finish_upload(
        &self,
        operation_id: &str,
        command_tx: std::sync::mpsc::Sender<crate::platform::interop::InteropCommand>,
    ) -> Result<UpdateStatus, String> {
        let upload = {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| "update state lock is poisoned".to_string())?;
            let upload = inner
                .upload
                .take()
                .ok_or_else(|| "no peer update upload is active".to_string())?;
            if upload.operation_id != operation_id {
                inner.upload = Some(upload);
                return Err("update operation id does not match the active upload".to_string());
            }
            inner.status.state = "verifying".to_string();
            inner.status.message = "Verifying staged update".to_string();
            upload
        };
        self.wake_gui();
        let source = Some("peer-upload".to_string());
        match finalize_upload_file(upload) {
            Ok((path, expected_sha256, version, size)) => self.stage_and_restart(
                operation_id.to_string(),
                source,
                path,
                expected_sha256,
                version,
                size,
                command_tx,
            ),
            Err(error) => {
                self.fail(Some(operation_id.to_string()), source, &error);
                Err(error)
            }
        }
    }

    pub fn abort_upload(&self, operation_id: &str) -> Result<UpdateStatus, String> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "update state lock is poisoned".to_string())?;
        let upload = inner
            .upload
            .take()
            .ok_or_else(|| "no peer update upload is active".to_string())?;
        if upload.operation_id != operation_id {
            inner.upload = Some(upload);
            return Err("update operation id does not match the active upload".to_string());
        }
        let ActiveUpload {
            path,
            file,
            received,
            expected_size,
            expected_sha256,
            version,
            ..
        } = upload;
        drop(file);
        if let Err(error) = fs::remove_file(path) {
            let error = format!("remove aborted update: {error}");
            drop(inner);
            self.fail(
                Some(operation_id.to_string()),
                Some("peer-upload".into()),
                &error,
            );
            return Err(error);
        }
        inner.status = UpdateStatus {
            operation_id: Some(operation_id.to_string()),
            state: "aborted".to_string(),
            source: Some("peer-upload".to_string()),
            bytes_done: received,
            bytes_total: Some(expected_size),
            sha256: Some(expected_sha256),
            version,
            message: "Peer update upload aborted and staging file removed".to_string(),
            error: None,
        };
        let status = inner.status.clone();
        drop(inner);
        self.wake_gui();
        Ok(status)
    }

    pub fn fetch_and_apply(
        &self,
        request: FetchUpdateRequest,
        command_tx: std::sync::mpsc::Sender<crate::platform::interop::InteropCommand>,
    ) -> Result<UpdateStatus, String> {
        let source = request.url.trim().to_string();
        if source.is_empty() {
            return Err("update URL is required".to_string());
        }
        let parsed =
            url::Url::parse(&source).map_err(|error| format!("invalid update URL: {error}"))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err("update URL must use HTTP or HTTPS".to_string());
        }
        let expected = request
            .sha256
            .as_deref()
            .map(normalize_sha256)
            .transpose()?;
        let operation_id = format!("update-{}", uuid::Uuid::new_v4());
        {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| "update state lock is poisoned".to_string())?;
            if inner.status.active() || inner.upload.is_some() {
                return Err("another update operation is already active".to_string());
            }
            inner.status = UpdateStatus {
                operation_id: Some(operation_id.clone()),
                state: "downloading".to_string(),
                source: Some(source.clone()),
                message: "Resolving update source".to_string(),
                sha256: expected.clone(),
                ..UpdateStatus::default()
            };
        }
        let manager = self.clone();
        self.wake_gui();
        let returned = self.status();
        std::thread::spawn(move || {
            match download_update(&manager, &operation_id, &source, expected.as_deref()) {
                Ok((path, digest, version, size)) => {
                    if let Err(error) = manager.stage_and_restart(
                        operation_id.clone(),
                        Some(source.clone()),
                        path,
                        digest,
                        version,
                        size,
                        command_tx,
                    ) {
                        manager.fail(Some(operation_id), Some(source), error);
                    }
                }
                Err(error) => manager.fail(Some(operation_id), Some(source), error),
            }
        });
        Ok(returned)
    }

    #[allow(clippy::too_many_arguments)]
    fn stage_and_restart(
        &self,
        operation_id: String,
        source: Option<String>,
        path: PathBuf,
        expected_sha256: String,
        version: Option<String>,
        size: u64,
        command_tx: std::sync::mpsc::Sender<crate::platform::interop::InteropCommand>,
    ) -> Result<UpdateStatus, String> {
        inspect_executable(&path)?;
        let current_sha = current_manifest()?.sha256;
        if current_sha == expected_sha256 {
            let _ = fs::remove_file(&path);
            let status = UpdateStatus {
                operation_id: Some(operation_id),
                state: "current".to_string(),
                source,
                bytes_done: size,
                bytes_total: Some(size),
                sha256: Some(expected_sha256),
                version,
                message: "This exact build is already running".to_string(),
                error: None,
            };
            self.replace_status(status.clone());
            return Ok(status);
        }
        prepare_self_update(&operation_id, &path, &expected_sha256)?;
        let status = UpdateStatus {
            operation_id: Some(operation_id),
            state: "restarting".to_string(),
            source,
            bytes_done: size,
            bytes_total: Some(size),
            sha256: Some(expected_sha256),
            version,
            message: "Update verified; gracefully closing before replacement".to_string(),
            error: None,
        };
        self.replace_status(status.clone());
        let manager = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(350));
            manager.dispatch_quit(&command_tx);
        });
        Ok(status)
    }
}

fn finalize_upload_file(
    mut upload: ActiveUpload,
) -> Result<(PathBuf, String, Option<String>, u64), String> {
    upload.file.flush().map_err(|error| error.to_string())?;
    upload.file.sync_all().map_err(|error| error.to_string())?;
    if upload.received != upload.expected_size {
        let _ = fs::remove_file(&upload.path);
        return Err(format!(
            "update upload is incomplete: received {}, expected {}",
            upload.received, upload.expected_size
        ));
    }
    let actual = format!("{:x}", upload.hasher.finalize());
    if actual != upload.expected_sha256 {
        let _ = fs::remove_file(&upload.path);
        return Err(format!(
            "update SHA-256 mismatch: expected {}, received {actual}",
            upload.expected_sha256
        ));
    }
    Ok((upload.path, actual, upload.version, upload.expected_size))
}

fn download_update(
    manager: &UpdateManager,
    operation_id: &str,
    source: &str,
    expected_sha256: Option<&str>,
) -> Result<(PathBuf, String, Option<String>, u64), String> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(15 * 60))
        .user_agent(format!("Pealayer/{} updater", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("create update client: {error}"))?;
    let (artifact_url, manifest_hash, version, manifest_size) =
        resolve_update_source(&client, source)?;
    let expected = expected_sha256
        .map(str::to_string)
        .or(manifest_hash)
        .map(|value| normalize_sha256(&value))
        .transpose()?;
    let mut response = client
        .get(&artifact_url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("download update: {error}"))?;
    let content_length = response.content_length().or(manifest_size);
    if let Some(size) = content_length {
        validate_update_size(size)?;
    }
    let path = update_scratch_path(operation_id, "download")?;
    let mut file =
        create_update_file(&path, true).map_err(|error| format!("create update file: {error}"))?;
    let mut hash = Sha256::new();
    let mut bytes_done = 0_u64;
    let mut buffer = vec![0_u8; 128 * 1024];
    loop {
        let count = response
            .read(&mut buffer)
            .map_err(|error| format!("read update response: {error}"))?;
        if count == 0 {
            break;
        }
        bytes_done = bytes_done.saturating_add(count as u64);
        validate_update_size(bytes_done)?;
        file.write_all(&buffer[..count])
            .map_err(|error| format!("write downloaded update: {error}"))?;
        hash.update(&buffer[..count]);
        if let Ok(mut inner) = manager.inner.lock() {
            inner.status.bytes_done = bytes_done;
            inner.status.bytes_total = content_length;
            inner.status.message = match content_length {
                Some(total) => format!(
                    "Downloaded {} of {}",
                    format_bytes(bytes_done),
                    format_bytes(total)
                ),
                None => format!("Downloaded {}", format_bytes(bytes_done)),
            };
        }
        manager.wake_gui();
    }
    file.flush().map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    if let Some(size) = content_length
        && size != bytes_done
    {
        let _ = fs::remove_file(&path);
        return Err(format!(
            "download size mismatch: expected {size} bytes, received {bytes_done}"
        ));
    }
    let actual = format!("{:x}", hash.finalize());
    if let Some(expected) = expected
        && actual != expected
    {
        let _ = fs::remove_file(&path);
        return Err(format!(
            "download SHA-256 mismatch: expected {expected}, received {actual}"
        ));
    }
    let (path, actual, bytes_done) = materialize_download(operation_id, &artifact_url, path)
        .map_err(|error| format!("prepare downloaded update: {error}"))?;
    if let Ok(mut inner) = manager.inner.lock() {
        inner.status.state = "verifying".to_string();
        inner.status.sha256 = Some(actual.clone());
        inner.status.version = version.clone();
        inner.status.message = "Verifying downloaded executable".to_string();
    }
    manager.wake_gui();
    Ok((path, actual, version, bytes_done))
}

fn resolve_update_source(
    client: &reqwest::blocking::Client,
    source: &str,
) -> Result<(String, Option<String>, Option<String>, Option<u64>), String> {
    if let Some(api_url) = github_release_api_url(source) {
        #[derive(Deserialize)]
        struct GithubAsset {
            name: String,
            browser_download_url: String,
            size: u64,
            digest: Option<String>,
        }
        #[derive(Deserialize)]
        struct GithubRelease {
            tag_name: Option<String>,
            assets: Vec<GithubAsset>,
        }
        let release: GithubRelease = client
            .get(api_url)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| format!("resolve GitHub release: {error}"))?
            .json()
            .map_err(|error| format!("decode GitHub release: {error}"))?;
        let asset = release
            .assets
            .into_iter()
            .find(|asset| release_asset_matches(&asset.name))
            .ok_or_else(|| {
                "GitHub release has no Pealayer executable for this platform".to_string()
            })?;
        return Ok((
            asset.browser_download_url,
            asset
                .digest
                .as_deref()
                .and_then(|value| value.strip_prefix("sha256:"))
                .map(str::to_string),
            release.tag_name,
            Some(asset.size),
        ));
    }

    let probe = client
        .get(source)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("resolve update URL: {error}"))?;
    let content_type = probe
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if content_type.contains("json") || source.to_ascii_lowercase().ends_with(".json") {
        let base = probe.url().clone();
        let manifest: UpdateManifest = probe
            .json()
            .map_err(|error| format!("decode update manifest: {error}"))?;
        if manifest.format != "pealayer-update" {
            return Err("unsupported update manifest format".to_string());
        }
        if manifest.platform != std::env::consts::OS || manifest.arch != std::env::consts::ARCH {
            return Err(format!(
                "manifest targets {}/{}, this host is {}/{}",
                manifest.platform,
                manifest.arch,
                std::env::consts::OS,
                std::env::consts::ARCH
            ));
        }
        validate_libmpv_compatibility(
            manifest.libmpv_runtime.as_ref(),
            adjacent_libmpv_runtime_identity()?.as_ref(),
        )?;
        let artifact = base
            .join(&manifest.artifact_url)
            .map_err(|error| format!("resolve manifest artifact URL: {error}"))?;
        return Ok((
            artifact.to_string(),
            Some(manifest.sha256),
            Some(manifest.version),
            Some(manifest.size),
        ));
    }
    let length = probe.content_length();
    Ok((probe.url().to_string(), None, None, length))
}

fn release_asset_matches(name: &str) -> bool {
    let platform = match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        "linux" => "linux",
        value => value,
    };
    let platform_arch = format!("{platform}-{}", std::env::consts::ARCH);
    let lower = name.to_ascii_lowercase();
    lower.contains("pealayer")
        && lower.contains(&platform_arch)
        && (lower.ends_with(".zip") || lower.ends_with(".tar.gz"))
        && !lower.ends_with(".sha256")
}

fn materialize_download(
    operation_id: &str,
    artifact_url: &str,
    downloaded: PathBuf,
) -> Result<(PathBuf, String, u64), String> {
    let lower = url::Url::parse(artifact_url)
        .ok()
        .map(|url| url.path().to_ascii_lowercase())
        .unwrap_or_else(|| artifact_url.to_ascii_lowercase());
    let extracted = if lower.ends_with(".zip") {
        extract_zip_executable(operation_id, &downloaded)
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        extract_tar_gz_executable(operation_id, &downloaded)
    } else {
        return Ok((
            downloaded.clone(),
            sha256_file(&downloaded)?,
            fs::metadata(&downloaded)
                .map_err(|error| error.to_string())?
                .len(),
        ));
    };
    let _ = fs::remove_file(&downloaded);
    let extracted = extracted?;
    let size = fs::metadata(&extracted)
        .map_err(|error| format!("inspect extracted executable: {error}"))?
        .len();
    validate_update_size(size)?;
    let sha256 = sha256_file(&extracted)?;
    Ok((extracted, sha256, size))
}

fn archive_entry_is_executable(path: &Path) -> bool {
    let expected = if cfg!(windows) {
        "pealayer.exe"
    } else {
        "pealayer"
    };
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

fn extract_zip_executable(operation_id: &str, archive_path: &Path) -> Result<PathBuf, String> {
    let archive = File::open(archive_path).map_err(|error| format!("open ZIP: {error}"))?;
    let mut archive =
        zip::ZipArchive::new(archive).map_err(|error| format!("read ZIP: {error}"))?;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("read ZIP entry: {error}"))?;
        let Some(path) = entry.enclosed_name() else {
            continue;
        };
        if !entry.is_file() || !archive_entry_is_executable(&path) {
            continue;
        }
        validate_update_size(entry.size())?;
        let output = update_scratch_path(operation_id, "candidate")?;
        let mut file = create_update_file(&output, true)
            .map_err(|error| format!("create extracted executable: {error}"))?;
        std::io::copy(&mut entry, &mut file)
            .map_err(|error| format!("extract executable from ZIP: {error}"))?;
        file.sync_all().map_err(|error| error.to_string())?;
        make_executable(&output)?;
        return Ok(output);
    }
    Err("update ZIP does not contain the Pealayer executable".to_string())
}

fn extract_tar_gz_executable(operation_id: &str, archive_path: &Path) -> Result<PathBuf, String> {
    let archive = File::open(archive_path).map_err(|error| format!("open TAR.GZ: {error}"))?;
    let decoder = flate2::read::GzDecoder::new(archive);
    let mut archive = tar::Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|error| format!("read TAR.GZ: {error}"))?;
    for entry in entries {
        let mut entry = entry.map_err(|error| format!("read TAR entry: {error}"))?;
        let path = entry
            .path()
            .map_err(|error| format!("read TAR entry path: {error}"))?;
        if !entry.header().entry_type().is_file() || !archive_entry_is_executable(&path) {
            continue;
        }
        let size = entry.size();
        validate_update_size(size)?;
        let output = update_scratch_path(operation_id, "candidate")?;
        let mut file = create_update_file(&output, true)
            .map_err(|error| format!("create extracted executable: {error}"))?;
        std::io::copy(&mut entry, &mut file)
            .map_err(|error| format!("extract executable from TAR.GZ: {error}"))?;
        file.sync_all().map_err(|error| error.to_string())?;
        make_executable(&output)?;
        return Ok(output);
    }
    Err("update TAR.GZ does not contain the Pealayer executable".to_string())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|error| error.to_string())?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).map_err(|error| error.to_string())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn github_release_api_url(source: &str) -> Option<String> {
    let parsed = url::Url::parse(source).ok()?;
    if parsed.host_str()? != "github.com" {
        return None;
    }
    let parts = parsed
        .path_segments()?
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 4 || parts[2] != "releases" {
        return None;
    }
    match parts.get(3).copied()? {
        "latest" => Some(format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            parts[0], parts[1]
        )),
        "tag" if parts.len() >= 5 => Some(format!(
            "https://api.github.com/repos/{}/{}/releases/tags/{}",
            parts[0], parts[1], parts[4]
        )),
        _ => None,
    }
}

pub fn current_manifest() -> Result<UpdateManifest, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let metadata = fs::metadata(&executable).map_err(|error| error.to_string())?;
    Ok(UpdateManifest {
        format: "pealayer-update".to_string(),
        service: "pealayer".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_commit: env!("PEALAYER_GIT_COMMIT").to_string(),
        git_dirty: env!("PEALAYER_GIT_DIRTY") == "true",
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        sha256: sha256_file(&executable)?,
        size: metadata.len(),
        artifact_url: "/api/update/artifact".to_string(),
        libmpv_runtime: adjacent_libmpv_runtime_identity()?,
    })
}

pub fn current_artifact() -> Result<Vec<u8>, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    fs::read(executable).map_err(|error| format!("read current executable: {error}"))
}

pub fn push_current_to_peer(target: &str) -> Result<UpdateStatus, String> {
    let target = normalize_peer_base_url(target)?;
    let manifest = current_manifest()?;
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(60))
        .user_agent(format!(
            "Pealayer/{} peer-updater",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|error| error.to_string())?;
    let begin = BeginUploadRequest {
        sha256: manifest.sha256.clone(),
        size: manifest.size,
        version: Some(manifest.version.clone()),
        platform: Some(manifest.platform.clone()),
        arch: Some(manifest.arch.clone()),
        libmpv_runtime: manifest.libmpv_runtime.clone(),
    };
    let mut status: UpdateStatus = client
        .post(format!("{target}/api/update/begin"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(serde_json::to_vec(&begin).map_err(|error| error.to_string())?)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("begin peer update: {error}"))?
        .json()
        .map_err(|error| format!("decode peer update response: {error}"))?;
    let operation_id = status
        .operation_id
        .clone()
        .ok_or_else(|| "peer omitted update operation id".to_string())?;
    let mut file = File::open(executable).map_err(|error| error.to_string())?;
    let mut offset = 0_u64;
    let mut buffer = vec![0_u8; UPDATE_CHUNK_BYTES];
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        let chunk_result = client
            .post(format!(
                "{target}/api/update/chunk?id={operation_id}&offset={offset}"
            ))
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .body(buffer[..count].to_vec())
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| format!("send peer update chunk: {error}"))
            .and_then(|response| {
                response
                    .json()
                    .map_err(|error| format!("decode peer update progress: {error}"))
            });
        status = match chunk_result {
            Ok(status) => status,
            Err(error) => {
                abort_peer_upload(&client, &target, &operation_id);
                return Err(error);
            }
        };
        offset += count as u64;
        println!("{}", status.message);
    }
    let finish_result = client
        .post(format!("{target}/api/update/finish"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(
            serde_json::to_vec(&FinishUploadRequest {
                operation_id: operation_id.clone(),
            })
            .map_err(|error| error.to_string())?,
        )
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("finish peer update: {error}"))
        .and_then(|response| {
            response
                .json()
                .map_err(|error| format!("decode peer update completion: {error}"))
        });
    status = match finish_result {
        Ok(status) => status,
        Err(error) => {
            abort_peer_upload(&client, &target, &operation_id);
            return Err(error);
        }
    };
    if status.state == "current" {
        return Ok(status);
    }

    let deadline = Instant::now() + Duration::from_secs(90);
    let mut observed_restart = false;
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(350));
        let response = client.get(format!("{target}/api/update/manifest")).send();
        match response.and_then(reqwest::blocking::Response::error_for_status) {
            Ok(response) => {
                if let Ok(peer_manifest) = response.json::<UpdateManifest>()
                    && peer_manifest.sha256 == manifest.sha256
                {
                    status.state = "completed".to_string();
                    status.message = format!(
                        "Peer restarted and acknowledged {}",
                        short_sha(&manifest.sha256)
                    );
                    return Ok(status);
                }
            }
            Err(_) => observed_restart = true,
        }
    }
    Err(if observed_restart {
        "peer stopped for update but did not return with the expected build".to_string()
    } else {
        "peer never restarted with the expected build".to_string()
    })
}

fn abort_peer_upload(client: &reqwest::blocking::Client, target: &str, operation_id: &str) {
    let _ = client
        .post(format!("{target}/api/update/abort"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(
            serde_json::to_vec(&FinishUploadRequest {
                operation_id: operation_id.to_string(),
            })
            .unwrap_or_default(),
        )
        .send();
}

pub fn request_update_from_url(
    target: &str,
    url: &str,
    sha256: Option<&str>,
) -> Result<UpdateStatus, String> {
    let target = normalize_peer_base_url(target)?;
    let request = FetchUpdateRequest {
        url: url.to_string(),
        sha256: sha256.map(str::to_string),
    };
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| error.to_string())?;
    let mut status: UpdateStatus = client
        .post(format!("{target}/api/update/from-url"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(serde_json::to_vec(&request).map_err(|error| error.to_string())?)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("request URL update: {error}"))?
        .json()
        .map_err(|error| format!("decode URL update response: {error}"))?;
    let deadline = Instant::now() + Duration::from_secs(300);
    let mut replacement_sha256 = sha256.map(normalize_sha256).transpose()?;
    let mut observed_restart = false;
    while Instant::now() < deadline {
        if replacement_sha256.is_none() {
            replacement_sha256 = status.sha256.clone();
        }
        match status.state.as_str() {
            "current" | "completed" => return Ok(status),
            "failed" => {
                return Err(status
                    .error
                    .clone()
                    .unwrap_or_else(|| status.message.clone()));
            }
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(300));
        match client
            .get(format!("{target}/api/update/status"))
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .and_then(reqwest::blocking::Response::json::<UpdateStatus>)
        {
            Ok(next) => status = next,
            Err(_) => observed_restart = true,
        }
        if let Some(expected) = replacement_sha256.as_deref()
            && let Ok(response) = client
                .get(format!("{target}/api/update/manifest"))
                .send()
                .and_then(reqwest::blocking::Response::error_for_status)
            && let Ok(manifest) = response.json::<UpdateManifest>()
            && manifest.sha256 == expected
        {
            status.state = "completed".to_string();
            status.sha256 = Some(expected.to_string());
            status.bytes_done = manifest.size;
            status.bytes_total = Some(manifest.size);
            status.version = Some(manifest.version);
            status.message = format!("Updated and acknowledged {}", short_sha(expected));
            status.error = None;
            return Ok(status);
        }
    }
    Err(if observed_restart {
        "Pealayer restarted but did not acknowledge the expected update".to_string()
    } else {
        "Pealayer did not finish the requested update before the timeout".to_string()
    })
}

pub fn peer_update_status(target: &str) -> Result<UpdateStatus, String> {
    let target = normalize_peer_base_url(target)?;
    reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| error.to_string())?
        .get(format!("{target}/api/update/status"))
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("query peer update status: {error}"))?
        .json()
        .map_err(|error| format!("decode peer update status: {error}"))
}

fn normalize_peer_base_url(value: &str) -> Result<String, String> {
    let value = value.trim().trim_end_matches('/');
    let value = if value.contains("://") {
        value.to_string()
    } else {
        format!("http://{value}")
    };
    let parsed = url::Url::parse(&value).map_err(|error| format!("invalid peer URL: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("peer URL must use HTTP or HTTPS".to_string());
    }
    Ok(value)
}

fn prepare_self_update(
    operation_id: &str,
    staged_path: &Path,
    expected_sha256: &str,
) -> Result<(), String> {
    let current_path = std::env::current_exe().map_err(|error| error.to_string())?;
    let current_path = current_path
        .canonicalize()
        .map_err(|error| format!("resolve current executable: {error}"))?;
    let directory = current_path
        .parent()
        .ok_or_else(|| "current executable has no parent directory".to_string())?;
    let staged_path = staged_path
        .canonicalize()
        .map_err(|error| format!("resolve staged executable: {error}"))?;
    let suffix = &expected_sha256[..12];
    let helper_extension = if cfg!(windows) {
        "helper.exe"
    } else {
        "helper"
    };
    let helper_path = directory.join(format!(".pealayer-update-{suffix}.{helper_extension}"));
    let journal_path = directory.join(format!(".pealayer-update-{suffix}.journal.json"));
    let backup_path = directory.join(format!(".pealayer-update-{suffix}.old"));
    let health_path = directory.join(format!(".pealayer-update-{suffix}.healthy"));
    for stale in [&helper_path, &journal_path, &backup_path, &health_path] {
        let _ = fs::remove_file(stale);
    }
    let mut original =
        File::open(&current_path).map_err(|error| format!("read update helper: {error}"))?;
    let mut helper = create_update_file(&helper_path, true)
        .map_err(|error| format!("create update helper: {error}"))?;
    std::io::copy(&mut original, &mut helper)
        .map_err(|error| format!("stage update helper: {error}"))?;
    helper.sync_all().map_err(|error| error.to_string())?;
    drop(helper);
    make_executable(&helper_path)?;
    let journal = UpdateJournal {
        format: "pealayer-self-update".to_string(),
        operation_id: operation_id.to_string(),
        parent_pid: std::process::id(),
        current_path: current_path.clone(),
        staged_path,
        backup_path,
        helper_path: helper_path.clone(),
        journal_path: journal_path.clone(),
        health_path,
        health_token: format!("{}-{}", uuid::Uuid::new_v4(), now_millis()),
        current_sha256: sha256_file(&current_path)?,
        replacement_sha256: expected_sha256.to_string(),
        arguments: std::env::args().skip(1).collect(),
        working_directory: std::env::current_dir().map_err(|error| error.to_string())?,
    };
    write_json_atomic(&journal_path, &journal)?;
    spawn_update_helper(&helper_path, &journal_path)?;
    Ok(())
}

fn spawn_update_helper(helper: &Path, journal: &Path) -> Result<(), String> {
    let mut command = Command::new(helper);
    command.arg(HELPER_ARGUMENT).arg(journal);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .spawn()
        .map_err(|error| format!("launch update helper: {error}"))?;
    Ok(())
}

pub fn helper_invocation(args: &[String]) -> Option<&str> {
    if args.len() == 3 && args[1] == HELPER_ARGUMENT {
        Some(args[2].as_str())
    } else {
        None
    }
}

pub fn run_update_helper(journal_path: &str) -> Result<(), String> {
    let content =
        fs::read(journal_path).map_err(|error| format!("read update journal: {error}"))?;
    let journal: UpdateJournal = serde_json::from_slice(&content)
        .map_err(|error| format!("decode update journal: {error}"))?;
    validate_journal(&journal, Path::new(journal_path))?;
    wait_for_parent_exit(journal.parent_pid, Duration::from_secs(120))?;
    verify_file_sha256(&journal.current_path, &journal.current_sha256)?;
    verify_file_sha256(&journal.staged_path, &journal.replacement_sha256)?;
    activate_update_files(&journal)?;
    if let Err(error) = verify_file_sha256(&journal.current_path, &journal.replacement_sha256) {
        rollback_update(&journal)?;
        return Err(error);
    }
    let mut child = launch_candidate(&journal, false)?;
    match wait_for_health(&journal, &mut child, Duration::from_secs(35)) {
        Ok(()) => {
            let _ = fs::remove_file(&journal.backup_path);
            let _ = fs::remove_file(&journal.health_path);
            let _ = fs::remove_file(&journal.journal_path);
            Ok(())
        }
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            rollback_update(&journal)?;
            let _ = launch_candidate(&journal, true)?;
            Err(format!(
                "candidate failed health check; rolled back: {error}"
            ))
        }
    }
}

fn activate_update_files(journal: &UpdateJournal) -> Result<(), String> {
    let _ = fs::remove_file(&journal.backup_path);
    fs::rename(&journal.current_path, &journal.backup_path)
        .map_err(|error| format!("preserve previous executable: {error}"))?;
    if let Err(error) = set_update_file_hidden(&journal.backup_path, true) {
        rollback_update(&journal)?;
        return Err(error);
    }
    if let Err(error) = fs::rename(&journal.staged_path, &journal.current_path) {
        rollback_update(&journal)?;
        return Err(format!("activate staged executable: {error}"));
    }
    // Rename preserves Windows attributes: staging is hidden, installed apps are not.
    if let Err(error) = set_update_file_hidden(&journal.current_path, false) {
        rollback_update(&journal)?;
        return Err(error);
    }
    Ok(())
}

fn launch_candidate(journal: &UpdateJournal, rollback: bool) -> Result<Child, String> {
    let mut command = Command::new(&journal.current_path);
    command
        .args(&journal.arguments)
        .current_dir(&journal.working_directory);
    if !rollback {
        command
            .env(HEALTH_PATH_ENV, &journal.health_path)
            .env(HEALTH_TOKEN_ENV, &journal.health_token)
            .env(HELPER_PATH_ENV, &journal.helper_path)
            .env(JOURNAL_PATH_ENV, &journal.journal_path);
    } else {
        command
            .env_remove(HEALTH_PATH_ENV)
            .env_remove(HEALTH_TOKEN_ENV)
            .env_remove(HELPER_PATH_ENV)
            .env_remove(JOURNAL_PATH_ENV);
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .spawn()
        .map_err(|error| format!("launch replacement: {error}"))
}

fn wait_for_health(
    journal: &UpdateJournal,
    child: &mut Child,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return Err(format!("replacement exited early with {status}"));
        }
        if let Ok(content) = fs::read_to_string(&journal.health_path)
            && content.trim() == journal.health_token
        {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("replacement startup health acknowledgement timed out".to_string())
}

fn rollback_update(journal: &UpdateJournal) -> Result<(), String> {
    verify_file_sha256(&journal.backup_path, &journal.current_sha256)?;
    let _ = fs::remove_file(&journal.current_path);
    fs::rename(&journal.backup_path, &journal.current_path)
        .map_err(|error| format!("restore previous executable: {error}"))?;
    set_update_file_hidden(&journal.current_path, false)?;
    verify_file_sha256(&journal.current_path, &journal.current_sha256)
}

pub fn schedule_startup_health_acknowledgement() {
    cleanup_orphaned_downloads();
    let Ok(path) = std::env::var(HEALTH_PATH_ENV) else {
        return;
    };
    let Ok(token) = std::env::var(HEALTH_TOKEN_ENV) else {
        return;
    };
    let helper = std::env::var(HELPER_PATH_ENV).ok();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(900));
        let acknowledgement = create_update_file(Path::new(&path), true).and_then(|mut file| {
            file.write_all(token.as_bytes())?;
            file.sync_all()
        });
        if let Err(error) = acknowledgement {
            eprintln!("write update health acknowledgement: {error}");
            return;
        }
        if let Some(helper) = helper {
            std::thread::sleep(Duration::from_secs(3));
            let _ = fs::remove_file(helper);
        }
    });
}

pub fn cleanup_orphaned_downloads() {
    let Some(directory) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    else {
        return;
    };
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let active_prefix = std::env::var(HELPER_PATH_ENV)
        .ok()
        .and_then(|path| PathBuf::from(path).file_name().map(|name| name.to_owned()))
        .and_then(|name| {
            let name = name.to_string_lossy();
            name.strip_suffix("helper.exe")
                .or_else(|| name.strip_suffix("helper"))
                .map(str::to_string)
        });
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let orphaned_download = name.starts_with(".update-") && name.ends_with(".download");
        let orphaned_transaction = name.starts_with(".pealayer-update-")
            && active_prefix
                .as_deref()
                .is_none_or(|prefix| !name.starts_with(prefix));
        if orphaned_download || orphaned_transaction {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn validate_journal(journal: &UpdateJournal, supplied_path: &Path) -> Result<(), String> {
    if journal.format != "pealayer-self-update" || journal.parent_pid == 0 {
        return Err("invalid self-update journal".to_string());
    }
    let supplied = supplied_path
        .canonicalize()
        .map_err(|error| format!("resolve update journal: {error}"))?;
    let recorded = journal
        .journal_path
        .canonicalize()
        .map_err(|error| format!("resolve recorded journal: {error}"))?;
    if supplied != recorded {
        return Err("self-update journal path identity mismatch".to_string());
    }
    let root = journal
        .current_path
        .parent()
        .ok_or_else(|| "self-update target has no parent".to_string())?;
    for path in [
        &journal.staged_path,
        &journal.backup_path,
        &journal.helper_path,
        &journal.journal_path,
        &journal.health_path,
    ] {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| "self-update journal path escapes the executable directory".to_string())?;
        if relative
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err("self-update journal path escapes the executable directory".to_string());
        }
    }
    normalize_sha256(&journal.current_sha256)?;
    normalize_sha256(&journal.replacement_sha256)?;
    Ok(())
}

fn wait_for_parent_exit(pid: u32, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !process_is_running(pid) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("Pealayer did not finish graceful shutdown before update timeout".to_string())
}

#[cfg(target_os = "windows")]
fn process_is_running(pid: u32) -> bool {
    use windows::Win32::Foundation::{CloseHandle, WAIT_TIMEOUT};
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
    };
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) else {
            return false;
        };
        let result = WaitForSingleObject(handle, 0) == WAIT_TIMEOUT;
        let _ = CloseHandle(handle);
        result
    }
}

#[cfg(not(target_os = "windows"))]
fn process_is_running(pid: u32) -> bool {
    if cfg!(target_os = "linux") {
        return Path::new(&format!("/proc/{pid}")).exists();
    }
    Command::new("ps")
        .args(["-p", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
}

fn update_scratch_path(operation_id: &str, suffix: &str) -> Result<PathBuf, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let directory = executable
        .parent()
        .ok_or_else(|| "current executable has no parent directory".to_string())?;
    Ok(directory.join(format!(".{operation_id}.{suffix}")))
}

fn validate_update_size(size: u64) -> Result<(), String> {
    if size == 0 || size > MAX_UPDATE_BYTES {
        return Err(format!(
            "update size must be between 1 byte and {}",
            format_bytes(MAX_UPDATE_BYTES)
        ));
    }
    Ok(())
}

fn normalize_sha256(value: &str) -> Result<String, String> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SHA-256 must contain exactly 64 hexadecimal characters".to_string());
    }
    Ok(value)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| format!("open {}: {error}", path.display()))?;
    let mut hash = Sha256::new();
    std::io::copy(&mut file, &mut hash).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", hash.finalize()))
}

fn verify_file_sha256(path: &Path, expected: &str) -> Result<(), String> {
    let actual = sha256_file(path)?;
    if actual != expected {
        return Err(format!(
            "SHA-256 mismatch for {}: expected {expected}, received {actual}",
            path.display()
        ));
    }
    Ok(())
}

fn inspect_executable(path: &Path) -> Result<(), String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut header = [0_u8; 4096];
    let count = file.read(&mut header).map_err(|error| error.to_string())?;
    #[cfg(target_os = "windows")]
    {
        if count < 64 || &header[..2] != b"MZ" {
            return Err("update is not a Windows PE executable".to_string());
        }
        let pe_offset = u32::from_le_bytes(header[0x3c..0x40].try_into().unwrap()) as usize;
        if pe_offset + 6 > count {
            file.seek(SeekFrom::Start(pe_offset as u64))
                .map_err(|error| error.to_string())?;
            let mut pe = [0_u8; 6];
            file.read_exact(&mut pe)
                .map_err(|error| error.to_string())?;
            return validate_pe_machine(&pe);
        }
        validate_pe_machine(&header[pe_offset..pe_offset + 6])?;
    }
    #[cfg(target_os = "linux")]
    {
        if count < 20 || &header[..4] != b"\x7fELF" {
            return Err("update is not an ELF executable".to_string());
        }
        let machine = match header[5] {
            1 => u16::from_le_bytes([header[18], header[19]]),
            2 => u16::from_be_bytes([header[18], header[19]]),
            _ => return Err("update has an invalid ELF byte order".to_string()),
        };
        let expected = match std::env::consts::ARCH {
            "x86_64" => 62,
            "x86" => 3,
            "aarch64" => 183,
            _ => machine,
        };
        if machine != expected {
            return Err(format!(
                "update ELF machine {machine} does not match this host"
            ));
        }
    }
    #[cfg(target_os = "macos")]
    {
        if count < 8 {
            return Err("update is not a Mach-O executable".to_string());
        }
        let magic = &header[..4];
        let universal = matches!(
            magic,
            b"\xca\xfe\xba\xbe" | b"\xbe\xba\xfe\xca" | b"\xca\xfe\xba\xbf" | b"\xbf\xba\xfe\xca"
        );
        if !universal {
            let little_endian = matches!(magic, b"\xce\xfa\xed\xfe" | b"\xcf\xfa\xed\xfe");
            let big_endian = matches!(magic, b"\xfe\xed\xfa\xce" | b"\xfe\xed\xfa\xcf");
            if !little_endian && !big_endian {
                return Err("update is not a Mach-O executable".to_string());
            }
            let cpu_type = if little_endian {
                u32::from_le_bytes(header[4..8].try_into().unwrap())
            } else {
                u32::from_be_bytes(header[4..8].try_into().unwrap())
            };
            let expected = match std::env::consts::ARCH {
                "x86_64" => 0x0100_0007,
                "aarch64" => 0x0100_000c,
                _ => cpu_type,
            };
            if cpu_type != expected {
                return Err(format!(
                    "update Mach-O CPU type 0x{cpu_type:08x} does not match this host"
                ));
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn validate_pe_machine(header: &[u8]) -> Result<(), String> {
    if header.len() < 6 || &header[..4] != b"PE\0\0" {
        return Err("update has an invalid PE header".to_string());
    }
    let machine = u16::from_le_bytes([header[4], header[5]]);
    let expected = match std::env::consts::ARCH {
        "x86_64" => 0x8664,
        "x86" => 0x014c,
        "aarch64" => 0xaa64,
        _ => machine,
    };
    if machine != expected {
        return Err(format!(
            "update PE machine 0x{machine:04x} does not match this host"
        ));
    }
    Ok(())
}

/// All updater-owned scratch files are hidden at creation, not just dot-prefixed.
/// Existing file attributes are changed only when activating/restoring an app.
fn create_update_file(path: &Path, exclusive: bool) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true);
    if exclusive {
        options.create_new(true);
    } else {
        options.create(true).truncate(true);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN;
        options.attributes(FILE_ATTRIBUTE_HIDDEN.0);
    }
    let file = options.open(path)?;
    // Creation flags do not change an already existing journal temporary file.
    set_update_file_hidden(path, true).map_err(std::io::Error::other)?;
    Ok(file)
}

#[cfg(windows)]
fn set_update_file_hidden(path: &Path, hidden: bool) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NORMAL, FILE_FLAGS_AND_ATTRIBUTES,
        GetFileAttributesW, SetFileAttributesW,
    };
    use windows::core::PCWSTR;
    let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: the path buffer is null-terminated and lives throughout both calls.
    let current = unsafe { GetFileAttributesW(PCWSTR(name.as_ptr())) };
    if current == u32::MAX {
        return Err(format!(
            "read update file attributes: {}",
            std::io::Error::last_os_error()
        ));
    }
    let mut attributes = current & !FILE_ATTRIBUTE_NORMAL.0;
    if hidden {
        attributes |= FILE_ATTRIBUTE_HIDDEN.0;
    } else {
        attributes &= !FILE_ATTRIBUTE_HIDDEN.0;
    }
    if attributes == 0 {
        attributes = FILE_ATTRIBUTE_NORMAL.0;
    }
    unsafe { SetFileAttributesW(PCWSTR(name.as_ptr()), FILE_FLAGS_AND_ATTRIBUTES(attributes)) }
        .map_err(|error| format!("set update file visibility: {error}"))
}

#[cfg(not(windows))]
fn set_update_file_hidden(_path: &Path, _hidden: bool) -> Result<(), String> {
    // Unix scratch paths already use leading dots. No installed filename changes.
    Ok(())
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let temporary = path.with_extension("tmp");
    let content = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let mut file = create_update_file(&temporary, false).map_err(|error| error.to_string())?;
    file.write_all(&content)
        .map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    drop(file);
    fs::rename(&temporary, path).map_err(|error| error.to_string())
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn short_sha(value: &str) -> &str {
    value.get(..12).unwrap_or(value)
}

fn adjacent_libmpv_runtime_identity() -> Result<Option<LibmpvRuntimeIdentity>, String> {
    if !cfg!(windows) {
        return Ok(None);
    }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let directory = executable
        .parent()
        .ok_or_else(|| "current executable has no parent directory".to_string())?;
    for file_name in ["libmpv-2.dll", "mpv-2.dll"] {
        let path = directory.join(file_name);
        if path.is_file() {
            return Ok(Some(LibmpvRuntimeIdentity {
                file_name: file_name.to_string(),
                sha256: sha256_file(&path)?,
            }));
        }
    }
    Ok(None)
}

fn validate_libmpv_compatibility(
    incoming: Option<&LibmpvRuntimeIdentity>,
    installed: Option<&LibmpvRuntimeIdentity>,
) -> Result<(), String> {
    let (Some(incoming), Some(installed)) = (incoming, installed) else {
        return Ok(());
    };
    if incoming.sha256.eq_ignore_ascii_case(&installed.sha256) {
        return Ok(());
    }
    Err(format!(
        "update libmpv runtime {} ({}) does not match this host's {} ({}); use a build produced for this host profile",
        incoming.file_name,
        short_sha(&incoming.sha256),
        installed.file_name,
        short_sha(&installed.sha256)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abort_closes_staging_file_and_cleanup_failure_does_not_lock_future_updates() {
        for missing_file in [false, true] {
            let manager = UpdateManager::default();
            let path =
                std::env::temp_dir().join(format!(".pealayer-abort-{}", uuid::Uuid::new_v4()));
            let file = create_update_file(&path, true).unwrap();
            manager.inner.lock().unwrap().upload = Some(ActiveUpload {
                operation_id: "abort-test".into(),
                path: path.clone(),
                file,
                hasher: Sha256::new(),
                expected_sha256: "0".repeat(64),
                expected_size: 1,
                received: 0,
                version: None,
            });
            manager.replace_status(UpdateStatus {
                state: "receiving".into(),
                ..Default::default()
            });
            if missing_file {
                fs::remove_file(&path).unwrap();
            }
            let result = manager.abort_upload("abort-test");
            assert_eq!(result.is_err(), missing_file);
            assert_eq!(
                manager.status().state,
                if missing_file { "failed" } else { "aborted" }
            );
            assert!(!manager.status().active());
            assert!(manager.inner.lock().unwrap().upload.is_none());
            assert!(!path.exists());
        }
    }

    #[cfg(windows)]
    #[test]
    fn update_files_are_hidden_until_activation_or_rollback() {
        use std::os::windows::fs::MetadataExt;
        use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_HIDDEN};
        let directory = std::env::temp_dir().join(format!(
            "pealayer-update-visibility-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&directory).unwrap();
        let current_path = directory.join("pealayer.exe");
        let staged_path = directory.join(".update.download");
        fs::write(&current_path, b"original").unwrap();
        let mut staged = create_update_file(&staged_path, true).unwrap();
        staged.write_all(b"replacement").unwrap();
        drop(staged);
        let attributes = |path: &Path| fs::metadata(path).unwrap().file_attributes();
        assert_ne!(attributes(&staged_path) & FILE_ATTRIBUTE_HIDDEN.0, 0);
        let other_attributes = attributes(&staged_path) & !FILE_ATTRIBUTE_HIDDEN.0;
        let journal = UpdateJournal {
            format: "pealayer-self-update".into(),
            operation_id: "visibility-test".into(),
            parent_pid: std::process::id(),
            current_path: current_path.clone(),
            staged_path,
            backup_path: directory.join(".update.old"),
            helper_path: directory.join(".update.helper.exe"),
            journal_path: directory.join(".update.journal.json"),
            health_path: directory.join(".update.healthy"),
            health_token: "test".into(),
            current_sha256: sha256_file(&current_path).unwrap(),
            replacement_sha256: format!("{:x}", Sha256::digest(b"replacement")),
            arguments: vec![],
            working_directory: directory.clone(),
        };
        write_json_atomic(&journal.journal_path, &journal).unwrap();
        assert_ne!(
            attributes(&journal.journal_path) & FILE_ATTRIBUTE_HIDDEN.0,
            0
        );
        assert!(!journal.journal_path.with_extension("tmp").exists());
        activate_update_files(&journal).unwrap();
        assert_eq!(attributes(&current_path) & FILE_ATTRIBUTE_HIDDEN.0, 0);
        assert_eq!(
            attributes(&current_path) & !FILE_ATTRIBUTE_HIDDEN.0,
            other_attributes
        );
        assert_ne!(
            attributes(&journal.backup_path) & FILE_ATTRIBUTE_HIDDEN.0,
            0
        );
        assert_ne!(
            attributes(&journal.backup_path) & FILE_ATTRIBUTE_ARCHIVE.0,
            0
        );
        assert_eq!(fs::read(&current_path).unwrap(), b"replacement");
        rollback_update(&journal).unwrap();
        assert_eq!(attributes(&current_path) & FILE_ATTRIBUTE_HIDDEN.0, 0);
        assert_eq!(fs::read(&current_path).unwrap(), b"original");
        // Windows allows deleting Hidden files without making them visible first.
        fs::remove_file(&journal.journal_path).unwrap();
        fs::remove_file(&current_path).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn idle_gui_is_woken_for_status_changes_and_queued_quit() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let manager = UpdateManager::default();
        let context = eframe::egui::Context::default();
        let wakeups = Arc::new(AtomicUsize::new(0));
        let observed = wakeups.clone();
        context.set_request_repaint_callback(move |_| {
            observed.fetch_add(1, Ordering::Relaxed);
        });
        manager.register_gui_context(context.clone());
        manager.replace_status(UpdateStatus {
            state: "receiving".into(),
            ..Default::default()
        });
        assert!(wakeups.load(Ordering::Relaxed) > 0);
        // eframe consumes the staging repaint long before the delayed Quit.
        // Requests within the same pending frame are legitimately coalesced.
        for _ in 0..4 {
            let mut output = context.run_ui(Default::default(), |_| {});
            output.textures_delta.clear();
        }
        let previous = wakeups.load(Ordering::Relaxed);
        let (sender, receiver) = std::sync::mpsc::channel();
        manager.dispatch_quit(&sender);
        assert!(matches!(
            receiver.try_recv(),
            Ok(crate::platform::interop::InteropCommand::Quit)
        ));
        assert!(
            wakeups.load(Ordering::Relaxed) > previous,
            "Quit must schedule its own redraw after enqueue"
        );
    }

    #[test]
    fn missing_dispatcher_reports_failure_instead_of_staying_restarting() {
        let manager = UpdateManager::default();
        let (sender, receiver) = std::sync::mpsc::channel();
        drop(receiver);
        manager.dispatch_quit(&sender);
        assert_eq!(manager.status().state, "failed");
        assert_eq!(
            manager.status().error.as_deref(),
            Some("application dispatcher is unavailable")
        );
    }

    #[test]
    fn byte_format_is_human_readable() {
        assert_eq!(format_bytes(6419456), "6.1 MiB");
        assert_eq!(format_bytes(512), "512 B");
    }

    #[test]
    fn github_release_urls_resolve_to_api() {
        assert_eq!(
            github_release_api_url("https://github.com/ToghrolTP/pealayer/releases/latest")
                .as_deref(),
            Some("https://api.github.com/repos/ToghrolTP/pealayer/releases/latest")
        );
        assert_eq!(
            github_release_api_url("https://github.com/ToghrolTP/pealayer/releases/tag/v0.2.0")
                .as_deref(),
            Some("https://api.github.com/repos/ToghrolTP/pealayer/releases/tags/v0.2.0")
        );
    }

    #[test]
    fn release_asset_selection_requires_this_platform_and_architecture() {
        let matching = format!(
            "Pealayer-{}-{}-main-deadbee.{}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            if cfg!(target_os = "linux") {
                "tar.gz"
            } else {
                "zip"
            }
        );
        assert!(release_asset_matches(&matching));
        assert!(!release_asset_matches(
            "Pealayer-windows-aarch64-main-deadbee.zip"
        ));
    }

    #[test]
    fn zip_release_extracts_only_the_platform_executable() {
        let operation = format!("update-test-{}", uuid::Uuid::new_v4());
        let archive_path = std::env::temp_dir().join(format!("{operation}.zip"));
        let archive = File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(archive);
        let expected_name = if cfg!(windows) {
            "bundle/pealayer.exe"
        } else {
            "bundle/pealayer"
        };
        writer
            .start_file(expected_name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"verified candidate").unwrap();
        writer.finish().unwrap();
        let extracted = extract_zip_executable(&operation, &archive_path).unwrap();
        assert_eq!(fs::read(&extracted).unwrap(), b"verified candidate");
        let _ = fs::remove_file(extracted);
        let _ = fs::remove_file(archive_path);
    }

    #[test]
    fn current_executable_matches_host_platform() {
        inspect_executable(&std::env::current_exe().unwrap()).unwrap();
    }

    #[test]
    fn peer_updates_reject_a_different_libmpv_runtime_profile() {
        let incoming = LibmpvRuntimeIdentity {
            file_name: "libmpv-2.dll".to_string(),
            sha256: "a".repeat(64),
        };
        let installed = LibmpvRuntimeIdentity {
            file_name: "libmpv-2.dll".to_string(),
            sha256: "b".repeat(64),
        };
        let error = validate_libmpv_compatibility(Some(&incoming), Some(&installed)).unwrap_err();
        assert!(error.contains("use a build produced for this host profile"));
        assert!(validate_libmpv_compatibility(Some(&installed), Some(&installed)).is_ok());
    }

    #[test]
    fn normal_cli_invocation_is_not_indexed_as_an_update_helper() {
        assert_eq!(helper_invocation(&["pealayer".to_string()]), None);
        assert_eq!(
            helper_invocation(&["pealayer".to_string(), "--smoke-test".to_string()]),
            None
        );
        let helper = [
            "pealayer".to_string(),
            HELPER_ARGUMENT.to_string(),
            "journal.json".to_string(),
        ];
        assert_eq!(helper_invocation(&helper), Some("journal.json"));
    }
}
