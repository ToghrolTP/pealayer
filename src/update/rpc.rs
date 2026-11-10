//! Process-local updater transport. RPC and HTTP share UpdateManager: a relay
//! never implements its own executable replacement or bypasses the owner.
use super::*;
use crate::platform::interop::InteropCommand;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::io::{Seek, SeekFrom};
use std::sync::mpsc::Sender;

// Base64 stays well below the existing 1 MiB HTTP/bridge message ceiling.
pub const CHUNK_BYTES: usize = 384 * 1024;
pub const METHODS: &[&str] = &[
    "pealayer.update.targets",
    "pealayer.update.peer.call",
    "pealayer.update.controller.call",
    "pealayer.update.bundle.list",
    "pealayer.update.bundle.read",
    "pealayer.update.manifest",
    "pealayer.update.status",
    "pealayer.update.artifact.read",
    "pealayer.update.upload.begin",
    "pealayer.update.upload.chunk",
    "pealayer.update.upload.finish",
    "pealayer.update.upload.abort",
    "pealayer.update.from-url",
];

pub fn is_method(method: &str) -> bool {
    method.starts_with("pealayer.update.")
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Forward {
    target: String,
    method: String,
    #[serde(default)]
    params: Value,
    #[serde(default)]
    route: Vec<String>,
}

fn direct_method(method: &str) -> bool {
    METHODS.contains(&method)
        && !matches!(
            method,
            "pealayer.update.targets"
                | "pealayer.update.peer.call"
                | "pealayer.update.controller.call"
        )
}

fn controller_method(method: &str) -> bool {
    matches!(
        method,
        "controller.software.targets"
            | "controller.software.call"
            | "controller.software.bundle.list"
            | "controller.software.bundle.read"
            | "controller.artifact.manifest"
            | "controller.artifact.list"
            | "controller.artifact.fetch"
            | "controller.artifact.upload.begin"
            | "controller.artifact.upload.chunk"
            | "controller.artifact.upload.finish"
            | "controller.artifact.upload.abort"
            | "controller.artifact.download.begin"
            | "controller.artifact.download.chunk"
            | "controller.artifact.download.close"
            | "controller.update.host"
            | "controller.update.status"
    )
}

fn peer_call(request: Forward) -> Result<Value, String> {
    if !direct_method(&request.method) {
        return Err(
            "Only direct updater operations may be forwarded; recursive relays are forbidden"
                .into(),
        );
    }
    let client = crate::peer::client().ok_or("No Pealayer authority is connected")?;
    let session = client
        .snapshot()
        .ok_or("Authority identity is unavailable")?;
    if request.target != format!("authority:{}", session.session.instance_id) {
        return Err("Authority changed; refresh update targets before sending".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let envelope = client.post(
        "/api/rpc",
        &json!({"jsonrpc":"2.0","id":id,"method":request.method,"params":request.params}),
    )?;
    if envelope["id"] != id || envelope["jsonrpc"] != "2.0" {
        return Err("Authority updater returned an invalid acknowledgement".into());
    }
    if let Some(error) = envelope.get("error") {
        return Err(format!(
            "Authority updater rejected request: {}",
            error["message"]
        ));
    }
    envelope
        .get("result")
        .cloned()
        .ok_or_else(|| "Authority updater omitted its result".into())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Chunk {
    operation_id: String,
    offset: u64,
    data: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadChunk {
    sha256: String,
    offset: u64,
    length: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BundleChunk {
    asset_id: String,
    sha256: String,
    offset: u64,
    length: usize,
}
static BUNDLES: std::sync::LazyLock<Mutex<std::collections::BTreeMap<String, ArtifactReader>>> =
    std::sync::LazyLock::new(Default::default);

fn read_bundle(request: BundleChunk) -> Result<Value, String> {
    let mut cache = BUNDLES
        .lock()
        .map_err(|_| "Software bundle reader unavailable")?;
    let key = format!("{}:{}", request.asset_id, request.sha256);
    if !cache.contains_key(&key) {
        if cache.len() >= 16
            && let Some(oldest) = cache.keys().next().cloned()
        {
            cache.remove(&oldest);
        }
        let (value, file) = super::bundle::open(&request.asset_id)?;
        let mut manifest = current_manifest()?;
        manifest.sha256 = value["sha256"]
            .as_str()
            .ok_or("Bundle SHA-256 required")?
            .into();
        manifest.size = value["size"].as_u64().ok_or("Bundle size required")?;
        if manifest.sha256 != request.sha256 {
            return Err("Software bundle identity changed".into());
        }
        cache.insert(
            key.clone(),
            ArtifactReader {
                manifest,
                file: Mutex::new(file),
            },
        );
    }
    cache.get(&key).unwrap().read(ReadChunk {
        sha256: request.sha256,
        offset: request.offset,
        length: request.length,
    })
}

struct ArtifactReader {
    manifest: UpdateManifest,
    file: Mutex<File>,
}
static ARTIFACT: OnceLock<Result<ArtifactReader, String>> = OnceLock::new();

impl ArtifactReader {
    fn current() -> Result<Self, String> {
        // Pin one open, verified image for the lifetime of this process. Do not
        // rehash/read the entire executable for every individual download chunk.
        let manifest = current_manifest()?;
        let path = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut buffer = [0; 64 * 1024];
        loop {
            let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        if sha256_hex(hash.finalize()) != manifest.sha256 {
            return Err("Running update artifact changed while opening it".into());
        }
        Ok(Self {
            manifest,
            file: Mutex::new(file),
        })
    }

    fn read(&self, request: ReadChunk) -> Result<Value, String> {
        if normalize_sha256(&request.sha256)? != self.manifest.sha256 {
            return Err("Update artifact identity changed; obtain a fresh manifest".into());
        }
        if request.length == 0
            || request.length > CHUNK_BYTES
            || request.offset > self.manifest.size
        {
            return Err(format!(
                "Artifact read requires an in-range offset and 1..{CHUNK_BYTES} bytes"
            ));
        }
        let mut bytes = vec![
            0;
            request
                .length
                .min((self.manifest.size - request.offset) as usize)
        ];
        let mut file = self
            .file
            .lock()
            .map_err(|_| "Update artifact reader unavailable")?;
        file.seek(SeekFrom::Start(request.offset))
            .map_err(|e| e.to_string())?;
        file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        let next = request.offset + bytes.len() as u64;
        Ok(
            json!({"sha256":self.manifest.sha256,"offset":request.offset,
            "next_offset":next,"size":self.manifest.size,"data":STANDARD.encode(bytes),
            "eof":next == self.manifest.size}),
        )
    }
}

fn empty(params: Value) -> Result<(), String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Empty {}
    serde_json::from_value::<Empty>(if params.is_null() { json!({}) } else { params })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn dispatch(
    manager: &UpdateManager,
    method: &str,
    params: Value,
    tx: &Sender<InteropCommand>,
) -> Result<Value, String> {
    let status = match method {
        "pealayer.update.bundle.list" => {
            empty(params)?;
            return super::bundle::list();
        }
        "pealayer.update.bundle.read" => {
            return read_bundle(serde_json::from_value(params).map_err(|e| e.to_string())?);
        }
        "pealayer.update.targets" => {
            empty(params)?;
            let mut targets = vec![
                json!({"id":"self","product":"pealayer","instance_id":crate::peer::instance_id()}),
            ];
            if let Some(client) = crate::peer::client()
                && let Some(session) = client.snapshot()
            {
                targets.push(json!({"id":format!("authority:{}",session.session.instance_id),"product":"pealayer","origin":client.origin.as_str()}));
            }
            return Ok(
                json!({"targets":targets,"controller_catalog_method":"pealayer.update.controller.call"}),
            );
        }
        "pealayer.update.peer.call" => {
            return peer_call(serde_json::from_value(params).map_err(|e| e.to_string())?);
        }
        "pealayer.update.controller.call" => {
            let mut request: Forward = serde_json::from_value(params).map_err(|e| e.to_string())?;
            if request.target != "controller" || !controller_method(&request.method) {
                return Err(
                    "Choose the connected controller and a software update operation".into(),
                );
            }
            if request.route.len() >= 8
                || request
                    .route
                    .iter()
                    .any(|id| id == crate::peer::instance_id())
            {
                return Err("Software update forwarding loop detected".into());
            }
            return if let Some(client) = crate::peer::client() {
                request.route.push(crate::peer::instance_id().into());
                let id = uuid::Uuid::new_v4().to_string();
                let response=client.post("/api/rpc", &json!({"jsonrpc":"2.0","id":id,"method":"pealayer.update.controller.call",
                    "params":{"target":"controller","method":request.method,"params":request.params,"route":request.route}}))?;
                if response["id"] != id || response.get("error").is_some() {
                    return Err("Authority did not acknowledge the controller update request; do not replay mutations".into());
                }
                response
                    .get("result")
                    .cloned()
                    .ok_or_else(|| "Authority omitted controller update acknowledgement".into())
            } else {
                crate::peer::controller_call(request.method, request.params)
            };
        }
        "pealayer.update.manifest" => {
            empty(params)?;
            let mut value = serde_json::to_value(current_manifest()?).map_err(|e| e.to_string())?;
            value["rpc"] = json!({"methods":METHODS,"chunk_bytes":CHUNK_BYTES,"encoding":"base64"});
            return Ok(value);
        }
        "pealayer.update.artifact.read" => {
            let request: ReadChunk = serde_json::from_value(params).map_err(|e| e.to_string())?;
            return ARTIFACT
                .get_or_init(ArtifactReader::current)
                .as_ref()
                .map_err(Clone::clone)?
                .read(request);
        }
        "pealayer.update.status" => {
            empty(params)?;
            manager.status()
        }
        "pealayer.update.upload.begin" => {
            manager.begin_upload(serde_json::from_value(params).map_err(|e| e.to_string())?)?
        }
        "pealayer.update.upload.chunk" => {
            let request: Chunk = serde_json::from_value(params).map_err(|e| e.to_string())?;
            if request.data.len() > CHUNK_BYTES.div_ceil(3) * 4 {
                return Err("Update chunk exceeds the RPC limit".into());
            }
            let bytes = STANDARD
                .decode(&request.data)
                .map_err(|_| "Update chunk is not valid base64")?;
            if bytes.len() > CHUNK_BYTES {
                return Err("Update chunk exceeds the RPC limit".into());
            }
            manager.append_chunk(&request.operation_id, request.offset, &bytes)?
        }
        "pealayer.update.upload.finish" => {
            let request: FinishUploadRequest =
                serde_json::from_value(params).map_err(|e| e.to_string())?;
            manager.finish_upload(&request.operation_id, tx.clone())?
        }
        "pealayer.update.upload.abort" => {
            let request: FinishUploadRequest =
                serde_json::from_value(params).map_err(|e| e.to_string())?;
            manager.abort_upload(&request.operation_id)?
        }
        "pealayer.update.from-url" => manager.fetch_and_apply(
            serde_json::from_value(params).map_err(|e| e.to_string())?,
            tx.clone(),
        )?,
        _ => return Err("Unknown Pealayer updater method".into()),
    };
    serde_json::to_value(status).map_err(|e| e.to_string())
}

pub fn payload(payload: &str, web: bool, tx: &Sender<InteropCommand>) -> Option<String> {
    let request: crate::platform::interop::JsonRpcRequest = serde_json::from_str(payload).ok()?;
    if !is_method(&request.method) {
        return None;
    }
    if request
        .jsonrpc
        .as_deref()
        .is_some_and(|version| version != "2.0")
    {
        return Some(crate::platform::interop::json_rpc_error(
            &request.id,
            -32600,
            "jsonrpc must be 2.0",
        ));
    }
    let config = crate::platform::interop::get_live_config();
    if web && !config.web_allow_updates {
        return Some(crate::platform::interop::json_rpc_error(
            &request.id,
            -32003,
            "Web update permission is disabled",
        ));
    }
    if !METHODS.contains(&request.method.as_str()) {
        return Some(crate::platform::interop::json_rpc_error(
            &request.id,
            -32601,
            "Unknown Pealayer updater method",
        ));
    }
    Some(
        match dispatch(&manager(), &request.method, request.params, tx) {
            Ok(value) => crate::platform::interop::json_rpc_result(&request.id, value),
            Err(error) => crate::platform::interop::json_rpc_error(&request.id, -32602, &error),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rpc_upload_reuses_manager_and_rejects_bad_offsets_encoding_and_identity() {
        let manager = UpdateManager::default();
        let (tx, rx) = std::sync::mpsc::channel();
        let bytes = b"isolated transfer fixture";
        let hash = sha256_hex(Sha256::digest(bytes));
        let begin = dispatch(
            &manager,
            "pealayer.update.upload.begin",
            json!({"size":bytes.len(),"sha256":hash}),
            &tx,
        )
        .unwrap();
        let id = begin["operation_id"].as_str().unwrap();
        for params in [
            json!({"operation_id":id,"offset":1,"data":STANDARD.encode(bytes)}),
            json!({"operation_id":id,"offset":0,"data":"not base64!"}),
            json!({"operation_id":"other","offset":0,"data":STANDARD.encode(bytes)}),
        ] {
            assert!(dispatch(&manager, "pealayer.update.upload.chunk", params, &tx).is_err());
        }
        let status = dispatch(
            &manager,
            "pealayer.update.upload.chunk",
            json!({"operation_id":id,"offset":0,"data":STANDARD.encode(bytes)}),
            &tx,
        )
        .unwrap();
        assert_eq!(status["bytes_done"], bytes.len());
        assert_eq!(status["sha256"], hash);
        let status = dispatch(
            &manager,
            "pealayer.update.upload.abort",
            json!({"operation_id":id}),
            &tx,
        )
        .unwrap();
        assert_eq!(status["state"], "aborted");
        assert!(
            rx.try_recv().is_err(),
            "transfer/abort must never queue Quit or hardware actions"
        );
    }

    #[test]
    fn artifact_rpc_is_bounded_and_pinned_to_the_manifest() {
        let reader = ArtifactReader::current().unwrap();
        let read = |sha256, offset, length| {
            reader.read(ReadChunk {
                sha256,
                offset,
                length,
            })
        };
        let first = read(reader.manifest.sha256.clone(), 0, 16).unwrap();
        assert_eq!(
            STANDARD
                .decode(first["data"].as_str().unwrap())
                .unwrap()
                .len(),
            16
        );
        assert_eq!(first["next_offset"], 16);
        assert!(read("a".repeat(64), 0, 16).is_err());
        assert!(read(reader.manifest.sha256.clone(), 0, CHUNK_BYTES + 1).is_err());
        assert!(read(reader.manifest.sha256.clone(), reader.manifest.size + 1, 16).is_err());
        let last = read(reader.manifest.sha256.clone(), reader.manifest.size, 16).unwrap();
        assert_eq!(last["eof"], true);
        assert_eq!(last["data"], "");
    }

    #[test]
    fn software_forwarding_never_grants_general_proxy_or_recursive_calls() {
        for method in [
            "controller.bridge.call",
            "controller.relay.set",
            "controller.estop.set",
            "controller.update.firmware",
            "pealayer.process.connect",
        ] {
            assert!(!controller_method(method), "{method}");
        }
        for method in [
            "pealayer.update.peer.call",
            "pealayer.update.controller.call",
            "pealayer.update.targets",
            "pealayer.play",
        ] {
            assert!(!direct_method(method), "{method}");
        }
        assert!(direct_method("pealayer.update.upload.chunk"));
        assert!(controller_method("controller.update.host"));
        assert!(controller_method("controller.software.call"));
    }
}
