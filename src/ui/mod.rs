pub mod about;
pub mod audio;
pub mod board_info;
pub mod controls;
mod monitor_controls;
pub mod color_picker;
pub mod dialog;
pub mod effects_library;
mod elegance_theme;
pub use elegance_theme::sync_elegance_theme;
pub mod error;
pub mod four_d;
pub mod group_picker;
pub mod hardware_control;
pub mod i18n;
pub mod icons;
pub mod layout;
mod timeline_toolbar;
pub mod media_inspector;
pub mod media_track_properties;
pub mod media_tracks;
pub mod menu;
pub mod open_url;
pub mod remote_location;
pub mod rf;
pub mod palette;
pub mod preferences;
pub mod peer_browser;
pub mod seek_preview;
pub mod status_bar;
pub mod subtitles;
pub mod toasts;
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

fn dark_accent_fill(
    accent: eframe::egui::Color32,
    surface: eframe::egui::Color32,
) -> eframe::egui::Color32 {
    let tint =
        |channel: u8, base: u8| ((u16::from(channel) * 28 + u16::from(base) * 72) / 100) as u8;
    eframe::egui::Color32::from_rgb(
        tint(accent.r(), surface.r()),
        tint(accent.g(), surface.g()),
        tint(accent.b(), surface.b()),
    )
}

/// Keep System as a preference, not a snapshot of the current OS scheme.
/// eframe/winit supplies native ThemeChanged events to egui's system_theme;
/// selecting an explicit theme here would stop following those events.
pub fn configure_native_appearance(ctx: &eframe::egui::Context, config: &crate::config::AppConfig) {
    ctx.set_theme(match crate::config::resolved_theme(config) {
        crate::config::AppTheme::System => eframe::egui::ThemePreference::System,
        crate::config::AppTheme::Light => eframe::egui::ThemePreference::Light,
        crate::config::AppTheme::Dark => eframe::egui::ThemePreference::Dark,
    });
    configure_native_visuals(ctx, config);
    sync_native_window_appearance(ctx, config.color_palette);
}

/// Call after native theme events as well as preference changes. The platform
/// setter caches applied values, so unchanged frames do not issue DWM calls.
pub fn sync_native_window_appearance(
    ctx: &eframe::egui::Context,
    palette: crate::config::ColorPalette,
) {
    crate::platform::windows::set_window_appearance(
        ctx.theme() == eframe::egui::Theme::Dark,
        palette,
    );
}

