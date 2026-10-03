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
    if app.board_settings_draft.is_none() {
        app.board_settings_draft = capabilities.settings.clone();
        app.board_settings_dirty = false;
    }

    let mut open = true;
    let geometry = crate::ui::dialog::bounded_geometry(
        ui.ctx().content_rect(),
        20.0,
        egui::vec2(640.0, 470.0),
        egui::vec2(480.0, 340.0),
        egui::vec2(760.0, 650.0),
    );
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
    .default_rect(geometry.default_rect)
    .min_size(geometry.min_size)
    .max_size(geometry.max_size)
    .constrain_to(geometry.bounds)
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
        app.board_settings_draft = None;
        app.board_settings_dirty = false;
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
            let name_align = crate::ui::i18n::input_alignment(app.rtl, &app.board_name_draft);
            ui.add_enabled(
                app.board_operation.is_none(),
                egui::TextEdit::singleline(&mut app.board_name_draft)
                    .horizontal_align(name_align)
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
    let Some(authoritative) = &capabilities.settings else {
        unavailable(
            ui,
            &app.tr("The attached board does not advertise settings."),
        );
        return;
    };
    let mut settings = app
        .board_settings_draft
        .clone()
        .unwrap_or_else(|| authoritative.clone());
    let before = settings.clone();

    ui.heading(app.tr("Board settings"));
    ui.label(
        egui::RichText::new(app.tr("Only settings advertised by the connected board are shown."))
            .weak(),
    );
    ui.add_space(8.0);

    section(ui, &app.tr("General"), |ui| {
        settings_grid(ui, "board_settings_general", |ui| {
            checkbox_setting(ui, &app.tr("Silent mode"), &mut settings.silent);
            checkbox_setting(
                ui,
                &app.tr("Door sound cues"),
                &mut settings.door_audio_enabled,
            );
            checkbox_setting(
                ui,
                &app.tr("Relay sound cues"),
                &mut settings.relay_audio_enabled,
            );
            setting_control(ui, &app.tr("Telemetry period"), |ui| {
                ui.add(crate::duration::time_value_drag(
                    &mut settings.stream_period_ms,
                    0..=u16::MAX as u64,
                    10.0,
                ))
                .on_hover_text(app.tr("Use 0 to disable periodic telemetry"));
            });
        });
    });

    if capabilities.supports_temperature_sensors {
        section(ui, &app.tr("Temperature sensors"), |ui| {
            checkbox_setting(
                ui,
                &app.tr("Swap temperature roles"),
                &mut settings.swap_temperature_roles,
            );
        });
    }

    if !capabilities.pwm_channels.is_empty() {
        section(ui, &app.tr("Enclosure lighting"), |ui| {
            settings_grid(ui, "board_settings_lighting", |ui| {
                setting_control(ui, &app.tr("Light mode"), |ui| {
                    enum_combo(
                        ui,
                        "board_light_mode",
                        &mut settings.light_mode,
                        &[
                            (0, app.tr("Off")),
                            (1, app.tr("Automatic (door)")),
                            (2, app.tr("On")),
                        ],
                    );
                });
                slider_setting(
                    ui,
                    &app.tr("On brightness"),
                    &mut settings.on_brightness,
                    0..=255,
                );
                slider_setting(
                    ui,
                    &app.tr("Off brightness"),
                    &mut settings.off_brightness,
                    0..=255,
                );
            });
        });
    }

    if capabilities.supports_segment_display {
        section(ui, &app.tr("Front-panel display"), |ui| {
            settings_grid(ui, "board_settings_display", |ui| {
                slider_setting(
                    ui,
                    &app.tr("Open brightness"),
                    &mut settings.display_brightness,
                    0..=7,
                );
                slider_setting(
                    ui,
                    &app.tr("Closed brightness"),
                    &mut settings.display_closed_brightness,
                    0..=7,
                );
                setting_control(ui, &app.tr("Default page"), |ui| {
                    ui.add(egui::DragValue::new(&mut settings.default_page).range(0..=13));
                });
                checkbox_setting(
                    ui,
                    &app.tr("Remember last page"),
                    &mut settings.save_last_page,
                );
            });
        });
    }

    if capabilities.supports_status_led_settings {
        section(ui, &app.tr("Status light"), |ui| {
            settings_grid(ui, "board_settings_status", |ui| {
                slider_setting(
                    ui,
                    &app.tr("Status brightness"),
                    &mut settings.status_brightness,
                    0..=255,
                );
                setting_control(ui, &app.tr("Fallback color"), |ui| {
                    enum_combo(
                        ui,
                        "board_status_color",
                        &mut settings.status_color,
                        &[
                            (0, app.tr("Red")),
                            (1, app.tr("Blue")),
                            (2, app.tr("Violet")),
                            (3, app.tr("Green")),
                            (4, app.tr("White")),
                        ],
                    );
                });
            });
            ui.add_space(6.0);
            let enabled_id = ui.make_persistent_id("status_led_custom_override_enabled");
            let color_id = ui.make_persistent_id("status_led_custom_override_color");
            let mut override_enabled = ui
                .data_mut(|data| data.get_temp::<bool>(enabled_id))
                .unwrap_or(false);
            let fallback = capabilities
                .status_led
                .as_ref()
                .map(|status| [status.red, status.green, status.blue])
                .unwrap_or([34, 197, 94]);
            let mut override_color = ui
                .data_mut(|data| data.get_temp::<[u8; 3]>(color_id))
                .unwrap_or(fallback);
            ui.horizontal(|ui| {
                let changed = ui
                    .checkbox(&mut override_enabled, app.tr("Override with custom color"))
                    .changed();
                ui.add_enabled_ui(override_enabled, |ui| {
                    ui.color_edit_button_srgb(&mut override_color)
                        .on_hover_text(app.tr("Choose the live status-light color"));
                });
                let busy = app.board_operation.is_some();
                if ui
                    .add_enabled(
                        override_enabled && !busy,
                        egui::Button::new(format!(
                            "{} {}",
                            crate::ui::icons::CHECK,
                            app.tr("Apply")
                        )),
                    )
                    .clicked()
                    && let Err(error) =
                        app.set_status_led_override(override_color, settings.status_brightness)
                {
                    app.set_osd(error);
                }
                if changed
                    && !override_enabled
                    && !busy
                    && let Err(error) = app.release_status_led_override()
                {
                    app.set_osd(error);
                }
            });
            ui.data_mut(|data| {
                data.insert_temp(enabled_id, override_enabled);
                data.insert_temp(color_id, override_color);
            });
        });
    }

    if !capabilities.relays.is_empty() {
        section(ui, &app.tr("Motion and relays"), |ui| {
            settings_grid(ui, "board_settings_motion", |ui| {
                setting_control(ui, &app.tr("Door policy"), |ui| {
                    enum_combo(
                        ui,
                        "board_motion_policy",
                        &mut settings.motion_door_policy,
                        &[
                            (0, app.tr("Always allow motion")),
                            (1, app.tr("Only while door is closed")),
                            (2, app.tr("Only while door is open")),
                            (3, app.tr("Never allow motion")),
                        ],
                    );
                });
                setting_control(ui, &app.tr("Exit hold"), |ui| {
                    ui.add(
                        egui::Slider::new(&mut settings.motion_exit_hold_seconds, 1..=31)
                            .suffix(" s"),
                    );
                });
                if capabilities.supports_motion_break_setting {
                    setting_control(ui, &app.tr("Motion break"), |ui| {
                        ui.add(crate::duration::time_value_drag(
                            &mut settings.motion_break_ms,
                            1..=255,
                            1.0,
                        ));
                    });
                }
            });
            ui.add_space(5.0);
            ui.label(egui::RichText::new(app.tr("Relays restored after restart")).weak());
            relay_mask_editor(ui, app, &mut settings.relay_restore_mask);
        });
    }

    if !capabilities.relays.is_empty() || !capabilities.pwm_channels.is_empty() {
        section(ui, &app.tr("Output persistence"), |ui| {
            bit_checkbox(
                ui,
                &mut settings.output_persistence,
                0x01,
                &app.tr("Remember motion defaults"),
                !capabilities.relays.is_empty(),
            );
            bit_checkbox(
                ui,
                &mut settings.output_persistence,
                0x02,
                &app.tr("Remember user relays"),
                !capabilities.relays.is_empty(),
            );
            bit_checkbox(
                ui,
                &mut settings.output_persistence,
                0x04,
                &app.tr("Remember PWM outputs"),
                !capabilities.pwm_channels.is_empty(),
            );
            bit_checkbox(
                ui,
                &mut settings.output_persistence,
                0x08,
                &app.tr("Retain motion direction when stopped"),
                !capabilities.relays.is_empty(),
            );
        });
    }

    if capabilities.supports_measurements {
        section(ui, &app.tr("Measurements"), |ui| {
            settings_grid(ui, "board_settings_measurements", |ui| {
                setting_control(ui, &app.tr("Voltage decimals"), |ui| {
                    ui.add(egui::Slider::new(&mut settings.voltage_decimals, 0..=2));
                });
                setting_control(ui, &app.tr("Current decimals"), |ui| {
                    ui.add(egui::Slider::new(&mut settings.current_decimals, 0..=2));
                });
            });
        });
    }

    section(ui, &app.tr("Advanced"), |ui| {
        checkbox_setting(
            ui,
            &app.tr("Programming latch"),
            &mut settings.programming_latch,
        );
        if settings.programming_latch {
            ui.label(
                egui::RichText::new(app.tr(
                    "Programming latch blocks motion, relays, PWM, and lighting until disabled.",
                ))
                .color(ui.visuals().warn_fg_color),
            );
        }
    });

    if settings != before {
        app.board_settings_dirty = settings != *authoritative;
        app.board_settings_draft = Some(settings.clone());
    }

    ui.horizontal(|ui| {
        let busy = app.board_operation.is_some();
        if ui
            .add_enabled_ui(!busy && app.board_settings_dirty, |ui| {
                crate::ui::dialog::action_button(
                    ui,
                    crate::ui::icons::FLOPPY_DISK,
                    &app.tr("Save to board"),
                )
            })
            .inner
            .clicked()
        {
            if let Err(error) = app.save_board_settings() {
                app.set_osd(error);
            }
        }
        if ui
            .add_enabled_ui(!busy && app.board_settings_dirty, |ui| {
                crate::ui::dialog::action_button(
                    ui,
                    crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                    &app.tr("Revert"),
                )
            })
            .inner
            .clicked()
        {
            app.board_settings_draft = Some(authoritative.clone());
            app.board_settings_dirty = false;
        }
        let storage = if capabilities.supports_persistent_settings && authoritative.persisted {
            app.tr("EEPROM (persisted)")
        } else if capabilities.supports_persistent_settings {
            app.tr("Not persisted")
        } else {
            app.tr("Live settings")
        };
        ui.label(egui::RichText::new(storage).weak());
    });
    if !app.board_operation_status.is_empty() {
        ui.label(egui::RichText::new(&app.board_operation_status).weak());
    }
}

