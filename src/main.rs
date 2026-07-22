#![windows_subsystem = "windows"]

pub mod app;
pub mod config;
pub mod mpv;
pub mod platform;
pub mod server;
pub mod ui;
pub mod four_d;

use app::PealayerApp;
use eframe::egui;
use libmpv2::{
    Mpv,
    render::{OpenGLInitParams, RenderParam, RenderParamApiType},
};
use mpv::render::RenderContextWrapper;
use mpv::render::mpv_get_proc_address;
use std::sync::{Arc, Mutex};

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_transparent(true),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "Pealayer",
        options,
        Box::new(|cc| {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = egui::Color32::from_rgb(33, 33, 33); // #212121
            visuals.window_fill = egui::Color32::from_rgb(26, 26, 26); // #1a1a1a
            cc.egui_ctx.set_visuals(visuals);

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


            let mpv = Mpv::with_initializer(|init| {
                init.set_property("vo", "libmpv")?;

                // Set up Arabic/Farsi Vazirmatn font for subtitles
                let current_dir = std::env::current_dir().unwrap();
                let font_dir = current_dir.join("test-data").join("vazirmatn");
                if let Some(font_dir_str) = font_dir.to_str() {
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

            let egui_ctx2 = cc.egui_ctx.clone();
            mpv_client.set_wakeup_callback(move || {
                egui_ctx2.request_repaint();
            });

            let loaded_config = crate::config::AppConfig::load();
            let _ = mpv_static.set_property("volume", loaded_config.volume);
            let _ = mpv_static.set_property("mute", loaded_config.is_muted);

            let (web_state_tx, web_cmd_rx) = crate::server::spawn_web_server(8080);

            Ok(Box::new(PealayerApp {
                mpv: mpv_static,
                mpv_client,
                render_context: Arc::new(Mutex::new(RenderContextWrapper(render_context))),
                playback_time: 0.0,
                duration: 0.0,
                is_paused: false,
                volume: loaded_config.volume,
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
                last_mouse_activity: std::time::Instant::now(),
                pin_controls: loaded_config.pin_controls,
                show_error: None,
                show_four_d_editor: true,

                timeline: crate::four_d::models::Timeline::new(),
                engine_handle: crate::four_d::engine::spawn_engine(),
                recording_keys: std::collections::HashMap::new(),
                dock_state: crate::ui::layout::create_initial_layout(),
                rtt_state: Arc::new(Mutex::new(crate::app::RttState {
                    video_texture: None,
                    video_fbo: None,
                    video_texture_id: None,
                })),
                selected_instance_ids: std::collections::HashSet::new(),
                relay_overrides: [None; 9],
                preset_library: vec![
                    // Atmospherics
                    crate::app::EffectPreset {
                        category: "Atmospherics".to_string(),
                        effect: crate::four_d::models::Effect::new(
                            "Water Splash".to_string(),
                            "💧".to_string(),
                            1500,
                            crate::four_d::patterns::generate_constant(1, true, 1500),
                        ),
                    },
                    crate::app::EffectPreset {
                        category: "Atmospherics".to_string(),
                        effect: crate::four_d::models::Effect::new(
                            "Mist Spray".to_string(),
                            "🌫".to_string(),
                            3000,
                            crate::four_d::patterns::generate_constant(1, true, 3000),
                        ),
                    },
                    crate::app::EffectPreset {
                        category: "Atmospherics".to_string(),
                        effect: crate::four_d::models::Effect::new(
                            "Wind Blast".to_string(),
                            "💨".to_string(),
                            2000,
                            crate::four_d::patterns::generate_constant(2, true, 2000),
                        ),
                    },
                    crate::app::EffectPreset {
                        category: "Atmospherics".to_string(),
                        effect: crate::four_d::models::Effect::new(
                            "Wind Gale".to_string(),
                            "🌀".to_string(),
                            5000,
                            crate::four_d::patterns::generate_constant(2, true, 5000),
                        ),
                    },
                    // Physical Effects
                    crate::app::EffectPreset {
                        category: "Physical Effects".to_string(),
                        effect: crate::four_d::models::Effect::new(
                            "Seat Rumble".to_string(),
                            "📳".to_string(),
                            1000,
                            crate::four_d::patterns::generate_constant(3, true, 1000),
                        ),
                    },
                    crate::app::EffectPreset {
                        category: "Physical Effects".to_string(),
                        effect: crate::four_d::models::Effect::new(
                            "Seat Shake".to_string(),
                            "🫨".to_string(),
                            2500,
                            crate::four_d::patterns::generate_constant(3, true, 2500),
                        ),
                    },
                    crate::app::EffectPreset {
                        category: "Physical Effects".to_string(),
                        effect: crate::four_d::models::Effect::new(
                            "Smoke Blast".to_string(),
                            "💨".to_string(),
                            1800,
                            crate::four_d::patterns::generate_constant(4, true, 1800),
                        ),
                    },
                    crate::app::EffectPreset {
                        category: "Physical Effects".to_string(),
                        effect: crate::four_d::models::Effect::new(
                            "Fog Screen".to_string(),
                            "🌫".to_string(),
                            4000,
                            crate::four_d::patterns::generate_constant(4, true, 4000),
                        ),
                    },
                    crate::app::EffectPreset {
                        category: "Auxiliary Controls".to_string(),
                        effect: crate::four_d::models::Effect::new(
                            "Aux Trigger A".to_string(),
                            "⚡".to_string(),
                            1500,
                            crate::four_d::patterns::generate_constant(5, true, 1500),
                        ),
                    },
                    crate::app::EffectPreset {
                        category: "Auxiliary Controls".to_string(),
                        effect: crate::four_d::models::Effect::new(
                            "Aux Trigger B".to_string(),
                            "🔌".to_string(),
                            2400,
                            crate::four_d::patterns::generate_constant(6, true, 2400),
                        ),
                    },
                ],
                effects_search_query: String::new(),
                track_muted: [false; 9],
                track_soloed: [false; 9],
                track_locked: [false; 9],
                active_drag: None,
                estop_active: false,
                serial_port: "COM3".to_string(),
                is_connected: false,
                lasso_origin: None,
                lasso_rect: None,
                current_video_path: None,
                show_remaining_time: loaded_config.show_remaining_time,
                osd_message: None,
                recent_media: loaded_config.recent_media,
                show_open_url_dialog: false,
                url_input_buffer: String::new(),
                is_window_operating: false,
                show_shortcuts_dialog: false,
                show_about_dialog: false,
                interop_rx: crate::platform::interop::spawn_interop_server(),
                web_state_tx,
                web_cmd_rx,
            }))
        }),
    )
}
