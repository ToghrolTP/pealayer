use crate::app::PealayerApp;
use eframe::egui;

fn friendly_name(value: &str) -> String {
    value
        .split(['-', '_', '.'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if ui.input(|input| input.viewport().fullscreen.unwrap_or(false)) { return; }

    let panel = egui::Panel::bottom("status_bar").show_inside(ui, |ui| {
        ui.with_layout(crate::ui::i18n::layout(app.rtl, egui::Align::Center), |ui| {
            if app.status_bar.media_rate
                && app.current_video_path.is_some()
                && app.media_fps.is_finite()
                && app.media_fps > 0.0
            {
                ui.label(format!("{:.3} fps", app.media_fps));
                ui.separator();
            }

            if app.status_bar.hardware {
                draw_hardware_status(app, ui);
            }

            if app.estop_active {
                ui.separator();
                ui.colored_label(
                    egui::Color32::from_rgb(231, 76, 60),
                    format!("{} {}", crate::ui::icons::WARNING, app.tr("E-STOP ACTIVE")),
                );
            }

            if app.status_bar.workspace {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(if app.show_four_d_editor {
                        app.tr("Workspace: NLE Layout")
                    } else {
                        app.tr("Workspace: Simple Player")
                    });
                });
            }
        });
    });

    panel.response.context_menu(|ui| {
        let title = app.tr("Status bar");
        let media_rate = app.tr("Media frame rate");
        let hardware = app.tr("Hardware connection");
        let telemetry = app.tr("Hardware telemetry");
        let workspace = app.tr("Workspace mode");
        ui.strong(title);
        ui.separator();
        let mut changed = false;
        changed |= ui.checkbox(&mut app.status_bar.media_rate, media_rate).changed();
        changed |= ui.checkbox(&mut app.status_bar.hardware, hardware).changed();
        changed |= ui.checkbox(&mut app.status_bar.telemetry, telemetry).changed();
        changed |= ui.checkbox(&mut app.status_bar.workspace, workspace).changed();
        if changed { app.save_config(); }
    });
}

fn draw_hardware_status(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let connection_requested = app
        .engine_handle
        .connection_requested
        .load(std::sync::atomic::Ordering::Relaxed);
    let capabilities = app.advertised_hardware();
    let board_ready = capabilities.as_ref().is_some_and(|value| value.board_connected);
    let coordinator_endpoint = crate::four_d::controller::is_controller_endpoint(&app.serial_port);
    let selected_transport = app.engine_handle.active_transport.try_lock().ok().and_then(|value| value.clone());
    let dot_color = if app.is_connected && (!coordinator_endpoint || board_ready) {
        egui::Color32::from_rgb(46, 204, 113)
    } else if connection_requested || app.is_connected {
        egui::Color32::from_rgb(241, 196, 15)
    } else {
        egui::Color32::from_rgb(231, 76, 60)
    };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, dot_color);

    let label_text = if app.is_connected && coordinator_endpoint && board_ready {
        let board_name = capabilities
            .as_ref()
            .and_then(|value| value.board_profile.as_ref())
            .map(|profile| friendly_name(&profile.key))
            .filter(|name| !name.is_empty())
            .or_else(|| capabilities.as_ref().map(|value| app.display_text(&value.board_name)))
            .unwrap_or_else(|| app.tr("Connected board"));
        selected_transport
            .as_deref()
            .map(|transport| format!("{board_name} via {transport}"))
            .unwrap_or(board_name)
    } else if app.is_connected && coordinator_endpoint {
        app.tr("PCController connected; board unavailable")
    } else if app.is_connected {
        format!("{} direct diagnostic", app.serial_port)
    } else if connection_requested {
        format!("{} {}", crate::ui::icons::PLUG, app.tr("Connecting…"))
    } else {
        app.tr("Hardware: Disconnected")
    };
    let connection_label = ui.label(label_text);
    if let Some(notice) = &app.connection_notice { connection_label.on_hover_text(notice); }

    let Some(capabilities) = capabilities.as_ref().filter(|value| value.board_connected) else { return; };
    if let Some(status_led) = &capabilities.status_led {
        let brightness = f32::from(status_led.brightness) / 255.0;
        let color = egui::Color32::from_rgb(
            (f32::from(status_led.red) * brightness).round() as u8,
            (f32::from(status_led.green) * brightness).round() as u8,
            (f32::from(status_led.blue) * brightness).round() as u8,
        );
        let (led_rect, led_response) = ui.allocate_exact_size(egui::vec2(17.0, 17.0), egui::Sense::click());
        ui.painter().circle_filled(led_rect.center(), 6.0, color);
        ui.painter().circle_stroke(
            led_rect.center(),
            6.0,
            egui::Stroke::new(1.25, ui.visuals().widgets.noninteractive.fg_stroke.color),
        );
        if led_response.on_hover_text(format!(
            "{} RGB({}, {}, {}) · {} {} · {} {}",
            app.tr("Hardware status LED"), status_led.red, status_led.green, status_led.blue,
            app.tr("brightness"), status_led.brightness, app.tr("effect"), status_led.effect,
        )).clicked() {
            app.preferences_tab = 2;
            app.show_preferences_dialog = true;
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }

    if app.status_bar.telemetry {
        let telemetry = &capabilities.telemetry;
        if let Some(bus_mv) = telemetry.bus_mv { ui.label(format!("{:.2} V", f64::from(bus_mv) / 1000.0)); }
        if let Some(current_ma) = telemetry.current_ma { ui.label(format!("{current_ma} mA")); }
        if let Some(temperature) = telemetry.led_temperature_centi_c { ui.label(format!("{:.1} °C", f64::from(temperature) / 100.0)); }
    }
    if let Some(warning) = capabilities.warnings.first() {
        let response = ui.colored_label(ui.visuals().warn_fg_color, format!("{} {}", crate::ui::icons::WARNING, warning.code));
        if response.on_hover_text(&warning.message).clicked() {
            app.preferences_tab = 2;
            app.show_preferences_dialog = true;
        }
    }
}
