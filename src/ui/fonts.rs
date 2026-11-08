//! Native font selection. Fontique uses DirectWrite, CoreText and fontconfig;
//! no font paths, installed face names or font-file copies are assumed.
use crate::config::{AppConfig, UiFontFamily};
use eframe::egui;
use fontique::{
    Attributes, Collection, CollectionOptions, FontWeight, GenericFamily, QueryFamily, QueryStatus,
    SourceCache, SourceCacheOptions,
};
use std::sync::{Arc, Mutex, OnceLock};

struct FontBook {
    collection: Collection,
    cache: SourceCache,
}
fn book() -> &'static Mutex<FontBook> {
    static BOOK: OnceLock<Mutex<FontBook>> = OnceLock::new();
    BOOK.get_or_init(|| {
        Mutex::new(FontBook {
            collection: Collection::new(CollectionOptions {
                shared: false,
                system_fonts: true,
            }),
            cache: SourceCache::new(SourceCacheOptions::default()),
        })
    })
}

pub fn family_names() -> Vec<String> {
    static NAMES: OnceLock<Vec<String>> = OnceLock::new();
    NAMES
        .get_or_init(|| {
            let mut names: Vec<_> = book()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .collection
                .family_names()
                .map(str::to_owned)
                .collect();
            names.sort_by_key(|name| name.to_lowercase());
            names.dedup();
            names
        })
        .clone()
}

fn resolve(name: Option<&str>, mono: bool, emphasis: bool) -> Option<egui::FontData> {
    let mut book = book().lock().unwrap_or_else(|e| e.into_inner());
    let FontBook { collection, cache } = &mut *book;
    let mut query = collection.query(cache);
    let mut families = Vec::new();
    if let Some(name) = name.filter(|name| !name.trim().is_empty()) {
        families.push(QueryFamily::Named(name));
    }
    families.push(QueryFamily::Generic(if mono {
        GenericFamily::Monospace
    } else {
        GenericFamily::SystemUi
    }));
    query.set_families(families);
    query.set_attributes(Attributes {
        weight: FontWeight::new(if emphasis { 600.0 } else { 400.0 }),
        ..Default::default()
    });
    let mut found = None;
    query.matches_with(|font| {
        // Reject bitmap/unsupported faces before handing them to egui. Keep TTC index.
        if ab_glyph::FontRef::try_from_slice_and_index(font.blob.as_ref(), font.index).is_ok() {
            let (blob, _) = font.blob.clone().into_raw_parts();
            let mut data = egui::FontData::from_blob(blob, font.index);
            // Fontique selects a face, but FontData does not inherit the query's
            // variable-font location. Some installed VFs default to weight 100.
            apply_weight(&mut data, if emphasis { 600.0 } else { 400.0 });
            found = Some(data);
            QueryStatus::Stop
        } else {
            QueryStatus::Continue
        }
    });
    found
}

fn apply_weight(data: &mut egui::FontData, weight: f32) {
    for axis in data.variation_axes() {
        if axis.tag.to_string() == "wght" {
            data.tweak.coords.push(axis.tag, weight.clamp(axis.range.min, axis.range.max));
        }
    }
}

#[cfg(any(windows, test))]
fn windows_monospace_family(names: &[String]) -> Option<String> {
    // Windows has no NONCLIENTMETRICS fixed-pitch preference. DirectWrite's
    // generic match may be Courier New; prefer its modern UI companions when
    // installed. Never assume Segoe's monospace variant exists on every host.
    ["Segoe UI Mono", "Segoe Monospace", "Cascadia Mono", "Consolas"]
        .iter()
        .find_map(|preferred| names.iter().find(|name| name.eq_ignore_ascii_case(preferred)).cloned())
}

