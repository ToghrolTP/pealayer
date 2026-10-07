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
    if ui.input(|input| input.viewport().fullscreen.unwrap_or(false)) {
        return;
    }

    let panel = egui::Panel::bottom("status_bar").show_inside(ui, |ui| {
        ui.with_layout(
            crate::ui::i18n::layout(app.rtl, egui::Align::Center),
            |ui| {
                if app.status_bar.media_rate
                    && app.current_video_path.is_some()
                    && app.media_fps.is_finite()
                    && app.media_fps > 0.0
                {
                    let response = ui.label(format!("{:.2} fps", app.media_fps));
                    if hide_item_menu(app, response, app.tr("Media frame rate")) {
                        app.status_bar.media_rate = false;
                        app.save_config();
                    }
                    ui.separator();
                }

                if app.status_bar.hardware {
                    draw_hardware_status(app, ui);
                }
                if let Ok(plan)=app.engine_handle.prepared_timeline.try_lock() && plan.has_items() {
                    ui.separator();
                    let state=if plan.error.is_some(){"Hardware timing fault"}
                        else if plan.deferred_reason.is_some(){"Hardware waiting"}
                        else if plan.acknowledged_revision!=plan.revision{"Preparing hardware"}
                        else {"Hardware timeline"};
                    let color=if plan.error.is_some(){ui.visuals().error_fg_color}else{ui.visuals().text_color()};
                    ui.colored_label(color,state).on_hover_text(plan.error.clone().or_else(||plan.deferred_reason.clone())
                        .unwrap_or_else(||format!("Revision {} · {} acknowledged commands · maximum ACK lateness {} ms",plan.revision,
                            plan.feedback["acknowledged"],plan.feedback["max_ack_lateness_ms"])));
                }

                if let Some(message) = current_status_message(app) {
                    ui.separator();
                    ui.add(
                        egui::Label::new(egui::RichText::new(&message).small())
                            .truncate()
                            .selectable(false),
                    )
                    .on_hover_text(message);
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
                        let response = ui.label(format!(
                            "{}: {}",
                            app.tr("Workspace"),
                            app.active_workspace_name()
                        ));
                        if hide_item_menu(app, response, app.tr("Workspace mode")) {
                            app.status_bar.workspace = false;
                            app.save_config();
                        }
                    });
                }
            },
        );
    });

    panel
        .response
        .interact(egui::Sense::click())
        .context_menu(|ui| {
            let title = app.tr("Status bar");
            let media_rate = app.tr("Media frame rate");
            let hardware = app.tr("Hardware connection");
            let telemetry = app.tr("Hardware telemetry");
            let status_rgb = app.tr("Physical status RGB");
            let warnings = app.tr("Hardware warnings");
            let workspace = app.tr("Workspace mode");
            ui.strong(title);
            ui.separator();
            let mut changed = false;
            changed |= ui
                .checkbox(&mut app.status_bar.media_rate, media_rate)
                .changed();
            changed |= ui
                .checkbox(&mut app.status_bar.hardware, hardware)
                .changed();
            changed |= ui
                .checkbox(&mut app.status_bar.telemetry, telemetry)
                .changed();
            changed |= ui
                .checkbox(&mut app.status_bar.status_rgb, status_rgb)
                .changed();
            changed |= ui
                .checkbox(&mut app.status_bar.warnings, warnings)
                .changed();
            changed |= ui
                .checkbox(&mut app.status_bar.workspace, workspace)
                .changed();
            if changed {
                app.save_config();
            }
        });
}

fn current_status_message(app: &PealayerApp) -> Option<String> {
    let update = crate::update::manager().status();
    if update.active() {
        return Some(match update.progress_percent() {
            Some(percent) => format!("{} · {percent:.0}%", update.message),
            None => update.message,
        });
    }
    let (message, timestamp) = app.osd_message.as_ref()?;
    let lifetime = std::time::Duration::from_secs_f32(
        app.osd_display_options
            .as_ref()
            .and_then(|options| options.timeout_seconds)
            .unwrap_or(app.osd_timeout_seconds)
            .max(0.25),
    );
    (timestamp.elapsed() <= lifetime).then(|| message.clone())
}

fn hide_item_menu(app: &PealayerApp, response: egui::Response, label: String) -> bool {
    let mut hide = false;
    response.context_menu(|ui| {
        ui.label(egui::RichText::new(&label).strong());
        ui.separator();
        if ui
            .button(format!(
                "{} {} {label}",
                crate::ui::icons::EYE_SLASH,
                app.tr("Hide")
            ))
            .clicked()
        {
            hide = true;
            ui.close();
        }
    });
    hide
}

