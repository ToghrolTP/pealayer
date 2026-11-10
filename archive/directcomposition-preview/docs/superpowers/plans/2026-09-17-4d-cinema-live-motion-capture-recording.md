# 4D Cinema Live Curve Recording & Motion Capture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement real-time 4D cinema actuator curve automation recording ("Motion Capture") with multi-modal inputs (gamepad/joystick, on-screen live fader, keyboard throttle), live hardware pass-through, real-time visual recording trail in the NLE timeline, and automated Ramer-Douglas-Peucker (RDP) keyframe decimation.

**Architecture:** 
- `RecordingSession` captures high-frequency 50Hz `(time_ms, normalized_value)` gesture samples from armed tracks during playback.
- Pure Rust Ramer-Douglas-Peucker (RDP) decimation compresses thousands of raw points into smooth, human-editable keyframes on punch-out.
- Immediate hardware pass-through sends `PwmSet` frames to the serial engine/VirtualBoard as the designer moves inputs, providing instantaneous physical feedback.
- NLE timeline renders active red recording ghost trails behind the playhead and interactive track arming buttons `[●]`.

**Tech Stack:** Rust 2024 edition, `egui`, `eframe`, `uuid`, `serde`, `libmpv2`.

**Spec:** `docs/superpowers/specs/2026-09-17-4d-cinema-live-motion-capture-recording.md`

## Global Constraints
- Zero unnecessary external dependencies in core algorithms; RDP must be pure Rust and $O(n \log n)$.
- Maintain backward compatibility with existing `.4d` / JSON sidecar project timelines.
- 50Hz real-time responsiveness (<20ms latency from physical input to serial `Command::PwmSet`).
- All 74 existing tests must remain 100% passing at each task gate.
- Zero compiler warnings (`-D warnings` standard).

---

### Task 1: Ramer-Douglas-Peucker (RDP) Algorithm & Sample Decimation Engine

**Files:**
- Create: `src/four_d/curve_record.rs`
- Modify: `src/four_d/mod.rs`
- Test: `tests/curve_record_test.rs`

**Interfaces:**
- Produces:
  - `pub struct RdpPoint { pub x: f64, pub y: f64 }`
  - `pub fn simplify_rdp(points: &[RdpPoint], epsilon: f64) -> Vec<RdpPoint>`
  - `pub struct RecordingSession`: manages per-track raw sample buffers, punch-in, punch-out, and committing decimated keyframes into `AnalogTrack`.

- [ ] **Step 1: Write the failing test**

Create `tests/curve_record_test.rs`:
```rust
use pealayer::four_d::curve::{AnalogTrack, Interpolation};
use pealayer::four_d::curve_record::{RdpPoint, RecordingSession, simplify_rdp};
use uuid::Uuid;

#[test]
fn test_rdp_collinear_points_reduced_to_endpoints() {
    // 5 collinear points along y = 0.5 * x
    let points = vec![
        RdpPoint { x: 0.0, y: 0.0 },
        RdpPoint { x: 100.0, y: 50.0 },
        RdpPoint { x: 200.0, y: 100.0 },
        RdpPoint { x: 300.0, y: 150.0 },
        RdpPoint { x: 400.0, y: 200.0 },
    ];
    let simplified = simplify_rdp(&points, 0.01);
    assert_eq!(simplified.len(), 2);
    assert_eq!(simplified[0].x, 0.0);
    assert_eq!(simplified[1].x, 400.0);
}

#[test]
fn test_rdp_preserves_sharp_peaks() {
    let points = vec![
        RdpPoint { x: 0.0, y: 0.0 },
        RdpPoint { x: 100.0, y: 0.0 },
        RdpPoint { x: 200.0, y: 1.0 }, // Peak
        RdpPoint { x: 300.0, y: 0.0 },
        RdpPoint { x: 400.0, y: 0.0 },
    ];
    let simplified = simplify_rdp(&points, 0.05);
    assert_eq!(simplified.len(), 5);
    assert_eq!(simplified[2].x, 200.0);
    assert_eq!(simplified[2].y, 1.0);
}

#[test]
fn test_recording_session_punch_in_and_commit() {
    let mut track = AnalogTrack::new("Wind Fan", 0);
    let track_id = track.id;

    let mut session = RecordingSession::new();
    session.record_sample(track_id, 1000, 0.0);
    session.record_sample(track_id, 1100, 0.25);
    session.record_sample(track_id, 1200, 0.50);
    session.record_sample(track_id, 1300, 0.75);
    session.record_sample(track_id, 1400, 1.0);

    assert_eq!(session.sample_count(track_id), 5);

    session.commit_to_track(&mut track, 0.02, Interpolation::Linear);

    // Collinear ramp from 1000 to 1400ms should be decimated to start & end keyframes
    assert_eq!(track.keyframes.len(), 2);
    assert_eq!(track.keyframes[0].time_ms, 1000);
    assert_eq!(track.keyframes[0].value, 0.0);
    assert_eq!(track.keyframes[1].time_ms, 1400);
    assert_eq!(track.keyframes[1].value, 1.0);

    // Buffer cleared after commit
    assert_eq!(session.sample_count(track_id), 0);
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --test curve_record_test`
Expected: FAIL with "unresolved import `pealayer::four_d::curve_record`"

