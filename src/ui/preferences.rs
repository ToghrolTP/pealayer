use crate::app::PealayerApp;
use crate::config::{AppConfig, AppLanguage, AppTheme};
use crate::preferences_contract::{
    PreferenceControl, PreferenceControlKind, preference_controls, preference_sections,
    set_value_at_path, value_at_path,
};
use eframe::egui;

const PREFERENCES_RAIL_WIDTH: f32 = 118.0;
const PREFERENCE_ROW_HEIGHT: f32 = 33.0;
const PREFERENCE_ROW_GAP: f32 = 3.0;
const PREFERENCE_COLUMN_GAP: f32 = 6.0;
const PREFERENCE_CONTROL_MAX_WIDTH: f32 = 420.0;
const PREFERENCE_CONTROL_MIN_WIDTH: f32 = 180.0;

pub(crate) struct NativePreferencesController {
    child: std::process::Child,
}

#[derive(Clone)]
pub(crate) struct PreferencesDraft {
    config: AppConfig,
    saved_config: AppConfig,
    tab: usize,
    status: String,
}

#[derive(Default)]
struct PreferencesOutcome {
    close: bool,
    save: bool,
}

impl PreferencesDraft {
    fn new(config: AppConfig, tab: usize) -> Self {
        Self {
            saved_config: config.clone(),
            config,
            tab: tab.min(preference_sections().len().saturating_sub(1)),
            status: String::new(),
        }
    }

    fn is_dirty(&self) -> bool {
        self.config != self.saved_config
    }

    fn mark_saved(&mut self) {
        self.saved_config = self.config.clone();
    }

    fn replace_from_disk(&mut self, config: AppConfig, status: String) {
        self.saved_config = config.clone();
        self.config = config;
        self.status = status;
    }

    pub(crate) fn apply_external_config(
        &mut self,
        config: AppConfig,
        status: String,
        conflict_status: String,
    ) {
        if self.is_dirty() {
            self.status = conflict_status;
        } else {
            self.replace_from_disk(config, status);
        }
    }

    fn from_live_config() -> Self {
        Self::new(
            crate::platform::interop::get_live_config(),
            preferences_helper_tab(&std::env::args().collect::<Vec<_>>()),
        )
    }
}

impl NativePreferencesController {
    fn spawn(owner_hwnd: isize, tab: usize) -> Result<Self, String> {
        let executable = std::env::current_exe()
            .map_err(|error| format!("resolve Pealayer executable: {error}"))?;
        let child = std::process::Command::new(executable)
            .arg("--preferences-helper")
            .arg(owner_hwnd.to_string())
            .arg("--preferences-tab")
            .arg(tab.to_string())
            .spawn()
            .map_err(|error| format!("open native Preferences window: {error}"))?;
        Ok(Self { child })
    }

    fn is_open(&mut self) -> bool {
        self.child.try_wait().is_ok_and(|status| status.is_none())
    }

