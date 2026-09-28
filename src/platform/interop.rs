use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
#[cfg(unix)]
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaunchRequest {
    pub operation_id: String,
    pub application_identity: String,
    pub sender_session_id: Option<u32>,
    pub sender_working_directory: Option<String>,
    pub target: Option<String>,
    pub fullscreen: bool,
    pub volume: Option<f64>,
    pub activate: bool,
}

impl LaunchRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.operation_id.trim().is_empty() || self.operation_id.len() > 128 {
            return Err("launch operation_id must contain 1 to 128 bytes".to_string());
        }
        if self.application_identity.trim().is_empty() || self.application_identity.len() > 256 {
            return Err("launch application_identity must contain 1 to 256 bytes".to_string());
        }
        if self
            .sender_working_directory
            .as_ref()
            .is_some_and(|value| value.len() > 32_768)
            || self.target.as_ref().is_some_and(|value| value.len() > 32_768)
        {
            return Err("launch path fields must not exceed 32768 bytes".to_string());
        }
        if self
            .volume
            .is_some_and(|value| !value.is_finite() || !(0.0..=130.0).contains(&value))
        {
            return Err("launch volume must be a finite value from 0 to 130".to_string());
        }
        Ok(())
    }
}

fn normalized_application_identity(identity: &str) -> String {
    identity.trim().to_lowercase()
}

fn validate_launch_destination(request: &LaunchRequest) -> Result<(), String> {
    let expected = crate::config::resolved_app_name(&crate::config::AppConfig::load());
    if normalized_application_identity(&request.application_identity)
        != normalized_application_identity(&expected)
    {
        return Err("launch request targets a different application identity".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        let expected_session = crate::platform::windows::current_session_id()?;
        if request.sender_session_id != Some(expected_session) {
            return Err("launch request targets a different Windows session".to_string());
        }
    }
    Ok(())
}

#[derive(Default)]
struct LaunchReceiptCache {
    operation_ids: std::collections::VecDeque<String>,
}

impl LaunchReceiptCache {
    const CAPACITY: usize = 1024;

    fn claim(&mut self, operation_id: &str) -> bool {
        if self.operation_ids.iter().any(|known| known == operation_id) {
            return false;
        }
        if self.operation_ids.len() == Self::CAPACITY {
            self.operation_ids.pop_front();
        }
        self.operation_ids.push_back(operation_id.to_string());
        true
    }

    fn release(&mut self, operation_id: &str) {
        self.operation_ids.retain(|known| known != operation_id);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum InteropCommand {
    Launch { request: LaunchRequest },
    Play,
    Pause,
    TogglePause,
    Seek { seconds: f64 },
    SeekAbs { percentage: f64 },
    #[serde(alias = "volume")]
    SetVolume {
        #[serde(alias = "level")]
        value: f64,
    },
    #[serde(alias = "open_video")]
    Open {
        #[serde(alias = "path")]
        target: String,
    },
    GetStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStatusResponse {
    pub status: String,
    pub playing: bool,
    pub volume: f64,
    pub playback_time: f64,
    pub duration: f64,
    pub current_video: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(default)]
    pub jsonrpc: Option<String>,
    #[serde(default)]
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

pub fn command_from_json_rpc(request: &JsonRpcRequest) -> Result<Option<InteropCommand>, String> {
    if request.jsonrpc.as_deref().is_some_and(|version| version != "2.0") {
        return Err("unsupported JSON-RPC version".to_string());
    }
    let number = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| request.params.get(*name).and_then(Value::as_f64))
            .ok_or_else(|| format!("missing numeric parameter: {}", names.join(" or ")))
    };
    let string = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| request.params.get(*name).and_then(Value::as_str))
            .map(str::to_string)
            .ok_or_else(|| format!("missing string parameter: {}", names.join(" or ")))
    };
    match request.method.as_str() {
        "play" | "pealayer.play" | "pealayer.player.play" => Ok(Some(InteropCommand::Play)),
        "pause" | "pealayer.pause" | "pealayer.player.pause" => {
            Ok(Some(InteropCommand::Pause))
        }
        "toggle" | "toggle_pause" | "pealayer.toggle" | "pealayer.player.toggle" => {
            Ok(Some(InteropCommand::TogglePause))
        }
        "seek" | "pealayer.seek" | "pealayer.player.seek" => {
            Ok(Some(InteropCommand::Seek {
                seconds: number(&["seconds"] )?,
            }))
        }
        "seek_abs" | "pealayer.seek_absolute" | "pealayer.player.seek_absolute" => {
            Ok(Some(InteropCommand::SeekAbs {
                percentage: number(&["percentage"] )?,
            }))
        }
        "volume" | "set_volume" | "pealayer.volume.set" | "pealayer.player.volume.set" => {
            Ok(Some(InteropCommand::SetVolume {
                value: number(&["value", "level"] )?,
            }))
        }
        "open" | "open_video" | "pealayer.open" | "pealayer.player.open" => {
            Ok(Some(InteropCommand::Open {
                target: string(&["target", "path"] )?,
            }))
        }
        "get_status" | "player.status" | "pealayer.status" | "pealayer.player.status" => {
            Ok(None)
        }
        method => Err(format!("unknown Pealayer JSON-RPC method: {method}")),
    }
}