- [ ] **Step 3: Implement minimal code**

Create `src/four_d/curve_record.rs`:
```rust
use std::collections::HashMap;
use uuid::Uuid;
use crate::four_d::curve::{AnalogTrack, Interpolation, Keyframe};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RdpPoint {
    pub x: f64,
    pub y: f64,
}

/// Perpendicular distance from point p to line segment (a -> b)
fn perpendicular_distance(p: RdpPoint, a: RdpPoint, b: RdpPoint) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let length_sq = dx * dx + dy * dy;
    if length_sq == 0.0 {
        let px = p.x - a.x;
        let py = p.y - a.y;
        return (px * px + py * py).sqrt();
    }
    let num = (dy * p.x - dx * p.y + b.x * a.y - b.y * a.x).abs();
    num / length_sq.sqrt()
}

/// Simplifies a 2D curve using the Ramer-Douglas-Peucker (RDP) algorithm.
pub fn simplify_rdp(points: &[RdpPoint], epsilon: f64) -> Vec<RdpPoint> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut max_dist = 0.0;
    let mut max_idx = 0;
    let first = points[0];
    let last = points[points.len() - 1];

    for (i, &p) in points.iter().enumerate().skip(1).take(points.len() - 2) {
        let dist = perpendicular_distance(p, first, last);
        if dist > max_dist {
            max_dist = dist;
            max_idx = i;
        }
    }

    if max_dist > epsilon {
        let left = simplify_rdp(&points[..=max_idx], epsilon);
        let right = simplify_rdp(&points[max_idx..], epsilon);
        let mut result = left;
        result.pop(); // Remove duplicate junction point
        result.extend(right);
        result
    } else {
        vec![first, last]
    }
}

/// Manages high-frequency live recording buffers and keyframe decimation.
#[derive(Debug, Clone, Default)]
pub struct RecordingSession {
    /// In-flight recorded samples keyed by AnalogTrack ID
    pub buffers: HashMap<Uuid, Vec<(u64, f32)>>,
}

impl RecordingSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_sample(&mut self, track_id: Uuid, time_ms: u64, value: f32) {
        let buf = self.buffers.entry(track_id).or_default();
        buf.push((time_ms, value.clamp(0.0, 1.0)));
    }

    pub fn sample_count(&self, track_id: Uuid) -> usize {
        self.buffers.get(&track_id).map(|b| b.len()).unwrap_or(0)
    }

    pub fn clear(&mut self) {
        self.buffers.clear();
    }

    pub fn get_live_samples(&self, track_id: Uuid) -> Option<&[(u64, f32)]> {
        self.buffers.get(&track_id).map(|v| v.as_slice())
    }

    /// Decimates raw samples and commits keyframes into the target track.
    pub fn commit_to_track(
        &mut self,
        track: &mut AnalogTrack,
        epsilon: f64,
        interpolation: Interpolation,
    ) {
        if let Some(samples) = self.buffers.remove(&track.id) {
            if samples.is_empty() {
                return;
            }

            let rdp_points: Vec<RdpPoint> = samples
                .iter()
                .map(|&(t, v)| RdpPoint {
                    x: t as f64,
                    y: v as f64 * 1000.0, // Scale value to match millisecond scale proportions
                })
                .collect();

            // Epsilon is scaled to match the normalized value scale (0.0..=1.0)
            let scaled_epsilon = epsilon * 1000.0;
            let simplified = simplify_rdp(&rdp_points, scaled_epsilon);

            let min_time = samples.first().map(|s| s.0).unwrap_or(0);
            let max_time = samples.last().map(|s| s.0).unwrap_or(0);

            // Remove existing keyframes within the recorded time range (punch-in replace)
            track.keyframes.retain(|k| k.time_ms < min_time || k.time_ms > max_time);

            for pt in simplified {
                let time_ms = pt.x.round().max(0.0) as u64;
                let value = (pt.y / 1000.0).clamp(0.0, 1.0) as f32;
                track.add_keyframe(Keyframe::new(time_ms, value, interpolation));
            }
        }
    }
}
```

