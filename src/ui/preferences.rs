use crate::ui::dropdown::DropdownUiExt;
use crate::app::PealayerApp;
use crate::config::{AppConfig, AppLanguage, AppTheme};
use crate::preferences_contract::{
    PreferenceControl, PreferenceControlKind, preference_controls, preference_sections,
    set_value_at_path, value_at_path,
};
use eframe::egui;
use crate::preferences_contract::PreferencesNavigation;

const PREFERENCES_RAIL_WIDTH: f32 = 118.0;
const PREFERENCE_ROW_HEIGHT: f32 = 33.0;
const PREFERENCE_ROW_GAP: f32 = 3.0;
const PREFERENCE_COLUMN_GAP: f32 = 6.0;
const PREFERENCE_CONTROL_MAX_WIDTH: f32 = 420.0;
const PREFERENCE_CONTROL_MIN_WIDTH: f32 = 180.0;
const REPLACEMENT_ARROW_WIDTH: f32 = 14.0;
const REPLACEMENT_DELETE_WIDTH: f32 = 26.0;
const REPLACEMENT_FIELD_MIN_WIDTH: f32 = 72.0;
const REPLACEMENT_ROW_MIN_WIDTH: f32 = REPLACEMENT_ARROW_WIDTH
    + REPLACEMENT_DELETE_WIDTH
    + PREFERENCE_COLUMN_GAP * 3.0
    + REPLACEMENT_FIELD_MIN_WIDTH * 2.0;

pub(crate) struct NativePreferencesController {
    child: std::process::Child,
    navigation: std::sync::mpsc::SyncSender<PreferencesHelperMessage>,
    branding_revision: Option<u64>,
    exit_verified: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum PreferencesHelperMessage {
    Navigate { navigation: PreferencesNavigation },
    Branding { effective: serde_json::Value, configured: serde_json::Value },
}

#[derive(Clone)]
pub(crate) struct PreferencesDraft {
    config: AppConfig,
    saved_config: AppConfig,
    previewed_config: AppConfig,
    tab: usize,
    status: String,
    status_until: Option<std::time::Instant>,
    confirm_close: bool,
    pending_navigation: Option<PreferencesNavigation>,
    scroll_y: f32,
}

#[derive(Default)]
struct PreferencesOutcome {
    close: bool,
    save: bool,
    close_after_save: bool,
    discard: bool,
}

impl PreferencesOutcome {
    fn request_save(&mut self) {
        self.save = true;
        self.close_after_save = true;
    }
}

impl PreferencesDraft {
    fn new(config: AppConfig, tab: usize) -> Self {
        Self {
            saved_config: config.clone(),
            previewed_config: config.clone(),
            config,
            tab: tab.min(preference_sections().len().saturating_sub(1)),
            status: String::new(),
            status_until: None,
            confirm_close: false,
            pending_navigation: None,
            scroll_y: 0.0,
        }
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.config != self.saved_config
    }

    fn mark_saved(&mut self) {
        self.saved_config = self.config.clone();
    }

    fn restore_saved(&mut self) {
        self.config = self.saved_config.clone();
        self.confirm_close = false;
    }

    fn take_preview_update(&mut self) -> Option<AppConfig> {
        if self.config == self.previewed_config {
            return None;
        }
        self.previewed_config = self.config.clone();
        Some(self.config.clone())
    }

    fn set_transient_status(&mut self, status: String) {
        self.status = status;
        self.status_until = Some(std::time::Instant::now() + std::time::Duration::from_secs(2));
    }

    fn replace_from_disk(&mut self, config: AppConfig, status: String) {
        self.saved_config = config.clone();
        self.previewed_config = config.clone();
        self.config = config;
        self.set_transient_status(status);
    }

    pub(crate) fn apply_external_config(
        &mut self,
        config: AppConfig,
        status: String,
        conflict_status: String,
    ) {
        if self.is_dirty() {
            // Appearance is shared live, even while unrelated preferences are
            // being edited. Keep that draft, but do not leave its theme controls
            // displaying values which no longer match the host and Web UI.
            self.sync_external_appearance(&config);
            self.status = conflict_status;
        } else {
            self.replace_from_disk(config, status);
        }
    }

    pub(crate) fn sync_external_appearance(&mut self, config: &AppConfig) {
        for target in [
            &mut self.config,
            &mut self.saved_config,
            &mut self.previewed_config,
        ] {
            target.copy_appearance_from(config);
        }
    }

    pub(crate) fn sync_external_branding(&mut self, values: &serde_json::Value) {
        let Ok(source) = AppConfig::default().apply_patch(values) else { return; };
        for target in [&mut self.config, &mut self.saved_config, &mut self.previewed_config] {
            // Do not validate unrelated in-progress fields: a partial draft
            // must not prevent a committed branding update reaching this UI.
            macro_rules! copy {
                ($($field:ident),*) => { $(if values.get(stringify!($field)).is_some() { target.$field = source.$field.clone(); })* };
            }
            copy!(app_name, app_description, app_icon, app_icon_preset, app_icon_playing, app_icon_paused, app_icon_stopped);
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
    fn spawn(owner_hwnd: isize, tab: usize, ctx: &egui::Context) -> Result<Self, String> {
        let executable = std::env::current_exe()
            .map_err(|error| format!("resolve Pealayer executable: {error}"))?;
        let mut child = std::process::Command::new(executable)
            .arg("--preferences-helper")
            .arg(owner_hwnd.to_string())
            .arg("--preferences-tab")
            .arg(tab.to_string())
            .arg("--preferences-owner-identity")
            .arg(ctx.data(|data| data.get_temp::<String>(egui::Id::new("native-listener-identity")))
                .unwrap_or_else(crate::cli::resolved_instance_identity))
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|error| format!("open native Preferences window: {error}"))?;
        let mut input = child.stdin.take().expect("piped Preferences input");
        let (navigation, requests) = std::sync::mpsc::sync_channel::<PreferencesHelperMessage>(8);
        // Never block the UI on the child or a full OS pipe. Only typed,
        // bounded navigation/branding messages travel here, not commands.
        if let Err(error) = std::thread::Builder::new().name("preferences-navigation".into()).spawn(move || {
            use std::io::Write;
            for request in requests {
                let Ok(mut line) = serde_json::to_vec(&request) else { continue; };
                line.push(b'\n');
                if input.write_all(&line).is_err() { break; }
            }
        }) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("start Preferences navigation: {error}"));
        }
        crate::diagnostics::record_shell_action("helper_spawned", "preferences");
        let exit_verified = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::{Foundation::{CloseHandle, WAIT_OBJECT_0}, System::Threading::{
                OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject, INFINITE,
            }};
            // A closed helper must cause a parent frame even when playback is
            // paused. Wait on an independent OS handle, not a Child mutex that
            // would block the GUI's try_wait(). No repaint polling is needed.
            if let Ok(handle) = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, child.id()) } {
                let process_handle = handle.0 as usize;
                let repaint = ctx.clone();
                let exited = exit_verified.clone();
                let watcher = std::thread::Builder::new()
                    .name("pealayer-preferences-exit".to_owned())
                    .spawn(move || {
                        let handle = windows::Win32::Foundation::HANDLE(process_handle as *mut _);
                        let status = unsafe { WaitForSingleObject(handle, INFINITE) };
                        if status == WAIT_OBJECT_0 {
                            exited.store(true, std::sync::atomic::Ordering::Release);
                        }
                        unsafe { let _ = CloseHandle(handle); }
                        crate::platform::interop::wake_command_dispatcher(&repaint);
                    });
                if watcher.is_err() { unsafe { let _ = CloseHandle(handle); } }
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = ctx;
        Ok(Self { child, navigation, branding_revision: None, exit_verified })
    }

    fn is_open(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(Some(_)) => {
                self.exit_verified.store(true, std::sync::atomic::Ordering::Release);
                false
            }
            Ok(None) | Err(_) => true, // An uninspectable child is not closed.
        }
    }

    pub(crate) fn confirmed_exited(&self) -> bool {
        self.exit_verified.load(std::sync::atomic::Ordering::Acquire)
    }

    fn close(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// All UI/IPC entry points share the same native/embedded lifecycle.
pub(crate) fn open(app: &mut PealayerApp, ctx: &egui::Context) {
    crate::diagnostics::record_shell_action("gui_open_requested", "preferences");
    if reap_exited_controller(&mut app.native_preferences) {
        if let Err(error) = app.cancel_preference_preview(ctx) { app.config_status = error; }
    }
    app.show_preferences_dialog = true;
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    ctx.request_repaint();
}

pub(crate) fn navigate(app: &mut PealayerApp, ctx: &egui::Context, navigation: PreferencesNavigation) {
    if let Err(error) = navigation.validate() { app.config_status = error; return; }
    open(app, ctx);
    if app.preferences_draft.is_none() {
        app.preferences_draft = Some(PreferencesDraft::new(app.runtime_config_snapshot(), app.preferences_tab));
    }
    app.preferences_draft.as_mut().unwrap().pending_navigation = Some(navigation);
}

fn receive_navigation(ctx: egui::Context) -> std::sync::mpsc::Receiver<PreferencesHelperMessage> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(8);
    std::thread::spawn(move || {
        use std::io::{BufRead, Read};
        let input = std::io::stdin();
        let mut input = input.lock();
        loop {
            // Navigation and branding are bounded typed records, never commands
            // or arbitrary config. Artwork travels as a path, not image bytes.
            let mut bytes = Vec::new();
            let mut bounded = (&mut input).take(65537);
            if bounded.read_until(b'\n', &mut bytes).ok().filter(|n| *n > 0).is_none() { break; }
            if bytes.len() > 65536 { break; }
            let Ok(request) = serde_json::from_slice::<PreferencesHelperMessage>(&bytes) else { continue; };
            let valid = match &request {
                PreferencesHelperMessage::Navigate { navigation } => navigation.validate(),
                PreferencesHelperMessage::Branding { effective, configured } =>
                    crate::branding::validate_branding_values(effective).and_then(|()| crate::branding::validate_branding_values(configured)),
            };
            if valid.is_ok() {
                if sender.send(request).is_err() { break; }
                ctx.request_repaint();
            }
        }
    });
    receiver
}

fn reap_exited_controller(controller: &mut Option<NativePreferencesController>) -> bool {
    let exited = controller.as_mut().is_some_and(|controller| !controller.is_open());
    if exited { *controller = None; }
    exited
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
    let icon = crate::branding::icon_bytes(&crate::platform::interop::get_live_config(), crate::branding::PlaybackIconState::Stopped)
        .and_then(|(_, bytes)| crate::branding::icon_data_from_bytes(&bytes));
    if let Some(icon) = icon {
        builder = builder.with_icon(icon);
    }
    builder
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
    owner_identity: String,
    native_window_initialized: bool,
    applied_appearance: Option<(AppTheme, bool, bool, [u8; 3], crate::config::ColorPalette,
        crate::config::UiControlDensity, crate::config::UiAnimationMode,
        crate::config::UiFontFamily, crate::config::UiFontFamily)>,
    config_watcher: Option<crate::config::ConfigFileWatcher>,
    config_reload_due: Option<std::time::Instant>,
    branding_config: Option<AppConfig>,
    navigation: std::sync::mpsc::Receiver<PreferencesHelperMessage>,
}

