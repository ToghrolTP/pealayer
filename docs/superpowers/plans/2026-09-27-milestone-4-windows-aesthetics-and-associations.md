# Milestone 4: Windows Aesthetics & File Associations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement native Windows dark title bar and Mica/Acrylic backdrop styling via Desktop Window Manager (DWM), alongside robust Windows file association registration/unregistration for all supported media types with CLI and API integration.

**Architecture:** 
1. Use Windows DWM attributes (`DwmSetWindowAttribute`) on HWND registration to configure immersive dark mode (`DWMWA_USE_IMMERSIVE_DARK_MODE`), system backdrop (`DWMWA_SYSTEMBACKDROP_TYPE` = Mica / Mica Alt / Acrylic), and caption color matching the player's palette (`#212121`).
2. Provide a unified `src/platform/associations.rs` module that configures registry keys under `HKCU\Software\Classes` using `winreg` and notifies the Windows shell via `SHChangeNotify` (`SHCNE_ASSOCCHANGED`).
3. Expose `--register-associations` and `--unregister-associations` CLI actions and an API callable from settings/UI.

**Tech Stack:** Rust 2024 edition, `windows 0.58` (`Win32_Graphics_Dwm`, `Win32_UI_Shell`, `Win32_Foundation`), `winreg 0.56`.

**Spec:** GitHub Issue #2 ("Player UX feedback" - Milestone 4: Windows Aesthetics & File Associations)

## Global Constraints
- Target platform: Windows (Win10 build 18985+, Win11) with safe no-op stubs on Linux/macOS.
- Code must compile with 0 warnings/errors on both `x86_64-unknown-linux-gnu` and `x86_64-pc-windows-gnu`.
- Supported media extensions: `mp4`, `mkv`, `avi`, `mov`, `webm`, `flv`, `wmv`, `ts`, `m3u8`, `mp3`, `flac`, `wav`, `aac`, `ogg`.
- Zero unnecessary dependencies; registry operations must be scoped to `HKCU` (Current User) so administrator elevation is never required.

---

### Task 1: Windows DWM Aesthetics (Dark Mode & Mica/Acrylic Backdrops)

**Files:**
- Modify: `src/platform/windows.rs`
- Modify: `src/app.rs`
- Test: `src/platform/windows.rs` (unit tests)

**Interfaces:**
- Produces:
  ```rust
  pub fn apply_windows_window_decorations(hwnd: isize);
  ```

- [ ] **Step 1: Write unit tests for decoration configuration in src/platform/windows.rs**

In `src/platform/windows.rs`:
```rust
#[test]
fn test_decoration_colorref_conversion() {
    // RGB(33, 33, 33) => COLORREF 0x00212121
    let r: u32 = 33;
    let g: u32 = 33;
    let b: u32 = 33;
    let colorref = r | (g << 8) | (b << 16);
    assert_eq!(colorref, 0x00212121);
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test test_decoration_colorref_conversion`
Expected: PASS

- [ ] **Step 3: Implement apply_windows_window_decorations in src/platform/windows.rs**

In `src/platform/windows.rs`:
```rust
#[cfg(target_os = "windows")]
pub fn apply_windows_window_decorations(hwnd_raw: isize) {
    use windows::Win32::Foundation::{BOOL, HWND};
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute,
        DWMWA_CAPTION_COLOR,
        DWMWA_SYSTEMBACKDROP_TYPE,
        DWMWA_TEXT_COLOR,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
        DWMSBT_MAINWINDOW,
    };
    use windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE;

    if hwnd_raw == 0 {
        return;
    }
    let hwnd = HWND(hwnd_raw as *mut _);

    unsafe {
        // 1. Enable immersive dark mode (attribute 20, fallback 19 for older Win10 builds)
        let dark_mode = BOOL::from(true);
        if DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark_mode as *const _ as *const _,
            std::mem::size_of::<BOOL>() as u32,
        ).is_err() {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWINDOWATTRIBUTE(19),
                &dark_mode as *const _ as *const _,
                std::mem::size_of::<BOOL>() as u32,
            );
        }

        // 2. Set Mica backdrop on Windows 11 (build 22621+ attribute 38 = DWMSBT_MAINWINDOW)
        let backdrop = DWMSBT_MAINWINDOW.0 as u32;
        if DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        ).is_err() {
            // Fallback for Windows 11 22000: DWMWA_MICA_EFFECT = 1029
            let mica_legacy = BOOL::from(true);
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWINDOWATTRIBUTE(1029),
                &mica_legacy as *const _ as *const _,
                std::mem::size_of::<BOOL>() as u32,
            );
        }

        // 3. Caption Color: #212121 (RGB 33, 33, 33 -> COLORREF 0x00212121)
        let caption_color: u32 = 0x00212121;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR,
            &caption_color as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );

        // 4. Text Color: White (0x00FFFFFF)
        let text_color: u32 = 0x00FFFFFF;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TEXT_COLOR,
            &text_color as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn apply_windows_window_decorations(_hwnd_raw: isize) {
    // No-op on non-Windows platforms
}
```
And inside `register_window_hwnd`:
```rust
pub fn register_window_hwnd(hwnd: isize) {
    WINDOW_HWND.store(hwnd, Ordering::SeqCst);
    apply_windows_window_decorations(hwnd);
}
```

