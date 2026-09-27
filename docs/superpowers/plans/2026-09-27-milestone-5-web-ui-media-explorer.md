# Milestone 5: Web UI Media Explorer & Issue #2 Completion Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide full media explorer support across the Web UI (local filesystem browsing, folder navigation, thumbnail retrieval, file launch, command aliases for `open_video`/`path` and `set_volume`/`level`), build the frontend distribution, and verify the complete Issue #2 checklist for closing.

**Architecture:** 
1. Enhance `InteropCommand` deserialization in `src/platform/interop.rs` with Serde field/variant aliases (`open_video` -> `Open`, `path` -> `target`, `level` -> `value`).
2. Add full Media Explorer capabilities (folder browsing, breadcrumbs, search, thumbnail previews, one-click play) to `src/server/web_assets.rs` embedded fallback so the zero-dependency embedded server matches the React SPA feature parity.
3. Verify `web_ui` production build and add an end-to-end integration test `tests/web_media_explorer_test.rs`.

**Tech Stack:** Rust 2024 edition, Serde, tiny_http, tungstenite, React/Vite.

**Spec:** GitHub Issue #2 ("Player UX feedback" - Milestone 5: Web UI Media Explorer)

## Global Constraints
- Preserve backward compatibility with existing CLI and NDJSON/JSON-RPC protocols.
- Ensure all tests pass on both Linux and Windows.
- Zero extra runtime dependencies.

---

### Task 1: InteropCommand Serde Aliases & Integration Test

**Files:**
- Modify: `src/platform/interop.rs`
- Create: `tests/web_media_explorer_test.rs`
- Test: `tests/web_media_explorer_test.rs`

**Interfaces:**
- Produces:
  ```rust
  #[serde(alias = "open_video")]
  Open {
      #[serde(alias = "path")]
      target: String,
  },
  #[serde(alias = "volume")]
  SetVolume {
      #[serde(alias = "level")]
      value: f64,
  },
  ```

- [ ] **Step 1: Write integration test for Web UI commands and browsing**

Create `tests/web_media_explorer_test.rs`:
```rust
use pealayer::platform::interop::InteropCommand;
use pealayer::server::spawn_web_server;
use std::time::Duration;

#[test]
fn test_web_command_aliases_and_browsing() {
    let ctx = eframe::egui::Context::default();
    let (_state_tx, cmd_rx) = spawn_web_server(18080, 18081, ctx);

    std::thread::sleep(Duration::from_millis(100));

    // 1. Test POST /api/player/command with open_video and path
    let client = tiny_http::Client::new(); // or standard reqwest/ureq/std::net::TcpStream
    let payload = r#"{"command":"open_video","path":"/tmp/test_clip.mp4"}"#;
    
    let mut stream = std::net::TcpStream::connect("127.0.0.1:18080").expect("Failed to connect to web server");
    let req = format!(
        "POST /api/player/command HTTP/1.1\r\nHost: 127.0.0.1:18080\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    );
    use std::io::{Read, Write};
    stream.write_all(req.as_bytes()).unwrap();
    let mut resp = String::new();
    stream.read_to_string(&mut resp).unwrap();
    assert!(resp.contains("200 OK"));

    let received = cmd_rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive command");
    if let InteropCommand::Open { target } = received {
        assert_eq!(target, "/tmp/test_clip.mp4");
    } else {
        panic!("Expected Open command, got {:?}", received);
    }

    // 2. Test set_volume with level
    let mut stream2 = std::net::TcpStream::connect("127.0.0.1:18080").expect("Failed to connect to web server");
    let payload2 = r#"{"command":"set_volume","level":75.0}"#;
    let req2 = format!(
        "POST /api/player/command HTTP/1.1\r\nHost: 127.0.0.1:18080\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload2.len(),
        payload2
    );
    stream2.write_all(req2.as_bytes()).unwrap();
    let mut resp2 = String::new();
    stream2.read_to_string(&mut resp2).unwrap();
    assert!(resp2.contains("200 OK"));

    let received2 = cmd_rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive command");
    if let InteropCommand::SetVolume { value } = received2 {
        assert_eq!(value, 75.0);
    } else {
        panic!("Expected SetVolume command, got {:?}", received2);
    }
}
```

- [ ] **Step 2: Run test to verify it fails without aliases**

Run: `cargo test --test web_media_explorer_test`
Expected: FAIL due to deserialization error on `open_video` and `level`

- [ ] **Step 3: Add aliases to InteropCommand in src/platform/interop.rs**

In `src/platform/interop.rs`:
```rust
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test web_media_explorer_test`
Expected: PASS

---

### Task 2: Embedded Fallback Web UI Media Explorer

**Files:**
- Modify: `src/server/web_assets.rs`
- Test: `tests/web_media_explorer_test.rs`

- [ ] **Step 1: Add media library browse test to tests/web_media_explorer_test.rs**

In `tests/web_media_explorer_test.rs`:
```rust
#[test]
fn test_web_fs_browse_endpoint() {
    let ctx = eframe::egui::Context::default();
    let (_state_tx, _cmd_rx) = spawn_web_server(18082, 18083, ctx);

    std::thread::sleep(Duration::from_millis(100));

    let mut stream = std::net::TcpStream::connect("127.0.0.1:18082").expect("Failed to connect to web server");
    let req = "GET /api/fs/browse HTTP/1.1\r\nHost: 127.0.0.1:18082\r\nConnection: close\r\n\r\n";
    use std::io::{Read, Write};
    stream.write_all(req.as_bytes()).unwrap();
    let mut resp = String::new();
    stream.read_to_string(&mut resp).unwrap();
    assert!(resp.contains("200 OK"));
    assert!(resp.contains("entries"));
}
```

- [ ] **Step 2: Update embedded INDEX_HTML in src/server/web_assets.rs**

Add a tab switcher in header (`Remote` / `Media Library`), directory browsing view with:
- Current path breadcrumbs and "Go to Parent" button
- Quick search / filter input
- Grid of file/folder items showing thumbnails via `/api/fs/thumbnail?path=...`
- Clicking directory browses it (`fetchDirectory(path)`)
- Clicking media file triggers `sendCmd('open', { target: item.path })` and switches to remote with status toast notification.

- [ ] **Step 3: Run tests and rebuild web_ui dist**

Run: `npm run build` in `web_ui`
Run: `cargo test`
Expected: PASS

---

### Task 3: Final Verification & Issue #2 Audit

**Files:**
- Audit: All Issue #2 requirements against implementation.
- Run: Full test suite on both targets.

- [ ] **Step 1: Run full cargo test suite**

Run: `cargo test`
Expected: All tests pass (0 failures)

- [ ] **Step 2: Run Windows cross-check**

Run: `cargo check --target x86_64-pc-windows-gnu`
Expected: 0 errors, 0 warnings

- [ ] **Step 3: Verify all 5 Milestones from Issue #2**

1. Milestone 1: Windows Shell Taskbar Integration (Progress, Jump List, Media Controls) -> VERIFIED
2. Milestone 2: Unified Cross-Platform IPC (Dual-protocol NDJSON/JSON-RPC, TCP + Unix socket) -> VERIFIED
3. Milestone 3: CLI Mode & Launch Arguments (`--fullscreen`, `--volume`, `--remote`, target forwarding) -> VERIFIED
4. Milestone 4: Windows Aesthetics & File Associations (DWM Dark Mode/Mica, `--register-associations`) -> VERIFIED
5. Milestone 5: Web UI Media Explorer (Browsing, Thumbnails, Command Aliases) -> VERIFIED
