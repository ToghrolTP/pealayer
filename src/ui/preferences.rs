use crate::app::PealayerApp;
use crate::config::{
    AppLanguage, AppTheme, MotionControlMode, OsdPosition, PlayerDragAction, VideoBackground,
};
use eframe::egui;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

const TABS: [(&str, &str); 5] = [
    (crate::ui::icons::SPARKLE, "Appearance"),
    (crate::ui::icons::PLAY, "Playback"),
    (crate::ui::icons::PLUG, "Hardware"),
    (crate::ui::icons::SLIDERS_HORIZONTAL, "Input"),
    (crate::ui::icons::GEAR, "Advanced"),
];

#[derive(Clone)]
pub(crate) struct NativePreferencesController {
    open: Arc<AtomicBool>,
    state: Arc<Mutex<NativePreferencesState>>,
}

struct NativePreferencesState {
    config: crate::config::AppConfig,
    endpoint: String,
    proxy_url: String,
    tab: usize,
    status: String,
    styled_composition: Option<(bool, bool, bool)>,
}

impl NativePreferencesController {
    fn from_live_config() -> Self {
        let config = crate::platform::interop::get_live_config();
        let endpoint = config
            .hardware_endpoint
            .clone()
            .unwrap_or_else(|| crate::four_d::controller::DEFAULT_ENDPOINT.to_string());
        let proxy_url = config.open_url_proxy_url.clone().unwrap_or_default();
        Self {
            open: Arc::new(AtomicBool::new(true)),
            state: Arc::new(Mutex::new(NativePreferencesState {
                config,
                endpoint,
                proxy_url,
                tab: 0,
                status: String::new(),
                styled_composition: None,
            })),
        }
    }

    fn is_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }

    fn close(&self) {
        self.open.store(false, Ordering::Release);
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
    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../../assets/pealayer-icon.png"))
    {
        builder = builder.with_icon(icon);
    }
    builder
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_preferences_dialog {
        return;
    }

    if app.native_dialog_windows {
        let ctx = ui.ctx().clone();
        let title = format!("{} — {}", app.tr("Preferences"), app.app_name);
        let controller = app
            .native_preferences
            .get_or_insert_with(NativePreferencesController::from_live_config)
            .clone();
        if !controller.is_open() {
            app.show_preferences_dialog = false;
            app.native_preferences = None;
            return;
        }
        let open = Arc::clone(&controller.open);
        let state = Arc::clone(&controller.state);
        let root_ctx = ctx.clone();
        let style_title = title.clone();
        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("pealayer_preferences_native"),
            native_preferences_viewport(title),
            move |ui, _class| {
                if ui.input(|input| input.viewport().close_requested())
                    || crate::ui::dialog::escape_pressed(ui.ctx())
                {
                    open.store(false, Ordering::Release);
                    root_ctx.request_repaint_of(egui::ViewportId::ROOT);
                    return;
                }
                let Ok(mut state) = state.lock() else {
                    return;
                };
                let composition = (
                    ui.visuals().dark_mode,
                    state.config.windows_dwm_theming,
                    state.config.windows_mica_backdrop,
                );
                if state.styled_composition != Some(composition) {
                    match crate::platform::windows::style_preferences_tool_window(
                        &style_title,
                        composition.0,
                        composition.1,
                        composition.2,
                    ) {
                        Ok(true) => state.styled_composition = Some(composition),
                        Ok(false) => ui
                            .ctx()
                            .request_repaint_after(std::time::Duration::from_millis(50)),
                        Err(error) => {
                            state.status = error;
                            state.styled_composition = Some(composition);
                        }
                    }
                }
                let close = egui::CentralPanel::default()
                    .show_inside(ui, |ui| {
                        egui::Frame::new()
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| draw_native_preferences_surface(&mut state, ui))
                            .inner
                    })
                    .inner;
                if close {
                    open.store(false, Ordering::Release);
                    root_ctx.request_repaint_of(egui::ViewportId::ROOT);
                }
            },
        );
        return;
    }

    if let Some(controller) = app.native_preferences.take() {
        controller.close();
    }

    let mut open = app.show_preferences_dialog;
    let bounds = ui.ctx().content_rect().shrink(18.0);
    let max_size = egui::vec2(bounds.width().min(700.0), bounds.height().min(620.0));
    let default_size = egui::vec2(max_size.x.min(620.0), max_size.y.min(520.0));
    let min_size = egui::vec2(max_size.x.min(390.0), max_size.y.min(330.0));
    let default_rect = crate::ui::dialog::centered_default_rect(bounds, default_size);
    if crate::ui::dialog::escape_pressed(ui.ctx()) {
        open = false;
    }
    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::GEAR,
        app.tr("Preferences")
    ))
    .id(egui::Id::new("preferences_dialog_bounded_v2"))
    .open(&mut open)
    .default_rect(default_rect)
    .min_size(min_size)
    .max_size(max_size)
    .constrain_to(bounds)
    .resizable(true)
    .movable(true)
    .collapsible(false)
    .show(ui.ctx(), |ui| draw_preferences_surface(app, ui));
    app.show_preferences_dialog = open;
}

fn native_tr(language: AppLanguage, key: &'static str) -> String {
    crate::ui::i18n::tr(crate::config::resolve_language(language), key)
}

fn save_native_preferences(state: &mut NativePreferencesState, ctx: &egui::Context) {
    state.config.hardware_endpoint =
        (!state.endpoint.trim().is_empty()).then(|| state.endpoint.trim().to_string());
    let proxy = state.proxy_url.trim();
    let proxy_valid = proxy.is_empty()
        || url::Url::parse(proxy).is_ok_and(|value| {
            matches!(value.scheme(), "http" | "https") && value.host_str().is_some()
        });
    if !proxy_valid {
        state.status = native_tr(
            state.config.language,
            "Enter a complete HTTP or HTTPS proxy URL.",
        );
        return;
    }
    state.config.open_url_proxy_url = (!proxy.is_empty()).then(|| proxy.to_string());
    match state.config.save() {
        Ok(()) => {
            crate::platform::interop::set_live_config(state.config.clone());
            state.status = format!(
                "{} {}",
                native_tr(state.config.language, "Saved"),
                crate::config::AppConfig::get_config_path().display()
            );
            ctx.request_repaint_of(egui::ViewportId::ROOT);
        }
        Err(error) => state.status = error,
    }
}

