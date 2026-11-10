# Virtual Board Digital Twin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Option B (Observability & Digital Twin UI) in Pealayer, providing a real-time virtual front panel (LCD/7-segment/RGB), a 2D cinema seat attitude kinematic visualizer, interactive simulation controls (door switch, tactile keys, RF cue injection), and an optional floating inspector window.

**Architecture:**
1. Upgrade `VirtualBoardSession` to retain a piped stdin handle for line-oriented simulation command dispatch (`send_command`).
2. Implement `CinemaSeatKinematics` continuous numerical integrator that drives realistic dual-cylinder stroke extension and Pitch/Roll angle estimation.
3. Build `src/ui/digital_twin.rs` with front-panel cluster, vector 2D seat wireframe, and simulation control pills.
4. Integrate the Digital Twin into the top of the `HardwareMonitor` tab when connected to `demo://virtual-board`, with a floating pop-out window option.

**Tech Stack:** Rust (egui, eframe, std::process), C++ (`virtual_board`), JSON-RPC (`PCController`).

**Spec:** `docs/superpowers/specs/2026-10-18-virtual-board-digital-twin-design.md`

## Global Constraints

- Alpha has one current API and persisted-data contract: do not add legacy shims or speculative migrations.
- Zero trailing whitespace across all files (`git diff --check`).
- Tests exercising persistence must receive an isolated `PEALAYER_CONFIG_FILE=$(mktemp)`.
- Never invoke egui Context accessors, widgets, or repaint callbacks from inside input/data/memory/output transactions (non-reentrant context lock).
- Standard input/selector/menu density comes from `ui::control_density` and the shared `ui_control_density` appearance preference.
- All 16 GitHub Actions CI matrix jobs must pass 100% green.

---

### Task 1: Piped Simulation Console in `VirtualBoardSession`

**Files:**
- Modify: `src/four_d/virtual_board.rs:115-180`
- Test: `tests/virtual_board_session_test.rs`

**Interfaces:**
- Produces:
  ```rust
  impl VirtualBoardSession {
      pub fn send_command(&self, command: &str) -> Result<(), String>;
  }
  ```

- [ ] **Step 1: Write the failing test**
  Add test in `tests/virtual_board_session_test.rs` asserting `session.send_command("door closed")` succeeds and returns `Ok(())`.

- [ ] **Step 2: Run test to confirm failure**
  Run `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --test virtual_board_session_test` to confirm `send_command` does not exist.

- [ ] **Step 3: Implement piped stdin in `VirtualBoardSession`**
  In `src/four_d/virtual_board.rs`:
  - Remove `--no-stdin` from spawn arguments.
  - Set `stdin(Stdio::piped())`.
  - Capture child stdin as `Arc<Mutex<Option<ChildStdin>>>` in `VirtualBoardSession`.
  - Implement `send_command(&self, command: &str) -> Result<(), String>`.

- [ ] **Step 4: Run test to verify it passes**
  Run `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --test virtual_board_session_test`.

- [ ] **Step 5: Verify Windows cross-compilation**
  Run `cargo check --target x86_64-pc-windows-gnu --tests`.

- [ ] **Step 6: Commit Task 1**
  `git commit -m "feat(four_d): add piped simulation console to VirtualBoardSession"`

---

### Task 2: Kinematic Integrator Engine (`CinemaSeatKinematics`)

**Files:**
- Create: `src/four_d/seat_kinematics.rs`
- Modify: `src/four_d/mod.rs`
- Test: `tests/seat_kinematics_test.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct CinemaSeatKinematics {
      pub left_stroke: f32,
      pub right_stroke: f32,
  }
  impl CinemaSeatKinematics {
      pub fn update(&mut self, dt_seconds: f32, left_dir: f32, right_dir: f32, interlock_tripped: bool);
      pub fn pitch_degrees(&self) -> f32;
      pub fn roll_degrees(&self) -> f32;
  }
  ```

- [ ] **Step 1: Write the failing test**
  Create `tests/seat_kinematics_test.rs` testing slew rate, range clamping (0.0..1.0), pitch angle calculation, roll angle calculation, and door interlock freeze.

- [ ] **Step 2: Run test to confirm failure**
  Run `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --test seat_kinematics_test` to verify failure.