#[cfg(target_os = "linux")]
fn desktop_name(mono: bool) -> Option<String> {
    // Read the active DE's preference first. fontconfig handles aliases/fallbacks.
    // Each helper is bounded; a broken session bus cannot hang application startup.
    fn output(program: &str, args: &[&str]) -> Option<String> {
        use std::{
            io::Read,
            process::{Command, Stdio},
            time::{Duration, Instant},
        };
        let mut child = Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().ok()? {
                if !status.success() {
                    return None;
                }
                let mut text = String::new();
                child
                    .stdout
                    .take()?
                    .take(4096)
                    .read_to_string(&mut text)
                    .ok()?;
                return Some(text.trim().trim_matches('\'').to_owned());
            }
            if start.elapsed() > Duration::from_millis(300) {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();
    let configured = if desktop.contains("kde") {
        let key = if mono { "fixed" } else { "font" };
        output(
            "kreadconfig6",
            &["--file", "kdeglobals", "--group", "General", "--key", key],
        )
        .or_else(|| {
            output(
                "kreadconfig5",
                &["--file", "kdeglobals", "--group", "General", "--key", key],
            )
        })
        .map(|text| text.split(',').next().unwrap_or_default().to_owned())
    } else if desktop.contains("xfce") {
        output(
            "xfconf-query",
            &[
                "-c",
                "xsettings",
                "-p",
                if mono {
                    "/Gtk/MonospaceFontName"
                } else {
                    "/Gtk/FontName"
                },
            ],
        )
    } else {
        let schema = if desktop.contains("cinnamon") {
            "org.cinnamon.desktop.interface"
        } else if desktop.contains("mate") {
            "org.mate.interface"
        } else {
            "org.gnome.desktop.interface"
        };
        output(
            "gsettings",
            &[
                "get",
                schema,
                if mono {
                    "monospace-font-name"
                } else {
                    "font-name"
                },
            ],
        )
    };
    configured
        .map(|text| without_point_size(&text))
        .filter(|name| !name.is_empty())
}

#[cfg(windows)]
fn desktop_name(mono: bool) -> Option<String> {
    if mono {
        return windows_monospace_family(&family_names());
    }
    use windows::Win32::UI::WindowsAndMessaging::{
        NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
        SystemParametersInfoW,
    };
    let mut metrics = NONCLIENTMETRICSW {
        cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32,
        ..Default::default()
    };
    unsafe {
        SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            metrics.cbSize,
            Some((&mut metrics as *mut NONCLIENTMETRICSW).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .ok()?;
    }
    let face = metrics.lfMessageFont.lfFaceName;
    Some(String::from_utf16_lossy(
        &face[..face.iter().position(|c| *c == 0).unwrap_or(face.len())],
    ))
}

#[cfg(target_os = "macos")]
fn desktop_name(mono: bool) -> Option<String> {
    // AppKit owns the user's fixed-pitch choice. Fontique's generic monospace
    // fallback is Courier; that is not the macOS preferred fixed-pitch face.
    let font = if mono {
        objc2_app_kit::NSFont::userFixedPitchFontOfSize(0.0)?
    } else {
        objc2_app_kit::NSFont::systemFontOfSize(0.0)
    };
    font.familyName().map(|name| name.to_string())
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn desktop_name(_mono: bool) -> Option<String> {
    None
}

#[cfg(any(target_os = "linux", test))]
fn without_point_size(text: &str) -> String {
    let text = text.trim().trim_matches('\'');
    match text.rsplit_once(' ') {
        Some((family, size)) if size.parse::<f32>().is_ok() => family.to_owned(),
        _ => text.to_owned(),
    }
}

pub fn install(ctx: &egui::Context, config: &AppConfig, persian: bool) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    fonts.font_data.insert(
        "pealayer-vazirmatn".into(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/Vazirmatn-Regular.ttf"
        ))),
    );
    // Licensed YekanBakh stays on the host. Only the open Vazirmatn fallback
    // is bundled; missing VF installations must not silently resolve to Arial.
    let persian_family = family_names().into_iter().find(|name|
        name.eq_ignore_ascii_case("Yekan Bakh VF") || name.eq_ignore_ascii_case("YekanBakh VF"));
    let mut persian_chain=vec!["pealayer-vazirmatn".to_string()];
    if let Some(name)=persian_family.as_deref() && let Some(data)=resolve(Some(name),false,false) {
        fonts.font_data.insert("pealayer-yekan-vf".into(),Arc::new(data));
        persian_chain.insert(0,"pealayer-yekan-vf".into());
    }
    fonts.families.insert(egui::FontFamily::Name("pealayer-persian".into()),persian_chain.clone());
    for (mono, choice, custom) in [
        (false, config.ui_font_family, &config.ui_font_name),
        (
            true,
            config.monospace_font_family,
            &config.monospace_font_name,
        ),
    ] {
        let family = if mono {
            egui::FontFamily::Monospace
        } else {
            egui::FontFamily::Proportional
        };
        let name = if choice == UiFontFamily::Custom {
            Some(custom.clone())
        } else {
            desktop_name(mono)
        };
        let use_native = matches!(choice, UiFontFamily::System | UiFontFamily::Custom);
        let regular = if mono {
            "pealayer-system-mono"
        } else {
            "pealayer-system-ui"
        };
        let strong = if mono {
            "pealayer-status-mono-strong"
        } else {
            "pealayer-status-strong"
        };
        if use_native && let Some(data) = resolve(name.as_deref(), mono, false) {
            fonts.font_data.insert(regular.into(), Arc::new(data));
            fonts
                .families
                .get_mut(&family)
                .unwrap()
                .insert(0, regular.into());
        }
        if !mono {
            let chain = fonts.families.get_mut(&family).unwrap();
            if persian && choice == UiFontFamily::System {
                for name in persian_chain.iter().rev() { chain.insert(0,name.clone()); }
            } else if choice == UiFontFamily::Vazirmatn {
                chain.insert(0, "pealayer-vazirmatn".into());
            } else {
                chain.push("pealayer-vazirmatn".into());
            }
        }
        let mut emphasis = fonts.families[&family].clone();
        if persian && !mono && choice==UiFontFamily::System {
            if let Some(name)=persian_family.as_deref() && let Some(data)=resolve(Some(name),false,true) {
                fonts.font_data.insert(strong.into(),Arc::new(data));emphasis.insert(0,strong.into());
            }
        }
        if use_native
            && !(persian && !mono && choice == UiFontFamily::System)
            && let Some(data) = resolve(name.as_deref(), mono, true)
        {
            fonts.font_data.insert(strong.into(), Arc::new(data));
            emphasis.insert(0, strong.into());
        }
        fonts
            .families
            .insert(egui::FontFamily::Name(strong.into()), emphasis);
    }
    ctx.set_fonts(fonts);
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
