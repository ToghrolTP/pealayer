#![windows_subsystem = "windows"]

pub mod app;
pub mod cli;
pub mod config;
pub mod duration;
pub mod four_d;
pub mod hardware_shortcuts;
pub mod media;
pub mod mpv;
pub mod platform;
pub mod preferences_contract;
pub mod server;
pub mod subtitle;
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
    let startup_args: Vec<String> = std::env::args().collect();
    if startup_args
        .iter()
        .any(|argument| argument == "--smoke-test")
    {
        match Mpv::new() {
            Ok(_) => std::process::exit(0),
            Err(error) => {
                eprintln!("libmpv smoke test failed: {error}");
                std::process::exit(2);
            }
        }
    }

    env_logger::init();

    if let Some(owner_hwnd) = crate::ui::preferences::preferences_helper_owner(&startup_args) {
        return crate::ui::preferences::run_native_preferences(owner_hwnd);
    }

    #[cfg(target_os = "windows")]
    let mut gui_ownership = None;

    let cli_options = match crate::cli::parse_cli_args(startup_args) {
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
            let config = crate::config::AppConfig::load();
            let launch_request = crate::cli::launch_request(&opts);
            if config.single_instance && crate::cli::try_forward_launch_request(&launch_request) {
                println!("Forwarded launch request to active Pealayer instance.");
                return Ok(());
            }
            #[cfg(target_os = "windows")]
            if config.single_instance {
                let app_identity = crate::cli::resolved_instance_identity();
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                loop {
                    match crate::platform::windows::acquire_gui_ownership(&app_identity) {
                        Ok(crate::platform::windows::GuiOwnership::Primary(owner)) => {
                            gui_ownership = Some(owner);
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
    crate::platform::interop::set_live_config(launch_config.clone());
    let app_name = crate::config::resolved_app_name(&launch_config);
    let language_preference = crate::config::resolved_language_preference(&launch_config);
    let language = crate::config::resolve_language(language_preference);
    let direction_preference = crate::config::resolved_direction_preference(&launch_config);
    let rtl = crate::config::resolve_rtl(direction_preference, language);
    crate::platform::windows::configure_window_composition(
        launch_config.windows_dwm_theming,
        launch_config.windows_mica_backdrop,
    );
    let initial_window_title = app_name.clone();
    let icon_data = crate::config::resolved_app_icon(&launch_config)
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| eframe::icon_data::from_png_bytes(&bytes).ok())
        .or_else(|| {
            eframe::icon_data::from_png_bytes(include_bytes!("../assets/pealayer-icon.png")).ok()
        });

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([800.0, 600.0])
        .with_clamp_size_to_monitor_size(true);
    if let Some(geometry) = launch_config
        .workspace_session
        .window_geometry
        .or(launch_config.window_geometry)
        .filter(|geometry| geometry.is_valid())
    {
        viewport = viewport
            .with_inner_size([geometry.width, geometry.height])
            .with_position([geometry.x, geometry.y])
            .with_maximized(geometry.maximized);
    }
    if cli_options.fullscreen {
        viewport = viewport.with_fullscreen(true);
    }
    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        persistence_path: Some(
            crate::config::AppConfig::get_config_path().with_file_name("workspace-state.ron"),
        ),
        glow_options: eframe::egui_glow::GlowConfiguration {
            vsync: launch_config.opengl_vsync,
            ..Default::default()
        },
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
            crate::ui::configure_native_visuals(&cc.egui_ctx, &loaded_config);
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
                crate::mpv::proxy::apply_before_initialize(
                    &init,
                    loaded_config.open_url_use_proxy,
                    loaded_config
                        .open_url_proxy_url
                        .as_deref()
                        .unwrap_or_default(),
                )?;

                // Set up Arabic/Farsi Vazirmatn font for subtitles
                if let Some(font_dir_str) = subtitle_font_directory
                    .as_deref()
                    .and_then(std::path::Path::to_str)
                {
                    init.set_property("sub-fonts-dir", font_dir_str)?;
                }
                init.set_property("sub-font", "Vazirmatn")?;
                // ASS/SSA tracks normally keep their embedded FontName and
                // ignore `sub-font`. Force mpv's normal subtitle styling so
                // the bundled Vazirmatn face is also stable in the native
                // fallback path (processed text uses the same face explicitly).
                init.set_property("sub-ass-override", "force")?;

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
                // `estimated-vf-fps` is a live decoder estimate and visibly jitters
                // whenever the UI repaints.  The status bar promises the media rate,
                // so observe the stable container metadata instead.
                .observe_property("container-fps", libmpv2::Format::Double, 13)
                .unwrap();
            mpv_client
                .observe_property("seekable", libmpv2::Format::Flag, 14)
                .unwrap();
            mpv_client
                .observe_property("demuxer-cache-duration", libmpv2::Format::Double, 15)
                .unwrap();
            mpv_client
                .observe_property("cache-buffering-state", libmpv2::Format::Int64, 16)
                .unwrap();
            mpv_client
                .observe_property("speed", libmpv2::Format::Double, 17)
                .unwrap();
            mpv_client
                .observe_property("vid", libmpv2::Format::String, 18)
                .unwrap();
            mpv_client
                .observe_property("sub-text", libmpv2::Format::String, 19)
                .unwrap();

            let egui_ctx2 = cc.egui_ctx.clone();
            mpv_client.set_wakeup_callback(move || {
                egui_ctx2.request_repaint();
            });

            let initial_volume = cli_options.volume.unwrap_or(loaded_config.volume);
            let startup_media_target = crate::media::startup_media_target(
                cli_options.target.as_deref(),
                loaded_config.restore_last_media_on_startup,
                loaded_config.last_media_target.as_deref(),
                &loaded_config.recent_media,
                &loaded_config.playback_positions,
            );
            // An explicit CLI media target starts normally. Only an automatic
            // last-session restore inherits the persisted pause state.
            let restore_startup_pause = cli_options
                .target
                .as_deref()
                .map(str::trim)
                .filter(|target| !target.is_empty())
                .is_none()
                && startup_media_target.is_some()
                && loaded_config.last_media_paused;
            let _ = mpv_static.set_property("volume", initial_volume);
            let _ = mpv_static.set_property("mute", loaded_config.is_muted);
            let _ = mpv_static.set_property("speed", loaded_config.playback_speed);
            crate::platform::windows::sync_windows_jump_list(&loaded_config.recent_media);

            let (interop_tx, interop_rx) = std::sync::mpsc::channel();
            crate::platform::interop::spawn_interop_listener(
                interop_tx.clone(),
                cc.egui_ctx.clone(),
                crate::cli::resolved_instance_identity(),
            );

            let control_port = crate::config::control_port();
            let web_runtime = crate::server::WebRuntimeConfig::production(
                app_name.clone(),
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
                crate::ui::platform_accent_rgb(&loaded_config),
            );
            let web_state_tx = crate::server::spawn_control_server_configured(
                control_port,
                cc.egui_ctx.clone(),
                web_runtime,
                interop_tx.clone(),
                crate::cli::resolved_instance_identity(),
            );
            let (_web_cmd_tx, web_cmd_rx) = std::sync::mpsc::channel();
            let engine_handle = crate::four_d::engine::spawn_engine();
            let controller_cmd_rx = crate::platform::interop::spawn_pccontroller_action_bridge(
                cc.egui_ctx.clone(),
                engine_handle.controller_push_target(),
            );

            let dock_state = loaded_config
                .workspace_session
                .dock_layout
                .as_deref()
                .or(loaded_config.workspace_dock_layout.as_deref())
                .and_then(|json| {
                    let mut ds = serde_json::from_str::<
                        egui_dock::DockState<crate::ui::layout::PealayerTab>,
                    >(json)
                    .ok()?;
                    crate::ui::layout::sanitize_dock_rects(&mut ds);
                    Some(ds)
                })
                .unwrap_or_else(crate::ui::layout::create_initial_layout);
            let mut workspace_profiles = loaded_config.workspace_profiles.clone();
            let default_dock_layout =
                serde_json::to_string(&crate::ui::layout::create_initial_layout()).ok();
            for id in ["simple", "nle"] {
                if let Some(profile) = workspace_profiles.get_mut(id)
                    && profile.dock_layout.is_none()
                {
                    profile.dock_layout = default_dock_layout.clone();
                }
            }

            let mut app = PealayerApp {
                app_name: app_name.clone(),
                app_publisher: crate::config::resolved_app_publisher(&loaded_config),
                app_copyright: crate::config::resolved_app_copyright(&loaded_config),
                last_window_title: String::new(),
                window_geometry: loaded_config
                    .workspace_session
                    .window_geometry
                    .or(loaded_config.window_geometry),
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
                is_seekable: false,
                media_metadata_loaded: false,
                cache_duration: None,
                cache_buffering_percent: None,
                media_fps: 0.0,
                is_paused: false,
                is_eof: false,
                volume: initial_volume,
                is_muted: loaded_config.is_muted,
                playback_rate: loaded_config.playback_speed,
                configured_playback_speed: loaded_config.playback_speed,
                temporary_fast_forward_speed: loaded_config.temporary_fast_forward_speed,
                video_surface_gesture: None,
                show_sub_settings: loaded_config.workspace_session.dialogs.subtitles,
                sub_visibility: true,
                sub_font_size: 55.0,
                sub_delay: 0.0,
                current_sid: "no".to_string(),
                sub_tracks: Vec::new(),
                subtitle_direction: loaded_config.subtitle_direction,
                subtitle_text_replacements: loaded_config.subtitle_text_replacements.clone(),
                subtitle_text: String::new(),
                current_vid: "no".to_string(),
                video_tracks: Vec::new(),
                show_audio_settings: loaded_config.workspace_session.dialogs.audio,
                audio_delay: 0.0,
                current_aid: "no".to_string(),
                audio_tracks: Vec::new(),
                media_tracks: Vec::new(),
                media_track_properties: None,
                selected_timeline_track: None,
                seek_pos: None,
                seek_controller: crate::mpv::seek::SeekController::new(
                    crate::mpv::seek::MpvSeekBackend::new(mpv_static),
                ),
                was_playing_before_scrub: false,
                is_scrubbing: false,
                pending_scrub_commit: None,
                last_mouse_activity: std::time::Instant::now(),
                pin_controls: loaded_config.pin_controls,
                show_error: None,
                show_four_d_editor: loaded_config.workspace_session.nle,

                timeline: crate::four_d::models::Timeline::new(),
                engine_handle,
                recording_session: crate::four_d::curve_record::RecordingSession::new(),
                input_capture: crate::four_d::input_capture::InputCaptureState::new(),
                is_recording: false,
                hardware_effect_authoring: crate::app::HardwareEffectAuthoringState::default(),
                recording_keys: std::collections::HashMap::new(),
                dock_state,
                rtt_state: Arc::new(Mutex::new(crate::app::RttState {
                    video_texture: None,
                    video_fbo: None,
                    video_texture_id: None,
                    texture_width: 1920,
                    texture_height: 1080,
                })),
                selected_instance_ids: std::collections::HashSet::new(),
                selected_keyframes: std::collections::HashSet::new(),
                selected_timeline_keyframe: None,
                active_keyframe_drag: None,
                timeline_zoom: 100.0,
                undo_stack: crate::four_d::history::UndoStack::default(),
                effects_search_query: String::new(),
                show_effect_library_editor: loaded_config.workspace_session.dialogs.effects_manager,
                effect_library_selection: loaded_config
                    .workspace_session
                    .dialogs
                    .effects_selection
                    .clone(),
                effect_library_draft: crate::app::ControllerEffectDraft::default(),
                effect_group_draft: None,
                track_muted: std::collections::BTreeSet::new(),
                track_soloed: std::collections::BTreeSet::new(),
                track_locked: std::collections::BTreeSet::new(),
                active_drag: None,
                estop_active: false,
                show_estop_control: loaded_config.show_estop_control,
                confirm_estop_release: loaded_config.confirm_estop_release,
                show_estop_release_dialog: false,
                skip_estop_release_confirmation_draft: false,
                serial_port: loaded_config
                    .hardware_endpoint
                    .clone()
                    .unwrap_or_else(|| crate::four_d::controller::DEFAULT_ENDPOINT.to_string()),
                is_connected: false,
                lasso_origin: None,
                lasso_rect: None,
                current_video_path: None,
                show_remaining_time: loaded_config.show_remaining_time,
                editing_elapsed_time: false,
                elapsed_time_input: String::new(),
                elapsed_edit_focus_requested: false,
                osd_message: None,
                recent_media: loaded_config.recent_media.clone(),
                last_media_target: loaded_config.last_media_target.clone(),
                restore_last_media_on_startup: loaded_config.restore_last_media_on_startup,
                remember_playback_position: loaded_config.remember_playback_position,
                playback_position_history_limit: loaded_config.playback_position_history_limit,
                playback_positions: loaded_config.playback_positions.clone(),
                pending_resume_position: None,
                last_playback_position_checkpoint: std::time::Instant::now(),
                show_open_url_dialog: loaded_config.workspace_session.dialogs.open_location,
                url_input_buffer: String::new(),
                open_url_multiline: loaded_config.open_url_multiline,
                open_url_history_expanded: loaded_config.open_url_history_expanded,
                open_url_recent_click_edits: loaded_config.open_url_recent_click_edits,
                open_url_fetch_remote_info: loaded_config.open_url_fetch_remote_info,
                open_url_fetch_remote_thumbnail: loaded_config.open_url_fetch_remote_thumbnail,
                open_url_use_proxy: loaded_config.open_url_use_proxy,
                open_url_proxy_url: loaded_config.open_url_proxy_url.clone().unwrap_or_default(),
                url_inspector: crate::ui::open_url::UrlInspector::default(),
                is_window_operating: false,
                show_shortcuts_dialog: loaded_config.workspace_session.dialogs.shortcuts,
                show_about_dialog: loaded_config.workspace_session.dialogs.about,
                about_tab: loaded_config.workspace_session.dialogs.about_tab,
                about_icon: None,
                show_preferences_dialog: loaded_config.workspace_session.dialogs.preferences,
                preferences_tab: loaded_config.workspace_session.dialogs.preferences_tab,
                preferences_draft: None,
                show_board_info_dialog: loaded_config.workspace_session.dialogs.board_information,
                board_info_tab: loaded_config
                    .workspace_session
                    .dialogs
                    .board_information_tab,
                board_name_draft: String::new(),
                show_hardware_channels_dialog: loaded_config
                    .workspace_session
                    .dialogs
                    .channel_manager,
                show_workspace_profiles_dialog: loaded_config
                    .workspace_session
                    .dialogs
                    .workspace_profiles,
                workspace_profile_name_draft: String::new(),
                workspace_profile_icon_draft: "window".to_string(),
                workspace_profiles,
                active_workspace_profile: loaded_config.active_workspace_profile.clone(),
                hardware_control_dialog_key: loaded_config
                    .workspace_session
                    .dialogs
                    .hardware_control_key
                    .clone(),
                hardware_channel_detail_active: loaded_config
                    .workspace_session
                    .dialogs
                    .hardware_channel_detail_active,
                hardware_control_name_draft: String::new(),
                hardware_control_group_draft: String::new(),
                hardware_control_icon_draft: String::new(),
                hardware_control_icon_search: String::new(),
                hardware_control_color_draft: String::new(),
                hardware_control_up_color_draft: String::new(),
                hardware_control_down_color_draft: String::new(),
                hardware_control_pwm_percent: 0.0,
                hardware_key_bindings: loaded_config.hardware_key_bindings.clone(),
                hardware_binding_dialog_channel: None,
                hardware_binding_draft: None,
                hardware_binding_capturing: false,
                hardware_hotkey_runtime:
                    crate::hardware_shortcuts::GlobalHardwareShortcutRuntime::default(),
                active_hardware_bindings: std::collections::BTreeSet::new(),
                board_operation: None,
                board_operation_status: String::new(),
                board_settings_draft: None,
                board_settings_dirty: false,
                board_reboot_armed: false,
                front_panel_refresh_attempted: false,
                front_panel_pending_key: None,
                pause_on_hardware_disconnect: loaded_config.pause_on_hardware_disconnect,
                auto_connect_hardware: loaded_config.auto_connect_hardware,
                click_player_to_toggle: loaded_config.click_player_to_toggle,
                show_subseconds: loaded_config.show_subseconds,
                quick_seek_seconds: loaded_config.quick_seek_seconds,
                frame_step_count: loaded_config.frame_step_count,
                wheel_seek_seconds: loaded_config.wheel_seek_seconds,
                osd_position: loaded_config.osd_position,
                osd_timeout_seconds: loaded_config.osd_timeout_seconds,
                paused_drag_action: loaded_config.paused_drag_action,
                playing_drag_action: loaded_config.playing_drag_action,
                fullscreen_video_background: loaded_config.fullscreen_video_background,
                motion_control_mode: loaded_config.motion_control_mode,
                held_motion_action: None,
                compact_hardware_controls: loaded_config.compact_hardware_controls,
                compact_timeline_tracks: loaded_config.compact_timeline_tracks,
                non_user_control_visibility: loaded_config.non_user_control_visibility,
                prefix_relay_identifiers: loaded_config.prefix_relay_identifiers,
                live_pwm_updates: loaded_config.live_pwm_updates,
                hardware_actions_on_press: loaded_config.hardware_actions_on_press,
                single_instance: loaded_config.single_instance,
                windows_mica_backdrop: loaded_config.windows_mica_backdrop,
                windows_dwm_theming: loaded_config.windows_dwm_theming,
                opengl_vsync: loaded_config.opengl_vsync,
                native_dialog_windows: loaded_config.native_dialog_windows,
                native_preferences: None,
                status_bar: loaded_config.status_bar,
                config_fingerprint: crate::config::AppConfig::fingerprint(
                    &crate::config::AppConfig::get_config_path(),
                )
                .ok(),
                config_watcher: None,
                config_reload_due: None,
                auto_reload_config: loaded_config.auto_reload_config,
                preference_preview_original: None,
                config_status: String::new(),
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
                last_taskbar_state: None,
                last_thumbnail_button_state: None,
            };

            if app.auto_connect_hardware {
                let configured_endpoint = app.serial_port.clone();
                let selected_endpoint = crate::four_d::controller::select_autoconnect_endpoint(
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

            if let Some(target) = startup_media_target {
                app.load_media_target(&target);
                if restore_startup_pause {
                    app.pause();
                }
            }
            for command in cli_options.commands {
                app.apply_interop_command(&cc.egui_ctx, command, "Command line");
            }

            Ok(Box::new(app))
        }),
    )
}
