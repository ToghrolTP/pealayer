use crate::app::PealayerApp;
use eframe::egui;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let mut clear_error = false;
    if let Some(err) = &app.show_error {
        egui::Window::new("Error")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ui.ctx(), |ui| {
                ui.label(err);
                if ui.button("Close").clicked() {
                    clear_error = true;
                }
            });
    }
    if clear_error {
        app.show_error = None;
    }
}