Export in `src/four_d/mod.rs`:
```rust
pub mod curve;
pub mod curve_record;
pub mod engine;
pub mod models;
pub mod patterns;
pub mod protocol;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test curve_record_test`
Expected: PASS (3 passed)

- [ ] **Step 5: Commit**

```bash
git add src/four_d/curve_record.rs src/four_d/mod.rs tests/curve_record_test.rs
git commit -m "feat(four_d): implement Ramer-Douglas-Peucker decimation and RecordingSession"
```

---

### Task 2: Multi-Modal Input Capture Controller (Keyboard & Live Throttle)

**Files:**
- Modify: `src/four_d/curve.rs:40-65`
- Create: `src/four_d/input_capture.rs`
- Modify: `src/four_d/mod.rs`
- Test: `tests/input_capture_test.rs`

**Interfaces:**
- Consumes: `src/four_d/curve.rs` (`AnalogTrack`)
- Produces:
  - `AnalogTrack.armed: bool`
  - `pub struct InputCaptureState`: manages live throttle value with rate-limited keyboard ramping and smoothing.

- [ ] **Step 1: Write the failing test**

Create `tests/input_capture_test.rs`:
```rust
use pealayer::four_d::curve::AnalogTrack;
use pealayer::four_d::input_capture::InputCaptureState;

#[test]
fn test_analog_track_armed_field_default_false() {
    let track = AnalogTrack::new("Wind Fan", 0);
    assert!(!track.armed);
}

#[test]
fn test_input_capture_throttle_clamping_and_ramping() {
    let mut input = InputCaptureState::new();
    assert_eq!(input.current_throttle, 0.0);

    // Set absolute value
    input.set_throttle(0.75);
    assert_eq!(input.current_throttle, 0.75);

    input.set_throttle(1.5); // Clamped
    assert_eq!(input.current_throttle, 1.0);

    input.set_throttle(-0.5); // Clamped
    assert_eq!(input.current_throttle, 0.0);

    // Ramp up by delta
    input.ramp_throttle(0.2);
    assert!((input.current_throttle - 0.2).abs() < 0.001);

    input.ramp_throttle(-0.1);
    assert!((input.current_throttle - 0.1).abs() < 0.001);
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --test input_capture_test`
Expected: FAIL with missing `armed` field or missing `input_capture` module.

- [ ] **Step 3: Implement minimal code**

In `src/four_d/curve.rs`:
Add `pub armed: bool` to `AnalogTrack`:
```rust
pub struct AnalogTrack {
    pub id: Uuid,
    pub name: String,
    pub channel: u8,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub armed: bool,
    pub keyframes: Vec<Keyframe>,
}
```
In `AnalogTrack::new`: initialize `armed: false`.

