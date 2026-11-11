use eframe::egui;

use crate::app::PealayerApp;
use crate::four_d::controller::HardwareCapabilities;
use crate::four_d::seat_kinematics::CinemaSeatKinematics;

#[path = "digital_twin_seat.rs"]
mod seat;

pub(super) fn is_virtual_board(capabilities: &HardwareCapabilities) -> bool {
    capabilities.port.product == "PCController Virtual Board"
}

fn draw_front_panel(app: &mut PealayerApp, ui: &mut egui::Ui, caps: &HardwareCapabilities) {
    let panel = caps.front_panel.as_ref();
    let pending = app.board_operation.as_deref() == Some("board-front-panel-refresh");
    if panel.is_none_or(|panel| panel.schema == 0)
        && !app.front_panel_refresh_attempted
        && app.board_operation.is_none()
        && let Err(error) = app.refresh_front_panel()
    {
        app.board_operation_status = error;
    }

    ui.horizontal_wrapped(|ui| {
        ui.strong(app.tr("Front panel"));
        if ui
            .add_enabled(
                app.board_operation.is_none(),
                egui::Button::new(crate::ui::icons::ARROW_CLOCKWISE),
            )
            .on_hover_text(app.tr("Read front panel"))
            .clicked()
            && let Err(error) = app.refresh_front_panel()
        {
            app.board_operation_status = error;
        }
    });
    if let Some(panel) = panel.filter(|p| p.raw_segments.len() == 4) {
        super::board_info::seven_segment_preview(
            ui,
            &panel.raw_segments,
            panel.brightness,
            panel.segments_active,
        );
    } else if pending || app.board_operation.as_deref() == Some("board-front-panel-refresh") {
        ui.weak(app.tr("Reading the front panel…"));
    } else {
        ui.weak(app.tr("No display frame received"));
    }
    if let Some(panel) = panel.filter(|p| p.lcd_available) {
        let frame = egui::Frame::new()
            .fill(egui::Color32::from_rgb(10, 24, 12))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(22, 60, 28)))
            .corner_radius(5.0)
            .inner_margin(egui::Margin::symmetric(8, 5));
        let width = (ui.available_width() - frame.total_margin().sum().x).max(1.0);
        frame.show(ui, |ui| {
            ui.set_width(width);
            for line in [&panel.lcd_line_1, &panel.lcd_line_2] {
                let text: String = line.chars().take(16).collect();
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(text)
                            .monospace()
                            .color(egui::Color32::from_rgb(86, 218, 120)),
                    )
                    .truncate(),
                );
            }
        });
    }
    ui.horizontal_wrapped(|ui| {
        if let Some(led) = &caps.status_led {
            let color = egui::Color32::from_rgb(led.red, led.green, led.blue)
                .gamma_multiply(f32::from(led.brightness) / 255.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 18.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 4.5, color);
        }
        if let Some(panel) = panel.filter(|p| p.schema > 0) {
            ui.weak(format!("{} {}", app.tr("Page"), panel.menu_page));
        }
        if caps.buzzer.is_playing() {
            let name = caps.buzzer.melody_name.trim();
            let label = if name.is_empty() {
                app.tr("Buzzer active")
            } else {
                name.to_owned()
            };
            ui.label(format!("{} {label}", crate::ui::icons::SPEAKER_HIGH));
        }
    });
    if !app.board_operation_status.is_empty()
        && app.board_operation.is_none()
        && app.front_panel_refresh_attempted
        && panel.is_none()
    {
        ui.add(egui::Label::new(egui::RichText::new(&app.board_operation_status).weak()).wrap());
    }
    if caps.supports_buzzer_controls()
        && crate::four_d::controller::is_demo_endpoint(&app.serial_port)
        && !crate::peer::active()
    {
        let label = app.tr("Virtual buzzer audio");
        if ui.checkbox(&mut app.virtual_buzzer_audio, label).changed() {
            app.save_config();
        }
    }
}

