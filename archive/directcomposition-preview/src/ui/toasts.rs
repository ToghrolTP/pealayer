use eframe::egui;

/// Root-viewport overlay: dialogs must not independently replay messages.
pub fn draw(ctx: &egui::Context) {
    let state = crate::messaging::snapshot();
    if state.toasts.is_empty() {
        return;
    }
    let width = (ctx.content_rect().width() - 24.0).clamp(120.0, 360.0);
    egui::Area::new(egui::Id::new("shared_toasts"))
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 80.0])
        .show(ctx, |ui| {
            ui.set_width(width);
            for toast in state.toasts.iter().rev().take(4) {
                let icon = match toast.severity {
                    crate::messaging::Severity::Success => crate::ui::icons::CHECK,
                    crate::messaging::Severity::Warning | crate::messaging::Severity::Error => {
                        crate::ui::icons::WARNING
                    }
                    crate::messaging::Severity::Info => crate::ui::icons::INFO,
                };
                egui::Frame::popup(ui.style())
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_width((width - 24.0).max(96.0));
                        ui.horizontal(|ui| {
                            let role = match toast.severity {
                                crate::messaging::Severity::Success => "green",
                                crate::messaging::Severity::Warning => "amber",
                                crate::messaging::Severity::Error => "red",
                                crate::messaging::Severity::Info => "blue",
                            };
                            let palette = crate::platform::interop::get_live_config().color_palette;
                            ui.label(egui::RichText::new(icon).color(crate::ui::palette::color(
                                palette,
                                ui.visuals().dark_mode,
                                role,
                            )));
                            ui.vertical(|ui| {
                                ui.set_width((width - 86.0).max(48.0));
                                if !toast.title.is_empty() {
                                    ui.label(egui::RichText::new(&toast.title).strong());
                                }
                                egui::ScrollArea::vertical()
                                    .id_salt(&toast.id)
                                    .max_height(140.0)
                                    .show(ui, |ui| {
                                        ui.add(egui::Label::new(&toast.message).wrap());
                                    });
                            });
                            if ui
                                .button(crate::ui::icons::X)
                                .on_hover_text("Dismiss notification")
                                .clicked()
                            {
                                crate::messaging::dismiss(&toast.id);
                                ctx.request_repaint();
                            }
                        });
                    });
                ui.add_space(6.0);
            }
        });
    if let Some(next) = state
        .toasts
        .iter()
        .filter_map(|toast| toast.expires_at_ms)
        .min()
    {
        ctx.request_repaint_after(std::time::Duration::from_millis(
            next.saturating_sub(crate::messaging::now_ms()).max(1),
        ));
    }
}