- [ ] **Step 4: Run cross-check and unit tests**

Run: `cargo test` and `cargo check --target x86_64-pc-windows-gnu`
Expected: PASS on both targets.

---

### Task 2: File Associations Infrastructure (`src/platform/associations.rs`)

**Files:**
- Create: `src/platform/associations.rs`
- Modify: `src/platform/mod.rs`
- Test: `src/platform/associations.rs`

**Interfaces:**
- Produces:
  ```rust
  pub const SUPPORTED_EXTENSIONS: &[&str] = &[...];
  pub fn register_file_associations(exe_path: Option<&std::path::Path>) -> Result<usize, String>;
  pub fn unregister_file_associations() -> Result<usize, String>;
  pub fn is_file_association_registered(ext: &str) -> bool;
  ```

- [ ] **Step 1: Write unit tests in src/platform/associations.rs**

Create `src/platform/associations.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions_format() {
        assert!(SUPPORTED_EXTENSIONS.contains(&"mp4"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"mkv"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"mp3"));
        for ext in SUPPORTED_EXTENSIONS {
            assert!(!ext.starts_with('.'), "Extension '{}' should not have leading dot", ext);
            assert_eq!(*ext, ext.to_lowercase(), "Extension '{}' should be lowercase", ext);
        }
    }

    #[test]
    fn test_progid_command_generation() {
        let path = std::path::Path::new(r"C:\Program Files\Pealayer\pealayer.exe");
        let cmd = format!("\"{}\" \"%1\"", path.display());
        assert_eq!(cmd, r#""C:\Program Files\Pealayer\pealayer.exe" "%1""#);
    }
}
```

- [ ] **Step 2: Run test to verify it fails/passes**

Run: `cargo test test_supported_extensions_format`
Expected: PASS once constants are defined.

- [ ] **Step 3: Implement associations module**

In `src/platform/associations.rs`:
```rust
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "webm", "flv", "wmv", "ts", "m3u8",
    "mp3", "flac", "wav", "aac", "ogg"
];

pub const PROGID: &str = "Pealayer.Media";
pub const PROGID_DESCRIPTION: &str = "Pealayer Media File";

#[cfg(target_os = "windows")]
pub fn register_file_associations(exe_path: Option<&std::path::Path>) -> Result<usize, String> {
    use winreg::enums::*;
    use winreg::RegKey;
    use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};

    let current_exe = match exe_path {
        Some(p) => p.to_path_buf(),
        None => std::env::current_exe().map_err(|e| format!("Failed to get current executable path: {}", e))?,
    };
    let exe_str = current_exe.to_str().ok_or("Invalid executable path string")?;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes_key = hkcu.open_subkey_with_flags(r"Software\Classes", KEY_ALL_ACCESS)
        .or_else(|_| hkcu.create_subkey(r"Software\Classes").map(|(k, _)| k))
        .map_err(|e| format!("Failed to open HKCU\\Software\\Classes: {}", e))?;

    // 1. Create ProgID: Software\Classes\Pealayer.Media
    let (progid_key, _) = classes_key.create_subkey(PROGID)
        .map_err(|e| format!("Failed to create ProgID key: {}", e))?;
    let _ = progid_key.set_value("", &PROGID_DESCRIPTION);

    // DefaultIcon
    let (icon_key, _) = progid_key.create_subkey("DefaultIcon")
        .map_err(|e| format!("Failed to create DefaultIcon key: {}", e))?;
    let icon_val = format!("\"{}\",0", exe_str);
    let _ = icon_key.set_value("", &icon_val);

    // shell\open\command
    let (cmd_key, _) = progid_key.create_subkey(r"shell\open\command")
        .map_err(|e| format!("Failed to create shell\\open\\command key: {}", e))?;
    let cmd_val = format!("\"{}\" \"%1\"", exe_str);
    let _ = cmd_key.set_value("", &cmd_val);

    // 2. Associate supported extensions
    let mut count = 0;
    for ext in SUPPORTED_EXTENSIONS {
        let ext_sub = format!(".{}", ext);
        if let Ok((ext_key, _)) = classes_key.create_subkey(&ext_sub) {
            let _ = ext_key.set_value("", &PROGID);
            if let Ok((openwith_key, _)) = ext_key.create_subkey("OpenWithProgids") {
                let _ = openwith_key.set_value(PROGID, &"");
            }
            count += 1;
        }
    }

    // 3. Notify Windows Shell
    unsafe {
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    }

    Ok(count)
}

#[cfg(not(target_os = "windows"))]
pub fn register_file_associations(_exe_path: Option<&std::path::Path>) -> Result<usize, String> {
    Ok(SUPPORTED_EXTENSIONS.len())
}

#[cfg(target_os = "windows")]
pub fn unregister_file_associations() -> Result<usize, String> {
    use winreg::enums::*;
    use winreg::RegKey;
    use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes_key = hkcu.open_subkey_with_flags(r"Software\Classes", KEY_ALL_ACCESS)
        .map_err(|e| format!("Failed to open HKCU\\Software\\Classes: {}", e))?;

    // 1. Remove extension bindings
    let mut count = 0;
    for ext in SUPPORTED_EXTENSIONS {
        let ext_sub = format!(".{}", ext);
        if let Ok(ext_key) = classes_key.open_subkey_with_flags(&ext_sub, KEY_ALL_ACCESS) {
            if let Ok(val) = ext_key.get_value::<String, _>("") {
                if val == PROGID {
                    let _ = ext_key.delete_value("");
                }
            }
            if let Ok(openwith_key) = ext_key.open_subkey_with_flags("OpenWithProgids", KEY_ALL_ACCESS) {
                let _ = openwith_key.delete_value(PROGID);
            }
            count += 1;
        }
    }

    // 2. Delete ProgID tree
    let _ = classes_key.delete_subkey_all(PROGID);

    // 3. Notify Windows Shell
    unsafe {
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    }

    Ok(count)
}

#[cfg(not(target_os = "windows"))]
pub fn unregister_file_associations() -> Result<usize, String> {
    Ok(SUPPORTED_EXTENSIONS.len())
}

#[cfg(target_os = "windows")]
pub fn is_file_association_registered(ext: &str) -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let clean_ext = ext.trim_start_matches('.');
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(format!(r"Software\Classes\.{}", clean_ext)) {
        if let Ok(val) = key.get_value::<String, _>("") {
            if val == PROGID {
                return true;
            }
        }
    }
    false
}

#[cfg(not(target_os = "windows"))]
pub fn is_file_association_registered(_ext: &str) -> bool {
    false
}
```

