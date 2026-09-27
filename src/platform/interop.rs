use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
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
        status: "ok".to_string(),
        playing: false,
        volume: 100.0,
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
                    let _ = tx.send(cmd);
                    egui_ctx.request_repaint();
                    let resp_val = serde_json::json!({"status": "ok"});
                    let resp = format_interop_response(id, &resp_val);
                    let _ = writer.write_all(resp.as_bytes());
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
    // 1. Cross-platform loopback TCP listener on 127.0.0.1:8082
    let tx_tcp = tx.clone();
    let ctx_tcp = egui_ctx.clone();
    thread::spawn(move || {
        if let Ok(listener) = TcpListener::bind("127.0.0.1:8082") {
            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    let tx_conn = tx_tcp.clone();
                    let ctx_conn = ctx_tcp.clone();
                    thread::spawn(move || {
                        if let Ok(read_clone) = stream.try_clone() {
                            let reader = BufReader::new(read_clone);
                            handle_client_connection(reader, stream, tx_conn, ctx_conn);
                        }
                    });
                }
            }
        } else {
            log::warn!("Could not bind loopback IPC TCP listener to 127.0.0.1:8082");
        }
    });

    // 2. Native Unix domain socket listener on Unix targets
    #[cfg(unix)]
    {
        let tx_unix = tx.clone();
        let ctx_unix = egui_ctx.clone();
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
                        thread::spawn(move || {
                            if let Ok(read_clone) = stream.try_clone() {
                                let reader = BufReader::new(read_clone);
                                handle_client_connection(reader, stream, tx_conn, ctx_conn);
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
}
