use eframe::egui::{self, Color32, Theme, ThemePreference};
use pealayer_downloads::ui::palette;
use serde::Deserialize;
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[path = "../../../shared/appearance_choices.rs"]
mod choices;
use choices::{UiAnimationMode, UiControlDensity, UiFontFamily};
#[path = "../../../shared/config_file_watcher.rs"]
mod config_file_watcher;
use config_file_watcher::ConfigFileWatcher;
#[path = "../../../shared/local_config_path.rs"]
mod local_config_path;
#[path = "../../../assets/themes/native_fonts.rs"]
mod native_fonts;
#[path = "../../../shared/native_motion.rs"]
mod native_motion;
#[path = "../../../shared/system_accent.rs"]
mod system_accent;
#[cfg(windows)]
#[path = "../../../shared/windows_chrome.rs"]
mod windows_chrome;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
struct Profile {
    theme: String,
    color_palette: String,
    accent_color: String,
    custom_accent_color: Option<String>,
    ui_control_density: UiControlDensity,
    ui_animation_mode: UiAnimationMode,
    ui_font_family: UiFontFamily,
    ui_font_name: String,
    monospace_font_family: UiFontFamily,
    monospace_font_name: String,
    language: String,
    windows_dwm_theming: bool,
    windows_mica_backdrop: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            color_palette: "native".into(),
            accent_color: "system".into(),
            custom_accent_color: None,
            ui_control_density: UiControlDensity::default(),
            ui_animation_mode: UiAnimationMode::default(),
            ui_font_family: UiFontFamily::default(),
            ui_font_name: String::new(),
            monospace_font_family: UiFontFamily::default(),
            monospace_font_name: String::new(),
            language: "system".into(),
            windows_dwm_theming: true,
            windows_mica_backdrop: false,
        }
    }
}

impl Profile {
    fn read(path: &Path) -> Option<Self> {
        const MAX_BYTES: u64 = 16 * 1024 * 1024;
        let file = std::fs::File::open(path).ok()?;
        if file.metadata().ok()?.len() > MAX_BYTES {
            return None;
        }
        serde_json::from_reader(file.take(MAX_BYTES + 1)).ok()
    }

    fn palette(&self) -> &'static str {
        match self.color_palette.as_str() {
            "studio" => "studio",
            "pastel" => "pastel",
            _ => "native",
        }
    }

    fn accent(&self) -> Color32 {
        let system_default = if cfg!(target_os = "macos") {
            [10, 132, 255]
        } else {
            [0, 120, 212]
        };
        let [r, g, b] = match self.accent_color.as_str() {
            "pealayer_green" => [56, 210, 122],
            "windows_blue" | "custom" => [0, 120, 212],
            "macos_blue" => [10, 132, 255],
            _ => system_accent::system_accent_color().unwrap_or(system_default),
        };
        if self.accent_color == "custom"
            && let Some(accent) = self
                .custom_accent_color
                .as_deref()
                .and_then(palette::parse_rgb)
        {
            return accent;
        }
        Color32::from_rgb(r, g, b)
    }

    fn apply(&self, ctx: &egui::Context, previous: Option<&Self>) {
        let theme_override = std::env::var("APP_THEME")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let theme = match theme_override.as_str() {
            "light" | "dark" | "system" => theme_override.as_str(),
            _ => self.theme.as_str(),
        };
        ctx.set_theme(match theme {
            "light" => ThemePreference::Light,
            "dark" => ThemePreference::Dark,
            _ => ThemePreference::System,
        });
        let accent = self.accent();
        let (dark, light) = palette::desktop_visuals(self.palette(), accent);
        ctx.set_visuals_of(Theme::Dark, dark);
        ctx.set_visuals_of(Theme::Light, light);
        palette::controls::configure(ctx, self.palette() == "pastel");
        ctx.all_styles_mut(|style| {
            palette::configure_interaction_style(style);
            palette::configure_density(style, self.ui_control_density.base_height());
        });
        palette::configure_desktop_text_sizes(ctx);
        self.sync_motion(ctx);
        if previous.is_none_or(|old| old.typography() != self.typography()) {
            let (proportional, monospace, persian) = self.typography();
            native_fonts::install(ctx, proportional, monospace, persian);
        }
    }

    fn typography(&self) -> ((UiFontFamily, &str), (UiFontFamily, &str), bool) {
        (
            (self.ui_font_family, self.ui_font_name.as_str()),
            (
                self.monospace_font_family,
                self.monospace_font_name.as_str(),
            ),
            std::env::var("APP_LOCALE")
                .ok()
                .as_deref()
                .unwrap_or(&self.language)
                .split(['-', '_'])
                .next()
                .is_some_and(|language| language.eq_ignore_ascii_case("fa")),
        )
    }

    fn sync_motion(&self, ctx: &egui::Context) {
        let animation_time = native_motion::duration(self.ui_animation_mode);
        if ctx.style_of(Theme::Dark).animation_time != animation_time
            || ctx.style_of(Theme::Light).animation_time != animation_time
        {
            ctx.all_styles_mut(|style| style.animation_time = animation_time);
        }
    }
}

