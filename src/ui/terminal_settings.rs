//! One real Preferences catalog, one config validator/persistence owner.
//! Shared shell views cannot disclose secrets or grant execution rights.
use crate::preferences_contract::{
    PreferenceControlKind as Kind, preference_controls, preference_sections, value_at_path,
};
use serde_json::{Value, json};
#[derive(Clone, Debug)]
pub struct Setting {
    pub key: String,
    pub label: String,
    pub section: String,
    pub page: String,
    pub group: String,
    pub value: String,
    pub kind: u32,
    pub choices: Vec<(String, String)>,
    pub read_only: bool,
}
fn protected(key: &str, kind: &Kind) -> bool {
    matches!(kind, Kind::File)
        || key == "open_url_proxy_url"
        || key.starts_with("ui.terminal.shell_")
        || matches!(
            key,
            "ui.terminal.controller_endpoint" | "ui.terminal.profile_id"
        )
}
fn policy(key: &str) -> bool {
    matches!(
        key,
        "ui.terminal.allow_system_shell"
            | "ui.terminal.allow_remote"
            | "ui.terminal.allow_remote_system_shell"
    )
}
fn display(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}
pub fn snapshot(config: &crate::config::AppConfig) -> Vec<Setting> {
    let root = serde_json::to_value(config).expect("configuration serialization");
    let sections = preference_sections();
    let mut result: Vec<Setting> = Vec::new();
    for control in preference_controls(config) {
        let Some(value) = value_at_path(&root, control.key) else {
            continue;
        };
        let private = protected(control.key, &control.kind);
        let text = if private {
            "Native Preferences only".into()
        } else {
            display(value)
        };
        let choices = control
            .options
            .iter()
            .filter(|option| !option.value.is_array() && !option.value.is_object())
            .map(|option| (display(&option.value), option.label.to_string()))
            .collect::<Vec<_>>();
        let choices = if choices.len() <= 64 {
            choices
        } else {
            Vec::new()
        };
        let page =
            if ["timeline_toolbar", "sidebar_rail", "workspace_tools"].contains(&control.section) {
                "Workspace"
            } else {
                sections
                    .iter()
                    .find(|section| section.id == control.section)
                    .map(|section| section.label)
                    .unwrap_or(control.section)
            };
        let complex = value.is_array() || value.is_object() || value.is_null();
        result.push(Setting {
            key: control.key.into(),
            label: control.label.into(),
            section: control.section.into(),
            page: page.into(),
            group: control.group.into(),
            kind: if private {
                4
            } else if value.is_boolean() {
                0
            } else if !choices.is_empty() && !complex {
                3
            } else if value.is_number() {
                5
            } else {
                4
            },
            choices,
            read_only: private || policy(control.key) || text.len() >= 192 || complex,
            value: if text.len() >= 192 {
                "Use get/set or native Preferences for this value".into()
            } else {
                text
            },
        });
        if let Some(key) = control.custom_key
            && !result.iter().any(|entry| entry.key == key)
            && let Some(value) = value_at_path(&root, key)
        {
            let text = display(value);
            let complex = value.is_array() || value.is_object();
            result.push(Setting {
                key: key.into(),
                label: format!(
                    "{} {}",
                    if matches!(control.kind, Kind::ActionList) {
                        "Hidden"
                    } else {
                        "Custom"
                    },
                    control.label.to_lowercase()
                ),
                section: control.section.into(),
                page: page.into(),
                group: control.group.into(),
                value: if text.len() >= 192 {
                    "Use get/set or native Preferences for this value".into()
                } else {
                    text.clone()
                },
                kind: 4,
                choices: Vec::new(),
                read_only: complex || text.len() >= 192,
            });
        }
    }
    result.sort_by_key(|entry| {
        sections
            .iter()
            .position(|section| section.id == entry.section)
            .unwrap_or(usize::MAX)
    });
    let mut keys = std::collections::HashSet::new();
    result.retain(|entry| keys.insert(entry.key.clone()));
    result
}
pub fn read(config: &crate::config::AppConfig, key: &str) -> Result<String, String> {
    let controls = preference_controls(config);
    let control = controls
        .iter()
        .find(|entry| entry.key == key || entry.custom_key == Some(key))
        .ok_or("Setting is not in Preferences")?;
    if protected(key, &control.kind) {
        return Err("This private setting is available only in native Preferences".into());
    }
    let root = serde_json::to_value(config).map_err(|e| e.to_string())?;
    value_at_path(&root, key)
        .map(display)
        .ok_or_else(|| "Setting is unavailable".into())
}
pub fn patch(
    config: &crate::config::AppConfig,
    key: &str,
    text: &str,
    expected: Option<&str>,
) -> Result<Value, String> {
    let controls = preference_controls(config);
    let control = controls
        .iter()
        .find(|entry| entry.key == key || entry.custom_key == Some(key))
        .ok_or("Setting is not in Preferences")?;
    if protected(key, &control.kind) || policy(key) {
        return Err("Private settings and execution grants require native Preferences".into());
    }
    let mut root = serde_json::to_value(config).map_err(|e| e.to_string())?;
    let old = value_at_path(&root, key).ok_or("Setting is unavailable")?;
    if expected.is_some_and(|expected| expected != display(old)) {
        return Err("Setting changed elsewhere; refreshed the table. Retry your edit.".into());
    }
    let option = if control.key == key {
        control
            .options
            .iter()
            .find(|option| display(&option.value) == text)
    } else {
        None
    };
    let value = if let Some(option) = option {
        option.value.clone()
    } else if old.is_string()
        || (old.is_null()
            && (control.custom_key == Some(key)
                || matches!(control.kind, Kind::Text | Kind::Color)))
    {
        Value::String(text.into())
    } else {
        serde_json::from_str(text).map_err(|_| "Enter valid JSON matching this setting's type")?
    };
    if let Some(number) = value.as_f64() {
        if !number.is_finite()
            || control.minimum.is_some_and(|min| number < min)
            || control.maximum.is_some_and(|max| number > max)
        {
            return Err("Value is outside this setting's Preferences range".into());
        }
    }
    if control.key == key
        && !control.options.is_empty()
        && matches!(
            control.kind,
            Kind::Select | Kind::ChoiceSlider | Kind::Accent
        )
        && !control.options.iter().any(|option| option.value == value)
    {
        return Err("Choose an advertised value for this setting".into());
    }
    crate::preferences_contract::set_value_at_path(&mut root, key, value)?;
    let first = key.split('.').next().ok_or("Invalid setting key")?;
    let patch = json!({first:root[first]});
    config.apply_patch(&patch)?;
    Ok(patch)
}
#[cfg(all(test, feature = "workspace-terminal"))]
mod tests {
    use super::*;
    #[test]
    fn catalog_uses_real_preferences_labels_choices_and_sections_without_secrets() {
        let config = crate::config::AppConfig::default();
        let entries = snapshot(&config);
        assert!(
            entries.len() > 150,
            "must not regress to a demonstration subset"
        );
        let palette = entries
            .iter()
            .find(|entry| entry.key == "color_palette")
            .unwrap();
        assert_eq!(palette.kind, 3);
        assert!(palette.choices.iter().any(|(value, _)| value == "studio"));
        assert!(entries.iter().any(|entry| entry.section == "playback"));
        assert!(
            entries
                .iter()
                .find(|entry| entry.key == "ui.terminal.allow_system_shell")
                .unwrap()
                .read_only
        );
        assert_eq!(
            entries
                .iter()
                .find(|entry| entry.key == "open_url_proxy_url")
                .unwrap()
                .value,
            "Native Preferences only"
        );
    }
    #[test]
    fn scalar_patch_preserves_ui_siblings_and_checks_observed_value() {
        let config = crate::config::AppConfig::default();
        let patch = patch(&config, "ui.terminal.cursor_blink", "false", Some("true")).unwrap();
        let updated = config.apply_patch(&patch).unwrap();
        assert!(!updated.ui.terminal.cursor_blink);
        assert_eq!(updated.ui.toolbar, config.ui.toolbar);
        assert!(super::patch(&config, "ui.terminal.cursor_blink", "false", Some("false")).is_err());
        assert!(super::patch(&config, "ui.terminal.allow_system_shell", "true", None).is_err());
        assert!(super::patch(&config, "open_url_proxy_url", "secret", None).is_err());
        assert!(super::patch(&config, "ui.terminal.font_size", "999", None).is_err());
        assert!(super::patch(&config, "color_palette", "invented", None).is_err());
    }
}
