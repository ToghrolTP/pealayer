pub mod about;
pub mod audio;
pub mod board_info;
pub mod controls;
pub mod dialog;
pub mod effects_library;
pub mod error;
pub mod four_d;
pub mod hardware_control;
pub mod i18n;
pub mod icons;
pub mod layout;
pub mod menu;
pub mod open_url;
pub mod preferences;
pub mod status_bar;
pub mod subtitles;
pub mod video;

/// Configure desktop-style interaction defaults. Static captions are not
/// documents, so they should not expose a text-selection cursor or highlight
/// when the user is trying to click, drag, or operate the surrounding control.
/// Editable widgets (and any labels explicitly marked selectable) retain their
/// normal text-editing behavior.
pub fn configure_interaction_style(style: &mut eframe::egui::Style) {
    style.interaction.selectable_labels = false;
    style.interaction.multi_widget_text_select = false;
}

pub fn platform_accent_color() -> eframe::egui::Color32 {
    crate::platform::windows::system_accent_color()
        .map(|[red, green, blue]| eframe::egui::Color32::from_rgb(red, green, blue))
        .unwrap_or_else(|| {
            if cfg!(target_os = "macos") {
                eframe::egui::Color32::from_rgb(10, 132, 255)
            } else {
                eframe::egui::Color32::from_rgb(0, 120, 212)
            }
        })
}

/// Install a restrained native desktop palette for both themes. The active
/// theme can change later without reconstructing widget styling, and both the
/// main window and independently hosted dialogs use this same function.
pub fn configure_native_visuals(ctx: &eframe::egui::Context) {
    use eframe::egui::{Color32, CornerRadius, Stroke, Theme, Visuals};

    let accent = platform_accent_color();
    let accent_text = if (u32::from(accent.r()) * 299
        + u32::from(accent.g()) * 587
        + u32::from(accent.b()) * 114)
        > 150_000
    {
        Color32::from_rgb(20, 20, 20)
    } else {
        Color32::WHITE
    };

    let mut dark = Visuals::dark();
    dark.panel_fill = Color32::from_rgb(32, 32, 32);
    dark.window_fill = Color32::from_rgb(36, 36, 36);
    dark.extreme_bg_color = Color32::from_rgb(24, 24, 24);
    dark.faint_bg_color = Color32::from_rgb(45, 45, 45);
    dark.selection.bg_fill = accent;
    dark.selection.stroke = Stroke::new(1.0_f32, accent_text);
    dark.widgets.inactive.weak_bg_fill = Color32::from_rgb(45, 45, 45);
    dark.widgets.inactive.bg_fill = Color32::from_rgb(45, 45, 45);
    dark.widgets.hovered.weak_bg_fill = Color32::from_rgb(55, 55, 55);
    dark.widgets.hovered.bg_fill = Color32::from_rgb(55, 55, 55);
    dark.widgets.active.weak_bg_fill = accent.gamma_multiply(0.82);
    dark.widgets.active.bg_fill = accent;
    dark.widgets.active.fg_stroke = Stroke::new(1.0_f32, accent_text);
    dark.window_corner_radius = CornerRadius::same(10);
    dark.menu_corner_radius = CornerRadius::same(8);

    let mut light = Visuals::light();
    light.panel_fill = Color32::from_rgb(243, 243, 243);
    light.window_fill = Color32::from_rgb(250, 250, 250);
    light.extreme_bg_color = Color32::WHITE;
    light.faint_bg_color = Color32::from_rgb(238, 238, 238);
    light.selection.bg_fill = accent;
    light.selection.stroke = Stroke::new(1.0_f32, accent_text);
    light.widgets.inactive.weak_bg_fill = Color32::from_rgb(251, 251, 251);
    light.widgets.inactive.bg_fill = Color32::from_rgb(251, 251, 251);
    light.widgets.hovered.weak_bg_fill = Color32::from_rgb(242, 242, 242);
    light.widgets.hovered.bg_fill = Color32::from_rgb(242, 242, 242);
    light.widgets.active.weak_bg_fill = accent.gamma_multiply(0.84);
    light.widgets.active.bg_fill = accent;
    light.widgets.active.fg_stroke = Stroke::new(1.0_f32, accent_text);
    light.window_corner_radius = CornerRadius::same(10);
    light.menu_corner_radius = CornerRadius::same(8);

    ctx.set_visuals_of(Theme::Dark, dark);
    ctx.set_visuals_of(Theme::Light, light);
    let mut style = (*ctx.global_style()).clone();
    configure_interaction_style(&mut style);
    style.spacing.item_spacing = eframe::egui::vec2(8.0, 7.0);
    style.spacing.button_padding = eframe::egui::vec2(10.0, 5.0);
    style.spacing.interact_size.y = 30.0;
    ctx.set_global_style(style);
}

#[cfg(test)]
mod tests {
    #[test]
    fn desktop_style_disables_accidental_caption_selection() {
        let mut style = eframe::egui::Style::default();
        style.interaction.selectable_labels = true;
        style.interaction.multi_widget_text_select = true;

        super::configure_interaction_style(&mut style);

        assert!(!style.interaction.selectable_labels);
        assert!(!style.interaction.multi_widget_text_select);
    }

    #[test]
    fn platform_accent_is_visible_and_opaque() {
        assert_eq!(super::platform_accent_color().a(), 255);
    }
}