Create `src/four_d/input_capture.rs`:
```rust
use std::time::Instant;

/// Captures physical and virtual inputs (keyboard, slider, gamepad) and normalizes to 0.0 ..= 1.0.
#[derive(Debug, Clone)]
pub struct InputCaptureState {
    pub current_throttle: f32,
    pub last_update: Instant,
    pub ramp_rate_per_sec: f32,
}

impl Default for InputCaptureState {
    fn default() -> Self {
        Self {
            current_throttle: 0.0,
            last_update: Instant::now(),
            ramp_rate_per_sec: 1.5, // Full stroke 0 -> 1 in ~0.67s
        }
    }
}

impl InputCaptureState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_throttle(&mut self, value: f32) {
        self.current_throttle = value.clamp(0.0, 1.0);
        self.last_update = Instant::now();
    }

    pub fn ramp_throttle(&mut self, delta: f32) {
        self.current_throttle = (self.current_throttle + delta).clamp(0.0, 1.0);
        self.last_update = Instant::now();
    }

    /// Updates throttle based on keyboard arrow/WASD inputs.
    pub fn update_from_keyboard(&mut self, up_pressed: bool, down_pressed: bool, dt_secs: f32) {
        if up_pressed && !down_pressed {
            self.ramp_throttle(self.ramp_rate_per_sec * dt_secs);
        } else if down_pressed && !up_pressed {
            self.ramp_throttle(-self.ramp_rate_per_sec * dt_secs);
        }
    }
}
```

Export in `src/four_d/mod.rs`:
```rust
pub mod curve;
pub mod curve_record;
pub mod engine;
pub mod input_capture;
pub mod models;
pub mod patterns;
pub mod protocol;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test input_capture_test`
Expected: PASS (2 passed)

- [ ] **Step 5: Commit**

```bash
git add src/four_d/curve.rs src/four_d/input_capture.rs src/four_d/mod.rs tests/input_capture_test.rs
git commit -m "feat(four_d): add armed state to AnalogTrack and implement InputCaptureState"
```

---

### Task 3: Real-Time Engine Hardware Pass-Through During Recording

**Files:**
- Modify: `src/four_d/engine.rs`
- Test: `tests/live_recording_engine_test.rs`

**Interfaces:**
- Consumes: `src/four_d/engine.rs`
- Produces:
  - `EngineMessage::LiveActuatorOverride { channel: u8, value: u8 }`: bypasses playback queue and immediately transmits `PwmSet` with lowest possible latency.

- [ ] **Step 1: Write the failing test**

Create `tests/live_recording_engine_test.rs`:
```rust
use pealayer::four_d::engine::{EngineMessage, spawn_engine};

#[test]
fn test_live_actuator_override_message() {
    let handle = spawn_engine();
    let res = handle.sender.send(EngineMessage::LiveActuatorOverride {
        channel: 2,
        value: 180,
    });
    assert!(res.is_ok());
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --test live_recording_engine_test`
Expected: FAIL with "no variant `LiveActuatorOverride` on `EngineMessage`"

- [ ] **Step 3: Implement minimal code**

In `src/four_d/engine.rs`:
Add variant to `EngineMessage`:
```rust
pub enum EngineMessage {
    UpdateQueue(Vec<CompiledAction>),
    UpdateAnalogTracks(Vec<crate::four_d::curve::AnalogTrack>),
    LiveActuatorOverride { channel: u8, value: u8 },
    Seek(u64),
    SendCommand(Command),
}
```

In `spawn_engine` message loop:
```rust
                    EngineMessage::LiveActuatorOverride { channel, value } => {
                        let ch = channel as usize;
                        if ch < 16 && value != last_pwm_values[ch] {
                            last_pwm_values[ch] = value;
                            if connected {
                                if let Some(ref mut port) = active_port {
                                    let cmd = Command::PwmSet { channel, value };
                                    let frame = cmd.to_frame();
                                    let _ = port.write_all(&frame);
                                }
                            }
                        }
                    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test live_recording_engine_test`
Expected: PASS (1 passed)

- [ ] **Step 5: Commit**

```bash
git add src/four_d/engine.rs tests/live_recording_engine_test.rs
git commit -m "feat(four_d): add LiveActuatorOverride to engine for real-time motion capture"
```

---

### Task 4: NLE Timeline UI: Arming, Live Fader HUD, and Real-Time Recording Ghost Line

**Files:**
- Modify: `src/app.rs`
- Modify: `src/main.rs`
- Modify: `src/ui/layout.rs`

