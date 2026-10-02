use crate::app::PealayerApp;
use eframe::egui;

/// Helper function to check if a specific relay is active at the current playback time
pub fn is_relay_active(app: &PealayerApp, relay_id: u8) -> bool {
    let t = (app.playback_time * 1000.0) as u64;
    // Traverse instances in reverse order (highest Z-index/last instance first)
    for inst in app.timeline.instances.iter().rev() {
        if let Some(effect) = app
            .timeline
            .templates
            .iter()
            .find(|tmpl| tmpl.id == inst.effect_id)
        {
            let end_time = inst.start_time_ms + effect.duration_ms;
            if t >= inst.start_time_ms && t < end_time {
                let offset_t = t - inst.start_time_ms;
                let mut latest_state = None;
                let mut max_offset = 0;
                for action in &effect.actions {
                    if action.relay_id == relay_id && action.offset_ms <= offset_t {
                        if latest_state.is_none() || action.offset_ms >= max_offset {
                            max_offset = action.offset_ms;
                            latest_state = Some(action.state);
                        }
                    }
                }
                if let Some(state) = latest_state {
                    return state;
                }
            }
        }
    }
    false
}

pub fn draw_editor(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_four_d_editor {
        return;
    }

    let mut dirty = false;
    let display_language = app.language;

    let mut open = app.show_four_d_editor;
    egui::Window::new(app.tr("4D Cinema Editor"))
        .open(&mut open)
        .vscroll(true)
        .default_width(420.0)
        .show(ui.ctx(), |ui| {
            // --- Section 1: Active Relay Status LEDs ---
            ui.heading(app.tr("Active output status"));
            ui.add_space(4.0);
            let relay_capabilities = app
                .advertised_hardware()
                .filter(|capabilities| capabilities.board_connected)
                .map(|capabilities| capabilities.relays)
                .unwrap_or_default();
            if relay_capabilities.is_empty() {
                ui.label(app.tr("Connect PCController to see advertised relay controls."));
            } else {
                ui.horizontal_wrapped(|ui| {
                    for relay in &relay_capabilities {
                        let relay_id = relay.id;
                        let active = is_relay_active(app, relay_id);
                        let color = if active {
                            egui::Color32::from_rgb(46, 204, 113) // Vibrant emerald green
                        } else {
                            egui::Color32::from_rgb(127, 140, 141) // Flat gray
                        };

                        ui.vertical(|ui| {
                            let size = egui::vec2(20.0, 20.0);
                            let (rect, _response) =
                                ui.allocate_exact_size(size, egui::Sense::hover());

                            if active {
                                // Subtle outer glow
                                ui.painter().circle_filled(
                                    rect.center(),
                                    10.0,
                                    egui::Color32::from_rgba_unmultiplied(46, 204, 113, 80),
                                );
                                ui.painter().circle_filled(rect.center(), 7.0, color);
                            } else {
                                ui.painter().circle_filled(rect.center(), 7.0, color);
                            }

                            ui.label(egui::RichText::new(app.display_text(&relay.name)).size(10.0));
                        });
                        ui.add_space(8.0);
                    }
                });
            }
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // --- Section 2: Effect Templates ---
            ui.heading(app.tr("Effect templates"));
            egui::Grid::new("templates_grid")
                .striped(true)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.label(app.tr("Type"));
                    ui.label(app.tr("Name"));
                    ui.label(app.tr("Duration"));
                    ui.label(app.tr("Actions"));
                    ui.end_row();

                    for template in &app.timeline.templates {
                        ui.label(crate::ui::icons::SPARKLE);
                        ui.label(app.display_text(&template.name));
                        ui.label(crate::duration::format_effect_duration_for_language(
                            display_language,
                            template.duration_ms,
                        ));
                        if ui.button(app.tr("Add to timeline")).clicked() {
                            let new_instance = crate::four_d::models::EffectInstance::new(
                                template.id,
                                (app.playback_time * 1000.0) as u64, // Fixed precision bug
                            );
                            app.timeline.instances.push(new_instance);
                            dirty = true;
                        }
                        ui.end_row();
                    }
                });

            ui.add_space(10.0);

            // --- Collapsible Custom Template Creator ---
            let custom_template_label = app.tr("Create custom template");
            if crate::ui::icons::disclosure_header(
                ui,
                "custom_template_creator_disclosure",
                &custom_template_label,
                false,
            ) {
                ui.indent("custom_template_creator_content", |ui| {
                    let name_id = ui.make_persistent_id("new_tmpl_name");
                    let icon_id = ui.make_persistent_id("new_tmpl_icon");
                    let duration_id = ui.make_persistent_id("new_tmpl_duration");
                    let relay_id_id = ui.make_persistent_id("new_tmpl_relay");

                    let mut name =
                        ui.data_mut(|d| d.get_temp::<String>(name_id).unwrap_or_default());
                    let mut icon =
                        ui.data_mut(|d| d.get_temp::<String>(icon_id).unwrap_or_default());
                    let mut duration_ms =
                        ui.data_mut(|d| d.get_temp::<u64>(duration_id).unwrap_or(1000));
                    let default_relay = relay_capabilities
                        .first()
                        .map(|relay| relay.id)
                        .unwrap_or(0);
                    let mut target_relay =
                        ui.data_mut(|d| d.get_temp::<u8>(relay_id_id).unwrap_or(default_relay));

                    egui::Grid::new("create_template_grid").show(ui, |ui| {
                        ui.label(app.tr("Name:"));
                        ui.text_edit_singleline(&mut name);
                        ui.end_row();

                        ui.label(app.tr("Icon:"));
                        ui.text_edit_singleline(&mut icon);
                        ui.end_row();

                        ui.label(app.tr("Duration (ms):"));
                        ui.add(egui::Slider::new(&mut duration_ms, 50..=10000).suffix("ms"));
                        ui.end_row();

                        ui.label(app.tr("Output:"));
                        egui::ComboBox::from_id_salt("custom_template_relay")
                            .selected_text(
                                relay_capabilities
                                    .iter()
                                    .find(|relay| relay.id == target_relay)
                                    .map(|relay| app.display_text(&relay.name))
                                    .unwrap_or_else(|| app.tr("No advertised output")),
                            )
                            .show_ui(ui, |ui| {
                                for relay in &relay_capabilities {
                                    ui.selectable_value(
                                        &mut target_relay,
                                        relay.id,
                                        app.display_text(&relay.name),
                                    );
                                }
                            });
                        ui.end_row();
                    });

                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                target_relay != 0 && !name.trim().is_empty(),
                                egui::Button::new(app.tr("Create")),
                            )
                            .clicked()
                        {
                            let actions = crate::four_d::patterns::generate_constant(
                                target_relay,
                                true,
                                duration_ms,
                            );
                            let new_effect = crate::four_d::models::Effect::with_target(
                                name.clone(),
                                icon.clone(),
                                duration_ms,
                                crate::four_d::models::HardwareTarget::Relay(target_relay),
                                actions,
                            );
                            app.timeline.templates.push(new_effect);

                            // Clear the temp name field
                            name.clear();
                            dirty = true;
                        }
                    });

                    ui.data_mut(|d| {
                        d.insert_temp(name_id, name);
                        d.insert_temp(icon_id, icon);
                        d.insert_temp(duration_id, duration_ms);
                        d.insert_temp(relay_id_id, target_relay);
                    });
                });
            }

            ui.add_space(20.0);

            // --- Section 3: Timeline Instances ---
            ui.heading(app.tr("Timeline instances"));

            // Sort instances by start time for display
            app.timeline
                .instances
                .sort_by_key(|inst| inst.start_time_ms);

            egui::Grid::new("instances_grid")
                .striped(true)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.label(app.tr("Start time"));
                    ui.label(app.tr("Effect"));
                    ui.label(app.tr("Actions"));
                    ui.end_row();

                    let mut to_remove = None;
                    let unavailable_effect = app.tr("Unavailable project effect");
                    let delete_label = app.tr("Delete");
                    let invalid_format_label =
                        app.tr("Invalid format. Use HH:MM:SS.cs (e.g., 00:01:23.45)");
                    let nudge_back_label = app.tr("Nudge back by");
                    let nudge_forward_label = app.tr("Nudge forward by");
                    let nudge_modifier_label = app.tr("Hold Shift for 100ms, Ctrl for 1s");
                    let set_current_label = app.tr("Set to current playback position");
                    let seek_event_label = app.tr("Seek video to this event");
                    let duration_label = app.tr("Dur");
                    let end_label = app.tr("End");

                    for (index, instance) in app.timeline.instances.iter_mut().enumerate() {
                        let id = ui.make_persistent_id(instance.id);

                        let mut time_str = ui.data_mut(|d| {
                            d.get_temp::<String>(id).unwrap_or_else(|| {
                                let ms = instance.start_time_ms;
                                let s = ms / 1000;
                                let cs = (ms % 1000) / 10;
                                format!(
                                    "{:02}:{:02}:{:02}.{:02}",
                                    s / 3600,
                                    (s / 60) % 60,
                                    s % 60,
                                    cs
                                )
                            })
                        });

                        // Check validity of current text on the fly
                        let is_valid = {
                            let parts: Vec<&str> = time_str.split(&[':', '.']).collect();
                            if parts.len() == 4 {
                                parts[0].parse::<u64>().is_ok()
                                    && parts[1].parse::<u64>().is_ok()
                                    && parts[2].parse::<u64>().is_ok()
                                    && parts[3].parse::<u64>().is_ok()
                            } else {
                                false
                            }
                        };

                        ui.horizontal(|ui| {
                            let mut text_edit =
                                egui::TextEdit::singleline(&mut time_str).desired_width(80.0);
                            if !is_valid {
                                text_edit = text_edit.text_color(egui::Color32::RED);
                            }
                            let mut response = ui.add(text_edit);

                            if response.has_focus() {
                                ui.data_mut(|d| d.insert_temp(id, time_str.clone()));
                            } else {
                                ui.data_mut(|d| d.remove::<String>(id));
                            }

                            if !is_valid {
                                response = response.on_hover_text(&invalid_format_label);
                            }

                            if response.changed() || response.lost_focus() {
                                if is_valid {
                                    let parts: Vec<&str> = time_str.split(&[':', '.']).collect();
                                    if let (Ok(h), Ok(m), Ok(s), Ok(cs)) = (
                                        parts[0].parse::<u64>(),
                                        parts[1].parse::<u64>(),
                                        parts[2].parse::<u64>(),
                                        parts[3].parse::<u64>(),
                                    ) {
                                        instance.start_time_ms =
                                            h * 3600000 + m * 60000 + s * 1000 + cs * 10;
                                        dirty = true;
                                    }
                                }
                            }

                            // Modifier-based nudge steps
                            let step = if ui.input(|i| i.modifiers.shift) {
                                100
                            } else if ui.input(|i| i.modifiers.command || i.modifiers.ctrl) {
                                1000
                            } else {
                                10
                            };

                            let step_desc = if ui.input(|i| i.modifiers.shift) {
                                "100ms"
                            } else if ui.input(|i| i.modifiers.command || i.modifiers.ctrl) {
                                "1s"
                            } else {
                                "10ms"
                            };

                            // Small nudge buttons
                            if ui
                                .small_button("-")
                                .on_hover_text(format!(
                                    "{} {} ({})",
                                    nudge_back_label, step_desc, nudge_modifier_label
                                ))
                                .clicked()
                            {
                                instance.start_time_ms =
                                    instance.start_time_ms.saturating_sub(step);
                                dirty = true;
                            }
                            if ui
                                .small_button("+")
                                .on_hover_text(format!(
                                    "{} {} ({})",
                                    nudge_forward_label, step_desc, nudge_modifier_label
                                ))
                                .clicked()
                            {
                                instance.start_time_ms += step;
                                dirty = true;
                            }

                            // Pick current time from seekbar/playback
                            if ui
                                .button(crate::ui::icons::RECORD)
                                .on_hover_text(&set_current_label)
                                .clicked()
                            {
                                instance.start_time_ms = (app.playback_time * 1000.0) as u64;
                                dirty = true;
                            }

                            // Seek video to this event's start time
                            if ui
                                .button(crate::ui::icons::MAGNIFYING_GLASS)
                                .on_hover_text(&seek_event_label)
                                .clicked()
                            {
                                let seconds = instance.start_time_ms as f64 / 1000.0;
                                let _ =
                                    app.mpv.command("seek", &[&seconds.to_string(), "absolute"]);
                            }
                        });

                        // Get template details
                        let template = app
                            .timeline
                            .templates
                            .iter()
                            .find(|t| t.id == instance.effect_id);

                        let (template_name, duration_ms) = match template {
                            Some(t) => (
                                crate::ui::i18n::visual_text(display_language, &t.name),
                                t.duration_ms,
                            ),
                            None => (unavailable_effect.clone(), 0),
                        };

                        // Vertical layout showing Name, Duration, and End Time
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(template_name).strong());

                            let end_time_ms = instance.start_time_ms + duration_ms;
                            let format_ms = |ms: u64| {
                                let s = ms / 1000;
                                let cs = (ms % 1000) / 10;
                                format!(
                                    "{:02}:{:02}:{:02}.{:02}",
                                    s / 3600,
                                    (s / 60) % 60,
                                    s % 60,
                                    cs
                                )
                            };

                            ui.label(
                                egui::RichText::new(format!(
                                    "{}: {} | {}: {}",
                                    duration_label,
                                    crate::duration::format_effect_duration_for_language(
                                        display_language,
                                        duration_ms,
                                    ),
                                    end_label,
                                    format_ms(end_time_ms)
                                ))
                                .weak()
                                .size(10.0),
                            );
                        });

                        if ui.button(&delete_label).clicked() {
                            to_remove = Some(index);
                        }
                        ui.end_row();
                    }

                    if let Some(index) = to_remove {
                        app.timeline.instances.remove(index);

                        dirty = true;
                    }
                });
        });

    if dirty {
        let compiled = crate::four_d::engine::compile_timeline(
            &app.timeline,
            &app.track_muted,
            &app.track_soloed,
        );
        let _ = app
            .engine_handle
            .sender
            .send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
    }

    app.show_four_d_editor = open;
}