fn draw_native_preferences_surface(state: &mut NativePreferencesState, ui: &mut egui::Ui) -> bool {
    let language = crate::config::resolved_language_preference(&state.config);
    let tr = |key: &'static str| native_tr(language, key);
    let mut changed = false;
    let mut close = false;

    let narrow = ui.available_width() < 580.0;
    if narrow {
        let (active_icon, active_name) = TABS[state.tab.min(TABS.len() - 1)];
        egui::ComboBox::from_id_salt("native_preferences_compact_tab")
            .width(ui.available_width())
            .selected_text(format!("{active_icon}  {}", tr(active_name)))
            .show_ui(ui, |ui| {
                for (index, (icon, name)) in TABS.into_iter().enumerate() {
                    ui.selectable_value(&mut state.tab, index, format!("{icon}  {}", tr(name)));
                }
            });
        ui.separator();
    }

    ui.horizontal_top(|ui| {
        if !narrow {
            ui.allocate_ui_with_layout(
                egui::vec2(126.0, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 5.0;
                    for (index, (icon, name)) in TABS.into_iter().enumerate() {
                        if ui
                            .add_sized(
                                [120.0, 32.0],
                                egui::Button::new(format!("{icon}  {}", tr(name)))
                                    .selected(state.tab == index),
                            )
                            .clicked()
                        {
                            state.tab = index;
                        }
                    }
                },
            );
            ui.separator();
        }

        let detail_width = ui.available_width();
        crate::ui::dialog::scroll_column(ui, "native_preferences_content", None, |ui| {
            ui.set_max_width((detail_width - 8.0).max(180.0));
            ui.spacing_mut().item_spacing.y = 8.0;
            match state.tab {
                0 => native_appearance_preferences(state, ui, &tr, &mut changed),
                1 => native_playback_preferences(state, ui, &tr, &mut changed),
                2 => native_hardware_preferences(state, ui, &tr, &mut changed),
                3 => native_input_preferences(state, ui, &tr, &mut changed),
                _ => native_advanced_preferences(state, ui, &tr, &mut changed),
            }
        });
    });

    if changed {
        state.styled_composition = None;
        save_native_preferences(state, ui.ctx());
    }

    ui.separator();
    ui.horizontal(|ui| {
        if !state.status.is_empty() {
            ui.add(egui::Label::new(egui::RichText::new(&state.status).small().weak()).truncate());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(format!("{}  {}", crate::ui::icons::X, tr("Close")))
                .clicked()
            {
                close = true;
            }
            if ui
                .button(format!(
                    "{}  {}",
                    crate::ui::icons::FLOPPY_DISK,
                    tr("Save now")
                ))
                .clicked()
            {
                save_native_preferences(state, ui.ctx());
            }
        });
    });
    close || !state.config.native_dialog_windows
}