    fn close(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for NativePreferencesController {
    fn drop(&mut self) {
        self.close();
    }
}

fn native_preferences_viewport(title: String) -> egui::ViewportBuilder {
    let mut builder = egui::ViewportBuilder::default()
        .with_title(title)
        .with_inner_size([700.0, 620.0])
        .with_min_inner_size([420.0, 360.0])
        .with_resizable(true)
        .with_decorations(true)
        .with_taskbar(false)
        .with_minimize_button(false)
        .with_maximize_button(false)
        .with_clamp_size_to_monitor_size(true);
    let icon = phosphor_preferences_icon().or_else(|| {
        eframe::icon_data::from_png_bytes(include_bytes!("../../assets/pealayer-icon.png")).ok()
    });
    if let Some(icon) = icon {
        builder = builder.with_icon(icon);
    }
    builder
}

fn phosphor_preferences_icon() -> Option<egui::IconData> {
    use ab_glyph::{Font, FontRef, PxScale, point};

    const SIZE: usize = 32;
    let font = FontRef::try_from_slice(egui_phosphor::Variant::Regular.font_bytes()).ok()?;
    let character = crate::ui::icons::GEAR.chars().next()?;
    let glyph_id = font.glyph_id(character);
    let scale = PxScale::from(24.0);
    let initial = font.outline_glyph(glyph_id.with_scale(scale))?;
    let bounds = initial.px_bounds();
    let position = point(
        (SIZE as f32 - bounds.width()) * 0.5 - bounds.min.x,
        (SIZE as f32 - bounds.height()) * 0.5 - bounds.min.y,
    );
    let outlined = font.outline_glyph(glyph_id.with_scale_and_position(scale, position))?;
    let mut rgba = vec![0_u8; SIZE * SIZE * 4];
    let pixel_bounds = outlined.px_bounds();
    outlined.draw(|x, y, coverage| {
        let px = pixel_bounds.min.x.floor() as i32 + x as i32;
        let py = pixel_bounds.min.y.floor() as i32 + y as i32;
        if px >= 0 && py >= 0 && px < SIZE as i32 && py < SIZE as i32 {
            let offset = (py as usize * SIZE + px as usize) * 4;
            rgba[offset..offset + 3].copy_from_slice(&[236, 241, 247]);
            rgba[offset + 3] = (coverage * 255.0).round() as u8;
        }
    });
    Some(egui::IconData {
        rgba,
        width: SIZE as u32,
        height: SIZE as u32,
    })
}

pub(crate) fn preferences_helper_owner(args: &[String]) -> Option<isize> {
    args.windows(2)
        .find(|pair| pair[0] == "--preferences-helper")
        .map(|pair| pair[1].parse::<isize>().unwrap_or_default())
}

fn preferences_helper_tab(args: &[String]) -> usize {
    args.windows(2)
        .find(|pair| pair[0] == "--preferences-tab")
        .and_then(|pair| pair[1].parse::<usize>().ok())
        .unwrap_or_default()
}

struct StandalonePreferencesApp {
    draft: PreferencesDraft,
    owner_hwnd: isize,
    native_window_initialized: bool,
    applied_appearance: Option<(AppTheme, bool, bool, [u8; 3])>,
    config_watcher: Option<crate::config::ConfigFileWatcher>,
    config_reload_due: Option<std::time::Instant>,
}

impl StandalonePreferencesApp {
    fn apply_appearance(&mut self, ctx: &egui::Context) {
        let appearance = (
            crate::config::resolved_theme(&self.draft.config),
            self.draft.config.windows_dwm_theming,
            self.draft.config.windows_mica_backdrop,
            crate::ui::platform_accent_rgb(&self.draft.config),
        );
        if self.applied_appearance == Some(appearance) {
            return;
        }
        ctx.set_theme(match appearance.0 {
            AppTheme::System => egui::ThemePreference::System,
            AppTheme::Light => egui::ThemePreference::Light,
            AppTheme::Dark => egui::ThemePreference::Dark,
        });
        crate::ui::configure_native_visuals(ctx, &self.draft.config);
        crate::platform::windows::configure_window_composition(appearance.1, appearance.2);
        crate::platform::windows::set_window_theme(ctx.global_style().visuals.dark_mode);
        self.applied_appearance = Some(appearance);
    }

    fn watch_external_config(&mut self, ctx: &egui::Context) {
        let path = AppConfig::get_config_path();
        if self.config_watcher.is_none() {
            let repaint = ctx.clone();
            match crate::config::ConfigFileWatcher::new(&path, move || {
                repaint.request_repaint();
            }) {
                Ok(watcher) => self.config_watcher = Some(watcher),
                Err(error) => {
                    self.draft.status = error;
                    return;
                }
            }
        }
        let changed = self
            .config_watcher
            .as_ref()
            .map(crate::config::ConfigFileWatcher::take_changed)
            .transpose();
        match changed {
            Ok(Some(true)) => {
                let delay = std::time::Duration::from_millis(90);
                self.config_reload_due = Some(std::time::Instant::now() + delay);
                ctx.request_repaint_after(delay);
            }
            Ok(_) => {}
            Err(error) => self.draft.status = error,
        }
        let Some(due) = self.config_reload_due else {
            return;
        };
        if std::time::Instant::now() < due {
            ctx.request_repaint_after(due.saturating_duration_since(std::time::Instant::now()));
            return;
        }
        self.config_reload_due = None;
        match AppConfig::load_from_path(&path) {
            Ok(config) if config == self.draft.saved_config => {}
            Ok(config) => {
                let language = self.draft.config.language;
                self.draft.apply_external_config(
                    config,
                    native_tr(language, "Preferences reloaded from disk"),
                    native_tr(
                        language,
                        "Configuration changed on disk; reload it or save your draft",
                    ),
                )
            }
            Err(error) => self.draft.status = error,
        }
    }
}

impl eframe::App for StandalonePreferencesApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        #[cfg(target_os = "windows")]
        if !self.native_window_initialized {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(handle) = frame.window_handle()
                && let RawWindowHandle::Win32(handle) = handle.as_raw()
            {
                let hwnd = handle.hwnd.get() as isize;
                crate::platform::windows::register_window_hwnd(hwnd);
                let _ = crate::platform::windows::set_window_owner(hwnd, self.owner_hwnd);
                self.native_window_initialized = true;
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = frame;
            self.native_window_initialized = true;
        }

        self.apply_appearance(ui.ctx());
        self.watch_external_config(ui.ctx());
        if crate::ui::dialog::escape_pressed(ui.ctx()) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let outcome = egui::CentralPanel::default()
            .show_inside(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 12,
                        right: 12,
                        top: 12,
                        bottom: 2,
                    })
                    .show(ui, |ui| draw_preferences_editor(&mut self.draft, ui))
                    .inner
            })
            .inner;
        if outcome.save {
            save_draft(&mut self.draft, ui.ctx());
        }
        self.apply_appearance(ui.ctx());
        if outcome.close || !self.draft.config.native_dialog_windows {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

pub(crate) fn run_native_preferences(owner_hwnd: isize) -> eframe::Result {
    let config = AppConfig::load();
    crate::platform::interop::set_live_config(config.clone());
    crate::platform::windows::configure_window_composition(
        config.windows_dwm_theming,
        config.windows_mica_backdrop,
    );
    let app_name = crate::config::resolved_app_name(&config);
    let language =
        crate::config::resolve_language(crate::config::resolved_language_preference(&config));
    let title = format!(
        "{} — {app_name}",
        crate::ui::i18n::tr(language, "Preferences")
    );
    let options = eframe::NativeOptions {
        viewport: native_preferences_viewport(title.clone()),
        renderer: eframe::Renderer::Glow,
        glow_options: eframe::egui_glow::GlowConfiguration {
            vsync: config.opengl_vsync,
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        &title,
        options,
        Box::new(move |creation| {
            crate::ui::i18n::configure_ui_fonts(
                &creation.egui_ctx,
                language == AppLanguage::Persian,
            );
            crate::ui::configure_native_visuals(&creation.egui_ctx, &config);
            Ok(Box::new(StandalonePreferencesApp {
                draft: PreferencesDraft::from_live_config(),
                owner_hwnd,
                native_window_initialized: false,
                applied_appearance: None,
                config_watcher: None,
                config_reload_due: None,
            }))
        }),
    )
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_preferences_dialog {
        app.preferences_draft = None;
        return;
    }

    let mut host = crate::ui::dialog::preferred_host(app.native_dialog_windows, cfg!(windows));
    if host == crate::ui::dialog::DialogHost::Native {
        if app.native_preferences.is_none() {
            let owner = app
                .window_handle
                .unwrap_or_else(crate::platform::windows::get_registered_hwnd);
            match NativePreferencesController::spawn(owner, app.preferences_tab) {
                Ok(controller) => app.native_preferences = Some(controller),
                Err(error) => {
                    app.config_status = error;
                    host = crate::ui::dialog::DialogHost::Embedded;
                }
            }
        }
        if app
            .native_preferences
            .as_mut()
            .is_some_and(NativePreferencesController::is_open)
        {
            return;
        }
        if host == crate::ui::dialog::DialogHost::Native {
            app.show_preferences_dialog = false;
            app.native_preferences = None;
            return;
        }
    }

    if let Some(mut controller) = app.native_preferences.take() {
        controller.close();
    }
    let mut draft = app.preferences_draft.take().unwrap_or_else(|| {
        PreferencesDraft::new(app.runtime_config_snapshot(), app.preferences_tab)
    });

    let mut open = app.show_preferences_dialog;
    let bounds = ui.ctx().content_rect().shrink(18.0);
    let max_size = egui::vec2(bounds.width().min(700.0), bounds.height().min(620.0));
    let default_size = egui::vec2(max_size.x.min(620.0), max_size.y.min(520.0));
    let min_size = egui::vec2(max_size.x.min(390.0), max_size.y.min(330.0));
    if crate::ui::dialog::escape_pressed(ui.ctx()) {
        open = false;
    }
    let outcome = egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::GEAR,
        app.tr("Preferences")
    ))
    .id(egui::Id::new("preferences_dialog_bounded_v3"))
    .open(&mut open)
    .default_rect(crate::ui::dialog::centered_default_rect(
        bounds,
        default_size,
    ))
    .min_size(min_size)
    .max_size(max_size)
    .constrain_to(bounds)
    .resizable(true)
    .movable(true)
    .collapsible(false)
    .show(ui.ctx(), |ui| draw_preferences_editor(&mut draft, ui))
    .and_then(|response| response.inner)
    .unwrap_or_default();

    if outcome.save {
        match validate_and_save(&draft.config) {
            Ok(()) => match app.apply_runtime_config(ui.ctx(), draft.config.clone()) {
                Ok(()) => {
                    app.config_fingerprint =
                        AppConfig::fingerprint(&AppConfig::get_config_path()).ok();
                    app.config_status = format!(
                        "Saved {}",
                        crate::config::display_config_path(&AppConfig::get_config_path())
                    );
                    draft.status = app.tr("Preferences saved");
                    draft.mark_saved();
                }
                Err(error) => draft.status = error,
            },
            Err(error) => draft.status = error,
        }
    }
    app.preferences_tab = draft.tab;
    open &= !outcome.close;
    app.show_preferences_dialog = open;
    if open {
        app.preferences_draft = Some(draft);
    }
}

