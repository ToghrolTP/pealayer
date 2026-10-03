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
        icon: effect.icon.clone(),
        description: String::new(),
        kind: "sequence".to_string(),
        program_json: String::new(),
        color: "green".to_string(),
        default_fps: 0,
        duration_ms: effect.duration_ms,
        default_pixels: 0,
        engine: effect.mode.clone(),
        steps: effect.steps.clone(),
        label: effect.label.clone(),
        lcd_message: effect.lcd_message.clone(),
        timing_tolerance_us: effect.timing_tolerance_us,
        keep_outputs_on_cancel: effect.keep_outputs_on_cancel,
        board_profile_key: effect.board_profile_key.clone(),
        board_profile_mode: effect.board_profile_mode.clone(),
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
        icon: effect.icon.clone(),
        description: effect.description.clone(),
        kind: "strip-stream".to_string(),
        program_json: serde_json::to_string_pretty(&effect.program).unwrap_or_default(),
        color: String::new(),
        default_fps: effect.default_fps.unwrap_or(20),
        duration_ms: effect.default_duration_ms.unwrap_or(1),
        default_pixels: effect.default_pixels.unwrap_or(1),
        engine: effect.engine.clone(),
        steps: Vec::new(),
        label: String::new(),
        lcd_message: String::new(),
        timing_tolerance_us: 0,
        keep_outputs_on_cancel: false,
        board_profile_key: String::new(),
        board_profile_mode: String::new(),
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

