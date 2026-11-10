# Virtual Board Demo Mode Architecture & Integration Design

- **Status**: Approved
- **Date**: 2026-10-16
- **Authors**: Antigravity & User

---

## 1. Context & Motivation

Pealayer authors and controls 4D cinema hardware (seats, pneumatic actuators, fans, lighting, and special effects) via the PCController coordinator. Previously, running and testing Pealayer required either physical cinema hardware connected via UART or manually orchestrating external background daemons and development scripts.

PCController provides a native C++ ATmega328P emulator (`virtual_board`) that implements the production firmware protocol over loopback TCP (`tcp://127.0.0.1:8765`), simulating 8 relays, 16 PWM channels, addressable RGB LED strips (WS2812), TM1637 4-digit segment display, 16x2 LCD, buzzer, temperature sensors, and RF events. Pealayer also already possesses an `EmbeddedHost` interface that can load the PCController C-ABI library (`pccontroller.dll` / `pccontroller.so`) in-process.

This design introduces a seamless, 1-click **Demo Mode** directly in the Pealayer UI. When selected, Pealayer manages the virtual MCU process and embedded coordinator behind the scenes, configures the full cinema seat motion profile, and loads a bundled showcase project (video + synchronized 4D timeline), allowing any user to immediately explore and test the entire application without physical hardware.

---

## 2. Goals & Non-Goals

### Goals
- **1-Click Experience**: Users can select `demo://virtual-board` from the hardware endpoint dropdown or menu and immediately have a working 4D cinema setup.
- **Strict Alpha Contract Adherence**: Use the authoritative PCController coordinator and native production firmware emulator rather than maintaining a custom mock board in Pealayer.
- **Process & Storage Isolation**: The virtual board runs with `--memory` (RAM-only EEPROM), and the embedded coordinator uses a disposable temporary data directory (`pealayer-demo-data-<pid>`). No persistent EEPROM or user configs are modified.
- **Automatic Showcase Content**: If no media is loaded, automatically loads `test-data/jellyfish.mp4` and seeds a rich, pre-authored 4D demo timeline (seat motion, wind curve, lighting, strobe) ready for playback.
- **Clean Lifecycle & Resource Teardown**: Automatically kills child processes and shuts down the coordinator on disconnect, exit, or endpoint switch.

### Non-Goals
- Synthesizing electrical faults, analog bus drift, or AVR cycle-exact timing (the virtual board deliberately runs on wall-clock time).
- Replacing or modifying production deployment routines on physical cinema hosts.

---

## 3. Architecture & Components

```
+--------------------------------------------------------------------------------+
| Pealayer Application                                                           |
|                                                                                |
|  [ UI Header Dropdown ] ---> Selects "demo://virtual-board"                    |
|          |                                                                     |
|          v                                                                     |
|  [ DemoSessionManager ]                                                        |
|     |                                                                          |
|     +--> 1. Spawns Child Process:                                              |
|     |       virtual_board --bind 127.0.0.1 --port <ephemeral> --memory --quiet |
|     |                                                                          |
|     +--> 2. Starts In-Process Coordinator:                                     |
|     |       EmbeddedHost (pccontroller.so / pccontroller.dll)                  |
|     |       Points coordinator to tcp://127.0.0.1:<ephemeral>                  |
|     |       Configures profile: mode="cinema-seat-motion"                      |
|     |                                                                          |
|     +--> 3. Attaches HardwareTransport::Controller to Pealayer Engine           |
|     |       Engine receives full HardwareCapabilities (Seat A/B, PWM, Relays)   |
|     |                                                                          |
|     +--> 4. Loads test-data/jellyfish.mp4 & Seeds 4D Demo Timeline              |
+--------------------------------------------------------------------------------+
```

### 3.1. `VirtualBoardSession` (Process Supervisor)
Located in `src/four_d/virtual_board.rs`:
- **Binary Resolution**:
  1. `PEALAYER_VIRTUAL_BOARD_BINARY` environment variable.
  2. Adjacent executable path (`bin/virtual_board` or `virtual_board.exe`).
  3. Development build path (`scratch/PCController/Tools/VirtualBoard/.build/release/bin/virtual_board`).