impl StandalonePreferencesApp {
    fn send_owner_command(&mut self, command: crate::platform::interop::InteropCommand) -> bool {
        let result = serde_json::to_string(&command)
            .map_err(|error| format!("serialize live preference preview: {error}"))
            .and_then(|payload| {
                crate::platform::interop::send_native_request(
                    &payload,
                    &self.owner_identity,
                    std::time::Duration::from_millis(750),
                )
            }).and_then(|reply| {
                let value: serde_json::Value = serde_json::from_str(&reply).map_err(|error| error.to_string())?;
                if let Some(error) = value.get("error") { return Err(format!("Preferences preview rejected: {error}")); }
                Ok(())
            });
        if let Err(error) = result {
            self.draft.status = error;
            self.draft.status_until = None;
            return false;
        }
        true
    }

    fn preview_changes(&mut self) {
        let previous = self.draft.previewed_config.clone();
        let Some(config) = self.draft.take_preview_update() else {
            return;
        };
        if !self.send_owner_command(crate::platform::interop::InteropCommand::PreviewConfig {
            config: Box::new(config),
        }) { self.draft.previewed_config = previous; }
    }

    fn finish_preview(&mut self, commit: bool) -> bool {
        let command = if commit {
            crate::platform::interop::InteropCommand::CommitPreviewConfig {
                config: Box::new(self.draft.config.clone()),
            }
        } else {
            crate::platform::interop::InteropCommand::CancelPreviewConfig
        };
        let sent = self.send_owner_command(command);
        if sent {
            self.draft.previewed_config = self.draft.config.clone();
        }
        sent
    }

