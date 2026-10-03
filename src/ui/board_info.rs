use crate::app::PealayerApp;
use eframe::egui;

const TABS: [(&str, &str); 4] = [
    (crate::ui::icons::INFO, "Overview"),
    (crate::ui::icons::CIRCUITRY, "Capabilities"),
    (crate::ui::icons::APP_WINDOW, "Front panel"),
    (crate::ui::icons::SLIDERS_HORIZONTAL, "Board settings"),
];

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_board_info_dialog {
        return;
    }
    let Some(capabilities) = app.advertised_hardware() else {
        app.show_board_info_dialog = false;
        return;
    };
    if app.board_name_draft.is_empty() {
        app.board_name_draft = capabilities.board_identity.stored_name.clone();
    }

    let mut open = true;
    let bounds = ui.ctx().content_rect().shrink(20.0);
    let max_size = egui::vec2(bounds.width().min(760.0), bounds.height().min(650.0));
    let default_size = egui::vec2(max_size.x.min(640.0), max_size.y.min(470.0));
    let default_rect = crate::ui::dialog::centered_default_rect(bounds, default_size);
    if crate::ui::dialog::escape_pressed(ui.ctx()) {
        open = false;
    }
    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::INFO,
        app.tr("Board information")
    ))
    .id(egui::Id::new("board_information_dialog_bounded_v2"))
    .open(&mut open)
    .default_rect(default_rect)
    .min_size([max_size.x.min(480.0), max_size.y.min(340.0)])
    .max_size(max_size)
    .constrain_to(bounds)
    .resizable(true)
    .movable(true)
    .collapsible(false)
    .show(ui.ctx(), |ui| {
        let connected = capabilities.board_connected;
        ui.horizontal(|ui| {
            let color = if connected {
                egui::Color32::from_rgb(34, 197, 94)
            } else {
                ui.visuals().warn_fg_color
            };
            let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 5.5, color);
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(crate::ui::i18n::visual_text(
                        app.language,
                        &capabilities.board_name,
                    ))
                    .heading()
                    .strong(),
                );
                let subtitle = [
                    capabilities.port.name.as_str(),
                    capabilities.port.product.as_str(),
                ]
                .into_iter()
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
                if !subtitle.is_empty() {
                    ui.label(egui::RichText::new(subtitle).weak());
                }
            });
        });
        ui.separator();

        let available_height = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(148.0, available_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    for (index, (icon, label)) in TABS.into_iter().enumerate() {
                        if crate::ui::dialog::navigation_button(
                            ui,
                            app.board_info_tab == index,
                            icon,
                            &app.tr(label),
                            ui.available_width(),
                        )
                        .clicked()
                        {
                            app.board_info_tab = index;
                        }
                    }
                },
            );
            ui.separator();
            crate::ui::dialog::scroll_column(ui, "board_info_content_v2", None, |ui| {
                match app.board_info_tab {
                    0 => overview(app, ui, &capabilities),
                    1 => capability_list(app, ui, &capabilities),
                    2 => front_panel(app, ui, &capabilities),
                    _ => settings(app, ui, &capabilities),
                }
            });
        });
    });
    app.show_board_info_dialog = open;
    if !open {
        app.board_name_draft.clear();
        app.board_reboot_armed = false;
    }
}

