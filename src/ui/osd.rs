use crate::app::PealayerApp;
use eframe::egui;

/// Total time (seconds) a message stays fully visible before fading.
const HOLD_SECS: f32 = 1.5;
/// Time (seconds) the fade-out animation takes.
const FADE_SECS: f32 = 1.0;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let should_clear = if let Some((msg, started)) = &app.osd_message {
        let elapsed = started.elapsed().as_secs_f32();
        let total = HOLD_SECS + FADE_SECS;

        if elapsed < total {
            let alpha = if elapsed < HOLD_SECS {
                1.0_f32
            } else {
                1.0 - (elapsed - HOLD_SECS) / FADE_SECS
            };

            let alpha_u8 = (alpha * 230.0) as u8;
            let rect = ui.max_rect();
            // Position near the top-center of the video area.
            let pos = rect.center_top() + egui::vec2(0.0, 28.0);
            let font = egui::FontId::proportional(22.0);
            let painter = ui.painter();

            // Drop shadow for readability over any content.
            painter.text(
                pos + egui::vec2(1.0, 1.0),
                egui::Align2::CENTER_TOP,
                msg,
                font.clone(),
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, alpha_u8),
            );
            // Main text.
            painter.text(
                pos,
                egui::Align2::CENTER_TOP,
                msg,
                font,
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha_u8),
            );

            ui.ctx().request_repaint();
            false
        } else {
            true
        }
    } else {
        false
    };

    if should_clear {
        app.osd_message = None;
    }
}
