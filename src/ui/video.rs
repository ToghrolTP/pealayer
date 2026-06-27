use crate::app::PealayerApp;
use crate::mpv::render::GetProcAddress;
use eframe::egui;
use std::sync::Arc;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let video_size = ui.available_size();
    if video_size.x <= 0.0 || video_size.y <= 0.0 {
        return;
    }

    let (rect, response) = ui.allocate_exact_size(video_size, egui::Sense::click());

    if response.double_clicked() {
        let is_fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
    }

    // 1. Calculate destination rect maintaining 16:9 aspect ratio
    let aspect_ratio = 16.0 / 9.0;
    let rect_w = rect.width();
    let rect_h = rect.height();
    
    let dest_rect = if rect_w / rect_h > aspect_ratio {
        // Height-constrained
        let new_w = rect_h * aspect_ratio;
        let x_offset = (rect_w - new_w) / 2.0;
        egui::Rect::from_min_size(
            egui::pos2(rect.min.x + x_offset, rect.min.y),
            egui::vec2(new_w, rect_h)
        )
    } else {
        // Width-constrained
        let new_h = rect_w / aspect_ratio;
        let y_offset = (rect_h - new_h) / 2.0;
        egui::Rect::from_min_size(
            egui::pos2(rect.min.x, rect.min.y + y_offset),
            egui::vec2(rect_w, new_h)
        )
    };

    // 2. Draw the offscreen texture if registered
    let texture_id_opt = {
        let rtt = app.rtt_state.lock().unwrap();
        rtt.video_texture_id
    };

    if let Some(texture_id) = texture_id_opt {
        ui.painter().image(
            texture_id,
            dest_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    // 3. Schedule PaintCallback to render the current frame from MPV into the FBO
    let render_context = app.render_context.clone();
    let rtt_state = app.rtt_state.clone();

    let callback = egui::PaintCallback {
        rect,
        callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |_info, painter| {
            let gl = painter.gl();
            let rtt = rtt_state.lock().unwrap();

            if let (Some(video_fbo), Ok(rc)) = (rtt.video_fbo, render_context.lock()) {
                unsafe {
                    use eframe::glow::HasContext;

                    // Query original FBO binding
                    let raw_fbo = gl.get_parameter_i32(eframe::glow::FRAMEBUFFER_BINDING) as u32;
                    let target_fbo = std::num::NonZeroU32::new(raw_fbo).map(eframe::glow::NativeFramebuffer);

                    // Bind our offscreen FBO
                    gl.bind_framebuffer(eframe::glow::FRAMEBUFFER, Some(video_fbo));
                    
                    // Render MPV frame at fixed size 1920x1080
                    let fbo_id = video_fbo.0.get() as i32;
                    let _ = rc.0.render::<GetProcAddress>(fbo_id, 1920, 1080, false);

                    // Restore original FBO binding
                    gl.bind_framebuffer(eframe::glow::FRAMEBUFFER, target_fbo);
                }
            }
        })),
    };

    ui.painter().add(callback);
}


