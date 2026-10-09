use crate::config::AppConfig;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackIconState {
    Stopped,
    Paused,
    Playing,
}

impl PlaybackIconState {
    pub fn from_player(has_media: bool, paused: bool, ended: bool) -> Self {
        if !has_media || ended {
            Self::Stopped
        } else if paused {
            Self::Paused
        } else {
            Self::Playing
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "stopped" => Some(Self::Stopped),
            "paused" => Some(Self::Paused),
            "playing" => Some(Self::Playing),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stopped => "stopped",
            Self::Paused => "paused",
            Self::Playing => "playing",
        }
    }
}

static CURRENT_STATE: AtomicU8 = AtomicU8::new(0);

pub fn set_current_state(state: PlaybackIconState) {
    CURRENT_STATE.store(
        match state {
            PlaybackIconState::Stopped => 0,
            PlaybackIconState::Paused => 1,
            PlaybackIconState::Playing => 2,
        },
        Ordering::Release,
    );
}

pub fn current_state() -> PlaybackIconState {
    match CURRENT_STATE.load(Ordering::Acquire) {
        1 => PlaybackIconState::Paused,
        2 => PlaybackIconState::Playing,
        _ => PlaybackIconState::Stopped,
    }
}

fn non_empty_path(value: Option<PathBuf>) -> Option<PathBuf> {
    value.filter(|path| !path.as_os_str().is_empty())
}

fn state_env(state: PlaybackIconState) -> &'static str {
    match state {
        PlaybackIconState::Stopped => "APP_ICON_STOPPED",
        PlaybackIconState::Paused => "APP_ICON_PAUSED",
        PlaybackIconState::Playing => "APP_ICON_PLAYING",
    }
}

/// Resolve a state-specific runtime icon. A missing state falls back to the
/// deployment's ordinary application icon, then the compiled Pealayer icon.
pub fn resolved_icon_path(config: &AppConfig, state: PlaybackIconState) -> Option<PathBuf> {
    non_empty_path(std::env::var_os(state_env(state)).map(PathBuf::from))
        .or_else(|| {
            non_empty_path(match state {
                PlaybackIconState::Stopped => config.app_icon_stopped.clone(),
                PlaybackIconState::Paused => config.app_icon_paused.clone(),
                PlaybackIconState::Playing => config.app_icon_playing.clone(),
            })
        })
        .or_else(|| crate::config::resolved_app_icon(config))
}

pub fn icon_bytes(config: &AppConfig, state: PlaybackIconState) -> Option<(PathBuf, Vec<u8>)> {
    if let Some(path) = resolved_icon_path(config, state)
        && let Ok(bytes) = std::fs::read(&path)
        && image::load_from_memory(&bytes).is_ok()
    {
        return Some((path, bytes));
    }
    let (name, bytes) = builtin_icon_bytes(config.app_icon_preset);
    Some((PathBuf::from(name), bytes.to_vec()))
}

/// Native Windows artwork stays multi-resolution ICO; other surfaces use PNG.
/// These assets are embedded so selecting a preset never creates a disk cache.
pub fn builtin_icon_bytes(preset: crate::config::AppIconPreset) -> (&'static str, &'static [u8]) {
    use crate::config::AppIconPreset;
    match preset {
        #[cfg(target_os = "windows")]
        AppIconPreset::Current => ("current.ico", include_bytes!("../assets/icon.ico")),
        #[cfg(target_os = "windows")]
        AppIconPreset::Classic => ("classic.ico", include_bytes!("../assets/icons/classic.ico")),
        #[cfg(not(target_os = "windows"))]
        AppIconPreset::Current => ("current.png", include_bytes!("../assets/pealayer-icon.png")),
        #[cfg(not(target_os = "windows"))]
        AppIconPreset::Classic => ("classic.png", include_bytes!("../assets/icons/classic.png")),
    }
}

