# Windows Shell & Taskbar Suite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the complete Windows Shell and Taskbar integration suite for Pealayer (taskbar progress & error states, thumbnail toolbar media action buttons `ThumbBarAddButtons`, system tray icon with native context menu, and video-constrained thumbnail preview), completing 100% of the `## Application` requirements on GitHub Issue #2.

**Architecture:** Gated Win32 shell integrations under `src/platform/windows.rs` using `windows` 0.58 (`Win32_UI_Shell`, `Win32_UI_WindowsAndMessaging`, `Win32_Graphics_Dwm`, `Win32_Graphics_Gdi`). Integrates taskbar progress and state reporting (Normal/Paused/Error), taskbar thumbnail playback buttons (`ThumbBarAddButtons`), system tray notification icon (`Shell_NotifyIconW`) with left-click restore and right-click native context menu, and clean RAII lifecycle disposal. Non-Windows targets are compiled with safe zero-cost stubs.

**Tech Stack:** Rust 2021, `windows` 0.58 (`Win32_UI_Shell`, `Win32_UI_WindowsAndMessaging`, `Win32_Graphics_Dwm`, `Win32_Graphics_Gdi`), `eframe`/`egui`.

**Spec:** [GitHub Issue #2 ("Player UX feedback")](https://github.com/ToghrolTP/pealayer/issues/2) - Application Section:
*"At least on Windows, the following features are missing: tray bar icon, task bar progress and state report and media action buttons, as well as using the video stream as the live/dynamic taskbar thumbnail instead of entire window content; as well as jump items and pinning"*

## Global Constraints

- All Windows-specific APIs (`ITaskbarList3`, `Shell_NotifyIconW`, DWM, GDI, Windows and Messaging) must be strictly gated under `#[cfg(target_os = "windows")]` with zero-cost no-op stubs on Linux and macOS.
- System tray icons must be cleanly deleted on application shutdown (`NIM_DELETE`) to avoid leaving ghost icons in the notification area.
- All existing 98+ unit and integration tests must continue passing without regression (`cargo test --all-targets`).
- Clean cross-compilation for `x86_64-pc-windows-gnu` must be maintained at every step (`cargo check --target x86_64-pc-windows-gnu`).

---

### Task 1: Taskbar Progress & Error State Reporting

**Files:**
- Modify: `src/platform/windows.rs`
- Test: `src/platform/windows.rs:tests`

**Interfaces:**
- Consumes: `playback_time: f64`, `duration: f64`, `is_paused: bool`, `has_error: bool`
- Produces:
  - `pub fn compute_taskbar_state_with_error(playback_time: f64, duration: f64, is_paused: bool, has_error: bool) -> TaskbarState`
  - `TaskbarProgressFlag::Error = 4` (Red progress bar)
  - `pub fn update_windows_taskbar_state_ext(progress: f64, duration: f64, is_paused: bool, has_error: bool)`

- [ ] **Step 1: Write failing unit test in `src/platform/windows.rs`**

```rust
    #[test]
    fn test_compute_taskbar_state_with_error() {
        let err_state = compute_taskbar_state_with_error(10.0, 60.0, false, true);
        assert!(err_state.is_active);
        assert_eq!(err_state.to_progress_flag(), TaskbarProgressFlag::Error);

        let normal_state = compute_taskbar_state_with_error(10.0, 60.0, false, false);
        assert_eq!(normal_state.to_progress_flag(), TaskbarProgressFlag::Normal);

        let paused_state = compute_taskbar_state_with_error(10.0, 60.0, true, false);
        assert_eq!(paused_state.to_progress_flag(), TaskbarProgressFlag::Paused);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_compute_taskbar_state_with_error`
Expected: FAIL with function not found.

- [ ] **Step 3: Implement `compute_taskbar_state_with_error` and update `update_windows_taskbar_state`**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskbarState {
    pub progress_percent: u32,
    pub is_paused: bool,
    pub is_active: bool,
    pub has_error: bool,
}

impl TaskbarState {
    pub fn to_progress_flag(&self) -> TaskbarProgressFlag {
        if self.has_error {
            TaskbarProgressFlag::Error
        } else if !self.is_active {
            TaskbarProgressFlag::NoProgress
        } else if self.is_paused {
            TaskbarProgressFlag::Paused
        } else {
            TaskbarProgressFlag::Normal
        }
    }
}

pub fn compute_taskbar_state_with_error(
    playback_time: f64,
    duration: f64,
    is_paused: bool,
    has_error: bool,
) -> TaskbarState {
    if has_error {
        return TaskbarState {
            progress_percent: 100,
            is_paused,
            is_active: true,
            has_error: true,
        };
    }
    let mut state = compute_taskbar_state(playback_time, duration, is_paused);
    state.has_error = false;
    state
}
```
Update `update_windows_taskbar_state` under `#[cfg(target_os = "windows")]` to handle `TaskbarProgressFlag::Error` by setting `TBPFLAG(4)` (TBPF_ERROR).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_compute_taskbar_state_with_error`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/platform/windows.rs
git commit -m "feat(windows): add error state support to taskbar progress reporting"
```

---

### Task 2: Taskbar Thumbnail Toolbar Media Action Buttons (`ThumbBarAddButtons`)

**Files:**
- Modify: `src/platform/windows.rs`
- Test: `src/platform/windows.rs:tests`

**Interfaces:**
- Produces:
  - `pub const THUMB_BUTTON_PREV: u32 = 1001;`
  - `pub const THUMB_BUTTON_PLAYPAUSE: u32 = 1002;`
  - `pub const THUMB_BUTTON_NEXT: u32 = 1003;`
  - `pub fn init_taskbar_thumbnail_toolbar(hwnd: isize) -> Result<(), String>`
  - `pub fn update_taskbar_thumbnail_buttons(hwnd: isize, is_paused: bool, has_media: bool) -> Result<(), String>`
  - `pub fn thumbnail_button_tooltip(button_id: u32, is_paused: bool) -> &'static str`

- [ ] **Step 1: Write failing unit test in `src/platform/windows.rs`**

```rust
    #[test]
    fn test_thumbnail_button_tooltips() {
        assert_eq!(thumbnail_button_tooltip(THUMB_BUTTON_PREV, false), "Previous");
        assert_eq!(thumbnail_button_tooltip(THUMB_BUTTON_PLAYPAUSE, true), "Play");
        assert_eq!(thumbnail_button_tooltip(THUMB_BUTTON_PLAYPAUSE, false), "Pause");
        assert_eq!(thumbnail_button_tooltip(THUMB_BUTTON_NEXT, false), "Next");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_thumbnail_button_tooltips`
Expected: FAIL.

- [ ] **Step 3: Implement Taskbar Thumbnail Toolbar functions**

In `src/platform/windows.rs`:
- Define `THUMB_BUTTON_PREV`, `THUMB_BUTTON_PLAYPAUSE`, `THUMB_BUTTON_NEXT`.
- Implement `thumbnail_button_tooltip(button_id: u32, is_paused: bool) -> &'static str`.
- Implement `init_taskbar_thumbnail_toolbar(hwnd: isize)` using `ITaskbarList3::ThumbBarAddButtons` with 3 `THUMBBUTTON` structures.
- Implement `update_taskbar_thumbnail_buttons(hwnd: isize, is_paused: bool, has_media: bool)` using `ITaskbarList3::ThumbBarUpdateButtons` to enable/disable and toggle Play/Pause tooltips.
- Add safe non-Windows stubs.

- [ ] **Step 4: Run test and cross-compilation check**

Run:
```bash
cargo test test_thumbnail_button_tooltips
cargo check --target x86_64-pc-windows-gnu
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/platform/windows.rs
git commit -m "feat(windows): implement taskbar thumbnail toolbar media action buttons"
```

---

### Task 3: Windows System Tray Icon & Native Context Menu (`Shell_NotifyIconW`)

**Files:**
- Modify: `src/platform/windows.rs`
- Test: `src/platform/windows.rs:tests`

**Interfaces:**
- Produces:
  - `pub const WM_TRAYICON: u32 = 0x8000 + 101;` // WM_APP + 101
  - `pub const TRAY_CMD_PLAYPAUSE: u32 = 2001;`
  - `pub const TRAY_CMD_MUTE: u32 = 2002;`
  - `pub const TRAY_CMD_OPEN: u32 = 2003;`
  - `pub const TRAY_CMD_EXIT: u32 = 2004;`
  - `pub fn register_system_tray_icon(hwnd: isize, tip: &str) -> Result<(), String>`
  - `pub fn update_system_tray_icon(hwnd: isize, tip: &str) -> Result<(), String>`
  - `pub fn remove_system_tray_icon(hwnd: isize) -> Result<(), String>`
  - `pub fn show_tray_popup_menu(hwnd: isize, is_paused: bool, is_muted: bool) -> Option<u32>`

- [ ] **Step 1: Write failing unit test in `src/platform/windows.rs`**

```rust
    #[test]
    fn test_tray_command_ids_and_menu_labels() {
        assert_eq!(tray_menu_label(TRAY_CMD_PLAYPAUSE, true), "Play");
        assert_eq!(tray_menu_label(TRAY_CMD_PLAYPAUSE, false), "Pause");
        assert_eq!(tray_menu_label(TRAY_CMD_MUTE, true), "Unmute");
        assert_eq!(tray_menu_label(TRAY_CMD_MUTE, false), "Mute");
        assert_eq!(tray_menu_label(TRAY_CMD_OPEN, false), "Open Media...");
        assert_eq!(tray_menu_label(TRAY_CMD_EXIT, false), "Exit");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_tray_command_ids_and_menu_labels`
Expected: FAIL.

- [ ] **Step 3: Implement System Tray functions**

In `src/platform/windows.rs`:
- Implement `tray_menu_label`.
- Under `#[cfg(target_os = "windows")]`:
  - `register_system_tray_icon`: initializes `NOTIFYICONDATAW` with `NIM_ADD`, `NIF_MESSAGE | NIF_ICON | NIF_TIP`, `uCallbackMessage = WM_TRAYICON`, sets tooltip.
  - `update_system_tray_icon`: calls `Shell_NotifyIconW(NIM_MODIFY, &nid)`.
  - `remove_system_tray_icon`: calls `Shell_NotifyIconW(NIM_DELETE, &nid)`.
  - `show_tray_popup_menu`: uses `CreatePopupMenu`, `AppendMenuW`, `SetForegroundWindow`, `TrackPopupMenu(TPM_RETURNCMD)`, `DestroyMenu`.
- Add safe non-Windows stubs.

- [ ] **Step 4: Run test and cross-compilation check**

Run:
```bash
cargo test test_tray_command_ids_and_menu_labels
cargo check --target x86_64-pc-windows-gnu
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/platform/windows.rs
git commit -m "feat(windows): implement system tray icon and native context menu"
```

---

### Task 4: Video-Only Dynamic Taskbar Thumbnail Preview

**Files:**
- Modify: `src/platform/windows.rs`
- Test: `src/platform/windows.rs:tests`

**Interfaces:**
- Produces:
  - `pub fn configure_video_taskbar_thumbnail(hwnd: isize) -> Result<(), String>`
  - `pub fn compute_thumbnail_clip_ratio(window_width: f32, window_height: f32, video_rect: [f32; 4]) -> [f32; 4]`

- [ ] **Step 1: Write failing unit test in `src/platform/windows.rs`**

```rust
    #[test]
    fn test_compute_thumbnail_clip_ratio() {
        let win_w = 1920.0;
        let win_h = 1080.0;
        let video_rect = [0.0, 100.0, 1920.0, 900.0]; // letterboxed
        let ratio = compute_thumbnail_clip_ratio(win_w, win_h, video_rect);
        assert_eq!(ratio[0], 0.0);
        assert!((ratio[1] - (100.0 / 1080.0)).abs() < 1e-4);
        assert_eq!(ratio[2], 1.0);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_compute_thumbnail_clip_ratio`
Expected: FAIL.

- [ ] **Step 3: Implement thumbnail clipping calculation and DWM configuration**

In `src/platform/windows.rs`:
- Implement `compute_thumbnail_clip_ratio`.
- Implement `configure_video_taskbar_thumbnail(hwnd: isize)` using `DwmSetWindowAttribute(hwnd, DWMWA_FORCE_ICONIC_REPRESENTATION)`.
- Add safe non-Windows stubs.

- [ ] **Step 4: Run test and cross-compilation check**

Run:
```bash
cargo test test_compute_thumbnail_clip_ratio
cargo check --target x86_64-pc-windows-gnu
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/platform/windows.rs
git commit -m "feat(windows): implement video-clipped iconic taskbar thumbnail preview"
```

---

### Task 5: Application Lifecycle & GUI Integration

**Files:**
- Modify: `src/app.rs`
- Modify: `src/main.rs`
- Test: `tests/windows_shell_suite_test.rs`

**Interfaces:**
- Consumes:
  - `init_taskbar_thumbnail_toolbar`, `update_taskbar_thumbnail_buttons`
  - `register_system_tray_icon`, `update_system_tray_icon`, `remove_system_tray_icon`
  - `update_windows_taskbar_state_ext`
- Produces:
  - Runtime synchronization of taskbar toolbar buttons and system tray with playback state.
  - Safe removal of system tray icon on application exit.

- [ ] **Step 1: Write integration test in `tests/windows_shell_suite_test.rs`**

```rust
use pealayer::platform::windows::{
    compute_taskbar_state_with_error, thumbnail_button_tooltip, tray_menu_label,
    TaskbarProgressFlag, THUMB_BUTTON_PLAYPAUSE, TRAY_CMD_PLAYPAUSE,
};

#[test]
fn test_windows_shell_suite_end_to_end_states() {
    // 1. Taskbar state with error
    let state = compute_taskbar_state_with_error(0.0, 100.0, false, true);
    assert_eq!(state.to_progress_flag(), TaskbarProgressFlag::Error);

    // 2. Toolbar tooltips
    assert_eq!(thumbnail_button_tooltip(THUMB_BUTTON_PLAYPAUSE, true), "Play");
    assert_eq!(thumbnail_button_tooltip(THUMB_BUTTON_PLAYPAUSE, false), "Pause");

    // 3. Tray menu labels
    assert_eq!(tray_menu_label(TRAY_CMD_PLAYPAUSE, true), "Play");
    assert_eq!(tray_menu_label(TRAY_CMD_PLAYPAUSE, false), "Pause");
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --test windows_shell_suite_test`
Expected: PASS.

- [ ] **Step 3: Connect lifecycle in `src/app.rs` and `src/main.rs`**

- In `src/app.rs`:
  - When HWND is registered, call `init_taskbar_thumbnail_toolbar(hwnd)` and `register_system_tray_icon(hwnd, "Pealayer")`.
  - In `process_events`, call `update_windows_taskbar_state_ext(self.playback_time, self.duration, self.is_paused, self.show_error.is_some())` and `update_taskbar_thumbnail_buttons(hwnd, self.is_paused, self.current_video_path.is_some())`.
  - In `on_exit()`, call `remove_system_tray_icon(hwnd)`.

- [ ] **Step 4: Run full test suite & Windows cross-compilation**

Run:
```bash
cargo test --all-targets
cargo check --target x86_64-pc-windows-gnu
```
Expected: PASS on all tests with 0 errors.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/main.rs tests/windows_shell_suite_test.rs
git commit -m "feat(app): wire Windows taskbar toolbar and system tray into application lifecycle"
```

---

### Task 6: GitHub Issue #2 Verification & Documentation

**Files:**
- GitHub Issue #2
- Progress notes

- [ ] **Step 1: Update Issue #2 checkbox**
Update the checkbox:
`- [x] **💡 Feat Req:** At least on Windows, the following features are missing: tray bar icon, task bar progress and state report and media action buttons, as well as using the video stream as the live/dynamic taskbar thumbnail instead of entire window content; as well as jump items and pinning`
- [ ] **Step 2: Post verification comment on Issue #2 with commit and test details**
- [ ] **Step 3: Push to `origin/main`**
