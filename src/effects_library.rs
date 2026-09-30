use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct UserStripEffectPreset {
    pub id: Uuid,
    pub name: String,
    pub category: String,
    pub description: String,
    pub hardware_effect_id: String,
    pub duration_ms: u64,
}

impl Default for UserStripEffectPreset {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            category: "Addressable strip".to_string(),
            description: String::new(),
            hardware_effect_id: String::new(),
            duration_ms: 5_000,
        }
    }
}

impl UserStripEffectPreset {
    pub fn duplicate(&self) -> Self {
        let mut duplicate = self.clone();
        duplicate.id = Uuid::new_v4();
        duplicate.name = format!("{} copy", self.name);
        duplicate
    }
}

pub fn path() -> PathBuf {
    crate::config::AppConfig::get_config_path()
        .parent()
        .map(|parent| parent.join("effects.json"))
        .unwrap_or_else(|| PathBuf::from("effects.json"))
}

pub fn load_or_seed() -> Vec<UserStripEffectPreset> {
    let path = path();
    if !path.is_file() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(
            &path,
            include_str!("../assets/examples/effects.sample.json"),
        );
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|json| serde_json::from_str::<Vec<UserStripEffectPreset>>(&json).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|preset| {
            !preset.name.trim().is_empty()
                && !preset.hardware_effect_id.trim().is_empty()
                && preset.duration_ms > 0
        })
        .collect()
}

pub fn save(presets: &[UserStripEffectPreset]) -> Result<(), String> {
    let path = path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create effect library directory: {error}"))?;
    }
    let json = serde_json::to_string_pretty(presets)
        .map_err(|error| format!("encode effect library: {error}"))?;
    std::fs::write(&path, json)
        .map_err(|error| format!("save effect library {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_effect_files_gain_new_optional_fields_without_losing_identity() {
        let preset: UserStripEffectPreset = serde_json::from_str(
            r#"{"id":"129160c3-8bda-4e5e-b771-e9243d084ba8","name":"Police","category":"Lighting","hardware_effect_id":"police","duration_ms":5000}"#,
        )
        .expect("old effect JSON should remain readable");
        assert_eq!(preset.name, "Police");
        assert!(preset.description.is_empty());
    }

    #[test]
    fn duplicating_an_effect_preserves_content_but_allocates_a_new_id() {
        let original = UserStripEffectPreset {
            name: "Thunder".into(),
            description: "Sharp flash".into(),
            hardware_effect_id: "white-thunder".into(),
            ..Default::default()
        };
        let duplicate = original.duplicate();
        assert_ne!(duplicate.id, original.id);
        assert_eq!(duplicate.hardware_effect_id, original.hardware_effect_id);
        assert_eq!(duplicate.description, original.description);
    }
}