pub fn icon_image(config: &AppConfig, state: PlaybackIconState) -> Option<image::DynamicImage> {
    icon_bytes(config, state).and_then(|(_, bytes)| image::load_from_memory(&bytes).ok())
}

/// Preserve native ICO masters; convert other artwork once to a shell-ready
/// multi-resolution ICO using high-quality, alpha-preserving resampling.
#[cfg(target_os = "windows")]
pub fn native_icon_bytes(config: &AppConfig, state: PlaybackIconState) -> Result<Vec<u8>, String> {
    let (_, bytes) = icon_bytes(config, state).ok_or("Application icon unavailable")?;
    if bytes.len() >= 6 && bytes[..4] == [0, 0, 1, 0] && u16::from_le_bytes([bytes[4], bytes[5]]) > 1 {
        return Ok(bytes);
    }
    let image = image::load_from_memory(&bytes).map_err(|error| error.to_string())?;
    let mut frames = Vec::new();
    for size in [16u32, 24, 32, 48, 64, 128, 256] {
        let resized = image.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
        let mut png = std::io::Cursor::new(Vec::new());
        resized.write_to(&mut png, image::ImageFormat::Png).map_err(|error| error.to_string())?;
        frames.push((size, png.into_inner()));
    }
    let mut ico = vec![0, 0, 1, 0];
    ico.extend((frames.len() as u16).to_le_bytes());
    let mut offset = 6 + frames.len() as u32 * 16;
    for (size, png) in &frames {
        ico.extend([*size as u8, *size as u8, 0, 0]);
        ico.extend(1u16.to_le_bytes());
        ico.extend(32u16.to_le_bytes());
        ico.extend((png.len() as u32).to_le_bytes());
        ico.extend(offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in frames { ico.extend(png); }
    Ok(ico)
}

pub fn icon_data_from_bytes(bytes: &[u8]) -> Option<egui::IconData> {
    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

pub fn icon_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "ico" => "image/x-icon",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/png",
    }
}

pub fn sync_native_window_icon(ctx: &egui::Context, state: PlaybackIconState) {
    set_current_state(state);
    let revision = crate::platform::interop::live_config_revision();
    let stamp = (state, revision);
    let already_synced = ctx.data_mut(|data| {
        data.get_temp::<(PlaybackIconState, u64)>(egui::Id::new(
            "pealayer-playback-window-icon-stamp",
        )) == Some(stamp)
    });
    if already_synced {
        return;
    }

    let config = crate::platform::interop::get_live_config();
    // Avoid decoding/rereading artwork for unrelated config changes (volume,
    // telemetry settings, etc.). File metadata also catches replacing artwork
    // at the same path when the configuration is next applied/reloaded.
    let source = icon_source_signature(&config, state);
    let changed = ctx.data_mut(|data| {
        let id = egui::Id::new("pealayer-playback-window-icon");
        if data.get_temp::<u64>(id) == Some(source) {
            false
        } else {
            data.insert_temp(id, source);
            true
        }
    });
    // A configuration revision does not necessarily change the resolved icon
    // (for example, changing the web sync cadence). Remember that revision
    // before returning so an unrelated config update cannot turn this back
    // into a per-frame configuration clone.
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new("pealayer-playback-window-icon-stamp"), stamp);
    });
    // Title-only changes still update the notification area's tooltip. Windows
    // copies the borrowed HWND icon; WM_SETICON updates it again after eframe
    // processes the viewport command below.
    let hwnd = crate::platform::windows::get_registered_hwnd();
    if hwnd != 0 {
        let _ = crate::platform::windows::update_system_tray_icon(
            hwnd, &crate::config::resolved_app_name(&config),
        );
    }
    if !changed {
        return;
    }

    let icon = icon_bytes(&config, state).and_then(|(_, bytes)| icon_data_from_bytes(&bytes))
        .or_else(|| {
            eframe::icon_data::from_png_bytes(include_bytes!("../assets/pealayer-icon.png")).ok()
        });
    if let Some(icon) = icon {
        let icon = std::sync::Arc::new(icon);
        let viewports = ctx.input(|input| input.raw.viewports.keys().copied().collect::<Vec<_>>());
        for viewport in viewports {
            ctx.send_viewport_cmd_to(viewport, egui::ViewportCommand::Icon(Some(icon.clone())));
        }
    }
}

