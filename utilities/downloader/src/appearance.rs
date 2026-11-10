use eframe::egui::{self, Color32, Stroke, Theme, ThemePreference, Visuals};
use pealayer_downloads::ui::palette;
use serde_json::Value;
use std::path::PathBuf;

#[path = "../../../shared/system_accent.rs"]
mod system_accent;

pub fn install(ctx: &egui::Context) {
    let profile = config_path()
        .and_then(|path| {
            let file = std::fs::File::open(path).ok()?;
            if file.metadata().ok()?.len() > 16 * 1024 * 1024 {
                return None;
            }
            serde_json::from_reader(file).ok()
        })
        .unwrap_or(Value::Null);
    let get = |key: &str| profile[key].as_str().unwrap_or_default();
    let name = if get("color_palette") == "studio" {
        "studio"
    } else {
        "native"
    };
    let theme_override = std::env::var("APP_THEME").unwrap_or_default();
    let theme = match theme_override.trim().to_ascii_lowercase().as_str() {
        "light" => "light",
        "dark" => "dark",
        "system" => "system",
        _ => get("theme"),
    };
    ctx.set_theme(match theme {
        "light" => ThemePreference::Light,
        "dark" => ThemePreference::Dark,
        _ => ThemePreference::System,
    });
    let native = if cfg!(target_os = "macos") {
        [10, 132, 255]
    } else {
        [0, 120, 212]
    };
    let [r, g, b] = match get("accent_color") {
        "pealayer_green" => [56, 210, 122],
        "windows_blue" => [0, 120, 212],
        "macos_blue" => [10, 132, 255],
        "system" | "" => system_accent::system_accent_color().unwrap_or(native),
        _ => native,
    };
    let mut accent = Color32::from_rgb(r, g, b);
    if get("accent_color") == "custom"
        && let Some(color) = palette::parse_rgb(get("custom_accent_color"))
    {
        accent = color;
    }
    let foreground =
        if u32::from(accent.r()) * 299 + u32::from(accent.g()) * 587 + u32::from(accent.b()) * 114
            > 150_000
        {
            Color32::BLACK
        } else {
            Color32::WHITE
        };
    for (theme, mut visuals) in [
        (Theme::Dark, Visuals::dark()),
        (Theme::Light, Visuals::light()),
    ] {
        palette::apply(&mut visuals, name);
        visuals.window_corner_radius = 10.into();
        visuals.menu_corner_radius = 8.into();
        visuals.selection.bg_fill = accent;
        visuals.selection.stroke = Stroke::new(1.0, foreground);
        visuals.hyperlink_color = accent;
        let pressed =
            palette::color(name, visuals.dark_mode, "surface-3").lerp_to_gamma(accent, 0.22);
        visuals.widgets.active.bg_fill = pressed;
        visuals.widgets.active.weak_bg_fill = pressed;
        visuals.widgets.active.fg_stroke.color = visuals.text_color();
        ctx.set_visuals_of(theme, visuals);
    }
    let density = match get("ui_control_density") {
        "extra_compact" => 18.0,
        "regular" => 28.0,
        "comfortable" => 32.0,
        "spacious" => 36.0,
        _ => 24.0,
    };
    ctx.all_styles_mut(|style| {
        palette::configure_density(style, density);
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.interaction.selectable_labels = false;
        style.interaction.multi_widget_text_select = false;
        if get("ui_animation_mode") == "Disabled" {
            style.animation_time = 0.0;
        }
    });

    let mut fonts = egui::FontDefinitions::default();
    let chosen: Option<&'static [u8]> = match get("ui_font_family") {
        "bundled" => Some(include_bytes!("../../../assets/fonts/InterVariable.ttf")),
        "vazirmatn" => Some(include_bytes!(
            "../../../assets/fonts/Vazirmatn-Regular.ttf"
        )),
        _ => None,
    };
    if let Some(data) = chosen {
        let name = "pealayer-downloader-ui".to_owned();
        fonts
            .font_data
            .insert(name.clone(), egui::FontData::from_static(data).into());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, name);
    }
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
}

fn config_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PEALAYER_CONFIG_FILE") {
        return Some(path.into());
    }
    #[cfg(windows)]
    if let Some(path) = std::env::var_os("APPDATA") {
        return Some(PathBuf::from(path).join("pealayer/config.json"));
    }
    #[cfg(target_os = "macos")]
    if let Some(path) = std::env::var_os("HOME") {
        return Some(PathBuf::from(path).join("Library/Application Support/Pealayer/config.json"));
    }
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|root| root.join("pealayer/config.json"))
}
