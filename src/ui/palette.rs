//! Shared palette data is also imported by the Web UI; neither renderer owns
//! a separate copy of the named themes.
use eframe::egui;
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
        crate::config::ColorPalette::Pastel => "pastel",
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
    // Expanded selectors/menu triggers use egui's independent open state.
    // Leaving it untouched leaks stock grey into the selected palette.
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, text);
    visuals.widgets.open.bg_stroke = hover_border;
    visuals.widgets.open.bg_fill = surface;
    visuals.widgets.open.weak_bg_fill = control;
    // Active/selection colors remain accent-aware and keep the existing light
    // theme contrast fix. Palette selection never changes geometry or sizing.
}

/// Paint a rounded gradient with the same clip and edge AA as a solid egui
/// rectangle. The two texels are cached per context and shared by all strips.
pub(super) fn paint_gradient(
    painter: &egui::Painter,
    rect: egui::Rect,
    radius: u8,
    start: egui::Color32,
    end: egui::Color32,
    opacity: f32,
) {
    if !rect.is_positive() || !rect.intersects(painter.clip_rect()) {
        return;
    }
    if radius == 0 {
        // egui's thin-rectangle path drops textures. Keep both color stops
        // on the vertices so a one-point accent line retains its gradient.
        let mut mesh = egui::Mesh::default();
        let start = start.gamma_multiply(opacity);
        let end = end.gamma_multiply(opacity);
        mesh.colored_vertex(rect.left_top(), start);
        mesh.colored_vertex(rect.right_top(), end);
        mesh.colored_vertex(rect.right_bottom(), end);
        mesh.colored_vertex(rect.left_bottom(), start);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(mesh);
        return;
    }
    let ctx = painter.ctx();
    let id = egui::Id::new(("pealayer-palette-gradient", start, end));
    let texture = ctx.data(|data| data.get_temp::<egui::TextureHandle>(id))
        .unwrap_or_else(|| {
            let texture = ctx.load_texture(
                "palette-gradient",
                egui::ColorImage::new([2, 1], vec![start, end]),
                egui::TextureOptions::LINEAR,
            );
            ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
            texture
        });
    let uv = egui::Rect::from_min_max(egui::pos2(0.25, 0.0), egui::pos2(0.75, 1.0));
    painter.add(egui::epaint::RectShape::filled(
        rect, radius, egui::Color32::WHITE.gamma_multiply(opacity),
    ).with_texture(texture.id(), uv));
}

#[cfg(test)]
mod tests {

