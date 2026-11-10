# Portable & System Configuration Storage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement dynamic, zero-leak portable configuration storage and Windows Registry / Linux XDG system configuration storage with automatic session persistence for Pealayer (closing the remaining Application requirement on GitHub Issue #2).

**Architecture:** A unified `ConfigStorage` system in `src/config.rs` that detects portable mode next to the executable (`pealayer.json`, `portable.flag`, or `PEALAYER_PORTABLE=1`) vs. system mode. In portable mode, all settings and MRU recent media are stored strictly in local JSON without touching the host OS. In system mode, settings are stored in Windows Registry (`HKCU\Software\Pealayer\Settings` & `MRU`) with `%APPDATA%\pealayer\config.json` backup on Windows, and `$XDG_CONFIG_HOME/pealayer/config.json` on Linux. Application lifecycle (`on_exit`, `save`) ensures persistent settings (volume, mute, pin, remaining time, MRU) are automatically preserved across sessions.

**Tech Stack:** Rust 2021, `winreg` (Windows Registry), `serde`, `serde_json`, `egui`/`eframe`, `libmpv2`.

**Spec:** [GitHub Issue #2 ("Player UX feedback")](https://github.com/ToghrolTP/pealayer/issues/2) - Application section:
*"Dynamic/portable configuration storage (Windows and Linux), as well as using system storage (e.g. Registry on Windows); useful for persistent settings (e.g. volume) as well as recently opened media (e.g. MRU on Windows)"*

## Global Constraints

- Windows-only Registry APIs must be strictly gated under `#[cfg(target_os = "windows")]` with safe, zero-cost stubs on Linux and macOS.
- Portable mode must never leave registry keys, temporary files, or directory artifacts outside the portable root directory.
- Absolute paths must be resolved relative to the executable (`std::env::current_exe()`) rather than the volatile shell working directory (`std::env::current_dir()`).
- Serde representation must maintain backward and forward compatibility with optional/default fields (`#[serde(default)]`).
- All 91 existing unit/integration tests must continue passing (`cargo test --all-targets`).
- Clean cross-compilation for `x86_64-pc-windows-gnu` must be maintained (`cargo check --target x86_64-pc-windows-gnu`).

---

### Task 1: Storage Mode Detection & Executable Path Resolution

**Files:**
- Modify: `src/config.rs`
- Test: `src/config.rs:tests`

**Interfaces:**
- Produces:
  - `pub enum StorageMode { Auto, Portable, System }`
  - `pub fn detect_executable_dir() -> PathBuf`
  - `pub fn detect_storage_mode(exe_dir: &std::path::Path) -> StorageMode`
  - `pub fn resolve_portable_config_path(exe_dir: &std::path::Path) -> PathBuf`
  - `pub fn resolve_system_config_path() -> PathBuf`

- [ ] **Step 1: Write the failing tests in `src/config.rs`**

```rust
    #[test]
    fn test_detect_storage_mode_portable_flag() {
        let temp_dir = std::env::temp_dir().join(format!("pealayer_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("portable.flag"), "").unwrap();

        let mode = detect_storage_mode(&temp_dir);
        assert_eq!(mode, StorageMode::Portable);

        let portable_path = resolve_portable_config_path(&temp_dir);
        assert_eq!(portable_path, temp_dir.join("pealayer.json"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_detect_storage_mode_system_default() {
        let temp_dir = std::env::temp_dir().join(format!("pealayer_test_sys_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mode = detect_storage_mode(&temp_dir);
        assert_eq!(mode, StorageMode::System);

        let sys_path = resolve_system_config_path();
        assert!(sys_path.ends_with("config.json"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_detect_storage_mode`
Expected: FAIL with missing functions `detect_storage_mode`, `resolve_portable_config_path`, etc.

- [ ] **Step 3: Implement minimal storage mode and path resolution functions in `src/config.rs`**

Add `StorageMode` enum and detection functions:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageMode {
    Auto,
    Portable,
    System,
}

pub fn detect_executable_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

pub fn detect_storage_mode(exe_dir: &std::path::Path) -> StorageMode {
    if std::env::var("PEALAYER_PORTABLE").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false)
        || exe_dir.join("portable.flag").exists()
        || exe_dir.join("pealayer.json").exists()
        || exe_dir.join("portable.dat").exists()
    {
        StorageMode::Portable
    } else {
        StorageMode::System
    }
}

