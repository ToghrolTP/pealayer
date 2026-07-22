use std::path::PathBuf;

pub const SUPPORTED_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "webm", "mov", "flv", "mp3", "flac", "wav"];

pub fn register_as_default_player() -> Result<String, String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed getting executable path: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        register_windows_file_associations(&current_exe)?;
        return Ok("Successfully registered Pealayer in Windows Registry as default media player.".to_string());
    }

    #[cfg(target_os = "linux")]
    {
        register_linux_desktop_association(&current_exe)?;
        return Ok("Successfully created ~/.local/share/applications/pealayer.desktop and registered MIME associations.".to_string());
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err("Default player registration is not supported on this OS.".to_string())
    }
}

#[cfg(target_os = "windows")]
fn register_windows_file_associations(exe_path: &PathBuf) -> Result<(), String> {
    let exe_str = exe_path.to_str().ok_or("Invalid executable path string")?;
    let cmd = format!("\"{}\" \"%1\"", exe_str);

    // Write HKCU\Software\Classes\Pealayer.Media
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let (prog_key, _) = hkcu.create_subkey("Software\\Classes\\Pealayer.Media")
        .map_err(|e| format!("Registry error creating ProgID: {}", e))?;
    prog_key.set_value("", &"Pealayer Media File")
        .map_err(|e| format!("Registry error setting ProgID description: {}", e))?;

    let (shell_cmd_key, _) = hkcu.create_subkey("Software\\Classes\\Pealayer.Media\\shell\\open\\command")
        .map_err(|e| format!("Registry error creating shell command: {}", e))?;
    shell_cmd_key.set_value("", &cmd)
        .map_err(|e| format!("Registry error setting command: {}", e))?;

    // Associate extensions
    for ext in SUPPORTED_EXTENSIONS {
        let subkey_name = format!("Software\\Classes\\.{}", ext);
        if let Ok((ext_key, _)) = hkcu.create_subkey(&subkey_name) {
            let _ = ext_key.set_value("", &"Pealayer.Media");
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn register_linux_desktop_association(exe_path: &PathBuf) -> Result<(), String> {
    let exe_str = exe_path.to_str().ok_or("Invalid executable path string")?;
    let home = std::env::var("HOME").map_err(|_| "HOME directory not set")?;
    let apps_dir = PathBuf::from(home).join(".local").join("share").join("applications");
    let _ = std::fs::create_dir_all(&apps_dir);

    let desktop_file_path = apps_dir.join("pealayer.desktop");
    let content = format!(
        "[Desktop Entry]\n\
Type=Application\n\
Name=Pealayer\n\
Comment=Modern 4D Video & Haptic Player\n\
Exec=\"{}\" %f\n\
Terminal=false\n\
Categories=AudioVideo;Player;Video;\n\
MimeType=video/mp4;video/x-matroska;video/x-msvideo;video/webm;video/quicktime;video/x-flv;\n",
        exe_str
    );

    std::fs::write(&desktop_file_path, content)
        .map_err(|e| format!("Failed writing desktop entry: {}", e))?;

    let _ = std::process::Command::new("xdg-mime")
        .args(&["default", "pealayer.desktop", "video/mp4", "video/x-matroska", "video/webm"])
        .status();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions_format() {
        assert!(SUPPORTED_EXTENSIONS.contains(&"mp4"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"mkv"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"avi"));
    }
}