fn native_appearance_preferences(
    state: &mut NativePreferencesState,
    ui: &mut egui::Ui,
    tr: &impl Fn(&'static str) -> String,
    changed: &mut bool,
) {
    ui.heading(tr("Appearance and language"));
    preference_section(ui, crate::ui::icons::SPARKLE, &tr("Interface"), |ui| {
        preference_grid(ui, "native_appearance_preferences_grid", |ui| {
            ui.label(tr("Theme"));
            egui::ComboBox::from_id_salt("native_preferences_theme")
                .selected_text(match state.config.theme {
                    AppTheme::System => tr("System"),
                    AppTheme::Light => tr("Light"),
                    AppTheme::Dark => tr("Dark"),
                })
                .show_ui(ui, |ui| {
                    for (theme, label) in [
                        (AppTheme::System, "System"),
                        (AppTheme::Light, "Light"),
                        (AppTheme::Dark, "Dark"),
                    ] {
                        *changed |= ui
                            .selectable_value(&mut state.config.theme, theme, tr(label))
                            .changed();
                    }
                });
            ui.end_row();

            ui.label(tr("Language"));
            egui::ComboBox::from_id_salt("native_preferences_language")
                .selected_text(match state.config.language {
                    AppLanguage::System => tr("System language"),
                    AppLanguage::English => tr("English"),
                    AppLanguage::Persian => tr("Persian"),
                })
                .show_ui(ui, |ui| {
                    for (value, label) in [
                        (AppLanguage::System, "System language"),
                        (AppLanguage::English, "English"),
                        (AppLanguage::Persian, "Persian"),
                    ] {
                        *changed |= ui
                            .selectable_value(&mut state.config.language, value, tr(label))
                            .changed();
                    }
                });
            ui.end_row();

            ui.label(tr("Fullscreen background"));
            egui::ComboBox::from_id_salt("native_fullscreen_background")
                .selected_text(match state.config.fullscreen_video_background {
                    VideoBackground::Black => tr("Black"),
                    VideoBackground::DarkGray => tr("Dark gray"),
                    VideoBackground::Theme => tr("Use app theme"),
                })
                .show_ui(ui, |ui| {
                    for (value, label) in [
                        (VideoBackground::Black, "Black"),
                        (VideoBackground::DarkGray, "Dark gray"),
                        (VideoBackground::Theme, "Use app theme"),
                    ] {
                        *changed |= ui
                            .selectable_value(
                                &mut state.config.fullscreen_video_background,
                                value,
                                tr(label),
                            )
                            .changed();
                    }
                });
            ui.end_row();
        });
    });
    preference_section(
        ui,
        crate::ui::icons::MONITOR_PLAY,
        &tr("On-screen display"),
        |ui| {
            preference_grid(ui, "native_osd_preferences_grid", |ui| {
                ui.label(tr("Position"));
                egui::ComboBox::from_id_salt("native_osd_position")
                    .selected_text(match state.config.osd_position {
                        OsdPosition::TopLeft => tr("Top left"),
                        OsdPosition::Center => tr("Center"),
                    })
                    .show_ui(ui, |ui| {
                        *changed |= ui
                            .selectable_value(
                                &mut state.config.osd_position,
                                OsdPosition::TopLeft,
                                tr("Top left"),
                            )
                            .changed();
                        *changed |= ui
                            .selectable_value(
                                &mut state.config.osd_position,
                                OsdPosition::Center,
                                tr("Center"),
                            )
                            .changed();
                    });
                ui.end_row();
            });
            *changed |= ui
                .add(
                    egui::Slider::new(&mut state.config.osd_timeout_seconds, 1.0..=12.0)
                        .text(tr("OSD timeout (seconds)")),
                )
                .changed();
        },
    );
}

fn native_playback_preferences(
    state: &mut NativePreferencesState,
    ui: &mut egui::Ui,
    tr: &impl Fn(&'static str) -> String,
    changed: &mut bool,
) {
    ui.heading(tr("Playback behavior"));
    preference_section(ui, crate::ui::icons::PLAY, &tr("Player controls"), |ui| {
        *changed |= ui
            .checkbox(
                &mut state.config.click_player_to_toggle,
                tr("Single-click the picture to play or pause"),
            )
            .changed();
        *changed |= ui
            .checkbox(
                &mut state.config.show_subseconds,
                tr("Show milliseconds in time displays"),
            )
            .changed();
        *changed |= ui
            .add(
                egui::Slider::new(&mut state.config.quick_seek_seconds, 0.1..=600.0)
                    .logarithmic(true)
                    .text(tr("Skip button and arrow-key step (seconds)")),
            )
            .changed();
        *changed |= ui
            .add(
                egui::Slider::new(&mut state.config.frame_step_count, 1..=120)
                    .text(tr("Frames per frame-step action")),
            )
            .changed();
        *changed |= ui
            .add(
                egui::Slider::new(&mut state.config.wheel_seek_seconds, 0.1..=60.0)
                    .logarithmic(true)
                    .text(tr("Mouse-wheel seek step (seconds)")),
            )
            .changed();
    });
    preference_section(
        ui,
        crate::ui::icons::LINK_SIMPLE,
        &tr("Open Location / URL"),
        |ui| {
            *changed |= ui
                .checkbox(
                    &mut state.config.open_url_multiline,
                    tr("Wrap long URLs in a text area"),
                )
                .changed();
            *changed |= ui
                .checkbox(
                    &mut state.config.open_url_history_expanded,
                    tr("Expand recent URL history by default"),
                )
                .changed();
            let mut play_recent = !state.config.open_url_recent_click_edits;
            if ui
                .checkbox(
                    &mut play_recent,
                    tr("Play when a recent location is clicked"),
                )
                .changed()
            {
                state.config.open_url_recent_click_edits = !play_recent;
                *changed = true;
            }
            *changed |= ui
                .checkbox(
                    &mut state.config.open_url_use_proxy,
                    tr("Use a proxy for remote inspection and playback"),
                )
                .changed();
            ui.label(tr("Custom proxy URL"));
            ui.add(
                egui::TextEdit::singleline(&mut state.proxy_url)
                    .desired_width(ui.available_width())
                    .hint_text("http://proxy.example:8080"),
            );
            let proxy = state.proxy_url.trim();
            if !proxy.is_empty()
                && !url::Url::parse(proxy).is_ok_and(|value| {
                    matches!(value.scheme(), "http" | "https") && value.host_str().is_some()
                })
            {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    tr("Enter a complete HTTP or HTTPS proxy URL."),
                );
            }
            let remote_count = state
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
                state
                    .config
                    .recent_media
                    .retain(|path| !crate::media::is_remote_media_target(&path.to_string_lossy()));
                *changed = true;
            }
        },
    );
}