pub fn resolve_portable_config_path(exe_dir: &std::path::Path) -> PathBuf {
    exe_dir.join("pealayer.json")
}

pub fn resolve_system_config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("pealayer").join("config.json");
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            return PathBuf::from(userprofile)
                .join("AppData")
                .join("Roaming")
                .join("pealayer")
                .join("config.json");
        }
    }

    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("pealayer").join("config.json");
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("pealayer").join("config.json")
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_detect_storage_mode`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): implement portable mode detection and executable path resolution"
```

---

### Task 2: Windows Registry Storage Backend

**Files:**
- Create: `src/platform/registry.rs`
- Modify: `src/platform/mod.rs`
- Test: `src/platform/registry.rs:tests`

**Interfaces:**
- Consumes: `AppConfig` from `crate::config::AppConfig`
- Produces:
  - `pub fn save_settings_to_registry(cfg: &crate::config::AppConfig) -> Result<(), String>`
  - `pub fn load_settings_from_registry() -> Result<Option<crate::config::AppConfig>, String>`
  - `pub fn clear_registry_settings() -> Result<(), String>`

- [ ] **Step 1: Write the failing tests in `src/platform/registry.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_registry_conversion() {
        assert_eq!(volume_to_dword(100.0), 100);
        assert_eq!(volume_to_dword(0.0), 0);
        assert_eq!(volume_to_dword(150.0), 150);
        assert_eq!(dword_to_volume(100), 100.0);
        assert_eq!(dword_to_volume(50), 50.0);
    }

    #[test]
    fn test_mru_item_key_formatting() {
        assert_eq!(mru_key_name(0), "Item1");
        assert_eq!(mru_key_name(9), "Item10");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_volume_registry_conversion`
Expected: FAIL (module does not exist)

- [ ] **Step 3: Implement Windows Registry backend in `src/platform/registry.rs` and expose in `src/platform/mod.rs`**

```rust
use crate::config::AppConfig;
use std::path::PathBuf;

pub fn volume_to_dword(volume: f64) -> u32 {
    volume.round().clamp(0.0, 300.0) as u32
}

pub fn dword_to_volume(dword: u32) -> f64 {
    (dword as f64).clamp(0.0, 300.0)
}

pub fn mru_key_name(index: usize) -> String {
    format!("Item{}", index + 1)
}

#[cfg(target_os = "windows")]
pub fn save_settings_to_registry(cfg: &AppConfig) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (settings_key, _) = hkcu
        .create_subkey(r"Software\Pealayer\Settings")
        .map_err(|e| format!("Failed to open Settings registry key: {e}"))?;

    settings_key
        .set_value("Volume", &volume_to_dword(cfg.volume))
        .map_err(|e| format!("Failed to set Volume: {e}"))?;
    settings_key
        .set_value("IsMuted", &(cfg.is_muted as u32))
        .map_err(|e| format!("Failed to set IsMuted: {e}"))?;
    settings_key
        .set_value("PinControls", &(cfg.pin_controls as u32))
        .map_err(|e| format!("Failed to set PinControls: {e}"))?;
    settings_key
        .set_value("ShowRemainingTime", &(cfg.show_remaining_time as u32))
        .map_err(|e| format!("Failed to set ShowRemainingTime: {e}"))?;

    let (mru_key, _) = hkcu
        .create_subkey(r"Software\Pealayer\MRU")
        .map_err(|e| format!("Failed to open MRU registry key: {e}"))?;

    let count = cfg.recent_media.len().min(10) as u32;
    mru_key
        .set_value("Count", &count)
        .map_err(|e| format!("Failed to set MRU Count: {e}"))?;

    for (i, path) in cfg.recent_media.iter().take(10).enumerate() {
        let path_str = path.to_string_lossy();
        mru_key
            .set_value(&mru_key_name(i), &path_str.as_ref())
            .map_err(|e| format!("Failed to set MRU {}: {e}", mru_key_name(i)))?;
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn save_settings_to_registry(_cfg: &AppConfig) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn load_settings_from_registry() -> Result<Option<AppConfig>, String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let settings_key = match hkcu.open_subkey(r"Software\Pealayer\Settings") {
        Ok(k) => k,
        Err(_) => return Ok(None),
    };

    let volume: u32 = settings_key.get_value("Volume").unwrap_or(100);
    let is_muted: u32 = settings_key.get_value("IsMuted").unwrap_or(0);
    let pin_controls: u32 = settings_key.get_value("PinControls").unwrap_or(0);
    let show_remaining_time: u32 = settings_key.get_value("ShowRemainingTime").unwrap_or(0);

    let mut recent_media = Vec::new();
    if let Ok(mru_key) = hkcu.open_subkey(r"Software\Pealayer\MRU") {
        let count: u32 = mru_key.get_value("Count").unwrap_or(0);
        for i in 0..count.min(10) {
            if let Ok(item) = mru_key.get_value::<String, _>(&mru_key_name(i as usize)) {
                if !item.trim().is_empty() {
                    recent_media.push(PathBuf::from(item));
                }
            }
        }
    }

    Ok(Some(AppConfig {
        volume: dword_to_volume(volume),
        is_muted: is_muted != 0,
        pin_controls: pin_controls != 0,
        show_remaining_time: show_remaining_time != 0,
        recent_media,
    }))
}

#[cfg(not(target_os = "windows"))]
pub fn load_settings_from_registry() -> Result<Option<AppConfig>, String> {
    Ok(None)
}
```

