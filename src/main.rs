#![windows_subsystem = "windows"]

pub mod app;
pub mod cli;
pub mod config;
pub mod four_d;
pub mod mpv;
pub mod platform;
pub mod server;
pub mod ui;

use app::PealayerApp;
use eframe::egui;
use libmpv2::{
    Mpv,
    render::{OpenGLInitParams, RenderParam, RenderParamApiType},
};
use mpv::render::RenderContextWrapper;
use mpv::render::mpv_get_proc_address;
use std::sync::{Arc, Mutex};

fn subtitle_font_directory() -> Option<std::path::PathBuf> {
    let packaged = std::env::current_exe()
        .ok()
        .and_then(|executable| {
            executable
                .parent()
                .map(|directory| directory.join("assets/fonts"))
        })
        .filter(|directory| directory.join("Vazirmatn-Regular.ttf").is_file());
    packaged.or_else(|| {
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fonts");
        source
            .join("Vazirmatn-Regular.ttf")
            .is_file()
            .then_some(source)
    })
}

fn main() -> eframe::Result {
    if std::env::args().any(|argument| argument == "--smoke-test") {
        match Mpv::new() {
            Ok(_) => std::process::exit(0),
            Err(error) => {
                eprintln!("libmpv smoke test failed: {error}");
                std::process::exit(2);
            }
        }
    }

    env_logger::init();

    #[cfg(target_os = "windows")]
    let gui_ownership;

    let args: Vec<String> = std::env::args().collect();
    let cli_options = match crate::cli::parse_cli_args(args) {
        Ok(crate::cli::CliAction::PrintHelp(msg)) => {
            println!("{}", msg);
            return Ok(());
        }
        Ok(crate::cli::CliAction::PrintVersion(ver)) => {
            println!("{}", ver);
            return Ok(());
        }
        Ok(crate::cli::CliAction::SendRemote(cmd)) => match crate::cli::send_remote_command(&cmd) {
            Ok(resp) => {
                println!("{}", resp);
                return Ok(());
            }
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        },
        Ok(crate::cli::CliAction::RegisterAssociations) => {
            match crate::platform::associations::register_file_associations(None) {
                Ok(count) => {
                    println!(
                        "Successfully registered Pealayer for {} media file types.",
                        count
                    );
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("Failed to register file associations: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Ok(crate::cli::CliAction::UnregisterAssociations) => {
            match crate::platform::associations::unregister_file_associations() {
                Ok(count) => {
                    println!(
                        "Successfully unregistered Pealayer media file associations ({} processed).",
                        count
                    );
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("Failed to unregister file associations: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Ok(crate::cli::CliAction::RunGui(opts)) => {
            let launch_request = crate::cli::launch_request(&opts);
            if crate::cli::try_forward_launch_request(&launch_request) {
                println!("Forwarded launch request to active Pealayer instance.");
                return Ok(());
            }
            #[cfg(target_os = "windows")]
            {
                let config = crate::config::AppConfig::load();
                let app_identity = crate::config::resolved_app_name(&config);
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                loop {
                    match crate::platform::windows::acquire_gui_ownership(&app_identity) {
                        Ok(crate::platform::windows::GuiOwnership::Primary(owner)) => {
                            gui_ownership = owner;
                            break;
                        }
                        Ok(crate::platform::windows::GuiOwnership::Existing) => {
                            if crate::cli::try_forward_launch_request(&launch_request) {
                                println!("Forwarded launch request to active Pealayer instance.");
                                return Ok(());
                            }
                            if std::time::Instant::now() >= deadline {
                                eprintln!(
                                    "The active Pealayer instance did not accept the launch request."
                                );
                                std::process::exit(3);
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        Err(error) => {
                            eprintln!("Could not establish Pealayer GUI ownership: {error}");
                            std::process::exit(3);
                        }
                    }
                }
            }
            opts
        }
        Err(err) => {
            eprintln!("Error: {}\nRun 'pealayer --help' for usage.", err);
            std::process::exit(1);
        }
    };

    #[cfg(target_os = "windows")]
    let _gui_ownership = gui_ownership;

    let launch_config = crate::config::AppConfig::load();
    let app_name = crate::config::resolved_app_name(&launch_config);
    let language_preference = crate::config::resolved_language_preference(&launch_config);
    let language = crate::config::resolve_language(language_preference);
    let direction_preference = crate::config::resolved_direction_preference(&launch_config);
    let rtl = crate::config::resolve_rtl(direction_preference, language);
    let initial_window_title = app_name.clone();
    let icon_data = crate::config::resolved_app_icon(&launch_config)
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| eframe::icon_data::from_png_bytes(&bytes).ok())
        .or_else(|| {
            eframe::icon_data::from_png_bytes(include_bytes!("../assets/pealayer-icon.png")).ok()
        });

    let mut viewport = egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]);
    if cli_options.fullscreen {
        viewport = viewport.with_fullscreen(true);
    }
    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        &initial_window_title,
        options,
        Box::new(move |cc| {
            let loaded_config = launch_config.clone();
            crate::ui::i18n::configure_ui_fonts(
                &cc.egui_ctx,
                language == crate::config::AppLanguage::Persian,
            );
            let theme_preference = match crate::config::resolved_theme(&loaded_config) {
                crate::config::AppTheme::System => egui::ThemePreference::System,
                crate::config::AppTheme::Light => egui::ThemePreference::Light,
                crate::config::AppTheme::Dark => egui::ThemePreference::Dark,
            };
            cc.egui_ctx.set_theme(theme_preference);
            let mut dark_visuals = egui::Visuals::dark();
            dark_visuals.panel_fill = egui::Color32::from_rgb(33, 33, 33);
            dark_visuals.window_fill = egui::Color32::from_rgb(26, 26, 26);
            cc.egui_ctx.set_visuals_of(egui::Theme::Dark, dark_visuals);
            cc.egui_ctx
                .set_visuals_of(egui::Theme::Light, egui::Visuals::light());
            crate::platform::windows::set_window_theme(
                cc.egui_ctx.global_style().visuals.dark_mode,
            );

            let mut style = (*cc.egui_ctx.global_style()).clone();
            for font_id in style.text_styles.values_mut() {
                if font_id.size > 12.0 {
                    font_id.size = 12.0;
                }
            }
            cc.egui_ctx.set_global_style(style);

            let get_proc = cc
                .get_proc_address
                .clone()
                .expect("Glow backend must provide get_proc_address");

            let subtitle_font_directory = subtitle_font_directory();
            let mpv = Mpv::with_initializer(|init| {
                init.set_property("vo", "libmpv")?;
                init.set_property("keep-open", "always")?;

                // Set up Arabic/Farsi Vazirmatn font for subtitles
                if let Some(font_dir_str) = subtitle_font_directory
                    .as_deref()
                    .and_then(std::path::Path::to_str)
                {
                    init.set_property("sub-fonts-dir", font_dir_str)?;
                }
                init.set_property("sub-font", "Vazirmatn")?;

                Ok(())
            })
            .unwrap();

            let mpv_static: &'static Mpv = Box::leak(Box::new(mpv));

            let mut render_context = mpv_static
                .create_render_context(vec![
                    RenderParam::ApiType(RenderParamApiType::OpenGl),
                    RenderParam::InitParams(OpenGLInitParams {
                        get_proc_address: mpv_get_proc_address,
                        ctx: get_proc,
                    }),
                ])
                .expect("Failed creating render context");

            let egui_ctx = cc.egui_ctx.clone();
            render_context.set_update_callback(move || {
                egui_ctx.request_repaint();
            });

            let mut mpv_client = mpv_static.create_client(None).unwrap();

            // Observe MPV properties for UI state ON THE CLIENT that receives events
            mpv_client
                .observe_property("time-pos", libmpv2::Format::Double, 1)
                .unwrap();
            mpv_client
                .observe_property("duration", libmpv2::Format::Double, 2)
                .unwrap();
            mpv_client
                .observe_property("pause", libmpv2::Format::Flag, 3)
                .unwrap();
            mpv_client
                .observe_property("volume", libmpv2::Format::Double, 4)
                .unwrap();
            mpv_client
                .observe_property("mute", libmpv2::Format::Flag, 5)
                .unwrap();
            mpv_client
                .observe_property("sub-visibility", libmpv2::Format::Flag, 6)
                .unwrap();
            mpv_client
                .observe_property("sub-font-size", libmpv2::Format::Double, 7)
                .unwrap();
            mpv_client
                .observe_property("sub-delay", libmpv2::Format::Double, 8)
                .unwrap();
            mpv_client
                .observe_property("sid", libmpv2::Format::String, 9)
                .unwrap();
            mpv_client
                .observe_property("audio-delay", libmpv2::Format::Double, 10)
                .unwrap();
            mpv_client
                .observe_property("aid", libmpv2::Format::String, 11)
                .unwrap();
            mpv_client
                .observe_property("eof-reached", libmpv2::Format::Flag, 12)
                .unwrap();
            mpv_client
                .observe_property("estimated-vf-fps", libmpv2::Format::Double, 13)
                .unwrap();

            let egui_ctx2 = cc.egui_ctx.clone();
            mpv_client.set_wakeup_callback(move || {
                egui_ctx2.request_repaint();
            });

            let initial_volume = cli_options.volume.unwrap_or(loaded_config.volume);
            let _ = mpv_static.set_property("volume", initial_volume);
            let _ = mpv_static.set_property("mute", loaded_config.is_muted);
            crate::platform::windows::sync_windows_jump_list(&loaded_config.recent_media);

            let (interop_tx, interop_rx) = std::sync::mpsc::channel();
            crate::platform::interop::spawn_interop_listener(
                interop_tx.clone(),
                cc.egui_ctx.clone(),
                app_name.clone(),
            );

            let http_port = crate::config::runtime_port("PEALAYER_HTTP_PORT", 8080);
            let ws_port = crate::config::runtime_port("PEALAYER_WS_PORT", 8081);
            let web_runtime = crate::server::WebRuntimeConfig::production(
                app_name.clone(),
                ws_port,
                match language {
                    crate::config::AppLanguage::Persian => "fa",
                    _ => "en",
                }
                .to_string(),
                if rtl { "rtl" } else { "ltr" }.to_string(),
                match crate::config::resolved_theme(&loaded_config) {
                    crate::config::AppTheme::Light => "light",
                    crate::config::AppTheme::Dark => "dark",
                    crate::config::AppTheme::System => "system",
                }
                .to_string(),
            );
            let (web_state_tx, web_cmd_rx) = crate::server::spawn_web_server_configured(
                http_port,
                ws_port,
                cc.egui_ctx.clone(),
                web_runtime,
            );
            let engine_handle = crate::four_d::engine::spawn_engine();
            let controller_cmd_rx = crate::platform::interop::spawn_pccontroller_action_bridge(
                cc.egui_ctx.clone(),
                engine_handle.controller_push_target(),
            );

            let mut app = PealayerApp {
                app_name: app_name.clone(),
                app_publisher: crate::config::resolved_app_publisher(&loaded_config),
                app_copyright: crate::config::resolved_app_copyright(&loaded_config),
                last_window_title: String::new(),
                language_preference,
                language,
                direction_preference,
                theme_preference: crate::config::resolved_theme(&loaded_config),
                rtl,
                mpv: mpv_static,
                mpv_client,
                render_context: Arc::new(Mutex::new(Some(RenderContextWrapper(render_context)))),
                playback_time: 0.0,
                duration: 0.0,
                media_fps: 0.0,
                is_paused: false,
                is_eof: false,
                volume: initial_volume,
                is_muted: loaded_config.is_muted,
                show_sub_settings: false,
                sub_visibility: true,
                sub_font_size: 55.0,
                sub_delay: 0.0,
                current_sid: "no".to_string(),
                sub_tracks: Vec::new(),
                show_audio_settings: false,
                audio_delay: 0.0,
                current_aid: "no".to_string(),
                audio_tracks: Vec::new(),
                seek_pos: None,
                seek_controller: crate::mpv::seek::SeekController::new(
                    crate::mpv::seek::MpvSeekBackend::new(mpv_static),
                ),
                was_playing_before_scrub: false,
                is_scrubbing: false,
                last_mouse_activity: std::time::Instant::now(),
                pin_controls: loaded_config.pin_controls,
                show_error: None,
                show_four_d_editor: true,

                timeline: crate::four_d::models::Timeline::new(),
                engine_handle,
                recording_session: crate::four_d::curve_record::RecordingSession::new(),
                input_capture: crate::four_d::input_capture::InputCaptureState::new(),
                is_recording: false,
                hardware_effect_authoring: crate::app::HardwareEffectAuthoringState::default(),
                recording_keys: std::collections::HashMap::new(),
                dock_state: crate::ui::layout::create_initial_layout(),
                rtt_state: Arc::new(Mutex::new(crate::app::RttState {
                    video_texture: None,
                    video_fbo: None,
                    video_texture_id: None,
                    texture_width: 1920,
                    texture_height: 1080,
                })),
                selected_instance_ids: std::collections::HashSet::new(),
                selected_keyframes: std::collections::HashSet::new(),
                active_keyframe_drag: None,
                timeline_zoom: 100.0,
                undo_stack: crate::four_d::history::UndoStack::default(),
                relay_overrides: std::collections::BTreeSet::new(),
                effects_search_query: String::new(),
                track_muted: std::collections::BTreeSet::new(),
                track_soloed: std::collections::BTreeSet::new(),
                track_locked: std::collections::BTreeSet::new(),
                active_drag: None,
                estop_active: false,
                serial_port: loaded_config
                    .hardware_endpoint
                    .clone()
                    .unwrap_or_else(|| crate::four_d::controller::DEFAULT_ENDPOINT.to_string()),
                is_connected: false,
                lasso_origin: None,
                lasso_rect: None,
                current_video_path: None,
                show_remaining_time: loaded_config.show_remaining_time,
                osd_message: None,
                recent_media: loaded_config.recent_media.clone(),
                show_open_url_dialog: false,
                url_input_buffer: String::new(),
                is_window_operating: false,
                show_shortcuts_dialog: false,
                show_about_dialog: false,
                show_preferences_dialog: false,
                preferences_tab: 0,
                pause_on_hardware_disconnect: loaded_config.pause_on_hardware_disconnect,
                auto_connect_hardware: loaded_config.auto_connect_hardware,
                click_player_to_toggle: loaded_config.click_player_to_toggle,
                show_subseconds: loaded_config.show_subseconds,
                wheel_seek_seconds: loaded_config.wheel_seek_seconds,
                osd_position: loaded_config.osd_position,
                osd_timeout_seconds: loaded_config.osd_timeout_seconds,
                paused_drag_action: loaded_config.paused_drag_action,
                playing_drag_action: loaded_config.playing_drag_action,
                fullscreen_video_background: loaded_config.fullscreen_video_background,
                status_bar: loaded_config.status_bar,
                was_hardware_connected: false,
                was_board_connected: false,
                connection_notice: None,
                workspace_before_fullscreen: None,
                // Let the first frame observe a CLI-started fullscreen viewport
                // as an entry transition so it always switches to Simple mode.
                was_fullscreen: false,
                desired_fullscreen: None,
                interop_rx,
                controller_cmd_rx,
                web_state_tx,
                web_cmd_rx,
                last_web_broadcast: None,
                media_controls: None,
                media_cmd_tx: interop_tx,
                window_handle: None,
                shell_initialized: false,
            };

            if app.auto_connect_hardware {
                let configured_endpoint = app.serial_port.clone();
                let selected_endpoint =
                    crate::four_d::controller::select_autoconnect_endpoint(
                        &configured_endpoint,
                        std::time::Duration::from_millis(250),
                    );
                if let Some(endpoint) = selected_endpoint {
                    app.serial_port = endpoint.clone();
                    if let Ok(mut selected) = app.engine_handle.serial_port.lock() {
                        *selected = endpoint;
                    }
                    app.engine_handle
                        .connection_requested
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }

            if let Some(target) = cli_options.target {
                if target.starts_with("http://") || target.starts_with("https://") {
                    app.load_url(&target);
                } else {
                    app.load_video_file(std::path::PathBuf::from(target));
                }
            }

            Ok(Box::new(app))
        }),
    )
}