fn native_hardware_preferences(
    state: &mut NativePreferencesState,
    ui: &mut egui::Ui,
    tr: &impl Fn(&'static str) -> String,
    changed: &mut bool,
) {
    ui.heading(tr("PCController and hardware"));
    preference_section(ui, crate::ui::icons::PLUG, &tr("Connection"), |ui| {
        *changed |= ui
            .checkbox(
                &mut state.config.auto_connect_hardware,
                tr("Discover and connect to PCController on startup"),
            )
            .changed();
        *changed |= ui
            .checkbox(
                &mut state.config.pause_on_hardware_disconnect,
                tr("Pause playback when hardware disconnects unexpectedly"),
            )
            .changed();
        ui.label(tr("Preferred endpoint"));
        if ui
            .add(
                egui::TextEdit::singleline(&mut state.endpoint)
                    .desired_width(ui.available_width())
                    .hint_text(tr(
                        "pccontroller://host:port, tcp://host:port, or direct:<device>",
                    )),
            )
            .changed()
        {
            *changed = true;
        }
    });
    preference_section(ui, crate::ui::icons::SEAT, &tr("Motion controls"), |ui| {
        preference_grid(ui, "native_motion_control_preferences", |ui| {
            ui.label(tr("Button behavior"));
            egui::ComboBox::from_id_salt("native_motion_control_mode")
                .selected_text(match state.config.motion_control_mode {
                    MotionControlMode::Toggle => tr("Toggle on press"),
                    MotionControlMode::Hold => tr("Run only while held"),
                })
                .show_ui(ui, |ui| {
                    *changed |= ui
                        .selectable_value(
                            &mut state.config.motion_control_mode,
                            MotionControlMode::Toggle,
                            tr("Toggle on press"),
                        )
                        .changed();
                    *changed |= ui
                        .selectable_value(
                            &mut state.config.motion_control_mode,
                            MotionControlMode::Hold,
                            tr("Run only while held"),
                        )
                        .changed();
                });
            ui.end_row();
        });
        *changed |= ui
            .checkbox(
                &mut state.config.compact_hardware_controls,
                tr("Use one-row compact hardware controls"),
            )
            .changed();
    });
}

fn native_input_preferences(
    state: &mut NativePreferencesState,
    ui: &mut egui::Ui,
    tr: &impl Fn(&'static str) -> String,
    changed: &mut bool,
) {
    ui.heading(tr("Mouse and gesture bindings"));
    preference_section(
        ui,
        crate::ui::icons::SELECTION_ALL,
        &tr("Video surface"),
        |ui| {
            *changed |= drag_action_selector(
                ui,
                tr("Drag while paused"),
                &mut state.config.paused_drag_action,
            );
            *changed |= drag_action_selector(
                ui,
                tr("Drag while playing"),
                &mut state.config.playing_drag_action,
            );
        },
    );
}

fn native_advanced_preferences(
    state: &mut NativePreferencesState,
    ui: &mut egui::Ui,
    tr: &impl Fn(&'static str) -> String,
    changed: &mut bool,
) {
    ui.heading(tr("Configuration"));
    preference_section(
        ui,
        crate::ui::icons::APP_WINDOW,
        &tr("Application instance"),
        |ui| {
            *changed |= ui
                .checkbox(
                    &mut state.config.single_instance,
                    tr("Use a single application instance"),
                )
                .changed();
        },
    );
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
                    state.status =
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
                    state.status =
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
        crate::ui::icons::APP_WINDOW,
        &tr("Windows graphics and composition"),
        |ui| {
            *changed |= ui
                .checkbox(
                    &mut state.config.windows_dwm_theming,
                    tr("Use DWM title-bar theming"),
                )
                .changed();
            *changed |= ui
                .checkbox(
                    &mut state.config.windows_mica_backdrop,
                    tr("Use Mica backdrop (may flicker with some OpenGL drivers)"),
                )
                .changed();
            *changed |= ui
                .checkbox(
                    &mut state.config.opengl_vsync,
                    tr("Use OpenGL vertical sync"),
                )
                .changed();
        },
    );
    preference_section(
        ui,
        crate::ui::icons::APP_WINDOW,
        &tr("Dialog windows"),
        |ui| {
            *changed |= ui
                .checkbox(
                    &mut state.config.native_dialog_windows,
                    tr("Open supported dialogs in separate OS windows"),
                )
                .changed();
            ui.label(
            egui::RichText::new(tr(
                "Preferences runs as an independent owned tool window so the player stays responsive.",
            ))
            .small()
            .weak(),
        );
        },
    );
    preference_section(ui, crate::ui::icons::GAUGE, &tr("Status bar"), |ui| {
        *changed |= ui
            .checkbox(
                &mut state.config.status_bar.hardware,
                tr("Hardware connection"),
            )
            .changed();
        *changed |= ui
            .checkbox(
                &mut state.config.status_bar.status_rgb,
                tr("Physical status RGB"),
            )
            .changed();
        *changed |= ui
            .checkbox(
                &mut state.config.status_bar.warnings,
                tr("Hardware warnings"),
            )
            .changed();
        *changed |= ui
            .checkbox(&mut state.config.status_bar.media_rate, tr("Media rate"))
            .changed();
        *changed |= ui
            .checkbox(&mut state.config.status_bar.telemetry, tr("Telemetry"))
            .changed();
        *changed |= ui
            .checkbox(&mut state.config.status_bar.workspace, tr("Workspace mode"))
            .changed();
    });
    preference_section(
        ui,
        crate::ui::icons::FLOPPY_DISK,
        &tr("Config file"),
        |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        crate::config::AppConfig::get_config_path()
                            .display()
                            .to_string(),
                    )
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
                match crate::config::AppConfig::load_from_path(
                    &crate::config::AppConfig::get_config_path(),
                ) {
                    Ok(config) => {
                        state.endpoint = config.hardware_endpoint.clone().unwrap_or_default();
                        state.proxy_url = config.open_url_proxy_url.clone().unwrap_or_default();
                        state.config = config;
                        state.status = tr("Preferences reloaded from disk");
                    }
                    Err(error) => state.status = error,
                }
            }
        },
    );
}

fn draw_preferences_surface(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let mut changed = false;
    // A vertical tab rail plus the minimum useful settings column needs
    // considerably more than 500 points. Switch before either side starts
    // squeezing controls into overlapping or single-glyph columns.
    let narrow = ui.available_width() < 580.0;
    if narrow {
        draw_compact_tab_selector(app, ui);
        ui.separator();
        draw_preferences_content(app, ui, &mut changed);
    } else {
        let content_height = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(126.0, content_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| draw_tabs(app, ui, false),
            );
            ui.separator();
            draw_preferences_content(app, ui, &mut changed);
        });
    }
    if changed {
        app.save_config();
    }
}

fn draw_tabs(app: &mut PealayerApp, ui: &mut egui::Ui, compact: bool) {
    ui.spacing_mut().item_spacing.y = 5.0;
    for (index, (icon, tab)) in TABS.into_iter().enumerate() {
        let width = if compact {
            (ui.available_width() / 2.0 - 4.0).max(112.0)
        } else {
            120.0
        };
        if ui
            .add_sized(
                [width, 32.0],
                egui::Button::new(format!("{icon}  {}", app.tr(tab)))
                    .selected(app.preferences_tab == index),
            )
            .clicked()
        {
            app.preferences_tab = index;
        }
    }
}