    #[test]
    fn thin_accent_rule_paints_both_gradient_stops() {
        use eframe::egui::{self, Color32, Shape};
        let ctx = egui::Context::default();
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(120.0, 1.0));
        let start = Color32::from_rgb(163, 147, 244);
        let end = Color32::from_rgb(239, 133, 188);
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            super::paint_gradient(ui.painter(), rect, 0, start, end, 1.0);
        });
        assert!(output.shapes.iter().any(|shape| match &shape.shape {
            Shape::Mesh(mesh) => mesh.vertices.iter().map(|vertex| vertex.color)
                .eq([start, end, end, start]),
            _ => false,
        }));
    }
    #[test]
    fn expanded_dropdown_paints_the_live_palette_without_changing_geometry() {
        use crate::config::{AppConfig, AppTheme, ColorPalette};
        use crate::ui::dropdown::{ComboBox, DropdownUiExt};
        use eframe::egui::{self, Color32, Shape};
        fn frame(ctx: &egui::Context, surface: Color32) -> (egui::Response, egui::FullOutput) {
            let mut trigger = None;
            let mut output = ctx.run_ui(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0))),
                ..Default::default()
            }, |ui| {
                trigger = Some(ComboBox::from_id_salt("palette-open-regression")
                    .width(220.0).selected_text("Palette choice").show_ui(ui, |ui| {
                        assert_eq!(ui.visuals().window_fill(), surface);
                        ui.dropdown_choice(false, "First choice");
                        ui.dropdown_choice(false, "Second choice");
                    }).response);
            });
            output.textures_delta.clear(); // Headless paint inspection, no GPU upload.
            (trigger.unwrap(), output)
        }
        fn filled(shape: &Shape, fill: Color32, target: egui::Rect) -> bool {
            match shape {
                Shape::Rect(rect) => rect.fill == fill && rect.rect.contains_rect(target),
                Shape::Vec(shapes) => shapes.iter().any(|shape| filled(shape, fill, target)),
                _ => false,
            }
        }
        for dark in [false, true] {
            let ctx = egui::Context::default();
            let mut previous_rect = None;
            for palette in [ColorPalette::Studio, ColorPalette::Pastel, ColorPalette::Native, ColorPalette::Studio] {
                let config = AppConfig {
                    color_palette: palette,
                    theme: if dark { AppTheme::Dark } else { AppTheme::Light },
                    ..Default::default()
                };
                crate::ui::configure_native_appearance(&ctx, &config);
                let style = ctx.global_style();
                let surface = super::color(palette, dark, "surface-1");
                let control = super::color(palette, dark, "surface-2");
                assert_eq!(style.visuals.widgets.open.bg_fill, surface);
                assert_eq!(style.visuals.widgets.open.weak_bg_fill, control);
                assert_eq!(style.visuals.widgets.open.bg_stroke.color, super::color(palette, dark, "line-strong"));
                assert_eq!(style.visuals.widgets.open.fg_stroke.color, super::color(palette, dark, "text"));
                // Keep the real popup open across live palette switches.
                let (trigger, _) = frame(&ctx, surface);
                egui::Popup::open_id(&ctx, trigger.id.with("popup"));
                frame(&ctx, surface); // Disabled sizing pass for a newly opened popup.
                let (trigger, output) = frame(&ctx, surface);
                assert!(ComboBox::is_open(&ctx, trigger.id));
                if let Some(rect) = previous_rect { assert_eq!(trigger.rect, rect); }
                previous_rect = Some(trigger.rect);
                assert!(output.shapes.iter().any(|shape| filled(&shape.shape, control, trigger.rect.shrink(2.0))),
                    "expanded selector must paint the active palette, not stock grey");
            }
        }
    }

    #[test]
    fn live_appearance_advertises_the_same_palette_preference() {
        let config = crate::config::AppConfig {
            color_palette: crate::config::ColorPalette::Native,
            ..Default::default()
        };
        let appearance = crate::platform::interop::AppearanceState::new(&config, true);
        let json = serde_json::to_value(&appearance).unwrap();
        assert_eq!(json["color_palette"], "native");
        assert_eq!(json["theme"], "system");
        assert_eq!(json["resolved_theme"], "dark");
        let green = crate::config::AppConfig {
            accent_color: crate::config::AccentColor::PealayerGreen,
            ..config.clone()
        };
        let preview = crate::platform::interop::AppearanceState::new(&green, false);
        assert_eq!(preview.resolved_accent, "#38d27a");
        assert_eq!(preview.resolved_theme, crate::config::AppTheme::Light);
        assert_ne!(appearance, preview);
    }

    #[test]
    fn palette_is_persisted_and_advertised_by_the_shared_preferences_contract() {
        use crate::config::{AppConfig, ColorPalette};
        let old: AppConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(old.color_palette, ColorPalette::Native);
        let changed = old
            .apply_patch(&serde_json::json!({"color_palette": "pastel"}))
            .unwrap();
        let restored: AppConfig =
            serde_json::from_str(&serde_json::to_string(&changed).unwrap()).unwrap();
        assert_eq!(restored.color_palette, ColorPalette::Pastel);
        let live = crate::platform::interop::AppearanceState::new(&restored, true);
        assert_eq!(serde_json::to_value(live).unwrap()["color_palette"], "pastel");
        let controls = crate::preferences_contract::preference_controls(&restored);
        let control = controls
            .iter()
            .find(|control| control.key == "color_palette")
            .unwrap();
        let options: Vec<_> = control.options.iter().map(|option| option.value.as_str().unwrap()).collect();
        assert_eq!(options, ["native", "studio", "pastel"]);
    }

    #[test]
    fn shared_palettes_are_complete_and_preserve_the_web_studio_colors() {
        use crate::config::ColorPalette;
        for dark in [true, false] {
            let baseline = super::colors(ColorPalette::Native, dark);
            for palette in [ColorPalette::Native, ColorPalette::Studio, ColorPalette::Pastel] {
                let values = super::colors(palette, dark);
                assert!(baseline.keys().all(|role| values.contains_key(role)));
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