pub fn json_rpc_result(id: &Value, result: Value) -> String {
    serde_json::json!({"jsonrpc":"2.0","id":id,"result":result}).to_string()
}

pub fn json_rpc_error(id: &Value, code: i32, message: &str) -> String {
    serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
        .to_string()
}

static LIVE_STATUS: std::sync::RwLock<Option<PlayerStatusResponse>> = std::sync::RwLock::new(None);

pub fn set_live_status(status: PlayerStatusResponse) {
    if let Ok(mut lock) = LIVE_STATUS.write() {
        *lock = Some(status);
    }
}

pub fn get_live_status() -> PlayerStatusResponse {
    if let Ok(lock) = LIVE_STATUS.read() {
        if let Some(ref st) = *lock {
            return st.clone();
        }
    }
    PlayerStatusResponse {
        status: "initializing".to_string(),
        playing: false,
        volume: 0.0,
        playback_time: 0.0,
        duration: 0.0,
        current_video: None,
    }
}

pub fn get_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("pealayer.sock")
    } else {
        PathBuf::from("/tmp").join("pealayer.sock")
    }
}

pub fn parse_interop_request(line: &str) -> Result<(Option<serde_json::Value>, InteropCommand), String> {
    let val: serde_json::Value = serde_json::from_str(line).map_err(|e| e.to_string())?;

    // Check if JSON-RPC 2.0 format
    if val.get("jsonrpc").and_then(|v| v.as_str()) == Some("2.0") || val.get("method").is_some() {
        let id = val.get("id").cloned();
        let method = val.get("method").and_then(|m| m.as_str()).ok_or("Missing method field")?;
        let params = val.get("params");

        let cmd = match method {
            "play" => InteropCommand::Play,
            "pause" => InteropCommand::Pause,
            "toggle_pause" | "toggle" => InteropCommand::TogglePause,
            "seek" => {
                let seconds = params.and_then(|p| p.get("seconds")).and_then(|s| s.as_f64()).unwrap_or(0.0);
                InteropCommand::Seek { seconds }
            }
            "seek_abs" => {
                let percentage = params.and_then(|p| p.get("percentage")).and_then(|p| p.as_f64()).unwrap_or(0.0);
                InteropCommand::SeekAbs { percentage }
            }
            "set_volume" | "volume" => {
                let value = params.and_then(|p| p.get("value").or_else(|| p.get("level"))).and_then(|v| v.as_f64()).unwrap_or(100.0);
                InteropCommand::SetVolume { value }
            }
            "open" | "open_video" => {
                let target = params.and_then(|p| p.get("target").or_else(|| p.get("path"))).and_then(|t| t.as_str()).unwrap_or("").to_string();
                InteropCommand::Open { target }
            }
            "get_status" | "player.status" => InteropCommand::GetStatus,
            other => return Err(format!("Unknown RPC method: {}", other)),
        };

        return Ok((id, cmd));
    }

    // Fall back to standard InteropCommand deserialization
    let cmd: InteropCommand = serde_json::from_value(val).map_err(|e| e.to_string())?;
    if let InteropCommand::Launch { request } = &cmd {
        request.validate()?;
    }
    Ok((None, cmd))
}