fn draw_compact_tab_selector(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let (active_icon, active_name) = TABS[app.preferences_tab.min(TABS.len() - 1)];
    egui::ComboBox::from_id_salt("preferences_compact_tab")
        .width(ui.available_width())
        .selected_text(format!("{active_icon}  {}", app.tr(active_name)))
        .show_ui(ui, |ui| {
            for (index, (icon, name)) in TABS.into_iter().enumerate() {
                if ui
                    .selectable_label(
                        app.preferences_tab == index,
                        format!("{icon}  {}", app.tr(name)),
                    )
                    .clicked()
                {
                    app.preferences_tab = index;
                }
            }
        });
}

fn draw_preferences_content(app: &mut PealayerApp, ui: &mut egui::Ui, changed: &mut bool) {
    let detail_width = ui.available_width();
    crate::ui::dialog::scroll_column(ui, "preferences_content_v2", None, |ui| {
        ui.set_max_width((detail_width - 8.0).max(180.0));
        ui.spacing_mut().item_spacing.y = 8.0;
        match app.preferences_tab {
            0 => appearance_preferences(app, ui, changed),
            1 => playback_preferences(app, ui, changed),
            2 => hardware_preferences(app, ui, changed),
            3 => input_preferences(app, ui, changed),
            _ => advanced_preferences(app, ui),
        }
    });
}

fn appearance_preferences(app: &mut PealayerApp, ui: &mut egui::Ui, changed: &mut bool) {
    ui.heading(app.tr("Appearance and language"));
    preference_section(ui, crate::ui::icons::SPARKLE, &app.tr("Interface"), |ui| {
        preference_grid(ui, "appearance_preferences_grid", |ui| {
            ui.label(app.tr("Theme"));
            let theme_label = match app.theme_preference {
                AppTheme::System => app.tr("System"),
                AppTheme::Light => app.tr("Light"),
                AppTheme::Dark => app.tr("Dark"),
            };
            egui::ComboBox::from_id_salt("preferences_theme")
                .selected_text(theme_label)
                .show_ui(ui, |ui| {
                    for (theme, label) in [
                        (AppTheme::System, "System"),
                        (AppTheme::Light, "Light"),
                        (AppTheme::Dark, "Dark"),
                    ] {
                        if ui
                            .selectable_label(app.theme_preference == theme, app.tr(label))
                            .clicked()
                        {
                            app.set_theme(ui.ctx(), theme);
                        }
                    }
                });
            ui.end_row();
            ui.label(app.tr("Language"));
            egui::ComboBox::from_id_salt("preferences_language")
                .selected_text(match app.language_preference {
                    AppLanguage::System => app.tr("System language"),
                    AppLanguage::English => app.tr("English"),
                    AppLanguage::Persian => app.tr("Persian"),
                })
                .show_ui(ui, |ui| {
                    for (language, label) in [
                        (AppLanguage::System, "System language"),
                        (AppLanguage::English, "English"),
                        (AppLanguage::Persian, "Persian"),
                    ] {
                        if ui
                            .selectable_label(app.language_preference == language, app.tr(label))
                            .clicked()
                        {
                            app.set_language(ui.ctx(), language);
                        }
                    }
                });
            ui.end_row();
            ui.label(app.tr("Fullscreen background"));
            let background_label = match app.fullscreen_video_background {
                VideoBackground::Black => app.tr("Black"),
                VideoBackground::DarkGray => app.tr("Dark gray"),
                VideoBackground::Theme => app.tr("Use app theme"),
            };
            let black_label = app.tr("Black");
            let dark_gray_label = app.tr("Dark gray");
            let theme_label = app.tr("Use app theme");
            egui::ComboBox::from_id_salt("fullscreen_video_background")
                .selected_text(background_label)
                .show_ui(ui, |ui| {
                    *changed |= ui
                        .selectable_value(
                            &mut app.fullscreen_video_background,
                            VideoBackground::Black,
                            black_label,
                        )
                        .changed();
                    *changed |= ui
                        .selectable_value(
                            &mut app.fullscreen_video_background,
                            VideoBackground::DarkGray,
                            dark_gray_label,
                        )
                        .changed();
                    *changed |= ui
                        .selectable_value(
                            &mut app.fullscreen_video_background,
                            VideoBackground::Theme,
                            theme_label,
                        )
                        .changed();
                });
            ui.end_row();
        });
    });
    preference_section(
        ui,
        crate::ui::icons::MONITOR_PLAY,
        &app.tr("On-screen display"),
        |ui| {
            let top_left_label = app.tr("Top left");
            let center_label = app.tr("Center");
            preference_grid(ui, "osd_preferences_grid", |ui| {
                ui.label(app.tr("Position"));
                egui::ComboBox::from_id_salt("preferences_osd_position")
                    .selected_text(match app.osd_position {
                        OsdPosition::TopLeft => top_left_label.clone(),
                        OsdPosition::Center => center_label.clone(),
                    })
                    .show_ui(ui, |ui| {
                        *changed |= ui
                            .selectable_value(
                                &mut app.osd_position,
                                OsdPosition::TopLeft,
                                top_left_label,
                            )
                            .changed();
                        *changed |= ui
                            .selectable_value(
                                &mut app.osd_position,
                                OsdPosition::Center,
                                center_label,
                            )
                            .changed();
                    });
                ui.end_row();
            });
            let timeout_label = app.tr("OSD timeout (seconds)");
            *changed |= ui
                .add(
                    egui::Slider::new(&mut app.osd_timeout_seconds, 1.0..=12.0).text(timeout_label),
                )
                .changed();
        },
    );
}