pub(crate) fn begin_new_effect(app: &mut PealayerApp, category: Option<String>) {
    let sequences = app
        .advertised_hardware()
        .map(|capabilities| capabilities.macros)
        .unwrap_or_default();
    start_new_sequence(app, &sequences);
    if let Some(category) = category.filter(|value| !value.trim().is_empty()) {
        app.effect_library_draft.category = category;
    }
    app.show_effect_library_editor = true;
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

fn effect_navigation_button(
    ui: &mut egui::Ui,
    selected: bool,
    icon: &str,
    title: &str,
    metadata: &str,
) -> egui::Response {
    crate::ui::dialog::navigation_detail_button(
        ui,
        selected,
        icon,
        title,
        metadata,
        ui.available_width(),
    )
}

const SEQUENCE_STEP_KINDS: &[&str] = &[
    "relay-mask",
    "relay",
    "pwm",
    "display",
    "rf",
    "beep",
    "rgb",
    "relays-off",
    "opcode",
];

fn reset_step_kind(step: &mut crate::four_d::controller::HardwareMacroStep, kind: String) {
    let at_us = step.at_us;
    *step = crate::four_d::controller::HardwareMacroStep {
        at_us,
        kind,
        ..Default::default()
    };
    match step.kind.as_str() {
        "relay" => {
            step.target = Some(1);
            step.value = Some(1);
        }
        "relay-mask" => step.value = Some(0),
        "pwm" => {
            step.target = Some(0);
            step.value = Some(0);
        }
        "display" => {
            step.destination = "segments".to_string();
            step.duration_ms = Some(1_500);
        }
        "rf" => {
            step.bits = Some(24);
            step.protocol = Some(1);
            step.pulse_us = Some(350);
        }
        "beep" => {
            step.frequency_hz = Some(1_000);
            step.duration_ms = Some(120);
        }
        "rgb" => {
            step.red = Some(255);
            step.green = Some(255);
            step.blue = Some(255);
            step.brightness = Some(255);
        }
        _ => {}
    }
}

fn sequence_duration_ms(steps: &[crate::four_d::controller::HardwareMacroStep]) -> u64 {
    steps
        .iter()
        .map(|step| step.at_us.div_ceil(1_000) + u64::from(step.duration_ms.unwrap_or_default()))
        .max()
        .unwrap_or(1)
        .max(1)
}

fn draw_sequence_step_editor(ui: &mut egui::Ui, draft: &mut ControllerEffectDraft) {
    draft.duration_ms = sequence_duration_ms(&draft.steps);
    ui.horizontal(|ui| {
        ui.heading("Sequence steps");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(format!("{} Add step", crate::ui::icons::PLUS))
                .clicked()
            {
                let at_us = draft
                    .steps
                    .last()
                    .map(|step| step.at_us.saturating_add(100_000))
                    .unwrap_or(0);
                let mut step = crate::four_d::controller::HardwareMacroStep {
                    at_us,
                    ..Default::default()
                };
                reset_step_kind(&mut step, "relay-mask".to_string());
                draft.steps.push(step);
            }
            ui.label(
                egui::RichText::new(format!(
                    "{} · {}",
                    draft.steps.len(),
                    crate::duration::format_effect_duration_for_language(
                        crate::config::AppLanguage::English,
                        draft.duration_ms,
                    )
                ))
                .small()
                .weak(),
            );
        });
    });
    ui.add_space(6.0);

    if draft.steps.is_empty() {
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.label(egui::RichText::new("This sequence has no actions yet.").strong());
                ui.label(
                    egui::RichText::new(
                        "Add a step, choose the peripheral command, then set its exact time and parameters.",
                    )
                    .weak(),
                );
            });
        return;
    }

    let mut move_step = None;
    let mut remove_step = None;
    for index in 0..draft.steps.len() {
        let step_count = draft.steps.len();
        let step = &mut draft.steps[index];
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(format!("Step {}", index + 1));
                    ui.label(egui::RichText::new(&step.kind).monospace().weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(crate::ui::icons::TRASH)
                            .on_hover_text("Remove step")
                            .clicked()
                        {
                            remove_step = Some(index);
                        }
                        if ui
                            .add_enabled(
                                index + 1 < step_count,
                                egui::Button::new(crate::ui::icons::ARROW_DOWN),
                            )
                            .on_hover_text("Move down")
                            .clicked()
                        {
                            move_step = Some((index, index + 1));
                        }
                        if ui
                            .add_enabled(index > 0, egui::Button::new(crate::ui::icons::ARROW_UP))
                            .on_hover_text("Move up")
                            .clicked()
                        {
                            move_step = Some((index, index - 1));
                        }
                    });
                });
                ui.separator();
                egui::Grid::new(("effect-sequence-step", index))
                    .num_columns(2)
                    .spacing([14.0, 7.0])
                    .show(ui, |ui| {
                        ui.label("Command");
                        let previous_kind = step.kind.clone();
                        egui::ComboBox::from_id_salt(("sequence-step-kind", index))
                            .selected_text(if step.kind.is_empty() {
                                "Choose…"
                            } else {
                                &step.kind
                            })
                            .show_ui(ui, |ui| {
                                for kind in SEQUENCE_STEP_KINDS {
                                    ui.selectable_value(&mut step.kind, (*kind).to_string(), *kind);
                                }
                            });
                        if step.kind != previous_kind {
                            reset_step_kind(step, step.kind.clone());
                        }
                        ui.end_row();

                        ui.label("Time");
                        let mut at_ms = step.at_us as f64 / 1_000.0;
                        if ui
                            .add(
                                egui::DragValue::new(&mut at_ms)
                                    .range(0.0..=3_600_000.0)
                                    .speed(1.0)
                                    .suffix(" ms"),
                            )
                            .changed()
                        {
                            step.at_us = (at_ms.max(0.0) * 1_000.0).round() as u64;
                        }
                        ui.end_row();

                        match step.kind.as_str() {
                            "relay" => {
                                ui.label("Relay");
                                ui.add(
                                    egui::DragValue::new(step.target.get_or_insert(1))
                                        .range(1..=255),
                                );
                                ui.end_row();
                                ui.label("State");
                                let value = step.value.get_or_insert(0);
                                egui::ComboBox::from_id_salt(("relay-state", index))
                                    .selected_text(if *value == 0 { "Off" } else { "On" })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(value, 0, "Off");
                                        ui.selectable_value(value, 1, "On");
                                    });
                                ui.end_row();
                            }
                            "relay-mask" => {
                                ui.label("Relay mask");
                                ui.add(
                                    egui::DragValue::new(step.value.get_or_insert(0))
                                        .range(0..=255),
                                );
                                ui.end_row();
                            }
                            "pwm" => {
                                ui.label("PWM channel");
                                ui.add(
                                    egui::DragValue::new(step.target.get_or_insert(0))
                                        .range(0..=255),
                                );
                                ui.end_row();
                                ui.label("Value");
                                ui.add(
                                    egui::DragValue::new(step.value.get_or_insert(0))
                                        .range(0..=4095),
                                );
                                ui.end_row();
                            }
                            "display" => {
                                ui.label("Display");
                                egui::ComboBox::from_id_salt(("display-destination", index))
                                    .selected_text(if step.destination.is_empty() {
                                        "Segments"
                                    } else {
                                        &step.destination
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut step.destination,
                                            "segments".to_string(),
                                            "Segments",
                                        );
                                        ui.selectable_value(
                                            &mut step.destination,
                                            "lcd".to_string(),
                                            "LCD",
                                        );
                                    });
                                ui.end_row();
                                ui.label("Text");
                                ui.text_edit_singleline(&mut step.text);
                                ui.end_row();
                                ui.label("Visible for");
                                ui.add(
                                    egui::DragValue::new(step.duration_ms.get_or_insert(1_500))
                                        .range(1..=65_535)
                                        .suffix(" ms"),
                                );
                                ui.end_row();
                            }
                            "rf" => {
                                ui.label("RF code");
                                ui.add(
                                    egui::DragValue::new(step.code.get_or_insert(0))
                                        .range(0..=u32::MAX),
                                );
                                ui.end_row();
                                ui.label("Bits / protocol");
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::DragValue::new(step.bits.get_or_insert(24))
                                            .range(1..=64),
                                    );
                                    ui.add(
                                        egui::DragValue::new(step.protocol.get_or_insert(1))
                                            .range(1..=255),
                                    );
                                });
                                ui.end_row();
                                ui.label("Pulse");
                                ui.add(
                                    egui::DragValue::new(step.pulse_us.get_or_insert(350))
                                        .range(1..=65_535)
                                        .suffix(" µs"),
                                );
                                ui.end_row();
                            }
                            "beep" => {
                                ui.label("Frequency");
                                ui.add(
                                    egui::DragValue::new(step.frequency_hz.get_or_insert(1_000))
                                        .range(1..=20_000)
                                        .suffix(" Hz"),
                                );
                                ui.end_row();
                                ui.label("Duration");
                                ui.add(
                                    egui::DragValue::new(step.duration_ms.get_or_insert(120))
                                        .range(1..=65_535)
                                        .suffix(" ms"),
                                );
                                ui.end_row();
                            }
                            "rgb" => {
                                ui.label("Color");
                                let mut color = egui::Color32::from_rgb(
                                    step.red.unwrap_or_default(),
                                    step.green.unwrap_or_default(),
                                    step.blue.unwrap_or_default(),
                                );
                                if ui.color_edit_button_srgba(&mut color).changed() {
                                    step.red = Some(color.r());
                                    step.green = Some(color.g());
                                    step.blue = Some(color.b());
                                }
                                ui.end_row();
                                ui.label("Brightness");
                                ui.add(
                                    egui::DragValue::new(step.brightness.get_or_insert(255))
                                        .range(0..=255),
                                );
                                ui.end_row();
                            }
                            "opcode" => {
                                ui.label("Opcode");
                                ui.add(
                                    egui::DragValue::new(step.opcode.get_or_insert(0))
                                        .range(0..=255),
                                );
                                ui.end_row();
                                ui.label("Payload (hex)");
                                ui.add(
                                    egui::TextEdit::singleline(&mut step.payload_hex)
                                        .desired_width(280.0)
                                        .font(egui::TextStyle::Monospace),
                                );
                                ui.end_row();
                                ui.label("Description");
                                ui.text_edit_singleline(&mut step.text);
                                ui.end_row();
                            }
                            _ => {}
                        }
                    });

                ui.add_space(5.0);
                ui.label(egui::RichText::new("Semantic actions").small().strong());
                let mut remove_action = None;
                for (action_index, action) in step.action_ids.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(action)
                                .desired_width(240.0)
                                .hint_text("seat.a.up"),
                        );
                        if ui.small_button(crate::ui::icons::X).clicked() {
                            remove_action = Some(action_index);
                        }
                    });
                }
                if let Some(action_index) = remove_action {
                    step.action_ids.remove(action_index);
                }
                if ui
                    .small_button(format!("{} Add semantic action", crate::ui::icons::PLUS))
                    .clicked()
                {
                    step.action_ids.push(String::new());
                }
            });
        ui.add_space(7.0);
    }
    if let Some((from, to)) = move_step {
        draft.steps.swap(from, to);
    }
    if let Some(index) = remove_step {
        draft.steps.remove(index);
    }
    draft.duration_ms = sequence_duration_ms(&draft.steps);
}