pub fn format_interop_response(id: Option<serde_json::Value>, result: &serde_json::Value) -> String {
    if let Some(id_val) = id {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id_val,
            "result": result
        }).to_string() + "\n"
    } else {
        result.to_string() + "\n"
    }
}

pub fn format_interop_error(id: Option<serde_json::Value>, code: i32, message: &str) -> String {
    if let Some(id_val) = id {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id_val,
            "error": {
                "code": code,
                "message": message
            }
        }).to_string() + "\n"
    } else {
        serde_json::json!({
            "status": "error",
            "message": message
        }).to_string() + "\n"
    }
}

fn handle_client_connection<R: std::io::Read, W: Write>(
    mut reader: BufReader<R>,
    mut writer: W,
    tx: std::sync::mpsc::Sender<InteropCommand>,
    egui_ctx: eframe::egui::Context,
    launch_receipts: Arc<Mutex<LaunchReceiptCache>>,
) {
    let mut line = String::new();
    while let Ok(n) = reader.read_line(&mut line) {
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            match parse_interop_request(trimmed) {
                Ok((id, InteropCommand::GetStatus)) => {
                    let status = get_live_status();
                    let resp_val = serde_json::to_value(&status).unwrap_or(serde_json::json!({"status": "ok"}));
                    let resp = format_interop_response(id, &resp_val);
                    let _ = writer.write_all(resp.as_bytes());
                    let _ = writer.flush();
                }
                Ok((id, cmd)) => {
                    let operation_id = match &cmd {
                        InteropCommand::Launch { request } => {
                            if let Err(error) = validate_launch_destination(request) {
                                let response = format_interop_error(id, -32600, &error);
                                let _ = writer.write_all(response.as_bytes());
                                let _ = writer.flush();
                                line.clear();
                                continue;
                            }
                            Some(request.operation_id.clone())
                        }
                        _ => None,
                    };
                    let response = if let Some(operation_id) = operation_id {
                        match launch_receipts.lock() {
                            Ok(mut receipts) if !receipts.claim(&operation_id) => {
                                format_interop_response(
                                    id,
                                    &serde_json::json!({"status": "accepted", "duplicate": true}),
                                )
                            }
                            Ok(mut receipts) => {
                                if tx.send(cmd).is_ok() {
                                    egui_ctx.request_repaint();
                                    format_interop_response(
                                        id,
                                        &serde_json::json!({"status": "accepted"}),
                                    )
                                } else {
                                    receipts.release(&operation_id);
                                    format_interop_error(
                                        id,
                                        -32000,
                                        "application dispatcher is unavailable",
                                    )
                                }
                            }
                            Err(_) => format_interop_error(
                                id,
                                -32000,
                                "launch receipt cache is unavailable",
                            ),
                        }
                    } else if tx.send(cmd).is_ok() {
                        egui_ctx.request_repaint();
                        format_interop_response(id, &serde_json::json!({"status": "accepted"}))
                    } else {
                        format_interop_error(id, -32000, "application dispatcher is unavailable")
                    };
                    let _ = writer.write_all(response.as_bytes());
                    let _ = writer.flush();
                }
                Err(err) => {
                    let err_resp = format_interop_error(None, -32600, &err);
                    let _ = writer.write_all(err_resp.as_bytes());
                    let _ = writer.flush();
                }
            }
        }
        line.clear();
    }
}

