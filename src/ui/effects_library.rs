use crate::app::{ControllerEffectDraft, PealayerApp};
use eframe::egui;

fn secondary_click_inside(ctx: &egui::Context, rect: egui::Rect) -> bool {
    ctx.input(|input| {
        input
            .pointer
            .button_released(egui::PointerButton::Secondary)
            && input
                .pointer
                .latest_pos()
                .is_some_and(|position| rect.contains(position))
    })
}

fn show_saved_effect_context_menu(
    app: &mut PealayerApp,
    response: &egui::Response,
    popup_id: egui::Id,
    name: &str,
    reference: &str,
    payload: &crate::app::EffectDragPayload,
    select: impl FnOnce(&mut PealayerApp),
) {
    let open = response.secondary_clicked() || secondary_click_inside(&response.ctx, response.rect);
    egui::Popup::menu(response)
        .id(popup_id)
        .at_pointer_fixed()
        .open_memory(open.then_some(egui::SetOpenCommand::Bool(true)))
        .show(|ui| {
            select(app);
            draw_saved_effect_context_menu(app, ui, name, reference, payload);
        });
}

pub(crate) fn select_sequence(
    app: &mut PealayerApp,
    effect: &crate::four_d::controller::HardwareMacro,
) {
    let reference = format!("effect:{}", effect.id);
    app.effect_library_selection = Some(reference.clone());
    app.effect_library_draft = ControllerEffectDraft {
        reference,
        id: effect.id.to_string(),
        name: effect.name.clone(),
        category: effect.category.clone(),
        description: String::new(),
        kind: "sequence".to_string(),
        program_json: String::new(),
        color: "green".to_string(),
        default_fps: 0,
        duration_ms: effect.duration_ms,
        default_pixels: 0,
        is_new: false,
    };
}

pub(crate) fn select_strip(
    app: &mut PealayerApp,
    effect: &crate::four_d::controller::HardwareStripEffect,
) {
    let reference = format!("effect:{}", effect.id);
    app.effect_library_selection = Some(reference.clone());
    app.effect_library_draft = ControllerEffectDraft {
        reference,
        id: effect.id.clone(),
        name: effect.name.clone(),
        category: effect.category.clone(),
        description: effect.description.clone(),
        kind: "strip-stream".to_string(),
        program_json: serde_json::to_string_pretty(&effect.program).unwrap_or_default(),
        color: String::new(),
        default_fps: effect.default_fps.unwrap_or(20),
        duration_ms: effect.default_duration_ms.unwrap_or(1),
        default_pixels: effect.default_pixels.unwrap_or(1),
        is_new: false,
    };
}

pub(crate) fn select_advertised_effect(
    app: &mut PealayerApp,
    source: crate::app::EffectPresetSource,
    strip_id: Option<&str>,
) -> Option<String> {
    let capabilities = app.advertised_hardware()?;
    match source {
        crate::app::EffectPresetSource::ControllerMacro(id) => {
            let effect = capabilities.macros.iter().find(|effect| effect.id == id)?;
            let reference = format!("effect:{id}");
            select_sequence(app, effect);
            Some(reference)
        }
        crate::app::EffectPresetSource::ControllerStrip => {
            let id = strip_id?;
            let effect = capabilities
                .strip_effects
                .iter()
                .find(|effect| effect.id == id)?;
            let reference = format!("effect:{id}");
            select_strip(app, effect);
            Some(reference)
        }
    }
}