fn validate_and_save(config: &AppConfig) -> Result<(), String> {
    config.validate()?;
    config.save()?;
    crate::platform::interop::set_live_config(config.clone());
    Ok(())
}

fn save_draft(draft: &mut PreferencesDraft, ctx: &egui::Context) {
    match validate_and_save(&draft.config) {
        Ok(()) => {
            draft.status = native_tr(draft.config.language, "Preferences saved");
            draft.mark_saved();
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        }
        Err(error) => draft.status = error,
    }
}

fn native_tr(language: AppLanguage, key: &'static str) -> String {
    crate::ui::i18n::tr(crate::config::resolve_language(language), key)
}

fn draw_preferences_editor(draft: &mut PreferencesDraft, ui: &mut egui::Ui) -> PreferencesOutcome {
    let language = crate::config::resolved_language_preference(&draft.config);
    let tr = |key: &'static str| native_tr(language, key);
    let rtl = crate::config::resolve_language(language) == AppLanguage::Persian;
    let mut outcome = PreferencesOutcome::default();

    egui::Panel::bottom("preferences_footer")
        .resizable(false)
        .show_separator_line(true)
        .show_inside(ui, |ui| {
            ui.add_space(9.0);
            crate::ui::dialog::action_bar(
                ui,
                rtl,
                |ui| {
                    if !draft.status.is_empty() {
                        ui.add(
                            egui::Label::new(egui::RichText::new(&draft.status).small().weak())
                                .truncate(),
                        );
                    }
                },
                |ui| {
                    if crate::ui::dialog::action_button(ui, crate::ui::icons::X, &tr("Close"))
                        .clicked()
                    {
                        outcome.close = true;
                    }
                    let save = ui
                        .add_enabled_ui(draft.is_dirty(), |ui| {
                            crate::ui::dialog::primary_action_button(
                                ui,
                                crate::ui::icons::FLOPPY_DISK,
                                &tr("Save"),
                            )
                        })
                        .inner
                        .on_disabled_hover_text(tr("No changes to save"));
                    if save.clicked() {
                        outcome.save = true;
                    }
                },
            );
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        let sections = preference_sections();
        let narrow = ui.available_width() < 560.0;
        if narrow {
            let active = &sections[draft.tab.min(sections.len() - 1)];
            egui::ComboBox::from_id_salt("preferences_compact_tab")
                .width(ui.available_width())
                .selected_text(format!("{}  {}", section_icon(active.id), tr(active.label)))
                .show_ui(ui, |ui| {
                    for (index, section) in sections.iter().enumerate() {
                        ui.selectable_value(
                            &mut draft.tab,
                            index,
                            format!("{}  {}", section_icon(section.id), tr(section.label)),
                        );
                    }
                });
            ui.add_space(5.0);
        }

        ui.horizontal_top(|ui| {
            if !narrow {
                ui.allocate_ui_with_layout(
                    egui::vec2(PREFERENCES_RAIL_WIDTH, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        for (index, section) in sections.iter().enumerate() {
                            if preferences_tab_button(
                                ui,
                                draft.tab == index,
                                section_icon(section.id),
                                &tr(section.label),
                            )
                            .clicked()
                            {
                                draft.tab = index;
                            }
                        }
                    },
                );
                ui.separator();
            }
            let detail_width = ui.available_width();
            crate::ui::dialog::scroll_column(ui, "preferences_contract_content", None, |ui| {
                ui.set_max_width((detail_width - 10.0).max(180.0));
                ui.spacing_mut().item_spacing.y = 8.0;
                let _ = draw_contract_section(draft, ui, &tr, &sections, rtl);
            });
        });
    });
    outcome
}

