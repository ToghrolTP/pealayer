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
pub mod workspace_profiles;

/// Configure desktop-style interaction defaults. Static captions are not
/// documents, so they should not expose a text-selection cursor or highlight
/// when the user is trying to click, drag, or operate the surrounding control.
/// Editable widgets (and any labels explicitly marked selectable) retain their
/// normal text-editing behavior.
pub fn configure_interaction_style(style: &mut eframe::egui::Style) {
    style.interaction.selectable_labels = false;
    style.interaction.multi_widget_text_select = false;
}

pub fn platform_accent_rgb(config: &crate::config::AppConfig) -> [u8; 3] {
    match config.accent_color {
        crate::config::AccentColor::PealayerGreen => [56, 210, 122],
        crate::config::AccentColor::WindowsBlue => [0, 120, 212],
        crate::config::AccentColor::MacosBlue => [10, 132, 255],
        crate::config::AccentColor::Custom => config
            .custom_accent_color
            .as_deref()
            .and_then(crate::config::parse_rgb_hex)
            .unwrap_or([0, 120, 212]),
        crate::config::AccentColor::System => crate::platform::windows::system_accent_color()
            .unwrap_or_else(|| {
                if cfg!(target_os = "macos") {
                    [10, 132, 255]
                } else {
                    [0, 120, 212]
                }
            }),
    }
}

fn accent_foreground(accent: eframe::egui::Color32) -> eframe::egui::Color32 {
    if (u32::from(accent.r()) * 299 + u32::from(accent.g()) * 587 + u32::from(accent.b()) * 114)
        > 150_000
    {
        eframe::egui::Color32::from_rgb(20, 20, 20)
    } else {
        eframe::egui::Color32::WHITE
    }
}

fn light_accent_fill(accent: eframe::egui::Color32) -> eframe::egui::Color32 {
    let tint = |channel: u8| ((u16::from(channel) * 28 + 255 * 72) / 100) as u8;
    eframe::egui::Color32::from_rgb(tint(accent.r()), tint(accent.g()), tint(accent.b()))
}

/// Install a restrained native desktop palette for both themes. The active
/// theme can change later without reconstructing widget styling, and both the
/// main window and independently hosted dialogs use this same function.
pub fn configure_native_visuals(ctx: &eframe::egui::Context, config: &crate::config::AppConfig) {
    use eframe::egui::{Color32, CornerRadius, Stroke, Theme, Visuals};

    let [red, green, blue] = platform_accent_rgb(config);
    let accent = Color32::from_rgb(red, green, blue);
    let accent_text = accent_foreground(accent);

    let mut dark = Visuals::dark();
    dark.panel_fill = Color32::from_rgb(32, 32, 32);
    dark.window_fill = Color32::from_rgb(36, 36, 36);
    dark.extreme_bg_color = Color32::from_rgb(24, 24, 24);
    dark.faint_bg_color = Color32::from_rgb(45, 45, 45);
    dark.weak_text_alpha = 0.74;
    dark.hyperlink_color = accent;
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
    light.weak_text_alpha = 0.76;
    light.hyperlink_color = accent;
    light.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(35, 42, 52));
    light.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(42, 50, 61));
    light.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(23, 29, 38));
    light.selection.bg_fill = accent;
    light.selection.stroke = Stroke::new(1.0_f32, accent_text);
    light.widgets.inactive.weak_bg_fill = Color32::from_rgb(251, 251, 251);
    light.widgets.inactive.bg_fill = Color32::from_rgb(251, 251, 251);
    light.widgets.hovered.weak_bg_fill = Color32::from_rgb(242, 242, 242);
    light.widgets.hovered.bg_fill = Color32::from_rgb(242, 242, 242);
    // egui also uses the active-widget foreground for `RichText::strong()`.
    // White contrast text on a saturated accent therefore made every strong
    // caption disappear on ordinary light surfaces. Keep the active surface a
    // pale accent tint and its foreground dark; selected items retain the full
    // accent plus `selection.stroke` contrast above.
    light.widgets.active.weak_bg_fill = light_accent_fill(accent);
    light.widgets.active.bg_fill = light_accent_fill(accent);
    light.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(23, 29, 38));
    light.window_corner_radius = CornerRadius::same(10);
    light.menu_corner_radius = CornerRadius::same(8);

    ctx.set_visuals_of(Theme::Dark, dark);
    ctx.set_visuals_of(Theme::Light, light);
    let mut style = (*ctx.global_style()).clone();
    configure_interaction_style(&mut style);
    ctx.set_global_style(style);
}

#[cfg(test)]
mod tests {
    use eframe::egui::Color32;

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
    fn platform_accent_is_visible() {
        assert_ne!(
            super::platform_accent_rgb(&crate::config::AppConfig::default()),
            [0, 0, 0]
        );
    }

    #[test]
    fn accent_foreground_stays_readable_for_every_built_in_accent() {
        assert_eq!(
            super::accent_foreground(Color32::from_rgb(56, 210, 122)),
            Color32::from_rgb(20, 20, 20)
        );
        assert_eq!(
            super::accent_foreground(Color32::from_rgb(0, 120, 212)),
            Color32::WHITE
        );
        assert_eq!(
            super::accent_foreground(Color32::from_rgb(10, 132, 255)),
            Color32::WHITE
        );
    }

    #[test]
    fn light_accent_surface_is_tinted_instead_of_saturated() {
        let fill = super::light_accent_fill(Color32::from_rgb(0, 120, 212));
        assert_eq!(fill, Color32::from_rgb(183, 217, 242));
    }

    #[test]
    fn web_and_native_accent_presets_are_kept_in_lockstep() {
        let web = include_str!("../../web_ui/src/App.tsx").to_ascii_lowercase();
        for color in ["#38d27a", "#0078d4", "#0a84ff"] {
            assert!(web.contains(color), "web accent preset {color} drifted");
        }
    }
}