fn key_button(app: &mut PealayerApp, ui: &mut egui::Ui, caps: &HardwareCapabilities, key: u8) {
    use std::time::{Duration, Instant};
    let label = format!("K{key}");
    let pressed = caps
        .front_panel
        .as_ref()
        .is_some_and(|panel| panel.pressed_keys & (1 << (key - 1)) != 0);
    let response = ui
        .add_enabled(
            caps.supports_virtual_panel_keys,
            egui::Button::new(&label).selected(pressed),
        )
        .on_hover_text(format!("{} {key}", app.tr("Front-panel key")));
    let id = response.id.with((
        "key-lease",
        caps.host_instance_id.as_str(),
        caps.connection_generation,
    ));
    let renewal = ui.data(|data| data.get_temp::<Instant>(id));
    let held = response.is_pointer_button_down_on()
        && ui.input(|input| input.focused && input.pointer.primary_down());
    let interval = Duration::from_millis(250);
    let command = if held {
        ui.ctx().request_repaint_after(interval);
        if renewal.is_none_or(|at| at.elapsed() >= interval) {
            ui.data_mut(|data| data.insert_temp(id, Instant::now()));
            Some(700)
        } else {
            None
        }
    } else if renewal.is_some() {
        ui.data_mut(|data| data.remove::<Instant>(id));
        Some(0)
    } else if response.clicked() {
        Some(130)
    } else {
        None
    };
    if let Some(hold_ms) = command
        && let Err(error) = app.virtual_front_panel_key(caps, key, hold_ms)
    {
        app.set_osd(error);
    }
}

/// Draws interactive simulation controls: door toggle, keys 1..4, RF trigger.
fn draw_simulation_controls(app: &mut PealayerApp, ui: &mut egui::Ui, caps: &HardwareCapabilities) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(app.tr("Board inputs"))
                .size(11.0)
                .strong()
                .color(ui.visuals().weak_text_color()),
        );
        ui.add_space(4.0);

        let door_open = caps.telemetry.door_open.unwrap_or(false);
        let owns_inputs =
            crate::four_d::controller::is_demo_endpoint(&app.serial_port) && !crate::peer::active();

        ui.horizontal_wrapped(|ui| {
            // Door switch button
            let (door_icon, door_label, door_color) = if door_open {
                (
                    crate::ui::icons::WARNING,
                    "Door: OPEN (Interlock Active)",
                    egui::Color32::from_rgb(239, 68, 68),
                )
            } else {
                (
                    crate::ui::icons::DOOR,
                    "Door: CLOSED (Normal)",
                    egui::Color32::from_rgb(52, 211, 153),
                )
            };

            let door_btn = egui::Button::new(
                egui::RichText::new(format!("{door_icon} {door_label}"))
                    .size(11.5)
                    .strong(),
            )
            .fill(door_color.gamma_multiply(0.2))
            .stroke(egui::Stroke::new(1.0, door_color));

            if ui
                .add_enabled(owns_inputs && caps.telemetry.door_open.is_some(), door_btn)
                .on_hover_text(app.tr("Toggle virtual enclosure door switch"))
                .clicked()
                && let Err(error) = app.send_virtual_board_command(if door_open {
                    "door closed"
                } else {
                    "door open"
                })
            {
                app.set_osd(error);
            }

            if caps.supports_virtual_panel_keys {
                for key in 1..=4 {
                    key_button(app, ui, caps, key);
                }
            }

            // RF Remote Injection
            let rf_btn = egui::Button::new(
                egui::RichText::new(format!(
                    "{} {}",
                    crate::ui::icons::RADIO,
                    app.tr("Inject RF Cue")
                ))
                .size(11.0),
            );
            if ui
                .add_enabled(owns_inputs, rf_btn)
                .on_hover_text(app.tr("Simulate 24-bit RF remote key packet"))
                .clicked()
                && let Err(error) = app.send_virtual_board_command("rf 0x4D2A 24 1 350")
            {
                app.set_osd(error);
            }
        });
    });
}