Expose in `src/platform/mod.rs`:
```rust
pub mod associations;
```

- [ ] **Step 4: Run tests and cross-compilation check**

Run: `cargo test` and `cargo check --target x86_64-pc-windows-gnu`
Expected: PASS

---

### Task 3: CLI Flags & Main Entry Integration

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Test: `src/cli.rs`

**Interfaces:**
- Updates `CliAction`:
  ```rust
  pub enum CliAction {
      RunGui(CliOptions),
      SendRemote(String),
      RegisterAssociations,
      UnregisterAssociations,
      PrintHelp(String),
      PrintVersion(String),
  }
  ```

- [ ] **Step 1: Add tests for CLI association flags**

In `src/cli.rs`:
```rust
#[test]
fn test_cli_association_flags() {
    let args_reg = vec!["pealayer".to_string(), "--register-associations".to_string()];
    assert_eq!(parse_cli_args(args_reg).unwrap(), CliAction::RegisterAssociations);

    let args_unreg = vec!["pealayer".to_string(), "--unregister-associations".to_string()];
    assert_eq!(parse_cli_args(args_unreg).unwrap(), CliAction::UnregisterAssociations);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_cli_association_flags`
Expected: FAIL with missing enum variants

- [ ] **Step 3: Implement CLI flags and main handling**

In `src/cli.rs`:
Add variants to `CliAction` and parse `--register-associations` / `--unregister-associations`.
Update `format_help_message()` with descriptions:
```
  --register-associations    Register Pealayer as the default handler for media files
  --unregister-associations  Unregister Pealayer file associations
```

In `src/main.rs`:
```rust
        Ok(crate::cli::CliAction::RegisterAssociations) => {
            match crate::platform::associations::register_file_associations(None) {
                Ok(count) => {
                    println!("Successfully registered Pealayer for {} media file types.", count);
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("Failed to register file associations: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Ok(crate::cli::CliAction::UnregisterAssociations) => {
            match crate::platform::associations::unregister_file_associations() {
                Ok(count) => {
                    println!("Successfully unregistered Pealayer media file associations ({} processed).", count);
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("Failed to unregister file associations: {}", e);
                    std::process::exit(1);
                }
            }
        }
```

- [ ] **Step 4: Run full test suite and smoke tests**

Run: `cargo test`
Run: `cargo check --target x86_64-pc-windows-gnu`
Run: `cargo run -- --register-associations`
Expected: PASS
