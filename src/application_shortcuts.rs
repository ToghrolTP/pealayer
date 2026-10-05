//! Persisted application accelerators, separate from global hardware bindings.
use crate::config::KeyChord;
use eframe::egui;
use global_hotkey::hotkey::{HotKey, Modifiers};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ApplicationShortcuts {
    pub fullscreen: String,
    pub media_information: String,
    pub media_folder: String,
    pub preferences: String,
    pub edit_config: String,
}

impl Default for ApplicationShortcuts {
    fn default() -> Self {
        Self {
            fullscreen: "F11".into(),
            media_information: "Shift+F10".into(),
            media_folder: "Ctrl+Shift+F10".into(),
            preferences: if cfg!(target_os = "macos") {
                "Cmd+,"
            } else {
                "Ctrl+,"
            }
            .into(),
            edit_config: if cfg!(target_os = "macos") {
                "Cmd+Shift+,"
            } else {
                "Ctrl+Shift+,"
            }
            .into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationAction {
    Fullscreen,
    MediaInformation,
    MediaFolder,
    Preferences,
    EditConfig,
}

pub fn parse_shortcut(value: &str) -> Result<Option<KeyChord>, String> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    let hotkey = value
        .trim()
        .parse::<HotKey>()
        .map_err(|error| error.to_string())?;
    let key = format!("{:?}", hotkey.key);
    if !egui::Key::ALL
        .iter()
        .any(|key_value| crate::hardware_shortcuts::egui_key_code(*key_value) == Some(key.as_str()))
    {
        return Err(format!("{key} is not supported by application shortcuts"));
    }
    Ok(Some(KeyChord {
        key,
        control: hotkey.mods.contains(Modifiers::CONTROL),
        shift: hotkey.mods.contains(Modifiers::SHIFT),
        alt: hotkey.mods.contains(Modifiers::ALT),
        super_key: hotkey.mods.contains(Modifiers::SUPER),
    }))
}

impl ApplicationShortcuts {
    pub fn entries(&self) -> [(ApplicationAction, &str); 5] {
        [
            (ApplicationAction::Fullscreen, &self.fullscreen),
            (ApplicationAction::MediaInformation, &self.media_information),
            (ApplicationAction::MediaFolder, &self.media_folder),
            (ApplicationAction::Preferences, &self.preferences),
            (ApplicationAction::EditConfig, &self.edit_config),
        ]
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut chords = std::collections::BTreeSet::new();
        for (action, value) in self.entries() {
            if let Some(chord) =
                parse_shortcut(value).map_err(|error| format!("{action:?}: {error}"))?
                && !chords.insert(chord)
            {
                return Err(
                    "application shortcuts must not assign the same chord to multiple actions"
                        .into(),
                );
            }
        }
        Ok(())
    }

    pub fn action_for_event(
        &self,
        event: &egui::Event,
        text_editing: bool,
    ) -> Option<ApplicationAction> {
        if !matches!(
            event,
            egui::Event::Key {
                pressed: true,
                repeat: false,
                ..
            }
        ) {
            return None;
        }
        self.entries().into_iter().find_map(|(action, value)| {
            let chord = parse_shortcut(value).ok().flatten()?;
            // Modified accelerators/function keys work during editing. A bare
            // letter rebound by the user must not intercept normal typing.
            if text_editing
                && !chord.control
                && !chord.super_key
                && !chord.alt
                && !chord.key.starts_with('F')
            {
                return None;
            }
            (crate::hardware_shortcuts::event_matches_chord(event, &chord) == Some(true))
                .then_some(action)
        })
    }
}

pub fn containing_media_folder(path: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let text = path.to_string_lossy();
    let local = if text.starts_with("file:") {
        url::Url::parse(&text)
            .map_err(|error| error.to_string())?
            .to_file_path()
            .map_err(|_| "The media file URL is not a local file path".to_owned())?
    } else if crate::media::is_remote_media_target(&text) {
        return Err("This media is a remote stream and has no local containing folder".into());
    } else {
        path.to_path_buf()
    };
    let absolute = if local.is_absolute() {
        local
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(local)
    };
    absolute
        .parent()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| "This media has no containing folder".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(key: egui::Key, modifiers: egui::Modifiers, repeat: bool, pressed: bool) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat,
            modifiers,
        }
    }
    #[test]
    fn application_shortcuts_use_exact_modifiers_and_ignore_repeat_and_release() {
        let bindings = ApplicationShortcuts::default();
        bindings.validate().unwrap();
        assert_eq!(
            bindings.action_for_event(
                &key(egui::Key::F11, egui::Modifiers::NONE, false, true),
                true
            ),
            Some(ApplicationAction::Fullscreen)
        );
        assert_eq!(
            bindings.action_for_event(
                &key(egui::Key::F10, egui::Modifiers::SHIFT, false, true),
                false
            ),
            Some(ApplicationAction::MediaInformation)
        );
        let mut modifiers = egui::Modifiers::SHIFT;
        modifiers.ctrl = true;
        assert_eq!(
            bindings.action_for_event(&key(egui::Key::F10, modifiers, false, true), false),
            Some(ApplicationAction::MediaFolder)
        );
        for event in [
            key(egui::Key::F10, egui::Modifiers::NONE, false, true),
            key(egui::Key::F11, egui::Modifiers::NONE, true, true),
            key(egui::Key::F11, egui::Modifiers::NONE, false, false),
        ] {
            assert_eq!(bindings.action_for_event(&event, false), None);
        }
    }
    #[test]
    fn application_shortcuts_are_configurable_persisted_and_validated() {
        let old: crate::config::AppConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(old.application_shortcuts, ApplicationShortcuts::default());
        let mut bindings = ApplicationShortcuts::default();
        bindings.fullscreen = "Alt+Enter".into();
        assert_eq!(
            bindings.action_for_event(
                &key(egui::Key::Enter, egui::Modifiers::ALT, false, true),
                false
            ),
            Some(ApplicationAction::Fullscreen)
        );
        assert_eq!(
            bindings.action_for_event(
                &key(egui::Key::F11, egui::Modifiers::NONE, false, true),
                false
            ),
            None
        );
        assert_eq!(
            serde_json::from_str::<ApplicationShortcuts>(
                &serde_json::to_string(&bindings).unwrap()
            )
            .unwrap(),
            bindings
        );
        bindings.fullscreen.clear();
        bindings.validate().unwrap();
        bindings.fullscreen = bindings.media_folder.clone();
        assert!(bindings.validate().is_err());
        bindings.fullscreen = "not-a-key".into();
        assert!(bindings.validate().is_err());
    }
    #[test]
    fn application_shortcuts_preserve_editing_and_use_native_preferences_modifiers() {
        let mut bindings = ApplicationShortcuts::default();
        let mut modifiers = egui::Modifiers::NONE;
        if cfg!(target_os = "macos") {
            modifiers.mac_cmd = true;
            modifiers.command = true;
        } else {
            modifiers.ctrl = true;
            modifiers.command = true;
        }
        assert_eq!(
            bindings.action_for_event(&key(egui::Key::Comma, modifiers, false, true), true),
            Some(ApplicationAction::Preferences)
        );
        modifiers.shift = true;
        assert_eq!(
            bindings.action_for_event(&key(egui::Key::Comma, modifiers, false, true), true),
            Some(ApplicationAction::EditConfig)
        );
        bindings.fullscreen = "G".into();
        assert_eq!(
            bindings.action_for_event(&key(egui::Key::G, egui::Modifiers::NONE, false, true), true),
            None
        );
        assert_eq!(
            bindings.action_for_event(
                &key(egui::Key::G, egui::Modifiers::NONE, false, true),
                false
            ),
            Some(ApplicationAction::Fullscreen)
        );
    }
    #[test]
    fn application_media_folder_is_local_only_and_handles_relative_paths() {
        assert!(
            containing_media_folder(std::path::Path::new("https://example.org/movie.mkv")).is_err()
        );
        assert_eq!(
            containing_media_folder(std::path::Path::new("movie.mkv")).unwrap(),
            std::env::current_dir().unwrap()
        );
    }
}