/// Draws the complete Digital Twin Card inside `HardwareMonitor` tab.
pub(super) fn draw_digital_twin_card(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    caps: &HardwareCapabilities,
) {
    let frame = egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(11, 10))
        .stroke(egui::Stroke::new(
            1.5,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(10.0);

    let width = (ui.available_width() - frame.total_margin().sum().x).max(1.0);
    frame.show(ui, |ui| {
        ui.set_width(width);
        let title = app.tr("Virtual Board Digital Twin");
        egui::containers::Sides::new().shrink_left().show(
            ui,
            |ui| {
                ui.add(egui::Label::new(egui::RichText::new(title).strong()).truncate());
            },
            |ui| {
                let float_icon = if app.show_digital_twin_window {
                    crate::ui::icons::X
                } else {
                    crate::ui::icons::ARROW_SQUARE_OUT
                };
                let float_tooltip = if app.show_digital_twin_window {
                    app.tr("Close floating digital twin window")
                } else {
                    app.tr("Pop out digital twin to floating window")
                };

                if ui
                    .button(egui::RichText::new(float_icon).size(13.0))
                    .on_hover_text(float_tooltip)
                    .clicked()
                {
                    app.show_digital_twin_window = !app.show_digital_twin_window;
                }
            },
        );
        if !app.show_digital_twin_window {
            ui.add_space(8.0);
            draw_twin_body(app, ui, caps);
        }
    });
}

fn draw_twin_body(app: &mut PealayerApp, ui: &mut egui::Ui, caps: &HardwareCapabilities) {
    let frozen = caps.telemetry.door_open.unwrap_or(false) || app.estop_active;
    let panel = |app: &mut PealayerApp, ui: &mut egui::Ui| {
        draw_front_panel(app, ui, caps);
        ui.add_space(6.0);
        draw_simulation_controls(app, ui, caps);
    };
    let seat = |app: &mut PealayerApp, ui: &mut egui::Ui| {
        if let Some(sample) = &caps.virtual_seats {
            seat::draw(
                app,
                ui,
                CinemaSeatKinematics::from_positions(sample.positions()),
                frozen,
            );
            ui.horizontal_wrapped(|ui| {
                ui.weak(app.tr("Full travel"));
                if crate::four_d::controller::is_demo_endpoint(&app.serial_port)
                    && !crate::peer::active()
                {
                    let draft = ui.make_persistent_id("virtual-travel-seconds-drag");
                    let mut seconds = ui
                        .data(|data| data.get_temp::<f64>(draft))
                        .unwrap_or(f64::from(sample.full_travel_ms) / 1000.0);
                    let setting = ui
                        .add_enabled(
                            app.board_operation.is_none(),
                            egui::DragValue::new(&mut seconds)
                                .range(0.1..=60.0)
                                .speed(0.05)
                                .max_decimals(2)
                                .suffix(" s")
                                .update_while_editing(false),
                        )
                        .on_hover_text(app.tr("Type the full travel time in seconds"));
                    if setting.dragged() {
                        ui.data_mut(|data| data.insert_temp(draft, seconds));
                    } else if setting.drag_stopped() || setting.changed() {
                        ui.data_mut(|data| data.remove::<f64>(draft));
                        let milliseconds = (seconds * 1000.0).round() as u32;
                        if milliseconds != sample.full_travel_ms
                            && let Err(error) = app.set_virtual_seat_travel(milliseconds)
                        {
                            app.set_osd(error);
                        }
                    }
                } else {
                    ui.label(format!(
                        "{:.2} s",
                        f64::from(sample.full_travel_ms) / 1000.0
                    ));
                }
            });
        } else {
            ui.weak(app.tr("Waiting for seat position feedback"));
        }
    };
    if ui.available_width() >= 520.0 {
        ui.columns(2, |columns| {
            panel(app, &mut columns[0]);
            seat(app, &mut columns[1]);
        });
    } else {
        panel(app, ui);
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        seat(app, ui);
    }
}

/// Draws the standalone floating Digital Twin window.
pub fn draw_digital_twin_window(app: &mut PealayerApp, ctx: &egui::Context) {
    if !app.show_digital_twin_window {
        return;
    }
    let Some(caps) = app
        .advertised_hardware()
        .filter(|caps| app.is_connected && caps.board_connected && is_virtual_board(caps))
    else {
        app.show_digital_twin_window = false;
        return;
    };

    let mut open = true;
    let geometry = crate::ui::dialog::bounded_geometry(
        ctx.content_rect(),
        24.0,
        egui::vec2(600.0, 480.0),
        egui::vec2(440.0, 360.0),
        egui::vec2(900.0, 680.0),
    );

    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::GAUGE,
        app.tr("Virtual Board Digital Twin")
    ))
    .id(egui::Id::new("digital_twin_floating_window"))
    .default_rect(geometry.default_rect)
    .min_size(geometry.min_size)
    .max_size(geometry.max_size)
    .resizable(true)
    .order(egui::Order::Foreground)
    .frame(crate::ui::dialog::opaque_window_frame_from_context(ctx))
    .constrain_to(geometry.bounds)
    .open(&mut open)
    .show(ctx, |ui| {
        super::layout::sidebar_scroll(ui, "floating-virtual-board-scroll", |ui| {
            draw_twin_body(app, ui, &caps);
            ui.add_space(8.0);
            egui::CollapsingHeader::new(app.tr("Buzzer & melodies"))
                .show(ui, |ui| super::layout::draw_buzzer_tool(app, ui, &caps));
        });
    });

    if !open {
        app.show_digital_twin_window = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::PealayerApp;
    use crate::four_d::controller::{
        DEMO_ENDPOINT, HardwareCapabilities, HardwareFrontPanel, HardwareMotionSide,
        HardwareMotionState, HardwareStatusLed,
    };

    #[test]
    fn test_digital_twin_app_integration() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = PealayerApp::default();
        app.engine_handle = crate::four_d::engine::memory_only_test_engine();
        app.serial_port = DEMO_ENDPOINT.to_string();
        app.is_connected = true;

        let mut capabilities = HardwareCapabilities {
            board_connected: true,
            board_name: "Virtual 4D Cinema Board".to_string(),
            port: crate::four_d::controller::HardwarePort {
                product: "PCController Virtual Board".into(),
                ..Default::default()
            },
            supports_virtual_panel_keys: true,
            front_panel: Some(HardwareFrontPanel {
                schema: 1,
                raw_segments: vec![0x3F, 0x06, 0x5B, 0x4F],
                brightness: 7,
                blink: false,
                segments_active: true,
                pressed_keys: 0,
                menu_page: 0,
                program_mode: 0,
                host_captured: false,
                lcd_available: true,
                lcd_address: 0x27,
                lcd_line_1: "Pealayer Cinema".into(),
                lcd_line_2: "Ready 4D Demo".into(),
            }),
            status_led: Some(HardwareStatusLed {
                red: 0,
                green: 255,
                blue: 128,
                brightness: 255,
                effect: 0,
                condition: 0,
            }),
            motion: Some(HardwareMotionState {
                left: HardwareMotionSide {
                    requested: "up".into(),
                    applied: "up".into(),
                    transitioning: false,
                    revision: 1,
                },
                right: HardwareMotionSide {
                    requested: "down".into(),
                    applied: "down".into(),
                    transitioning: false,
                    revision: 1,
                },
            }),
            ..Default::default()
        };
        capabilities.apply_snapshot_values(&serde_json::json!({
            "connected": true, "motion": { "simulation": {
                "left_position": 0.21, "right_position": 0.79,
                "full_travel_ms": 3000, "sampled_at": "2026-11-11T00:00:00Z"
            }}
        }));
        app.update_hardware_capabilities(Some(capabilities.clone()));

        let ctx = egui::Context::default();

        fn discard_ui_output(mut output: egui::FullOutput) {
            output.textures_delta.clear();
        }

        for width in [180.0, 250.0, 700.0] {
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 800.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    let right = ui.max_rect().right();
                    draw_digital_twin_card(&mut app, ui, &capabilities);
                    assert!(
                        ui.min_rect().right() <= right + 0.5,
                        "the twin exceeded {width} points"
                    );
                },
            );
            let painted_text: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.text()),
                    _ => None,
                })
                .collect();
            assert!(
                painted_text
                    .iter()
                    .any(|text| text.contains("Left side") && text.contains("21%"))
            );
            assert!(
                painted_text
                    .iter()
                    .any(|text| text.contains("Right side") && text.contains("79%"))
            );
            assert!(painted_text.contains(&"K4"));
            assert!(!painted_text.contains(&"PLAY"));
            discard_ui_output(output);
        }

        // 2. Verify floating window rendering
        assert!(!app.show_digital_twin_window);
        app.show_digital_twin_window = true;
        discard_ui_output(ctx.run_ui(egui::RawInput::default(), |ui| {
            draw_digital_twin_window(&mut app, ui.ctx());
        }));

        // 3. Verify send_virtual_board_command dispatches to engine handle
        let (tx, rx) = std::sync::mpsc::channel();
        app.engine_handle.sender = tx;
        assert!(app.send_virtual_board_command("door closed").is_ok());
        assert!(app.send_virtual_board_command("rf 0x4D2A 24 1 350").is_ok());

        match rx.try_recv() {
            Ok(crate::four_d::engine::EngineMessage::VirtualBoardCommand(cmd)) => {
                assert_eq!(cmd, "door closed");
            }
            _ => panic!("Expected VirtualBoardCommand('door closed')"),
        }
        match rx.try_recv() {
            Ok(crate::four_d::engine::EngineMessage::VirtualBoardCommand(cmd)) => {
                assert_eq!(cmd, "rf 0x4D2A 24 1 350");
            }
            _ => panic!("Expected VirtualBoardCommand('rf 0x4D2A 24 1 350')"),
        }

        // Verify floating window close toggle.
        app.show_digital_twin_window = false;
        discard_ui_output(ctx.run_ui(egui::RawInput::default(), |ui| {
            draw_digital_twin_window(&mut app, ui.ctx());
        }));
        assert!(!app.show_digital_twin_window);
    }
}