- [ ] **Step 4: Run tests and cross-compile check**

Run:
```bash
cargo test test_volume_registry_conversion
cargo check --target x86_64-pc-windows-gnu
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/platform/registry.rs src/platform/mod.rs
git commit -m "feat(platform): implement Windows Registry settings and MRU storage backend"
```

---

### Task 3: Unified Dynamic & Portable Storage Manager

**Files:**
- Modify: `src/config.rs`
- Test: `src/config.rs:tests`

**Interfaces:**
- Consumes:
  - `StorageMode`, `detect_executable_dir`, `detect_storage_mode` from Task 1
  - `save_settings_to_registry`, `load_settings_from_registry` from Task 2
- Produces:
  - `AppConfig::load()` (supports portable vs system with Windows registry priority)
  - `AppConfig::save()` (writes to portable file or registry + system file)
  - `AppConfig::load_with_mode(mode: StorageMode, exe_dir: &Path)`
  - `AppConfig::save_with_mode(&self, mode: StorageMode, exe_dir: &Path)`

- [ ] **Step 1: Write the failing tests in `src/config.rs`**

```rust
    #[test]
    fn test_portable_save_and_load_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("pealayer_port_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut cfg = AppConfig::default();
        cfg.volume = 77.0;
        cfg.is_muted = true;
        cfg.pin_controls = true;
        cfg.show_remaining_time = true;
        cfg.recent_media.push(PathBuf::from("/media/video.mp4"));

        cfg.save_with_mode(StorageMode::Portable, &temp_dir);

        let portable_file = temp_dir.join("pealayer.json");
        assert!(portable_file.exists());

        let loaded = AppConfig::load_with_mode(StorageMode::Portable, &temp_dir);
        assert_eq!(loaded.volume, 77.0);
        assert!(loaded.is_muted);
        assert!(loaded.pin_controls);
        assert!(loaded.show_remaining_time);
        assert_eq!(loaded.recent_media, vec![PathBuf::from("/media/video.mp4")]);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_portable_save_and_load_roundtrip`
Expected: FAIL (missing `save_with_mode` / `load_with_mode`)

- [ ] **Step 3: Implement `save_with_mode`, `load_with_mode`, and update `save()` & `load()`**

