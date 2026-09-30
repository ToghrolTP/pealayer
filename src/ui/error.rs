use crate::app::PealayerApp;
use eframe::egui;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let mut clear_error = false;
    if let Some(err) = app.show_error.clone() {
        let heading = app.tr(error_heading(&err));
        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            clear_error = true;
        }
        let bounds = ui.ctx().content_rect().shrink(20.0);
        let max_width = bounds.width().min(520.0);
        egui::Window::new(format!("{} {heading}", crate::ui::icons::WARNING))
            .id(egui::Id::new("error_dialog_bounded_v2"))
            .collapsible(false)
            .resizable(true)
            .movable(true)
            .default_size([max_width.min(480.0), bounds.height().min(300.0)])
            .min_size([max_width.min(340.0), bounds.height().min(220.0)])
            .max_size([max_width, bounds.height().min(420.0)])
            .constrain_to(bounds)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ui.ctx(), |ui| {
                ui.with_layout(crate::ui::i18n::vertical_layout(app.rtl), |ui| {
                    ui.with_layout(crate::ui::i18n::layout(app.rtl, egui::Align::Min), |ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
                        let center = icon_rect.center();
                        ui.painter().circle_filled(
                            center,
                            18.0,
                            egui::Color32::from_rgb(190, 52, 48),
                        );
                        ui.painter().text(
                            center,
                            egui::Align2::CENTER_CENTER,
                            crate::ui::icons::WARNING,
                            egui::FontId::proportional(20.0),
                            egui::Color32::WHITE,
                        );
                        ui.vertical(|ui| {
                            ui.heading(heading);
                            ui.label(
                                app.tr("Pealayer could not complete the requested operation."),
                            );
                        });
                    });
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(app.tr("Technical details")).strong());
                    let mut detail = err.clone();
                    ui.add(
                        egui::TextEdit::multiline(&mut detail)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace),
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(format!(
                                "{} {}",
                                crate::ui::icons::COPY,
                                app.tr("Copy details")
                            ))
                            .clicked()
                        {
                            ui.ctx().copy_text(err.clone());
                        }
                        if ui
                            .button(format!("{} {}", crate::ui::icons::X, app.tr("Close")))
                            .clicked()
                            || ui.input(|input| input.key_pressed(egui::Key::Enter))
                        {
                            clear_error = true;
                        }
                    });
                });
            });
    }
    if clear_error {
        app.show_error = None;
    }
}

fn error_heading(message: &str) -> &'static str {
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("connect") || normalized.contains("timed out") {
        "Connection problem"
    } else if normalized.contains("media") || normalized.contains("play") {
        "Playback problem"
    } else {
        "Operation failed"
    }
}

#[cfg(test)]
mod tests {
    use super::error_heading;

    #[test]
    fn selects_contextual_error_heading() {
        assert_eq!(
            error_heading("connect to PCController at 127.0.0.1:8787: connection timed out"),
            "Connection problem"
        );
        assert_eq!(
            error_heading("Failed to play the selected media"),
            "Playback problem"
        );
        assert_eq!(error_heading("permission denied"), "Operation failed");
    }
}