fn overview(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
) {
    section(ui, &app.tr("Identity"), |ui| {
        egui::Grid::new("board_identity_grid")
            .num_columns(2)
            .spacing([18.0, 8.0])
            .show(ui, |ui| {
                let stored_name = if capabilities.board_identity.stored_name_available
                    && !capabilities.board_identity.stored_name.trim().is_empty()
                {
                    capabilities.board_identity.stored_name.clone()
                } else {
                    app.tr("Not assigned")
                };
                row(ui, &app.tr("Stored board name"), &stored_name);
                row(
                    ui,
                    &app.tr("Product identity"),
                    &capabilities.board_identity.product_name,
                );
                if capabilities.board_identity.stored_name_available {
                    row(
                        ui,
                        &app.tr("Name storage"),
                        &if capabilities.board_identity.stored_name_persisted {
                            app.tr("EEPROM (persisted)")
                        } else {
                            app.tr("Not persisted")
                        },
                    );
                }
                row(
                    ui,
                    &app.tr("Board kind"),
                    &capabilities.board_identity.board_kind.to_string(),
                );
                let build_hash = capabilities
                    .board_identity
                    .build_hash
                    .map(|value| format!("{value:08X}"))
                    .unwrap_or_else(|| app.tr("Not advertised"));
                row(ui, &app.tr("Build hash"), &build_hash);
                let build_timestamp = capabilities
                    .board_identity
                    .build_timestamp
                    .clone()
                    .unwrap_or_else(|| app.tr("Not advertised"));
                row(ui, &app.tr("Build timestamp"), &build_timestamp);
                if let Some(profile) = &capabilities.board_profile {
                    row(ui, &app.tr("Profile"), &profile.key);
                    row(ui, &app.tr("Mode"), &profile.mode);
                    row(ui, &app.tr("Profile revision"), &profile.revision);
                }
            });
    });

    section(ui, &app.tr("Connection"), |ui| {
        egui::Grid::new("board_connection_grid")
            .num_columns(2)
            .spacing([18.0, 8.0])
            .show(ui, |ui| {
                optional_row(ui, &app.tr("Device path"), &capabilities.port.name);
                optional_row(ui, &app.tr("Device name"), &capabilities.port.display_name);
                optional_row(
                    ui,
                    &app.tr("Friendly name"),
                    &capabilities.port.friendly_name,
                );
                optional_row(ui, &app.tr("Product"), &capabilities.port.product);
                optional_row(ui, &app.tr("Manufacturer"), &capabilities.port.manufacturer);
                let usb_id = match (capabilities.port.vid.trim(), capabilities.port.pid.trim()) {
                    ("", "") => String::new(),
                    (vid, "") => format!("VID {vid}"),
                    ("", pid) => format!("PID {pid}"),
                    (vid, pid) => format!("VID {vid} · PID {pid}"),
                };
                optional_row(ui, &app.tr("USB identity"), &usb_id);
                optional_row(
                    ui,
                    &app.tr("Serial number"),
                    &capabilities.port.serial_number,
                );
                optional_row(
                    ui,
                    &app.tr("Device instance"),
                    &capabilities.port.instance_id,
                );
            });
    });

    section(ui, &app.tr("Rename board"), |ui| {
        ui.label(
            egui::RichText::new(
                app.tr("Stored by the attached board and read back through PCController."),
            )
            .weak(),
        );
        ui.horizontal(|ui| {
            ui.add_enabled(
                app.board_operation.is_none(),
                egui::TextEdit::singleline(&mut app.board_name_draft)
                    .char_limit(8)
                    .desired_width(150.0),
            );
            if ui
                .add_enabled(
                    app.board_operation.is_none(),
                    egui::Button::new(format!(
                        "{} {}",
                        crate::ui::icons::FLOPPY_DISK,
                        app.tr("Save")
                    )),
                )
                .clicked()
            {
                if let Err(error) = app.rename_board() {
                    app.set_osd(error);
                }
            }
        });
    });

    section(ui, &app.tr("Board commands"), |ui| {
        let label = if app.board_reboot_armed {
            app.tr("Confirm board reboot")
        } else {
            app.tr("Reboot board")
        };
        if ui
            .add_enabled(
                app.board_operation.is_none(),
                egui::Button::new(format!("{} {label}", crate::ui::icons::POWER)),
            )
            .clicked()
        {
            if app.board_reboot_armed {
                if let Err(error) = app.reboot_board() {
                    app.set_osd(error);
                }
            } else {
                app.board_reboot_armed = true;
            }
        }
        if !app.board_operation_status.is_empty() {
            ui.label(egui::RichText::new(&app.board_operation_status).weak());
        }
    });
}

