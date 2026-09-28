use crate::app::PealayerApp;
use eframe::egui;

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if ui.input(|i| i.viewport().fullscreen.unwrap_or(false)) {
        return;
    }

    egui::Panel::bottom("status_bar").show_inside(ui, |ui| {
        ui.with_layout(crate::ui::i18n::layout(app.rtl, egui::Align::Center), |ui| {
            // FPS Counter
            let dt = ui.input(|i| i.stable_dt);
            let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
            ui.label(format!("FPS: {:.0}", fps));

            ui.separator();

            // Hardware Connection Status
            ui.horizontal(|ui| {
                let connection_requested = app
                    .engine_handle
                    .connection_requested
                    .load(std::sync::atomic::Ordering::Relaxed);
                let capabilities = app.advertised_hardware();
                let board_ready = capabilities
                    .as_ref()
                    .is_some_and(|capabilities| capabilities.board_connected);
                let coordinator_endpoint =
                    crate::four_d::controller::is_controller_endpoint(&app.serial_port);
                let selected_transport = app
                    .engine_handle
                    .active_transport
                    .try_lock()
                    .ok()
                    .and_then(|transport| transport.clone());
                let transport_suffix = selected_transport
                    .as_deref()
                    .map(|transport| format!(" via {transport}"))
                    .unwrap_or_default();
                let dot_color = if app.is_connected && (!coordinator_endpoint || board_ready) {
                    egui::Color32::from_rgb(46, 204, 113) // Green
                } else if connection_requested || app.is_connected {
                    egui::Color32::from_rgb(241, 196, 15) // Amber
                } else {
                    egui::Color32::from_rgb(231, 76, 60) // Red
                };

                let size = egui::vec2(12.0, 12.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 4.0, dot_color);

                let label_text = if app.is_connected {
                    if coordinator_endpoint && board_ready {
                        let board_name = capabilities
                            .as_ref()
                            .map(|capabilities| capabilities.board_name.as_str())
                            .filter(|name| !name.is_empty())
                            .unwrap_or("board");
                        format!(
                            "Hardware: PCController + {board_name} connected{transport_suffix}"
                        )
                    } else if coordinator_endpoint {
                        if app.language.is_rtl() {
                            format!(
                                "سخت‌افزار: PCController متصل است؛ برد در دسترس نیست{transport_suffix}"
                            )
                        } else {
                            format!(
                                "Hardware: PCController connected; board unavailable{transport_suffix}"
                            )
                        }
                    } else {
                        format!("Hardware: {} direct diagnostic", app.serial_port)
                    }
                } else if connection_requested {
                    format!("Hardware: Connecting to {}…", app.serial_port)
                } else {
                    if app.language.is_rtl() {
                        "سخت‌افزار: قطع است".to_string()
                    } else {
                        "Hardware: Disconnected".to_string()
                    }
                };
                ui.label(label_text);
            });

            if app.estop_active {
                ui.separator();
                ui.colored_label(
                    egui::Color32::from_rgb(231, 76, 60),
                    app.tr("E-STOP ACTIVE"),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if app.show_four_d_editor {
                    ui.label(app.tr("Workspace: NLE Layout"));
                } else {
                    ui.label(app.tr("Workspace: Simple Player"));
                }
            });
        });
    });
}
