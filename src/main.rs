pub mod app;
pub mod mpv;
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
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "Pealayer",
        options,
        Box::new(|cc| {
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

            Ok(Box::new(PealayerApp {
                mpv: mpv_static,
                mpv_client,
                render_context: Arc::new(Mutex::new(RenderContextWrapper(render_context))),
                playback_time: 0.0,
                duration: 0.0,
                is_paused: false,
                volume: 100.0,
                is_muted: false,
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
                show_error: None,
                show_four_d_editor: false,
                timeline: crate::four_d::models::Timeline::new(),
                engine_handle: crate::four_d::engine::spawn_engine(),
            }))
        }),
    )
}