fn playback_preferences(app: &mut PealayerApp, ui: &mut egui::Ui, changed: &mut bool) {
    ui.heading(app.tr("Playback behavior"));
    preference_section(
        ui,
        crate::ui::icons::PLAY,
        &app.tr("Player controls"),
        |ui| {
            let click_label = app.tr("Single-click the picture to play or pause");
            *changed |= ui
                .checkbox(&mut app.click_player_to_toggle, click_label)
                .changed();
            let subseconds_label = app.tr("Show milliseconds in time displays");
            *changed |= ui
                .checkbox(&mut app.show_subseconds, subseconds_label)
                .changed();
            let quick_seek_label = app.tr("Skip button and arrow-key step (seconds)");
            *changed |= ui
                .add(
                    egui::Slider::new(&mut app.quick_seek_seconds, 0.1..=600.0)
                        .logarithmic(true)
                        .text(quick_seek_label),
                )
                .changed();
            let frame_step_label = app.tr("Frames per frame-step action");
            *changed |= ui
                .add(egui::Slider::new(&mut app.frame_step_count, 1..=120).text(frame_step_label))
                .changed();
            let wheel_label = app.tr("Mouse-wheel seek step (seconds)");
            *changed |= ui
                .add(
                    egui::Slider::new(&mut app.wheel_seek_seconds, 0.1..=60.0)
                        .logarithmic(true)
                        .text(wheel_label),
                )
                .changed();
            ui.label(
                egui::RichText::new(
                    app.tr("Click the duration display to toggle total and remaining time."),
                )
                .weak(),
            );
        },
    );
    preference_section(
        ui,
        crate::ui::icons::LINK_SIMPLE,
        &app.tr("Open Location / URL"),
        |ui| {
            let wrap_label = app.tr("Wrap long URLs in a text area");
            *changed |= ui
                .checkbox(&mut app.open_url_multiline, wrap_label)
                .changed();
            let history_label = app.tr("Expand recent URL history by default");
            *changed |= ui
                .checkbox(&mut app.open_url_history_expanded, history_label)
                .changed();
            let mut play_recent_on_click = !app.open_url_recent_click_edits;
            let default_click_label = app.tr("Play when a recent location is clicked");
            if ui
                .checkbox(&mut play_recent_on_click, default_click_label)
                .changed()
            {
                app.open_url_recent_click_edits = !play_recent_on_click;
                *changed = true;
            }
            let proxy_label = app.tr("Use a proxy for remote inspection and playback");
            let mut proxy_settings_changed = ui
                .checkbox(&mut app.open_url_use_proxy, proxy_label)
                .changed();

            let inherited_proxy = crate::ui::open_url::inherited_proxy_url();
            if let Some(proxy) = inherited_proxy.as_deref() {
                ui.label(
                    egui::RichText::new(format!(
                        "{}: {}",
                        app.tr("Inherited proxy"),
                        crate::ui::open_url::proxy_display_value(proxy)
                    ))
                    .small()
                    .weak(),
                );
            }

            ui.label(app.tr("Custom proxy URL"));
            let proxy_changed = ui
                .add(
                    egui::TextEdit::singleline(&mut app.open_url_proxy_url)
                        .desired_width(ui.available_width())
                        .hint_text("http://proxy.example:8080"),
                )
                .changed();
            let custom_proxy = app.open_url_proxy_url.trim();
            let proxy_valid = custom_proxy.is_empty()
                || url::Url::parse(custom_proxy).is_ok_and(|value| {
                    matches!(value.scheme(), "http" | "https") && value.host_str().is_some()
                });
            if proxy_changed && proxy_valid {
                *changed = true;
                proxy_settings_changed = true;
            }
            if !proxy_valid {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    app.tr("Enter a complete HTTP or HTTPS proxy URL."),
                );
            }
            ui.label(
                egui::RichText::new(app.tr(
                    "Leave the custom proxy blank to inherit the operating-system environment.",
                ))
                .small()
                .weak(),
            );
            let playback_proxy =
                crate::mpv::proxy::playback_proxy(app.open_url_use_proxy, &app.open_url_proxy_url);
            if let Some(proxy) = playback_proxy.as_deref() {
                ui.label(
                    egui::RichText::new(format!(
                        "{}: {}",
                        app.tr("MPV playback proxy"),
                        crate::ui::open_url::proxy_display_value(proxy)
                    ))
                    .small()
                    .weak(),
                );
            } else if app.open_url_use_proxy
                && crate::ui::open_url::effective_proxy_url(&app.open_url_proxy_url).is_some()
            {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    app.tr("MPV requires an http:// proxy URL; URL inspection can still use HTTPS proxy URLs."),
                );
            }
            ui.label(
                egui::RichText::new(app.tr(
                    "MPV applies this proxy to supported HTTP media requests. HTTPS and extractor proxy support depends on the bundled MPV and FFmpeg backends.",
                ))
                .small()
                .weak(),
            );
            if proxy_settings_changed {
                *changed = true;
                if let Err(error) = crate::mpv::proxy::apply_runtime(
                    app.mpv,
                    app.open_url_use_proxy,
                    &app.open_url_proxy_url,
                ) {
                    app.show_error = Some(error);
                }
            }

            let remote_count = app
                .recent_media
                .iter()
                .filter(|path| crate::media::is_remote_media_target(&path.to_string_lossy()))
                .count();
            let clear_label = format!("{} ({remote_count})", app.tr("Clear remote history"));
            if ui
                .add_enabled(
                    remote_count > 0,
                    egui::Button::new(format!("{}  {clear_label}", crate::ui::icons::TRASH)),
                )
                .clicked()
            {
                app.clear_recent_remote_media();
            }
        },
    );
}