fn draw_contract_section(
    draft: &mut PreferencesDraft,
    ui: &mut egui::Ui,
    tr: &impl Fn(&'static str) -> String,
    sections: &[crate::preferences_contract::PreferenceSection],
    rtl_ui: bool,
) -> bool {
    let section = &sections[draft.tab.min(sections.len() - 1)];
    ui.heading(tr(section_heading(section.id)));
    ui.add_space(2.0);
    let controls = preference_controls(&draft.config);
    let mut values = serde_json::to_value(&draft.config).unwrap_or_else(|_| serde_json::json!({}));
    let mut changed = false;
    let mut clear_remote_history = false;
    let mut groups = Vec::new();
    for control in controls
        .iter()
        .filter(|control| control.section == section.id)
    {
        if groups.last().copied() != Some(control.group) {
            groups.push(control.group);
        }
    }
    let section_controls = controls
        .iter()
        .filter(|control| control.section == section.id)
        .collect::<Vec<_>>();
    let label_width = preference_label_column_width(ui, &section_controls, tr);
    for group in groups {
        preference_section(ui, group_icon(group), &tr(group), |ui| {
            ui.spacing_mut().item_spacing.y = PREFERENCE_ROW_GAP;
            for control in controls
                .iter()
                .filter(|control| control.section == section.id && control.group == group)
            {
                changed |=
                    render_contract_control(ui, control, &mut values, tr, label_width, rtl_ui);
            }
            if section.id == "playback" && group == "Open Location / URL" {
                let remote_count = draft
                    .config
                    .recent_media
                    .iter()
                    .filter(|path| crate::media::is_remote_media_target(&path.to_string_lossy()))
                    .count();
                if ui
                    .add_enabled(
                        remote_count > 0,
                        egui::Button::new(format!(
                            "{}  {} ({remote_count})",
                            crate::ui::icons::TRASH,
                            tr("Clear remote history")
                        )),
                    )
                    .clicked()
                {
                    clear_remote_history = true;
                    changed = true;
                }
            }
        });
    }
    if changed {
        match serde_json::from_value::<AppConfig>(values) {
            Ok(config) => draft.config = config,
            Err(error) => {
                draft.status = format!("Invalid preference value: {error}");
                return false;
            }
        }
    }
    if clear_remote_history {
        draft
            .config
            .recent_media
            .retain(|path| !crate::media::is_remote_media_target(&path.to_string_lossy()));
    }
    if section.id == "advanced" {
        draw_advanced_actions(draft, ui, tr);
    }
    changed
}

fn render_contract_control(
    ui: &mut egui::Ui,
    control: &PreferenceControl,
    values: &mut serde_json::Value,
    tr: &impl Fn(&'static str) -> String,
    label_width: f32,
    rtl_ui: bool,
) -> bool {
    let current = value_at_path(values, control.key)
        .cloned()
        .unwrap_or_default();
    let mut replacement = None;
    let mut companion_changed = false;
    let control_icon = preference_control_icon(&control.kind);
    match control.kind {
        PreferenceControlKind::Accent => {
            let selected = current.as_str().unwrap_or("system");
            let selected_label = control
                .options
                .iter()
                .find(|option| option.value.as_str() == Some(selected))
                .map(|option| tr(option.label))
                .unwrap_or_else(|| selected.to_string());
            preference_row(ui, control_icon, &tr(control.label), label_width, |ui| {
                let control_width = ui.available_width().min(PREFERENCE_CONTROL_MAX_WIDTH);
                ui.allocate_ui_with_layout(
                    egui::vec2(control_width, PREFERENCE_ROW_HEIGHT),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        egui::ComboBox::from_id_salt(("preference-accent", control.key))
                            .width(ui.available_width().max(108.0))
                            .selected_text(selected_label)
                            .show_ui(ui, |ui| {
                                ui.set_min_width(310.0);
                                for option in &control.options {
                                    ui.horizontal(|ui| {
                                        let is_custom = option.value.as_str() == Some("custom");
                                        let custom_key =
                                            control.custom_key.unwrap_or("custom_accent_color");
                                        let mut custom_hex = value_at_path(values, custom_key)
                                            .and_then(serde_json::Value::as_str)
                                            .unwrap_or("#0078d4")
                                            .to_string();
                                        let option_color = if is_custom {
                                            crate::config::parse_rgb_hex(&custom_hex)
                                        } else {
                                            option
                                                .color
                                                .as_deref()
                                                .and_then(crate::config::parse_rgb_hex)
                                        };
                                        if !is_custom && let Some(color) = option_color {
                                            color_swatch(ui, color);
                                        }
                                        let label_clicked = ui
                                            .selectable_label(
                                                option.value == current,
                                                tr(option.label),
                                            )
                                            .clicked();

                                        if is_custom {
                                            ui.add_space(6.0);
                                            let mut rgb = crate::config::parse_rgb_hex(&custom_hex)
                                                .unwrap_or([0, 120, 212]);
                                            if ui.color_edit_button_srgb(&mut rgb).changed() {
                                                custom_hex = format!(
                                                    "#{:02X}{:02X}{:02X}",
                                                    rgb[0], rgb[1], rgb[2]
                                                );
                                                companion_changed |= set_value_at_path(
                                                    values,
                                                    custom_key,
                                                    serde_json::Value::String(custom_hex.clone()),
                                                )
                                                .is_ok();
                                                replacement = Some(option.value.clone());
                                            }
                                            let response = ui.add(
                                                egui::TextEdit::singleline(&mut custom_hex)
                                                    .desired_width(84.0)
                                                    .char_limit(7)
                                                    .hint_text("#0078D4"),
                                            );
                                            if response.changed() {
                                                companion_changed |= set_value_at_path(
                                                    values,
                                                    custom_key,
                                                    serde_json::Value::String(
                                                        custom_hex.trim().to_string(),
                                                    ),
                                                )
                                                .is_ok();
                                                replacement = Some(option.value.clone());
                                            }
                                        }

                                        if label_clicked {
                                            replacement = Some(option.value.clone());
                                            ui.close();
                                        }
                                    });
                                }
                            });
                    },
                );
            });
        }
        PreferenceControlKind::Boolean => {
            let stored = current.as_bool().unwrap_or_default();
            let mut displayed = if control.inverted { !stored } else { stored };
            let checkbox_response = preference_checkbox_row(ui, &tr(control.label), &mut displayed);
            if checkbox_response.changed() {
                replacement = Some(serde_json::Value::Bool(if control.inverted {
                    !displayed
                } else {
                    displayed
                }));
            }
        }
        PreferenceControlKind::Select => {
            let selected = current.as_str().unwrap_or_default();
            let selected_label = control
                .options
                .iter()
                .find(|option| option.value.as_str() == Some(selected))
                .map(|option| tr(option.label))
                .unwrap_or_else(|| selected.to_string());
            preference_row(ui, control_icon, &tr(control.label), label_width, |ui| {
                let control_width = ui.available_width().min(PREFERENCE_CONTROL_MAX_WIDTH);
                egui::ComboBox::from_id_salt(("preference", control.key))
                    .width(control_width)
                    .selected_text(selected_label)
                    .show_ui(ui, |ui| {
                        for option in &control.options {
                            if ui
                                .selectable_label(option.value == current, tr(option.label))
                                .clicked()
                            {
                                replacement = Some(option.value.clone());
                            }
                        }
                    });
            });
        }
        PreferenceControlKind::Number => {
            let mut number = current.as_f64().unwrap_or_default();
            let mut slider = egui::Slider::new(
                &mut number,
                control.minimum.unwrap_or(0.0)..=control.maximum.unwrap_or(100.0),
            );
            if let Some(step) = control.step {
                slider = slider.step_by(step);
            }
            if control.logarithmic {
                slider = slider.logarithmic(true);
            }
            let (_, response) =
                preference_row(ui, control_icon, &tr(control.label), label_width, |ui| {
                    let control_width = ui.available_width().min(PREFERENCE_CONTROL_MAX_WIDTH);
                    ui.add_sized([control_width, PREFERENCE_ROW_HEIGHT], slider)
                });
            if response.changed() {
                replacement = Some(if current.is_u64() || current.is_i64() {
                    serde_json::json!(number.round() as u64)
                } else {
                    serde_json::json!(number)
                });
            }
        }
        PreferenceControlKind::Text => {
            let mut text = current.as_str().unwrap_or_default().to_string();
            let text_align = crate::ui::i18n::input_alignment(rtl_ui, &text);
            preference_row(ui, control_icon, &tr(control.label), label_width, |ui| {
                let control_width = ui.available_width().min(PREFERENCE_CONTROL_MAX_WIDTH);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut text)
                        .horizontal_align(text_align)
                        .desired_width(control_width)
                        .hint_text(control.placeholder.unwrap_or_default()),
                );
                if response.changed() {
                    replacement = Some(if text.trim().is_empty() {
                        serde_json::Value::Null
                    } else {
                        serde_json::Value::String(text.trim().to_string())
                    });
                }
            });
        }
    }
    if let Some(description) = control.description {
        ui.label(egui::RichText::new(tr(description)).small().weak());
    }
    replacement
        .is_some_and(|replacement| set_value_at_path(values, control.key, replacement).is_ok())
        || companion_changed
}