fn config_path() -> PathBuf {
    if let Some(path) = std::env::var_os("PEALAYER_CONFIG_FILE") {
        return path.into();
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
        && local_config_path::is_portable(dir, std::env::var("PEALAYER_PORTABLE").ok().as_deref())
    {
        return local_config_path::resolve_portable_config_path(dir);
    }
    local_config_path::resolve_system_config_path()
}

pub fn install(ctx: &egui::Context) {
    Profile::read(&config_path())
        .unwrap_or_default()
        .apply(ctx, None);
}

pub struct Appearance {
    path: PathBuf,
    profile: Profile,
    watcher: Option<ConfigFileWatcher>,
    reload_after: Option<Instant>,
    #[cfg(windows)]
    chrome: Option<(isize, bool, bool, bool, u32, u32)>,
}

impl Appearance {
    pub fn new(ctx: &egui::Context) -> Self {
        let path = config_path();
        let profile = Profile::read(&path).unwrap_or_default();
        profile.apply(ctx, None);
        let repaint = ctx.clone();
        let watcher = ConfigFileWatcher::new(&path, move || repaint.request_repaint()).ok();
        Self {
            path,
            profile,
            watcher,
            reload_after: None,
            #[cfg(windows)]
            chrome: None,
        }
    }

    pub fn sync(&mut self, ctx: &egui::Context, _frame: &eframe::Frame) {
        if self
            .watcher
            .as_ref()
            .is_some_and(|watcher| watcher.take_changed().unwrap_or(false))
        {
            self.reload_after = Some(Instant::now() + Duration::from_millis(90));
        }
        if let Some(due) = self.reload_after {
            if due > Instant::now() {
                ctx.request_repaint_after(due.saturating_duration_since(Instant::now()));
            } else {
                self.reload_after = None;
                if let Some(profile) =
                    Profile::read(&self.path).filter(|profile| *profile != self.profile)
                {
                    profile.apply(ctx, Some(&self.profile));
                    self.profile = profile;
                }
            }
        }
        self.profile.sync_motion(ctx);
        #[cfg(windows)]
        self.sync_caption(ctx, _frame);
    }

    #[cfg(windows)]
    fn sync_caption(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let Ok(handle) = frame.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return;
        };
        let style = ctx.style_of(ctx.theme());
        let visuals = &style.visuals;
        let colorref = |color: Color32| windows_chrome::colorref([color.r(), color.g(), color.b()]);
        let chrome = (
            handle.hwnd.get(),
            visuals.dark_mode,
            self.profile.windows_dwm_theming,
            self.profile.windows_mica_backdrop,
            colorref(visuals.panel_fill),
            colorref(visuals.text_color()),
        );
        if self.chrome != Some(chrome) {
            let (hwnd, dark, theming, mica, caption, text) = chrome;
            windows_chrome::apply(hwnd, dark, theming, mica, caption, text);
            self.chrome = Some(chrome);
        }
    }
}
