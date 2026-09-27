use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(unix)]
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum InteropCommand {
    Play,
    Pause,
    TogglePause,
    Seek { seconds: f64 },
    SeekAbs { percentage: f64 },
    SetVolume { value: f64 },
    Open { target: String },
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
    let number = |name: &str| {
        request
            .params
            .get(name)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("missing numeric parameter: {name}"))
    };
    let string = |name: &str| {
        request
            .params
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("missing string parameter: {name}"))
    };
    match request.method.as_str() {
        "pealayer.play" | "pealayer.player.play" => Ok(Some(InteropCommand::Play)),
        "pealayer.pause" | "pealayer.player.pause" => Ok(Some(InteropCommand::Pause)),
        "pealayer.toggle" | "pealayer.player.toggle" => Ok(Some(InteropCommand::TogglePause)),
        "pealayer.seek" | "pealayer.player.seek" => Ok(Some(InteropCommand::Seek {
            seconds: number("seconds")?,
        })),
        "pealayer.seek_absolute" | "pealayer.player.seek_absolute" => {
            Ok(Some(InteropCommand::SeekAbs {
                percentage: number("percentage")?,
            }))
        }
        "pealayer.volume.set" | "pealayer.player.volume.set" => {
            Ok(Some(InteropCommand::SetVolume {
                value: number("value")?,
            }))
        }
        "pealayer.open" | "pealayer.player.open" => Ok(Some(InteropCommand::Open {
            target: string("target")?,
        })),
        "pealayer.status" | "pealayer.player.status" => Ok(None),
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

pub fn get_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("pealayer.sock")
    } else {
        PathBuf::from("/tmp").join("pealayer.sock")
    }
}

#[cfg(unix)]
pub fn spawn_interop_server(egui_ctx: eframe::egui::Context) -> Receiver<InteropCommand> {
    let (tx, rx) = channel::<InteropCommand>();

    thread::spawn(move || {
        let socket_path = get_socket_path();
        if socket_path.exists() {
            let _ = std::fs::remove_file(&socket_path);
        }

        if let Ok(listener) = UnixListener::bind(&socket_path) {
            for stream in listener.incoming() {
                if let Ok(mut stream) = stream {
                    let tx_clone = tx.clone();
                    let egui_ctx_conn = egui_ctx.clone();
                    thread::spawn(move || {
                        let mut reader = BufReader::new(stream.try_clone().unwrap());
                        let mut line = String::new();
                        if reader.read_line(&mut line).is_ok() {
                            if let Ok(cmd) = serde_json::from_str::<InteropCommand>(line.trim()) {
                                let _ = tx_clone.send(cmd);
                                egui_ctx_conn.request_repaint();
                                let _ = stream.write_all(b"{\"status\":\"ok\"}\n");
                            } else {
                                let _ = stream.write_all(b"{\"status\":\"error\",\"message\":\"Invalid JSON command\"}\n");
                            }
                        }
                    });
                }
            }
        }
    });

    rx
}

#[cfg(not(unix))]
pub fn spawn_interop_server(egui_ctx: eframe::egui::Context) -> Receiver<InteropCommand> {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    let (tx, rx) = channel::<InteropCommand>();
    thread::spawn(move || {
        let Ok(listener) = TcpListener::bind("127.0.0.1:8082") else {
            return;
        };
        for mut stream in listener.incoming().flatten() {
            let Ok(clone) = stream.try_clone() else {
                continue;
            };
            let tx = tx.clone();
            let egui_ctx = egui_ctx.clone();
            thread::spawn(move || {
                let mut reader = BufReader::new(clone);
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() {
                    return;
                }
                let response = if let Ok(command) = serde_json::from_str::<InteropCommand>(line.trim()) {
                    let _ = tx.send(command);
                    egui_ctx.request_repaint();
                    serde_json::json!({"status":"ok"}).to_string()
                } else if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(line.trim()) {
                    match command_from_json_rpc(&request) {
                        Ok(Some(command)) => {
                            let _ = tx.send(command);
                            egui_ctx.request_repaint();
                            json_rpc_result(&request.id, serde_json::json!({"accepted":true}))
                        }
                        Ok(None) => json_rpc_result(
                            &request.id,
                            serde_json::json!({"status":"available_via_http_or_websocket"}),
                        ),
                        Err(error) => json_rpc_error(&request.id, -32601, &error),
                    }
                } else {
                    serde_json::json!({"status":"error","message":"invalid JSON command"})
                        .to_string()
                };
                let _ = writeln!(stream, "{response}");
            });
        }
    });
    rx
}