fn color_swatch(ui: &mut egui::Ui, [red, green, blue]: [u8; 3]) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
    ui.painter().circle_filled(
        rect.center(),
        5.0,
        egui::Color32::from_rgb(red, green, blue),
    );
    ui.painter().circle_stroke(
        rect.center(),
        5.0,
        egui::Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );
}

#[derive(Clone, Copy)]
enum ConfigPathAction {
    Edit,
    OpenContainingFolder,
}

fn perform_config_path_action(
    action: ConfigPathAction,
    config: &AppConfig,
) -> Result<&'static str, String> {
    let path = AppConfig::get_config_path();
    if !path.exists() {
        config.save()?;
    }
    let target = match action {
        ConfigPathAction::Edit => path.clone(),
        ConfigPathAction::OpenContainingFolder => {
            path.parent()
                .map(std::path::Path::to_path_buf)
                .ok_or_else(|| format!("configuration path has no parent: {}", path.display()))?
        }
    };
    open::that_detached(&target).map_err(|error| format!("open {}: {error}", target.display()))?;
    Ok(match action {
        ConfigPathAction::Edit => "Opened configuration in the external editor",
        ConfigPathAction::OpenContainingFolder => "Opened the configuration folder",
    })
}

fn draw_advanced_actions(
    draft: &mut PreferencesDraft,
    ui: &mut egui::Ui,
    tr: &impl Fn(&'static str) -> String,
) {
    preference_section(
        ui,
        crate::ui::icons::FILE_VIDEO,
        &tr("File associations"),
        |ui| {
            let registered = crate::platform::associations::SUPPORTED_EXTENSIONS
                .iter()
                .filter(|extension| {
                    crate::platform::associations::is_file_association_registered(extension)
                })
                .count();
            ui.label(format!(
                "{}: {registered}/{}",
                tr("Registered media types"),
                crate::platform::associations::SUPPORTED_EXTENSIONS.len()
            ));
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(format!(
                        "{}  {}",
                        crate::ui::icons::CHECK_SQUARE,
                        tr("Register as a media player")
                    ))
                    .clicked()
                {
                    draft.status =
                        match crate::platform::associations::register_file_associations(None) {
                            Ok(count) => format!("{}: {count}", tr("Registered media types")),
                            Err(error) => error,
                        };
                }
                if ui
                    .add_enabled(
                        registered > 0,
                        egui::Button::new(format!(
                            "{}  {}",
                            crate::ui::icons::X,
                            tr("Remove file associations")
                        )),
                    )
                    .clicked()
                {
                    draft.status =
                        match crate::platform::associations::unregister_file_associations() {
                            Ok(count) => format!("{}: {count}", tr("Removed media types")),
                            Err(error) => error,
                        };
                }
            });
        },
    );
    preference_section(
        ui,
        crate::ui::icons::FLOPPY_DISK,
        &tr("Config file"),
        |ui| {
            let path = AppConfig::get_config_path();
            let response = ui
                .add(
                    egui::Label::new(
                        egui::RichText::new(crate::config::display_config_path(&path)).monospace(),
                    )
                    .wrap()
                    .selectable(false)
                    .sense(egui::Sense::click()),
                )
                .on_hover_text(tr("Edit config file in an external editor"));
            let mut path_action = response.clicked().then_some(ConfigPathAction::Edit);
            response.context_menu(|ui| {
                if ui
                    .button(format!(
                        "{}  {}",
                        crate::ui::icons::PENCIL_SIMPLE,
                        tr("Edit config file")
                    ))
                    .clicked()
                {
                    path_action = Some(ConfigPathAction::Edit);
                    ui.close();
                }
                if ui
                    .button(format!(
                        "{}  {}",
                        crate::ui::icons::FOLDER_OPEN,
                        tr("Open containing folder")
                    ))
                    .clicked()
                {
                    path_action = Some(ConfigPathAction::OpenContainingFolder);
                    ui.close();
                }
            });
            if let Some(action) = path_action {
                draft.status = match perform_config_path_action(action, &draft.saved_config) {
                    Ok(status) => tr(status),
                    Err(error) => error,
                };
            }
            if ui
                .button(format!(
                    "{}  {}",
                    crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                    tr("Reload from disk")
                ))
                .clicked()
            {
                match AppConfig::load_from_path(&AppConfig::get_config_path()) {
                    Ok(config) => {
                        draft.replace_from_disk(config, tr("Preferences reloaded from disk"));
                    }
                    Err(error) => draft.status = error,
                }
            }
        },
    );
}