pub fn draw_editor(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_effect_library_editor {
        return;
    }
    let mut open = app.show_effect_library_editor;
    let display_language = app.language;
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
                                    controller_lane: Some(crate::app::controller_macro_lane(effect)),
                                };
                                let selected = app.effect_library_selection.as_deref()
                                    == Some(reference.as_str());
                                let response = effect_navigation_button(
                                    ui,
                                    selected,
                                    crate::ui::icons::WAVEFORM,
                                    &crate::ui::i18n::visual_text(app.language, &effect.name),
                                    &format!(
                                        "{} · {} · {}",
                                        effect.category,
                                        effect.mode,
                                        crate::duration::format_effect_duration_for_language(
                                            display_language,
                                            effect.duration_ms
                                        )
                                    ),
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
                                    controller_lane: Some(
                                        crate::four_d::models::ControllerEffectLane::Lighting,
                                    ),
                                };
                                let selected = app.effect_library_selection.as_deref()
                                    == Some(reference.as_str());
                                let response = effect_navigation_button(
                                    ui,
                                    selected,
                                    crate::ui::icons::SPARKLE,
                                    &crate::ui::i18n::visual_text(app.language, &effect.name),
                                    &format!(
                                        "{} · {} · {}",
                                        effect.category,
                                        effect.engine,
                                        crate::duration::format_effect_duration_for_language(
                                            display_language,
                                            effect.default_duration_ms.unwrap_or_default()
                                        )
                                    ),
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
                                app.tr("Icon"),
                                app.tr("Frame rate"),
                                app.tr("LED count"),
                                app.tr("Duration"),
                                app.tr("Execution"),
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
                                    ui.label(&labels.6);
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            crate::ui::icons::named_control_icon(&draft.icon)
                                                .unwrap_or(crate::ui::icons::SPARKLE),
                                        );
                                        ui.add(
                                            egui::TextEdit::singleline(&mut draft.icon)
                                                .desired_width(150.0)
                                                .hint_text("sparkle"),
                                        );
                                        egui::ComboBox::from_id_salt("effect_icon_preset")
                                            .selected_text("Presets")
                                            .show_ui(ui, |ui| {
                                                for (key, label, glyph) in
                                                    crate::ui::icons::CONTROL_ICON_PRESETS
                                                {
                                                    if ui
                                                        .selectable_label(
                                                            draft.icon.eq_ignore_ascii_case(key),
                                                            format!("{glyph}  {label}"),
                                                        )
                                                        .clicked()
                                                    {
                                                        draft.icon = (*key).to_string();
                                                    }
                                                }
                                            });
                                    });
                                    ui.end_row();
                                    if draft.kind != "sequence" {
                                        ui.label(&labels.7);
                                        ui.add(
                                            egui::DragValue::new(&mut draft.default_fps)
                                                .range(1..=30),
                                        );
                                        ui.end_row();
                                        ui.label(&labels.8);
                                        ui.add(
                                            egui::DragValue::new(&mut draft.default_pixels)
                                                .range(1..=100),
                                        );
                                        ui.end_row();
                                    }
                                    ui.label(&labels.9);
                                    if draft.kind == "sequence" {
                                        ui.label(
                                            egui::RichText::new(
                                                crate::duration::format_effect_duration_for_language(
                                                    display_language,
                                                    sequence_duration_ms(&draft.steps),
                                                ),
                                            )
                                            .strong(),
                                        );
                                    } else {
                                        ui.add(crate::duration::time_value_drag(
                                            &mut draft.duration_ms,
                                            1..=3_600_000,
                                            100.0,
                                        ));
                                    }
                                    ui.end_row();
                                    if draft.kind == "sequence" {
                                        ui.label(&labels.10);
                                        egui::ComboBox::from_id_salt("effect_sequence_engine")
                                            .selected_text(if draft.engine == "mcu" {
                                                "Board clock"
                                            } else {
                                                "Host scheduler"
                                            })
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(
                                                    &mut draft.engine,
                                                    "host".to_string(),
                                                    "Host scheduler",
                                                );
                                                ui.selectable_value(
                                                    &mut draft.engine,
                                                    "mcu".to_string(),
                                                    "Board clock",
                                                );
                                            });
                                        ui.end_row();
                                    }
                                });
                            if app.effect_library_draft.kind == "sequence" {
                                ui.add_space(12.0);
                                draw_sequence_step_editor(ui, &mut app.effect_library_draft);
                                ui.add_space(8.0);
                                if crate::ui::icons::disclosure_header(
                                    ui,
                                    "effect_sequence_properties",
                                    "Sequence properties",
                                    false,
                                ) {
                                    egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                                        egui::Grid::new("effect_sequence_properties_grid")
                                            .num_columns(2)
                                            .spacing([14.0, 8.0])
                                            .show(ui, |ui| {
                                                ui.label("Timing tolerance");
                                                ui.add(
                                                    egui::DragValue::new(
                                                        &mut app.effect_library_draft
                                                            .timing_tolerance_us,
                                                    )
                                                    .range(0..=5_000_000)
                                                    .suffix(" µs"),
                                                );
                                                ui.end_row();
                                                ui.label("Keep outputs on cancel");
                                                ui.checkbox(
                                                    &mut app.effect_library_draft
                                                        .keep_outputs_on_cancel,
                                                    "",
                                                );
                                                ui.end_row();
                                                ui.label("Board profile");
                                                ui.text_edit_singleline(
                                                    &mut app.effect_library_draft.board_profile_key,
                                                );
                                                ui.end_row();
                                                ui.label("Board mode");
                                                ui.text_edit_singleline(
                                                    &mut app.effect_library_draft
                                                        .board_profile_mode,
                                                );
                                                ui.end_row();
                                                ui.label("Label");
                                                ui.text_edit_singleline(
                                                    &mut app.effect_library_draft.label,
                                                );
                                                ui.end_row();
                                                ui.label("LCD message");
                                                ui.text_edit_singleline(
                                                    &mut app.effect_library_draft.lcd_message,
                                                );
                                                ui.end_row();
                                            });
                                    });
                                }
                            } else {
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

    #[test]
    fn new_effect_starts_as_an_empty_editable_sequence() {
        let mut app = PealayerApp::default();
        begin_new_effect(&mut app, Some("Motion".to_string()));

        assert!(app.show_effect_library_editor);
        assert!(app.effect_library_draft.is_new);
        assert_eq!(app.effect_library_draft.kind, "sequence");
        assert_eq!(app.effect_library_draft.category, "Motion");
        assert!(app.effect_library_draft.steps.is_empty());
        assert_eq!(app.effect_library_draft.engine, "host");
    }

    #[test]
    fn sequence_duration_includes_each_steps_active_duration() {
        let steps = vec![
            crate::four_d::controller::HardwareMacroStep {
                at_us: 250_000,
                duration_ms: Some(500),
                ..Default::default()
            },
            crate::four_d::controller::HardwareMacroStep {
                at_us: 900_500,
                duration_ms: Some(100),
                ..Default::default()
            },
        ];

        assert_eq!(sequence_duration_ms(&steps), 1_001);
    }
}
