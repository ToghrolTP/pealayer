use eframe::egui;
use crate::messaging::{Toast, Severity};
use crate::platform::interop::InteropCommand;

fn diagnostic(toast: &Toast) -> String {
    format!("{}\n{}\n\nSource: {}\nTime: {}\n\n{}", toast.title, toast.message,
        toast.source, toast.created_at_ms, toast.details.as_deref().unwrap_or(&toast.message))
}

/// Root-viewport overlay. Opening details retains the diagnostic beyond toast expiry.
pub fn draw(ctx: &egui::Context, details_label: &str) -> Option<InteropCommand> {
    let state = crate::messaging::snapshot();
    let detail_id = egui::Id::new("notification_diagnostic");
    let mut detail = ctx.data(|data| data.get_temp::<Toast>(detail_id));
    let mut activation = None;
    let width = (ctx.content_rect().width() - 24.0).clamp(120.0, 360.0);
    if !state.toasts.is_empty() {
        egui::Area::new(egui::Id::new("shared_toasts"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::RIGHT_TOP, [-12.0, 80.0])
            .show(ctx, |ui| {
                ui.set_width(width);
                for toast in state.toasts.iter().rev().take(4) {
                    ui.push_id((&state.instance_id, &toast.id, toast.created_at_ms), |ui| {
                        let rect_id = ui.id().with("rect");
                        let old_rect = ctx.data(|data| data.get_temp::<egui::Rect>(rect_id));
                        let hovered = old_rect.is_some_and(|rect| ui.rect_contains_pointer(rect));
                        let frame = egui::Frame::popup(ui.style()).inner_margin(12.0).show(ui, |ui| {
                            ui.set_width((width - 24.0).max(96.0));
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.set_width((width - 68.0).max(40.0));
                                    let icon = match toast.severity {
                                        Severity::Success => crate::ui::icons::CHECK,
                                        Severity::Error | Severity::Warning => crate::ui::icons::WARNING,
                                        Severity::Info => crate::ui::icons::INFO,
                                    };
                                    let role = match toast.severity { Severity::Success => "green", Severity::Warning => "amber", Severity::Error => "red", Severity::Info => "blue" };
                                    let palette = crate::platform::interop::get_live_config().color_palette;
                                    ui.label(egui::RichText::new(format!("{} {}", icon, toast.title)).strong()
                                        .color(crate::ui::palette::color(palette, ui.visuals().dark_mode, role)));
                                    let clickable = toast.default_action.is_some() || toast.severity == Severity::Error || toast.details.is_some();
                                    let response = egui::ScrollArea::vertical().id_salt("message").max_height(140.0)
                                        .show(ui, |ui| ui.add(egui::Label::new(&toast.message).wrap()
                                            .sense(if clickable { egui::Sense::click() } else { egui::Sense::hover() }))).inner;
                                    if response.clicked() {
                                        if let Some(id) = &toast.default_action {
                                            activation = Some(invoke(toast, id));
                                        } else { detail = Some(toast.clone()); }
                                    }
                                    ui.horizontal_wrapped(|ui| {
                                        for action in &toast.actions {
                                            if ui.button(&action.label).clicked() {
                                                activation = Some(invoke(toast, &action.id));
                                            }
                                        }
                                    });
                                    if toast.severity == Severity::Error || toast.details.is_some() {
                                        // Reserve a fixed action row. Keyboard focus also reveals its controls.
                                        ui.horizontal(|ui| {
                                            for (icon, label, copy) in [(crate::ui::icons::COPY, "Copy error", true), (crate::ui::icons::INFO, details_label, false)] {
                                                ui.scope(|ui| {
                                                    let button_id = ui.next_auto_id();
                                                    let focus = ui.memory(|memory| memory.has_focus(button_id));
                                                    if !hovered && !focus && !response.has_focus() { ui.set_opacity(0.0); }
                                                    let button = ui.button(icon).on_hover_text(label);
                                                    if button.gained_focus() { ctx.request_repaint(); }
                                                    if button.clicked() {
                                                        if copy { ctx.copy_text(diagnostic(toast)); }
                                                        else { detail = Some(toast.clone()); }
                                                    }
                                                });
                                            }
                                        });
                                    }
                                });
                                if crate::ui::dialog::close_button(ui, "Dismiss notification").clicked() {
                                    crate::messaging::dismiss(&toast.id);
                                }
                            });
                        });
                        ctx.data_mut(|data| data.insert_temp(rect_id, frame.response.rect));
                    });
                    ui.add_space(6.0);
                }
            });
    }
    if let Some(toast) = detail.as_ref() {
        let mut close = false;
        let modal = egui::Modal::new(detail_id).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 48.0).clamp(160.0, 600.0));
            ui.horizontal(|ui| {
                ui.heading(details_label);
                close = crate::ui::dialog::close_button(ui, "Close").clicked();
            });
            ui.separator();
            egui::ScrollArea::vertical().max_height(ctx.content_rect().height() * 0.6).show(ui, |ui| {
                ui.add(egui::Label::new(diagnostic(toast)).wrap().selectable(true));
            });
            if ui.button(format!("{} Copy error", crate::ui::icons::COPY)).clicked() { ctx.copy_text(diagnostic(toast)); }
        });
        if close || modal.should_close() { detail = None; }
    }
    ctx.data_mut(|data| {
        if let Some(detail) = detail { data.insert_temp(detail_id, detail); }
        else { data.remove::<Toast>(detail_id); }
    });
    if let Some(next) = state.toasts.iter().filter_map(|toast| toast.expires_at_ms).min() {
        ctx.request_repaint_after(std::time::Duration::from_millis(next.saturating_sub(crate::messaging::now_ms()).max(1)));
    }
    activation
}
fn invoke(toast: &Toast, action: &str) -> InteropCommand {
    InteropCommand::InvokeToastAction { id: toast.id.clone(), created_at_ms: toast.created_at_ms, action_id: action.into() }
}