fn preference_row<R>(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    desired_label_width: f32,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> (egui::Response, R) {
    let width = ui.available_width();
    let Some(label_width) = inline_preference_label_width(width, desired_label_width) else {
        return ui
            .vertical(|ui| {
                let label_response = ui
                    .horizontal_wrapped(|ui| {
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(icon)
                                .size(14.0)
                                .color(ui.visuals().selection.bg_fill),
                        );
                        ui.add(egui::Label::new(egui::RichText::new(label).size(13.0)).wrap());
                    })
                    .response;
                ui.add_space(2.0);
                let body_result = ui
                    .allocate_ui_with_layout(
                        egui::vec2(width, PREFERENCE_ROW_HEIGHT),
                        egui::Layout::left_to_right(egui::Align::Center),
                        body,
                    )
                    .inner;
                (label_response, body_result)
            })
            .inner;
    };
    ui.allocate_ui_with_layout(
        egui::vec2(width, PREFERENCE_ROW_HEIGHT),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = PREFERENCE_COLUMN_GAP;
            let label_response = ui
                .allocate_ui_with_layout(
                    egui::vec2(label_width, PREFERENCE_ROW_HEIGHT),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(icon)
                                .size(14.0)
                                .color(ui.visuals().selection.bg_fill),
                        );
                        ui.label(egui::RichText::new(label).size(13.0));
                    },
                )
                .response;
            (label_response, body(ui))
        },
    )
    .inner
}