/// Apply theme-independent desktop defaults to both styles, so a live OS
/// switch cannot reset font sizes or re-enable selection of static captions.
pub fn configure_main_window_style(ctx: &eframe::egui::Context) {
    ctx.all_styles_mut(|style| {
        for font_id in style.text_styles.values_mut() {
            if font_id.size > 12.0 {
                font_id.size = 12.0;
            }
        }
    });
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
    // egui uses the active foreground for ordinary bold captions too. A
    // bright accent's dark contrast text must only be used on selected fills,
    // never on dark panels. Tint active controls and keep their text light.
    let dark_active = dark_accent_fill(
        accent,
        palette::color(config.color_palette, true, "surface-2"),
    );
    dark.widgets.active.weak_bg_fill = dark_active;
    dark.widgets.active.bg_fill = dark_active;
    dark.widgets.active.fg_stroke =
        Stroke::new(1.0_f32, palette::color(config.color_palette, true, "text"));
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
    light.widgets.active.fg_stroke =
        Stroke::new(1.0_f32, palette::color(config.color_palette, false, "text"));
    light.window_corner_radius = CornerRadius::same(10);
    light.menu_corner_radius = CornerRadius::same(8);

    palette::apply(&mut dark, config.color_palette);
    palette::apply(&mut light, config.color_palette);
    ctx.set_visuals_of(Theme::Dark, dark);
    ctx.set_visuals_of(Theme::Light, light);
    ctx.all_styles_mut(configure_interaction_style);
    elegance_theme::set_source_palette(ctx, config.color_palette);
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
    fn system_theme_events_switch_live_without_losing_palette_or_desktop_defaults() {
        use crate::config::{AppConfig, AppTheme, ColorPalette};
        use eframe::egui::{Context, RawInput, Theme, ThemePreference};

        for palette in [ColorPalette::Native, ColorPalette::Studio] {
            let config = AppConfig {
                color_palette: palette,
                ..Default::default()
            };
            let ctx = Context::default();
            super::configure_native_appearance(&ctx, &config);
            super::configure_main_window_style(&ctx);
            for theme in [Theme::Light, Theme::Dark, Theme::Light] {
                // This is the RawInput produced by egui-winit on ThemeChanged.
                ctx.run_ui(
                    RawInput {
                        system_theme: Some(theme),
                        ..Default::default()
                    },
                    |ui| {
                        let ctx = ui.ctx();
                        assert_eq!(ctx.theme(), theme);
                        assert_eq!(
                            ctx.options(|options| options.theme_preference),
                            ThemePreference::System
                        );
                        let style = ctx.global_style();
                        assert_eq!(
                            style.visuals.panel_fill,
                            super::palette::color(palette, theme == Theme::Dark, "surface-0")
                        );
                        assert!(!style.interaction.selectable_labels);
                        assert!(!style.interaction.multi_widget_text_select);
                        assert!(style.text_styles.values().all(|font| font.size <= 12.0));
                        let state = crate::platform::interop::AppearanceState::new(
                            &config,
                            theme == Theme::Dark,
                        );
                        assert_eq!(state.theme, AppTheme::System);
                        assert_eq!(
                            state.resolved_theme,
                            if theme == Theme::Dark {
                                AppTheme::Dark
                            } else {
                                AppTheme::Light
                            }
                        );
                    },
                )
                .drop_without_applying_deltas();
            }
            assert_eq!(config.theme, AppTheme::System);
        }
    }

    #[test]
    fn system_theme_events_respect_explicit_overrides_and_resume_when_system_is_selected() {
        use crate::config::{AppConfig, AppTheme};
        use eframe::egui::{Context, RawInput, Theme};

        let ctx = Context::default();
        for (preference, native, expected) in [
            (AppTheme::Light, Theme::Dark, Theme::Light),
            (AppTheme::Dark, Theme::Light, Theme::Dark),
            (AppTheme::System, Theme::Light, Theme::Light),
            (AppTheme::System, Theme::Dark, Theme::Dark),
        ] {
            super::configure_native_appearance(
                &ctx,
                &AppConfig {
                    theme: preference,
                    ..Default::default()
                },
            );
            ctx.run_ui(
                RawInput {
                    system_theme: Some(native),
                    ..Default::default()
                },
                |ui| {
                    let ctx = ui.ctx();
                    assert_eq!(ctx.theme(), expected);
                },
            )
            .drop_without_applying_deltas();
        }
    }

    #[test]
    fn system_theme_uses_framework_fallback_if_os_does_not_supply_a_scheme() {
        use eframe::egui::{Context, RawInput, ThemePreference};
        let ctx = Context::default();
        super::configure_native_appearance(&ctx, &crate::config::AppConfig::default());
        ctx.run_ui(RawInput::default(), |ui| {
            let ctx = ui.ctx();
            assert_eq!(ctx.system_theme(), None);
            assert_eq!(
                ctx.options(|options| options.theme_preference),
                ThemePreference::System
            );
            assert_eq!(ctx.theme(), ctx.options(|options| options.fallback_theme));
        })
        .drop_without_applying_deltas();
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
        let web = include_str!("../../web_ui/src/appearance.ts").to_ascii_lowercase();
        for color in ["#38d27a", "#0078d4", "#0a84ff"] {
            assert!(web.contains(color), "web accent preset {color} drifted");
        }
    }

    #[test]
    fn palette_accent_contrast_keeps_bold_captions_readable_on_both_themes() {
        use crate::config::{AccentColor, AppConfig, ColorPalette};
        use eframe::egui::{Context, Theme};
        fn luminance(color: Color32) -> f64 {
            let linear = |channel: u8| {
                let value = f64::from(channel) / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
        }
        let contrast = |text: Color32, fill: Color32| {
            let (a, b) = (luminance(text), luminance(fill));
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        };
        for palette in [ColorPalette::Native, ColorPalette::Studio] {
            for accent in [
                AccentColor::PealayerGreen,
                AccentColor::WindowsBlue,
                AccentColor::MacosBlue,
                AccentColor::Custom,
            ] {
                let ctx = Context::default();
                let config = AppConfig {
                    color_palette: palette,
                    accent_color: accent,
                    custom_accent_color: Some("#ffff00".into()),
                    ..Default::default()
                };
                super::configure_native_visuals(&ctx, &config);
                for theme in [Theme::Dark, Theme::Light] {
                    let style = ctx.style_of(theme);
                    let visuals = &style.visuals;
                    let text = visuals.strong_text_color();
                    for fill in [
                        visuals.panel_fill,
                        visuals.window_fill,
                        visuals.widgets.active.bg_fill,
                    ] {
                        assert!(
                            contrast(text, fill) >= 4.5,
                            "{palette:?}/{accent:?}/{theme:?}: {text:?} on {fill:?}"
                        );
                    }
                    if accent == AccentColor::PealayerGreen {
                        assert!(
                            contrast(visuals.selection.stroke.color, visuals.selection.bg_fill)
                                >= 4.5
                        );
                    }
                }
            }
        }
    }
}