fn capability_list(
    app: &PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
) {
    ui.heading(app.tr("Advertised capabilities"));
    ui.add_space(6.0);
    let advertised = capability_badges(capabilities)
        .into_iter()
        .filter(|(available, _, _)| *available)
        .collect::<Vec<_>>();
    let columns = capability_column_count(ui.available_width());
    for badges in advertised.chunks(columns) {
        ui.columns(columns, |cells| {
            for (cell, (_, icon, label)) in cells.iter_mut().zip(badges.iter()) {
                capability_pill(cell, icon, &app.tr(label));
            }
        });
        ui.add_space(6.0);
    }
    ui.add_space(10.0);
    egui::Grid::new("board_capability_counts")
        .num_columns(2)
        .spacing([20.0, 8.0])
        .show(ui, |ui| {
            row(
                ui,
                &app.tr("Capability bits"),
                &format!("0x{:08X}", capabilities.capability_bits),
            );
            row(
                ui,
                &app.tr("Controls"),
                &capabilities.controls.len().to_string(),
            );
            row(
                ui,
                &app.tr("Relays"),
                &capabilities.relays.len().to_string(),
            );
            row(
                ui,
                &app.tr("PWM channels"),
                &capabilities.pwm_channels.len().to_string(),
            );
            row(
                ui,
                &app.tr("Peripherals"),
                &capabilities.peripherals.len().to_string(),
            );
            row(
                ui,
                &app.tr("Timed effects"),
                &capabilities.macros.len().to_string(),
            );
            row(
                ui,
                &app.tr("Lighting effects"),
                &capabilities.strip_effects.len().to_string(),
            );
        });
}

fn front_panel(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
) {
    let Some(front_panel) = &capabilities.front_panel else {
        unavailable(
            ui,
            &app.tr("The attached board does not advertise front-panel state."),
        );
        return;
    };
    ui.heading(app.tr("Live front-panel state"));
    ui.add_space(6.0);
    section(ui, &app.tr("Live physical display"), |ui| {
        if front_panel.raw_segments.len() == 4 {
            seven_segment_preview(
                ui,
                &front_panel.raw_segments,
                front_panel.brightness,
                front_panel.segments_active,
            );
            ui.add_space(7.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} {}", app.tr("Page"), front_panel.menu_page))
                        .weak(),
                );
                ui.label(egui::RichText::new("·").weak());
                ui.label(
                    egui::RichText::new(format!(
                        "{} {}/7",
                        app.tr("Brightness"),
                        front_panel.brightness.min(7)
                    ))
                    .weak(),
                );
            });
        } else {
            unavailable(ui, &app.tr("No exact seven-segment frame is available."));
        }
    });

    section(ui, &app.tr("Physical board keys"), |ui| {
        ui.columns(2, |columns| {
            for (index, (key, label)) in FRONT_PANEL_KEYS.into_iter().enumerate() {
                let pressed = front_panel.pressed_keys & (1 << index) != 0;
                let text = format!("{}  {key} · {}", key_icon(index), app.tr(label));
                let button = egui::Button::new(text)
                    .selected(pressed)
                    .min_size(egui::vec2(columns[index % 2].available_width(), 34.0));
                if columns[index % 2]
                    .add_enabled(app.board_operation.is_none(), button)
                    .on_hover_text(
                        app.tr("Send the same front-panel key press through PCController"),
                    )
                    .clicked()
                {
                    if let Err(error) = app.press_front_panel_key(key) {
                        app.set_osd(error);
                    }
                }
                if index == 1 {
                    columns[0].add_space(6.0);
                    columns[1].add_space(6.0);
                }
            }
        });
        if !app.board_operation_status.is_empty() {
            ui.add_space(5.0);
            ui.label(egui::RichText::new(&app.board_operation_status).weak());
        }
    });

    egui::Grid::new("front_panel_grid")
        .num_columns(2)
        .spacing([18.0, 8.0])
        .show(ui, |ui| {
            row(
                ui,
                &app.tr("Raw segments"),
                &format!("{:02X?}", front_panel.raw_segments),
            );
            row(
                ui,
                &app.tr("Brightness"),
                &front_panel.brightness.to_string(),
            );
            row(ui, &app.tr("Blink"), &yes_no(app, front_panel.blink));
            row(
                ui,
                &app.tr("Pressed keys"),
                &format!("0x{:02X}", front_panel.pressed_keys),
            );
            row(ui, &app.tr("Menu page"), &front_panel.menu_page.to_string());
            row(
                ui,
                &app.tr("Program mode"),
                &front_panel.program_mode.to_string(),
            );
            row(
                ui,
                &app.tr("LCD available"),
                &yes_no(app, front_panel.lcd_available),
            );
            if front_panel.lcd_available {
                row(
                    ui,
                    &app.tr("LCD address"),
                    &format!("0x{:02X}", front_panel.lcd_address),
                );
                row(ui, &app.tr("LCD line 1"), &front_panel.lcd_line_1);
                row(ui, &app.tr("LCD line 2"), &front_panel.lcd_line_2);
            }
        });
}

