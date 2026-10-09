use std::sync::{
    OnceLock, RwLock,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AudioDevice {
    pub name: String,
    pub description: String,
}

/// mpv device names include their backend (for example `wasapi/...`). Keep
/// that name intact rather than combining it with a separate `ao` override.
static DEVICES: OnceLock<RwLock<Vec<AudioDevice>>> = OnceLock::new();
static DISCOVERING: AtomicBool = AtomicBool::new(false);
pub fn available_audio_devices() -> Vec<AudioDevice> {
    if crate::mpv::external::active() {
        return crate::mpv::external::property("audio-device-list")
            .and_then(|v| serde_json::from_value(v).ok()).unwrap_or_else(default_devices);
    }
    DEVICES
        .get_or_init(|| RwLock::new(default_devices()))
        .read()
        .map(|v| v.clone())
        .unwrap_or_default()
}
pub fn refresh_devices() {
    if DISCOVERING.swap(true, Ordering::SeqCst) {
        return;
    }
    let devices = discover_devices();
    DISCOVERING.store(false, Ordering::SeqCst);
    let Some(devices) = devices else {
        return;
    };
    if let Ok(mut current) = DEVICES.get_or_init(|| RwLock::new(Vec::new())).write() {
        *current = devices;
    }
}
fn default_devices() -> Vec<AudioDevice> {
    vec![AudioDevice {
        name: "auto".to_string(),
        description: "System default".to_string(),
    }]
}
fn discover_devices() -> Option<Vec<AudioDevice>> {
    let mut devices = default_devices();
    let Ok(mpv) = libmpv2::Mpv::with_initializer(|init| {
        init.set_option("vo", "null")?;
        Ok(())
    }) else {
        return None;
    };
    let count = mpv
        .get_property::<i64>("audio-device-list/count")
        .ok()?
        .clamp(0, 128);
    for index in 0..count {
        let Ok(name) = mpv.get_property::<String>(&format!("audio-device-list/{index}/name"))
        else {
            continue;
        };
        if name.is_empty() || devices.iter().any(|device| device.name == name) {
            continue;
        }
        let description = mpv
            .get_property::<String>(&format!("audio-device-list/{index}/description"))
            .ok()
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| name.clone());
        devices.push(AudioDevice { name, description });
    }
    Some(devices)
}

pub fn select_device(mpv: &libmpv2::Mpv, selected: &str) -> Result<(), String> {
    let selected = if selected.trim().is_empty() {
        "auto"
    } else {
        selected
    };
    mpv.set_property("audio-device", selected)
        .map_err(|error| format!("select audio output {selected}: {error}"))
}
