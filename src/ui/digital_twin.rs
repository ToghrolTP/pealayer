use eframe::egui;

use crate::app::PealayerApp;
use crate::four_d::controller::{HardwareFrontPanel, HardwareStatusLed};
use crate::four_d::seat_kinematics::CinemaSeatKinematics;

/// Formats a line of text to exactly 16 characters for the 16x2 LCD display.
pub fn format_lcd_line(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() >= 16 {
        trimmed.chars().take(16).collect()
    } else {
        format!("{trimmed:<16}")
    }
}

/// Decodes or formats up to 4 characters for the 4-digit 7-segment display.
pub fn decode_7segment_text(_raw: &[u8], fallback: &str) -> String {
    let trimmed = fallback.trim();
    if trimmed.is_empty() {
        "----".to_string()
    } else if trimmed.chars().count() >= 4 {
        trimmed.chars().take(4).collect::<String>().to_uppercase()
    } else {
        format!("{trimmed:^4}").to_uppercase()
    }
}

/// Formats pitch or roll degrees into a human-readable badge string.
pub fn attitude_badge_label(name: &str, deg: f32) -> String {
    if deg.abs() < 0.05 {
        format!("{name}: 0.0°")
    } else if deg > 0.0 {
        format!("{name}: +{deg:.1}°")
    } else {
        format!("{name}: {deg:.1}°")
    }
}

/// Draws the virtual front-panel cluster: 16x2 LCD, 4-digit 7-segment display, and RGB status LED.
pub fn draw_front_panel(
    ui: &mut egui::Ui,
    panel: Option<&HardwareFrontPanel>,
    led: Option<&HardwareStatusLed>,
    connected: bool,
) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new("HARDWARE FRONT PANEL")
                .size(11.0)
                .strong()
                .color(ui.visuals().weak_text_color()),
        );
        ui.add_space(4.0);

        // 16x2 LCD Matrix Bezel
        let lcd_bg = egui::Color32::from_rgb(10, 24, 12);
        let lcd_border = egui::Color32::from_rgb(22, 60, 28);
        let lcd_text_color = egui::Color32::from_rgb(34, 197, 94);

        egui::Frame::new()
            .fill(lcd_bg)
            .stroke(egui::Stroke::new(1.5, lcd_border))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.set_width(200.0);
                let line1 = format_lcd_line(
                    panel
                        .map(|p| p.lcd_line_1.as_str())
                        .unwrap_or("PCController"),
                );
                let line2 = format_lcd_line(
                    panel
                        .map(|p| p.lcd_line_2.as_str())
                        .unwrap_or("Virtual board"),
                );

                ui.monospace(
                    egui::RichText::new(line1)
                        .size(12.5)
                        .color(lcd_text_color)
                        .strong(),
                );
                ui.monospace(
                    egui::RichText::new(line2)
                        .size(12.5)
                        .color(lcd_text_color)
                        .strong(),
                );
            });

        ui.add_space(8.0);

        // Sub-panel: 7-segment display & Status RGB indicator
        ui.horizontal(|ui| {
            // 7-segment display
            let seg_bg = egui::Color32::from_rgb(20, 14, 5);
            let seg_border = egui::Color32::from_rgb(60, 42, 10);
            let seg_color = egui::Color32::from_rgb(245, 158, 11); // Amber phosphor

            egui::Frame::new()
                .fill(seg_bg)
                .stroke(egui::Stroke::new(1.0, seg_border))
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    let seg_text = decode_7segment_text(
                        panel.map(|p| p.raw_segments.as_slice()).unwrap_or(&[]),
                        "PLAY",
                    );
                    ui.monospace(
                        egui::RichText::new(seg_text)
                            .size(14.0)
                            .strong()
                            .color(seg_color),
                    );
                });

            ui.add_space(12.0);

            // RGB status indicator
            let (rgb_color, condition_text) = if !connected {
                (egui::Color32::from_rgb(100, 100, 100), "Offline")
            } else if let Some(led) = led {
                (
                    egui::Color32::from_rgb(led.red, led.green, led.blue),
                    match led.condition {
                        0 => "Normal",
                        1 => "Alert",
                        2 => "Warning",
                        _ => "Active",
                    },
                )
            } else {
                (egui::Color32::from_rgb(52, 211, 153), "Connected")
            };

            let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
            ui.painter().circle_filled(
                rect.center(),
                7.5,
                rgb_color.gamma_multiply(0.25),
            );
            ui.painter().circle_filled(rect.center(), 5.0, rgb_color);
            ui.painter().circle_filled(
                rect.center() - egui::vec2(1.5, 1.5),
                1.5,
                egui::Color32::WHITE.gamma_multiply(0.7),
            );

            ui.label(
                egui::RichText::new(condition_text)
                    .size(11.0)
                    .color(ui.visuals().weak_text_color()),
            );
        });
    });
}

