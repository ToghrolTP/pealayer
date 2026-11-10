//! Shared palette data is also imported by the Web UI; neither renderer owns
//! a separate copy of the named themes.
use super::egui;
use std::collections::BTreeMap;
use std::sync::OnceLock;

type PaletteCatalog = BTreeMap<String, BTreeMap<String, BTreeMap<String, String>>>;

pub fn colors(palette: &str, dark: bool) -> &'static BTreeMap<String, String> {
    static CATALOG: OnceLock<PaletteCatalog> = OnceLock::new();
    let catalog = CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("palettes.json"))
            .expect("bundled theme palettes must be valid")
    });
    &catalog[palette][if dark { "dark" } else { "light" }]
}

pub fn color(palette: &str, dark: bool, role: &str) -> egui::Color32 {
    let hex = &colors(palette, dark)[role];
    parse_rgb(hex).expect("bundled palette colors must be RGB hex values")
}

pub fn parse_rgb(hex: &str) -> Option<egui::Color32> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let rgb = u32::from_str_radix(hex, 16).ok()?;
    Some(egui::Color32::from_rgb(
        (rgb >> 16) as u8,
        (rgb >> 8) as u8,
        rgb as u8,
    ))
}

pub fn apply(visuals: &mut egui::Visuals, palette: &str) {
    use egui::Stroke;
    let dark = visuals.dark_mode;
    let rgb = |role| color(palette, dark, role);
    let panel = rgb("surface-0");
    let surface = rgb("surface-1");
    let control = rgb("surface-2");
    let hover = rgb("surface-3");
    let text = rgb("text");
    let muted = rgb("muted");
    let border = Stroke::new(1.0, rgb("line"));
    let hover_role = if colors(palette, dark).contains_key("hover-line") {
        "hover-line"
    } else {
        "line-strong"
    };
    let hover_border = Stroke::new(1.0, rgb(hover_role));
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
    // Expanded selectors/menu triggers use egui's independent open state.
    // Leaving it untouched leaks stock grey into the selected palette.
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, text);
    visuals.widgets.open.bg_stroke = hover_border;
    visuals.widgets.open.bg_fill = surface;
    visuals.widgets.open.weak_bg_fill = control;
    // Active/selection colors remain accent-aware and keep the existing light
    // theme contrast fix. Palette selection never changes geometry or sizing.
}

pub fn configure_density(style: &mut egui::Style, height: f32) {
    style.spacing.interact_size.y = height;
    style.spacing.button_padding.y = if height == 18.0 {
        1.0
    } else {
        ((height - style.spacing.icon_width) * 0.5).max(0.0)
    };
    for state in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        state.expansion = 0.0;
    }
}
