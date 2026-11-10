//! Curated non-secret settings, backed by the application's one config owner.
//! Neither shell text nor remote clients can enable their own execution grants.
use serde_json::{Value, json};
pub const KEYS: &[&str] = &[
    "theme",
    "color_palette",
    "accent_color",
    "ui_control_density",
    "ui.toolbar.enabled",
    "ui.terminal.enabled",
    "ui.terminal.font_size",
    "ui.terminal.scrollback_lines",
    "ui.terminal.cursor_style",
    "ui.terminal.cursor_blink",
    "ui.terminal.accent_commands",
];
fn at<'a>(root: &'a Value, key: &str) -> Option<&'a Value> {
    key.split('.').try_fold(root, |value, part| value.get(part))
}
fn display(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}
pub fn snapshot(config: &crate::config::AppConfig) -> Vec<(String, String)> {
    let root = serde_json::to_value(config).expect("configuration serialization");
    KEYS.iter()
        .filter_map(|key| Some(((*key).into(), display(at(&root, key)?))))
        .collect()
}
pub fn patch(
    config: &crate::config::AppConfig,
    key: &str,
    text: &str,
    expected: Option<&str>,
) -> Result<Value, String> {
    if !KEYS.contains(&key) {
        return Err("Setting is unavailable in the shell. Security, executable paths and private values require native Preferences.".into());
    }
    let mut root = serde_json::to_value(config).map_err(|e| e.to_string())?;
    let old = at(&root, key).ok_or("Setting is unavailable")?;
    if expected.is_some_and(|expected| expected != display(old)) {
        return Err("Setting changed elsewhere; refreshed the table. Retry your edit.".into());
    }
    let value = if old.is_string() {
        Value::String(text.into())
    } else {
        serde_json::from_str(text).map_err(|_| "Enter a valid number or true/false")?
    };
    if value.is_object() || value.is_array() || value.is_null() {
        return Err("Enter a scalar setting value".into());
    }
    let mut parts = key.split('.').peekable();
    let first = parts.peek().unwrap().to_string();
    let mut cursor = &mut root;
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            *cursor.get_mut(part).ok_or("Setting disappeared")? = value.clone();
        } else {
            cursor = cursor.get_mut(part).ok_or("Setting disappeared")?;
        }
    }
    let patch = json!({first.clone():root[&first]});
    config.apply_patch(&patch)?;
    Ok(patch)
}
#[cfg(test)]
mod tests {
    use super::*;
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
    }
}
