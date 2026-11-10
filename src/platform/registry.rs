use crate::config::AppConfig;
#[cfg(target_os = "windows")]
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
    use winreg::RegKey;
    use winreg::enums::*;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (settings_key, _) = hkcu
        .create_subkey(r"Software\Pealayer\Settings")
        .map_err(|e| format!("Failed to open Settings registry key: {e}"))?;

    let config_json = serde_json::to_string(cfg)
        .map_err(|error| format!("Failed to serialize complete settings: {error}"))?;
    settings_key
        .set_value("ConfigJson", &config_json)
        .map_err(|e| format!("Failed to set complete ConfigJson: {e}"))?;

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
    use winreg::RegKey;
    use winreg::enums::*;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let settings_key = match hkcu.open_subkey(r"Software\Pealayer\Settings") {
        Ok(k) => k,
        Err(_) => return Ok(None),
    };

    if let Ok(config_json) = settings_key.get_value::<String, _>("ConfigJson") {
        if let Ok(config) = serde_json::from_str::<AppConfig>(&config_json) {
            return Ok(Some(config));
        }
    }

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
        ..AppConfig::default()
    }))
}

#[cfg(not(target_os = "windows"))]
pub fn load_settings_from_registry() -> Result<Option<AppConfig>, String> {
    Ok(None)
}

#[cfg(target_os = "windows")]
pub fn clear_registry_settings() -> Result<(), String> {
    use winreg::RegKey;
    use winreg::enums::*;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(r"Software\Pealayer\Settings");
    let _ = hkcu.delete_subkey_all(r"Software\Pealayer\MRU");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn clear_registry_settings() -> Result<(), String> {
    Ok(())
}

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

    #[test]
    fn test_non_windows_registry_stubs() {
        #[cfg(not(target_os = "windows"))]
        let cfg = AppConfig::default();
        #[cfg(not(target_os = "windows"))]
        {
            assert!(save_settings_to_registry(&cfg).is_ok());
            assert!(load_settings_from_registry().unwrap().is_none());
            assert!(clear_registry_settings().is_ok());
        }
    }
}
