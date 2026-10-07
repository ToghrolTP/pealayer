use crate::app::PealayerApp;
use eframe::egui;

/// One manager for every workspace, including the two editable profiles that
/// seed a first installation. The same stable IDs, names, icons, and ordering
/// are published to the Web UI through the live status contract.
pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_workspace_profiles_dialog {
        return;
    }

    let mut open = true;
    let bounds = ui.ctx().content_rect().shrink(20.0);
    let max_size = egui::vec2(bounds.width().min(780.0), bounds.height().min(620.0));
    let default_size = egui::vec2(max_size.x.min(700.0), max_size.y.min(500.0));
    if crate::ui::dialog::escape_pressed(ui.ctx()) {
        open = false;
    }

    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::TABS,
        app.tr("Workspaces")
    ))
    .id(egui::Id::new("workspace_profiles_dialog_v2"))
    .open(&mut open)
    .default_rect(crate::ui::dialog::centered_default_rect(
        bounds,
        default_size,
    ))
    .min_size([520.0, 320.0])
    .max_size(max_size)
    .constrain_to(bounds)
    .movable(true)
    .resizable(true)
    .collapsible(false)
    .show(ui.ctx(), |ui| {
        ui.horizontal(|ui| {
            let name_hint = app.tr("Workspace name");
            ui.add_sized(
                [ui.available_width().max(280.0) - 250.0, 30.0],
                egui::TextEdit::singleline(&mut app.workspace_profile_name_draft)
                    .hint_text(name_hint),
            );
            workspace_icon_picker(
                app.language,
                ui,
                "new-workspace-icon",
                &mut app.workspace_profile_icon_draft,
            );
            if crate::ui::dialog::primary_action_button(
                ui,
                crate::ui::icons::PLUS,
                &app.tr("Add workspace"),
            )
            .clicked()
            {
                let name = app.workspace_profile_name_draft.clone();
                app.save_workspace_profile(ui.ctx(), &name);
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        let profiles = app.ordered_workspace_profiles();
        if profiles.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(app.tr("No workspace profiles"));
            });
        } else {
            let profile_count = profiles.len();
            crate::ui::dialog::scroll_column(
                ui,
                "workspace_profiles_list",
                Some(ui.available_height().max(140.0)),
                |ui| {
                    for (index, (id, profile)) in profiles.into_iter().enumerate() {
                        let active = app.active_workspace_profile.as_deref() == Some(id.as_str());
                        let mut name = profile.name.clone();
                        let mut icon = profile.icon.clone();
                        let mut restore = false;
                        let mut overwrite = false;
                        let mut delete = false;
                        let mut move_direction = 0;
                        let mut metadata_changed = false;

                        egui::Frame::group(ui.style())
                            .inner_margin(egui::Margin::symmetric(10, 7))
                            .corner_radius(8.0)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    if active {
                                        ui.colored_label(
                                            ui.visuals().selection.bg_fill,
                                            crate::ui::icons::CHECK_SQUARE,
                                        )
                                        .on_hover_text(app.tr("Active workspace"));
                                    } else {
                                        ui.label(crate::ui::icons::APP_WINDOW);
                                    }
                                    let name_response = ui.add_sized(
                                        [ui.available_width().max(420.0) - 390.0, 28.0],
                                        egui::TextEdit::singleline(&mut name),
                                    );
                                    metadata_changed |= name_response.changed();
                                    metadata_changed |= workspace_icon_picker(
                                        app.language,
                                        ui,
                                        format!("workspace-icon-{id}"),
                                        &mut icon,
                                    );

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            delete = ui
                                                .button(crate::ui::icons::TRASH)
                                                .on_hover_text(app.tr("Delete workspace"))
                                                .clicked();
                                            if ui
                                                .add_enabled(
                                                    index + 1 < profile_count,
                                                    egui::Button::new(crate::ui::icons::ARROW_DOWN),
                                                )
                                                .on_hover_text(app.tr("Move down"))
                                                .clicked()
                                            {
                                                move_direction = 1;
                                            }
                                            if ui
                                                .add_enabled(
                                                    index > 0,
                                                    egui::Button::new(crate::ui::icons::ARROW_UP),
                                                )
                                                .on_hover_text(app.tr("Move up"))
                                                .clicked()
                                            {
                                                move_direction = -1;
                                            }
                                            overwrite = ui
                                                .button(crate::ui::icons::FLOPPY_DISK)
                                                .on_hover_text(
                                                    app.tr("Replace with current workspace"),
                                                )
                                                .clicked();
                                            restore = ui
                                                .button(format!(
                                                    "{}  {}",
                                                    crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                                                    app.tr("Restore")
                                                ))
                                                .clicked();
                                        },
                                    );
                                });
                            });

                        if metadata_changed && !name.trim().is_empty() {
                            app.update_workspace_profile_metadata(&id, &name, &icon);
                        }
                        if restore {
                            app.restore_workspace_profile(ui.ctx(), &id);
                        }
                        if overwrite {
                            app.overwrite_workspace_profile(ui.ctx(), &id);
                        }
                        if move_direction != 0 {
                            app.move_workspace_profile(&id, move_direction);
                        }
                        if delete {
                            app.delete_workspace_profile(&id);
                        }
                        ui.add_space(4.0);
                    }
                },
            );
        }

        if !app.config_status.is_empty() {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&app.config_status)
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        }
    });
    app.show_workspace_profiles_dialog = open;
}

fn workspace_icon_picker(
    language: crate::config::AppLanguage,
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    value: &mut String,
) -> bool {
    if value.trim().is_empty() {
        *value = "window".to_string();
    }
    crate::ui::icons::searchable_workspace_icon_picker(
        ui,
        id,
        value,
        126.0,
        &crate::ui::i18n::tr(language, "Search icons..."),
        &crate::ui::i18n::tr(language, "Presets"),
        &crate::ui::i18n::tr(language, "No matching icons"),
        language,
    )
}
