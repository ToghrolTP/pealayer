use crate::app::PealayerApp;
use crate::config::{AppConfig, AppLanguage, AppTheme};
use crate::preferences_contract::{
    PreferenceControl, PreferenceControlKind, preference_controls, preference_sections,
    set_value_at_path, value_at_path,
};
use eframe::egui;

const PREFERENCES_RAIL_WIDTH: f32 = 118.0;
const PREFERENCE_ROW_HEIGHT: f32 = 36.0;
const PREFERENCE_ROW_GAP: f32 = 1.0;
const PREFERENCE_LABEL_WIDTH: f32 = 176.0;
const PREFERENCE_COLUMN_GAP: f32 = 6.0;
const PREFERENCE_CONTROL_MAX_WIDTH: f32 = 420.0;

pub(crate) struct NativePreferencesController {
    child: std::process::Child,
}

#[derive(Clone)]
pub(crate) struct PreferencesDraft {
    config: AppConfig,
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
            config,
            tab: tab.min(preference_sections().len().saturating_sub(1)),
            status: String::new(),
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
        vsync: config.opengl_vsync,
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
                    app.config_status = format!("Saved {}", AppConfig::get_config_path().display());
                    draft.status = app.tr("Preferences saved");
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
            draft.status = format!(
                "{} {}",
                native_tr(draft.config.language, "Saved"),
                AppConfig::get_config_path().display()
            );
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
                    if crate::ui::dialog::primary_action_button(
                        ui,
                        crate::ui::icons::FLOPPY_DISK,
                        &tr("Save"),
                    )
                    .clicked()
                    {
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
                outcome.save |= draw_contract_section(draft, ui, &tr, &sections);
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
) -> bool {
    let section = &sections[draft.tab.min(sections.len() - 1)];
    ui.heading(tr(section_heading(section.id)));
    ui.add_space(2.0);
    let controls = preference_controls();
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
    for group in groups {
        preference_section(ui, group_icon(group), &tr(group), |ui| {
            ui.spacing_mut().item_spacing.y = PREFERENCE_ROW_GAP;
            for control in controls
                .iter()
                .filter(|control| control.section == section.id && control.group == group)
            {
                changed |= render_contract_control(ui, control, &mut values, tr);
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
) -> bool {
    let current = value_at_path(values, control.key)
        .cloned()
        .unwrap_or_default();
    let mut replacement = None;
    let control_icon = preference_control_icon(&control.kind);
    match control.kind {
        PreferenceControlKind::Boolean => {
            let stored = current.as_bool().unwrap_or_default();
            let mut displayed = if control.inverted { !stored } else { stored };
            let (label_response, checkbox_response) =
                preference_row(ui, control_icon, &tr(control.label), |ui| {
                    ui.checkbox(&mut displayed, "")
                });
            let label_clicked = label_response
                .interact(egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked();
            if label_clicked {
                displayed = !displayed;
            }
            if checkbox_response.changed() || label_clicked {
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
            preference_row(ui, control_icon, &tr(control.label), |ui| {
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
            let (_, response) = preference_row(ui, control_icon, &tr(control.label), |ui| {
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
            preference_row(ui, control_icon, &tr(control.label), |ui| {
                let control_width = ui.available_width().min(PREFERENCE_CONTROL_MAX_WIDTH);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut text)
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
            ui.add(
                egui::Label::new(
                    egui::RichText::new(AppConfig::get_config_path().display().to_string())
                        .monospace(),
                )
                .wrap()
                .selectable(true),
            );
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
                        draft.config = config;
                        draft.status = tr("Preferences reloaded from disk");
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
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> (egui::Response, R) {
    let width = ui.available_width();
    let label_width = PREFERENCE_LABEL_WIDTH.min((width * 0.42).max(128.0));
    ui.allocate_ui_with_layout(
        egui::vec2(width, PREFERENCE_ROW_HEIGHT),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = PREFERENCE_COLUMN_GAP;
            let (label_rect, label_response) = ui.allocate_exact_size(
                egui::vec2(label_width, PREFERENCE_ROW_HEIGHT),
                egui::Sense::hover(),
            );
            let painter = ui.painter().with_clip_rect(label_rect);
            painter.text(
                egui::pos2(label_rect.left() + 8.0, label_rect.center().y),
                egui::Align2::CENTER_CENTER,
                icon,
                egui::FontId::proportional(14.0),
                ui.visuals().selection.bg_fill,
            );
            painter.text(
                egui::pos2(label_rect.left() + 22.0, label_rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(13.0),
                ui.visuals().text_color(),
            );
            (label_response, body(ui))
        },
    )
    .inner
}

fn preference_control_icon(kind: &PreferenceControlKind) -> &'static str {
    match kind {
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
        assert!(PREFERENCE_ROW_GAP <= 1.0);
        assert!(PREFERENCE_COLUMN_GAP <= 6.0);
        assert!(PREFERENCE_CONTROL_MAX_WIDTH >= 400.0);
    }
}
