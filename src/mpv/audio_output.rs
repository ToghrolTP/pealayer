use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioDevice {
    pub name: String,
    pub description: String,
}

/// mpv device names include their backend (for example `wasapi/...`). Keep
/// that name intact rather than combining it with a separate `ao` override.
pub fn available_audio_devices() -> &'static [AudioDevice] {
    static DEVICES: OnceLock<Vec<AudioDevice>> = OnceLock::new();
    DEVICES.get_or_init(|| {
        let mut devices = vec![AudioDevice {
            name: "auto".to_string(),
            description: "System default".to_string(),
        }];
        let Ok(mpv) = libmpv2::Mpv::with_initializer(|init| {
            init.set_option("vo", "null")?;
            Ok(())
        }) else {
            return devices;
        };
        let count = mpv
            .get_property::<i64>("audio-device-list/count")
            .unwrap_or_default()
            .clamp(0, 128);
        for index in 0..count {
            let Ok(name) = mpv.get_property::<String>(&format!("audio-device-list/{index}/name")) else {
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
        devices
    })
}

pub fn select_device(mpv: &libmpv2::Mpv, selected: &str) -> Result<(), String> {
    let selected = if selected.trim().is_empty() { "auto" } else { selected };
    mpv.set_property("audio-device", selected)
        .map_err(|error| format!("select audio output {selected}: {error}"))
}