fn settings_grid(ui: &mut egui::Ui, id: &'static str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([18.0, 8.0])
        .min_col_width(150.0)
        .show(ui, body);
}

fn setting_control(ui: &mut egui::Ui, label: &str, control: impl FnOnce(&mut egui::Ui)) {
    ui.label(egui::RichText::new(label).weak());
    ui.horizontal(|ui| {
        ui.set_min_width((ui.available_width() - 4.0).max(150.0));
        control(ui);
    });
    ui.end_row();
}

fn checkbox_setting(ui: &mut egui::Ui, label: &str, value: &mut bool) {
    setting_control(ui, label, |ui| {
        ui.checkbox(value, "");
    });
}

fn slider_setting(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u8,
    range: std::ops::RangeInclusive<u8>,
) {
    setting_control(ui, label, |ui| {
        ui.add_sized(
            [ui.available_width().max(180.0), 22.0],
            egui::Slider::new(value, range),
        );
    });
}

fn enum_combo(ui: &mut egui::Ui, id: &'static str, value: &mut u8, options: &[(u8, String)]) {
    let selected = options
        .iter()
        .find(|(candidate, _)| candidate == value)
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| value.to_string());
    egui::ComboBox::from_id_salt(id)
        .selected_text(selected)
        .width(ui.available_width().max(180.0))
        .show_ui(ui, |ui| {
            for (candidate, label) in options {
                ui.selectable_value(value, *candidate, label);
            }
        });
}

fn bit_checkbox(ui: &mut egui::Ui, value: &mut u8, bit: u8, label: &str, available: bool) {
    if !available {
        return;
    }
    let mut enabled = *value & bit != 0;
    if ui.checkbox(&mut enabled, label).changed() {
        if enabled {
            *value |= bit;
        } else {
            *value &= !bit;
        }
    }
}

fn relay_mask_editor(ui: &mut egui::Ui, app: &PealayerApp, mask: &mut u8) {
    ui.horizontal_wrapped(|ui| {
        for index in 0..8 {
            bit_checkbox(
                ui,
                mask,
                1 << index,
                &format!("{} {}", app.tr("Relay"), index + 1),
                true,
            );
        }
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
