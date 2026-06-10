use crate::app::PealayerApp;
use crate::mpv::render::GetProcAddress;
use eframe::egui;
use std::sync::Arc;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let video_size = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(video_size, egui::Sense::click());

    if response.double_clicked() {
        let is_fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
    }

    let render_context = app.render_context.clone();

    let callback = egui::PaintCallback {
        rect,
        callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |info, painter| {
            if let Ok(rc) = render_context.lock() {
                let fbo = unsafe {
                    use eframe::glow::HasContext;
                    painter
                        .gl()
                        .get_parameter_i32(eframe::glow::FRAMEBUFFER_BINDING)
                };

                let vp = info.viewport_in_pixels();
                let _ = rc.0.render::<GetProcAddress>(
                    fbo,
                    vp.width_px,
                    vp.height_px,
                    true,
                );
            }
        })),
    };

    ui.painter().add(callback);
}
