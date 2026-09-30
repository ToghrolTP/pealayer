use crate::app::PealayerApp;
use eframe::egui;

pub fn draw_editor(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_effect_library_editor {
        return;
    }
    let mut open = app.show_effect_library_editor;
    let available = app
        .advertised_hardware()
        .map(|capabilities| capabilities.strip_effects)
        .unwrap_or_default();

    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::SPARKLE,
        app.tr("Effect library")
    ))
    .open(&mut open)
    .default_size([620.0, 470.0])
    .min_size([500.0, 360.0])
    .max_size([920.0, 760.0])
    .resizable(true)
    .collapsible(false)
    .show(ui.ctx(), |ui| {
        let height = ui.available_height().max(330.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(190.0, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    if ui
                        .add_sized(
                            [ui.available_width(), 32.0],
                            egui::Button::new(format!(
                                "{} {}",
                                crate::ui::icons::SPARKLE,
                                app.tr("New effect")
                            )),
                        )
                        .clicked()
                    {
                        let mut draft = crate::effects_library::UserStripEffectPreset::default();
                        if let Some(effect) = available.first() {
                            draft.hardware_effect_id = effect.id.clone();
                        }
                        app.effect_library_selection = Some(draft.id);
                        app.effect_library_draft = draft;
                    }
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(app.tr("My effects"))
                            .small()
                            .strong()
                            .color(ui.visuals().weak_text_color()),
                    );
                    egui::ScrollArea::vertical()
                        .id_salt("effect_editor_list")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for preset in app.user_strip_effects.clone() {
                                let selected = app.effect_library_selection == Some(preset.id);
                                let response = ui.add_sized(
                                    [ui.available_width(), 42.0],
                                    egui::Button::new(
                                        egui::RichText::new(format!(
                                            "{}  {}\n   {}",
                                            crate::ui::icons::SPARKLE,
                                            preset.name,
                                            preset.category
                                        ))
                                        .size(12.0),
                                    )
                                    .selected(selected),
                                );
                                if response.clicked() {
                                    app.effect_library_selection = Some(preset.id);
                                    app.effect_library_draft = preset;
                                }
                            }
                        });
                },
            );

            ui.separator();
            let detail_width = ui.available_width().max(285.0);
            ui.allocate_ui_with_layout(
                egui::vec2(detail_width, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("effect_editor_details")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_width(detail_width - 10.0);
                            ui.heading(app.tr("Effect definition"));
                            ui.label(
                                egui::RichText::new(app.tr(
                                    "Saved here; rendered only by capabilities advertised from PCController.",
                                ))
                                .weak()
                                .small(),
                            );
                            ui.add_space(8.0);

                            egui::Grid::new("effect_definition_grid")
                                .num_columns(2)
                                .spacing([14.0, 9.0])
                                .show(ui, |ui| {
                                    ui.label(app.tr("Name"));
                                    ui.add(
                                        egui::TextEdit::singleline(
                                            &mut app.effect_library_draft.name,
                                        )
                                        .desired_width(ui.available_width().max(180.0)),
                                    );
                                    ui.end_row();

                                    ui.label(app.tr("Category"));
                                    ui.add(
                                        egui::TextEdit::singleline(
                                            &mut app.effect_library_draft.category,
                                        )
                                        .desired_width(ui.available_width().max(180.0)),
                                    );
                                    ui.end_row();

                                    ui.label(app.tr("Live renderer"));
                                    let renderer_name = available
                                        .iter()
                                        .find(|item| {
                                            item.id == app.effect_library_draft.hardware_effect_id
                                        })
                                        .map(|item| app.display_text(&item.name))
                                        .unwrap_or_else(|| app.tr("Not available on this board"));
                                    let renderer_options = available
                                        .iter()
                                        .map(|renderer| {
                                            (renderer.id.clone(), app.display_text(&renderer.name))
                                        })
                                        .collect::<Vec<_>>();
                                    egui::ComboBox::from_id_salt("effect_renderer")
                                        .selected_text(renderer_name)
                                        .width(ui.available_width().max(180.0))
                                        .show_ui(ui, |ui| {
                                            for (renderer_id, renderer_name) in renderer_options {
                                                ui.selectable_value(
                                                    &mut app.effect_library_draft.hardware_effect_id,
                                                    renderer_id,
                                                    renderer_name,
                                                );
                                            }
                                        });
                                    ui.end_row();

                                    ui.label(app.tr("Duration"));
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut app.effect_library_draft.duration_ms,
                                            )
                                            .range(1..=3_600_000)
                                            .speed(100.0),
                                        );
                                        ui.label(app.tr("milliseconds"));
                                    });
                                    ui.end_row();
                                });

                            ui.add_space(8.0);
                            ui.label(app.tr("Description"));
                            ui.add_sized(
                                [ui.available_width(), 72.0],
                                egui::TextEdit::multiline(
                                    &mut app.effect_library_draft.description,
                                )
                                .desired_rows(3),
                            );
                            ui.add_space(12.0);

                            let valid = !app.effect_library_draft.name.trim().is_empty()
                                && !app.effect_library_draft.category.trim().is_empty()
                                && available.iter().any(|effect| {
                                    effect.id == app.effect_library_draft.hardware_effect_id
                                });
                            let exists = app
                                .user_strip_effects
                                .iter()
                                .any(|preset| preset.id == app.effect_library_draft.id);
                            ui.horizontal_wrapped(|ui| {
                                if ui
                                    .add_enabled(
                                        valid,
                                        egui::Button::new(format!(
                                            "{} {}",
                                            crate::ui::icons::FLOPPY_DISK,
                                            app.tr("Save")
                                        )),
                                    )
                                    .clicked()
                                {
                                    if let Some(existing) = app
                                        .user_strip_effects
                                        .iter_mut()
                                        .find(|preset| {
                                            preset.id == app.effect_library_draft.id
                                        })
                                    {
                                        *existing = app.effect_library_draft.clone();
                                    } else {
                                        app.user_strip_effects
                                            .push(app.effect_library_draft.clone());
                                    }
                                    match crate::effects_library::save(&app.user_strip_effects) {
                                        Ok(()) => app.set_osd(app.tr("Effect library saved")),
                                        Err(error) => app.set_osd(error),
                                    }
                                }
                                if ui
                                    .add_enabled(
                                        exists,
                                        egui::Button::new(format!(
                                            "{} {}",
                                            crate::ui::icons::COPY,
                                            app.tr("Duplicate")
                                        )),
                                    )
                                    .clicked()
                                {
                                    let duplicate = app.effect_library_draft.duplicate();
                                    app.effect_library_selection = Some(duplicate.id);
                                    app.effect_library_draft = duplicate;
                                }
                                if ui
                                    .add_enabled(
                                        exists,
                                        egui::Button::new(format!(
                                            "{} {}",
                                            crate::ui::icons::TRASH,
                                            app.tr("Delete")
                                        )),
                                    )
                                    .clicked()
                                {
                                    app.user_strip_effects.retain(|preset| {
                                        preset.id != app.effect_library_draft.id
                                    });
                                    app.effect_library_selection = None;
                                    app.effect_library_draft =
                                        crate::effects_library::UserStripEffectPreset::default();
                                    if let Err(error) =
                                        crate::effects_library::save(&app.user_strip_effects)
                                    {
                                        app.set_osd(error);
                                    }
                                }
                            });
                            if available.is_empty() {
                                ui.add_space(8.0);
                                ui.colored_label(
                                    ui.visuals().warn_fg_color,
                                    app.tr("Connect a board with an advertised strip renderer to create or edit effects."),
                                );
                            }
                        });
                },
            );
        });
    });
    app.show_effect_library_editor = open;
}