**Interfaces:**
- Consumes:
  - `RecordingSession` from `src/four_d/curve_record.rs`
  - `InputCaptureState` from `src/four_d/input_capture.rs`
  - `EngineMessage::LiveActuatorOverride` from `src/four_d/engine.rs`
- Produces:
  - Professional `[●]` Record Arm button in each analog track header.
  - Live fader slider in track header when armed.
  - Active red recording trail drawn on timeline during playback.
  - Automatic punch-in on play and punch-out with RDP keyframe decimation on pause/seek/stop.

- [ ] **Step 1: Add state to `PealayerApp`**

In `src/app.rs`:
```rust
    pub(crate) recording_session: crate::four_d::curve_record::RecordingSession,
    pub(crate) input_capture: crate::four_d::input_capture::InputCaptureState,
    pub(crate) is_recording: bool,
```

In `src/main.rs`:
Initialize in `PealayerApp`:
```rust
    recording_session: crate::four_d::curve_record::RecordingSession::new(),
    input_capture: crate::four_d::input_capture::InputCaptureState::new(),
    is_recording: false,
```

- [ ] **Step 2: Add Track Arming `[●]` & Live Fader in Track Headers**

In `src/ui/layout.rs`:
In the analog track headers loop:
- Add arm button `[●]`:
```rust
    // Record Arm Button [●]
    let arm_color = if track.armed {
        egui::Color32::from_rgb(255, 60, 60)
    } else {
        egui::Color32::from_rgb(120, 120, 120)
    };
    let arm_btn = ui.selectable_label(
        track.armed,
        egui::RichText::new("●").size(12.0).color(arm_color),
    );
    if arm_btn.clicked() {
        track.armed = !track.armed;
    }
```
- If `track.armed`, show a live fader slider `0.0..=1.0` right in the track header:
```rust
    if track.armed {
        let mut val = self.app.input_capture.current_throttle;
        let slider = egui::Slider::new(&mut val, 0.0..=1.0)
            .show_value(false)
            .text("Live");
        if ui.add_sized([50.0, 16.0], slider).changed() {
            self.app.input_capture.set_throttle(val);
            let byte_val = (val * 255.0).round() as u8;
            let _ = self.app.engine_handle.sender.send(
                crate::four_d::engine::EngineMessage::LiveActuatorOverride {
                    channel: track.channel,
                    value: byte_val,
                },
            );
        }
    }
```

- [ ] **Step 3: Implement Live Recording Sample Collection & Ghost Trail**

In `src/ui/layout.rs`:
- Check keyboard throttle hotkeys (`W`/`S` or `Up`/`Down`) during frame update:
```rust
    let up = ui.input(|i| i.key_down(egui::Key::W) || i.key_down(egui::Key::ArrowUp));
    let down = ui.input(|i| i.key_down(egui::Key::S) || i.key_down(egui::Key::ArrowDown));
    let dt = ui.input(|i| i.unstable_dt).min(0.05);
    self.app.input_capture.update_from_keyboard(up, down, dt);
```
- When playing (`!self.app.is_paused && self.app.duration > 0.0`):
  For any track where `track.armed`:
  - Capture sample: `self.app.recording_session.record_sample(track.id, cur_time_ms, self.app.input_capture.current_throttle)`
  - Send real-time override to engine: `EngineMessage::LiveActuatorOverride { channel: track.channel, value: (self.app.input_capture.current_throttle * 255.0) as u8 }`
- Draw red recording ghost trail:
  For any armed track with live samples in `self.app.recording_session.get_live_samples(track.id)`:
  - Plot polyline in vivid red `egui::Color32::from_rgb(255, 50, 50)` with thickness `2.5`.

- [ ] **Step 4: Auto Punch-Out & RDP Decimation on Pause / Stop / Seek**

In `src/ui/layout.rs`:
Detect pause transition (`was_playing && is_paused` or `seek`):
```rust
    if was_recording && !is_playing_now {
        for track in self.app.timeline.analog_tracks.iter_mut() {
            if track.armed {
                self.app.recording_session.commit_to_track(
                    track,
                    0.015, // 1.5% RDP tolerance
                    crate::four_d::curve::Interpolation::Smooth,
                );
            }
        }
        let _ = self.app.engine_handle.sender.send(
            crate::four_d::engine::EngineMessage::UpdateAnalogTracks(
                self.app.timeline.analog_tracks.clone(),
            ),
        );
    }
```