fn icon_source_signature(config: &AppConfig, state: PlaybackIconState) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    format!("{:?}", config.app_icon_preset).hash(&mut hash);
    let path = resolved_icon_path(config, state);
    path.hash(&mut hash);
    if let Some(path) = path && let Ok(metadata) = std::fs::metadata(path) {
        metadata.len().hash(&mut hash);
        metadata.modified().ok().hash(&mut hash);
    }
    hash.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrelated_settings_do_not_force_icon_decoding() {
        let mut config = AppConfig::default();
        let before = icon_source_signature(&config, PlaybackIconState::Stopped);
        config.volume = 42.0;
        assert_eq!(before, icon_source_signature(&config, PlaybackIconState::Stopped));
        config.app_icon_preset = crate::config::AppIconPreset::Classic;
        assert_ne!(before, icon_source_signature(&config, PlaybackIconState::Stopped));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn shell_derivative_preserves_native_master_and_multiple_resolutions() {
        let config = AppConfig::default();
        let bytes = native_icon_bytes(&config, PlaybackIconState::Stopped).unwrap();
        assert_eq!(&bytes[..4], &[0, 0, 1, 0]);
        assert!(u16::from_le_bytes([bytes[4], bytes[5]]) > 1);
        assert!(image::load_from_memory(&bytes).is_ok());
    }

    #[test]
    fn builtin_presets_are_distinct_decodable_and_need_no_files() {
        use crate::config::AppIconPreset;
        let (current_name, current) = builtin_icon_bytes(AppIconPreset::Current);
        let (classic_name, classic) = builtin_icon_bytes(AppIconPreset::Classic);
        assert_ne!(current, classic);
        assert_ne!(current_name, classic_name);
        for bytes in [current, classic] {
            assert!(icon_data_from_bytes(bytes).is_some());
        }
        // Both surface formats are preserved, with release-safe native sizes.
        for (ico, png) in [
            (
                &include_bytes!("../assets/icon.ico")[..],
                &include_bytes!("../assets/pealayer-icon.png")[..],
            ),
            (
                &include_bytes!("../assets/icons/classic.ico")[..],
                &include_bytes!("../assets/icons/classic.png")[..],
            ),
        ] {
            let native = image::load_from_memory(ico).unwrap();
            let web = image::load_from_memory(png).unwrap();
            assert_eq!(native.width(), native.height());
            assert!(native.width() >= 128);
            assert_eq!((web.width(), web.height()), (512, 512));
            let count = u16::from_le_bytes([ico[4], ico[5]]);
            assert!(count > 1, "Windows icons must contain multiple resolutions");
        }
    }

    #[test]
    fn preset_round_trip_and_unknown_value_validation() {
        let mut config = AppConfig::default();
        assert_eq!(
            config.app_icon_preset,
            crate::config::AppIconPreset::Current
        );
        config.app_icon_preset = crate::config::AppIconPreset::Classic;
        let mut value = serde_json::to_value(&config).unwrap();
        assert_eq!(value["app_icon_preset"], "classic");
        let restored: AppConfig = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(restored.app_icon_preset, config.app_icon_preset);
        value["app_icon_preset"] = serde_json::json!("unknown");
        assert!(serde_json::from_value::<AppConfig>(value).is_err());
    }

    #[test]
    fn invalid_custom_icon_falls_back_to_selected_preset() {
        let config = AppConfig {
            app_icon: Some(PathBuf::from("missing-pealayer-test-icon.ico")),
            app_icon_preset: crate::config::AppIconPreset::Classic,
            ..AppConfig::default()
        };
        for state in [
            PlaybackIconState::Stopped,
            PlaybackIconState::Paused,
            PlaybackIconState::Playing,
        ] {
            let (name, bytes) = icon_bytes(&config, state).unwrap();
            let (expected_name, expected_bytes) = builtin_icon_bytes(config.app_icon_preset);
            assert_eq!(name, PathBuf::from(expected_name));
            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    fn custom_and_state_overrides_keep_precedence_over_bundled_choice() {
        let mut config = AppConfig {
            app_icon_preset: crate::config::AppIconPreset::Classic,
            app_icon: Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/icon.ico")),
            ..AppConfig::default()
        };
        let (_, bytes) = icon_bytes(&config, PlaybackIconState::Playing).unwrap();
        assert_eq!(bytes.as_slice(), &include_bytes!("../assets/icon.ico")[..]);
        config.app_icon_playing =
            Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/icons/classic.ico"));
        let (_, bytes) = icon_bytes(&config, PlaybackIconState::Playing).unwrap();
        assert_eq!(bytes.as_slice(), &include_bytes!("../assets/icons/classic.ico")[..]);
    }

    #[test]
    fn changing_builtin_preset_publishes_a_new_window_icon() {
        let _lock = crate::app::tests::lock_app_tests();
        let ctx = egui::Context::default();
        let mut config = AppConfig::default();
        crate::platform::interop::set_live_config(config.clone());
        let mut output = ctx.run_ui(egui::RawInput::default(), |_ui| {
            sync_native_window_icon(&ctx, PlaybackIconState::Stopped);
        });
        output.textures_delta.clear();
        config.app_icon_preset = crate::config::AppIconPreset::Classic;
        crate::platform::interop::set_live_config(config);
        let mut output = ctx.run_ui(egui::RawInput::default(), |_ui| {
            sync_native_window_icon(&ctx, PlaybackIconState::Stopped);
        });
        output.textures_delta.clear();
        assert!(output.viewport_output.values().any(|viewport| {
            viewport
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Icon(Some(_))))
        }));
        crate::platform::interop::set_live_config(AppConfig::default());
    }

    #[test]
    fn icon_sync_records_unrelated_config_revisions() {
        let _lock = crate::app::tests::lock_app_tests();
        let ctx = egui::Context::default();
        let mut config = crate::config::AppConfig::default();
        crate::platform::interop::set_live_config(config.clone());
        sync_native_window_icon(&ctx, PlaybackIconState::Stopped);

        config.web_sync_interval_ms = config.web_sync_interval_ms.saturating_add(1);
        crate::platform::interop::set_live_config(config);
        let revision = crate::platform::interop::live_config_revision();
        sync_native_window_icon(&ctx, PlaybackIconState::Stopped);

        let stamp = ctx.data_mut(|data| {
            data.get_temp::<(PlaybackIconState, u64)>(egui::Id::new(
                "pealayer-playback-window-icon-stamp",
            ))
        });
        assert_eq!(stamp, Some((PlaybackIconState::Stopped, revision)));
        crate::platform::interop::set_live_config(crate::config::AppConfig::default());
    }

    #[test]
    fn playback_state_is_unambiguous() {
        assert_eq!(
            PlaybackIconState::from_player(false, false, false),
            PlaybackIconState::Stopped
        );
        assert_eq!(
            PlaybackIconState::from_player(true, true, false),
            PlaybackIconState::Paused
        );
        assert_eq!(
            PlaybackIconState::from_player(true, false, false),
            PlaybackIconState::Playing
        );
        assert_eq!(
            PlaybackIconState::from_player(true, false, true),
            PlaybackIconState::Stopped
        );
    }

    #[test]
    fn missing_state_uses_the_base_icon() {
        let mut config = AppConfig::default();
        config.app_icon = Some(PathBuf::from("base.png"));
        assert_eq!(
            resolved_icon_path(&config, PlaybackIconState::Playing),
            config.app_icon
        );
    }
}
