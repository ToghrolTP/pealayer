# 4D Cinema Live Curve Recording & Motion Capture Specification (Phase 2, Step 3)

## 1. Goal
Provide professional 4D cinema motion/sound designers with real-time analog curve authoring ("Automation Recording"). Allow authors to arm analog tracks and capture continuous actuator gestures (fans, water mist intensity, scent dispersal, shaker rumble) in real time using gamepads, keyboard throttles, or on-screen faders during video playback, with automatic Ramer-Douglas-Peucker (RDP) keyframe simplification and zero-latency hardware pass-through.

## 2. Professional UX Principles & Design Requirements
1. **Track Record Arming**:
   - Each analog track in the NLE timeline features a dedicated `[●]` (Record Arm) toggle button in its header.
   - Visual state:
     - Unarmed: subtle neutral icon `○`
     - Armed: vivid recording red `●` with glowing header accent border.
   - Multiple tracks can be armed simultaneously for multi-axis motion capture (e.g. Left Stick Y = Wind, Right Trigger = Rumble).
2. **Multi-Modal Input Capture**:
   - **Gamepad / Joystick**: Cross-platform gamepad polling via `gilrs`. Left Stick Y-axis and Analog Triggers map to armed tracks with deadzone filtering.
   - **Interactive Live Fader / Jog**: Real-time on-screen slider directly in the track header or timeline HUD.
   - **Keyboard Throttle**: `W` / `Up` ramps throttle up; `S` / `Down` ramps down; number keys `0`-`9` set 0%-90%.
3. **Live Pass-Through Monitoring**:
   - During recording, actuator values are immediately dispatched to hardware/VirtualBoard (`Command::PwmSet`) at 50Hz (20ms intervals), giving the designer immediate physical feedback.
4. **Live Visual Recording Trail**:
   - During active playback while armed, the NLE timeline paints an active recording trail in crimson red behind the moving playhead.
5. **Ramer-Douglas-Peucker (RDP) Keyframe Decimation**:
   - Raw 50Hz sampling generates 3,000 points/minute.
   - On punch-out (stop/pause/seek), the raw samples are decimated using RDP with epsilon tolerance `0.015` (1.5%).
   - Decimated keyframes are inserted into `AnalogTrack::keyframes`, preserving linear/smooth interpolation and replacing overlapping existing keyframes in the recorded time window.
   - Full serialization and undo compatibility.

## 3. Architecture & Interfaces
- `src/four_d/curve_record.rs`:
  - `RdpPoint`: struct for 2D curve simplification: `time_ms: f64, value: f64`.
  - `simplify_curve_rdp(points: &[RdpPoint], epsilon: f64) -> Vec<RdpPoint>`: Pure Rust RDP implementation.
  - `RecordedSample`: `(u64, f32)` timestamp and normalized value.
  - `RecordingSession`: Manages active capture buffers per armed track.
- `src/four_d/gamepad.rs`:
  - Optional/embedded gamepad listener polling analog sticks and triggers, translating to normalized 0.0..=1.0 values.
- `src/four_d/curve.rs`:
  - Adds `pub armed: bool` to `AnalogTrack`.
- `src/ui/layout.rs`:
  - Record button in track header.
  - Live fader widget.
  - Red recording ghost line rendering during active capture.
  - Punch-in on play, punch-out and decimation on pause/seek.

## 4. Test Strategy
- Unit tests for RDP curve decimation:
  - Exact preservation of endpoints.
  - Elimination of collinear points on linear ramp.
  - Retention of sharp peaks and valleys.
  - Threshold tolerance bounds.
- Unit tests for RecordingSession:
  - Sample collection, punch-in, punch-out, and track keyframe replacement.
- Integration tests with VirtualBoard:
  - Verifying 50Hz `PwmSet` command streaming during recording.