pub fn spawn_interop_listener(tx: std::sync::mpsc::Sender<InteropCommand>, egui_ctx: eframe::egui::Context) {
    // 1. Cross-platform loopback TCP listener. The overridable port allows
    // isolated test and screenshot profiles without displacing a live app.
    let tx_tcp = tx.clone();
    let ctx_tcp = egui_ctx.clone();
    let launch_receipts = Arc::new(Mutex::new(LaunchReceiptCache::default()));
    let tcp_launch_receipts = launch_receipts.clone();
    thread::spawn(move || {
        let ipc_port = crate::config::runtime_port("PEALAYER_IPC_PORT", 8082);
        let address = format!("127.0.0.1:{ipc_port}");
        if let Ok(listener) = TcpListener::bind(&address) {
            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    let tx_conn = tx_tcp.clone();
                    let ctx_conn = ctx_tcp.clone();
                    let receipts_conn = tcp_launch_receipts.clone();
                    thread::spawn(move || {
                        if let Ok(read_clone) = stream.try_clone() {
                            let reader = BufReader::new(read_clone);
                            handle_client_connection(reader, stream, tx_conn, ctx_conn, receipts_conn);
                        }
                    });
                }
            }
        } else {
            log::warn!("Could not bind loopback IPC TCP listener to {address}");
        }
    });

    // 2. Native Unix domain socket listener on Unix targets
    #[cfg(unix)]
    {
        let tx_unix = tx.clone();
        let ctx_unix = egui_ctx.clone();
        let unix_launch_receipts = launch_receipts.clone();
        thread::spawn(move || {
            let socket_path = get_socket_path();
            if socket_path.exists() {
                let _ = std::fs::remove_file(&socket_path);
            }

            if let Ok(listener) = UnixListener::bind(&socket_path) {
                for stream in listener.incoming() {
                    if let Ok(stream) = stream {
                        let tx_conn = tx_unix.clone();
                        let ctx_conn = ctx_unix.clone();
                        let receipts_conn = unix_launch_receipts.clone();
                        thread::spawn(move || {
                            if let Ok(read_clone) = stream.try_clone() {
                                let reader = BufReader::new(read_clone);
                                handle_client_connection(reader, stream, tx_conn, ctx_conn, receipts_conn);
                            }
                        });
                    }
                }
            }
        });
    }
}

pub fn spawn_interop_server(egui_ctx: eframe::egui::Context) -> Receiver<InteropCommand> {
    let (tx, rx) = channel::<InteropCommand>();
    spawn_interop_listener(tx, egui_ctx);
    rx
}

const PCCONTROLLER_WEBSOCKET_ENDPOINT: &str = "ws://127.0.0.1:8787/ipc";
const PCCONTROLLER_ACTIONS: &str = "app.page,pealayer.play,pealayer.pause,pealayer.toggle,pealayer.seek,pealayer.seek_absolute,pealayer.volume.set,pealayer.open";

struct ControllerAction {
    command: Option<InteropCommand>,
    acknowledgement: Value,
    receipt_key: String,
}

/// A targeted PCController action. The UI acknowledges it only after applying
/// the command, so PCController never records queueing as successful execution.
pub struct ControllerDelivery {
    pub command: InteropCommand,
    acknowledgement: Value,
    acknowledgement_tx: std::sync::mpsc::Sender<Value>,
}

impl ControllerDelivery {
    pub fn acknowledge_applied(self) {
        let _ = self.acknowledgement_tx.send(self.acknowledgement);
    }
}