/// Draws the 2D cinema seat attitude rig and actuator cylinders.
pub fn draw_seat_attitude(
    ui: &mut egui::Ui,
    kinematics: &CinemaSeatKinematics,
    door_open: bool,
) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("2-AXIS CINEMA SEAT KINEMATICS")
                    .size(11.0)
                    .strong()
                    .color(ui.visuals().weak_text_color()),
            );

            let pitch_deg = kinematics.pitch_degrees();
            let roll_deg = kinematics.roll_degrees();

            let pitch_badge = attitude_badge_label("Pitch", pitch_deg);
            let roll_badge = attitude_badge_label("Roll", roll_deg);

            let pitch_color = if pitch_deg.abs() < 0.2 {
                ui.visuals().text_color()
            } else if pitch_deg > 0.0 {
                egui::Color32::from_rgb(56, 189, 248) // Sky blue for tilt back
            } else {
                egui::Color32::from_rgb(52, 211, 153) // Green for tilt forward
            };

            ui.colored_label(
                pitch_color,
                egui::RichText::new(pitch_badge).strong().size(11.5),
            );
            ui.colored_label(
                ui.visuals().text_color(),
                egui::RichText::new(roll_badge).strong().size(11.5),
            );
        });
        ui.add_space(4.0);

        // Canvas for 2D Seat Wireframe
        let desired_size = egui::vec2(ui.available_width().max(240.0), 120.0);
        let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::hover());
        let rect = response.rect;

        let bg_fill = ui.visuals().faint_bg_color.gamma_multiply(0.5);
        painter.rect_filled(rect, 6.0, bg_fill);
        painter.rect_stroke(
            rect,
            6.0,
            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
            egui::StrokeKind::Inside,
        );

        let ground_y = rect.max.y - 14.0;
        let ground_p1 = egui::pos2(rect.min.x + 20.0, ground_y);
        let ground_p2 = egui::pos2(rect.max.x - 20.0, ground_y);
        let ground_stroke = egui::Stroke::new(2.0, ui.visuals().widgets.noninteractive.fg_stroke.color);
        painter.line_segment([ground_p1, ground_p2], ground_stroke);

        let center_x = rect.center().x;
        let left_cyl_x = center_x - 55.0;
        let right_cyl_x = center_x + 55.0;

        let cyl_height = 40.0;
        let max_piston_extension = 26.0;

        let left_ext = kinematics.left_stroke * max_piston_extension;
        let right_ext = kinematics.right_stroke * max_piston_extension;

        let left_top_y = ground_y - cyl_height - left_ext;
        let right_top_y = ground_y - cyl_height - right_ext;

        let cyl_stroke = egui::Stroke::new(1.5, ui.visuals().text_color().gamma_multiply(0.6));
        let piston_stroke = egui::Stroke::new(3.0, egui::Color32::from_rgb(56, 189, 248));

        // Left cylinder
        painter.rect_stroke(
            egui::Rect::from_min_max(
                egui::pos2(left_cyl_x - 6.0, ground_y - cyl_height),
                egui::pos2(left_cyl_x + 6.0, ground_y),
            ),
            2.0,
            cyl_stroke,
            egui::StrokeKind::Inside,
        );
        painter.line_segment(
            [
                egui::pos2(left_cyl_x, ground_y - cyl_height),
                egui::pos2(left_cyl_x, left_top_y),
            ],
            piston_stroke,
        );

        // Right cylinder
        painter.rect_stroke(
            egui::Rect::from_min_max(
                egui::pos2(right_cyl_x - 6.0, ground_y - cyl_height),
                egui::pos2(right_cyl_x + 6.0, ground_y),
            ),
            2.0,
            cyl_stroke,
            egui::StrokeKind::Inside,
        );
        painter.line_segment(
            [
                egui::pos2(right_cyl_x, ground_y - cyl_height),
                egui::pos2(right_cyl_x, right_top_y),
            ],
            piston_stroke,
        );

        // Rotated Seat Base & Profile
        let seat_center = egui::pos2(center_x, (left_top_y + right_top_y) * 0.5);
        let rot = egui::emath::Rot2::from_angle(kinematics.pitch_radians());

        let rotate_point = |p: egui::Pos2| -> egui::Pos2 {
            seat_center + rot * (p - seat_center)
        };

        // Seat base beam
        let beam_l = rotate_point(egui::pos2(center_x - 70.0, seat_center.y));
        let beam_r = rotate_point(egui::pos2(center_x + 70.0, seat_center.y));
        painter.line_segment(
            [beam_l, beam_r],
            egui::Stroke::new(3.5, ui.visuals().text_color()),
        );

        // Seat Cushion
        let cushion_p1 = rotate_point(egui::pos2(center_x - 50.0, seat_center.y - 6.0));
        let cushion_p2 = rotate_point(egui::pos2(center_x + 50.0, seat_center.y - 6.0));
        painter.line_segment(
            [cushion_p1, cushion_p2],
            egui::Stroke::new(6.0, egui::Color32::from_rgb(52, 211, 153)),
        );

        // Seat Backrest (Left side angled up)
        let back_bottom = rotate_point(egui::pos2(center_x - 45.0, seat_center.y - 6.0));
        let back_top = rotate_point(egui::pos2(center_x - 60.0, seat_center.y - 48.0));
        painter.line_segment(
            [back_bottom, back_top],
            egui::Stroke::new(5.0, egui::Color32::from_rgb(52, 211, 153)),
        );

        // Headrest
        let headrest_bottom = rotate_point(egui::pos2(center_x - 62.0, seat_center.y - 50.0));
        let headrest_top = rotate_point(egui::pos2(center_x - 66.0, seat_center.y - 62.0));
        painter.line_segment(
            [headrest_bottom, headrest_top],
            egui::Stroke::new(6.0, egui::Color32::from_rgb(56, 189, 248)),
        );

        // Readout pills at bottom of canvas
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "Left Actuator: {:.0}%",
                    kinematics.left_stroke * 100.0
                ))
                .size(11.0)
                .color(ui.visuals().weak_text_color()),
            );
            ui.label("·");
            ui.label(
                egui::RichText::new(format!(
                    "Right Actuator: {:.0}%",
                    kinematics.right_stroke * 100.0
                ))
                .size(11.0)
                .color(ui.visuals().weak_text_color()),
            );
            if door_open {
                ui.label("·");
                ui.colored_label(
                    egui::Color32::from_rgb(239, 68, 68),
                    egui::RichText::new("MOTION FROZEN").strong().size(11.0),
                );
            }
        });
    });
}

