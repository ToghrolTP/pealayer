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
    let path = resolved_icon_path(config, state)?;
    std::fs::read(&path).ok().map(|bytes| (path, bytes))
}

pub fn icon_image(config: &AppConfig, state: PlaybackIconState) -> Option<image::DynamicImage> {
    icon_bytes(config, state).and_then(|(_, bytes)| image::load_from_memory(&bytes).ok())
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
    let config = crate::platform::interop::get_live_config();
    let key = resolved_icon_path(&config, state)
        .map(|path| format!("{}:{}", state.as_str(), path.display()))
        .unwrap_or_else(|| format!("{}:<builtin>", state.as_str()));
    let changed = ctx.data_mut(|data| {
        let id = egui::Id::new("pealayer-playback-window-icon");
        if data.get_temp::<String>(id).as_deref() == Some(&key) {
            false
        } else {
            data.insert_temp(id, key);
            true
        }
    });
    if !changed {
        return;
    }

    let icon = icon_bytes(&config, state)
        .and_then(|(_, bytes)| icon_data_from_bytes(&bytes))
        .or_else(|| {
            eframe::icon_data::from_png_bytes(include_bytes!("../assets/pealayer-icon.png")).ok()
        });
    if let Some(icon) = icon {
        ctx.send_viewport_cmd_to(
            egui::ViewportId::ROOT,
            egui::ViewportCommand::Icon(Some(std::sync::Arc::new(icon))),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