const PCCONTROLLER_WEBSOCKET_ENDPOINT: &str = "ws://127.0.0.1:8787/ipc";
const PCCONTROLLER_ACTIONS: &str = "app.page,pealayer.play,pealayer.pause,pealayer.toggle,pealayer.seek,pealayer.seek_absolute,pealayer.volume.set,pealayer.open";

struct ControllerAction {
    command: Option<InteropCommand>,
    acknowledgement: Value,
    receipt_key: String,
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
    let deadline = expires_at.parse::<jiff::Timestamp>().ok()?;
    if deadline <= jiff::Timestamp::now() {
        return None;
    }

    let kind = event.get("kind")?.as_str()?.trim().to_ascii_lowercase();
    let value = metadata
        .get(if kind == "app.page" { "page" } else { "value" })
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let command = match kind.as_str() {
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
            "back" | "previous" | "rewind" => Some(InteropCommand::Seek { seconds: -10.0 }),
            "forward" | "next" => Some(InteropCommand::Seek { seconds: 10.0 }),
            _ => None,
        },
        _ => None,
    };
    let (state, reason) = if command.is_some() {
        ("applied", None)
    } else {
        ("rejected", Some("unsupported_or_invalid_pealayer_action"))
    };
    let mut acknowledgement = serde_json::json!({
        "operation_id": operation_id,
        "delivery_id": delivery_id,
        "instance_id": instance_id,
        "state": state,
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
                        "rpc": "http://127.0.0.1:8080/api/rpc",
                        "websocket": "ws://127.0.0.1:8081",
                        "ipc": "tcp://127.0.0.1:8082",
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

fn run_pccontroller_action_bridge(
    tx: &std::sync::mpsc::Sender<InteropCommand>,
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

    let mut last_report = Instant::now();
    let mut receipts = HashSet::new();
    let mut receipt_order = VecDeque::new();
    loop {
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
                if message.get("method").and_then(Value::as_str) != Some("controller.state") {
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
                        tx.send(command)
                            .map_err(|error| format!("deliver PCController action to player: {error}"))?;
                        egui_ctx.request_repaint();
                    }
                }
                socket
                    .send(controller_rpc(
                        next_id,
                        "controller.app.action.ack",
                        action.acknowledgement,
                    ))
                    .map_err(|error| format!("acknowledge PCController action: {error}"))?;
                next_id = next_id.wrapping_add(1).max(1);
            }
            Ok(Message::Ping(payload)) => {
                socket
                    .send(Message::Pong(payload))
                    .map_err(|error| format!("answer PCController ping: {error}"))?;
            }
            Ok(Message::Close(_)) => return Err("PCController action WebSocket closed".to_string()),
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(error) => return Err(format!("read PCController action WebSocket: {error}")),
        }
    }
}

pub fn spawn_pccontroller_action_bridge(
    egui_ctx: eframe::egui::Context,
) -> Receiver<InteropCommand> {
    let (tx, rx) = channel();
    thread::spawn(move || loop {
        if let Err(error) = run_pccontroller_action_bridge(&tx, &egui_ctx) {
            eprintln!("[PCController] {error}; retrying in 2 seconds");
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
    fn test_json_rpc_command_mapping() {
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
    fn maps_exact_target_pccontroller_action_and_acknowledges_delivery() {
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
}
