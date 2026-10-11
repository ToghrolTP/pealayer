//! One preference-governed timing policy, including stable hover action slots.
use eframe::egui;
fn mode_id() -> egui::Id { egui::Id::new("ui-animation-mode") }
pub fn configure(ctx: &egui::Context, mode: crate::config::UiAnimationMode) {
    ctx.data_mut(|data| data.insert_temp(mode_id(), mode));
    sync(ctx);
}
pub fn sync(ctx: &egui::Context) {
    let Some(mode) = ctx.data(|data| data.get_temp::<crate::config::UiAnimationMode>(mode_id())) else { return; };
    let enabled = match mode {
        crate::config::UiAnimationMode::System => system_enabled(),
        crate::config::UiAnimationMode::Enabled => true,
        crate::config::UiAnimationMode::Disabled => false,
    };
    let duration = if enabled { 0.12 } else { 0.0 };
    if ctx.style_of(egui::Theme::Dark).animation_time != duration || ctx.style_of(egui::Theme::Light).animation_time != duration {
        ctx.all_styles_mut(|style| style.animation_time = duration);
    }
}
pub fn hover_alpha(ui: &egui::Ui, id: egui::Id, visible: bool) -> f32 {
    sync(ui.ctx());
    let theme = if ui.visuals().dark_mode { egui::Theme::Dark } else { egui::Theme::Light };
    ui.ctx().animate_bool_with_time(id, visible, ui.ctx().style_of(theme).animation_time)
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_modes_control_all_shared_hover_timings() {
        let ctx = egui::Context::default();
        configure(&ctx, crate::config::UiAnimationMode::Disabled);
        assert_eq!(ctx.style_of(egui::Theme::Dark).animation_time, 0.0);
        configure(&ctx, crate::config::UiAnimationMode::Enabled);
        assert_eq!(ctx.style_of(egui::Theme::Light).animation_time, 0.12);
    }
}