fn controller_action_from_event(event: &Value, instance_id: &str) -> Option<ControllerAction> {
    let metadata = event.get("metadata")?.as_object()?;
    if metadata.get("target_instance")?.as_str()? != instance_id {
        return None;
    }
    let operation_id = metadata.get("operation_id")?.as_str()?.trim();
    let delivery_id = metadata.get("operation_delivery_id")?.as_str()?.trim();
    let expires_at = metadata.get("operation_expires_at")?.as_str()?.trim();
    if operation_id.is_empty() || delivery_id.is_empty() || expires_at.is_empty() {
        return None;
    }

    let kind = event.get("kind")?.as_str()?.trim().to_ascii_lowercase();
    let value = metadata
        .get(if kind == "app.page" { "page" } else { "value" })
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let expired = expires_at
        .parse::<jiff::Timestamp>()
        .map(|deadline| deadline <= jiff::Timestamp::now())
        .unwrap_or(true);
    let command = if expired {
        None
    } else {
        match kind.as_str() {
            "pealayer.play" => Some(InteropCommand::Play),
            "pealayer.pause" => Some(InteropCommand::Pause),
            "pealayer.toggle" => Some(InteropCommand::TogglePause),
            "pealayer.seek" => value
                .parse::<f64>()
                .ok()
                .map(|seconds| InteropCommand::Seek { seconds }),
            "pealayer.seek_absolute" => value
                .parse::<f64>()
                .ok()
                .map(|percentage| InteropCommand::SeekAbs { percentage }),
            "pealayer.volume.set" => value
                .parse::<f64>()
                .ok()
                .map(|value| InteropCommand::SetVolume { value }),
            "pealayer.open" if !value.is_empty() => Some(InteropCommand::Open {
                target: value.to_string(),
            }),
            "app.page" => match value.to_ascii_lowercase().as_str() {
                "play" | "player.play" => Some(InteropCommand::Play),
                "pause" | "player.pause" => Some(InteropCommand::Pause),
                "toggle" | "player.toggle" => Some(InteropCommand::TogglePause),
                "back" | "previous" | "rewind" => {
                    Some(InteropCommand::Seek { seconds: -10.0 })
                }
                "forward" | "next" => Some(InteropCommand::Seek { seconds: 10.0 }),
                _ => None,
            },
            _ => None,
        }
    };
    let reason = if expired {
        Some("expired")
    } else if command.is_none() {
        Some("unsupported_or_invalid_pealayer_action")
    } else {
        None
    };
    let mut acknowledgement = serde_json::json!({
        "operation_id": operation_id,
        "delivery_id": delivery_id,
        "instance_id": instance_id,
        "state": if command.is_some() { "applied" } else { "rejected" },
    });
    if let Some(reason) = reason {
        acknowledgement["reason"] = Value::String(reason.to_string());
    }
    Some(ControllerAction {
        command,
        acknowledgement,
        receipt_key: format!("{operation_id}\0{delivery_id}"),
    })
}

fn controller_rpc(id: u64, method: &str, params: Value) -> tungstenite::Message {
    tungstenite::Message::Text(
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        })
        .to_string()
        .into(),
    )
}

fn report_controller_instance(
    socket: &mut tungstenite::WebSocket<
        tungstenite::stream::MaybeTlsStream<std::net::TcpStream>,
    >,
    id: u64,
    instance_id: &str,
) -> Result<(), String> {
    socket
        .send(controller_rpc(
            id,
            "controller.app.instance.report",
            serde_json::json!({
                "id": instance_id,
                "surface": "pealayer",
                "page": "player",
                "state": "active",
                "lease_seconds": 45,
                "self": {
                    "kind": "native",
                    "vars": {
                        "pid": std::process::id().to_string(),
                        "rpc": format!("http://127.0.0.1:{}/api/rpc", crate::config::runtime_port("PEALAYER_HTTP_PORT", 8080)),
                        "websocket": format!("ws://127.0.0.1:{}", crate::config::runtime_port("PEALAYER_WS_PORT", 8081)),
                        "ipc": format!("tcp://127.0.0.1:{}", crate::config::runtime_port("PEALAYER_IPC_PORT", 8082)),
                    },
                },
                "values": {
                    "app_actions": PCCONTROLLER_ACTIONS,
                    "coordinator": "pccontroller",
                    "serial_owner": "pccontroller",
                },
            }),
        ))
        .map_err(|error| format!("report Pealayer instance to PCController: {error}"))
}

fn send_action_ack(
    socket: &mut tungstenite::WebSocket<
        tungstenite::stream::MaybeTlsStream<std::net::TcpStream>,
    >,
    next_id: &mut u64,
    acknowledgement: Value,
) -> Result<(), String> {
    socket
        .send(controller_rpc(
            *next_id,
            "controller.app.action.ack",
            acknowledgement,
        ))
        .map_err(|error| format!("acknowledge PCController action: {error}"))?;
    *next_id = next_id.wrapping_add(1).max(1);
    Ok(())
}

