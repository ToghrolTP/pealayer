# Milestone 2: Unified Cross-Platform IPC Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a unified cross-platform IPC subsystem on `127.0.0.1:8082` supporting both raw NDJSON and JSON-RPC 2.0 command envelopes with live status querying on Windows and Linux.

**Architecture:** A lightweight TCP loopback server listening on `127.0.0.1:8082` (concurrent with the Unix domain socket on Unix platforms). Client lines are parsed as either standard NDJSON or JSON-RPC 2.0 requests, mapped to `InteropCommand`, dispatched into the player's event channel, and acknowledged with formatted JSON/JSON-RPC responses utilizing an in-memory live status snapshot.

**Tech Stack:** Rust 2024 edition, `std::net::TcpListener`, `serde`, `serde_json`, `eframe::egui`.

**Spec:** [Issue #2](https://github.com/ToghrolTP/pealayer/issues/2) and Milestone 2 design approved in chat.

## Global Constraints
- Must compile cleanly on both Linux (`x86_64-unknown-linux-gnu`) and Windows (`x86_64-pc-windows-msvc`).
- Loopback listener must bind strictly to `127.0.0.1:8082` to avoid external network exposure.
- All errors (port bind failure, malformed JSON, client disconnect) must be handled gracefully without crashing the UI.
- Preserve backward compatibility for existing Unix socket clients and channel types (`Receiver<InteropCommand>`).

---

### Task 1: Dual-Protocol Command Parser (NDJSON & JSON-RPC 2.0)

**Files:**
- Modify: `src/platform/interop.rs`
- Test: `src/platform/interop.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn parse_interop_request(line: &str) -> Result<(Option<serde_json::Value>, InteropCommand), String>;
  pub fn format_interop_response(id: Option<serde_json::Value>, result: &serde_json::Value) -> String;
  pub fn format_interop_error(id: Option<serde_json::Value>, code: i32, message: &str) -> String;
  ```

- [ ] **Step 1: Write failing unit test for dual-protocol parsing**

Add `test_parse_dual_protocol_requests` in `src/platform/interop.rs`:
```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_parse_dual_protocol_requests`
Expected: FAIL with "cannot find function `parse_interop_request` in this scope"

- [ ] **Step 3: Implement minimal parser and response formatters**

In `src/platform/interop.rs`:
```rust
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
            "set_volume" => {
                let value = params.and_then(|p| p.get("value")).and_then(|v| v.as_f64()).unwrap_or(100.0);
                InteropCommand::SetVolume { value }
            }
            "open" => {
                let target = params.and_then(|p| p.get("target")).and_then(|t| t.as_str()).unwrap_or("").to_string();
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_parse_dual_protocol_requests`
Expected: PASS

---

### Task 2: Live Player Status Snapshot Synchronization

**Files:**
- Modify: `src/platform/interop.rs`
- Modify: `src/app.rs`
- Test: `src/platform/interop.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn set_live_status(status: PlayerStatusResponse);
  pub fn get_live_status() -> PlayerStatusResponse;
  ```

- [ ] **Step 1: Write failing unit test for status snapshot**

In `src/platform/interop.rs`:
```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_live_status_snapshot`
Expected: FAIL with "cannot find function `set_live_status` in this scope"

- [ ] **Step 3: Implement thread-safe status storage and hook into app.rs**

In `src/platform/interop.rs`:
```rust
use std::sync::RwLock;

static LIVE_STATUS: RwLock<Option<PlayerStatusResponse>> = RwLock::new(None);

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
```

In `src/app.rs` inside broadcast block:
```rust
crate::platform::interop::set_live_status(status_resp.clone());
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_live_status_snapshot`
Expected: PASS

---

### Task 3: Cross-Platform Loopback TCP Listener & Unix Socket Integration

**Files:**
- Modify: `src/platform/interop.rs`
- Test: `tests/interop_tcp_test.rs`

**Interfaces:**
- Consumes: `parse_interop_request`, `format_interop_response`, `format_interop_error`, `get_live_status`
- Produces: `spawn_interop_listener(tx: Sender<InteropCommand>, egui_ctx: Context)`

- [ ] **Step 1: Write integration test for loopback TCP connection**

Create `tests/interop_tcp_test.rs`:
```rust
use pealayer::platform::interop::{InteropCommand, spawn_interop_listener, set_live_status, PlayerStatusResponse};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc::channel;

#[test]
fn test_loopback_tcp_interop() {
    let (tx, rx) = channel::<InteropCommand>();
    let ctx = eframe::egui::Context::default();
    spawn_interop_listener(tx, ctx);

    // Allow background thread to bind
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Connect to 127.0.0.1:8082
    let mut stream = TcpStream::connect("127.0.0.1:8082").expect("Failed to connect to loopback IPC");
    stream.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"play\"}\n").unwrap();
    stream.flush().unwrap();

    let cmd = rx.recv_timeout(std::time::Duration::from_secs(1)).expect("Did not receive Play command");
    assert!(matches!(cmd, InteropCommand::Play));

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.contains("\"result\":{\"status\":\"ok\"}"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test interop_tcp_test`
Expected: FAIL connection refused

- [ ] **Step 3: Implement connection handler and TCP listener**

In `src/platform/interop.rs`:
```rust
use std::net::TcpListener;

fn handle_client_stream<S: std::io::Read + std::io::Write>(
    mut stream: S,
    tx: std::sync::mpsc::Sender<InteropCommand>,
    egui_ctx: eframe::egui::Context,
) {
    let mut reader = BufReader::new(match stream.try_clone() {
        // ... handled per stream type or via split
    });
    // Parse line, match GetStatus -> return get_live_status(), else send to tx and return ok
}

pub fn spawn_interop_listener(tx: std::sync::mpsc::Sender<InteropCommand>, egui_ctx: eframe::egui::Context) {
    // 1. Loopback TCP on 127.0.0.1:8082 for all platforms
    let tx_tcp = tx.clone();
    let ctx_tcp = egui_ctx.clone();
    thread::spawn(move || {
        if let Ok(listener) = TcpListener::bind("127.0.0.1:8082") {
            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    let tx_conn = tx_tcp.clone();
                    let ctx_conn = ctx_tcp.clone();
                    thread::spawn(move || {
                        handle_tcp_stream(stream, tx_conn, ctx_conn);
                    });
                }
            }
        } else {
            log::warn!("Could not bind IPC TCP listener to 127.0.0.1:8082");
        }
    });

    // 2. Unix socket listener for Unix platforms
    #[cfg(unix)]
    {
        // Bind pealayer.sock and forward to handle_unix_stream
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test interop_tcp_test`
Expected: PASS

- [ ] **Step 5: Run full project test suite**

Run: `cargo test`
Expected: All unit tests and integration tests PASS.