    fn apply_appearance(&mut self, ctx: &egui::Context) {
        if self.branding_config.as_ref() != Some(&self.draft.config) {
            let config = &self.draft.config;
            let name = crate::config::resolved_app_name(config);
            let language = crate::config::resolve_language(crate::config::resolved_language_preference(config));
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!("{} — {name}", native_tr(language, "Preferences"))));
            crate::branding::apply_native_window_icon(ctx, config, crate::branding::PlaybackIconState::Stopped);
            self.branding_config = Some(config.clone());
        }
        let appearance = (
            crate::config::resolved_theme(&self.draft.config),
            self.draft.config.windows_dwm_theming,
            self.draft.config.windows_mica_backdrop,
            crate::ui::platform_accent_rgb(&self.draft.config),
            self.draft.config.color_palette,
            self.draft.config.ui_control_density,
            self.draft.config.ui_animation_mode,
            self.draft.config.ui_font_family,
            self.draft.config.monospace_font_family,
        );
        if self.applied_appearance != Some(appearance) {
            crate::ui::configure_native_appearance(ctx, &self.draft.config);
            crate::platform::windows::configure_window_composition(appearance.1, appearance.2);
            self.applied_appearance = Some(appearance);
        }
        // Native OS changes do not dirty the draft or alter the cached
        // preference tuple. Still refresh this independently hosted titlebar.
        crate::ui::sync_native_window_appearance(ctx, appearance.4);
        crate::ui::i18n::configure_preferred_fonts(ctx, &self.draft.config);
    }

    fn watch_external_config(&mut self, ctx: &egui::Context) {
        if !self.draft.config.auto_reload_config {
            self.config_watcher = None;
            self.config_reload_due = None;
            return;
        }
        if self.config_watcher.is_none() {
            // Resolve storage mode only when attaching the watcher. Without an
            // explicit config override it probes portable markers on disk;
            // doing that on every wheel frame can block on the filesystem.
            let path = AppConfig::get_config_path();
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
        match AppConfig::load_from_path(&AppConfig::get_config_path()) {
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
                crate::diagnostics::record_shell_action("helper_window_ready", "preferences");
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = frame;
            self.native_window_initialized = true;
        }

        for request in self.navigation.try_iter().take(8) {
            match request {
                PreferencesHelperMessage::Navigate { navigation } => {
                    self.draft.pending_navigation = Some(navigation);
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                PreferencesHelperMessage::Branding { effective, configured } => {
                    if crate::branding::set_runtime_values(&effective).is_ok() {
                        self.draft.sync_external_branding(&configured);
                        self.branding_config = None;
                    }
                }
            }
        }
        self.apply_appearance(ui.ctx());
        self.watch_external_config(ui.ctx());
        let close_requested = ui.ctx().input(|input| input.viewport().close_requested());
        if close_requested && self.draft.is_dirty() {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.draft.confirm_close = true;
        }
        if crate::ui::dialog::escape_pressed(ui.ctx()) {
            if self.draft.is_dirty() {
                self.draft.confirm_close = true;
            } else {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
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
            let saved = save_draft(&mut self.draft, ui.ctx());
            let committed = saved && self.finish_preview(true);
            if committed && outcome.close_after_save {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if outcome.discard {
            self.draft.restore_saved();
            if self.finish_preview(false) {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }
        self.preview_changes();
        self.apply_appearance(ui.ctx());
        if outcome.close {
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
    let effective_vsync = crate::platform::linux::resolve_effective_opengl_vsync(
        config.opengl_vsync,
        crate::platform::linux::is_wayland_session(),
    );
    let options = eframe::NativeOptions {
        viewport: native_preferences_viewport(title.clone()),
        renderer: eframe::Renderer::Glow,
        persistence_path: Some(
            AppConfig::get_config_path().with_file_name("preferences-window-state.ron"),
        ),
        glow_options: eframe::egui_glow::GlowConfiguration {
            vsync: effective_vsync,
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
            crate::ui::configure_native_appearance(&creation.egui_ctx, &config);
            Ok(Box::new(StandalonePreferencesApp {
                draft: PreferencesDraft::from_live_config(),
                owner_hwnd,
                owner_identity: std::env::args().collect::<Vec<_>>().windows(2)
                    .find(|pair| pair[0] == "--preferences-owner-identity")
                    .map(|pair| pair[1].clone()).unwrap_or_else(crate::cli::resolved_instance_identity),
                native_window_initialized: false,
                applied_appearance: None,
                config_watcher: None,
                config_reload_due: None,
                branding_config: None,
                navigation: receive_navigation(creation.egui_ctx.clone()),
            }))
        }),
    )
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    // Preview/config reconciliation can hide this dialog before its helper
    // exits. Reap independently of visibility, otherwise inspection retains a
    // permanently unknown draft even after the native window was closed.
    if reap_exited_controller(&mut app.native_preferences) {
        if let Err(error) = app.cancel_preference_preview(ui.ctx()) { app.config_status = error; }
        app.show_preferences_dialog = false;
    }
    if !app.show_preferences_dialog {
        app.preferences_draft = None;
        return;
    }

    let mut host = crate::ui::dialog::preferred_host(app.native_dialog_windows, cfg!(windows));
    // A helper without the peer authority would edit the local profile.
    // Keep the exact same editor in-process until that helper is connected.
    if crate::peer::active(){host=crate::ui::dialog::DialogHost::Embedded;}
    if host == crate::ui::dialog::DialogHost::Native {
        if app.native_preferences.is_none() {
            let owner = app
                .window_handle
                .unwrap_or_else(crate::platform::windows::get_registered_hwnd);
            match NativePreferencesController::spawn(owner, app.preferences_tab, ui.ctx()) {
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
            if let Some(request) = app.preferences_draft.as_mut().and_then(|draft| draft.pending_navigation.take()) {
                match app.native_preferences.as_ref().unwrap().navigation.try_send(PreferencesHelperMessage::Navigate { navigation: request }) {
                    Ok(()) => {},
                    Err(std::sync::mpsc::TrySendError::Full(PreferencesHelperMessage::Navigate { navigation })) => {
                        app.preferences_draft.as_mut().unwrap().pending_navigation = Some(navigation);
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(20));
                    }
                    Err(error) => app.config_status = format!("Preferences navigation not delivered: {error}"),
                }
            }
            let controller = app.native_preferences.as_mut().unwrap();
            let revision = crate::platform::interop::live_config_revision();
            if controller.branding_revision != Some(revision) {
                let config = crate::platform::interop::get_live_config();
                let message = PreferencesHelperMessage::Branding {
                    effective: crate::branding::visible_values(&config),
                    configured: crate::branding::configured_values(&config),
                };
                match controller.navigation.try_send(message) {
                    Ok(()) => controller.branding_revision = Some(revision),
                    Err(std::sync::mpsc::TrySendError::Full(_)) => ui.ctx().request_repaint_after(std::time::Duration::from_millis(20)),
                    Err(_) => {},
                }
            }
            return;
        }
        if host == crate::ui::dialog::DialogHost::Native {
            if let Err(error) = app.cancel_preference_preview(ui.ctx()) {
                app.config_status = error;
            }
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

    if (!open || outcome.close) && draft.is_dirty() {
        open = true;
        draft.confirm_close = true;
    }
    if outcome.discard {
        draft.restore_saved();
        if let Err(error) = app.cancel_preference_preview(ui.ctx()) {
            draft.status = error;
            draft.status_until = None;
            open = true;
        } else {
            draft.previewed_config = draft.config.clone();
            open = false;
        }
    }
    if outcome.save {
        match validate_and_save(&draft.config) {
            Ok(()) => match app.commit_preference_preview(ui.ctx(), draft.config.clone()) {
                Ok(()) => {
                    app.config_fingerprint =
                        AppConfig::fingerprint(&AppConfig::get_config_path()).ok();
                    app.config_status = format!(
                        "Saved {}",
                        crate::config::display_config_path(&AppConfig::get_config_path())
                    );
                    draft.set_transient_status(app.tr("Preferences saved"));
                    draft.mark_saved();
                    draft.previewed_config = draft.config.clone();
                    if outcome.close_after_save {
                        open = false;
                    }
                }
                Err(error) => draft.status = error,
            },
            Err(error) => draft.status = error,
        }
    }
    if let Some(config) = draft.take_preview_update()
        && let Err(error) = app.preview_runtime_config(ui.ctx(), config)
    {
        draft.status = error;
        draft.status_until = None;
    }
    app.preferences_tab = draft.tab;
    if outcome.close && !draft.is_dirty() {
        open = false;
    }
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

fn save_draft(draft: &mut PreferencesDraft, ctx: &egui::Context) -> bool {
    match validate_and_save(&draft.config) {
        Ok(()) => {
            draft.set_transient_status(native_tr(draft.config.language, "Preferences saved"));
            draft.mark_saved();
            ctx.request_repaint_of(egui::ViewportId::ROOT);
            true
        }
        Err(error) => {
            draft.status = error;
            draft.status_until = None;
            false
        }
    }
}

fn native_tr(language: AppLanguage, key: &'static str) -> String {
    crate::ui::i18n::tr(crate::config::resolve_language(language), key)
}

fn draw_preferences_editor(draft: &mut PreferencesDraft, ui: &mut egui::Ui) -> PreferencesOutcome {
    let navigation = draft.pending_navigation.take();
    if let Some(section) = navigation.as_ref().and_then(|request| request.section.as_ref()) {
        if let Some(index) = preference_sections().iter().position(|item| item.id == section) { draft.tab = index; }
    }
    crate::ui::sync_elegance_theme(ui.ctx());
    // Resolve System once per pass, not once per caption. On Windows that
    // resolution reads the locale Registry key. Scrolling repaints many rows.
    // Keep it frame-local so live language changes still apply immediately.
    let language = crate::config::resolve_language(crate::config::resolved_language_preference(&draft.config));
    let tr = |key: &'static str| crate::ui::i18n::tr(language, key);
    let rtl = language == AppLanguage::Persian;
    let mut outcome = PreferencesOutcome::default();
    let now = std::time::Instant::now();
    if draft.status_until.is_some_and(|until| now >= until) {
        draft.status.clear();
        draft.status_until = None;
    }
    let footer_text = if draft.is_dirty() {
        tr("Unsaved changes")
    } else {
        draft.status.clone()
    };
    let footer_visible = !footer_text.is_empty();
    if let Some(until) = draft.status_until {
        ui.ctx()
            .request_repaint_after(until.saturating_duration_since(now));
    }

    egui::Panel::bottom("preferences_footer")
        .resizable(false)
        .show_separator_line(true)
        .show_inside(ui, |ui| {
            ui.add_space(9.0);
            crate::ui::dialog::action_bar(
                ui,
                rtl,
                |ui| {
                    let alpha = crate::ui::motion::hover_alpha(ui,
                        egui::Id::new("preferences-footer-status-visible"),
                        footer_visible,
                    );
                    if alpha > 0.01 {
                        ui.scope(|ui| {
                            ui.set_opacity(alpha);
                            ui.add(
                                egui::Label::new(egui::RichText::new(&footer_text).small().weak())
                                    .truncate(),
                            );
                        });
                    }
                },
                |ui| {
                    if crate::ui::dialog::action_button(ui, crate::ui::icons::X, &tr("Close"))
                        .clicked()
                    {
                        if draft.is_dirty() {
                            draft.confirm_close = true;
                        } else {
                            outcome.close = true;
                        }
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
                        outcome.request_save();
                    }
                },
            );
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        let sections = preference_sections();
        let narrow = ui.available_width() < 560.0;
        let rail_width = sections.iter().map(|section| {
            ui.painter().layout_no_wrap(tr(section.label), egui::TextStyle::Button.resolve(ui.style()),
                ui.visuals().text_color()).size().x + 44.0
        }).fold(PREFERENCES_RAIL_WIDTH, f32::max);
        if narrow {
            let active = &sections[draft.tab.min(sections.len() - 1)];
            crate::ui::dropdown::ComboBox::from_id_salt("preferences_compact_tab")
                .width(ui.available_width())
                .selected_text(format!("{}  {}", section_icon(active.id), tr(active.label)))
                .show_ui(ui, |ui| {
                    for (index, section) in sections.iter().enumerate() {
                        ui.dropdown_value(
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
                    egui::vec2(rail_width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        crate::ui::dialog::scroll_column(ui, "preferences_navigation", None, |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        let mut previous_group = "";
                        for (index, section) in sections.iter().enumerate() {
                            if draft.config.preferences_grouped_navigation && previous_group != section.navigation_group {
                                if index > 0 { ui.add_space(9.0); }
                                ui.label(egui::RichText::new(tr(section.navigation_group).to_uppercase()).size(10.0).weak());
                                previous_group = section.navigation_group;
                            }
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
                        });
                    },
                );
                ui.separator();
            }
            let detail_width = ui.available_width();
            let section_id = sections[draft.tab.min(sections.len() - 1)].id;
            draft.scroll_y = crate::ui::dialog::scroll_column_at(ui, ("preferences_contract_content", section_id), navigation.as_ref().map(|request| request.scroll_y), |ui| {
                ui.set_max_width((detail_width - 10.0).max(180.0));
                ui.spacing_mut().item_spacing.y = 8.0;
                let _ = draw_contract_section(draft, ui, &tr, &sections, language, rtl);
            });
        });
    });
    if draft.confirm_close {
        let mut cancel = false;
        let mut discard = false;
        let mut save = false;
        let confirm = crate::ui::dialog::confirmation_modal(ui.ctx(), egui::Id::new("preferences-confirm-close"),
            &tr("Unsaved preferences"), crate::ui::icons::WARNING,
            &tr("Save your changes before closing Preferences?"), &tr("Close"), |ui| {
                if ui.add(elegance::Button::new(format!("{}  {}", crate::ui::icons::FLOPPY_DISK, tr("Save")))
                    .min_width(crate::ui::dialog::ACTION_BUTTON_WIDTH)).clicked() {
                    save = true;
                }
                if ui.add(elegance::Button::new(format!("{}  {}", crate::ui::icons::TRASH, tr("Discard")))
                    .accent(elegance::Accent::Red).min_width(crate::ui::dialog::ACTION_BUTTON_WIDTH)).clicked() {
                    discard = true;
                }
                if ui.add(elegance::Button::new(format!("{}  {}", crate::ui::icons::X, tr("Cancel")))
                    .outline().min_width(crate::ui::dialog::ACTION_BUTTON_WIDTH)).clicked() {
                    cancel = true;
                }
                cancel || save || discard
            });
        if save {
            outcome.request_save();
        }
        if discard {
            outcome.discard = true;
        }
        draft.confirm_close = !confirm.dismissed && !cancel && !save && !discard;
    }
    outcome
}

fn draw_contract_section(
    draft: &mut PreferencesDraft,
    ui: &mut egui::Ui,
    tr: &impl Fn(&'static str) -> String,
    sections: &[crate::preferences_contract::PreferenceSection],
    language: AppLanguage,
    rtl_ui: bool,
) -> bool {
    let section = &sections[draft.tab.min(sections.len() - 1)];
    ui.heading(tr(section_heading(section.id)));
    ui.add_space(2.0);
    let controls = preference_controls(&draft.config);
    let mut values = serde_json::to_value(&draft.config).unwrap_or_else(|_| serde_json::json!({}));
    let mut changed = false;
    let mut clear_remote_history = false;
    let mut clear_playback_positions = false;
    let groups = crate::preferences_contract::preference_groups(&controls, &values);
    for presentation in groups.iter().filter(|group| group.section == section.id) {
        let group = presentation.name;
        let group_controls = controls
            .iter()
            .filter(|control| control.section == section.id && control.group == group)
            .collect::<Vec<_>>();
        let label_width = preference_label_column_width(ui, &group_controls, tr);
        if section.id == "advanced" && group == "Config file" {
            draw_advanced_actions(draft, ui, tr);
        }
        preference_section_with_disclosure(ui, group_icon(group), &tr(group),
            presentation.collapsible.then_some(presentation.default_open), |ui| {
            ui.spacing_mut().item_spacing.y = PREFERENCE_ROW_GAP;
            for control in group_controls {
                changed |=
                    render_contract_control(ui, control, &mut values, tr, label_width, language, rtl_ui);
            }
            if section.id == "timeline_toolbar" {
                let mut order = crate::config::normalize_timeline_toolbar_order(&draft.config.timeline_toolbar_order);
                let mut hidden = draft.config.timeline_toolbar_hidden.clone();
                if super::timeline_toolbar::customization(ui, &mut order, &mut hidden, tr) {
                    values["timeline_toolbar_order"] = serde_json::to_value(crate::config::persisted_timeline_toolbar_order(&order)).unwrap();
                    values["timeline_toolbar_hidden"] = serde_json::to_value(hidden).unwrap();
                    changed = true;
                }
            }
            if group == "Typography" {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label("The quick brown fox · 0123456789 ·");
                    ui.label(egui::RichText::new("فارسی").family(egui::FontFamily::Name("pealayer-persian".into())));
                });
                ui.label(egui::RichText::new("00:03:42.125  PWM 75%  Aa Bb 0O 1Il").monospace());
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
            if section.id == "playback" && group == "Playback history" {
                let remembered_count = draft.config.playback_positions.len();
                if ui
                    .add_enabled(
                        remembered_count > 0,
                        egui::Button::new(format!(
                            "{}  {} ({remembered_count})",
                            crate::ui::icons::TRASH,
                            tr("Clear remembered positions")
                        )),
                    )
                    .clicked()
                {
                    clear_playback_positions = true;
                    changed = true;
                }
            }
            if section.id == "advanced" && group == "Config file" {
                draw_config_file_actions(draft, ui, tr);
            }
        });
    }
    if changed {
        match serde_json::from_value::<AppConfig>(values) {
            Ok(mut config) => {
                config.normalize_playback_positions();
                draft.config = config;
            }
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
    if clear_playback_positions {
        draft.config.playback_positions.clear();
    }
    changed
}

#[cfg(test)]
fn unique_preference_groups(controls: &[PreferenceControl], section: &str) -> Vec<&'static str> {
    crate::preferences_contract::preference_groups(controls, &serde_json::Value::Null)
        .into_iter().filter(|group| group.section == section).map(|group| group.name).collect()
}

fn preference_defaults() -> &'static serde_json::Value {
    // Reset values are immutable, not live config. Serializing the entire
    // document for every numeric row on every wheel-animation frame is wasted
    // work, including rows outside the current viewport.
    static DEFAULTS: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    DEFAULTS.get_or_init(|| serde_json::to_value(AppConfig::default()).expect("serialize preference defaults"))
}

fn render_contract_control(
    ui: &mut egui::Ui,
    control: &PreferenceControl,
    values: &mut serde_json::Value,
    tr: &impl Fn(&'static str) -> String,
    label_width: f32,
    language: AppLanguage,
    rtl_ui: bool,
) -> bool {
    let current = value_at_path(values, control.key)
        .cloned()
        .unwrap_or_default();
    let mut replacement = None;
    let mut companion_changed = false;
    let mut description_rendered = false;
    let control_icon = preference_control_icon(control.icon);
    match control.kind {
        PreferenceControlKind::ActionList => {
            let hidden_key = control.custom_key.expect("action list requires its visibility key");
            let mut order = Vec::new();
            for value in current.as_array().into_iter().flatten().chain(control.options.iter().map(|option| &option.value)) {
                if control.options.iter().any(|option| option.value == *value) && !order.contains(value) { order.push(value.clone()); }
            }
            let mut hidden = value_at_path(values, hidden_key).and_then(|value| value.as_array()).cloned().unwrap_or_default();
            let mut changed = false;
            for (index, value) in order.clone().into_iter().enumerate() {
                let option = control.options.iter().find(|option| option.value == value).unwrap();
                ui.push_id((control.key, value.to_string()), |ui| ui.horizontal(|ui| {
                    let mut visible = !hidden.contains(&value);
                    if ui.checkbox(&mut visible, format!("{}  {}", option.icon.unwrap_or(""), tr(option.label))).changed() {
                        hidden.retain(|candidate| *candidate != value);
                        if !visible { hidden.push(value.clone()); }
                        changed = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for (enabled, next, icon, tip) in [
                            (index + 1 < order.len(), (index + 1).min(order.len() - 1), crate::ui::icons::ARROW_DOWN, "Move down"),
                            (index > 0, index.saturating_sub(1), crate::ui::icons::ARROW_UP, "Move up"),
                        ] {
                            if ui.add_enabled(enabled, egui::Button::new(icon).small()).on_hover_text(tr(tip)).clicked() {
                                order.swap(index, next); changed = true;
                            }
                        }
                    });
                }));
            }
            if ui.button(format!("{}  {}", crate::ui::icons::ARROW_COUNTER_CLOCKWISE, tr("Reset shortcuts"))).clicked() {
                order = value_at_path(preference_defaults(), control.key).and_then(|value| value.as_array()).cloned().unwrap_or_default();
                hidden = value_at_path(preference_defaults(), hidden_key).and_then(|value| value.as_array()).cloned().unwrap_or_default();
                changed = true;
            }
            if changed {
                replacement = Some(serde_json::Value::Array(order));
                companion_changed = set_value_at_path(values, hidden_key, serde_json::Value::Array(hidden)).is_ok();
            }
        }
        PreferenceControlKind::Accent => {
            let selected = current.as_str().unwrap_or("system");
            let custom_key = control.custom_key.unwrap_or("custom_accent_color");
            let mut custom_hex = value_at_path(values, custom_key).and_then(serde_json::Value::as_str)
                .unwrap_or("#0078d4").to_string();
            preference_row(ui, control_icon, &tr(control.label), label_width, |ui| {
                let width = ui.available_width().min(PREFERENCE_CONTROL_MAX_WIDTH);
                let options = control.options.iter().filter(|option| option.value.as_str() != Some("custom"))
                    .map(|option| crate::ui::color_picker::ColorChoice { value: option.value.as_str().unwrap_or_default().into(),
                        label: tr(option.label), rgb: option.color.as_deref().and_then(crate::config::parse_rgb_hex).unwrap_or([0,120,212]) }).collect::<Vec<_>>();
                let mut choice = selected.to_string();
                if crate::ui::color_picker::color_dropdown(ui, ("preference-accent", control.key), &mut choice,
                    &mut custom_hex, &options, width, &tr("Custom")).changed() {
                    if choice == "custom" {
                        companion_changed |= set_value_at_path(values, custom_key,
                            serde_json::Value::String(custom_hex.trim().to_string())).is_ok();
                    }
                    replacement = Some(serde_json::Value::String(choice));
                }
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
            let audio_output = matches!(control.key, "audio_device" | "sfx_audio_device");
            let font_picker = matches!(control.key, "ui_font_name" | "monospace_font_name");
            let dynamic_label = audio_output || font_picker;
            let selected = current.as_str().unwrap_or_default();
            let selected_label = control
                .options
                .iter()
                .find(|option| option.value.as_str() == Some(selected))
                .map(|option| if dynamic_label { option.description.clone().unwrap_or_else(|| tr(option.label)) } else { tr(option.label) })
                .unwrap_or_else(|| selected.to_string());
            preference_row(ui, control_icon, &tr(control.label), label_width, |ui| {
                let control_width = ui.available_width().min(PREFERENCE_CONTROL_MAX_WIDTH);
                let selected_text: egui::WidgetText = if control.key == "color_palette" {
                    palette_label(ui, &selected_label).into()
                } else { selected_label.into() };
                let popup = crate::ui::dropdown::ComboBox::from_id_salt(("preference", control.key))
                    .width(control_width)
                    .truncate()
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        let search_id = ui.id().with(("font-search", control.key));
                        let mut search = ui.data_mut(|d| d.get_temp::<String>(search_id).unwrap_or_default());
                        if font_picker {
                            ui.add(egui::TextEdit::singleline(&mut search).hint_text(tr("Search…")).desired_width(control_width));
                            ui.separator();
                            ui.data_mut(|d| d.insert_temp(search_id, search.clone()));
                        }
                        let query = search.to_lowercase();
                        for option in &control.options {
                            if font_picker && !option.value.as_str().unwrap_or_default().to_lowercase().contains(&query) { continue; }
                            let caption = if dynamic_label { option.description.clone().unwrap_or_else(|| tr(option.label)) } else { tr(option.label) };
                            let response = if control.key == "color_palette" {
                                palette_choice(ui, option.value == current, &caption, option.value.as_str().unwrap_or("native"))
                            } else { ui.dropdown_choice(
                                    option.value == current,
                                    caption,
                                ) };
                            if response.clicked() {
                                replacement = Some(option.value.clone());
                            }
                        }
                    });
                if control.key == "color_palette" {
                    paint_palette_preview(ui, popup.response.rect, selected);
                }
                if audio_output {
                    let id = ui.id().with(("audio-output-open", control.key));
                    let was_open = ui.data_mut(|d| d.get_temp::<bool>(id).unwrap_or(false));
                    if popup.inner.is_some() && !was_open {
                        if let Some(client) = crate::peer::client() {
                            let _ = client.queue("/api/player/command", serde_json::to_value(crate::platform::interop::InteropCommand::RefreshAudioOutputs).unwrap());
                        } else {
                            let ctx = ui.ctx().clone();
                            std::thread::spawn(move || { crate::mpv::audio_output::refresh_devices(); ctx.request_repaint(); });
                        }
                    }
                    ui.data_mut(|d| d.insert_temp(id, popup.inner.is_some()));
                }
            });
        }
        PreferenceControlKind::ChoiceSlider => {
            if !control.options.is_empty() {
                let mut index = control.options.iter().position(|option| option.value == current).unwrap_or(0);
                preference_row(ui, control_icon, &tr(control.label), label_width, |ui| {
                    let labels: Vec<_> = control.options.iter().map(|option| tr(option.label)).collect();
                    let width = ui.available_width().min(PREFERENCE_CONTROL_MAX_WIDTH);
                    let response = crate::ui::control_density::choice_slider(ui, &mut index,
                        &labels, &tr(control.label), width);
                    if response.changed() {
                        replacement = Some(control.options[index].value.clone());
                    }
                });
            }
        }
        PreferenceControlKind::MultiSelect => {
            let mut selected = current
                .as_array()
                .map(|values| {
                    values
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let selected_text = match selected.as_slice() {
                [] => tr("Choose interfaces"),
                [value] => control
                    .options
                    .iter()
                    .find(|option| option.value.as_str() == Some(value.as_str()))
                    .and_then(|option| option.description.clone())
                    .unwrap_or_else(|| value.clone()),
                values => format!("{} {}", values.len(), tr("addresses selected")),
            };
            preference_row(ui, control_icon, &tr(control.label), label_width, |ui| {
                let control_width = ui.available_width().min(PREFERENCE_CONTROL_MAX_WIDTH);
                crate::ui::dropdown::ComboBox::from_id_salt(("preference-multi-select", control.key))
                    .width(control_width)
                    .truncate()
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        ui.set_width(control_width.max(240.0).min(ui.ctx().content_rect().width() - 32.0));
                        for option in &control.options {
                            let Some(value) = option.value.as_str() else {
                                continue;
                            };
                            let mut checked = selected.iter().any(|selected| selected == value);
                            ui.horizontal(|ui| {
                                let checkbox = ui.checkbox(&mut checked, "");
                                let clicked = checkbox.clicked();
                                let mut checkbox = crate::ui::dropdown::register_choice(ui, checkbox, checked);
                                if checkbox.clicked() && !clicked {
                                    checked = !checked;
                                    checkbox.mark_changed();
                                }
                                if checkbox.changed() {
                                    if checked {
                                        selected.push(value.to_string());
                                    } else {
                                        selected.retain(|selected| selected != value);
                                    }
                                    replacement = Some(serde_json::json!(selected));
                                }
                                ui.label(preference_option_icon(option.icon));
                                ui.vertical(|ui| {
                                    ui.add(egui::Label::new(egui::RichText::new(tr(option.label)).strong()).wrap());
                                    if let Some(detail) = option.description.as_deref() {
                                        ui.add(egui::Label::new(egui::RichText::new(detail).small().weak()).wrap());
                                    }
                                });
                            });
                        }
                    });
            });
        }
        PreferenceControlKind::Number => {
            let mut number = current.as_f64().unwrap_or_default();
            let range = control.minimum.unwrap_or(0.0)..=control.maximum.unwrap_or(100.0);
            let reset = value_at_path(preference_defaults(), control.key).and_then(serde_json::Value::as_f64).unwrap_or(*range.start());
            let mut steps: std::collections::BTreeMap<String, crate::config::NumericInputSteps> =
                serde_json::from_value(values["numeric_input_steps"].clone()).unwrap_or_default();
            let previous_steps = steps.clone();
            let (_, changed) =
                preference_row(ui, control_icon, &tr(control.label), label_width, |ui| {
                    let mut changed = crate::ui::dialog::receive_numeric_paste(ui, control.key, &mut number, &range, "");
                    // Preserve fine adjustments/pasted values between frames;
                    // Always clamping also re-quantizes to the drag step on paint.
                    let mut slider = egui::Slider::new(&mut number, range.clone())
                        .clamping(egui::SliderClamping::Edits);
                    if let Some(step) = control.step { slider = slider.step_by(step); }
                    if control.logarithmic { slider = slider.logarithmic(true); }
                    let control_width = ui.available_width().min(PREFERENCE_CONTROL_MAX_WIDTH);
                    let response = ui.add_sized([control_width, PREFERENCE_ROW_HEIGHT], slider);
                    changed |= response.changed();
                    changed |= crate::ui::dialog::numeric_context_menu(ui, &response, control.key, &mut number,
                        range.clone(), control.step.unwrap_or(1.0), reset, "", &mut steps, language,
                        if current.is_u64() || current.is_i64() { 1.0 } else { 1e-9 });
                    changed
                });
            if steps != previous_steps {
                companion_changed |= set_value_at_path(values, "numeric_input_steps", serde_json::json!(steps)).is_ok();
            }
            if changed {
                replacement = Some(if current.is_u64() {
                    serde_json::json!(number.round() as u64)
                } else if current.is_i64() {
                    serde_json::json!(number.round() as i64)
                } else {
                    serde_json::json!(number)
                });
            }
        }
        PreferenceControlKind::ReplacementList => {
            let mut replacements: Vec<crate::subtitle::SubtitleReplacement> =
                serde_json::from_value(current.clone()).unwrap_or_default();
            let mut changed = false;
            let mut remove = None;
            let replacement_body_height = PREFERENCE_ROW_HEIGHT * (replacements.len() + 1) as f32
                + PREFERENCE_ROW_GAP * replacements.len() as f32;
            preference_multiline_row(
                ui,
                control_icon,
                &tr(control.label),
                control.description.map(tr),
                label_width,
                replacement_body_height,
                |ui| {
                    for (index, item) in replacements.iter_mut().enumerate() {
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), PREFERENCE_ROW_HEIGHT),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.spacing_mut().item_spacing.x = PREFERENCE_COLUMN_GAP;
                                let available = ui.available_width();
                                let fixed_width = REPLACEMENT_ARROW_WIDTH
                                    + REPLACEMENT_DELETE_WIDTH
                                    + PREFERENCE_COLUMN_GAP * 3.0;
                                let field_width = ((available - fixed_width) / 2.0)
                                    .max(REPLACEMENT_FIELD_MIN_WIDTH);
                                let from_align =
                                    crate::ui::i18n::input_alignment(rtl_ui, &item.from);
                                let to_align = crate::ui::i18n::input_alignment(rtl_ui, &item.to);
                                changed |= ui
                                    .add_sized(
                                        [field_width, crate::ui::control_density::height(ui)],
                                        crate::ui::dialog::singleline_text_edit(ui, &mut item.from)
                                            .horizontal_align(from_align)
                                            .hint_text(tr("Source text")),
                                    )
                                    .changed();
                                ui.label(crate::ui::icons::ARROW_RIGHT);
                                changed |= ui
                                    .add_sized(
                                        [field_width, crate::ui::control_density::height(ui)],
                                        crate::ui::dialog::singleline_text_edit(ui, &mut item.to)
                                            .horizontal_align(to_align)
                                            .hint_text(tr("Replacement")),
                                    )
                                    .changed();
                                if ui
                                    .add_sized(
                                        [REPLACEMENT_DELETE_WIDTH, PREFERENCE_ROW_HEIGHT],
                                        egui::Button::new(crate::ui::icons::TRASH).frame(false),
                                    )
                                    .on_hover_text(tr("Remove replacement"))
                                    .clicked()
                                {
                                    remove = Some(index);
                                }
                            },
                        );
                    }
                    if let Some(index) = remove {
                        replacements.remove(index);
                        changed = true;
                    }
                    if ui
                        .add_sized(
                            [ui.available_width().min(180.0), PREFERENCE_ROW_HEIGHT],
                            egui::Button::new(format!(
                                "{}  {}",
                                crate::ui::icons::PLUS,
                                tr("Add replacement")
                            )),
                        )
                        .clicked()
                    {
                        replacements.push(crate::subtitle::SubtitleReplacement::default());
                        changed = true;
                    }
                },
            );
            description_rendered = true;
            if changed {
                replacement = serde_json::to_value(replacements).ok();
            }
        }
        PreferenceControlKind::File => {
            let mut text = current.as_str().unwrap_or_default().to_string();
            let mut edited = false;
            if let Some(selected) = crate::ui::peer_browser::take_preference_file(ui.ctx(), control.key) {
                text = selected;
                edited = true;
            }
            preference_row(ui, control_icon, &tr(control.label), label_width, |ui| {
                let controls = file_controls(ui, &mut text, &tr(control.placeholder.unwrap_or_default()), &tr("Browse..."));
                edited |= controls.0.changed();
                if controls.1.on_hover_text(tr("Choose an image file")).clicked() {
                    if crate::peer::active() {
                        crate::ui::peer_browser::open(ui.ctx(), crate::ui::peer_browser::Purpose::PreferenceFile {
                            key: control.key.to_owned(), extensions: control.file_extensions.iter().map(|extension| (*extension).to_owned()).collect(),
                        }, None);
                    } else {
                        let mut dialog = rfd::FileDialog::new().set_title(tr("Choose an image file"))
                            .add_filter("Images", &control.file_extensions);
                        if let Some(parent) = std::path::Path::new(&text).parent().filter(|parent| parent.is_dir()) {
                            dialog = dialog.set_directory(parent);
                        }
                        if let Some(path) = dialog.pick_file() {
                            text = path.to_string_lossy().into_owned();
                            edited = true;
                        }
                    }
                }
                if controls.2.on_hover_text(tr("Use default icon")).clicked() {
                    text.clear();
                    edited = true;
                }
            });
            if edited {
                replacement = Some(if text.trim().is_empty() { serde_json::Value::Null }
                    else { serde_json::Value::String(text.trim().to_owned()) });
            }
        }
        PreferenceControlKind::Color => {
            let mut hex = current.as_str().unwrap_or("#969696").to_string();
            preference_row(ui, control_icon, &tr(control.label), label_width, |ui| {
                if crate::ui::color_picker::color_field(ui, &mut hex, [150,150,150], 154.0).changed() {
                    replacement = Some(serde_json::Value::String(hex));
                }
            });
        }
        PreferenceControlKind::Text => {
            let mut text = current.as_str().unwrap_or_default().to_string();
            let text_align = crate::ui::i18n::input_alignment(rtl_ui, &text);
            preference_row(ui, control_icon, &tr(control.label), label_width, |ui| {
                let control_width = ui.available_width().min(PREFERENCE_CONTROL_MAX_WIDTH);
                let response = ui.add(
                    crate::ui::dialog::singleline_text_edit(ui, &mut text)
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
    if !description_rendered && let Some(description) = control.description {
        ui.label(egui::RichText::new(tr(description)).small().weak());
    }
    replacement
        .is_some_and(|replacement| set_value_at_path(values, control.key, replacement).is_ok())
        || companion_changed
}

#[derive(Clone, Copy)]
pub(crate) enum ConfigPathAction {
    Edit,
    OpenContainingFolder,
}

pub(crate) fn perform_config_path_action(
    action: ConfigPathAction,
    config: &AppConfig,
) -> Result<&'static str, String> {
    if let Some(client)=crate::peer::client(){
        client.queue("/api/player/command",serde_json::json!({"command":"edit_configuration"}))?;
        return Ok("Requested the server configuration editor");
    }
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
    if crate::peer::active(){return;}
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
}

fn draw_config_file_actions(
    draft: &mut PreferencesDraft,
    ui: &mut egui::Ui,
    tr: &impl Fn(&'static str) -> String,
) {
    let path = AppConfig::get_config_path();
    if crate::peer::active(){
        ui.label(format!("{}  {}",crate::ui::icons::GLOBE,crate::config::display_config_path(&path)));
        ui.label(egui::RichText::new("Stored on the connected server").weak());
        if let Some(config)=ui.ctx().data_mut(|data|data.remove_temp::<AppConfig>(egui::Id::new("peer_imported_config"))){draft.config=config;draft.set_transient_status(tr("Configuration imported; review and save"));}
        ui.horizontal(|ui|{
            if ui.button(format!("{} Import",crate::ui::icons::FOLDER_OPEN)).clicked(){crate::ui::peer_browser::open(ui.ctx(),crate::ui::peer_browser::Purpose::ConfigImport,None);}
            if ui.button(format!("{} Export",crate::ui::icons::ARROW_SQUARE_OUT)).clicked(){crate::ui::peer_browser::open(ui.ctx(),crate::ui::peer_browser::Purpose::ConfigExport(Box::new(draft.config.clone())),None);}
            if ui.button(format!("{} Reload",crate::ui::icons::ARROW_CLOCKWISE)).clicked(){draft.replace_from_disk(AppConfig::load(),tr("Preferences reloaded from disk"));}
        });
        return;
    }
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
        if ui
            .button(format!(
                "{}  {}",
                crate::ui::icons::COPY,
                tr("Copy full path")
            ))
            .clicked()
        {
            ui.ctx().copy_text(path.display().to_string());
            draft.set_transient_status(tr("Configuration path copied"));
            ui.close();
        }
    });
    if let Some(action) = path_action {
        match perform_config_path_action(action, &draft.saved_config) {
            Ok(status) => draft.set_transient_status(tr(status)),
            Err(error) => {
                draft.status = error;
                draft.status_until = None;
            }
        }
    }
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(format!(
                "{}  {}",
                crate::ui::icons::FOLDER_OPEN,
                tr("Import")
            ))
            .clicked()
            && let Some(import_path) = rfd::FileDialog::new()
                .add_filter("JSON configuration", &["json"])
                .pick_file()
        {
            match AppConfig::load_from_path(&import_path) {
                Ok(config) => {
                    draft.config = config;
                    draft.set_transient_status(tr("Configuration imported; review and save"));
                }
                Err(error) => {
                    draft.status = error;
                    draft.status_until = None;
                }
            }
        }
        if ui
            .button(format!(
                "{}  {}",
                crate::ui::icons::ARROW_SQUARE_OUT,
                tr("Export")
            ))
            .clicked()
            && let Some(export_path) = rfd::FileDialog::new()
                .add_filter("JSON configuration", &["json"])
                .set_file_name("pealayer-settings.json")
                .save_file()
        {
            match draft.config.save_to_path(&export_path) {
                Ok(()) => draft.set_transient_status(tr("Configuration exported")),
                Err(error) => {
                    draft.status = error;
                    draft.status_until = None;
                }
            }
        }
        if ui
            .button(format!(
                "{}  {}",
                crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                tr("Reload from disk")
            ))
            .clicked()
        {
            match AppConfig::load_from_path(&path) {
                Ok(config) => {
                    draft.replace_from_disk(config, tr("Preferences reloaded from disk"));
                }
                Err(error) => {
                    draft.status = error;
                    draft.status_until = None;
                }
            }
        }
    });
}

fn palette_label(ui: &egui::Ui, caption: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.append(caption, 68.0, egui::TextFormat::simple(
        egui::TextStyle::Button.resolve(ui.style()), ui.visuals().text_color()));
    job
}

fn paint_palette_preview(ui: &egui::Ui, row: egui::Rect, name: &str) {
    let palette = if name == "studio" { crate::config::ColorPalette::Studio } else { crate::config::ColorPalette::Native };
    let color = |role| crate::ui::palette::color(palette, ui.visuals().dark_mode, role);
    let rect = egui::Rect::from_center_size(egui::pos2(row.left() + 34.0, row.center().y), egui::vec2(54.0, (row.height() - 6.0).clamp(10.0, 22.0)));
    ui.painter().rect(rect, 3.0, color("canvas"), egui::Stroke::new(1.0, color("line-strong")), egui::StrokeKind::Inside);
    let panel = egui::Rect::from_min_max(rect.min + egui::vec2(14.0, 3.0), rect.max - egui::vec2(3.0, 3.0));
    ui.painter().rect_filled(panel, 2.0, color("surface-2"));
    ui.painter().hline(panel.left() + 4.0..=panel.right() - 4.0, panel.center().y, egui::Stroke::new(1.5, color("text")));
}

fn palette_choice(ui: &mut egui::Ui, selected: bool, caption: &str, name: &str) -> egui::Response {
    let label = palette_label(ui, caption);
    let response = ui.dropdown_choice(selected, label);
    paint_palette_preview(ui, response.rect, name);
    response
}

/// Allocate the complete row once. In particular the reset button's actual
/// padded width must not grow the parent and cascade into subsequent rows.
fn file_controls(ui: &mut egui::Ui, text: &mut String, hint: &str, browse: &str)
    -> (egui::Response, egui::Response, egui::Response) {
    let width = ui.available_width().min(PREFERENCE_CONTROL_MAX_WIDTH);
    let height = crate::ui::control_density::height(ui);
    let font = egui::TextStyle::Button.resolve(ui.style());
    let caption = format!("{} {browse}", crate::ui::icons::FOLDER_OPEN);
    let browse_width = ui.painter().layout_no_wrap(caption.clone(), font.clone(), ui.visuals().text_color()).size().x
        + ui.spacing().button_padding.x * 2.0;
    let clear_width = height.max(ui.painter().layout_no_wrap(crate::ui::icons::ARROW_COUNTER_CLOCKWISE.into(), font, ui.visuals().text_color()).size().x
        + ui.spacing().button_padding.x * 2.0);
    let (row, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let clear = egui::Rect::from_min_max(egui::pos2(row.right() - clear_width, row.top()), row.max);
    let browse = egui::Rect::from_min_max(egui::pos2(clear.left() - PREFERENCE_COLUMN_GAP - browse_width, row.top()), egui::pos2(clear.left() - PREFERENCE_COLUMN_GAP, row.bottom()));
    let input = egui::Rect::from_min_max(row.min, egui::pos2(browse.left() - PREFERENCE_COLUMN_GAP, row.bottom()));
    let edit = crate::ui::dialog::singleline_text_edit(ui, text).hint_text(hint).desired_width(input.width());
    let edit = ui.put(input, edit).on_hover_text(text.as_str());
    let pick = ui.put(browse, egui::Button::new(caption));
    let reset = ui.add_enabled_ui(!text.is_empty(), |ui| {
        ui.put(clear, egui::Button::new(crate::ui::icons::ARROW_COUNTER_CLOCKWISE))
    }).inner;
    (edit, pick, reset)
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
                        ui.set_width(label_width);
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

fn preference_multiline_row<R>(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    description: Option<String>,
    desired_label_width: f32,
    body_height: f32,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let width = ui.available_width();
    let draw_label = |ui: &mut egui::Ui| {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(icon)
                    .size(14.0)
                    .color(ui.visuals().selection.bg_fill),
            );
            ui.add(egui::Label::new(egui::RichText::new(label).size(13.0)).wrap());
        });
        if let Some(description) = description.as_deref() {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(27.0);
                ui.label(egui::RichText::new(description).small().weak());
            });
        }
    };

    let Some(label_width) =
        inline_preference_label_width(width, desired_label_width).filter(|label_width| {
            width - label_width - PREFERENCE_COLUMN_GAP >= REPLACEMENT_ROW_MIN_WIDTH
        })
    else {
        return ui
            .vertical(|ui| {
                draw_label(ui);
                ui.add_space(2.0);
                ui.set_width(width.min(PREFERENCE_CONTROL_MAX_WIDTH));
                body(ui)
            })
            .inner;
    };

    let control_width = (width - label_width - PREFERENCE_COLUMN_GAP)
        .max(PREFERENCE_CONTROL_MIN_WIDTH)
        .min(PREFERENCE_CONTROL_MAX_WIDTH);
    ui.allocate_ui_with_layout(
        egui::vec2(width, body_height),
        egui::Layout::left_to_right(egui::Align::Min),
        |ui| {
            ui.spacing_mut().item_spacing.x = PREFERENCE_COLUMN_GAP;
            ui.allocate_ui_with_layout(
                egui::vec2(label_width, body_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(label_width);
                    draw_label(ui);
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(control_width, body_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(control_width);
                    body(ui)
                },
            )
            .inner
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
    // the active preference card still begin on the same vertical guide.
    (text_width + 34.0).max(128.0)
}

fn preference_control_icon(name: &str) -> &'static str {
    use egui_phosphor::regular::*;
    match name {
        "circle-half" => CIRCLE_HALF,
        "gauge" => GAUGE,
        "film-strip" => FILM_STRIP,
        "palette" => PALETTE,
        "tabs" => TABS,
        "swatches" => SWATCHES,
        "translate" => TRANSLATE,
        "frame-corners" => FRAME_CORNERS,
        "push-pin" => PUSH_PIN,
        "arrows-out" => ARROWS_OUT,
        "target" => TARGET,
        "clock" => CLOCK,
        "image" => IMAGE,
        "play" => PLAY,
        "pause" => PAUSE,
        "stop-circle" => STOP_CIRCLE,
        "speaker-high" => SPEAKER_HIGH,
        "subtitles" => SUBTITLES,
        "waveform" => WAVEFORM,
        "globe" => GLOBE,
        "keyboard" => KEYBOARD,
        "check-square" => CHECK_SQUARE,
        "sliders-horizontal" => SLIDERS_HORIZONTAL,
        "text-align-left" => TEXT_ALIGN_LEFT,
        "text-t" => TEXT_T,
        "list-checks" => LIST_CHECKS,
        "pencil-simple" => PENCIL_SIMPLE,
        _ => GEAR,
    }
}

fn preference_option_icon(icon: Option<&str>) -> &'static str {
    match icon.unwrap_or_default() {
        "wifi" => crate::ui::icons::BROADCAST,
        "ethernet" => crate::ui::icons::PLUG,
        "vpn" => crate::ui::icons::LOCK,
        "virtual" => crate::ui::icons::CIRCUITRY,
        "loopback" => crate::ui::icons::ARROW_CLOCKWISE,
        "globe" => crate::ui::icons::GLOBE,
        _ => crate::ui::icons::LINK,
    }
}

fn preference_section(
    ui: &mut egui::Ui,
    icon: &str,
    title: &str,
    body: impl FnOnce(&mut egui::Ui),
) {
    preference_section_with_disclosure(ui, icon, title, None, body);
}

fn preference_section_with_disclosure(
    ui: &mut egui::Ui,
    icon: &str,
    title: &str,
    default_open: Option<bool>,
    body: impl FnOnce(&mut egui::Ui),
) {
    crate::ui::sync_elegance_theme(ui.ctx());
    elegance::Card::new()
        .corner_radius(9.0)
        .padding(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(default_open) = default_open {
                egui::CollapsingHeader::new(egui::RichText::new(format!("{icon}  {title}")).size(15.0).strong())
                    .id_salt(("preferences-optional-group", title))
                    .default_open(default_open)
                    .show(ui, body);
                return;
            }
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
        "timeline_toolbar" => crate::ui::icons::SLIDERS_HORIZONTAL,
        "sidebar_rail" => crate::ui::icons::TABS,
        "web" => crate::ui::icons::GLOBE,
        _ => crate::ui::icons::GEAR,
    }
}

fn group_icon(group: &str) -> &'static str {
    match group {
        "Interface" => crate::ui::icons::SPARKLE,
        "On-screen display" => crate::ui::icons::MONITOR_PLAY,
        "Application icons" => crate::ui::icons::IMAGE,
          "Player controls" => crate::ui::icons::PLAY,
          "External mpv" => crate::ui::icons::MONITOR_PLAY,
        "Playback history" => crate::ui::icons::CLOCK_COUNTER_CLOCKWISE,
        "Config file" => crate::ui::icons::FLOPPY_DISK,
        "Open Location / URL" => crate::ui::icons::LINK_SIMPLE,
        "Connection" => crate::ui::icons::PLUG,
        "Motion controls" => crate::ui::icons::SEAT,
        "Keyboard shortcuts" => crate::ui::icons::KEYBOARD,
        "Web server" => crate::ui::icons::GLOBE,
        "Permissions" => crate::ui::icons::LOCK,
        "Live synchronization" => crate::ui::icons::BROADCAST,
        "Video surface" => crate::ui::icons::SELECTION_ALL,
        "Window movement" => crate::ui::icons::APP_WINDOW,
        "Status bar" => crate::ui::icons::GAUGE,
        "Typography" => crate::ui::icons::TEXT_T,
        "Timeline toolbar" => crate::ui::icons::SLIDERS_HORIZONTAL,
        "Sidebar rail" | "Shortcuts" => crate::ui::icons::TABS,
        "Windows graphics and composition" | "Dialog windows" => crate::ui::icons::APP_WINDOW,
        _ => crate::ui::icons::GEAR,
    }
}

fn section_heading(section: &str) -> &'static str {
    match section {
        "appearance" => "Appearance and language",
        "playback" => "Playback behavior",
        "hardware" => "PCController and hardware",
        "input" => "Keyboard, mouse, and gesture bindings",
        "timeline_toolbar" => "Timeline toolbar",
        "sidebar_rail" => "Sidebar rail",
        "web" => "Web UI and network APIs",
        _ => "Configuration",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accent_frame(ctx: &egui::Context, control: &PreferenceControl, values: &mut serde_json::Value,
        events: Vec<egui::Event>) -> egui::FullOutput {
        let mut output = ctx.run_ui(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0,800.0))),
            time: Some(ctx.cumulative_frame_nr() as f64 * 0.5),
            events, ..Default::default()
        }, |ui| { render_contract_control(ui, control, values, &str::to_string, 160.0, AppLanguage::English, false); });
        output.textures_delta.clear();
        output
    }

    fn accent_click(ctx: &egui::Context, control: &PreferenceControl, values: &mut serde_json::Value,
        pos: egui::Pos2) -> egui::FullOutput {
        accent_frame(ctx, control, values, vec![egui::Event::PointerMoved(pos), egui::Event::PointerButton {
            pos, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE }]);
        accent_frame(ctx, control, values, vec![egui::Event::PointerButton {
            pos, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE }])
    }

    fn accent_text_position(output: &egui::FullOutput, text: &str) -> egui::Pos2 {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(shape) if shape.galley.job.text == text =>
                Some(shape.pos + shape.galley.size() / 2.0), _ => None,
        }).unwrap_or_else(|| panic!("missing text {text}"))
    }

    #[test]
    fn accent_dropdown_has_closed_swatch_equal_rows_and_one_custom_picker_target() {
        let control = preference_controls(&AppConfig::default()).into_iter()
            .find(|control| control.key == "accent_color").unwrap();
        for dark in [false,true] {
            for scale in [1.0,1.25,1.5,2.0] {
                for click_hex in [false,true] {
                    let ctx = egui::Context::default();
                    ctx.set_pixels_per_point(scale);
                    ctx.set_visuals(if dark { egui::Visuals::dark() } else { egui::Visuals::light() });
                    let mut values = serde_json::json!({"accent_color":"system", "custom_accent_color":"#0078D4"});
                    crate::ui::color_picker::remember_color(&ctx, egui::Color32::from_rgb(1,2,3));
                    let closed = accent_frame(&ctx,&control,&mut values,vec![]);
                    assert!(closed.shapes.iter().any(|shape| matches!(&shape.shape,
                        egui::epaint::Shape::Circle(circle) if circle.radius == 5.0)), "closed color indicator");
                    let trigger = accent_text_position(&closed,"System accent");
                    accent_click(&ctx,&control,&mut values,trigger);
                    accent_frame(&ctx,&control,&mut values,vec![]);
                    let open = accent_frame(&ctx,&control,&mut values,vec![]);
                    let blue = accent_text_position(&open,"macOS blue");
                    let row_height = open.shapes.iter().filter_map(|shape| match &shape.shape {
                        egui::epaint::Shape::Rect(rect) if rect.rect.contains(blue) && rect.rect.width() > 50.0 => Some(rect.rect.height()),
                        _ => None,
                    }).min_by(f32::total_cmp).expect("preset geometry");
                    let menu_width = open.shapes.iter().find_map(|shape| match &shape.shape {
                        egui::epaint::Shape::Rect(rect) if rect.rect.height() == row_height && rect.rect.contains(blue) => Some(rect.rect.width()), _=>None,
                    }).expect("preset row");
                    let rows: Vec<_> = open.shapes.iter().filter_map(|shape| match &shape.shape {
                        egui::epaint::Shape::Rect(rect) if rect.rect.height() == row_height
                            && rect.rect.width() == menu_width
                            && rect.rect.center().y >= blue.y - 2.0 * (row_height + 2.0) - 0.5
                            && rect.rect.center().y <= blue.y + row_height + 2.5 => Some(rect.rect), _ => None,
                    }).collect();
                    assert_eq!(rows.len(),4,"four full-width rows at scale {scale}");
                    assert!(rows.iter().all(|row| row.width() == rows[0].width()));
                    let preview = open.shapes.iter().find_map(|shape| match &shape.shape {
                        egui::epaint::Shape::Rect(rect) if rect.rect.width() == 124.0 => Some((shape.clip_rect,rect.rect)),
                        _=>None,
                    }).expect("custom preview");
                    assert!(preview.0.contains_rect(preview.1), "no 1px clipping at scale {scale}");
                    let pos = if click_hex { preview.1.center() } else { accent_text_position(&open,"Custom") };
                    accent_click(&ctx,&control,&mut values,pos);
                    accent_frame(&ctx,&control,&mut values,vec![]);
                    let palette = accent_frame(&ctx,&control,&mut values,vec![]);
                    assert_eq!(values["accent_color"],"custom");
                    assert!(palette.shapes.iter().any(|shape| matches!(&shape.shape, egui::epaint::Shape::Text(shape) if shape.galley.job.text == "Recent colors")), "palette texts: {:?}", palette.shapes.iter().filter_map(|shape| match &shape.shape { egui::epaint::Shape::Text(shape)=>Some(&shape.galley.job.text), _=>None }).collect::<Vec<_>>());
                    accent_text_position(&palette,"Pealayer colors");
                    assert!(palette.shapes.iter().any(|shape| matches!(&shape.shape,
                        egui::epaint::Shape::Circle(circle) if circle.radius == 7.0
                            && circle.fill == egui::Color32::from_rgb(245,158,11))), "Orange swatch");
                }
            }
        }
    }

    #[test]
    fn application_icons_optional_card_does_not_render_unconfigured_fields() {
        for configured in [false, true] {
            let ctx = egui::Context::default();
            let mut rendered = false;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                preference_section_with_disclosure(ui, crate::ui::icons::IMAGE, "Application icons",
                    Some(configured), |_| { rendered = true; });
            });
            output.textures_delta.clear();
            assert_eq!(rendered, configured);
        }
    }

    #[test]
    fn preferences_reopen_reaps_an_exited_helper_before_setting_open() {
        #[cfg(windows)]
        let mut command = {
            use std::os::windows::process::CommandExt;
            let mut command = std::process::Command::new("cmd.exe");
            command.args(["/c", "exit", "0"]).creation_flags(0x08000000);
            command
        };
        #[cfg(not(windows))]
        let mut command = std::process::Command::new("true");
        let mut child = command.spawn().unwrap();
        child.wait().unwrap();
        let mut controller = Some(NativePreferencesController { child, navigation: std::sync::mpsc::sync_channel(1).0,
            branding_revision: None, exit_verified: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)) });
        assert!(!controller.as_ref().unwrap().confirmed_exited(), "unverified is still unknown");
        assert!(!controller.as_mut().unwrap().is_open());
        assert!(controller.as_ref().unwrap().confirmed_exited(), "confirmed process exit is not an open draft");
        assert!(reap_exited_controller(&mut controller));
        assert!(controller.is_none());
        assert!(!reap_exited_controller(&mut controller));
    }

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
    fn preferences_scrolling_does_not_edit_or_reverse_the_draft() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut draft = PreferencesDraft::new(AppConfig::default(), 0);
        let ctx = egui::Context::default();
        crate::ui::configure_native_appearance(&ctx, &draft.config);
        let mut offsets = Vec::new();
        let started = std::time::Instant::now();
        for frame in 0..120 {
            let mut events = vec![egui::Event::PointerMoved(egui::pos2(390.0, 310.0))];
            if (5..65).contains(&frame) {
                events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -5.0),
                    modifiers: egui::Modifiers::NONE,
                    source: egui::MouseWheelSource::Wheel,
                    phase: egui::TouchPhase::Move,
                });
            }
            ctx.run_ui(egui::RawInput {
                time: Some(frame as f64 / 60.0),
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 450.0))),
                events,
                ..Default::default()
            }, |ui| { draw_preferences_editor(&mut draft, ui); }).drop_without_applying_deltas();
            offsets.push(draft.scroll_y);
        }
        eprintln!("Preferences 120 scroll frames: {:?}", started.elapsed());
        assert!(offsets.last().copied().unwrap() >= 250.0);
        assert!(offsets.windows(2).all(|pair| pair[1] >= pair[0]));
        assert!(!draft.is_dirty(), "scrolling must not trigger live configuration previews");
    }

    #[test]
    fn preferences_mouse_notch_smoothing_continues_until_settled() {
        let _guard = crate::app::tests::lock_app_tests();
        for width in [440.0, 700.0] {
            let mut draft = PreferencesDraft::new(AppConfig::default(), 0);
            let ctx = egui::Context::default();
            crate::ui::configure_native_appearance(&ctx, &draft.config);
            let mut offsets = Vec::new();
            for frame in 0..60 {
                let mut events = vec![egui::Event::PointerMoved(egui::pos2(300.0, 280.0))];
                if frame == 5 {
                    events.push(egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta: egui::vec2(0.0, -3.0),
                        modifiers: egui::Modifiers::NONE,
                        source: egui::MouseWheelSource::Unknown, // Windows/winit
                        phase: egui::TouchPhase::Move,
                    });
                }
                let output = ctx.run_ui(egui::RawInput {
                    time: Some(frame as f64 / 60.0),
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 450.0))),
                    events,
                    ..Default::default()
                }, |ui| { draw_preferences_editor(&mut draft, ui); });
                if (6..10).contains(&frame) {
                    assert_eq!(output.viewport_output[&egui::ViewportId::ROOT].repaint_delay,
                        std::time::Duration::ZERO, "wheel tail must schedule another paint");
                }
                output.drop_without_applying_deltas();
                offsets.push(draft.scroll_y);
            }
            assert!(offsets[6] > offsets[5], "a notch must animate between frames");
            assert!(offsets.windows(2).all(|pair| pair[1] >= pair[0]), "no backward jumps");
            assert_eq!(offsets[45], offsets[59], "scrolling settles instead of repainting forever");
            assert!(!draft.is_dirty());
        }
    }

    #[test]
    fn cached_numeric_resets_match_current_config_defaults() {
        let expected = serde_json::to_value(AppConfig::default()).unwrap();
        assert_eq!(preference_defaults(), &expected);
        assert!(std::ptr::eq(preference_defaults(), preference_defaults()));
    }

    #[test]
    fn navigation_changes_scroll_and_section_without_editing_preferences() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut draft = PreferencesDraft::new(AppConfig::default(), 1);
        let ctx = egui::Context::default();
        let render = |draft: &mut PreferencesDraft| {
            ctx.run_ui(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 450.0))),
                ..Default::default()
            }, |ui| { draw_preferences_editor(draft, ui); }).drop_without_applying_deltas();
        };
        render(&mut draft);
        draft.pending_navigation = Some(PreferencesNavigation { section: Some("appearance".into()), scroll_y: 400.0 });
        render(&mut draft);
        assert_eq!(draft.tab, 0);
        assert!(draft.scroll_y > 0.0);
        assert!(draft.pending_navigation.is_none());
        assert!(!draft.is_dirty());
        let appearance_scroll = draft.scroll_y;
        draft.tab = 1;
        render(&mut draft);
        assert_eq!(draft.scroll_y, 0.0, "a new section starts at the top");
        draft.pending_navigation = Some(PreferencesNavigation { section: None, scroll_y: 170.0 });
        render(&mut draft);
        let playback_scroll = draft.scroll_y;
        draft.tab = 0;
        render(&mut draft);
        assert_eq!(draft.scroll_y, appearance_scroll, "Appearance retains its own position");
        draft.tab = 1;
        render(&mut draft);
        assert_eq!(draft.scroll_y, playback_scroll, "Playback retains its own position");
        draft.pending_navigation = Some(PreferencesNavigation { section: None, scroll_y: 0.0 });
        render(&mut draft);
        assert_eq!(draft.scroll_y, 0.0);
        assert!(draft.take_preview_update().is_none());
    }

    #[test]
    fn timeline_toolbar_navigation_opens_exact_page_and_preserves_draft() {
        let _guard = crate::app::tests::lock_app_tests();
        let request = PreferencesNavigation::timeline_toolbar();
        request.validate().unwrap();
        let round_trip: PreferencesHelperMessage = serde_json::from_value(
            serde_json::to_value(PreferencesHelperMessage::Navigate { navigation: request.clone() }).unwrap()).unwrap();
        assert!(matches!(round_trip, PreferencesHelperMessage::Navigate { navigation } if navigation == request));
        for width in [390.0, 700.0, 1200.0] {
            let ctx = egui::Context::default();
            let mut draft = PreferencesDraft::new(AppConfig::default(), 0);
            draft.config.timeline_toolbar_hidden.clear(); // existing unsaved edits
            let expected = draft.config.clone();
            for _ in 0..2 {
                draft.tab = 1; // another previously selected page
                draft.pending_navigation = Some(request.clone());
                for _ in 0..3 {
                    ctx.run_ui(egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 600.0))),
                        ..Default::default()
                    }, |ui| { draw_preferences_editor(&mut draft, ui); }).drop_without_applying_deltas();
                }
                assert_eq!(preference_sections()[draft.tab].id, "timeline_toolbar");
                assert_eq!(draft.scroll_y, 0.0);
                assert_eq!(draft.config, expected, "navigation must not modify unsaved controls");
                assert!(draft.is_dirty());
                assert!(draft.pending_navigation.is_none());
            }
        }
    }

    #[test]
    fn sidebar_shortcut_preferences_render_without_mutating_values_at_all_widths() {
        let _guard = crate::app::tests::lock_app_tests();
        for width in [390.0,700.0,1200.0] {
            let ctx=egui::Context::default();
            let mut draft=PreferencesDraft::new(AppConfig::default(),0);
            let expected=draft.config.clone();
            draft.pending_navigation=Some(PreferencesNavigation {section:Some("sidebar_rail".into()),scroll_y:0.0});
            for _ in 0..3 {
                ctx.run_ui(egui::RawInput {screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(width,600.0))),..Default::default()}, |ui| {
                    draw_preferences_editor(&mut draft,ui);
                }).drop_without_applying_deltas();
            }
            assert_eq!(preference_sections()[draft.tab].id,"sidebar_rail");
            assert_eq!(draft.config,expected); assert!(!draft.is_dirty());
        }
    }

    #[test]
    fn file_rows_have_identical_bounds_without_cascading() {
        for dark in [false, true] {
            for width in [180.0, 250.0, 420.0] {
                for density in [crate::config::UiControlDensity::ExtraCompact, crate::config::UiControlDensity::Compact, crate::config::UiControlDensity::Spacious] {
                    let ctx = egui::Context::default();
                    let config = AppConfig { ui_control_density: density, ..Default::default() };
                    crate::ui::configure_native_appearance(&ctx, &config);
                    ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
                    let mut rectangles = Vec::new();
                    ctx.run_ui(egui::RawInput::default(), |ui| {
                        ui.set_width(width);
                        for index in 0..4 {
                            let mut text = if index == 0 { "C:/icons/application.ico".into() } else { String::new() };
                            let (field, browse, reset) = file_controls(ui, &mut text, "Use default icon", "Browse...");
                            rectangles.push([field.rect, browse.rect, reset.rect]);
                        }
                    }).drop_without_applying_deltas();
                    for row in &rectangles {
                        for (actual, first) in row.iter().zip(rectangles[0]) {
                            assert!((actual.left() - first.left()).abs() < 0.1);
                            assert!((actual.right() - first.right()).abs() < 0.1, "{width}/{density:?}: {rectangles:?}");
                        }
                        assert!(row[0].right() + 1.0 < row[1].left());
                        assert!(row[1].right() + 1.0 < row[2].left());
                        assert!((row[2].right() - row[0].left() - width).abs() < 0.1);
                    }
                }
            }
        }
    }

    #[test]
    fn external_branding_updates_do_not_save_or_discard_other_draft_fields() {
        let mut draft = PreferencesDraft::new(AppConfig::default(), 0);
        draft.config.volume = 17.0;
        let saved_volume = draft.saved_config.volume;
        let values = serde_json::json!({"app_name":"Stage","app_description":"Preview","app_icon_preset":"classic"});
        draft.sync_external_branding(&values);
        for config in [&draft.config, &draft.saved_config, &draft.previewed_config] {
            assert_eq!(config.app_name.as_deref(), Some("Stage"));
            assert_eq!(config.app_description.as_deref(), Some("Preview"));
            assert_eq!(config.app_icon_preset, crate::config::AppIconPreset::Classic);
        }
        assert_eq!(draft.config.volume, 17.0);
        assert_eq!(draft.saved_config.volume, saved_volume);
        assert!(draft.is_dirty());
        let message = PreferencesHelperMessage::Branding {
            effective: crate::branding::configured_values(&draft.config),
            configured: crate::branding::configured_values(&draft.saved_config),
        };
        assert!(matches!(serde_json::from_slice::<PreferencesHelperMessage>(&serde_json::to_vec(&message).unwrap()).unwrap(), PreferencesHelperMessage::Branding { .. }));
    }

    #[test]
    fn web_settings_cards_stay_inside_the_detail_column() {
        let _guard = crate::app::tests::lock_app_tests();
        let sections = preference_sections();
        let index = sections.iter().position(|section| section.id == "web").unwrap();
        for width in [320.0, 480.0, 720.0] {
            let ctx = egui::Context::default();
            let mut draft = PreferencesDraft::new(AppConfig::default(), index);
            crate::ui::configure_native_appearance(&ctx, &draft.config);
            ctx.run_ui(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width + 32.0, 900.0))), ..Default::default() }, |ui| {
                ui.set_width(width);
                let right = ui.max_rect().right();
                draw_contract_section(&mut draft, ui, &str::to_string, &sections, AppLanguage::English, false);
                assert!(ui.min_rect().right() <= right + 0.5, "{width}: content {} > column {right}", ui.min_rect().right());
            }).drop_without_applying_deltas();
        }
    }

    #[cfg(windows)]
    #[test]
    fn helper_previews_reach_the_named_owner_not_the_branding_name() {
        let identity = format!("preferences-preview-test-{}", uuid::Uuid::new_v4());
        let (sender, receiver) = std::sync::mpsc::channel();
        crate::platform::interop::spawn_interop_listener(sender, egui::Context::default(), identity.clone());
        let mut helper = StandalonePreferencesApp {
            draft: PreferencesDraft::new(AppConfig::default(), 0), owner_hwnd: 0, owner_identity: identity,
            native_window_initialized: false, applied_appearance: None,
            config_watcher: None, config_reload_due: None, branding_config: None,
            navigation: std::sync::mpsc::sync_channel(1).1,
        };
        helper.draft.config.color_palette = crate::config::ColorPalette::Studio;
        // Listener startup is asynchronous; retry only its bounded initial bind.
        let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            helper.preview_changes();
            if helper.draft.previewed_config == helper.draft.config { break; }
            assert!(std::time::Instant::now() < until, "{}", helper.draft.status);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(matches!(receiver.recv_timeout(std::time::Duration::from_secs(1)).unwrap(),
            crate::platform::interop::InteropCommand::PreviewConfig { config } if config.color_palette == crate::config::ColorPalette::Studio));
        assert!(helper.finish_preview(false));
        assert!(matches!(receiver.recv_timeout(std::time::Duration::from_secs(1)).unwrap(),
            crate::platform::interop::InteropCommand::CancelPreviewConfig));
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
    fn native_preferences_use_configured_branding() {
        let builder = native_preferences_viewport("Preferences — Custom".to_string());
        assert!(builder.icon.is_some());
        let source = include_str!("preferences.rs");
        assert!(source.contains("branding_config = Some(config.clone())"));
        assert!(source.contains("ViewportCommand::Title"));
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
    fn non_contiguous_preference_groups_render_one_card_each() {
        let controls = preference_controls(&AppConfig::default());
        for section in preference_sections() {
            let groups = unique_preference_groups(&controls, section.id);
            let unique = groups
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>();
            assert_eq!(
                groups.len(),
                unique.len(),
                "duplicate card in {}",
                section.id
            );
        }

        let playback = unique_preference_groups(&controls, "playback");
        assert_eq!(
            playback
                .iter()
                .filter(|group| **group == "Subtitles")
                .count(),
            1
        );
        assert_eq!(
            playback
                .iter()
                .filter(|group| **group == "Open Location / URL")
                .count(),
            1
        );

        let hardware = unique_preference_groups(&controls, "hardware");
        assert_eq!(
            hardware
                .iter()
                .filter(|group| **group == "Motion controls")
                .count(),
            1
        );
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
    fn subtitle_replacements_use_the_shared_preference_alignment_grid() {
        let source = include_str!("preferences.rs");
        let replacement_branch = source
            .split_once("PreferenceControlKind::ReplacementList => {")
            .expect("replacement preference branch")
            .1
            .split_once("PreferenceControlKind::Text => {")
            .expect("text preference branch")
            .0;
        assert!(replacement_branch.contains("preference_multiline_row("));
        assert!(replacement_branch.contains("PREFERENCE_ROW_HEIGHT"));
        assert!(replacement_branch.contains("description_rendered = true"));
        let multiline_layout = source
            .split_once("fn preference_multiline_row")
            .expect("multiline preference layout helper")
            .1
            .split_once("fn preference_checkbox_row")
            .expect("checkbox preference layout helper")
            .0;
        assert!(multiline_layout.contains("REPLACEMENT_ROW_MIN_WIDTH"));
        assert!(multiline_layout.contains("width - label_width - PREFERENCE_COLUMN_GAP"));
        assert!(multiline_layout.contains("egui::vec2(width, body_height)"));
        assert!(multiline_layout.contains("egui::vec2(label_width, body_height)"));
        assert!(multiline_layout.contains("egui::vec2(control_width, body_height)"));
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
    fn unsaved_preferences_corner_close_and_escape_keep_the_dirty_draft() {
        for escape in [false, true] {
            let context = egui::Context::default();
            crate::ui::i18n::configure_ui_fonts(&context, false);
            let mut draft = PreferencesDraft::new(AppConfig::default(), 0);
            draft.config.playback_speed = 1.25;
            draft.confirm_close = true;
            let before = draft.config.clone();
            let saved = draft.saved_config.clone();
            let render = |draft: &mut PreferencesDraft, events, time| context.run_ui(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 800.0))),
                events, time: Some(time), ..Default::default()
            }, |ui| {
                let outcome = draw_preferences_editor(draft, ui);
                assert!(!outcome.save && !outcome.discard && !outcome.close);
            });
            for time in [0.0, 0.05, 0.1] {
                let mut output = render(&mut draft, vec![], time);
                output.textures_delta.clear();
            }
            let mut output = render(&mut draft, vec![], 0.15);
            let point = output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "×" =>
                    Some(text.pos + text.galley.mesh_bounds.center().to_vec2()),
                _ => None,
            }).expect("unsaved-preferences close glyph");
            output.textures_delta.clear();
            if escape {
                let mut output = render(&mut draft, vec![egui::Event::Key {
                    key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }], 0.2);
                output.textures_delta.clear();
            } else {
                for (time, pressed) in [(0.2, true), (0.25, false)] {
                    let mut output = render(&mut draft, vec![egui::Event::PointerMoved(point),
                        egui::Event::PointerButton { pos: point, button: egui::PointerButton::Primary,
                            pressed, modifiers: egui::Modifiers::NONE }], time);
                    output.textures_delta.clear();
                }
            }
            assert!(!draft.confirm_close);
            assert!(draft.is_dirty());
            assert_eq!(draft.config, before);
            assert_eq!(draft.saved_config, saved);
        }
    }

    #[test]
    fn every_preferences_save_requests_close_after_success() {
        let mut outcome = PreferencesOutcome::default();
        assert!(!outcome.close_after_save);
        outcome.request_save();
        assert!(outcome.save);
        assert!(outcome.close_after_save);
        assert!(!outcome.close); // Hosts must wait for persistence/commit success.
        let editor = include_str!("preferences.rs")
            .split_once("fn draw_preferences_editor(")
            .unwrap().1
            .split_once("fn draw_contract_section(")
            .unwrap().0;
        assert_eq!(editor.matches("outcome.request_save()").count(), 2);
    }

    #[test]
    fn workspace_interface_uses_its_tabs_icon_not_the_generic_fallback() {
        assert_eq!(preference_control_icon("tabs"), egui_phosphor::regular::TABS);
        assert_ne!(preference_control_icon("tabs"), egui_phosphor::regular::GEAR);
    }

    #[test]
    fn failed_preferences_save_preserves_the_dirty_draft() {
        let mut draft = PreferencesDraft::new(AppConfig::default(), 0);
        draft.config.playback_speed = 0.0;
        assert!(!save_draft(&mut draft, &egui::Context::default()));
        assert!(draft.is_dirty());
        assert!(draft.status.contains("playback_speed"));
        assert_eq!(draft.saved_config.playback_speed, 1.0);
    }

    #[test]
    fn live_preview_is_incremental_and_discard_restores_the_saved_configuration() {
        let original = AppConfig::default();
        let mut draft = PreferencesDraft::new(original.clone(), 0);
        draft.config.pin_controls = !original.pin_controls;
        draft.config.workspace_ui = crate::config::WorkspaceUi::EguiDock;
        draft.config.preferences_grouped_navigation = false;
        draft.config.ui_font_family = crate::config::UiFontFamily::Custom;
        draft.config.ui_font_name = "Example UI family".into();
        draft.config.monospace_font_family = crate::config::UiFontFamily::Custom;
        draft.config.monospace_font_name = "Example fixed width family".into();
        let preview = draft.take_preview_update().expect("live preview");
        assert_eq!(preview.pin_controls, !original.pin_controls);
        assert_eq!(preview.workspace_ui, crate::config::WorkspaceUi::EguiDock);
        assert_eq!(preview.ui_font_name, "Example UI family");
        assert_eq!(preview.monospace_font_name, "Example fixed width family");
        assert!(draft.take_preview_update().is_none());
        draft.restore_saved();
        let restored = draft.take_preview_update().expect("discard preview");
        assert_eq!(restored, original);
        assert!(!draft.is_dirty());
    }

    #[test]
    fn open_preferences_follows_system_events_without_creating_unsaved_changes() {
        let config = AppConfig::default();
        let mut helper = StandalonePreferencesApp {
            draft: PreferencesDraft::new(config.clone(), 0),
            owner_hwnd: 0,
            owner_identity: "preferences-test-owner".into(),
            native_window_initialized: false,
            applied_appearance: None,
            config_watcher: None,
            config_reload_due: None,
            branding_config: None,
            navigation: std::sync::mpsc::sync_channel(1).1,
        };
        let ctx = egui::Context::default();
        helper.apply_appearance(&ctx);
        let cached_settings = helper.applied_appearance;
        for theme in [egui::Theme::Light, egui::Theme::Dark, egui::Theme::Light] {
            ctx.run_ui(
                egui::RawInput {
                    system_theme: Some(theme),
                    ..Default::default()
                },
                |ui| {
                    let ctx = ui.ctx();
                    helper.apply_appearance(ctx);
                    assert_eq!(ctx.theme(), theme);
                    assert_eq!(helper.applied_appearance, cached_settings);
                    assert!(!helper.draft.is_dirty());
                    assert!(helper.draft.take_preview_update().is_none());
                    assert_eq!(helper.draft.config, config);
                },
            )
            .drop_without_applying_deltas();
        }
    }

    #[test]
    fn open_preferences_applies_density_and_motion_changes_without_reopening() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut helper = StandalonePreferencesApp {
            draft: PreferencesDraft::new(AppConfig::default(), 0), owner_hwnd: 0, owner_identity: "preferences-test-owner".into(),
            native_window_initialized: false, applied_appearance: None,
            config_watcher: None, config_reload_due: None, branding_config: None,
            navigation: std::sync::mpsc::sync_channel(1).1,
        };
        let ctx = egui::Context::default();
        helper.apply_appearance(&ctx);
        assert_eq!(ctx.global_style().spacing.interact_size.y, 24.0);
        helper.draft.config.ui_control_density = crate::config::UiControlDensity::Comfortable;
        helper.draft.config.ui_animation_mode = crate::config::UiAnimationMode::Disabled;
        helper.apply_appearance(&ctx);
        assert_eq!(ctx.global_style().spacing.interact_size.y, 32.0);
        assert_eq!(ctx.global_style().animation_time, 0.0);
        helper.draft.restore_saved();
        helper.apply_appearance(&ctx);
        assert_eq!(ctx.global_style().spacing.interact_size.y, 24.0);
    }

    #[test]
    fn shared_appearance_updates_an_open_dirty_draft_without_losing_other_edits() {
        let original = AppConfig::default();
        let mut draft = PreferencesDraft::new(original.clone(), 0);
        draft.config.pin_controls = !original.pin_controls;
        let remote = AppConfig {
            theme: AppTheme::Dark,
            color_palette: crate::config::ColorPalette::Studio,
            accent_color: crate::config::AccentColor::PealayerGreen,
            ..original.clone()
        };
        draft.apply_external_config(
            remote.clone(),
            "Updated".into(),
            "Other draft edits retained".into(),
        );
        assert_eq!(draft.config.pin_controls, !original.pin_controls);
        assert_eq!(draft.config.color_palette, remote.color_palette);
        assert_eq!(draft.config.accent_color, remote.accent_color);
        assert!(draft.is_dirty());
        draft.restore_saved();
        assert_eq!(draft.config, remote);
    }

    #[test]
    fn config_path_is_clickable_and_has_all_requested_actions() {
        let source = include_str!("preferences.rs");
        assert!(source.contains("response.clicked()"));
        assert!(source.contains("ConfigPathAction::Edit"));
        assert!(source.contains("ConfigPathAction::OpenContainingFolder"));
        assert!(source.contains("Copy full path"));
        assert!(source.contains("tr(\"Import\")"));
        assert!(source.contains("tr(\"Export\")"));
        assert!(source.contains("add_enabled_ui(draft.is_dirty()"));
    }
}