fn run_pccontroller_action_bridge(
    tx: &std::sync::mpsc::Sender<ControllerDelivery>,
    egui_ctx: &eframe::egui::Context,
) -> Result<(), String> {
    use std::collections::{HashSet, VecDeque};
    use std::io::ErrorKind;
    use std::time::{Duration, Instant};
    use tungstenite::Message;

    let (mut socket, _) = tungstenite::connect(PCCONTROLLER_WEBSOCKET_ENDPOINT)
        .map_err(|error| format!("connect to PCController action WebSocket: {error}"))?;
    if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .map_err(|error| format!("configure PCController action read timeout: {error}"))?;
    }

    let instance_id = format!("pealayer:desktop-{}", std::process::id());
    let mut next_id = 1_u64;
    socket
        .send(controller_rpc(
            next_id,
            "controller.subscribe",
            serde_json::json!({"topics":["state","events","opcodes"],"after_id":0}),
        ))
        .map_err(|error| format!("subscribe to PCController actions: {error}"))?;
    next_id += 1;
    report_controller_instance(&mut socket, next_id, &instance_id)?;
    next_id += 1;

    let (acknowledgement_tx, acknowledgement_rx) = std::sync::mpsc::channel();
    let mut last_report = Instant::now();
    let mut receipts = HashSet::new();
    let mut receipt_order = VecDeque::new();
    loop {
        while let Ok(acknowledgement) = acknowledgement_rx.try_recv() {
            send_action_ack(&mut socket, &mut next_id, acknowledgement)?;
        }
        if last_report.elapsed() >= Duration::from_secs(30) {
            report_controller_instance(&mut socket, next_id, &instance_id)?;
            next_id = next_id.wrapping_add(1).max(1);
            last_report = Instant::now();
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                let Ok(message) = serde_json::from_str::<Value>(&text) else {
                    continue;
                };
                if !matches!(
                    message.get("method").and_then(Value::as_str),
                    Some("controller.state" | "controller.event")
                ) {
                    continue;
                }
                let Some(action) = message
                    .get("params")
                    .and_then(|event| controller_action_from_event(event, &instance_id))
                else {
                    continue;
                };
                let first_delivery = receipts.insert(action.receipt_key.clone());
                if first_delivery {
                    receipt_order.push_back(action.receipt_key.clone());
                    if receipt_order.len() > 256 {
                        if let Some(oldest) = receipt_order.pop_front() {
                            receipts.remove(&oldest);
                        }
                    }
                    if let Some(command) = action.command {
                        tx.send(ControllerDelivery {
                            command,
                            acknowledgement: action.acknowledgement,
                            acknowledgement_tx: acknowledgement_tx.clone(),
                        })
                        .map_err(|error| {
                            format!("deliver PCController action to player: {error}")
                        })?;
                        egui_ctx.request_repaint();
                    } else {
                        send_action_ack(
                            &mut socket,
                            &mut next_id,
                            action.acknowledgement,
                        )?;
                    }
                }
            }
            Ok(Message::Ping(payload)) => {
                socket
                    .send(Message::Pong(payload))
                    .map_err(|error| format!("answer PCController ping: {error}"))?;
            }
            Ok(Message::Close(_)) => {
                return Err("PCController action WebSocket closed".to_string())
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(error) => return Err(format!("read PCController action WebSocket: {error}")),
        }
    }
}