pub(crate) fn duplicate_selected(app: &mut PealayerApp) {
    let duplicate_id = if app.effect_library_draft.kind == "sequence" {
        let used = app
            .advertised_hardware()
            .map(|capabilities| {
                capabilities
                    .macros
                    .into_iter()
                    .map(|effect| effect.id)
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .unwrap_or_default();
        (0_u16..=255)
            .find(|candidate| !used.contains(&u64::from(*candidate)))
            .unwrap_or(0)
            .to_string()
    } else {
        format!("{}-copy", app.effect_library_draft.id)
    };
    app.effect_library_selection = None;
    app.effect_library_draft.reference.clear();
    app.effect_library_draft.id = duplicate_id;
    app.effect_library_draft.name = format!("{} copy", app.effect_library_draft.name);
    app.effect_library_draft.is_new = true;
    app.show_effect_library_editor = true;
}

fn draw_saved_effect_context_menu(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    name: &str,
    reference: &str,
    payload: &crate::app::EffectDragPayload,
) {
    ui.strong(crate::ui::i18n::visual_text(app.language, name));
    ui.label(egui::RichText::new(reference).monospace().weak().small());
    ui.separator();
    if ui
        .button(format!(
            "{} {}",
            crate::ui::icons::PENCIL_SIMPLE,
            app.tr("Manage")
        ))
        .clicked()
    {
        app.show_effect_library_editor = true;
        ui.close();
    }
    if ui
        .button(format!("{} {}", crate::ui::icons::PLAY, app.tr("Run now")))
        .clicked()
    {
        if let Err(error) = app.play_controller_effect(reference) {
            app.set_osd(error);
        }
        ui.close();
    }
    if ui
        .button(format!(
            "{} {}",
            crate::ui::icons::STOP_CIRCLE,
            app.tr("Stop")
        ))
        .clicked()
    {
        if let Err(error) = app.stop_controller_effect(reference) {
            app.set_osd(error);
        }
        ui.close();
    }
    if ui
        .button(format!(
            "{} {}",
            crate::ui::icons::PLUS,
            app.tr("Place at playhead")
        ))
        .clicked()
    {
        app.place_controller_effect_at_playhead(payload);
        ui.close();
    }
    ui.separator();
    if ui
        .button(format!(
            "{} {}",
            crate::ui::icons::COPY,
            app.tr("Duplicate")
        ))
        .clicked()
    {
        duplicate_selected(app);
        ui.close();
    }
    if ui
        .button(format!("{} {}", crate::ui::icons::TRASH, app.tr("Delete")))
        .clicked()
    {
        if let Err(error) = app.delete_controller_effect() {
            app.set_osd(error);
        }
        ui.close();
    }
}

fn start_new_sequence(
    app: &mut PealayerApp,
    sequences: &[crate::four_d::controller::HardwareMacro],
) {
    let next_id = (0_u16..=255)
        .find(|candidate| {
            !sequences
                .iter()
                .any(|effect| effect.id == u64::from(*candidate))
        })
        .unwrap_or(0);
    app.effect_library_selection = None;
    app.effect_library_draft = ControllerEffectDraft {
        id: next_id.to_string(),
        kind: "sequence".to_string(),
        category: "Effects".to_string(),
        color: "green".to_string(),
        is_new: true,
        ..Default::default()
    };
}

fn start_new_lighting(
    app: &mut PealayerApp,
    effects: &[crate::four_d::controller::HardwareStripEffect],
) {
    if let Some(template) = effects.first() {
        select_strip(app, template);
        app.effect_library_draft.reference.clear();
        app.effect_library_draft.id.clear();
        app.effect_library_draft.name.clear();
    } else {
        app.effect_library_draft = ControllerEffectDraft::default();
    }
    app.effect_library_selection = None;
    app.effect_library_draft.is_new = true;
}

pub fn draw_editor(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_effect_library_editor {
        return;
    }
    let mut open = app.show_effect_library_editor;
    let capabilities = app.advertised_hardware();
    let sequences = capabilities
        .as_ref()
        .map(|value| value.macros.clone())
        .unwrap_or_default();
    let strips = capabilities
        .as_ref()
        .map(|value| value.strip_effects.clone())
        .unwrap_or_default();
    let bounds = ui.ctx().content_rect().shrink(20.0);
    let max_size = egui::vec2(bounds.width().min(900.0), bounds.height().min(720.0));

    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::SPARKLE,
        app.tr("Effect properties")
    ))
    .id(egui::Id::new("controller_effect_library_dialog"))
    .open(&mut open)
    .default_size([max_size.x.min(720.0), max_size.y.min(520.0)])
    .min_size([max_size.x.min(540.0), max_size.y.min(380.0)])
    .max_size(max_size)
    .constrain_to(bounds)
    .resizable(true)
    .collapsible(false)
    .show(ui.ctx(), |ui| {
        let height = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(220.0_f32.min(ui.available_width() * 0.36), height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .button(format!("{} {}", crate::ui::icons::PLUS, app.tr("Sequence")))
                            .clicked()
                        {
                            start_new_sequence(app, &sequences);
                        }
                        if ui
                            .button(format!("{} {}", crate::ui::icons::PLUS, app.tr("Lighting")))
                            .clicked()
                        {
                            start_new_lighting(app, &strips);
                        }
                    });
                    ui.add_space(8.0);
                    egui::ScrollArea::vertical()
                        .id_salt("controller_effect_editor_list")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for effect in &sequences {
                                let reference = format!("effect:{}", effect.id);
                                let payload = crate::app::EffectDragPayload {
                                    name: effect.name.clone(),
                                    icon: String::new(),
                                    duration_ms: effect.duration_ms,
                                    target: crate::four_d::models::HardwareTarget::ControllerMacro,
                                    actions: Vec::new(),
                                    controller_macro: Some(
                                        crate::four_d::models::ControllerMacroCue {
                                            id: effect.id,
                                            mode: effect.mode.clone(),
                                        },
                                    ),
                                    controller_strip_effect: None,
                                };
                                let selected = app.effect_library_selection.as_deref()
                                    == Some(reference.as_str());
                                let response = ui.add_sized(
                                    [ui.available_width(), 44.0],
                                    egui::Button::new(format!(
                                        "{}  {}\n   {} · {}",
                                        crate::ui::icons::WAVEFORM,
                                        effect.name,
                                        effect.category,
                                        effect.mode
                                    ))
                                    .selected(selected),
                                );
                                if response.clicked() {
                                    select_sequence(app, effect);
                                }
                                show_saved_effect_context_menu(
                                    app,
                                    &response,
                                    egui::Id::new(("effect-manager-sequence-menu", effect.id)),
                                    &effect.name,
                                    &reference,
                                    &payload,
                                    |app| select_sequence(app, effect),
                                );
                            }
                            for effect in &strips {
                                let reference = format!("effect:{}", effect.id);
                                let payload = crate::app::EffectDragPayload {
                                    name: effect.name.clone(),
                                    icon: String::new(),
                                    duration_ms: effect.default_duration_ms.unwrap_or(5_000),
                                    target: crate::four_d::models::HardwareTarget::ControllerMacro,
                                    actions: Vec::new(),
                                    controller_macro: None,
                                    controller_strip_effect: Some(
                                        crate::four_d::models::ControllerStripEffectCue {
                                            id: effect.id.clone(),
                                        },
                                    ),
                                };
                                let selected = app.effect_library_selection.as_deref()
                                    == Some(reference.as_str());
                                let response = ui.add_sized(
                                    [ui.available_width(), 44.0],
                                    egui::Button::new(format!(
                                        "{}  {}\n   {} · {}",
                                        crate::ui::icons::SPARKLE,
                                        effect.name,
                                        effect.category,
                                        effect.engine
                                    ))
                                    .selected(selected),
                                );
                                if response.clicked() {
                                    select_strip(app, effect);
                                }
                                show_saved_effect_context_menu(
                                    app,
                                    &response,
                                    egui::Id::new(("effect-manager-strip-menu", &effect.id)),
                                    &effect.name,
                                    &reference,
                                    &payload,
                                    |app| select_strip(app, effect),
                                );
                            }
                        });
                },
            );
            ui.separator();
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("controller_effect_editor_details")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.heading(if app.effect_library_draft.is_new {
                                app.tr("New effect")
                            } else {
                                app.tr("Effect properties")
                            });
                            let labels = (
                                app.tr("Type"),
                                app.tr("Timed multi-peripheral sequence"),
                                app.tr("Addressable lighting"),
                                app.tr("Stable ID"),
                                app.tr("Name"),
                                app.tr("Category"),
                                app.tr("Frame rate"),
                                app.tr("LED count"),
                                app.tr("Duration"),
                                app.tr("milliseconds"),
                            );
                            let draft = &mut app.effect_library_draft;
                            egui::Grid::new("controller_effect_definition_grid")
                                .num_columns(2)
                                .spacing([14.0, 9.0])
                                .show(ui, |ui| {
                                    ui.label(&labels.0);
                                    ui.label(if draft.kind == "sequence" {
                                        &labels.1
                                    } else {
                                        &labels.2
                                    });
                                    ui.end_row();
                                    ui.label(&labels.3);
                                    ui.add_enabled(
                                        draft.is_new,
                                        egui::TextEdit::singleline(&mut draft.id)
                                            .desired_width(260.0),
                                    );
                                    ui.end_row();
                                    ui.label(&labels.4);
                                    ui.add(
                                        egui::TextEdit::singleline(&mut draft.name)
                                            .desired_width(320.0),
                                    );
                                    ui.end_row();
                                    ui.label(&labels.5);
                                    ui.add(
                                        egui::TextEdit::singleline(&mut draft.category)
                                            .desired_width(320.0),
                                    );
                                    ui.end_row();
                                    if draft.kind != "sequence" {
                                        ui.label(&labels.6);
                                        ui.add(
                                            egui::DragValue::new(&mut draft.default_fps)
                                                .range(1..=30),
                                        );
                                        ui.end_row();
                                        ui.label(&labels.7);
                                        ui.add(
                                            egui::DragValue::new(&mut draft.default_pixels)
                                                .range(1..=100),
                                        );
                                        ui.end_row();
                                    }
                                    ui.label(&labels.8);
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::DragValue::new(&mut draft.duration_ms)
                                                .range(1..=3_600_000)
                                                .speed(100.0),
                                        );
                                        ui.label(&labels.9);
                                    });
                                    ui.end_row();
                                });
                            if app.effect_library_draft.kind != "sequence" {
                                ui.add_space(8.0);
                                ui.label(app.tr("Lighting program"));
                                ui.add_sized(
                                    [ui.available_width(), 150.0],
                                    egui::TextEdit::multiline(
                                        &mut app.effect_library_draft.program_json,
                                    )
                                    .code_editor()
                                    .desired_rows(8),
                                );
                                ui.add_space(8.0);
                                ui.label(app.tr("Description"));
                                ui.add_sized(
                                    [ui.available_width(), 72.0],
                                    egui::TextEdit::multiline(
                                        &mut app.effect_library_draft.description,
                                    )
                                    .desired_rows(3),
                                );
                            }
                            ui.add_space(12.0);
                            let saved = !app.effect_library_draft.is_new;
                            let reference = app.effect_library_draft.reference.clone();
                            ui.horizontal_wrapped(|ui| {
                                if ui
                                    .button(format!(
                                        "{} {}",
                                        crate::ui::icons::FLOPPY_DISK,
                                        app.tr("Save to PCController")
                                    ))
                                    .clicked()
                                {
                                    if let Err(error) = app.save_controller_effect() {
                                        app.set_osd(error);
                                    }
                                }
                                if ui
                                    .add_enabled(
                                        saved,
                                        egui::Button::new(format!(
                                            "{} {}",
                                            crate::ui::icons::PLAY,
                                            app.tr("Run")
                                        )),
                                    )
                                    .clicked()
                                {
                                    if let Err(error) = app.play_controller_effect(&reference) {
                                        app.set_osd(error);
                                    }
                                }
                                if ui
                                    .add_enabled(
                                        saved,
                                        egui::Button::new(format!(
                                            "{} {}",
                                            crate::ui::icons::COPY,
                                            app.tr("Duplicate")
                                        )),
                                    )
                                    .clicked()
                                {
                                    duplicate_selected(app);
                                }
                                if ui
                                    .add_enabled(
                                        saved,
                                        egui::Button::new(format!(
                                            "{} {}",
                                            crate::ui::icons::TRASH,
                                            app.tr("Delete")
                                        )),
                                    )
                                    .clicked()
                                {
                                    if let Err(error) = app.delete_controller_effect() {
                                        app.set_osd(error);
                                    }
                                }
                            });
                            if !app.hardware_effect_authoring.status.is_empty() {
                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new(&app.hardware_effect_authoring.status)
                                        .weak()
                                        .small(),
                                );
                            }
                        });
                },
            );
        });
    });
    app.show_effect_library_editor = open;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_catalog_creates_no_pealayer_owned_lighting_definition() {
        let mut app = PealayerApp::default();
        start_new_lighting(&mut app, &[]);
        assert!(app.effect_library_draft.is_new);
        assert_eq!(app.effect_library_draft.kind, "strip-stream");
        assert!(app.effect_library_draft.program_json.is_empty());
        assert!(app.effect_library_selection.is_none());
    }
}
