# Pealayer

Pealayer is a lightweight, hardware-accelerated media player built in Rust. It utilizes [egui](https://github.com/emilk/egui) (via `eframe`) for its user interface and [mpv](https://mpv.io/) (via the `libmpv2` crate) as its underlying playback engine.

Designed for fluid desktop performance, it provides a clean graphical control layout over mpv's powerful media backend, complete with hardware-accelerated OpenGL video rendering directly within the egui window context.

---

## Features

- **Hardware Accelerated Playback**: Renders video frames directly into an OpenGL texture within the egui interface using a custom mpv render context.
- **Dynamic Controls**:
  - Intuitive seek bar with real-time time-position and duration display.
  - Play/pause toggles and precise volume adjustments (including rapid mute/unmute).
- **Subtitles & Text Configuration**:
  - Easily toggle subtitle visibility.
  - Dynamically resize subtitles via UI slider control.
  - Adjust subtitle delay to fix out-of-sync tracks.
  - Support for multi-track subtitle selection.
  - Bundled with the **Vazirmatn** font out of the box to render Arabic and Farsi subtitles beautifully.
- **Audio Controls**:
  - Interactive audio settings panel.
  - Precision audio delay/sync adjustments.
  - Multi-track audio stream switching.
- **Auto-Hiding Interface**: Controls automatically fade out during playback inactivity to provide an immersive, distraction-free viewing experience.

---

## Prerequisites

Because Pealayer relies on the `mpv` library, you must have the developer dependencies of `libmpv` installed on your host system to build and compile the application.

### Linux (Debian / Ubuntu)

Install the development packages for `mpv` and standard audio/graphics helpers:
```bash
sudo apt update
sudo apt install libmpv-dev pkg-config libasound2-dev libx11-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

### macOS

Install `mpv` via Homebrew:
```bash
brew install mpv
```

### Windows

1. Download the `mpv` dynamic libraries (`mpv.dll` and `mpv.lib`) from a repository/build site (e.g., [mpv-player/mpv](https://mpv.io/installation/)).
2. Place `mpv.dll` and `mpv.lib` in your system path or directory where your built binaries reside, or configure cargo to find them.

---


## CI/CD

GitHub Actions now builds release artifacts for **Linux (Ubuntu)** and **Windows** on every push and pull request.

- Windows CI downloads **libmpv** from a GitHub release and bundles `mpv-2.dll` with the built executable.
- Each run uploads platform-specific archives as workflow artifacts.
- Tag-triggered runs publish those artifacts to GitHub Releases automatically.
- You can also run the workflow manually and enable branch-based prereleases.

---

## Installation & Running

1. **Clone the repository**:
   ```bash
   git clone https://github.com/ToghrolTP/pealayer.git
   cd pealayer
   ```

2. **Run the player**:
   ```bash
   cargo run --release
   ```

---

## Project Structure

- `src/main.rs`: Entry point initializing the `eframe` window and creating the `mpv` instance, OpenGL render context, and property observers.
- `src/app.rs`: Implements the main `PealayerApp` struct containing playback states, tracks, delays, and state synchronization.
- `src/ui/`: Contains the user interface rendering components, control layout, overlays, settings windows, and style configuration.
- `src/mpv/`: Contains wrappers and integration utilities for managing mpv events, client streams, and OpenGL render contexts.
- `test-data/`: Holds sample testing assets, including the bundled Vazirmatn font directory.

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