- [ ] **Step 3: Implement `CinemaSeatKinematics`**
  In `src/four_d/seat_kinematics.rs`:
  - Implement struct, default initialization (0.5 neutral), `update`, `pitch_degrees`, `roll_degrees`.
  - Register module in `src/four_d/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**
  Run `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --test seat_kinematics_test`.

- [ ] **Step 5: Verify Windows cross-compilation**
  Run `cargo check --target x86_64-pc-windows-gnu --tests`.

- [ ] **Step 6: Commit Task 2**
  `git commit -m "feat(four_d): implement cinema seat kinematic integrator"`

---

### Task 3: Digital Twin UI Component & Rendering (`src/ui/digital_twin.rs`)

**Files:**
- Create: `src/ui/digital_twin.rs`
- Modify: `src/ui/mod.rs`
- Test: `tests/digital_twin_ui_test.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn draw_digital_twin_card(app: &mut PealayerApp, ui: &mut egui::Ui);
  pub fn draw_digital_twin_window(app: &mut PealayerApp, ctx: &egui::Context);
  ```

- [ ] **Step 1: Write the failing test**
  Create `tests/digital_twin_ui_test.rs` verifying helper functions for LCD formatting, 7-segment segment decoding, and attitude wireframe calculations without panic.

- [ ] **Step 2: Run test to confirm failure**
  Run `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --test digital_twin_ui_test`.

- [ ] **Step 3: Implement `src/ui/digital_twin.rs`**
  - Implement 16x2 LCD matrix renderer with bezel and dot-matrix styling.
  - Implement 4-digit 7-segment display with glowing segments.
  - Implement Tri-color Enclosure RGB LED.
  - Implement 2D vector cinema seat attitude profile with animated piston shafts and pitch/roll readouts.
  - Implement interactive simulation controls (Door toggle, K1..K4 buttons, RF remote injection).
  - Implement floating window `draw_digital_twin_window`.

- [ ] **Step 4: Run test to verify it passes**
  Run `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --test digital_twin_ui_test`.

- [ ] **Step 5: Verify Windows cross-compilation**
  Run `cargo check --target x86_64-pc-windows-gnu --tests`.

- [ ] **Step 6: Commit Task 3**
  `git commit -m "feat(ui): add digital twin front-panel and 2D seat attitude visualizer"`

---

### Task 4: Integration into `PealayerApp` & `HardwareMonitor`

**Files:**
- Modify: `src/app.rs`
- Modify: `src/ui/layout.rs`
- Test: `tests/digital_twin_integration_test.rs`

**Interfaces:**
- Connects `app.seat_kinematics`, `app.show_digital_twin_window`, and updates kinematics on frame ticks.
- Embeds `draw_digital_twin_card` at the top of `PealayerTab::HardwareMonitor` when `is_demo_endpoint(&app.serial_port)`.
- Renders `draw_digital_twin_window` in `app.rs` when `show_digital_twin_window` is true.

- [ ] **Step 1: Write the failing integration test**
  Create `tests/digital_twin_integration_test.rs` testing that demo session bootstrap initializes kinematics, door switch command updates telemetry, and the digital twin responds.

- [ ] **Step 2: Run test to confirm failure**
  Run `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --test digital_twin_integration_test`.

- [ ] **Step 3: Wire `seat_kinematics` and `digital_twin` in `app.rs` and `layout.rs`**
  - Add `seat_kinematics: CinemaSeatKinematics` and `show_digital_twin_window: bool` to `PealayerApp`.
  - Update `seat_kinematics` in the main UI update loop from active motion directions and door state.
  - In `layout.rs`, call `draw_digital_twin_card` at top of `PealayerTab::HardwareMonitor` if `is_demo_endpoint(&self.app.serial_port)`.
  - In `app.rs`, call `draw_digital_twin_window` when `self.show_digital_twin_window` is true.

- [ ] **Step 4: Run integration test to verify it passes**
  Run `PEALAYER_CONFIG_FILE=$(mktemp) cargo test --test digital_twin_integration_test`.

- [ ] **Step 5: Run full test suite**
  Run `PEALAYER_CONFIG_FILE=$(mktemp) cargo test`.

- [ ] **Step 6: Commit Task 4**
  `git commit -m "feat(demo): integrate Digital Twin card into Hardware Monitor and floating window"`

---

### Task 5: Zero Whitespace Audit, Git Push & CI Matrix Monitoring

**Files:**
- All modified/created files

- [ ] **Step 1: Run whitespace audit against origin/main**
  Run `git diff --check origin/main HEAD`. Ensure 0 errors.

- [ ] **Step 2: Windows GNU target cross-check**
  Run `cargo check --target x86_64-pc-windows-gnu --tests`.

- [ ] **Step 3: Push commits to remote branch**
  Run `git push origin feature/virtual-board-demo`.

- [ ] **Step 4: Watch GitHub Actions CI run**
  Watch workflow run to completion:
  `gh run watch <run-id> --exit-status`

- [ ] **Step 5: Verify all PR checks green**
  Run `gh pr checks 207` and verify 100% green.