fn draw_hardware_status(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let connection_requested = app
        .engine_handle
        .connection_requested
        .load(std::sync::atomic::Ordering::Relaxed);
    let capabilities = app.advertised_hardware();
    let board_ready = capabilities
        .as_ref()
        .is_some_and(|value| value.board_connected);
    let coordinator_endpoint = crate::four_d::controller::is_controller_endpoint(&app.serial_port);
    let selected_transport = app
        .engine_handle
        .active_transport
        .try_lock()
        .ok()
        .and_then(|value| value.clone());
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
            .map(|value| app.display_text(&value.board_name))
            .filter(|name| !name.is_empty())
            .or_else(|| {
                capabilities
                    .as_ref()
                    .and_then(|value| value.board_profile.as_ref())
                    .map(|profile| friendly_name(&profile.key))
            })
            .unwrap_or_else(|| app.tr("Connected board"));
        selected_transport
            .as_deref()
            .map(|transport| format!("{board_name} via {transport}"))
            .unwrap_or(board_name)
    } else if app.is_connected && coordinator_endpoint {
        app.tr("PCController connected; board not connected")
    } else if app.is_connected {
        format!("{} direct diagnostic", app.serial_port)
    } else if connection_requested {
        format!("{} {}", crate::ui::icons::PLUG, app.tr("Connecting…"))
    } else {
        app.tr("Hardware: Disconnected")
    };
    let mut connection_label = ui.label(label_text);
    if app.is_connected && coordinator_endpoint && !board_ready {
        connection_label =
            connection_label.on_hover_text(app.hardware_unavailable_detail(capabilities.as_ref()));
    } else if let Some(notice) = &app.connection_notice {
        connection_label = connection_label.on_hover_text(notice);
    }
    if hide_item_menu(app, connection_label, app.tr("Hardware connection")) {
        app.status_bar.hardware = false;
        app.save_config();
        return;
    }

    let Some(capabilities) = capabilities.as_ref().filter(|value| value.board_connected) else {
        return;
    };
    if app.status_bar.status_rgb
        && let Some(status_led) = &capabilities.status_led
    {
        let brightness = f32::from(status_led.brightness) / 255.0;
        // PCController publishes the already-composited physical RGB result.
        // Do not multiply it by brightness a second time: the Web UI mirrors
        // these raw channels directly and Pealayer must show the same hue.
        let color = status_led_display_color(status_led);
        let (led_rect, led_response) =
            ui.allocate_exact_size(egui::vec2(17.0, 17.0), egui::Sense::click());
        ui.painter().circle_filled(
            led_rect.center(),
            8.0,
            color.gamma_multiply((0.12 + brightness * 0.20).clamp(0.12, 0.32)),
        );
        ui.painter().circle_filled(led_rect.center(), 6.0, color);
        ui.painter().circle_stroke(
            led_rect.center(),
            6.0,
            egui::Stroke::new(
                1.25_f32,
                ui.visuals().widgets.noninteractive.fg_stroke.color,
            ),
        );
        let led_response = led_response.on_hover_text(format!(
            "{} RGB({}, {}, {}) · {} {} · {} {}",
            app.tr("Hardware status LED"),
            status_led.red,
            status_led.green,
            status_led.blue,
            app.tr("brightness"),
            status_led.brightness,
            app.tr("effect"),
            status_led.effect,
        ));
        if led_response.clicked() {
            app.preferences_tab = 2;
            app.show_preferences_dialog = true;
        }
        if hide_item_menu(app, led_response, app.tr("Physical status RGB")) {
            app.status_bar.status_rgb = false;
            app.save_config();
        }
    }

    if app.status_bar.telemetry {
        let telemetry = &capabilities.telemetry;
        let mut telemetry_response = None;
        if let Some(bus_mv) = telemetry.bus_mv {
            telemetry_response = Some(ui.label(format!("{:.2} V", f64::from(bus_mv) / 1000.0)));
        }
        if let Some(current_ma) = telemetry.current_ma {
            telemetry_response = Some(ui.label(format!("{current_ma} mA")));
        }
        if let Some(temperature) = telemetry.led_temperature_centi_c {
            telemetry_response =
                Some(ui.label(format!("{:.1} °C", f64::from(temperature) / 100.0)));
        }
        if let Some(response) = telemetry_response
            && hide_item_menu(app, response, app.tr("Hardware telemetry"))
        {
            app.status_bar.telemetry = false;
            app.save_config();
        }
    }
    if app.status_bar.warnings
        && let Some(warning) = capabilities.warnings.first()
    {
        let response = ui
            .colored_label(
                ui.visuals().warn_fg_color,
                format!("{} {}", crate::ui::icons::WARNING, warning.code),
            )
            .on_hover_text(&warning.message);
        if response.clicked() {
            app.preferences_tab = 2;
            app.show_preferences_dialog = true;
        }
        if hide_item_menu(app, response, app.tr("Hardware warnings")) {
            app.status_bar.warnings = false;
            app.save_config();
        }
    }
}

fn status_led_display_color(
    status_led: &crate::four_d::controller::HardwareStatusLed,
) -> egui::Color32 {
    egui::Color32::from_rgb(status_led.red, status_led.green, status_led.blue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_status_led_uses_authoritative_rgb_without_double_dimming() {
        let led = crate::four_d::controller::HardwareStatusLed {
            red: 0,
            green: 255,
            blue: 0,
            brightness: 32,
            effect: 0,
            condition: 0,
        };
        assert_eq!(
            status_led_display_color(&led),
            egui::Color32::from_rgb(0, 255, 0)
        );
    }

    #[test]
    fn status_bar_message_uses_the_same_lifetime_as_osd() {
        let mut app = PealayerApp::default();
        app.osd_timeout_seconds = 4.0;
        app.set_osd("Remote command accepted".to_string());
        assert_eq!(
            current_status_message(&app).as_deref(),
            Some("Remote command accepted")
        );
    }
}