fn settings(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
) {
    let Some(settings) = &capabilities.settings else {
        unavailable(
            ui,
            &app.tr("The attached board does not advertise settings."),
        );
        return;
    };
    ui.heading(app.tr("Board settings"));
    let mut silent = settings.silent;
    if ui
        .add_enabled(
            app.board_operation.is_none(),
            egui::Checkbox::new(&mut silent, app.tr("Silent mode")),
        )
        .changed()
    {
        if let Err(error) = app.set_board_silent(silent) {
            app.set_osd(error);
        }
    }
    ui.add_space(8.0);
    egui::Grid::new("board_settings_grid")
        .num_columns(2)
        .spacing([20.0, 8.0])
        .show(ui, |ui| {
            row(ui, &app.tr("Light mode"), &settings.light_mode.to_string());
            row(
                ui,
                &app.tr("On brightness"),
                &settings.on_brightness.to_string(),
            );
            row(
                ui,
                &app.tr("Off brightness"),
                &settings.off_brightness.to_string(),
            );
            row(
                ui,
                &app.tr("Display brightness"),
                &settings.display_brightness.to_string(),
            );
            row(
                ui,
                &app.tr("Status brightness"),
                &settings.status_brightness.to_string(),
            );
            row(
                ui,
                &app.tr("Output persistence"),
                &settings.output_persistence.to_string(),
            );
            row(
                ui,
                &app.tr("Stream period"),
                &format!("{} ms", settings.stream_period_ms),
            );
            row(
                ui,
                &app.tr("Default page"),
                &settings.default_page.to_string(),
            );
            row(
                ui,
                &app.tr("Motion break"),
                &format!("{} ms", settings.motion_break_ms),
            );
            row(ui, &app.tr("Persisted"), &yes_no(app, settings.persisted));
        });
}

fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(12))
        .corner_radius(9.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new(title).strong());
            ui.add_space(5.0);
            body(ui);
        });
    ui.add_space(8.0);
}

fn row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(egui::RichText::new(label).weak());
    ui.label(value);
    ui.end_row();
}

fn optional_row(ui: &mut egui::Ui, label: &str, value: &str) {
    if !value.trim().is_empty() {
        row(ui, label, value);
    }
}

const FRONT_PANEL_KEYS: [(&str, &str); 4] = [
    ("K1", "Previous"),
    ("K2", "Next"),
    ("K3", "Decrease"),
    ("K4", "Select"),
];

fn capability_badges(
    capabilities: &crate::four_d::controller::HardwareCapabilities,
) -> [(bool, &'static str, &'static str); 7] {
    [
        (
            !capabilities.relays.is_empty(),
            crate::ui::icons::PLUG,
            "Relay outputs",
        ),
        (
            !capabilities.pwm_channels.is_empty(),
            crate::ui::icons::SLIDERS_HORIZONTAL,
            "PWM outputs",
        ),
        (
            capabilities.supports_rf_transmit,
            crate::ui::icons::RADIO,
            "RF transmit",
        ),
        (
            capabilities.supports_addressable_led,
            crate::ui::icons::SPARKLE,
            "Addressable strip",
        ),
        (
            capabilities.supports_segment_display,
            crate::ui::icons::GAUGE,
            "Seven-segment display",
        ),
        (
            capabilities.supports_lcd_display,
            crate::ui::icons::APP_WINDOW,
            "LCD display",
        ),
        (
            !capabilities.macros.is_empty(),
            crate::ui::icons::CLOCK,
            "Timed effects",
        ),
    ]
}

fn capability_column_count(available_width: f32) -> usize {
    if available_width >= 360.0 { 2 } else { 1 }
}