fn preference_checkbox_row(ui: &mut egui::Ui, label: &str, checked: &mut bool) -> egui::Response {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), PREFERENCE_ROW_HEIGHT),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            // Keep the real checkbox and its caption in one widget. Rendering a
            // decorative checkbox icon in the label column and an empty checkbox
            // in the value column let responsive stacking split them across two
            // lines, making the visible icon look like a non-interactive control.
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            ui.add(egui::Checkbox::new(
                checked,
                egui::RichText::new(label).size(13.0),
            ))
        },
    )
    .inner
}

fn inline_preference_label_width(row_width: f32, desired_label_width: f32) -> Option<f32> {
    let maximum = (row_width - PREFERENCE_CONTROL_MIN_WIDTH - PREFERENCE_COLUMN_GAP).max(128.0);
    (desired_label_width <= maximum).then_some(desired_label_width.min(maximum))
}

fn preference_label_column_width(
    ui: &egui::Ui,
    controls: &[&PreferenceControl],
    tr: &impl Fn(&'static str) -> String,
) -> f32 {
    let text_width = ui.fonts_mut(|fonts| {
        controls
            .iter()
            .map(|control| {
                fonts
                    .layout_no_wrap(
                        tr(control.label),
                        egui::FontId::proportional(13.0),
                        egui::Color32::WHITE,
                    )
                    .rect
                    .width()
            })
            .fold(0.0_f32, f32::max)
    });
    // Left inset + icon + icon/text gap. The value is content-derived, not a
    // clipping width, so localized captions stay readable and all controls in
    // the active section still begin on the same vertical guide.
    (text_width + 34.0).max(128.0)
}

fn preference_control_icon(kind: &PreferenceControlKind) -> &'static str {
    match kind {
        PreferenceControlKind::Accent => crate::ui::icons::PALETTE,
        PreferenceControlKind::Boolean => crate::ui::icons::CHECK_SQUARE,
        PreferenceControlKind::Number => crate::ui::icons::SLIDERS_HORIZONTAL,
        PreferenceControlKind::Select => crate::ui::icons::LIST_CHECKS,
        PreferenceControlKind::Text => crate::ui::icons::PENCIL_SIMPLE,
    }
}

fn preference_section(
    ui: &mut egui::Ui,
    icon: &str,
    title: &str,
    body: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color.gamma_multiply(0.42))
        .stroke(egui::Stroke::new(
            1.0_f32,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(9.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(format!("{icon}  {title}"))
                    .size(15.0)
                    .strong()
                    .color(ui.visuals().widgets.hovered.fg_stroke.color),
            );
            ui.add_space(8.0);
            body(ui);
        });
    ui.add_space(7.0);
}

fn preferences_tab_button(
    ui: &mut egui::Ui,
    selected: bool,
    icon: &str,
    label: &str,
) -> egui::Response {
    crate::ui::dialog::navigation_button(ui, selected, icon, label, ui.available_width())
}

