# Wayland Occlusion ANR Deadlock Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent Wayland EGL swap buffer deadlock and compositor ANR ("Application Not Responding") timeouts when switching focus or workspace.

**Architecture:** Detect native Wayland sessions at runtime without mutating process environments. On Wayland, set `glow_options.vsync = false` (`SwapInterval::DontWait`) to avoid blocking `eglSwapBuffers` on withheld `wl_surface.frame` callbacks when windows are occluded or on inactive workspaces. Drive presentation pacing through `display_refresh_rate`-based timer pacing (`PlaybackRepaintPacing::Paced`), preserving tear-free compositor delivery, event loop liveness, and prompt `xdg_wm_base.pong` responses.

**Tech Stack:** Rust 2024, `eframe` (Glow/egui), `glutin`, `winit`, Wayland `xdg-shell`.

**Spec:** Issue [#205](https://github.com/ToghrolTP/pealayer/issues/205).

## Global Constraints

- Alpha has one current API and persisted-data contract: do not add legacy schema migrations or speculative future-version handling.
- Operational fault tolerance: runtime safety checks for Wayland EGL driver behavior are permitted and encouraged.
- Isolation: unit tests must never mutate global process environment variables or point fixtures at the user's OS configuration.
- Preservation: do not break Windows WGL VSync or existing pacing tests.

---

### Task 1: Wayland Session Detection and Effective VSync Resolution Helper

**Files:**
- Create: `src/platform/linux.rs`
- Modify: `src/platform/mod.rs`
- Test: `src/platform/linux.rs` (in-file `tests` module)

**Interfaces:**
- Consumes: Environment queries via pure functional lookup `Fn(&str) -> Option<OsString>`.
- Produces:
  - `pub fn is_wayland_session_from_env<F: Fn(&str) -> Option<std::ffi::OsString>>(env_lookup: F) -> bool`
  - `pub fn is_wayland_session() -> bool`
  - `pub fn resolve_effective_opengl_vsync(configured_vsync: bool, is_wayland: bool) -> bool`

- [ ] **Step 1: Write the failing tests in `src/platform/linux.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::collections::HashMap;

    #[test]
    fn test_is_wayland_session_from_env() {
        let mut env = HashMap::new();
        let lookup = |k: &str| env.get(k).cloned();

        // 1. Neither variable set -> false
        assert!(!is_wayland_session_from_env(lookup));

        // 2. WAYLAND_DISPLAY set -> true
        env.insert("WAYLAND_DISPLAY".to_string(), OsString::from("wayland-1"));
        assert!(is_wayland_session_from_env(lookup));

        // 3. WINIT_UNIX_BACKEND=x11 override -> false even if WAYLAND_DISPLAY is present
        env.insert("WINIT_UNIX_BACKEND".to_string(), OsString::from("x11"));
        assert!(!is_wayland_session_from_env(lookup));

        // 4. Reset override, WAYLAND_SOCKET set -> true
        env.remove("WINIT_UNIX_BACKEND");
        env.remove("WAYLAND_DISPLAY");
        env.insert("WAYLAND_SOCKET".to_string(), OsString::from("3"));
        assert!(is_wayland_session_from_env(lookup));
    }

    #[test]
    fn test_resolve_effective_opengl_vsync() {
        // On non-Wayland (Windows, macOS, X11), configured value is preserved
        assert!(resolve_effective_opengl_vsync(true, false));
        assert!(!resolve_effective_opengl_vsync(false, false));

        // On Wayland, hardware swap wait is disabled to prevent occlusion deadlock
        assert!(!resolve_effective_opengl_vsync(true, true));
        assert!(!resolve_effective_opengl_vsync(false, true));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `PEALAYER_CONFIG_FILE=/tmp/test-config.json cargo test --lib platform::linux::tests -v`
Expected: FAIL with "module not found" or compilation error.

- [ ] **Step 3: Write minimal implementation in `src/platform/linux.rs` and wire into `src/platform/mod.rs`**

```rust
pub fn is_wayland_session_from_env<F: Fn(&str) -> Option<std::ffi::OsString>>(env_lookup: F) -> bool {
    #[cfg(target_os = "linux")]
    {
        if env_lookup("WINIT_UNIX_BACKEND").as_deref() == Some(std::ffi::OsStr::new("x11")) {
            return false;
        }
        env_lookup("WAYLAND_DISPLAY").is_some() || env_lookup("WAYLAND_SOCKET").is_some()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = env_lookup;
        false
    }
}

pub fn is_wayland_session() -> bool {
    is_wayland_session_from_env(|k| std::env::var_os(k))
}

pub fn resolve_effective_opengl_vsync(configured_vsync: bool, is_wayland: bool) -> bool {
    if is_wayland {
        false
    } else {
        configured_vsync
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `PEALAYER_CONFIG_FILE=/tmp/test-config.json cargo test --lib platform::linux::tests -v`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/platform/linux.rs src/platform/mod.rs
git commit -m "fix(platform): add Wayland session detection and effective VSync resolver"
```

---

### Task 2: Startup and Preferences Native Options Integration

**Files:**
- Modify: `src/startup.rs:295-298`
- Modify: `src/ui/preferences.rs:475-480`
- Test: `src/startup.rs` or integration test

**Interfaces:**
- Consumes: `crate::platform::linux::{is_wayland_session, resolve_effective_opengl_vsync}`
- Produces: NativeOptions with `glow_options.vsync` resolved through `resolve_effective_opengl_vsync`.

- [ ] **Step 1: Write unit test in `src/startup.rs` verifying Glow vsync options configuration**

```rust
#[test]
fn test_glow_vsync_options_resolution() {
    assert!(!crate::platform::linux::resolve_effective_opengl_vsync(true, true));
    assert!(crate::platform::linux::resolve_effective_opengl_vsync(true, false));
}
```

- [ ] **Step 2: Run test to verify it compiles and runs**

Run: `PEALAYER_CONFIG_FILE=/tmp/test-config.json cargo test --lib startup::tests::test_glow_vsync_options_resolution -v`
Expected: PASS.

- [ ] **Step 3: Update `src/startup.rs` and `src/ui/preferences.rs`**

Use `resolve_effective_opengl_vsync(launch_config.opengl_vsync, crate::platform::linux::is_wayland_session())` for `eframe::NativeOptions` initialization in both entry points.

- [ ] **Step 4: Verify with `cargo check` and run test suite**

Run: `cargo check`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/startup.rs src/ui/preferences.rs
git commit -m "fix(startup): use effective VSync options in eframe Glow initialization"
```

---

### Task 3: PealayerApp Playback Pacing Integration

**Files:**
- Modify: `src/app.rs` around line 5620-5630 and `effective_opengl_vsync`
- Test: `src/app.rs` around tests for `playback_repaint_pacing`

**Interfaces:**
- Consumes: `self.opengl_vsync` and `crate::platform::linux::{is_wayland_session, resolve_effective_opengl_vsync}`
- Produces: `effective_opengl_vsync(&self) -> bool` and updated `playback_repaint_pacing(&self)`.

- [ ] **Step 1: Write failing test in `src/app.rs` testing effective VSync and Wayland playback pacing**

```rust
#[test]
fn test_playback_repaint_pacing_on_wayland() {
    let mut app = PealayerApp::default();
    app.current_video_path = Some(std::path::PathBuf::from("test.mp4"));
    app.is_paused = false;
    app.is_eof = false;
    app.opengl_vsync = true;
    app.display_refresh_rate = 60.0;

    // Simulate non-wayland resolution
    assert_eq!(
        app.playback_repaint_pacing_resolved(false),
        Some(PlaybackRepaintPacing::VSync)
    );

    // On Wayland, pacing must adapt to display refresh rate rather than blocking hardware VSync
    match app.playback_repaint_pacing_resolved(true) {
        Some(PlaybackRepaintPacing::Paced(delay)) => {
            let ms = delay.as_secs_f64() * 1000.0;
            assert!((ms - 16.666).abs() < 0.1);
        }
        other => panic!("Expected Paced delay, got {other:?}"),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `PEALAYER_CONFIG_FILE=/tmp/test-config.json cargo test --lib app::tests::test_playback_repaint_pacing_on_wayland -v`
Expected: FAIL.

- [ ] **Step 3: Implement `effective_opengl_vsync` and `playback_repaint_pacing_resolved` in `src/app.rs`**

```rust
pub fn effective_opengl_vsync(&self) -> bool {
    crate::platform::linux::resolve_effective_opengl_vsync(
        self.opengl_vsync,
        crate::platform::linux::is_wayland_session(),
    )
}

pub fn playback_repaint_pacing_resolved(&self, is_wayland: bool) -> Option<PlaybackRepaintPacing> {
    if !self.is_active_playback() {
        return None;
    }
    if crate::platform::linux::resolve_effective_opengl_vsync(self.opengl_vsync, is_wayland) {
        Some(PlaybackRepaintPacing::VSync)
    } else {
        let target_hz = self.display_refresh_rate.clamp(20.0, 360.0);
        let target_interval = std::time::Duration::from_secs_f64(1.0 / target_hz);
        Some(PlaybackRepaintPacing::Paced(target_interval))
    }
}

pub fn playback_repaint_pacing(&self) -> Option<PlaybackRepaintPacing> {
    self.playback_repaint_pacing_resolved(crate::platform::linux::is_wayland_session())
}
```

- [ ] **Step 4: Run tests to verify all pass**

Run: `PEALAYER_CONFIG_FILE=/tmp/test-config.json cargo test --lib app::tests::test_playback_repaint_pacing_on_wayland -v`
Expected: PASS.
Run: `PEALAYER_CONFIG_FILE=/tmp/test-config.json cargo test --lib app::tests::test_playback_pacing_modes -v`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "fix(app): adapt playback repaint pacing to desktop refresh on Wayland"
```

---

### Task 4: Full Test Suite Verification and Pull Request

**Files:**
- Test: All unit and integration tests with `PEALAYER_CONFIG_FILE`
- Documentation: Verification evidence in PR body

- [ ] **Step 1: Run complete unit and integration tests**

Run: `PEALAYER_CONFIG_FILE=/tmp/test-config.json cargo test --lib`
Expected: All tests PASS with zero failures.

- [ ] **Step 2: Check git status and git diff**

Run: `git status` and `git diff` to ensure no stray or untracked temporary files exist.

- [ ] **Step 3: Push branch to GitHub**

Run: `git push -u origin fix/wayland-occlusion-anr-deadlock`

- [ ] **Step 4: Create GitHub Pull Request**

Create PR linking issue #205 with complete root cause, solution, and test verification details using `gh pr create`.
