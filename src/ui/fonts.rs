use crate::config::{AppConfig, UiFontFamily};
use eframe::egui;

#[path = "../../assets/themes/native_fonts.rs"]
mod native;
pub use native::family_names;
#[cfg(test)]
use native::{resolve, windows_monospace_family, without_point_size};

pub fn install(ctx: &egui::Context, config: &AppConfig, persian: bool) {
    native::install(
        ctx,
        (config.ui_font_family, &config.ui_font_name),
        (config.monospace_font_family, &config.monospace_font_name),
        persian,
    );
}

pub(crate) fn status_family(ui: &egui::Ui) -> egui::FontFamily {
    let family = egui::FontFamily::Name("pealayer-status".into());
    if ui.fonts_mut(|fonts| fonts.families().contains(&family)) { family }
    else { egui::FontFamily::Proportional }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_monospace_prefers_installed_modern_faces_without_inventing_one() {
        let names = ["Courier New", "Consolas", "Cascadia Mono"].map(str::to_owned);
        assert_eq!(windows_monospace_family(&names).as_deref(), Some("Cascadia Mono"));
        assert_eq!(windows_monospace_family(&["consolas".into()]).as_deref(), Some("consolas"));
        assert_eq!(windows_monospace_family(&["Courier New".into()]), None);
    }

    #[test]
    fn installed_variable_fonts_keep_requested_regular_and_semibold_weights() {
        // Host-conditional coverage: CI need not install private YekanBakh.
        for name in family_names().into_iter().filter(|name|
            name.contains("Yekan") || name == "Cascadia Mono") {
            for (emphasis, expected) in [(false,400.0_f32),(true,600.0_f32)] {
                let font = resolve(Some(&name),false,emphasis).unwrap();
                if let Some(axis) = font.variation_axes().into_iter().find(|axis| axis.tag.to_string()=="wght") {
                    assert!(font.tweak.coords.as_ref().iter().any(|(tag,value)|
                        tag.to_string()=="wght" && *value==expected.clamp(axis.range.min,axis.range.max)), "{name}");
                }
            }
        }
    }
    #[test]
    fn desktop_family_parsing_preserves_multiword_names() {
        assert_eq!(super::without_point_size("'Noto Sans 11'"), "Noto Sans");
        assert_eq!(
            super::without_point_size("JetBrains Mono 10.5"),
            "JetBrains Mono"
        );
        assert_eq!(
            super::without_point_size("DejaVu Sans Mono"),
            "DejaVu Sans Mono"
        );
    }

    #[test]
    fn native_and_custom_font_choices_keep_monospace_and_icon_fallbacks() {
        let ctx = egui::Context::default();
        for choice in [
            UiFontFamily::System,
            UiFontFamily::Custom,
            UiFontFamily::Bundled,
        ] {
            let config = AppConfig {
                ui_font_family: choice,
                monospace_font_family: choice,
                ui_font_name: "Unavailable test family".into(),
                monospace_font_name: "Unavailable test family".into(),
                ..Default::default()
            };
            install(&ctx, &config, false);
            let mut output = ctx.run_ui(Default::default(), |ui| {
                let widths = ui.fonts_mut(|f| {
                    ['0', '1', 'W', 'i'].map(|ch| f.glyph_width(&egui::FontId::monospace(12.0), ch))
                });
                assert!(widths[0] > 0.0);
                for width in widths {
                    assert!(
                        (width - widths[0]).abs() < 0.05,
                        "Monospace advances differ: {widths:?}"
                    );
                }
                for family in [
                    egui::FontFamily::Proportional,
                    egui::FontFamily::Monospace,
                    egui::FontFamily::Name("pealayer-status-strong".into()),
                    egui::FontFamily::Name("pealayer-status-mono-strong".into()),
                ] {
                    ui.label(
                        egui::RichText::new(format!("{} 12:34", crate::ui::icons::PLAY))
                            .family(family),
                    );
                }
            });
            output.textures_delta.clear();
        }
    }
}