fn hardware_preferences(app: &mut PealayerApp, ui: &mut egui::Ui, changed: &mut bool) {
    ui.heading(app.tr("PCController and hardware"));
    preference_section(ui, crate::ui::icons::PLUG, &app.tr("Connection"), |ui| {
        let reconnect_label = app.tr("Discover and connect to PCController on startup");
        *changed |= ui
            .checkbox(&mut app.auto_connect_hardware, reconnect_label)
            .changed();
        let pause_label = app.tr("Pause playback when hardware disconnects unexpectedly");
        *changed |= ui
            .checkbox(&mut app.pause_on_hardware_disconnect, pause_label)
            .changed();
        ui.label(app.tr("Preferred endpoint"));
        let endpoint_hint = app.tr("pccontroller://host:port, tcp://host:port, or direct:<device>");
        *changed |= ui
            .add(
                egui::TextEdit::singleline(&mut app.serial_port)
                    .desired_width(ui.available_width())
                    .hint_text(endpoint_hint),
            )
            .changed();
        if let Some(notice) = &app.connection_notice {
            ui.colored_label(ui.visuals().warn_fg_color, notice);
        }
    });
    preference_section(
        ui,
        crate::ui::icons::SEAT,
        &app.tr("Motion controls"),
        |ui| {
            let toggle_label = app.tr("Toggle on press");
            let hold_label = app.tr("Run only while held");
            let compact_label = app.tr("Use one-row compact hardware controls");
            preference_grid(ui, "motion_control_preferences", |ui| {
                ui.label(app.tr("Button behavior"));
                egui::ComboBox::from_id_salt("motion_control_mode")
                    .selected_text(match app.motion_control_mode {
                        MotionControlMode::Toggle => toggle_label.clone(),
                        MotionControlMode::Hold => hold_label.clone(),
                    })
                    .show_ui(ui, |ui| {
                        *changed |= ui
                            .selectable_value(
                                &mut app.motion_control_mode,
                                MotionControlMode::Toggle,
                                &toggle_label,
                            )
                            .changed();
                        *changed |= ui
                            .selectable_value(
                                &mut app.motion_control_mode,
                                MotionControlMode::Hold,
                                &hold_label,
                            )
                            .changed();
                    });
                ui.end_row();
            });
            *changed |= ui
                .checkbox(&mut app.compact_hardware_controls, compact_label)
                .changed();
            ui.label(
                egui::RichText::new(app.tr(
                    "Toggle mode keeps a direction active until another action is chosen. Hold mode sends Stop when the pressed direction is released.",
                ))
                .small()
                .weak(),
            );
        },
    );
}

fn input_preferences(app: &mut PealayerApp, ui: &mut egui::Ui, changed: &mut bool) {
    ui.heading(app.tr("Mouse and gesture bindings"));
    preference_section(
        ui,
        crate::ui::icons::SELECTION_ALL,
        &app.tr("Video surface"),
        |ui| {
            *changed |=
                drag_action_selector(ui, app.tr("Drag while paused"), &mut app.paused_drag_action);
            *changed |= drag_action_selector(
                ui,
                app.tr("Drag while playing"),
                &mut app.playing_drag_action,
            );
            ui.label(egui::RichText::new(app.tr("Ctrl+wheel over the picture adjusts volume; wheel seeks in the reversed Y direction.")).weak());
        },
    );
}

