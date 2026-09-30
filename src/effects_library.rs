use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserStripEffectPreset {
    pub id: Uuid,
    pub name: String,
    pub category: String,
    pub hardware_effect_id: String,
    pub duration_ms: u64,
}

impl Default for UserStripEffectPreset {
    fn default() -> Self {
        Self { id: Uuid::new_v4(), name: String::new(), category: "Addressable strip".to_string(), hardware_effect_id: String::new(), duration_ms: 5_000 }
    }
}

pub fn path() -> PathBuf {
    crate::config::AppConfig::get_config_path().parent().map(|parent| parent.join("effects.json")).unwrap_or_else(|| PathBuf::from("effects.json"))
}

pub fn load_or_seed() -> Vec<UserStripEffectPreset> {
    let path = path();
    if !path.is_file() {
        if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
        let _ = std::fs::write(&path, include_str!("../assets/examples/effects.sample.json"));
    }
    std::fs::read_to_string(path).ok()
        .and_then(|json| serde_json::from_str::<Vec<UserStripEffectPreset>>(&json).ok())
        .unwrap_or_default().into_iter()
        .filter(|preset| !preset.name.trim().is_empty() && !preset.hardware_effect_id.trim().is_empty() && preset.duration_ms > 0)
        .collect()
}

pub fn save(presets: &[UserStripEffectPreset]) -> Result<(), String> {
    let path = path();
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|error| format!("create effect library directory: {error}"))?; }
    let json = serde_json::to_string_pretty(presets).map_err(|error| format!("encode effect library: {error}"))?;
    std::fs::write(&path, json).map_err(|error| format!("save effect library {}: {error}", path.display()))
}