fn capability_pill(ui: &mut egui::Ui, icon: &str, text: &str) {
    let width = ui.available_width().max(44.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 30.0), egui::Sense::hover());
    let visuals = ui.visuals();
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect(
        rect,
        15.0,
        visuals.selection.bg_fill.gamma_multiply(0.30),
        egui::Stroke::new(1.0_f32, visuals.selection.bg_fill.gamma_multiply(0.55)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        egui::pos2(rect.left() + 11.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::proportional(15.0),
        visuals.selection.bg_fill,
    );
    painter.text(
        egui::pos2(rect.left() + 32.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(12.0),
        visuals.text_color(),
    );
    response.on_hover_text(text);
}

fn key_icon(index: usize) -> &'static str {
    match index {
        0 => crate::ui::icons::ARROW_UP,
        1 => crate::ui::icons::ARROW_DOWN,
        2 => crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
        _ => crate::ui::icons::CHECK,
    }
}

fn seven_segment_preview(ui: &mut egui::Ui, raw_segments: &[u8], brightness: u8, active: bool) {
    const LINES: [(egui::Pos2, egui::Pos2); 7] = [
        (egui::pos2(8.0, 5.0), egui::pos2(28.0, 5.0)),
        (egui::pos2(31.0, 8.0), egui::pos2(31.0, 27.0)),
        (egui::pos2(31.0, 32.0), egui::pos2(31.0, 51.0)),
        (egui::pos2(8.0, 54.0), egui::pos2(28.0, 54.0)),
        (egui::pos2(5.0, 32.0), egui::pos2(5.0, 51.0)),
        (egui::pos2(5.0, 8.0), egui::pos2(5.0, 27.0)),
        (egui::pos2(8.0, 29.5), egui::pos2(28.0, 29.5)),
    ];
    let digit_width = 40.0_f32;
    let gap = 4.0_f32;
    let content_width = digit_width * 4.0 + gap * 3.0;
    let outer_size = egui::vec2((content_width + 20.0).min(ui.available_width()), 76.0);
    let (rect, _) = ui.allocate_exact_size(outer_size, egui::Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect(
        rect,
        10.0,
        egui::Color32::from_rgb(21, 18, 24),
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(76, 65, 82)),
        egui::StrokeKind::Inside,
    );
    let origin = egui::pos2(rect.left() + 10.0, rect.top() + 7.0);
    let lit_alpha = if active {
        170 + brightness.min(7) * 12
    } else {
        150
    };
    let lit = egui::Color32::from_rgba_unmultiplied(255, 174, 38, lit_alpha);
    let dim = egui::Color32::from_rgba_unmultiplied(194, 181, 205, 35);
    for (digit, mask) in raw_segments.iter().take(4).enumerate() {
        let offset = egui::vec2(digit as f32 * (digit_width + gap), 0.0);
        for (bit, (start, end)) in LINES.into_iter().enumerate() {
            let color = if mask & (1 << bit) != 0 { lit } else { dim };
            painter.line_segment(
                [
                    origin + start.to_vec2() + offset,
                    origin + end.to_vec2() + offset,
                ],
                egui::Stroke::new(5.0_f32, color),
            );
        }
        let dot = origin + egui::vec2(36.0, 54.0) + offset;
        painter.circle_filled(dot, 2.2, if mask & 0x80 != 0 { lit } else { dim });
    }
}

fn unavailable(ui: &mut egui::Ui, message: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        ui.label(egui::RichText::new(crate::ui::icons::INFO).size(28.0));
        ui.label(egui::RichText::new(message).weak());
    });
}

fn yes_no(app: &PealayerApp, value: bool) -> String {
    app.tr(if value { "Yes" } else { "No" })
}

#[cfg(test)]
mod tests {
    use super::{FRONT_PANEL_KEYS, capability_column_count};

    #[test]
    fn capability_badges_use_bounded_responsive_columns() {
        assert_eq!(capability_column_count(359.0), 1);
        assert_eq!(capability_column_count(360.0), 2);
        assert_eq!(capability_column_count(900.0), 2);
    }

    #[test]
    fn front_panel_key_names_match_pccontroller_web_ui() {
        assert_eq!(
            FRONT_PANEL_KEYS,
            [
                ("K1", "Previous"),
                ("K2", "Next"),
                ("K3", "Decrease"),
                ("K4", "Select"),
            ]
        );
    }
}