In `src/config.rs`:
```rust
impl AppConfig {
    pub fn load_with_mode(mode: StorageMode, exe_dir: &std::path::Path) -> Self {
        match mode {
            StorageMode::Portable => {
                let path = resolve_portable_config_path(exe_dir);
                if path.exists() {
                    if let Ok(data) = std::fs::read_to_string(&path) {
                        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
                            return cfg;
                        }
                    }
                }
                Self::default()
            }
            StorageMode::System | StorageMode::Auto => {
                // On Windows, try reading from Registry first
                #[cfg(target_os = "windows")]
                {
                    if let Ok(Some(reg_cfg)) = crate::platform::registry::load_settings_from_registry() {
                        return reg_cfg;
                    }
                }

                // Fall back to system configuration file
                let path = resolve_system_config_path();
                if path.exists() {
                    if let Ok(data) = std::fs::read_to_string(&path) {
                        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
                            return cfg;
                        }
                    }
                }
                Self::default()
            }
        }
    }

    pub fn save_with_mode(&self, mode: StorageMode, exe_dir: &std::path::Path) {
        match mode {
            StorageMode::Portable => {
                let path = resolve_portable_config_path(exe_dir);
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Ok(json) = serde_json::to_string_pretty(self) {
                    let _ = std::fs::write(path, json);
                }
            }
            StorageMode::System | StorageMode::Auto => {
                // On Windows, save to Registry
                #[cfg(target_os = "windows")]
                {
                    let _ = crate::platform::registry::save_settings_to_registry(self);
                }

                // Save to system config file (as shadow backup / cross-platform standard)
                let path = resolve_system_config_path();
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Ok(json) = serde_json::to_string_pretty(self) {
                    let _ = std::fs::write(path, json);
                }
            }
        }
    }

    pub fn load() -> Self {
        let exe_dir = detect_executable_dir();
        let mode = detect_storage_mode(&exe_dir);
        Self::load_with_mode(mode, &exe_dir)
    }

    pub fn save(&self) {
        let exe_dir = detect_executable_dir();
        let mode = detect_storage_mode(&exe_dir);
        self.save_with_mode(mode, &exe_dir);
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_portable_save_and_load_roundtrip`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): integrate unified dynamic and portable configuration storage"
```

---

### Task 4: Application Lifecycle State Persistence Integration

**Files:**
- Modify: `src/app.rs`
- Modify: `src/main.rs`
- Test: `tests/config_persistence_test.rs`

**Interfaces:**
- Consumes:
  - `save_config()` in `PealayerApp`
- Produces:
  - Automatic invocation of `self.save_config()` on `eframe::App::save` and `eframe::App::on_exit`.
  - Integration test verifying settings persistence across simulated app restarts.

- [ ] **Step 1: Write integration test `tests/config_persistence_test.rs`**

```rust
use pealayer::config::{AppConfig, StorageMode};
use std::path::PathBuf;

#[test]
fn test_app_config_persistence_cycle() {
    let temp_dir = std::env::temp_dir().join(format!("pealayer_e2e_cfg_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Session 1: User modifies settings
    let mut session1_config = AppConfig::load_with_mode(StorageMode::Portable, &temp_dir);
    session1_config.volume = 125.0;
    session1_config.is_muted = true;
    session1_config.pin_controls = true;
    session1_config.show_remaining_time = true;
    session1_config.recent_media.push(PathBuf::from("/test/movie1.mkv"));
    session1_config.recent_media.push(PathBuf::from("/test/movie2.mp4"));

    session1_config.save_with_mode(StorageMode::Portable, &temp_dir);

    // Session 2: Fresh launch restores all settings
    let session2_config = AppConfig::load_with_mode(StorageMode::Portable, &temp_dir);
    assert_eq!(session2_config.volume, 125.0);
    assert!(session2_config.is_muted);
    assert!(session2_config.pin_controls);
    assert!(session2_config.show_remaining_time);
    assert_eq!(session2_config.recent_media.len(), 2);
    assert_eq!(session2_config.recent_media[0], PathBuf::from("/test/movie1.mkv"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --test config_persistence_test`
Expected: PASS

- [ ] **Step 3: Connect lifecycle hooks in `src/app.rs`**

In `src/app.rs`, inside `impl eframe::App for PealayerApp`:
```rust
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.save_config();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_config();
    }
```
And in `src/main.rs`:
Ensure `eframe::NativeOptions` sets `persist_window: true` or preserves configuration on exit.

- [ ] **Step 4: Run full test suite & Windows cross-compilation**

Run:
```bash
cargo test --all-targets
cargo check --target x86_64-pc-windows-gnu
```
Expected: PASS on all tests with zero errors.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/main.rs tests/config_persistence_test.rs
git commit -m "feat(app): connect lifecycle hooks to persist user settings on application exit"
```

---

### Task 5: Issue #2 Verification & GitHub Documentation

**Files:**
- GitHub Issue #2
- Progress notes

- [ ] **Step 1: Check GitHub Issue #2**
Verify all requirements for Dynamic/portable configuration storage and Windows Registry are satisfied.
- [ ] **Step 2: Update Issue #2 checkbox**
Update the checkbox:
`- [x] **💡 Feat Req:** Dynamic/portable configuration storage (Windows and LInux), as well as using system storage (e.g. Registry on Windows); useful for persistent settings (e.g. volume) as well as recently opened media (e.e MRU on Windows)`
- [ ] **Step 3: Post verification comment on Issue #2 with test evidence and commits**
- [ ] **Step 4: Push to `origin/main`**
