use crate::app::PealayerApp;
use eframe::egui;

pub fn draw_editor(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_effect_library_editor { return; }
    let mut open = app.show_effect_library_editor;
    let available = app.advertised_hardware().map(|capabilities| capabilities.strip_effects).unwrap_or_default();
    egui::Window::new(app.tr("Effect library"))
        .open(&mut open)
        .default_size([650.0, 430.0])
        .min_size([520.0, 340.0])
        .resizable(true)
        .show(ui.ctx(), |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(210.0);
                    if ui.button(format!("{} {}", crate::ui::icons::SPARKLE, app.tr("New effect"))).clicked() {
                        let mut draft = crate::effects_library::UserStripEffectPreset::default();
                        if let Some(effect) = available.first() { draft.hardware_effect_id = effect.id.clone(); }
                        app.effect_library_selection = Some(draft.id);
                        app.effect_library_draft = draft;
                    }
                    ui.separator();
                    egui::ScrollArea::vertical().id_salt("effect_editor_list").show(ui, |ui| {
                        for preset in app.user_strip_effects.clone() {
                            if ui.selectable_label(app.effect_library_selection == Some(preset.id), &preset.name).clicked() {
                                app.effect_library_selection = Some(preset.id);
                                app.effect_library_draft = preset;
                            }
                        }
                    });
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_min_width(330.0);
                    ui.heading(app.tr("Effect definition"));
                    ui.label(app.tr("Name"));
                    ui.text_edit_singleline(&mut app.effect_library_draft.name);
                    ui.label(app.tr("Category"));
                    ui.text_edit_singleline(&mut app.effect_library_draft.category);
                    ui.label(app.tr("Live renderer"));
                    let renderer_name = available.iter().find(|item| item.id == app.effect_library_draft.hardware_effect_id).map(|item| app.display_text(&item.name)).unwrap_or_else(|| app.tr("Not available on this board"));
                    let renderer_options = available
                        .iter()
                        .map(|renderer| (renderer.id.clone(), app.display_text(&renderer.name)))
                        .collect::<Vec<_>>();
                    egui::ComboBox::from_id_salt("effect_renderer").selected_text(renderer_name).show_ui(ui, |ui| {
                        for (renderer_id, renderer_name) in renderer_options {
                            ui.selectable_value(&mut app.effect_library_draft.hardware_effect_id, renderer_id, renderer_name);
                        }
                    });
                    ui.label(app.tr("Duration (milliseconds)"));
                    ui.add(egui::DragValue::new(&mut app.effect_library_draft.duration_ms).range(1..=3_600_000).speed(100.0));
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let valid = !app.effect_library_draft.name.trim().is_empty()
                            && !app.effect_library_draft.category.trim().is_empty()
                            && available.iter().any(|effect| effect.id == app.effect_library_draft.hardware_effect_id);
                        if ui.add_enabled(valid, egui::Button::new(format!("{} {}", crate::ui::icons::FLOPPY_DISK, app.tr("Save")))).clicked() {
                            if let Some(existing) = app.user_strip_effects.iter_mut().find(|preset| preset.id == app.effect_library_draft.id) {
                                *existing = app.effect_library_draft.clone();
                            } else {
                                app.user_strip_effects.push(app.effect_library_draft.clone());
                            }
                            match crate::effects_library::save(&app.user_strip_effects) {
                                Ok(()) => app.set_osd(app.tr("Effect library saved")),
                                Err(error) => app.set_osd(error),
                            }
                        }
                        let exists = app.user_strip_effects.iter().any(|preset| preset.id == app.effect_library_draft.id);
                        if ui.add_enabled(exists, egui::Button::new(format!("{} {}", crate::ui::icons::X, app.tr("Delete")))).clicked() {
                            app.user_strip_effects.retain(|preset| preset.id != app.effect_library_draft.id);
                            app.effect_library_selection = None;
                            app.effect_library_draft = crate::effects_library::UserStripEffectPreset::default();
                            if let Err(error) = crate::effects_library::save(&app.user_strip_effects) { app.set_osd(error); }
                        }
                    });
                    if available.is_empty() {
                        ui.colored_label(ui.visuals().warn_fg_color, app.tr("Connect a board with an advertised strip renderer to create or edit effects."));
                    }
                });
            });
        });
    app.show_effect_library_editor = open;
}
