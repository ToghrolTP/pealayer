use crate::app::PealayerApp;
use eframe::egui;

/// Named workspace snapshots complement the automatic last-session restore.
/// The profile payload remains in the shared AppConfig contract so desktop,
/// IPC, and Web configuration surfaces can discover the same saved profiles.
pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_workspace_profiles_dialog {
        return;
    }

    let mut open = true;
    let bounds = ui.ctx().content_rect().shrink(20.0);
    let max_size = egui::vec2(bounds.width().min(620.0), bounds.height().min(520.0));
    let default_size = egui::vec2(max_size.x.min(520.0), max_size.y.min(430.0));
    if crate::ui::dialog::escape_pressed(ui.ctx()) {
        open = false;
    }

    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::TABS,
        app.tr("Workspace profiles")
    ))
    .id(egui::Id::new("workspace_profiles_dialog_v1"))
    .open(&mut open)
    .default_rect(crate::ui::dialog::centered_default_rect(
        bounds,
        default_size,
    ))
    .min_size([360.0, 280.0])
    .max_size(max_size)
    .constrain_to(bounds)
    .movable(true)
    .resizable(true)
    .collapsible(false)
    .show(ui.ctx(), |ui| {
        ui.label(app.tr(
            "Save this window, panel, dialog, and scroll arrangement as a reusable workspace.",
        ));
        ui.add_space(8.0);
        let name_hint = app.tr("Workspace name");
        ui.horizontal(|ui| {
            ui.label(format!(
                "{}  {}",
                crate::ui::icons::PENCIL_SIMPLE,
                app.tr("Name")
            ));
            let response = ui.add_sized(
                [ui.available_width().max(180.0) - 116.0, 28.0],
                egui::TextEdit::singleline(&mut app.workspace_profile_name_draft)
                    .hint_text(name_hint),
            );
            let save_requested = crate::ui::dialog::primary_action_button(
                ui,
                crate::ui::icons::FLOPPY_DISK,
                &app.tr("Save"),
            )
            .clicked()
                || (response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)));
            if save_requested {
                let name = app.workspace_profile_name_draft.clone();
                app.save_workspace_profile(ui.ctx(), &name);
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        if app.workspace_profiles.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(app.tr("No saved workspace profiles"));
            });
        } else {
            let entries = app.workspace_profiles.keys().cloned().collect::<Vec<_>>();
            crate::ui::dialog::scroll_column(
                ui,
                "workspace_profiles_list",
                Some(ui.available_height().max(120.0)),
                |ui| {
                    for name in entries {
                        let active = app.active_workspace_profile.as_deref() == Some(name.as_str());
                        egui::Frame::group(ui.style())
                            .inner_margin(egui::Margin::symmetric(10, 7))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let icon = if active {
                                        crate::ui::icons::CHECK_SQUARE
                                    } else {
                                        crate::ui::icons::APP_WINDOW
                                    };
                                    ui.label(icon);
                                    ui.strong(&name);
                                    if active {
                                        ui.label(
                                            egui::RichText::new(app.tr("Active"))
                                                .small()
                                                .color(ui.visuals().weak_text_color()),
                                        );
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .button(crate::ui::icons::TRASH)
                                                .on_hover_text(app.tr("Delete workspace profile"))
                                                .clicked()
                                            {
                                                app.delete_workspace_profile(&name);
                                            }
                                            if ui
                                                .button(format!(
                                                    "{}  {}",
                                                    crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                                                    app.tr("Restore")
                                                ))
                                                .clicked()
                                            {
                                                app.restore_workspace_profile(ui.ctx(), &name);
                                            }
                                        },
                                    );
                                });
                            });
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