/// Draws interactive simulation controls: door toggle, keys 1..4, RF trigger.
pub fn draw_simulation_controls(app: &mut PealayerApp, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new("SIMULATION CONTROLS & INTERLOCKS")
                .size(11.0)
                .strong()
                .color(ui.visuals().weak_text_color()),
        );
        ui.add_space(4.0);

        let door_open = app
            .advertised_hardware()
            .and_then(|caps| caps.telemetry.door_open)
            .unwrap_or(false);

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

            if ui.add(door_btn).on_hover_text("Toggle virtual enclosure door switch").clicked() {
                let _ = app.send_virtual_board_command("door toggle");
            }

            ui.add_space(8.0);

            // Tactile Front Panel Buttons K1..K4
            for key in 1..=4 {
                let key_label = format!("K{key}");
                if ui
                    .button(egui::RichText::new(&key_label).size(11.0).strong())
                    .on_hover_text(format!("Click front-panel Key {key}"))
                    .clicked()
                {
                    let _ = app.send_virtual_board_command(&format!("key {key} click"));
                }
            }

            ui.add_space(8.0);

            // RF Remote Injection
            let rf_btn = egui::Button::new(
                egui::RichText::new(format!("{} Inject RF Cue", crate::ui::icons::RADIO))
                    .size(11.0),
            );
            if ui.add(rf_btn).on_hover_text("Simulate 24-bit RF remote key packet").clicked() {
                let _ = app.send_virtual_board_command("rfrecv 0x4D2A 24 1 350");
            }
        });
    });
}

