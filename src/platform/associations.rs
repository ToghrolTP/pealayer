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

#[cfg(target_os = "linux")]
pub fn register_file_associations(_exe_path: Option<&std::path::Path>) -> Result<usize, String> {
    crate::platform::association::register_as_default_player()?;
    Ok(SUPPORTED_EXTENSIONS.len())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
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

#[cfg(target_os = "linux")]
pub fn unregister_file_associations() -> Result<usize, String> {
    if let Ok(home) = std::env::var("HOME") {
        let desktop = std::path::PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("applications")
            .join("pealayer.desktop");
        if desktop.exists() {
            let _ = std::fs::remove_file(desktop);
        }
    }
    Ok(SUPPORTED_EXTENSIONS.len())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
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