- **Process Launch**:
  - Reserves an available loopback port via `TcpListener::bind(("127.0.0.1", 0))`.
  - Spawns `virtual_board` with arguments:
    `["--bind", "127.0.0.1", "--port", &port.to_string(), "--no-stdin", "--quiet", "--memory"]`.
  - Performs a non-blocking TCP connect check with a 3-second deadline to ensure readiness.
- **Teardown**:
  - Implements `Drop` to send `kill()` and `wait()` on the child process, preventing orphaned background processes.

### 3.2. `DemoCoordinatorSession` (In-Process Coordinator)
- **Coordinator Startup**:
  - Initializes `EmbeddedHostOptions` pointing to a private OS temporary directory (`pealayer-demo-data-<pid>`) with `disable_auto_connect = true`, `disable_native = true`, and `enable_integrations = false`.
  - Loads the native library via `EmbeddedHost::load_and_start`.
- **Hardware Profile Provisioning**:
  - Connects the coordinator to the virtual board:
    `controller.connect` with `{"port": "tcp://127.0.0.1:<port>"}`.
  - Updates the active profile to `cinema-seat-motion`:
    `controller.board_profile.update` with `{"key": "demo", "mode": "cinema-seat-motion"}`.
- **Engine Attachment**:
  - Installs the resulting `ControllerClient` into Pealayer's `HardwareTransport::Controller`.
  - Dispatches `refresh_capabilities()`, exposing the complete cinema feature set to Pealayer's engine and UI.

### 3.3. Showcase Media & Timeline Seeder
Located in `src/four_d/demo_timeline.rs`:
- Generates a pre-configured `Timeline` synchronized to `test-data/jellyfish.mp4`:
  - **Seat Motion**: Pitch, roll, and heave cues assigned to Seat A and Seat B.
  - **Analog PWM Track**: Smooth fan acceleration curves mapped to PWM channel 0.
  - **Relay Cues**: Timed triggers for atmospheric wind, mist, and lighting strobes.
  - **RGB Strip**: Ambient color pulses for the WS2812 addressable LED strip.
  - **Segment Display**: Timecode and status messages on the TM1637 display.
- Automatically loaded if no media/timeline is currently playing or loaded.

### 3.4. UI Integration & Feedback
Located in `src/ui/menu.rs`:
- **Hardware Dropdown**: Includes `demo://virtual-board` labeled as `"⚡ Virtual Board (Demo Simulation)"`.
- **Top Menu**: Adds `"Start Virtual Board Demo"` under the `File` or `Hardware` menu.
- **Visual Status**: Shows `⚡ Virtual Board (Demo)` with a green indicator dot when active.
- **Teardown on Disconnect**: Clicking "Disconnect" or selecting another hardware endpoint terminates the demo session and restores default hardware state.

---

## 4. Error Handling & Edge Cases

1. **Missing Binaries**:
   - If `virtual_board` executable or `pccontroller.dll/.so` is missing, connection fails safely without modifying the current endpoint.
   - A descriptive warning toast is published via `crate::messaging::publish` indicating which binary is missing and where it was searched.
2. **Port Allocation / Startup Failure**:
   - If port reservation or process launch fails, the temporary directory is cleaned up and a connection error is recorded in the UI status.
3. **Application Exit**:
   - Standard process termination hooks and RAII guards guarantee that `VirtualBoardSession` drops and reaps the child process.

---

## 5. Verification Plan

1. **Unit Tests**:
   - Resolver tests verifying binary search precedence (`PEALAYER_VIRTUAL_BOARD_BINARY` -> `bin/` -> `scratch/...`).
   - Demo timeline generation test validating valid cues, keyframes, and analog curve structures.
2. **Integration Test**:
   - `tests/virtual_board_demo_test.rs`:
     - Spawns `VirtualBoardSession`.
     - Boots `EmbeddedHost` and connects to the virtual board.
     - Confirms cinema seat capabilities are parsed.
     - Executes relay, PWM, and seat motion commands.
     - Drops the session and verifies clean child process termination.
3. **Manual Interactive Test**:
   - Run `cargo run`.
   - Select `demo://virtual-board`.
   - Verify that `test-data/jellyfish.mp4` loads, pressing Spacebar plays video + 4D timeline, and hardware monitor UI displays real-time motion and relay states.
