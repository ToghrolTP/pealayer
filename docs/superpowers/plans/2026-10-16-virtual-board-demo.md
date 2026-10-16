# Virtual Board Demo Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide an integrated, 1-click "Virtual Board (Demo Simulation)" in Pealayer that manages the C++ ATmega328P virtual board emulator and in-process PCController coordinator, pre-loading a synchronized showcase project so users can explore all cinema 4D features without physical hardware.

**Architecture:** A supervisor (`VirtualBoardSession`) resolves the native `virtual_board` binary and launches it on an ephemeral loopback port in RAM-only EEPROM mode (`--memory`). A demo coordinator session initializes `EmbeddedHost` (PCController C-ABI library) in an isolated temporary data root, connects it to the virtual board, and configures the `cinema-seat-motion` profile. Pealayer's engine attaches to this client when `demo://virtual-board` is selected, pre-populating `test-data/jellyfish.mp4` and a rich 4D timeline.

**Tech Stack:** Rust 2024, `std::process`, `eframe` / `egui`, `libloading`, PCController C-ABI (`EmbeddedHost`), C++17 `virtual_board`.

**Spec:** [docs/superpowers/specs/2026-10-16-virtual-board-demo-design.md](file:///home/toghrol/Documents/rust/pealayer/docs/superpowers/specs/2026-10-16-virtual-board-demo-design.md)

## Global Constraints
- **Alpha Contract**: No mock board implementations in Pealayer. Hardware capabilities, seat profiles, and relay/PWM commands must flow through the authoritative PCController coordinator and shared firmware protocol.
- **Isolation**: Use `--memory` for the virtual MCU to ensure RAM-only EEPROM. Embedded coordinator data root must use an isolated temporary path (`pealayer-demo-data-<pid>`) and never touch user configs or production registries.
- **Clean Teardown**: RAII guards and drop implementations must kill and wait on child processes to prevent orphaned background processes.
- **Error Transparency**: Missing binaries or failed connections must emit typed, user-facing warning toasts via `crate::messaging::publish` rather than silently publishing empty hardware states or crashing.

---

### Task 1: Virtual Board Process Supervisor (`VirtualBoardSession`)

**Files:**
- Create: `src/four_d/virtual_board.rs`
- Modify: `src/four_d/mod.rs`
- Test: `tests/virtual_board_session_test.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct VirtualBoardSession {
      child: std::process::Child,
      pub port: u16,
  }
  impl VirtualBoardSession {
      pub fn resolve_binary_path() -> Option<std::path::PathBuf>;
      pub fn spawn() -> Result<Self, String>;
  }
  impl Drop for VirtualBoardSession {
      fn drop(&mut self);
  }
  ```

- [x] **Step 1: Write the failing test**

Create `tests/virtual_board_session_test.rs`:
```rust
use pealayer::four_d::virtual_board::VirtualBoardSession;
use std::net::TcpStream;

#[test]
fn test_virtual_board_binary_resolution_and_session_spawn() {
    let binary = VirtualBoardSession::resolve_binary_path();
    if binary.is_none() {
        eprintln!("Skipping test: virtual_board binary not found in dev or env path");
        return;
    }
    let session = VirtualBoardSession::spawn().expect("spawn virtual board session");
    assert!(session.port > 0, "session must bind to a positive loopback port");

    // Verify TCP socket accepts connections
    let stream = TcpStream::connect(("127.0.0.1", session.port));
    assert!(stream.is_ok(), "TCP connection to virtual board must succeed");
    drop(stream);

    drop(session);
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test --test virtual_board_session_test`
Expected: FAIL with "unresolved import `pealayer::four_d::virtual_board`"

- [x] **Step 3: Implement `VirtualBoardSession`**

Create `src/four_d/virtual_board.rs`:
```rust
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub struct VirtualBoardSession {
    child: Child,
    pub port: u16,
}

impl VirtualBoardSession {
    pub fn resolve_binary_path() -> Option<PathBuf> {
        if let Some(path) = std::env::var_os("PEALAYER_VIRTUAL_BOARD_BINARY") {
            let pb = PathBuf::from(path);
            if pb.is_file() {
                return Some(pb);
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let candidates = [
                    dir.join("virtual_board"),
                    dir.join("virtual_board.exe"),
                    dir.join("bin").join("virtual_board"),
                    dir.join("bin").join("virtual_board.exe"),
                ];
                for candidate in candidates {
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
        let dev_candidates = [
            PathBuf::from("scratch/PCController/Tools/VirtualBoard/.build/release/bin/virtual_board"),
            PathBuf::from("scratch/PCController/Tools/VirtualBoard/.build/release/bin/virtual_board.exe"),
            PathBuf::from("scratch/PCController/Tools/VirtualBoard/.build/bin/virtual_board"),
        ];
        for candidate in dev_candidates {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }

    pub fn spawn() -> Result<Self, String> {
        let binary = Self::resolve_binary_path().ok_or_else(|| {
            "virtual_board binary could not be found. Check bin/virtual_board or set PEALAYER_VIRTUAL_BOARD_BINARY.".to_string()
        })?;

        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|e| format!("reserve ephemeral loopback port for virtual board: {e}"))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("resolve reserved loopback port: {e}"))?
            .port();
        drop(listener);

        let child = Command::new(&binary)
            .args([
                "--bind",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--no-stdin",
                "--quiet",
                "--memory",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("spawn virtual_board child process ({}): {e}", binary.display()))?;

        let mut session = Self { child, port };
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut ready = false;
        while Instant::now() < deadline {
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                ready = true;
                break;
            }
            if let Ok(Some(status)) = session.child.try_wait() {
                return Err(format!("virtual_board exited prematurely with status: {status}"));
            }
            thread::sleep(Duration::from_millis(20));
        }

        if !ready {
            let _ = session.child.kill();
            let _ = session.child.wait();
            return Err("virtual_board did not start listening within 3 seconds".to_string());
        }

        Ok(session)
    }
}

impl Drop for VirtualBoardSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
```

Register module in `src/four_d/mod.rs`:
```rust
pub mod virtual_board;
```

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test --test virtual_board_session_test`
Expected: PASS

- [x] **Step 5: Commit**

```bash
git add src/four_d/virtual_board.rs src/four_d/mod.rs tests/virtual_board_session_test.rs
git commit -m "feat(four_d): add VirtualBoardSession supervisor for demo hardware"
```

---

### Task 2: Showcase Media & 4D Demo Timeline Seeder (`src/four_d/demo_timeline.rs`)

**Files:**
- Create: `src/four_d/demo_timeline.rs`
- Modify: `src/four_d/mod.rs`
- Test: `tests/demo_timeline_test.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn build_demo_showcase_timeline() -> crate::four_d::models::Timeline;
  pub fn demo_media_target() -> Option<std::path::PathBuf>;
  ```

- [x] **Step 1: Write the failing test**

Create `tests/demo_timeline_test.rs`:
```rust
use pealayer::four_d::demo_timeline::{build_demo_showcase_timeline, demo_media_target};

#[test]
fn test_demo_showcase_timeline_structure() {
    let timeline = build_demo_showcase_timeline();
    assert!(!timeline.instances.is_empty(), "timeline must contain effect instances");
    assert!(!timeline.templates.is_empty(), "timeline must contain template definitions");
    assert!(!timeline.analog_tracks.is_empty(), "timeline must contain analog fan curves");

    // Check that seat motion cues exist
    let has_seat_motion = timeline.instances.iter().any(|inst| {
        inst.effect_id.contains("seat") || inst.effect_id.contains("motion")
    });
    assert!(has_seat_motion, "demo timeline must include cinema seat motion effects");

    let media = demo_media_target();
    assert!(media.is_some(), "demo media target must be provided");
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test --test demo_timeline_test`
Expected: FAIL with "unresolved import `pealayer::four_d::demo_timeline`"

- [x] **Step 3: Implement `demo_timeline.rs`**

Create `src/four_d/demo_timeline.rs`:
```rust
use std::path::PathBuf;
use crate::four_d::curve::{AnalogKeyframe, AnalogTrack};
use crate::four_d::models::{Effect, EffectInstance, Timeline, TimelineKeyframe};

pub fn demo_media_target() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("test-data/jellyfish.mp4"),
        PathBuf::from("assets/jellyfish.mp4"),
    ];
    for candidate in candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn build_demo_showcase_timeline() -> Timeline {
    let mut timeline = Timeline::default();

    // 1. Templates
    timeline.templates.push(Effect {
        id: "seat.pitch_forward".to_string(),
        name: "Seat Pitch Forward".to_string(),
        track: "seat.a".to_string(),
        action: "seat.pitch".to_string(),
        value: 75,
        default_duration: 1200,
        color: "#3498db".to_string(),
    });
    timeline.templates.push(Effect {
        id: "seat.heave_bump".to_string(),
        name: "Seat Heave Bump".to_string(),
        track: "seat.a".to_string(),
        action: "seat.heave".to_string(),
        value: 100,
        default_duration: 800,
        color: "#e74c3c".to_string(),
    });
    timeline.templates.push(Effect {
        id: "wind.strobe_blast".to_string(),
        name: "Wind Strobe Blast".to_string(),
        track: "relays".to_string(),
        action: "relay.1".to_string(),
        value: 1,
        default_duration: 1500,
        color: "#f1c40f".to_string(),
    });

    // 2. Placed Cue Instances
    timeline.instances.push(EffectInstance {
        id: uuid::Uuid::new_v4().to_string(),
        effect_id: "seat.pitch_forward".to_string(),
        track: "seat.a".to_string(),
        start_time: 1000,
        duration: 1200,
        intensity: 0.85,
    });
    timeline.instances.push(EffectInstance {
        id: uuid::Uuid::new_v4().to_string(),
        effect_id: "wind.strobe_blast".to_string(),
        track: "relays".to_string(),
        start_time: 2500,
        duration: 1500,
        intensity: 1.0,
    });
    timeline.instances.push(EffectInstance {
        id: uuid::Uuid::new_v4().to_string(),
        effect_id: "seat.heave_bump".to_string(),
        track: "seat.a".to_string(),
        start_time: 4200,
        duration: 800,
        intensity: 0.95,
    });

    // 3. Analog Fan Curve Track (PWM Channel 0)
    timeline.analog_tracks.push(AnalogTrack {
        channel: 0,
        name: "Wind Speed (Fan PWM)".to_string(),
        keyframes: vec![
            AnalogKeyframe { timestamp_ms: 0, value: 0.0 },
            AnalogKeyframe { timestamp_ms: 1500, value: 0.4 },
            AnalogKeyframe { timestamp_ms: 2500, value: 0.9 },
            AnalogKeyframe { timestamp_ms: 4500, value: 0.2 },
            AnalogKeyframe { timestamp_ms: 6000, value: 0.0 },
        ],
    });

    // 4. Timeline Keyframe Guides
    timeline.keyframes.push(TimelineKeyframe {
        timestamp_ms: 1000,
        label: "Submerge Descent".to_string(),
    });
    timeline.keyframes.push(TimelineKeyframe {
        timestamp_ms: 2500,
        label: "Current Surge".to_string(),
    });
    timeline.keyframes.push(TimelineKeyframe {
        timestamp_ms: 4200,
        label: "Seabed Impact".to_string(),
    });

    timeline
}
```

Register module in `src/four_d/mod.rs`:
```rust
pub mod demo_timeline;
```

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test --test demo_timeline_test`
Expected: PASS

- [x] **Step 5: Commit**

```bash
git add src/four_d/demo_timeline.rs src/four_d/mod.rs tests/demo_timeline_test.rs
git commit -m "feat(four_d): add demo showcase timeline and media resolver"
```

---

### Task 3: Demo Coordinator Session (`DemoCoordinatorSession`)

**Files:**
- Create: `src/four_d/demo_coordinator.rs`
- Modify: `src/four_d/mod.rs`
- Test: `tests/demo_coordinator_test.rs`

**Interfaces:**
- Consumes: `VirtualBoardSession` (from Task 1)
- Produces:
  ```rust
  pub struct DemoCoordinatorSession {
      _virtual_board: VirtualBoardSession,
      pub client: crate::four_d::controller::ControllerClient,
      temp_root: std::path::PathBuf,
  }
  impl DemoCoordinatorSession {
      pub fn start() -> Result<Self, String>;
  }
  impl Drop for DemoCoordinatorSession {
      fn drop(&mut self);
  }
  ```

- [x] **Step 1: Write the failing test**

Create `tests/demo_coordinator_test.rs`:
```rust
use pealayer::four_d::demo_coordinator::DemoCoordinatorSession;

#[test]
fn test_demo_coordinator_session_connects_and_provisions_profile() {
    if std::env::var_os("PEALAYER_PCCONTROLLER_LIBRARY").is_none() &&
       pealayer::four_d::embedded_host::library_candidates().is_empty() {
        eprintln!("Skipping test: pccontroller library candidate not available");
        return;
    }
    if pealayer::four_d::virtual_board::VirtualBoardSession::resolve_binary_path().is_none() {
        eprintln!("Skipping test: virtual_board binary not found");
        return;
    }

    let mut session = DemoCoordinatorSession::start().expect("start demo coordinator session");
    let capabilities = session.client.hardware_capabilities().expect("retrieve hardware capabilities");
    assert!(capabilities.board_connected, "virtual board must be connected");
    assert!(capabilities.controls.iter().any(|c| c.key == "seat.a"), "cinema seat motion profile must be provisioned");
    drop(session);
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test --test demo_coordinator_test`
Expected: FAIL with "unresolved import `pealayer::four_d::demo_coordinator`"

- [x] **Step 3: Implement `demo_coordinator.rs`**

Create `src/four_d/demo_coordinator.rs`:
```rust
use std::path::PathBuf;
use serde_json::json;
use crate::four_d::controller::ControllerClient;
use crate::four_d::embedded_host::{EmbeddedHost, EmbeddedHostOptions};
use crate::four_d::virtual_board::VirtualBoardSession;

pub struct DemoCoordinatorSession {
    _virtual_board: VirtualBoardSession,
    pub client: ControllerClient,
    temp_root: PathBuf,
}

impl DemoCoordinatorSession {
    pub fn start() -> Result<Self, String> {
        let vb = VirtualBoardSession::spawn()?;

        let run_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("pealayer-demo-coordinator-{}-{run_id}", std::process::id()));
        std::fs::create_dir_all(&temp_root).map_err(|e| format!("create demo coordinator temp root: {e}"))?;

        let options = EmbeddedHostOptions {
            data_root: temp_root.clone(),
            app_id: "pealayer.demo".into(),
            app_name: "Pealayer Demo".into(),
            disable_auto_connect: true,
            disable_native: true,
            enable_integrations: false,
            http_address: Some("127.0.0.1:0".into()),
        };

        let candidates = crate::four_d::embedded_host::library_candidates();
        let mut host = None;
        let mut last_error = String::new();
        for path in candidates {
            match EmbeddedHost::load_and_start(&path, &options) {
                Ok(loaded) => {
                    host = Some(loaded);
                    break;
                }
                Err(err) => {
                    last_error = err;
                }
            }
        }

        let mut host = host.ok_or_else(|| {
            format!("could not load PCController library for demo mode: {last_error}")
        })?;

        let address = host
            .endpoints()
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|ep| ep.get("address"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| "coordinator exposed no valid listening address".to_string())?
            .to_owned();

        let mut client = ControllerClient::connect(&address)
            .map_err(|e| format!("connect to demo coordinator at {address}: {e}"))?;

        let connected = client.call("controller.connect", json!({"port": format!("tcp://127.0.0.1:{}", vb.port)}))
            .map_err(|e| format!("attach coordinator to virtual board: {e}"))?;
        if connected.get("connected").and_then(|v| v.as_bool()) != Some(true) {
            return Err("coordinator reported failed virtual board attachment".to_string());
        }

        let profile = client.call("controller.board_profile.get", json!({}))
            .unwrap_or_else(|_| json!({"revision": 0}));
        let revision = profile.get("revision").cloned().unwrap_or(json!(0));
        let _ = client.call("controller.board_profile.update", json!({
            "key": "demo",
            "mode": "cinema-seat-motion",
            "expected_revision": revision,
        }));

        Ok(Self {
            _virtual_board: vb,
            client,
            temp_root,
        })
    }
}

impl Drop for DemoCoordinatorSession {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.temp_root);
    }
}
```

Register module in `src/four_d/mod.rs`:
```rust
pub mod demo_coordinator;
```

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test --test demo_coordinator_test`
Expected: PASS (or graceful skip if library missing)

- [x] **Step 5: Commit**

```bash
git add src/four_d/demo_coordinator.rs src/four_d/mod.rs tests/demo_coordinator_test.rs
git commit -m "feat(four_d): add DemoCoordinatorSession for virtual board and profile provisioning"
```

---

### Task 4: Engine Integration & Demo Endpoint Support

**Files:**
- Modify: `src/four_d/controller.rs`
- Modify: `src/four_d/engine.rs`
- Test: `tests/demo_endpoint_test.rs`

**Interfaces:**
- Consumes: `DemoCoordinatorSession` (from Task 3)
- Produces: Constant `DEMO_ENDPOINT: &str = "demo://virtual-board";` recognized by `is_controller_endpoint` and `available_endpoints`.

- [x] **Step 1: Write the failing test**

Create `tests/demo_endpoint_test.rs`:
```rust
use pealayer::four_d::controller::{is_controller_endpoint, is_demo_endpoint, available_endpoints, DEMO_ENDPOINT};

#[test]
fn test_demo_endpoint_recognition() {
    assert_eq!(DEMO_ENDPOINT, "demo://virtual-board");
    assert!(is_demo_endpoint(DEMO_ENDPOINT));
    assert!(is_controller_endpoint(DEMO_ENDPOINT));

    let endpoints = available_endpoints();
    assert!(endpoints.iter().any(|ep| ep == DEMO_ENDPOINT), "available_endpoints must include DEMO_ENDPOINT");
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test --test demo_endpoint_test`
Expected: FAIL with "cannot find function `is_demo_endpoint` in `pealayer::four_d::controller`"

- [x] **Step 3: Update `controller.rs` and `engine.rs`**

In `src/four_d/controller.rs`:
Add:
```rust
pub const DEMO_ENDPOINT: &str = "demo://virtual-board";

pub fn is_demo_endpoint(endpoint: &str) -> bool {
    endpoint.trim() == DEMO_ENDPOINT
}
```

Update `is_controller_endpoint(endpoint: &str)` in `src/four_d/controller.rs`:
```rust
pub fn is_controller_endpoint(endpoint: &str) -> bool {
    let endpoint = endpoint.trim();
    endpoint == DEMO_ENDPOINT
        || endpoint.starts_with("pccontroller://")
        || endpoint.starts_with("udp://")
        || endpoint.starts_with("tcp://")
        || (!endpoint.starts_with("direct:")
            && !endpoint.contains('/')
            && !endpoint.contains('\\')
            && normalize_endpoint(endpoint).is_ok())
}
```

Update `available_endpoints()` in `src/four_d/controller.rs`:
```rust
pub fn available_endpoints() -> Vec<String> {
    let mut endpoints = vec![DEMO_ENDPOINT.to_string(), DEFAULT_ENDPOINT.to_string()];
    if let Ok(ports) = serialport::available_ports() {
        let mut direct = ports
            .into_iter()
            .map(|port| format!("direct:{}", port.port_name))
            .collect::<Vec<_>>();
        direct.sort_by_key(|name| name.to_ascii_lowercase());
        direct.dedup();
        endpoints.extend(direct);
    }
    endpoints
}
```

In `src/four_d/engine.rs`:
Update `HardwareTransport` and connect loop to handle `is_demo_endpoint`:
```rust
let transport = if crate::four_d::controller::is_demo_endpoint(&endpoint) {
    match crate::four_d::demo_coordinator::DemoCoordinatorSession::start() {
        Ok(session) => Ok(HardwareTransport::Controller(session.client)),
        Err(err) => {
            let _ = crate::messaging::publish(crate::messaging::ToastRequest {
                id: Some("demo.error".into()),
                title: "Virtual Board Demo Failed".into(),
                message: err.clone(),
                severity: crate::messaging::Severity::Warning,
                timeout_ms: 8000,
            }, "hardware");
            Err(err)
        }
    }
} else if crate::four_d::controller::is_controller_endpoint(&endpoint) {
    crate::four_d::controller::ControllerClient::connect_preferred(&endpoint)
        .map(HardwareTransport::Controller)
} else { ... }
```

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test --test demo_endpoint_test`
Expected: PASS

- [x] **Step 5: Commit**

```bash
git add src/four_d/controller.rs src/four_d/engine.rs tests/demo_endpoint_test.rs
git commit -m "feat(engine): add DEMO_ENDPOINT support and engine session bootstrap"
```

---

### Task 5: UI Menu & Header Dropdown Integration

**Files:**
- Modify: `src/ui/menu.rs`
- Modify: `src/app.rs`
- Test: `tests/ui_demo_menu_test.rs`

**Interfaces:**
- Consumes: `DEMO_ENDPOINT` (from Task 4) & `build_demo_showcase_timeline` (from Task 2)
- Produces: Visual dropdown entry with label `⚡ Virtual Board (Demo Simulation)`, menu item "Start Virtual Board Demo", and auto-loading of the showcase project upon connecting to demo.

- [x] **Step 1: Write the failing test**

Create `tests/ui_demo_menu_test.rs`:
```rust
use pealayer::four_d::controller::DEMO_ENDPOINT;

#[test]
fn test_demo_endpoint_formatting_and_identity() {
    assert!(pealayer::four_d::controller::is_demo_endpoint(DEMO_ENDPOINT));
}
```

- [x] **Step 2: Update `src/ui/menu.rs` and `src/app.rs`**

In `src/ui/menu.rs`:
In `header_endpoint_combo`:
Render `demo://virtual-board` with an explicit label:
```rust
for endpoint in crate::four_d::controller::available_endpoints()
    .into_iter().chain(crate::server::binary_udp::discovered_controllers())
{
    let display_label = if crate::four_d::controller::is_demo_endpoint(&endpoint) {
        format!("{} Virtual Board (Demo Simulation)", crate::ui::icons::LIGHTNING)
    } else {
        endpoint.clone()
    };
    endpoint_changed |= ui
        .dropdown_value(
            &mut app.serial_port,
            endpoint.clone(),
            display_label,
        )
        .changed();
}
```

In the status indicator in `src/ui/menu.rs`:
```rust
} else if app.is_connected {
    if crate::four_d::controller::is_demo_endpoint(&app.serial_port) {
        format!("{} Virtual Board (Demo)", crate::ui::icons::LIGHTNING)
    } else if crate::four_d::controller::is_controller_endpoint(&app.serial_port) {
        ...
```

In the **File** top menu in `src/ui/menu.rs`:
Add:
```rust
if ui.button(format!("{} Start Virtual Board Demo", crate::ui::icons::LIGHTNING)).clicked() {
    ui.close();
    app.start_virtual_board_demo();
}
```

In `src/app.rs`:
Add method `start_virtual_board_demo`:
```rust
pub fn start_virtual_board_demo(&mut self) {
    self.serial_port = crate::four_d::controller::DEMO_ENDPOINT.to_string();
    if self.current_video_path.is_none() {
        if let Some(media) = crate::four_d::demo_timeline::demo_media_target() {
            self.load_media_target(&media.to_string_lossy());
        }
    }
    if self.timeline.instances.is_empty() {
        self.timeline = crate::four_d::demo_timeline::build_demo_showcase_timeline();
    }
    self.request_hardware_connection();
    self.save_config();
}
```

- [x] **Step 3: Run test and verify app compiles**

Run: `cargo test --test ui_demo_menu_test`
Expected: PASS

- [x] **Step 4: Commit**

```bash
git add src/ui/menu.rs src/app.rs tests/ui_demo_menu_test.rs
git commit -m "feat(ui): add Virtual Board Demo option to header combo and menu"
```

---

### Task 6: End-to-End Integration Test & Verification

**Files:**
- Create: `tests/virtual_board_demo_integration_test.rs`

- [x] **Step 1: Write the end-to-end integration test**

Create `tests/virtual_board_demo_integration_test.rs`:
```rust
use pealayer::four_d::controller::{ControllerClient, DEMO_ENDPOINT, is_demo_endpoint};
use pealayer::four_d::demo_coordinator::DemoCoordinatorSession;
use pealayer::four_d::demo_timeline::build_demo_showcase_timeline;

#[test]
fn test_virtual_board_demo_full_lifecycle() {
    if std::env::var_os("PEALAYER_PCCONTROLLER_LIBRARY").is_none() &&
       pealayer::four_d::embedded_host::library_candidates().is_empty() {
        eprintln!("Skipping integration test: PCController library candidate not found");
        return;
    }
    if pealayer::four_d::virtual_board::VirtualBoardSession::resolve_binary_path().is_none() {
        eprintln!("Skipping integration test: virtual_board binary not found");
        return;
    }

    assert!(is_demo_endpoint(DEMO_ENDPOINT));

    // 1. Start Demo coordinator session
    let mut session = DemoCoordinatorSession::start().expect("start demo session");

    // 2. Reconcile capabilities
    let caps = session.client.hardware_capabilities().expect("hardware capabilities");
    assert!(caps.board_connected);
    assert!(caps.supports_segment_display);
    assert!(caps.supports_addressable_led);
    assert!(caps.controls.iter().any(|c| c.key == "seat.a"));

    // 3. Verify showcase timeline loads valid cues
    let timeline = build_demo_showcase_timeline();
    assert!(!timeline.instances.is_empty());
    assert!(!timeline.analog_tracks.is_empty());

    // 4. Test cue execution through coordinator
    let relay_res = session.client.call("controller.command.execute", serde_json::json!({"command": "relay 1 on"}));
    assert!(relay_res.is_ok());

    let pwm_res = session.client.call("controller.pwm.set", serde_json::json!({"channel": 0, "raw_value": 2048}));
    assert!(pwm_res.is_ok());

    // 5. Clean teardown
    drop(session);
}
```

- [x] **Step 2: Run the integration test**

Run: `cargo test --test virtual_board_demo_integration_test -- --nocapture`
Expected: PASS

- [x] **Step 3: Commit**

```bash
git add tests/virtual_board_demo_integration_test.rs
git commit -m "test(demo): add full lifecycle integration test for virtual board demo mode"
```
