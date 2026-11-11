//! One preference-governed timing policy, including stable hover action slots.
use eframe::egui;
use crate::config::UiAnimationMode;

#[path = "../../shared/native_motion.rs"]
mod native;
fn mode_id() -> egui::Id { egui::Id::new("ui-animation-mode") }
pub fn configure(ctx: &egui::Context, mode: crate::config::UiAnimationMode) {
    ctx.data_mut(|data| data.insert_temp(mode_id(), mode));
    sync(ctx);
}
pub fn sync(ctx: &egui::Context) {
    let Some(mode) = ctx.data(|data| data.get_temp::<crate::config::UiAnimationMode>(mode_id())) else { return; };
    let duration = native::duration(mode);
    if ctx.style_of(egui::Theme::Dark).animation_time != duration || ctx.style_of(egui::Theme::Light).animation_time != duration {
        ctx.all_styles_mut(|style| style.animation_time = duration);
    }
}
pub fn hover_alpha(ui: &egui::Ui, id: egui::Id, visible: bool) -> f32 {
    sync(ui.ctx());
    let theme = if ui.visuals().dark_mode { egui::Theme::Dark } else { egui::Theme::Light };
    ui.ctx().animate_bool_with_time(id, visible, ui.ctx().style_of(theme).animation_time)
}
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
