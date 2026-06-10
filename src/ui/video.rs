use crate::app::PealayerApp;
use crate::mpv::render::GetProcAddress;
use eframe::egui;
use std::sync::Arc;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let video_size = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(video_size, egui::Sense::click());

    // Double-click: open a file if nothing is loaded; otherwise toggle fullscreen.
    if response.double_clicked() {
        if !app.has_video {
            open_file_dialog(app);
        } else {
            let is_fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
        }
    }

    // Right-click context menu.
    response.context_menu(|ui| {
        if ui.button("Open File…").clicked() {
            ui.close();
            open_file_dialog(app);
        }
        if ui.button("Open URL…").clicked() {
            app.show_url_dialog = true;
            ui.close();
        }
        if app.has_video {
            ui.separator();
            let pause_label = if app.is_paused { "▶ Resume" } else { "⏸ Pause" };
            if ui.button(pause_label).clicked() {
                let _ = app.mpv.command("cycle", &["pause"]);
                ui.close();
            }
            if ui.button("⏹ Close Video").clicked() {
                let _ = app.mpv.command("stop", &[]);
                ui.close();
            }
        }
    });

    // Drag-and-drop: highlight drop zone while files hover, load on drop.
    let is_hovering = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
    if is_hovering {
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(0, 120, 220, 60),
        );
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 120, 220)),
            egui::StrokeKind::Inside,
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Drop to open",
            egui::FontId::proportional(24.0),
            egui::Color32::WHITE,
        );
    }

    let dropped: Vec<_> = ui.ctx().input(|i| i.raw.dropped_files.clone());
    for file in dropped {
        if let Some(path) = &file.path {
            if let Some(path_str) = path.to_str() {
                let _ = app.mpv.command("loadfile", &[path_str, "replace"]);
            } else {
                app.show_error = Some("Cannot open file: path contains non-UTF-8 characters.".to_string());
            }
        }
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

                let _ = rc.0.render::<GetProcAddress>(
                    fbo,
                    info.viewport.width() as i32,
                    info.viewport.height() as i32,
                    true,
                );
            }
        })),
    };

    ui.painter().add(callback);
}

fn open_file_dialog(app: &mut PealayerApp) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter(
            "Video Files",
            &["mp4", "mkv", "avi", "webm", "mov", "flv", "wmv", "m4v", "ts", "m2ts"],
        )
        .pick_file()
    {
        if let Some(path_str) = path.to_str() {
            let _ = app.mpv.command("loadfile", &[path_str, "replace"]);
        } else {
            app.show_error = Some("Cannot open file: path contains non-UTF-8 characters.".to_string());
        }
    }
}
