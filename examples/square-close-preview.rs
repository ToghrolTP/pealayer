//! Isolated visual fixture: production chrome, no player/board/session or saves.
use eframe::egui;

struct Preview;

impl eframe::App for Preview {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("Isolated UI fixture — no preferences or hardware are changed.");
        });
        let mut resolved = false;
        let response = pealayer::ui::dialog::confirmation_modal(&ctx,
            egui::Id::new("square-close-preview"), "Unsaved preferences",
            pealayer::ui::icons::WARNING, "Save your changes before closing Preferences?", "Close", |ui| {
                for (label, icon, accent, outline) in [
                    ("Save", pealayer::ui::icons::FLOPPY_DISK, elegance::Accent::Blue, false),
                    ("Discard", pealayer::ui::icons::TRASH, elegance::Accent::Red, false),
                    ("Cancel", pealayer::ui::icons::X, elegance::Accent::Blue, true),
                ] {
                    let button = elegance::Button::new(format!("{icon}  {label}")).accent(accent)
                        .min_width(pealayer::ui::dialog::ACTION_BUTTON_WIDTH);
                    resolved |= ui.add(if outline { button.outline() } else { button }).clicked();
                }
                resolved
            });
        if resolved || response.dismissed { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
    }

    fn persist_egui_memory(&self) -> bool { false }
}

fn main() -> eframe::Result {
    eframe::run_native("Pealayer square close fixture", eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default().with_inner_size([580.0, 280.0]),
        persist_window: false,
        ..Default::default()
    }, Box::new(|cc| {
        pealayer::ui::i18n::configure_ui_fonts(&cc.egui_ctx, false);
        pealayer::ui::configure_native_appearance(&cc.egui_ctx, &pealayer::config::AppConfig {
            theme: pealayer::config::AppTheme::Dark,
            color_palette: pealayer::config::ColorPalette::Studio,
            accent_color: pealayer::config::AccentColor::Custom,
            custom_accent_color: Some("#6863C8".into()),
            ..Default::default()
        });
        pealayer::ui::configure_main_window_style(&cc.egui_ctx);
        Ok(Box::new(Preview))
    }))
}
