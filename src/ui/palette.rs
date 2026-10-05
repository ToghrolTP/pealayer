//! Shared palette data is also imported by the Web UI; neither renderer owns
//! a separate copy of the named themes.
use std::collections::BTreeMap;
use std::sync::OnceLock;

type PaletteCatalog = BTreeMap<String, BTreeMap<String, BTreeMap<String, String>>>;

pub fn colors(
    palette: crate::config::ColorPalette,
    dark: bool,
) -> &'static BTreeMap<String, String> {
    static CATALOG: OnceLock<PaletteCatalog> = OnceLock::new();
    let catalog = CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../../assets/themes/palettes.json"))
            .expect("bundled theme palettes must be valid")
    });
    &catalog[match palette {
        crate::config::ColorPalette::Studio => "studio",
        crate::config::ColorPalette::Native => "native",
    }][if dark { "dark" } else { "light" }]
}

pub fn color(
    palette: crate::config::ColorPalette,
    dark: bool,
    role: &str,
) -> eframe::egui::Color32 {
    let [r, g, b] = crate::config::parse_rgb_hex(&colors(palette, dark)[role])
        .expect("bundled palette colors must be RGB hex values");
    eframe::egui::Color32::from_rgb(r, g, b)
}

pub fn apply(visuals: &mut eframe::egui::Visuals, palette: crate::config::ColorPalette) {
    use eframe::egui::Stroke;
    let dark = visuals.dark_mode;
    let rgb = |role| color(palette, dark, role);
    let panel = rgb("surface-0");
    let surface = rgb("surface-1");
    let control = rgb("surface-2");
    let hover = rgb("surface-3");
    let text = rgb("text");
    let muted = rgb("muted");
    let border = Stroke::new(1.0, rgb("line"));
    let hover_border = Stroke::new(1.0, rgb("line-strong"));
    visuals.warn_fg_color = rgb("amber");
    visuals.error_fg_color = rgb("red");
    visuals.panel_fill = panel;
    visuals.window_fill = surface;
    visuals.window_stroke = border;
    visuals.extreme_bg_color = rgb("canvas");
    visuals.text_edit_bg_color = Some(panel);
    visuals.code_bg_color = control;
    visuals.faint_bg_color = control;
    visuals.weak_text_color = Some(muted);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text);
    visuals.widgets.noninteractive.bg_stroke = border;
    visuals.widgets.noninteractive.bg_fill = surface;
    visuals.widgets.noninteractive.weak_bg_fill = surface;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, text);
    visuals.widgets.inactive.bg_stroke = border;
    visuals.widgets.inactive.bg_fill = control;
    visuals.widgets.inactive.weak_bg_fill = control;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, text);
    visuals.widgets.hovered.bg_stroke = hover_border;
    visuals.widgets.hovered.bg_fill = hover;
    visuals.widgets.hovered.weak_bg_fill = hover;
    // Active/selection colors remain accent-aware and keep the existing light
    // theme contrast fix. Palette selection never changes geometry or sizing.
}

#[cfg(test)]
mod tests {
    #[test]
    fn live_appearance_advertises_the_same_palette_preference() {
        let config = crate::config::AppConfig {
            color_palette: crate::config::ColorPalette::Native,
            ..Default::default()
        };
        let appearance = crate::platform::interop::AppearanceState::from(&config);
        let json = serde_json::to_value(&appearance).unwrap();
        assert_eq!(json["color_palette"], "native");
        assert_eq!(json["theme"], "system");
    }

    #[test]
    fn palette_is_persisted_and_advertised_by_the_shared_preferences_contract() {
        use crate::config::{AppConfig, ColorPalette};
        let old: AppConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(old.color_palette, ColorPalette::Studio);
        let changed = old
            .apply_patch(&serde_json::json!({"color_palette": "native"}))
            .unwrap();
        let restored: AppConfig =
            serde_json::from_str(&serde_json::to_string(&changed).unwrap()).unwrap();
        assert_eq!(restored.color_palette, ColorPalette::Native);
        let controls = crate::preferences_contract::preference_controls(&restored);
        let control = controls
            .iter()
            .find(|control| control.key == "color_palette")
            .unwrap();
        assert_eq!(control.options.len(), 2);
    }

    #[test]
    fn shared_palettes_are_complete_and_preserve_the_web_studio_colors() {
        use crate::config::ColorPalette;
        for palette in [ColorPalette::Studio, ColorPalette::Native] {
            for dark in [true, false] {
                let values = super::colors(palette, dark);
                assert_eq!(values.len(), 14);
                for value in values.values() {
                    assert!(crate::config::parse_rgb_hex(value).is_some());
                }
            }
        }
        assert_eq!(
            super::colors(ColorPalette::Studio, true)["surface-1"],
            "#11161e"
        );
        assert_eq!(
            super::colors(ColorPalette::Studio, false)["surface-0"],
            "#f8fafc"
        );
    }

    #[test]
    fn native_and_web_share_surface_and_text_roles_without_changing_widget_sizes() {
        use crate::config::ColorPalette;
        use eframe::egui::Visuals;
        for dark in [true, false] {
            let mut visuals = if dark {
                Visuals::dark()
            } else {
                Visuals::light()
            };
            let radius = visuals.window_corner_radius;
            super::apply(&mut visuals, ColorPalette::Studio);
            assert_eq!(
                visuals.window_fill,
                super::color(ColorPalette::Studio, dark, "surface-1")
            );
            assert_eq!(
                visuals.widgets.inactive.fg_stroke.color,
                super::color(ColorPalette::Studio, dark, "text")
            );
            assert_eq!(visuals.window_corner_radius, radius);
        }
    }
}