fn section_icon(section: &str) -> &'static str {
    match section {
        "appearance" => crate::ui::icons::SPARKLE,
        "playback" => crate::ui::icons::PLAY,
        "hardware" => crate::ui::icons::PLUG,
        "input" => crate::ui::icons::SLIDERS_HORIZONTAL,
        _ => crate::ui::icons::GEAR,
    }
}

fn group_icon(group: &str) -> &'static str {
    match group {
        "Interface" => crate::ui::icons::SPARKLE,
        "On-screen display" => crate::ui::icons::MONITOR_PLAY,
        "Player controls" => crate::ui::icons::PLAY,
        "Open Location / URL" => crate::ui::icons::LINK_SIMPLE,
        "Connection" => crate::ui::icons::PLUG,
        "Motion controls" => crate::ui::icons::SEAT,
        "Video surface" => crate::ui::icons::SELECTION_ALL,
        "Status bar" => crate::ui::icons::GAUGE,
        "Windows graphics and composition" | "Dialog windows" => crate::ui::icons::APP_WINDOW,
        _ => crate::ui::icons::GEAR,
    }
}

fn section_heading(section: &str) -> &'static str {
    match section {
        "appearance" => "Appearance and language",
        "playback" => "Playback behavior",
        "hardware" => "PCController and hardware",
        "input" => "Mouse and gesture bindings",
        _ => "Configuration",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_helper_bypasses_the_primary_single_instance_path() {
        let args = vec![
            "pealayer.exe".to_string(),
            "--preferences-helper".to_string(),
            "12345".to_string(),
            "--preferences-tab".to_string(),
            "2".to_string(),
        ];
        assert_eq!(preferences_helper_owner(&args), Some(12345));
        assert_eq!(preferences_helper_tab(&args), 2);
        assert_eq!(
            preferences_helper_owner(&["pealayer.exe".to_string()]),
            None
        );
    }

    #[test]
    fn native_preferences_use_owned_tool_window_chrome() {
        let builder = native_preferences_viewport("Preferences — Pealayer".to_string());
        assert_eq!(builder.taskbar, Some(false));
        assert_eq!(builder.minimize_button, Some(false));
        assert_eq!(builder.maximize_button, Some(false));
        assert_eq!(builder.resizable, Some(true));
    }

    #[test]
    fn native_preferences_use_a_dedicated_phosphor_style_icon() {
        let icon = phosphor_preferences_icon().expect("preferences icon");
        assert_eq!((icon.width, icon.height), (32, 32));
        assert!(icon.rgba.chunks_exact(4).any(|pixel| pixel[3] == 255));
        assert!(icon.rgba.chunks_exact(4).any(|pixel| pixel[3] == 0));
    }

    #[test]
    fn preferences_uses_the_shared_dialog_control_system() {
        let source = include_str!("preferences.rs");
        assert!(source.contains("dialog::navigation_button"));
        assert!(source.contains("dialog::action_button"));
        assert!(source.contains("dialog::primary_action_button"));
        assert!(!source.contains(concat!("const PREFERENCES_", "TAB_HEIGHT")));
        assert!(!source.contains(concat!("const PREFERENCES_", "ACTION_HEIGHT")));
        assert!(PREFERENCES_RAIL_WIDTH >= 110.0);
        assert!(PREFERENCE_ROW_HEIGHT > crate::ui::dialog::NAVIGATION_HEIGHT);
        assert_eq!(PREFERENCE_ROW_GAP, 3.0);
        assert!(PREFERENCE_ROW_HEIGHT + PREFERENCE_ROW_GAP < 37.0);
        assert!(PREFERENCE_COLUMN_GAP <= 6.0);
        assert!(PREFERENCE_CONTROL_MAX_WIDTH >= 400.0);
    }

    #[test]
    fn preference_captions_expand_for_content_and_stack_before_clipping() {
        assert_eq!(inline_preference_label_width(700.0, 286.0), Some(286.0));
        assert_eq!(inline_preference_label_width(420.0, 286.0), None);
    }

    #[test]
    fn boolean_preferences_use_one_real_inline_checkbox() {
        let source = include_str!("preferences.rs");
        let boolean_branch = source
            .split_once("PreferenceControlKind::Boolean => {")
            .expect("boolean preference branch")
            .1
            .split_once("PreferenceControlKind::Select => {")
            .expect("select preference branch")
            .0;
        assert!(boolean_branch.contains("preference_checkbox_row"));
        assert!(!boolean_branch.contains("preference_row("));
        assert!(!boolean_branch.contains("CHECK_SQUARE"));
    }

    #[test]
    fn save_state_tracks_real_unsaved_preference_changes() {
        let mut draft = PreferencesDraft::new(AppConfig::default(), 0);
        assert!(!draft.is_dirty());
        draft.config.pin_controls = !draft.config.pin_controls;
        assert!(draft.is_dirty());
        draft.mark_saved();
        assert!(!draft.is_dirty());
    }

    #[test]
    fn config_path_is_clickable_and_has_both_requested_actions() {
        let source = include_str!("preferences.rs");
        assert!(source.contains("response.clicked()"));
        assert!(source.contains("ConfigPathAction::Edit"));
        assert!(source.contains("ConfigPathAction::OpenContainingFolder"));
        assert!(source.contains("add_enabled_ui(draft.is_dirty()"));
    }
}
