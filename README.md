# Pealayer 🎬⚡

Pealayer is a lightweight, hardware-accelerated 4D media player built in Rust. It integrates **egui** (via `eframe` & `glow`) with **mpv** (via `libmpv2`) to provide a seamless video rendering context directly inside a modern, Premiere-inspired multi-tab NLE interface. 

With Pealayer, you can program physical sensory effects—like seat rumbles, fans, fog screens, and water valves—perfectly synchronized with a video's playback timeline.

![Layout Preview](docs/assets/layout_preview.gif)

---

## ✨ Core Features

*   **📺 OpenGL RTT Video Rendering**: Renders video frames with hardware acceleration directly into an offscreen framebuffer texture inside egui, maintaining a locked 16:9 aspect ratio container.
*   **⏱️ NLE Timeline Editor**: A responsive, multi-track layout (Video, Audio, Relays R1-R8) with drag-and-drop, multi-select, playhead snapping, and click-and-drag cue resizing.
*   **🔌 Live USB Serial Connection**: Real-time communication with physical microcontrollers (e.g. Arduino, ESP32) using an ASCII command protocol over configurable serial ports.
*   **🔴 Live Macro Recording**: Hold keyboard hotkeys `F1-F8` during playback to record sensory cue activations in real-time onto corresponding relay tracks.
*   **⚠️ Hardware Monitor & Safety**: Live status LEDs, manual "Force ON" overrides, and a global Emergency Stop (**E-STOP**) lock switch.
*   **💾 Automatic Sidecar Pairing**: Opens `movie.mp4` and automatically finds and loads a matching `movie.4d.json` timeline sidecar file in the same folder.

---

## 🛠️ Feature Walkthrough

### Timeline Editing & Transport Controls
Drag effects (Mist Spray, Seat Shake, Wind Gale) from the **Effects Library** directly onto the timeline tracks. Drag, select, resize, and scrub the playhead with ease.

![Feature Walkthrough](docs/assets/feature_walkthrough.gif)

### Keyboard Scripting & Live Override
Observe relay status LEDs in the **Hardware Monitor** tab. Toggle manual overrides, activate the E-STOP to shut down all relays, or use F1-F8 key macros to capture physical events in real-time.

![Macro Recording in Action](docs/assets/macro_recording.gif)

---

## 🔌 Hardware Serial Protocol

Pealayer communicates with microcontrollers (Arduino, ESP32, USB Relay Boards) over a standard serial connection (9600 baud rate by default). 

### Command Format
Whenever a relay state changes or a seek/pause occurs, Pealayer transmits a newline-terminated ASCII string:

| Command | Action | Example |
| :--- | :--- | :--- |
| `R{relay_id}:1\n` | Turn relay `relay_id` (1 to 8) **ON** | `R3:1\n` (Turn ON Seat Rumble) |
| `R{relay_id}:0\n` | Turn relay `relay_id` (1 to 8) **OFF** | `R3:0\n` (Turn OFF Seat Rumble) |

> [!TIP]
> **Safety Guard:** On playback pause, seek, disconnection, or E-STOP trigger, Pealayer automatically transmits OFF commands (`R{id}:0\n`) for all 8 relays to prevent hardware from locking in an active state.

---

## ⌨️ Shortcut Keybindings

| Keybinding | Action |
| :--- | :--- |
| `Space` | Toggle Play / Pause |
| `F` | Toggle Fullscreen |
| `M` | Mute / Unmute Video Audio |
| `Arrow Left` / `Arrow Right` | Seek 5 seconds backward / forward |
| `Arrow Up` / `Arrow Down` | Adjust master volume |
| `F1` to `F8` | Record cue activations on Relays 1 to 8 |

---

## ⚙️ Prerequisites

Because Pealayer relies on `mpv` libraries, compilation requires host development dependencies:

### Linux (Debian / Ubuntu)
```bash
sudo apt update
sudo apt install libmpv-dev pkg-config libasound2-dev libx11-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

### macOS
```brew install mpv
```

### Windows
1. Download the `mpv` shared libraries (`mpv.dll` and `mpv.lib`) from a repository/build site.
2. Put `mpv.dll` and `mpv.lib` in your system path or binary target folder so cargo can discover them.

---

## 🚀 Installation & Running

1. **Clone the repository**:
   ```bash
   git clone https://github.com/ToghrolTP/pealayer.git
   cd pealayer
   ```

2. **Run the player**:
   ```bash
   cargo run --release
   ```

3. **Verify the installation (Tests)**:
   ```bash
   cargo test
   ```

---

## 📂 Project Structure

- `src/main.rs`: Entry point initializing `eframe`, GL context, and observers.
- `src/app.rs`: Main `PealayerApp` struct containing playback states, delays, and sidecar loaders.
- `src/ui/`: UI rendering panels (timeline layout, controls, dialogs).
- `src/four_d/`: Core 4D engine, compilation logic, and background thread serial interface.
- `docs/assets/`: Embedded visual media and GIFs.

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
