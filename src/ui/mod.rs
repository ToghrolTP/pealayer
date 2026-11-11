pub mod about;
pub mod audio;
pub mod board_info;
pub mod controls;
pub(crate) mod mouse_buttons;
pub mod control_density;
mod monitor_controls;
pub mod color_picker;
pub mod craft_theme;
pub mod motion;
pub mod digital_twin;
pub mod dialog;
pub(crate) mod window_chrome;
pub mod dropdown;
pub mod effects_library;
pub(crate) mod strip_designer;
pub(crate) mod strip_layers;
pub(crate) mod pixel_diagnostic;
mod elegance_theme;
pub use elegance_theme::sync_elegance_theme;
pub mod error;
pub mod four_d;
pub mod group_picker;
pub mod hardware_control;
pub(crate) mod hardware_monitor;
pub(crate) mod pwm_policy;
pub mod i18n;
pub(crate) mod fonts;
pub(crate) mod numeric_text;
pub(crate) mod egui_dock;
pub(crate) mod panel_chrome;
pub(crate) mod player_mode;
pub mod icons;
pub(crate) mod keycaps;
pub mod layout;
mod timeline_toolbar;
pub(crate) mod zoom_shortcuts;
pub(crate) mod timeline_strip;
pub mod media_inspector;
pub mod media_track_properties;
pub mod media_tracks;
pub mod menu;
pub mod open_url;
pub mod remote_location;
pub mod rf;
pub mod palette;
pub mod preferences;
pub mod projects;
pub mod peer_browser;
pub mod seek_preview;
pub mod status_bar;
pub mod sidebar_rail;
pub mod sidebar_shortcuts;
pub mod workspace_toolbar;
pub mod terminal;
pub mod terminal_settings;
#[cfg(feature="workspace-terminal")]
pub mod terminal_remote;
#[cfg(feature = "workspace-terminal")]
mod terminal_backend;
#[cfg(feature = "workspace-terminal")]
mod portable_shell_host;
#[cfg(feature = "workspace-terminal")]
mod shell_status;
#[cfg(feature = "workspace-terminal")]
pub mod terminal_profiles;
#[cfg(feature = "workspace-terminal")]
mod terminal_screen;
#[cfg(feature = "workspace-terminal")]
mod terminal_parser;
pub(crate) mod action_customization;
pub(crate) mod control_bar;
pub(crate) mod simple_controls;
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
    palette::native::configure_interaction_style(style);
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

use palette::native::accent_foreground;

/// Keep System as a preference, not a snapshot of the current OS scheme.
/// eframe/winit supplies native ThemeChanged events to egui's system_theme;
/// selecting an explicit theme here would stop following those events.
pub fn configure_native_appearance(ctx: &eframe::egui::Context, config: &crate::config::AppConfig) {
    motion::configure(ctx, config.ui_animation_mode);
    control_bar::configure_idle_backgrounds(ctx, config.transport_idle_backgrounds);
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
    palette::native::configure_desktop_text_sizes(ctx);
}

/// Install a restrained native desktop palette for both themes. The active
/// theme can change later without reconstructing widget styling, and both the
/// main window and independently hosted dialogs use this same function.
pub fn configure_native_visuals(ctx: &eframe::egui::Context, config: &crate::config::AppConfig) {
    window_chrome::install(ctx);
    i18n::configure_preferred_fonts(ctx, config);
    color_picker::configure_language(ctx, config.language);
    use eframe::egui::{Color32, Theme};

    let [red, green, blue] = platform_accent_rgb(config);
    let accent = Color32::from_rgb(red, green, blue);
    let (dark, light) = palette::native::desktop_visuals(palette::name(config.color_palette), accent);
    let tokens_dark = craft_theme::Tokens::for_kind(craft_theme::ThemeKind::Dark)
        .with_appearance(Some([red, green, blue]), false).with_visuals(&dark);
    let tokens_light = craft_theme::Tokens::for_kind(craft_theme::ThemeKind::Light)
        .with_appearance(Some([red, green, blue]), false).with_visuals(&light);

    ctx.set_visuals_of(Theme::Dark, dark);
    ctx.set_visuals_of(Theme::Light, light);
    palette::native::controls::configure(ctx, config.color_palette == crate::config::ColorPalette::Pastel);
    ctx.all_styles_mut(configure_interaction_style);
    ctx.all_styles_mut(|style| control_density::configure(style, config.ui_control_density));
    craft_theme::set_themes(ctx, tokens_dark, tokens_light);
    elegance_theme::set_source_palette(ctx, config.color_palette);
    motion::configure(ctx, config.ui_animation_mode);
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
        let fill = super::palette::native::light_accent_fill(Color32::from_rgb(0, 120, 212));
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

    #[test]
    fn craft_tokens_follow_active_theme_and_preserve_each_pealayer_palette() {
        for palette in [crate::config::ColorPalette::Native, crate::config::ColorPalette::Studio] {
            let ctx = eframe::egui::Context::default();
            let config = crate::config::AppConfig { color_palette: palette, ..Default::default() };
            super::configure_native_appearance(&ctx, &config);
            for theme in [eframe::egui::Theme::Light, eframe::egui::Theme::Dark, eframe::egui::Theme::Light] {
                ctx.set_theme(theme);
                let tokens = super::craft_theme::current(&ctx);
                let style = ctx.style_of(theme);
                assert_eq!(tokens.kind == super::craft_theme::ThemeKind::Light, theme == eframe::egui::Theme::Light);
                assert_eq!(tokens.panel_bg, style.visuals.panel_fill);
                assert_eq!(tokens.card_bg, style.visuals.window_fill);
                assert_eq!(tokens.text, style.visuals.text_color());
            }
        }
    }

    #[test]
    fn native_visuals_use_craft_theme_tokens_and_widget_palette() {
        let ctx = eframe::egui::Context::default();
        let config = crate::config::AppConfig {
            theme: crate::config::AppTheme::Dark,
            ..Default::default()
        };
        super::configure_native_visuals(&ctx, &config);
        let dark_style = ctx.style_of(eframe::egui::Theme::Dark);
        let tokens = super::craft_theme::current(&ctx);

        assert_eq!(dark_style.visuals.widgets.hovered.bg_fill, tokens.hover);
        assert_eq!(dark_style.visuals.widgets.active.bg_fill, tokens.pressed);
        assert_eq!(dark_style.visuals.widgets.inactive.bg_stroke.color, tokens.field_border);
    }
}
pub(crate) mod validated_input;