/// Draws the complete Digital Twin Card inside `HardwareMonitor` tab.
pub fn draw_digital_twin_card(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let capabilities = app.advertised_hardware();
    let connected = app.is_connected && capabilities.as_ref().is_some_and(|c| c.board_connected);
    let front_panel = capabilities.as_ref().and_then(|c| c.front_panel.as_ref());
    let status_led = capabilities.as_ref().and_then(|c| c.status_led.as_ref());
    let door_open = capabilities
        .as_ref()
        .and_then(|c| c.telemetry.door_open)
        .unwrap_or(false);

    let kinematics = app.seat_kinematics;

    let frame = egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(14, 12))
        .stroke(egui::Stroke::new(
            1.5,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(10.0);

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new(format!(
                    "{} Virtual Board Digital Twin",
                    crate::ui::icons::GAUGE
                ))
                .size(14.5),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let float_icon = if app.show_digital_twin_window {
                    crate::ui::icons::X
                } else {
                    crate::ui::icons::ARROW_SQUARE_OUT
                };
                let float_tooltip = if app.show_digital_twin_window {
                    "Close floating digital twin window"
                } else {
                    "Pop out digital twin to floating window"
                };

                if ui
                    .button(egui::RichText::new(float_icon).size(13.0))
                    .on_hover_text(float_tooltip)
                    .clicked()
                {
                    app.show_digital_twin_window = !app.show_digital_twin_window;
                }
            });
        });

        ui.add_space(8.0);

        // Responsive grid: left column (Front Panel) & right column (Seat Kinematics)
        let is_wide = ui.available_width() > 520.0;
        if is_wide {
            ui.columns(2, |cols| {
                draw_front_panel(&mut cols[0], front_panel, status_led, connected);
                draw_seat_attitude(&mut cols[1], &kinematics, door_open);
            });
        } else {
            draw_front_panel(ui, front_panel, status_led, connected);
            ui.add_space(10.0);
            draw_seat_attitude(ui, &kinematics, door_open);
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);
        draw_simulation_controls(app, ui);
    });
}

/// Draws the standalone floating Digital Twin window.
pub fn draw_digital_twin_window(app: &mut PealayerApp, ctx: &egui::Context) {
    if !app.show_digital_twin_window {
        return;
    }

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
        "Virtual Board Digital Twin"
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
        let capabilities = app.advertised_hardware();
        let connected = app.is_connected && capabilities.as_ref().is_some_and(|c| c.board_connected);
        let front_panel = capabilities.as_ref().and_then(|c| c.front_panel.as_ref());
        let status_led = capabilities.as_ref().and_then(|c| c.status_led.as_ref());
        let door_open = capabilities
            .as_ref()
            .and_then(|c| c.telemetry.door_open)
            .unwrap_or(false);

        let kinematics = app.seat_kinematics;

        draw_front_panel(ui, front_panel, status_led, connected);
        ui.add_space(12.0);
        draw_seat_attitude(ui, &kinematics, door_open);
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(6.0);
        draw_simulation_controls(app, ui);
    });

    if !open {
        app.show_digital_twin_window = false;
    }
}
