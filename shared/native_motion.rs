use super::UiAnimationMode;

pub fn duration(mode: UiAnimationMode) -> f32 {
    let enabled = match mode {
        UiAnimationMode::System => system_enabled(),
        UiAnimationMode::Enabled => true,
        UiAnimationMode::Disabled => false,
    };
    if enabled { 0.12 } else { 0.0 }
}

#[cfg(windows)]
fn system_enabled() -> bool {
    use std::sync::{LazyLock, Mutex};
    use std::time::{Duration, Instant};
    static CACHE: LazyLock<Mutex<Option<(Instant, bool)>>> = LazyLock::new(|| Mutex::new(None));
    let Ok(mut cache) = CACHE.lock() else { return false; };
    if let Some((at, value)) = *cache && at.elapsed() < Duration::from_secs(2) { return value; }
    use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS};
    let mut enabled = windows::core::BOOL(1);
    let result = unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0,
        Some((&mut enabled as *mut windows::core::BOOL).cast()), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)) };
    let value = result.is_ok() && enabled.as_bool();
    *cache = Some((Instant::now(), value));
    value
}
#[cfg(not(windows))]
fn system_enabled() -> bool { true }