pub fn spawn_pccontroller_action_bridge(
    egui_ctx: eframe::egui::Context,
) -> Receiver<ControllerDelivery> {
    let (tx, rx) = channel();
    thread::spawn(move || loop {
        if let Err(error) = run_pccontroller_action_bridge(&tx, &egui_ctx) {
            log::warn!("[PCController] {error}; retrying in 2 seconds");
        }
        thread::sleep(std::time::Duration::from_secs(2));
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_interop_commands() {
        let play_json = r#"{"command":"play"}"#;
        let cmd: InteropCommand = serde_json::from_str(play_json).unwrap();
        assert!(matches!(cmd, InteropCommand::Play));

        let seek_json = r#"{"command":"seek","seconds":10.5}"#;
        let cmd: InteropCommand = serde_json::from_str(seek_json).unwrap();
        if let InteropCommand::Seek { seconds } = cmd {
            assert_eq!(seconds, 10.5);
        } else {
            panic!("Expected Seek command");
        }

        let open_json = r#"{"command":"open","target":"/video.mp4"}"#;
        let cmd: InteropCommand = serde_json::from_str(open_json).unwrap();
        if let InteropCommand::Open { target } = cmd {
            assert_eq!(target, "/video.mp4");
        } else {
            panic!("Expected Open command");
        }

        let open_alias_json = r#"{"command":"open_video","path":"/video2.mp4"}"#;
        let cmd: InteropCommand = serde_json::from_str(open_alias_json).unwrap();
        if let InteropCommand::Open { target } = cmd {
            assert_eq!(target, "/video2.mp4");
        } else {
            panic!("Expected Open command with aliases");
        }

        let vol_alias_json = r#"{"command":"volume","level":45.0}"#;
        let cmd: InteropCommand = serde_json::from_str(vol_alias_json).unwrap();
        if let InteropCommand::SetVolume { value } = cmd {
            assert_eq!(value, 45.0);
        } else {
            panic!("Expected SetVolume command with aliases");
        }
    }

    #[test]
    fn launch_request_is_bounded_and_additive() {
        let json = r#"{
            "command":"launch",
            "request":{
                "operation_id":"launch-test-1",
                "application_identity":"Pealayer",
                "sender_session_id":null,
                "sender_working_directory":"/sender/work",
                "target":"media/clip.mkv",
                "fullscreen":true,
                "volume":42.0,
                "activate":true,
                "future_optional_field":"ignored"
            },
            "future_envelope_field":true
        }"#;
        let (_, command) = parse_interop_request(json).unwrap();
        let InteropCommand::Launch { request } = command else {
            panic!("expected launch command");
        };
        assert_eq!(request.operation_id, "launch-test-1");
        assert_eq!(request.target.as_deref(), Some("media/clip.mkv"));
        assert_eq!(request.volume, Some(42.0));

        let invalid = r#"{
            "command":"launch",
            "request":{
                "operation_id":"",
                "application_identity":"Pealayer",
                "sender_session_id":null,
                "sender_working_directory":null,
                "target":null,
                "fullscreen":false,
                "volume":131.0,
                "activate":true
            }
        }"#;
        assert!(parse_interop_request(invalid).is_err());
    }

    #[test]
    fn launch_destination_rejects_other_identity_and_session() {
        let options = crate::cli::CliOptions {
            target: None,
            fullscreen: false,
            volume: None,
        };
        let request = crate::cli::launch_request(&options);
        assert!(validate_launch_destination(&request).is_ok());

        let mut other_identity = request.clone();
        other_identity.application_identity = "Different Player".to_string();
        assert!(validate_launch_destination(&other_identity).is_err());

        #[cfg(target_os = "windows")]
        {
            let mut other_session = request;
            other_session.sender_session_id = Some(
                crate::platform::windows::current_session_id()
                    .expect("current Windows session")
                    .wrapping_add(1),
            );
            assert!(validate_launch_destination(&other_session).is_err());
        }
    }

    #[test]
    fn launch_receipts_claim_once_and_recover_after_release() {
        let mut receipts = LaunchReceiptCache::default();
        assert!(receipts.claim("launch-one"));
        assert!(!receipts.claim("launch-one"));
        receipts.release("launch-one");
        assert!(receipts.claim("launch-one"));
    }

    #[test]
    fn test_status_response_serialization() {
        let resp = PlayerStatusResponse {
            status: "ok".to_string(),
            playing: true,
            volume: 80.0,
            playback_time: 15.0,
            duration: 120.0,
            current_video: Some("/path/file.mp4".to_string()),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"playing\":true"));
        assert!(json.contains("\"volume\":80.0"));
    }

    #[test]
    fn test_parse_dual_protocol_requests() {
        // Standard NDJSON format
        let raw = r#"{"command":"play"}"#;
        let (id, cmd) = parse_interop_request(raw).unwrap();
        assert!(id.is_none());
        assert!(matches!(cmd, InteropCommand::Play));

        // JSON-RPC 2.0 format with id and method
        let rpc = r#"{"jsonrpc":"2.0","id":42,"method":"seek","params":{"seconds":15.0}}"#;
        let (id, cmd) = parse_interop_request(rpc).unwrap();
        assert_eq!(id, Some(serde_json::json!(42)));
        if let InteropCommand::Seek { seconds } = cmd {
            assert_eq!(seconds, 15.0);
        } else {
            panic!("Expected Seek command");
        }

        // JSON-RPC 2.0 set_volume
        let vol_rpc = r#"{"jsonrpc":"2.0","id":"vol-1","method":"set_volume","params":{"value":75.0}}"#;
        let (id, cmd) = parse_interop_request(vol_rpc).unwrap();
        assert_eq!(id, Some(serde_json::json!("vol-1")));
        if let InteropCommand::SetVolume { value } = cmd {
            assert_eq!(value, 75.0);
        } else {
            panic!("Expected SetVolume command");
        }
    }

    #[test]
    fn parses_namespaced_json_rpc_commands() {
        let request: JsonRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":7,"method":"pealayer.seek","params":{"seconds":12.5}}"#,
        )
        .unwrap();
        assert!(matches!(
            command_from_json_rpc(&request).unwrap(),
            Some(InteropCommand::Seek { seconds: 12.5 })
        ));
        assert!(json_rpc_result(&request.id, serde_json::json!({"ok":true}))
            .contains("\"id\":7"));
    }

    #[test]
    fn test_live_status_snapshot() {
        let status = PlayerStatusResponse {
            status: "ok".to_string(),
            playing: true,
            volume: 92.0,
            playback_time: 45.5,
            duration: 120.0,
            current_video: Some("/path/sample.mkv".to_string()),
        };
        set_live_status(status.clone());
        let retrieved = get_live_status();
        assert_eq!(retrieved.playing, true);
        assert_eq!(retrieved.volume, 92.0);
        assert_eq!(retrieved.playback_time, 45.5);
        assert_eq!(retrieved.current_video, Some("/path/sample.mkv".to_string()));
    }

    #[test]
    fn maps_exact_target_pccontroller_action() {
        let event = serde_json::json!({
            "kind": "pealayer.seek",
            "metadata": {
                "target_instance": "pealayer:test",
                "operation_id": "operation-1",
                "operation_delivery_id": "delivery-1",
                "operation_expires_at": "2099-01-01T00:00:00Z",
                "value": "-12.5",
            },
        });
        let action = controller_action_from_event(&event, "pealayer:test").unwrap();
        assert!(matches!(
            action.command,
            Some(InteropCommand::Seek { seconds: -12.5 })
        ));
        assert_eq!(action.acknowledgement["state"], "applied");
        assert_eq!(action.acknowledgement["delivery_id"], "delivery-1");
        assert!(controller_action_from_event(&event, "pealayer:other").is_none());
    }

    #[test]
    fn expired_targeted_action_is_rejected() {
        let event = serde_json::json!({
            "kind": "pealayer.play",
            "metadata": {
                "target_instance": "pealayer:test",
                "operation_id": "operation-expired",
                "operation_delivery_id": "delivery-expired",
                "operation_expires_at": "2000-01-01T00:00:00Z",
            },
        });
        let action = controller_action_from_event(&event, "pealayer:test").unwrap();
        assert!(action.command.is_none());
        assert_eq!(action.acknowledgement["state"], "rejected");
        assert_eq!(action.acknowledgement["reason"], "expired");
    }
}
