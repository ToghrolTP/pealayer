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
        app.board_name_draft = capabilities.board_name.clone();
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
                row(ui, &app.tr("Board name"), &capabilities.board_name);
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
    ui.horizontal_wrapped(|ui| {
        for (available, label) in [
            (!capabilities.relays.is_empty(), "Relay outputs"),
            (!capabilities.pwm_channels.is_empty(), "PWM outputs"),
            (capabilities.supports_rf_transmit, "RF transmit"),
            (capabilities.supports_addressable_led, "Addressable strip"),
            (
                capabilities.supports_segment_display,
                "Seven-segment display",
            ),
            (capabilities.supports_lcd_display, "LCD display"),
            (!capabilities.macros.is_empty(), "Timed effects"),
        ] {
            if available {
                pill(ui, &app.tr(label));
            }
        }
    });
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
    app: &PealayerApp,
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

fn pill(ui: &mut egui::Ui, text: &str) {
    let width = capability_pill_width(text, ui.available_width());
    egui::Frame::new()
        .fill(ui.visuals().selection.bg_fill.gamma_multiply(0.34))
        .corner_radius(99.0)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            let content_width = (width - 16.0).max(28.0);
            ui.set_min_width(content_width);
            ui.set_max_width(content_width);
            ui.add_sized(
                [content_width, 18.0],
                egui::Label::new(egui::RichText::new(text).small().strong()).truncate(),
            )
            .on_hover_text(text);
        });
}

fn capability_pill_width(text: &str, available_width: f32) -> f32 {
    let natural_width = text.chars().count() as f32 * 7.0 + 24.0;
    natural_width.clamp(44.0, available_width.clamp(44.0, 220.0))
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
    use super::capability_pill_width;

    #[test]
    fn capability_pills_are_clamped_to_the_visible_panel() {
        assert_eq!(capability_pill_width("RF", 180.0), 44.0);
        assert_eq!(capability_pill_width(&"x".repeat(80), 180.0), 180.0);
        assert_eq!(capability_pill_width(&"x".repeat(80), 480.0), 220.0);
    }
}