- [ ] **Step 5: Run full cargo test suite**

Run: `cargo test`
Expected: ALL tests pass cleanly with 0 warnings.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/main.rs src/ui/layout.rs
git commit -m "feat(ui): add live automation curve recording, track arming, and NLE ghost trail"
```

---

### Task 5: End-to-End Simulation Test: Live Automation Recording into VirtualBoard

**Files:**
- Create: `tests/live_recording_sim_test.rs`

**Interfaces:**
- Consumes:
  - `RecordingSession` from `src/four_d/curve_record.rs`
  - `VirtualBoardProcess` from `tests/pccontroller_virtual_board_test.rs`
  - `Command::to_pccontroller_frame` from `src/four_d/protocol.rs`
- Produces:
  - End-to-end verification of real-time gesture streaming, RDP decimation, and physical actuator command receipt by C++ `VirtualBoard`.

- [ ] **Step 1: Write integration test**

Create `tests/live_recording_sim_test.rs`:
```rust
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::thread;
use std::time::Duration;

use pealayer::four_d::curve::{AnalogTrack, Interpolation};
use pealayer::four_d::curve_record::RecordingSession;
use pealayer::four_d::protocol::{Command, decode_pccontroller_frame};

struct TestVirtualBoard {
    child: Child,
    pub port: u16,
}

impl TestVirtualBoard {
    fn spawn(port: u16) -> Self {
        let bin = "scratch/PCController/Tools/VirtualBoard/.build/release/bin/virtual_board";
        let child = ProcessCommand::new(bin)
            .args(["--bind", "127.0.0.1", "--port", &port.to_string(), "--no-stdin", "--quiet"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("VirtualBoard executable not found");
        thread::sleep(Duration::from_millis(150));
        Self { child, port }
    }
}

impl Drop for TestVirtualBoard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn test_live_recording_to_virtual_board_stream() {
    let port = 8793;
    let _vb = TestVirtualBoard::spawn(port);

    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .expect("Failed to connect to VirtualBoard");

    // 1. Simulate live motion capture session at 50Hz for 500ms (10 samples)
    let mut track = AnalogTrack::new("Wind Turbine", 0);
    track.armed = true;
    let track_id = track.id;

    let mut session = RecordingSession::new();
    let mut seq = 10u8;

    for i in 0..10 {
        let t = i * 50;
        let val = (i as f32) / 10.0;
        session.record_sample(track_id, t, val);

        // Hardware pass-through frame
        let byte_val = (val * 255.0).round() as u8;
        let cmd = Command::PwmSet { channel: track.channel, value: byte_val };
        let frame = cmd.to_pccontroller_frame(seq);
        seq = seq.wrapping_add(1);
        stream.write_all(&frame).expect("Write failed");
    }

    // 2. Punch-out & commit with RDP decimation
    session.commit_to_track(&mut track, 0.02, Interpolation::Smooth);

    // Linear ramp from 0 to 450ms should decimate to start & end keyframes
    assert!(track.keyframes.len() <= 3, "RDP should decimate linear ramp: got {}", track.keyframes.len());
    assert_eq!(track.keyframes.first().unwrap().time_ms, 0);
    assert_eq!(track.keyframes.last().unwrap().time_ms, 450);

    // 3. Read back last ACK from VirtualBoard
    let mut buf = [0u8; 32];
    stream.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    let n = stream.read(&mut buf).expect("Failed to read ACK from VirtualBoard");
    assert!(n > 0);
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --test live_recording_sim_test`
Expected: PASS (1 passed)

- [ ] **Step 3: Run full workspace test suite**

Run: `cargo test`
Expected: ALL 78+ unit and integration tests passing across all suites.

- [ ] **Step 4: Commit**

```bash
git add tests/live_recording_sim_test.rs
git commit -m "test(sim): add end-to-end live motion capture integration test with VirtualBoard"
```
