use crate::app::PealayerApp;
use eframe::egui;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if ui.input(|i| i.viewport().fullscreen.unwrap_or(false)) {
        return;
    }

    egui::Panel::bottom("status_bar").show_inside(ui, |ui| {
        ui.horizontal(|ui| {
            // FPS Counter
            let dt = ui.input(|i| i.stable_dt);
            let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
            ui.label(format!("FPS: {:.0}", fps));

            ui.separator();

            // Simulated memory info for high density look
            ui.label("Memory: 42.8 MB");

            ui.separator();

            // System Ready status with a green light
            ui.horizontal(|ui| {
                let size = egui::vec2(12.0, 12.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                
                // Pulsing glow effect
                let time = ui.input(|i| i.time);
                let alpha = (120.0 + (time * 3.0).sin() * 50.0) as u8;
                ui.painter().circle_filled(
                    rect.center(),
                    6.0,
                    egui::Color32::from_rgba_unmultiplied(46, 204, 113, alpha),
                );
                ui.painter().circle_filled(
                    rect.center(),
                    4.0,
                    egui::Color32::from_rgb(46, 204, 113),
                );
                
                ui.label("System Ready");
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if app.show_four_d_editor {
                    ui.label("Workspace: NLE Layout");
                } else {
                    ui.label("Workspace: Simple Player");
                }
            });
        });
    });
}
