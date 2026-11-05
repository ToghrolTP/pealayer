//! Bounded ordered uploads. Completed assets are durable authoring inputs, not
//! thumbnail cache entries. No caller-supplied path can choose the write target.
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{Read, Write},
    path::PathBuf,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;
pub const CHUNK_BYTES: usize = 512 * 1024;
const MAX_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_ACTIVE: usize = 8;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Begin {
    pub batch_id: Uuid,
    pub name: String,
    pub size: u64,
}
struct Upload {
    batch: Uuid,
    path: PathBuf,
    final_path: PathBuf,
    size: u64,
    offset: u64,
    touched: Instant,
}
#[derive(Default)]
struct Uploads {
    active: HashMap<Uuid, Upload>,
    completed: HashMap<Uuid, Vec<PathBuf>>,
}
static UPLOADS: LazyLock<Mutex<Uploads>> = LazyLock::new(|| Mutex::new(Uploads::default()));
fn root() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("PEALAYER_CONFIG_FILE") {
        return Ok(PathBuf::from(path)
            .parent()
            .ok_or("Invalid profile path")?
            .join("assets"));
    }
    #[cfg(windows)]
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return Ok(PathBuf::from(path).join("Programs/Pealayer/assets"));
    }
    Ok(crate::config::AppConfig::get_config_path()
        .parent()
        .ok_or("Asset directory is unavailable")?
        .join("assets"))
}
fn valid_begin(request: &Begin) -> Result<(), String> {
    let stem = request
        .name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if request.size == 0
        || request.size > MAX_BYTES
        || request.name.is_empty()
        || request.name.len() > 240
        || request.name == "."
        || request.name == ".."
        || request.name.starts_with('.')
        || request.name.ends_with(['.', ' '])
        || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit())
        || request
            .name
            .chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
    {
        return Err("Choose a regular named asset of 1 byte to 8 GiB".into());
    }
    Ok(())
}
pub fn begin(request: Begin) -> Result<serde_json::Value, String> {
    begin_in(root()?, request)
}
fn begin_in(root: PathBuf, request: Begin) -> Result<serde_json::Value, String> {
    valid_begin(&request)?;
    let mut uploads = UPLOADS
        .lock()
        .map_err(|_| "Asset upload state is unavailable")?;
    uploads.active.retain(|_, upload| {
        if upload.touched.elapsed() > Duration::from_secs(1800) {
            let _ = std::fs::remove_file(&upload.path);
            false
        } else {
            true
        }
    });
    if uploads.active.len() >= MAX_ACTIVE {
        return Err("Too many asset transfers; finish or cancel another upload".into());
    }
    if uploads.completed.len() >= 512
        || uploads.completed.get(&request.batch_id).map_or(0, Vec::len)
            + uploads
                .active
                .values()
                .filter(|u| u.batch == request.batch_id)
                .count()
            >= crate::assets::MAX_FILES
    {
        return Err("Finish or cancel pending asset batches before uploading more files".into());
    }
    let directory = root.join(request.batch_id.to_string());
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let final_path = directory.join(&request.name);
    if final_path.exists() || uploads.active.values().any(|u| u.final_path == final_path) {
        return Err("Duplicate asset filename in this batch".into());
    }
    let id = Uuid::new_v4();
    let path = directory.join(format!(".{id}.part"));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    uploads.active.insert(
        id,
        Upload {
            batch: request.batch_id,
            path,
            final_path,
            size: request.size,
            offset: 0,
            touched: Instant::now(),
        },
    );
    Ok(serde_json::json!({"id":id,"chunk_bytes":CHUNK_BYTES,"owner_id":crate::peer::instance_id()}))
}
pub fn chunk(id: Uuid, offset: u64, bytes: &[u8]) -> Result<serde_json::Value, String> {
    let mut uploads = UPLOADS
        .lock()
        .map_err(|_| "Asset upload state is unavailable")?;
    let upload = uploads
        .active
        .get_mut(&id)
        .ok_or("Asset upload expired or does not exist")?;
    if offset != upload.offset
        || bytes.is_empty()
        || bytes.len() > CHUNK_BYTES
        || upload.offset + bytes.len() as u64 > upload.size
    {
        return Err("Asset chunk has invalid offset or size".into());
    }
    std::fs::OpenOptions::new()
        .append(true)
        .open(&upload.path)
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|e| e.to_string())?;
    upload.offset += bytes.len() as u64;
    upload.touched = Instant::now();
    Ok(serde_json::json!({"offset":upload.offset}))
}
pub fn finish(id: Uuid) -> Result<serde_json::Value, String> {
    let mut uploads = UPLOADS
        .lock()
        .map_err(|_| "Asset upload state is unavailable")?;
    let upload = uploads
        .active
        .get(&id)
        .ok_or("Asset upload expired or does not exist")?;
    if upload.offset != upload.size {
        return Err("Asset upload is incomplete".into());
    }
    // The destination was reserved at Begin, and no overwrite is permitted.
    if upload.final_path.exists() {
        return Err("Asset destination already exists".into());
    }
    std::fs::OpenOptions::new()
        .write(true)
        .open(&upload.path)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    std::fs::rename(&upload.path, &upload.final_path).map_err(|e| e.to_string())?;
    let path = upload.final_path.clone();
    let batch = upload.batch;
    uploads.active.remove(&id);
    uploads
        .completed
        .entry(batch)
        .or_default()
        .push(path.clone());
    Ok(serde_json::json!({"path":path,"owner_id":crate::peer::instance_id()}))
}
/// Delete only this transfer's explicitly registered files, never a directory
/// supplied by the caller. Successful imports release these receipts first.
pub fn abort_batch(batch: Uuid) -> Result<serde_json::Value, String> {
    let mut uploads = UPLOADS
        .lock()
        .map_err(|_| "Asset upload state is unavailable")?;
    let ids = uploads
        .active
        .iter()
        .filter(|(_, u)| u.batch == batch)
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    let mut paths = uploads.completed.remove(&batch).unwrap_or_default();
    for id in ids {
        if let Some(upload) = uploads.active.remove(&id) {
            paths.push(upload.path);
        }
    }
    for path in paths {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(serde_json::json!({"cancelled":true}))
}
pub fn retain_paths(paths: &[String]) {
    if let Ok(mut uploads) = UPLOADS.lock() {
        uploads.completed.retain(|_, files| {
            files.retain(|file| !paths.iter().any(|p| std::path::Path::new(p) == file));
            !files.is_empty()
        });
    }
}
pub fn cancel(id: Uuid) -> Result<serde_json::Value, String> {
    if let Some(upload) = UPLOADS
        .lock()
        .map_err(|_| "Asset upload state is unavailable")?
        .active
        .remove(&id)
    {
        std::fs::remove_file(upload.path).map_err(|e| e.to_string())?;
    }
    Ok(serde_json::json!({"cancelled":true}))
}
fn response(value: reqwest::blocking::Response) -> Result<serde_json::Value, String> {
    let status = value.status();
    let value: serde_json::Value = value.json().map_err(|e| e.to_string())?;
    if status.is_success() {
        Ok(value)
    } else {
        Err(value["error"]
            .as_str()
            .unwrap_or("Asset upload failed")
            .into())
    }
}
pub fn forward(
    client: &crate::peer::Client,
    request: &crate::assets::ImportRequest,
) -> Result<Option<String>, String> {
    let batch = Uuid::new_v4();
    let owner = client
        .snapshot()
        .ok_or("Remote session is unavailable")?
        .session
        .instance_id;
    let mut paths = Vec::new();
    if matches!(
        request.destination,
        crate::assets::Destination::AudioSource { .. }
    ) && (request.paths.len() != 1
        || crate::assets::file_kind(std::path::Path::new(&request.paths[0]))
            != crate::assets::FileKind::Audio)
    {
        return Err("Choose one audio file for the sound effect source".into());
    }
    let transfer = (|| {
        for path in &request.paths {
            let path = std::path::Path::new(path);
            let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let begin = Begin {
                batch_id: batch,
                name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or("Asset filename must be valid Unicode")?
                    .into(),
                size: file.metadata().map_err(|e| e.to_string())?.len(),
            };
            valid_begin(&begin)?;
            let value = client.post(
                "/api/assets/upload/begin",
                &serde_json::to_value(begin).map_err(|e| e.to_string())?,
            )?;
            if value["owner_id"].as_str() != Some(owner.as_str()) {
                return Err("Asset session owner changed".into());
            }
            if value["chunk_bytes"].as_u64() != Some(CHUNK_BYTES as u64) {
                return Err("Invalid asset transfer chunk contract".into());
            }
            let id = value["id"]
                .as_str()
                .ok_or("Missing asset transfer ID")?
                .to_string();
            let transferred = (|| {
                let mut offset = 0;
                let mut buffer = vec![0; CHUNK_BYTES];
                loop {
                    let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
                    if n == 0 {
                        break;
                    }
                    let receipt=response(
                    client
                        .request(
                            reqwest::Method::PUT,
                            &format!("/api/assets/upload/chunk?id={id}&offset={offset}&owner_id={owner}"),
                        )?
                        .timeout(Duration::from_secs(30))
                        .body(buffer[..n].to_vec())
                        .send()
                        .map_err(|e| e.to_string())?,
                )?;
                    if receipt["offset"].as_u64() != Some(offset + n as u64) {
                        return Err("Asset transfer offset mismatch".into());
                    }
                    offset += n as u64;
                }
                client.post(
                    &format!("/api/assets/upload/finish?id={id}&owner_id={owner}"),
                    &serde_json::json!({}),
                )
            })();
            match transferred {
                Ok(value) => {
                    if value["owner_id"].as_str() != Some(owner.as_str()) {
                        return Err("Asset session owner changed".into());
                    }
                    paths.push(
                        value["path"]
                            .as_str()
                            .ok_or("Missing uploaded asset path")?
                            .to_string(),
                    );
                }
                Err(error) => {
                    let _ = client
                        .request(
                            reqwest::Method::DELETE,
                            &format!("/api/assets/upload?id={id}&owner_id={owner}"),
                        )
                        .and_then(|r| r.send().map_err(|e| e.to_string()));
                    return Err(error);
                }
            }
        }
        if matches!(
            request.destination,
            crate::assets::Destination::AudioSource { .. }
        ) {
            client.post(
                &format!("/api/assets/upload/retain?owner_id={owner}"),
                &serde_json::json!({"paths":paths}),
            )?;
            return Ok(paths.into_iter().next());
        }
        // A command may be queued even if its reply is lost. Protect its inputs
        // before submission; failed acknowledgement must never delete live assets.
        client.post(
            &format!("/api/assets/upload/retain?owner_id={owner}"),
            &serde_json::json!({"paths":paths}),
        )?;
        client.post("/api/player/command",&serde_json::json!({"command":"import_assets","request":{"paths":paths,"destination":request.destination,"owner_id":owner}}))?;
        Ok(None)
    })();
    if transfer.is_err() {
        let _ = client
            .request(
                reqwest::Method::DELETE,
                &format!("/api/assets/upload/batch?batch_id={batch}&owner_id={owner}"),
            )
            .and_then(|r| r.send().map_err(|e| e.to_string()));
    }
    transfer
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordered_upload_rejects_traversal_overrun_and_incomplete_finish() {
        let root = std::env::temp_dir().join(format!("pealayer-assets-{}", Uuid::new_v4()));
        let batch = Uuid::new_v4();
        for name in [
            "../bad.wav",
            "C:\\bad.wav",
            ".hidden",
            "bad.",
            "CON.wav",
            "NUL",
            "LPT1.wav",
        ] {
            assert!(
                begin_in(
                    root.clone(),
                    Begin {
                        batch_id: batch,
                        name: name.into(),
                        size: 4
                    }
                )
                .is_err()
            );
        }
        let v = begin_in(
            root.clone(),
            Begin {
                batch_id: batch,
                name: "sound.wav".into(),
                size: 4,
            },
        )
        .unwrap();
        let id = Uuid::parse_str(v["id"].as_str().unwrap()).unwrap();
        assert!(chunk(id, 1, b"xx").is_err());
        assert!(finish(id).is_err());
        chunk(id, 0, b"ab").unwrap();
        assert!(chunk(id, 2, b"cde").is_err());
        chunk(id, 2, b"cd").unwrap();
        let final_path = finish(id).unwrap()["path"].as_str().unwrap().to_string();
        assert_eq!(std::fs::read(&final_path).unwrap(), b"abcd");
        assert!(chunk(id, 4, b"x").is_err());
        abort_batch(batch).unwrap();
        assert!(!std::path::Path::new(&final_path).exists());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn batch_cancel_keeps_retained_assets_and_removes_only_its_receipts() {
        let root = std::env::temp_dir().join(format!("pealayer-assets-{}", Uuid::new_v4()));
        let batch = Uuid::new_v4();
        let id = Uuid::parse_str(
            begin_in(
                root.clone(),
                Begin {
                    batch_id: batch,
                    name: "safe.wav".into(),
                    size: 1,
                },
            )
            .unwrap()["id"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        chunk(id, 0, b"a").unwrap();
        let path = finish(id).unwrap()["path"].as_str().unwrap().to_string();
        retain_paths(&[path.clone()]);
        let unfinished = Uuid::parse_str(
            begin_in(
                root.clone(),
                Begin {
                    batch_id: batch,
                    name: "partial.wav".into(),
                    size: 10,
                },
            )
            .unwrap()["id"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        abort_batch(batch).unwrap();
        assert!(finish(unfinished).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"a");
        std::fs::remove_dir_all(root).unwrap();
    }
}