fn advanced_preferences(app: &mut PealayerApp, ui: &mut egui::Ui) {
    ui.heading(app.tr("Configuration"));
    preference_section(
        ui,
        crate::ui::icons::APP_WINDOW,
        &app.tr("Application instance"),
        |ui| {
            let label = app.tr("Use a single application instance");
            if ui.checkbox(&mut app.single_instance, label).changed() {
                app.save_config();
            }
            ui.label(
                egui::RichText::new(app.tr(
                    "When enabled, files and player commands from a new Pealayer process are delivered to the active window through native local IPC.",
                ))
                .small()
                .weak(),
            );
        },
    );
    preference_section(
        ui,
        crate::ui::icons::FILE_VIDEO,
        &app.tr("File associations"),
        |ui| {
            let registered = crate::platform::associations::SUPPORTED_EXTENSIONS
                .iter()
                .filter(|extension| {
                    crate::platform::associations::is_file_association_registered(extension)
                })
                .count();
            ui.label(format!(
                "{}: {registered}/{}",
                app.tr("Registered media types"),
                crate::platform::associations::SUPPORTED_EXTENSIONS.len()
            ));
            ui.label(
                egui::RichText::new(app.tr(
                    "Register Pealayer with the operating system, then choose it as the default app for the media types you want.",
                ))
                .small()
                .weak(),
            );
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(format!(
                        "{}  {}",
                        crate::ui::icons::CHECK_SQUARE,
                        app.tr("Register as a media player")
                    ))
                    .clicked()
                {
                    match crate::platform::associations::register_file_associations(None) {
                        Ok(count) => {
                            app.set_osd(format!("{}: {count}", app.tr("Registered media types")))
                        }
                        Err(error) => app.show_error = Some(error),
                    }
                }
                if ui
                    .add_enabled(
                        registered > 0,
                        egui::Button::new(format!(
                            "{}  {}",
                            crate::ui::icons::X,
                            app.tr("Remove file associations")
                        )),
                    )
                    .clicked()
                {
                    match crate::platform::associations::unregister_file_associations() {
                        Ok(count) => {
                            app.set_osd(format!("{}: {count}", app.tr("Removed media types")))
                        }
                        Err(error) => app.show_error = Some(error),
                    }
                }
            });
        },
    );
    preference_section(
        ui,
        crate::ui::icons::APP_WINDOW,
        &app.tr("Windows graphics and composition"),
        |ui| {
            let mut changed = false;
            let dwm_label = app.tr("Use DWM title-bar theming");
            let mica_label = app.tr("Use Mica backdrop (may flicker with some OpenGL drivers)");
            let vsync_label = app.tr("Use OpenGL vertical sync");
            changed |= ui
                .checkbox(&mut app.windows_dwm_theming, dwm_label)
                .changed();
            changed |= ui
                .checkbox(&mut app.windows_mica_backdrop, mica_label)
                .changed();
            changed |= ui.checkbox(&mut app.opengl_vsync, vsync_label).changed();
            ui.label(
                egui::RichText::new(app.tr(
                    "Mica and DWM changes apply immediately. OpenGL vertical sync applies after restart.",
                ))
                .small()
                .weak(),
            );
            if changed {
                crate::platform::windows::configure_window_composition(
                    app.windows_dwm_theming,
                    app.windows_mica_backdrop,
                );
                app.save_config();
            }
        },
    );
    preference_section(
        ui,
        crate::ui::icons::APP_WINDOW,
        &app.tr("Dialog windows"),
        |ui| {
            let label = app.tr("Open supported dialogs in separate OS windows");
            if ui
                .checkbox(&mut app.native_dialog_windows, label)
                .on_hover_text(
                    app.tr("Allows supported dialogs to move outside the main application window."),
                )
                .changed()
            {
                app.save_config();
            }
            ui.label(
                egui::RichText::new(app.tr(
                    "Preferences runs as an independent owned tool window so the player stays responsive.",
                ))
                .small()
                .weak(),
            );
        },
    );
    preference_section(ui, crate::ui::icons::GAUGE, &app.tr("Status bar"), |ui| {
        let hardware_label = app.tr("Hardware connection");
        let rgb_label = app.tr("Physical status RGB");
        let warnings_label = app.tr("Hardware warnings");
        let media_rate_label = app.tr("Media rate");
        let telemetry_label = app.tr("Telemetry");
        let workspace_label = app.tr("Workspace mode");
        let mut changed = false;
        changed |= ui
            .checkbox(&mut app.status_bar.hardware, hardware_label)
            .changed();
        changed |= ui
            .checkbox(&mut app.status_bar.status_rgb, rgb_label)
            .changed();
        changed |= ui
            .checkbox(&mut app.status_bar.warnings, warnings_label)
            .changed();
        changed |= ui
            .checkbox(&mut app.status_bar.media_rate, media_rate_label)
            .changed();
        changed |= ui
            .checkbox(&mut app.status_bar.telemetry, telemetry_label)
            .changed();
        changed |= ui
            .checkbox(&mut app.status_bar.workspace, workspace_label)
            .changed();
        if changed {
            app.save_config();
        }
    });
    preference_section(
        ui,
        crate::ui::icons::FLOPPY_DISK,
        &app.tr("Config file"),
        |ui| {
            ui.label(app.tr("Settings are stored atomically in the active portable or OS-native per-user JSON configuration and watched for external changes."));
            ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        crate::config::AppConfig::get_config_path()
                            .display()
                            .to_string(),
                    )
                    .monospace(),
                )
                .wrap(),
            );
            ui.horizontal(|ui| {
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::FLOPPY_DISK,
                        app.tr("Save now")
                    ))
                    .clicked()
                {
                    app.save_config();
                    app.set_osd(app.tr("Preferences saved"));
                }
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                        app.tr("Reload from disk")
                    ))
                    .clicked()
                {
                    match app.reload_config_from_disk(ui.ctx()) {
                        Ok(()) => app.set_osd(app.tr("Preferences reloaded from disk")),
                        Err(error) => app.set_osd(error),
                    }
                }
            });
            if !app.config_status.is_empty() {
                ui.label(egui::RichText::new(&app.config_status).small().weak());
            }
            ui.label(
                egui::RichText::new(
                    "API: GET/POST /api/config · JSON-RPC pealayer.config.get/update/reload",
                )
                .small()
                .monospace()
                .weak(),
            );
        },
    );
}

fn drag_action_selector(ui: &mut egui::Ui, label: String, value: &mut PlayerDragAction) -> bool {
    let before = *value;
    egui::Grid::new(ui.next_auto_id())
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            ui.label(label);
            egui::ComboBox::from_id_salt(ui.next_auto_id())
                .selected_text(drag_action_name(*value))
                .show_ui(ui, |ui| {
                    for action in [
                        PlayerDragAction::MoveWindow,
                        PlayerDragAction::Seek,
                        PlayerDragAction::TemporaryFastForward,
                        PlayerDragAction::None,
                    ] {
                        ui.selectable_value(value, action, drag_action_name(action));
                    }
                });
            ui.end_row();
        });
    before != *value
}

fn preference_section(
    ui: &mut egui::Ui,
    icon: &str,
    title: &str,
    body: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(12))
        .corner_radius(9.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(format!("{icon}  {title}")).strong());
                ui.add_space(6.0);
                body(ui);
            });
        });
    ui.add_space(8.0);
}

fn preference_grid(ui: &mut egui::Ui, id: &'static str, body: impl FnOnce(&mut egui::Ui)) {
    let narrow = ui.available_width() < 360.0;
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([if narrow { 8.0 } else { 18.0 }, 10.0])
        .min_col_width(if narrow { 72.0 } else { 120.0 })
        .show(ui, body);
}

fn drag_action_name(action: PlayerDragAction) -> &'static str {
    match action {
        PlayerDragAction::MoveWindow => "Move window",
        PlayerDragAction::Seek => "Seek",
        PlayerDragAction::TemporaryFastForward => "Temporary fast-forward",
        PlayerDragAction::None => "No action",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn deferred_preferences_state_is_safe_to_share_with_the_viewport_renderer() {
        assert_send_sync::<NativePreferencesController>();
    }

    #[test]
    fn native_preferences_use_owned_tool_window_chrome() {
        let builder = native_preferences_viewport("Preferences — Pealayer".to_string());
        assert_eq!(builder.taskbar, Some(false));
        assert_eq!(builder.decorations, Some(true));
        assert_eq!(builder.minimize_button, Some(false));
        assert_eq!(builder.maximize_button, Some(false));
        assert_eq!(builder.resizable, Some(true));
    }
}
