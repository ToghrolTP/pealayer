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
    path: PathBuf,
    final_path: PathBuf,
    size: u64,
    offset: u64,
    touched: Instant,
}
#[derive(Default)]
struct Uploads {
    active: HashMap<Uuid, Upload>,
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
    if request.size == 0
        || request.size > MAX_BYTES
        || request.name.is_empty()
        || request.name.len() > 240
        || request.name == "."
        || request.name == ".."
        || request.name.starts_with('.')
        || request.name.ends_with(['.', ' '])
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
            path,
            final_path,
            size: request.size,
            offset: 0,
            touched: Instant::now(),
        },
    );
    Ok(serde_json::json!({"id":id,"chunk_bytes":CHUNK_BYTES}))
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
    std::fs::OpenOptions::new().write(true).open(&upload.path)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    std::fs::rename(&upload.path, &upload.final_path).map_err(|e| e.to_string())?;
    let path = upload.final_path.clone();
    uploads.active.remove(&id);
    Ok(serde_json::json!({"path":path}))
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
) -> Result<(), String> {
    let batch = Uuid::new_v4();
    let mut paths = Vec::new();
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
                response(
                    client
                        .request(
                            reqwest::Method::PUT,
                            &format!("/api/assets/upload/chunk?id={id}&offset={offset}"),
                        )?
                        .timeout(Duration::from_secs(30))
                        .body(buffer[..n].to_vec())
                        .send()
                        .map_err(|e| e.to_string())?,
                )?;
                offset += n as u64;
            }
            client.post(
                &format!("/api/assets/upload/finish?id={id}"),
                &serde_json::json!({}),
            )
        })();
        match transferred {
            Ok(value) => paths.push(
                value["path"]
                    .as_str()
                    .ok_or("Missing uploaded asset path")?
                    .to_string(),
            ),
            Err(error) => {
                let _ = client
                    .request(
                        reqwest::Method::DELETE,
                        &format!("/api/assets/upload?id={id}"),
                    )
                    .and_then(|r| r.send().map_err(|e| e.to_string()));
                return Err(error);
            }
        }
    }
    client.post("/api/player/command",&serde_json::json!({"command":"import_assets","request":{"paths":paths,"destination":request.destination}}))?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordered_upload_rejects_traversal_overrun_and_incomplete_finish() {
        let root = std::env::temp_dir().join(format!("pealayer-assets-{}", Uuid::new_v4()));
        let batch = Uuid::new_v4();
        for name in ["../bad.wav", "C:\\bad.wav", ".hidden", "bad."] {
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
        assert_eq!(std::fs::read(final_path).unwrap(), b"abcd");
        assert!(chunk(id, 4, b"x").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
