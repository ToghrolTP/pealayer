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

pub(crate) fn saved_effect_preview_action_presentation(
    app: &mut PealayerApp,
    reference: &str,
) -> (
    crate::app::ControllerEffectPreviewPhase,
    &'static str,
    String,
    bool,
) {
    let phase = app.controller_effect_preview_phase(reference);
    let (icon, label) = match phase {
        crate::app::ControllerEffectPreviewPhase::Run => {
            (crate::ui::icons::PLAY, app.tr("Run now"))
        }
        crate::app::ControllerEffectPreviewPhase::Starting => {
            (crate::ui::icons::PLAY, app.tr("Starting…"))
        }
        crate::app::ControllerEffectPreviewPhase::Stop => {
            (crate::ui::icons::STOP_CIRCLE, app.tr("Stop"))
        }
        crate::app::ControllerEffectPreviewPhase::Stopping => {
            (crate::ui::icons::STOP_CIRCLE, app.tr("Stopping…"))
        }
    };
    let enabled = app.controller_effect_preview_action_enabled(phase);
    (phase, icon, label, enabled)
}

pub(crate) fn draw_saved_effect_preview_action(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    reference: &str,
) {
    let (_, icon, label, enabled) = saved_effect_preview_action_presentation(app, reference);
    if ui
        .add_enabled(enabled, egui::Button::new(format!("{icon} {label}")))
        .clicked()
    {
        if let Err(error) = app.invoke_controller_effect_preview_action(reference) {
            app.set_osd(error);
        }
        ui.close();
    }
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
        color: effect.color.clone(),
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

pub(crate) fn save_advertised_effect_identity(
    app: &mut PealayerApp,
    source: crate::app::EffectPresetSource,
    strip_id: Option<&str>,
    name: String,
    icon: String,
) -> Result<(), String> {
    select_advertised_effect(app, source, strip_id)
        .ok_or_else(|| app.tr("Effect is no longer available"))?;
    app.effect_library_draft.name = name;
    app.effect_library_draft.icon = icon;
    app.save_controller_effect()
}

pub(crate) fn move_dragged_effect_to_group(
    app: &mut PealayerApp,
    payload: &crate::app::EffectDragPayload,
    category: String,
) -> Result<(), String> {
    let category = category.trim();
    if category.is_empty() || category.len() > 64 || category.chars().any(char::is_control) {
        return Err("Effect group must be a bounded printable value".to_string());
    }
    let selected = if let Some(effect) = payload.controller_macro.as_ref() {
        select_advertised_effect(
            app,
            crate::app::EffectPresetSource::ControllerMacro(effect.id),
            None,
        )
    } else if let Some(effect) = payload.controller_strip_effect.as_ref() {
        select_advertised_effect(
            app,
            crate::app::EffectPresetSource::ControllerStrip,
            Some(effect.id.as_str()),
        )
    } else {
        None
    };
    selected.ok_or_else(|| app.tr("Effect is no longer available"))?;
    if app.effect_library_draft.category == category {
        return Ok(());
    }
    app.effect_library_draft.category = category.to_string();
    app.save_controller_effect()
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
    draw_saved_effect_preview_action(app, ui, reference);
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

pub(crate) fn begin_new_group(app: &mut PealayerApp) {
    app.effect_group_draft = Some(crate::app::ControllerEffectGroupDraft {
        original_name: String::new(),
        name: String::new(),
        icon: String::new(),
    });
}

pub(crate) fn empty_library_context_menu(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
) -> egui::Response {
    let response = ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::click());
    response.context_menu(|ui| {
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::PLUS,
                app.tr("New effect")
            ))
            .clicked()
        {
            begin_new_effect(app, None);
            ui.close();
        }
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::FOLDER_OPEN,
                app.tr("New group")
            ))
            .clicked()
        {
            begin_new_group(app);
            ui.close();
        }
    });
    response
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

fn sequence_step_kinds(
    capabilities: Option<&crate::four_d::controller::HardwareCapabilities>,
) -> Vec<&'static str> {
    let Some(capabilities) = capabilities.filter(|value| value.board_connected) else {
        return Vec::new();
    };
    let mut kinds = Vec::new();
    if !capabilities.relays.is_empty() {
        kinds.extend(["motion", "relay", "relay-mask", "relays-off"]);
    }
    if !capabilities.pwm_channels.is_empty() {
        kinds.extend(["pwm", "pwm-off"]);
    }
    if capabilities.supports_segment_display || capabilities.supports_lcd_display {
        kinds.push("display");
    }
    if capabilities.supports_rf_transmit {
        kinds.push("rf");
    }
    // Buzzer and status-RGB push support are advertised independently of the
    // peripheral channel list.
    if capabilities.capability_bits & (1 << 27) != 0 {
        kinds.push("beep");
    }
    if capabilities.supports_status_led_settings {
        kinds.push("rgb");
    }
    if capabilities.supports_addressable_led {
        kinds.push("addressable");
    }
    if capabilities.front_panel.is_some() {
        kinds.extend(["menu", "menu-action"]);
    }
    // Raw acknowledged commands remain available for expert diagnostics, but
    // are intentionally last and never stand in for missing capability data.
    kinds.push("opcode");
    kinds
}

fn reset_step_kind(step: &mut crate::four_d::controller::HardwareMacroStep, kind: String) {
    let at_us = step.at_us;
    *step = crate::four_d::controller::HardwareMacroStep {
        at_us,
        kind,
        ..Default::default()
    };
    match step.kind.as_str() {
        "motion" => {
            step.target = Some(0);
            step.value = Some(1);
            step.duration_ms = Some(500);
            step.action_ids = vec!["seat.a.up".to_string()];
        }
        "relay" => {
            step.target = Some(0);
            step.value = Some(1);
            step.duration_ms = Some(500);
        }
        "relay-mask" => step.value = Some(0),
        "pwm" => {
            step.target = Some(0);
            step.value = Some(0);
            step.duration_ms = Some(1_000);
            step.easing = "linear".to_string();
            step.sample_rate_hz = Some(30);
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
            step.duration_ms = Some(1_000);
            step.easing = "linear".to_string();
            step.sample_rate_hz = Some(30);
        }
        "addressable" => {
            step.target = Some(0);
            step.red = Some(255);
            step.green = Some(255);
            step.blue = Some(255);
            step.brightness = Some(255);
            step.duration_ms = Some(1_000);
            step.easing = "linear".to_string();
            step.sample_rate_hz = Some(30);
        }
        "menu" | "menu-action" => step.target = Some(0),
        _ => {}
    }
}

fn refresh_semantic_action(step: &mut crate::four_d::controller::HardwareMacroStep) {
    let action = match step.kind.as_str() {
        "motion" => {
            let side = if step.target.unwrap_or_default() == 0 {
                "a"
            } else {
                "b"
            };
            let verb = match step.value.unwrap_or_default() {
                1 => "up",
                2 => "down",
                _ => "stop",
            };
            Some(format!("seat.{side}.{verb}"))
        }
        "relay" => {
            let target = step.target.unwrap_or_default().saturating_add(1);
            let verb = if step.value.unwrap_or_default() == 0 {
                "off"
            } else {
                "on"
            };
            Some(format!("relay.{target}.{verb}"))
        }
        _ => None,
    };
    if let Some(action) = action {
        step.action_ids.clear();
        step.action_ids.push(action);
    }
}

pub(crate) fn sequence_duration_ms(steps: &[crate::four_d::controller::HardwareMacroStep]) -> u64 {
    steps
        .iter()
        .map(|step| {
            let duration = u64::from(step.duration_ms.unwrap_or_default());
            let repeats = u64::from(step.repeat_count.unwrap_or(1).max(1));
            let interval = u64::from(
                step.repeat_interval_ms
                    .unwrap_or_else(|| u32::from(step.duration_ms.unwrap_or(1)).max(1)),
            );
            step.at_us.div_ceil(1_000)
                + repeats.saturating_sub(1).saturating_mul(interval)
                + duration
        })
        .max()
        .unwrap_or(1)
        .max(1)
}

fn append_melody_steps(
    steps: &mut Vec<crate::four_d::controller::HardwareMacroStep>,
    melody: &crate::four_d::controller::HardwareMelody,
) -> Option<usize> {
    let first = steps.len();
    let base_us = if steps.is_empty() {
        0
    } else {
        sequence_duration_ms(steps).saturating_mul(1_000)
    };
    let mut offset_ms = 0_u64;
    for note in &melody.notes {
        steps.push(crate::four_d::controller::HardwareMacroStep {
            at_us: base_us.saturating_add(offset_ms.saturating_mul(1_000)),
            kind: "beep".to_string(),
            frequency_hz: Some(note.frequency_hz),
            duration_ms: Some(note.duration_ms),
            ..Default::default()
        });
        offset_ms = offset_ms.saturating_add(u64::from(note.duration_ms));
        if note.gap_ms > 0 {
            steps.push(crate::four_d::controller::HardwareMacroStep {
                at_us: base_us.saturating_add(offset_ms.saturating_mul(1_000)),
                kind: "beep".to_string(),
                frequency_hz: Some(0),
                duration_ms: Some(note.gap_ms),
                ..Default::default()
            });
            offset_ms = offset_ms.saturating_add(u64::from(note.gap_ms));
        }
    }
    (steps.len() > first).then_some(first)
}

#[derive(Clone, Copy, Debug)]
struct SequenceCueDragState {
    index: usize,
    mode: crate::app::DragMode,
    pointer_x: f32,
    start_ms: u64,
    duration_ms: u64,
}

fn sequence_lane(
    step: &crate::four_d::controller::HardwareMacroStep,
) -> (String, String, &'static str) {
    let target = step.target.unwrap_or_default();
    match step.kind.as_str() {
        "motion" => {
            let side = if target == 0 { "A" } else { "B" };
            (
                format!("motion.{target}"),
                format!("Seat {side}"),
                crate::ui::icons::SEAT,
            )
        }
        "relay" => (
            format!("relay.{target}"),
            format!("R{}", target.saturating_add(1)),
            crate::ui::icons::PLUG,
        ),
        "pwm" => (
            format!("pwm.{target}"),
            format!("P{}", target.saturating_add(1)),
            crate::ui::icons::LIGHTBULB,
        ),
        "rgb" => (
            "status-rgb".to_string(),
            "Status RGB".to_string(),
            crate::ui::icons::PALETTE,
        ),
        "addressable" => (
            format!("strip.{target}"),
            format!("Strip pixel {}", target.saturating_add(1)),
            crate::ui::icons::SPARKLE,
        ),
        "display" => (
            format!("display.{}", step.destination),
            "Display".to_string(),
            crate::ui::icons::MONITOR_PLAY,
        ),
        "rf" => (
            "rf".to_string(),
            "Radio".to_string(),
            crate::ui::icons::RADIO,
        ),
        "beep" => (
            "buzzer".to_string(),
            "Buzzer".to_string(),
            crate::ui::icons::SPEAKER_HIGH,
        ),
        other => (
            other.to_string(),
            other.replace('-', " "),
            crate::ui::icons::CIRCUITRY,
        ),
    }
}

fn sequence_lane_index_at_pointer(
    pointer_y: f32,
    rows_top: f32,
    row_height: f32,
    lane_count: usize,
) -> Option<usize> {
    let relative_y = pointer_y - rows_top;
    if relative_y < 0.0 || relative_y >= row_height * lane_count as f32 {
        return None;
    }
    Some((relative_y / row_height).floor() as usize)
}

/// Retargets a cue to another compatible timeline lane while preserving its
/// timing and command value. Incompatible lanes are deliberately ignored so a
/// vertical move cannot silently turn a seat action into a different command.
fn retarget_sequence_cue_to_lane(
    step: &mut crate::four_d::controller::HardwareMacroStep,
    lane_key: &str,
) -> bool {
    let changed = if step.kind == "display" {
        let Some(destination) = lane_key.strip_prefix("display.") else {
            return false;
        };
        if step.destination == destination {
            false
        } else {
            step.destination = destination.to_string();
            true
        }
    } else {
        let prefix = match step.kind.as_str() {
            "motion" => "motion.",
            "relay" => "relay.",
            "pwm" => "pwm.",
            "addressable" => "strip.",
            _ => return false,
        };
        let Some(target) = lane_key
            .strip_prefix(prefix)
            .and_then(|target| target.parse::<u8>().ok())
        else {
            return false;
        };
        if step.target == Some(target) {
            false
        } else {
            step.target = Some(target);
            true
        }
    };
    if changed {
        refresh_semantic_action(step);
    }
    changed
}

fn sequence_cue_label(step: &crate::four_d::controller::HardwareMacroStep) -> String {
    match step.kind.as_str() {
        "motion" => match step.value.unwrap_or_default() {
            1 => "Up".to_string(),
            2 => "Down".to_string(),
            _ => "Stop".to_string(),
        },
        "relay" => {
            if step.value.unwrap_or_default() == 0 {
                "Off".to_string()
            } else {
                "On".to_string()
            }
        }
        "pwm" => match step.to_value {
            Some(end) => format!("{} → {end}", step.value.unwrap_or_default()),
            None => format!("{}", step.value.unwrap_or_default()),
        },
        "rgb" | "addressable" => format!(
            "#{:02X}{:02X}{:02X}",
            step.red.unwrap_or_default(),
            step.green.unwrap_or_default(),
            step.blue.unwrap_or_default()
        ),
        "display" => step.text.clone(),
        "rf" => format!("0x{:X}", step.code.unwrap_or_default()),
        "beep" if step.frequency_hz.unwrap_or_default() == 0 => "Rest".to_string(),
        "beep" => format!("{} Hz", step.frequency_hz.unwrap_or_default()),
        _ => step.kind.clone(),
    }
}

fn quantize_time(value_ms: i64, quantum_ms: u64) -> u64 {
    let quantum = quantum_ms.max(1) as i64;
    let value = value_ms.max(0);
    ((value + quantum / 2) / quantum * quantum) as u64
}

fn strip_sequence_leading_delay(steps: &mut [crate::four_d::controller::HardwareMacroStep]) {
    let Some(first) = steps.iter().map(|step| step.at_us).min() else {
        return;
    };
    for step in steps {
        step.at_us = step.at_us.saturating_sub(first);
    }
}

fn quantize_sequence(steps: &mut [crate::four_d::controller::HardwareMacroStep], quantum_ms: u64) {
    for step in steps {
        step.at_us = quantize_time(step.at_us.div_ceil(1_000) as i64, quantum_ms) * 1_000;
        if let Some(duration) = &mut step.duration_ms {
            *duration = quantize_time(i64::from(*duration), quantum_ms)
                .max(1)
                .min(u64::from(u16::MAX)) as u16;
        }
        if let Some(interval) = &mut step.repeat_interval_ms {
            *interval = quantize_time(i64::from(*interval), quantum_ms)
                .max(1)
                .min(u64::from(u32::MAX)) as u32;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SequenceCueMenuAction {
    Edit,
    Duplicate,
    Delete,
}

fn sequence_cue_value_options(kind: &str) -> &'static [(u16, &'static str)] {
    match kind {
        "motion" => &[(1, "Up"), (2, "Down"), (0, "Stop")],
        "relay" => &[(1, "On"), (0, "Off")],
        _ => &[],
    }
}

/// Changes only the authoring draft. No preview or hardware command is sent.
fn sequence_cue_context_menu(
    ui: &mut egui::Ui,
    step: &mut crate::four_d::controller::HardwareMacroStep,
    quantum_ms: u64,
    human_readable_time_units: bool,
) -> Option<SequenceCueMenuAction> {
    let mut action = None;
    ui.strong(format!(
        "{} · {}",
        sequence_lane(step).1,
        sequence_cue_label(step)
    ));
    ui.separator();
    if ui
        .button(format!("{} Edit cue…", crate::ui::icons::PENCIL_SIMPLE))
        .clicked()
    {
        action = Some(SequenceCueMenuAction::Edit);
        ui.close();
    }
    let options = sequence_cue_value_options(&step.kind);
    if !options.is_empty() {
        ui.menu_button(format!("{} Action", crate::ui::icons::LIGHTNING), |ui| {
            for &(value, caption) in options {
                if ui
                    .selectable_label(step.value.unwrap_or_default() == value, caption)
                    .clicked()
                {
                    step.value = Some(value);
                    refresh_semantic_action(step);
                    ui.close();
                }
            }
        });
    } else if step.kind == "pwm" {
        ui.menu_button(
            format!("{} Intensity", crate::ui::icons::SLIDERS_HORIZONTAL),
            |ui| {
                let mut percent = f64::from(step.value.unwrap_or_default()) * 100.0 / 4095.0;
                if ui
                    .add(egui::Slider::new(&mut percent, 0.0..=100.0).suffix("%"))
                    .changed()
                {
                    step.value = Some((percent * 4095.0 / 100.0).round() as u16);
                }
            },
        );
    } else if matches!(step.kind.as_str(), "rgb" | "addressable") {
        ui.menu_button(format!("{} Color", crate::ui::icons::PALETTE), |ui| {
            let mut color = egui::Color32::from_rgb(
                step.red.unwrap_or_default(),
                step.green.unwrap_or_default(),
                step.blue.unwrap_or_default(),
            );
            if crate::ui::color_picker::color_button_srgba(ui, &mut color).changed() {
                step.red = Some(color.r());
                step.green = Some(color.g());
                step.blue = Some(color.b());
            }
        });
    }
    ui.menu_button(format!("{} Timing", crate::ui::icons::CLOCK), |ui| {
        ui.horizontal(|ui| {
            ui.label("Start");
            let mut at_us = step.at_us;
            if ui
                .add(crate::duration::time_value_us_drag(
                    &mut at_us,
                    0..=3_600_000_000,
                    1_000.0,
                    human_readable_time_units,
                ))
                .changed()
            {
                step.at_us = at_us;
            }
        });
        ui.horizontal(|ui| {
            let mut sustained = step.duration_ms.is_some();
            if ui.checkbox(&mut sustained, "Duration").changed() {
                step.duration_ms = sustained.then_some(1000);
            }
            if let Some(duration) = &mut step.duration_ms {
                let mut duration_ms = u64::from(*duration);
                if ui
                    .add(crate::duration::time_value_drag(
                        &mut duration_ms,
                        1..=65_535,
                        10.0,
                        human_readable_time_units,
                    ))
                    .changed()
                {
                    *duration = duration_ms as u16;
                }
            }
        });
        if ui.button("Move to start").clicked() {
            step.at_us = 0;
            ui.close();
        }
        if ui
            .button(format!(
                "Quantize to {}",
                crate::duration::format_time_value_ms_with_preference(
                    quantum_ms,
                    human_readable_time_units,
                )
            ))
            .clicked()
        {
            quantize_sequence(std::slice::from_mut(step), quantum_ms);
            ui.close();
        }
    });
    ui.menu_button(
        format!("{} Repeat", crate::ui::icons::ARROW_CLOCKWISE),
        |ui| {
            egui::Grid::new(ui.id().with("repeat-fields"))
                .num_columns(2)
                .show(ui, |ui| {
                    draw_repeat_controls(ui, step, human_readable_time_units)
                });
        },
    );
    ui.separator();
    if ui
        .button(format!("{} Duplicate", crate::ui::icons::COPY))
        .clicked()
    {
        action = Some(SequenceCueMenuAction::Duplicate);
        ui.close();
    }
    if ui
        .button(format!("{} Delete", crate::ui::icons::TRASH))
        .clicked()
    {
        action = Some(SequenceCueMenuAction::Delete);
        ui.close();
    }
    action
}

fn sequence_cue_drag_mode(
    duration_resizable: bool,
    left: f32,
    right: f32,
    pointer_x: f32,
) -> crate::app::DragMode {
    if duration_resizable {
        crate::app::classify_clip_drag_mode(left, right, pointer_x)
    } else {
        crate::app::DragMode::Move
    }
}

fn paint_sequence_resize_handles(
    painter: &egui::Painter,
    cue: egui::Rect,
    color: egui::Color32,
) {
    painter.line_segment(
        [
            cue.left_top() + egui::vec2(4.0, 5.0),
            cue.left_bottom() + egui::vec2(4.0, -5.0),
        ],
        egui::Stroke::new(1.5, color),
    );
    painter.line_segment(
        [
            cue.right_top() + egui::vec2(-4.0, 5.0),
            cue.right_bottom() + egui::vec2(-4.0, -5.0),
        ],
        egui::Stroke::new(1.5, color),
    );
}

fn paint_sequence_move_grip(
    painter: &egui::Painter,
    cue: egui::Rect,
    color: egui::Color32,
) {
    let center_x = if cue.width() < 40.0 {
        cue.center().x
    } else {
        cue.left() + 10.0
    };
    let half_height = ((cue.height() - 10.0) / 2.0).clamp(3.0, 7.0);
    for offset in [-2.5_f32, 0.0, 2.5] {
        painter.line_segment(
            [
                egui::pos2(center_x + offset, cue.center().y - half_height),
                egui::pos2(center_x + offset, cue.center().y + half_height),
            ],
            egui::Stroke::new(1.35, color.gamma_multiply(0.82)),
        );
    }
}

fn paint_sequence_cue_label(
    painter: &egui::Painter,
    cue: egui::Rect,
    label: &str,
    font_id: egui::FontId,
    color: egui::Color32,
    hide_overflow: bool,
    duration_resizable: bool,
) {
    let left_padding = if duration_resizable { 8.0 } else { 18.0 };
    if !hide_overflow {
        painter.text(
            cue.left_center() + egui::vec2(left_padding, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            font_id,
            color,
        );
        return;
    }

    let right_padding = if duration_resizable { 8.0 } else { 5.0 };
    let content_rect = egui::Rect::from_min_max(
        egui::pos2(cue.left() + left_padding, cue.top() + 2.0),
        egui::pos2(cue.right() - right_padding, cue.bottom() - 2.0),
    );
    if content_rect.width() < 2.0 || content_rect.height() < 2.0 {
        return;
    }
    let mut job = egui::text::LayoutJob {
        wrap: egui::text::TextWrapping::truncate_at_width(content_rect.width()),
        ..Default::default()
    };
    job.append(label, 0.0, egui::TextFormat::simple(font_id, color));
    let galley = painter.layout_job(job);
    painter.with_clip_rect(content_rect).galley(
        egui::pos2(
            content_rect.left(),
            content_rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        color,
    );
}

fn draw_sequence_timeline(
    ui: &mut egui::Ui,
    draft: &mut ControllerEffectDraft,
    selected_index: &mut usize,
    human_readable_time_units: bool,
    hide_cue_text_overflow: bool,
) {
    let state_prefix = (
        "effect-sequence-timeline",
        draft.reference.clone(),
        draft.id.clone(),
    );
    let zoom_id = egui::Id::new((state_prefix.clone(), "zoom"));
    let quantize_id = egui::Id::new((state_prefix.clone(), "quantize"));
    let snap_id = egui::Id::new((state_prefix.clone(), "snap"));
    let drag_id = egui::Id::new((state_prefix.clone(), "drag"));
    let mut zoom = ui
        .data_mut(|data| data.get_persisted::<f32>(zoom_id).unwrap_or(90.0))
        .clamp(30.0, 320.0);
    let mut quantum_ms = ui
        .data_mut(|data| data.get_persisted::<u64>(quantize_id).unwrap_or(50))
        .max(1);
    let mut snap = ui.data_mut(|data| data.get_persisted::<bool>(snap_id).unwrap_or(true));
    let mut menu_action = None;
    let edit_request_id = egui::Id::new((state_prefix.clone(), "edit-request"));

    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("{} Effect timeline", crate::ui::icons::WAVEFORM));
                ui.separator();
                ui.label("Zoom");
                ui.add(egui::Slider::new(&mut zoom, 30.0..=320.0).show_value(false));
                ui.separator();
                ui.checkbox(&mut snap, "Snap");
                egui::ComboBox::from_id_salt((state_prefix.clone(), "grid"))
                    .selected_text(crate::duration::format_time_value_ms(quantum_ms))
                    .show_ui(ui, |ui| {
                        for value in [1_u64, 10, 20, 25, 50, 100, 250, 500, 1_000] {
                            ui.selectable_value(
                                &mut quantum_ms,
                                value,
                                crate::duration::format_time_value_ms(value),
                            );
                        }
                    });
                if ui
                    .button(format!("{} Quantize", crate::ui::icons::SELECTION_ALL))
                    .clicked()
                {
                    quantize_sequence(&mut draft.steps, quantum_ms);
                }
                if ui
                    .button(format!("{} Remove delay", crate::ui::icons::SCISSORS))
                    .on_hover_text("Move the first cue to zero without changing relative timing")
                    .clicked()
                {
                    strip_sequence_leading_delay(&mut draft.steps);
                }
            });
            ui.data_mut(|data| {
                data.insert_persisted(zoom_id, zoom);
                data.insert_persisted(quantize_id, quantum_ms);
                data.insert_persisted(snap_id, snap);
            });
            ui.add_space(6.0);

            if draft.steps.is_empty() {
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), 96.0),
                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        ui.label(
                            egui::RichText::new("Add or record a cue to begin the effect").weak(),
                        );
                    },
                );
                return;
            }

            *selected_index = (*selected_index).min(draft.steps.len() - 1);
            let mut lanes =
                std::collections::BTreeMap::<String, (String, &'static str, Vec<usize>)>::new();
            for (index, step) in draft.steps.iter().enumerate() {
                let (key, label, icon) = sequence_lane(step);
                lanes
                    .entry(key)
                    .or_insert_with(|| (label, icon, Vec::new()))
                    .2
                    .push(index);
            }
            let lane_keys = lanes.keys().cloned().collect::<Vec<_>>();

            let label_width = 126.0;
            let row_height = 42.0;
            let ruler_height = 28.0;
            let px_per_ms = zoom / 1_000.0;
            let duration_ms = sequence_duration_ms(&draft.steps).max(1_000);
            let timeline_width = (duration_ms as f32 * px_per_ms + 80.0)
                .max((ui.available_width() - label_width).max(360.0));
            let canvas_size = egui::vec2(
                label_width + timeline_width,
                ruler_height + lanes.len() as f32 * row_height,
            );
            egui::ScrollArea::horizontal()
                .id_salt((state_prefix.clone(), "scroll"))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    let (canvas, canvas_response) =
                        ui.allocate_exact_size(canvas_size, egui::Sense::click());
                    let painter = ui.painter_at(canvas);
                    let timeline_left = canvas.left() + label_width;
                    let visuals = ui.visuals().clone();
                    painter.rect_filled(canvas, 7.0, visuals.extreme_bg_color);
                    painter.line_segment(
                        [
                            egui::pos2(timeline_left, canvas.top()),
                            egui::pos2(timeline_left, canvas.bottom()),
                        ],
                        visuals.widgets.noninteractive.bg_stroke,
                    );

                    let major_ms = if zoom >= 180.0 {
                        500
                    } else if zoom >= 70.0 {
                        1_000
                    } else {
                        2_000
                    };
                    let mut tick = 0_u64;
                    while tick <= duration_ms.saturating_add(major_ms) {
                        let x = timeline_left + tick as f32 * px_per_ms;
                        painter.line_segment(
                            [egui::pos2(x, canvas.top()), egui::pos2(x, canvas.bottom())],
                            egui::Stroke::new(
                                1.0,
                                visuals
                                    .widgets
                                    .noninteractive
                                    .bg_stroke
                                    .color
                                    .gamma_multiply(0.55),
                            ),
                        );
                        painter.text(
                            egui::pos2(x + 4.0, canvas.top() + 12.0),
                            egui::Align2::LEFT_CENTER,
                            crate::duration::format_timeline_time_ms(tick, major_ms < 1_000),
                            egui::FontId::proportional(10.5),
                            visuals.weak_text_color(),
                        );
                        tick = tick.saturating_add(major_ms);
                    }

                    for (lane_index, (_key, (label, icon, indexes))) in lanes.iter().enumerate() {
                        let row_top = canvas.top() + ruler_height + lane_index as f32 * row_height;
                        let row = egui::Rect::from_min_size(
                            egui::pos2(canvas.left(), row_top),
                            egui::vec2(canvas.width(), row_height),
                        );
                        if lane_index % 2 == 0 {
                            painter.rect_filled(
                                row,
                                0.0,
                                visuals.faint_bg_color.gamma_multiply(0.45),
                            );
                        }
                        painter.text(
                            egui::pos2(canvas.left() + 10.0, row.center().y),
                            egui::Align2::LEFT_CENTER,
                            format!("{icon}  {label}"),
                            egui::FontId::proportional(12.0),
                            visuals.text_color(),
                        );
                        painter.line_segment(
                            [
                                egui::pos2(canvas.left(), row.bottom()),
                                egui::pos2(canvas.right(), row.bottom()),
                            ],
                            egui::Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color),
                        );

                        for &index in indexes {
                            let step = &draft.steps[index];
                            let duration_resizable = step.duration_ms.is_some();
                            let start_ms = step.at_us.div_ceil(1_000);
                            let display_duration =
                                u64::from(step.duration_ms.unwrap_or_default()).max(60);
                            let left = timeline_left + start_ms as f32 * px_per_ms;
                            let width = (display_duration as f32 * px_per_ms).max(28.0);
                            let cue = egui::Rect::from_min_max(
                                egui::pos2(left, row_top + 6.0),
                                egui::pos2(
                                    (left + width).min(canvas.right() - 2.0),
                                    row.bottom() - 6.0,
                                ),
                            );
                            let mut response = ui.interact(
                                cue,
                                egui::Id::new((state_prefix.clone(), "cue", index)),
                                egui::Sense::click_and_drag(),
                            );
                            let cue_label = sequence_cue_label(step);
                            let duration_label = step.duration_ms.map_or_else(
                                || "Instant cue (drag to move)".to_string(),
                                |duration| crate::duration::format_time_value_ms(u64::from(duration)),
                            );
                            response = response.on_hover_text(format!(
                                "{}\nLane: {}\nStarts: {}\nDuration: {}",
                                cue_label,
                                label,
                                crate::duration::format_time_value_ms(start_ms),
                                duration_label,
                            ));
                            if response.hovered()
                                && let Some(pointer) = ui.ctx().pointer_latest_pos()
                            {
                                let drag_mode = sequence_cue_drag_mode(
                                    duration_resizable,
                                    cue.left(),
                                    cue.right(),
                                    pointer.x,
                                );
                                ui.ctx().set_cursor_icon(match drag_mode {
                                    crate::app::DragMode::Move => egui::CursorIcon::Grab,
                                    crate::app::DragMode::ResizeLeft
                                    | crate::app::DragMode::ResizeRight => {
                                        egui::CursorIcon::ResizeHorizontal
                                    }
                                });
                            }
                            let selected = *selected_index == index;
                            let interaction = ui.style().interact_selectable(&response, selected);
                            let fill = if selected {
                                visuals.selection.bg_fill
                            } else if response.hovered() {
                                interaction.weak_bg_fill.gamma_multiply(1.25)
                            } else {
                                interaction.weak_bg_fill
                            };
                            painter.rect(
                                cue,
                                5.0,
                                fill,
                                egui::Stroke::new(
                                    if selected { 1.5 } else { 1.0 },
                                    if selected {
                                        visuals.selection.stroke.color
                                    } else {
                                        interaction.bg_stroke.color
                                    },
                                ),
                                egui::StrokeKind::Inside,
                            );
                            paint_sequence_cue_label(
                                &painter,
                                cue,
                                &cue_label,
                                egui::FontId::proportional(11.5),
                                if selected {
                                    visuals.selection.stroke.color
                                } else {
                                    interaction.fg_stroke.color
                                },
                                hide_cue_text_overflow,
                                duration_resizable,
                            );
                            if duration_resizable {
                                if response.hovered() || selected {
                                    paint_sequence_resize_handles(
                                        &painter,
                                        cue,
                                        interaction.fg_stroke.color,
                                    );
                                }
                            } else {
                                paint_sequence_move_grip(
                                    &painter,
                                    cue,
                                    interaction.fg_stroke.color,
                                );
                            }
                            if response.clicked() || response.double_clicked() {
                                *selected_index = index;
                            }
                            if response.drag_started_by(egui::PointerButton::Primary) {
                                *selected_index = index;
                                if let Some(pointer) = response.interact_pointer_pos() {
                                    ui.data_mut(|data| {
                                        data.insert_temp(
                                            drag_id,
                                            SequenceCueDragState {
                                                index,
                                                mode: sequence_cue_drag_mode(
                                                    duration_resizable,
                                                    cue.left(),
                                                    cue.right(),
                                                    pointer.x,
                                                ),
                                                pointer_x: pointer.x,
                                                start_ms,
                                                duration_ms: u64::from(
                                                    step.duration_ms.unwrap_or(60),
                                                )
                                                .max(1),
                                            },
                                        );
                                    });
                                }
                            }
                            if response.dragged_by(egui::PointerButton::Primary)
                                && let Some(pointer) = response.interact_pointer_pos()
                                && let Some(drag) = ui
                                    .data_mut(|data| data.get_temp::<SequenceCueDragState>(drag_id))
                                && drag.index == index
                            {
                                let delta_ms =
                                    ((pointer.x - drag.pointer_x) / px_per_ms).round() as i64;
                                let step = &mut draft.steps[index];
                                match drag.mode {
                                    crate::app::DragMode::Move => {
                                        let moved = drag.start_ms as i64 + delta_ms;
                                        let moved = if snap {
                                            quantize_time(moved, quantum_ms)
                                        } else {
                                            moved.max(0) as u64
                                        };
                                        step.at_us = moved.saturating_mul(1_000);
                                        if let Some(lane_index) = sequence_lane_index_at_pointer(
                                            pointer.y,
                                            canvas.top() + ruler_height,
                                            row_height,
                                            lane_keys.len(),
                                        ) && let Some(lane_key) = lane_keys.get(lane_index)
                                        {
                                            retarget_sequence_cue_to_lane(step, lane_key);
                                        }
                                    }
                                    crate::app::DragMode::ResizeRight => {
                                        let duration = (drag.duration_ms as i64 + delta_ms).max(1);
                                        let duration = if snap {
                                            quantize_time(duration, quantum_ms).max(1)
                                        } else {
                                            duration as u64
                                        };
                                        step.duration_ms =
                                            Some(duration.min(u64::from(u16::MAX)) as u16);
                                    }
                                    crate::app::DragMode::ResizeLeft => {
                                        let end = drag.start_ms.saturating_add(drag.duration_ms);
                                        let start = (drag.start_ms as i64 + delta_ms)
                                            .clamp(0, end.saturating_sub(1) as i64);
                                        let start = if snap {
                                            quantize_time(start, quantum_ms)
                                                .min(end.saturating_sub(1))
                                        } else {
                                            start as u64
                                        };
                                        step.at_us = start.saturating_mul(1_000);
                                        step.duration_ms = Some(
                                            end.saturating_sub(start)
                                                .max(1)
                                                .min(u64::from(u16::MAX))
                                                as u16,
                                        );
                                    }
                                }
                            }
                            if response.drag_stopped() {
                                ui.data_mut(|data| data.remove::<SequenceCueDragState>(drag_id));
                            }
                            response.context_menu(|ui| {
                                *selected_index = index;
                                if let Some(action) = sequence_cue_context_menu(
                                    ui,
                                    &mut draft.steps[index],
                                    quantum_ms,
                                    human_readable_time_units,
                                ) {
                                    menu_action = Some((index, action));
                                }
                            });
                        }
                    }
                    canvas_response.context_menu(|ui| {
                        ui.strong("Effect timeline");
                        ui.separator();
                        ui.checkbox(&mut snap, "Snap to grid");
                        if ui
                            .button(format!(
                                "{} Quantize all cues",
                                crate::ui::icons::SELECTION_ALL
                            ))
                            .clicked()
                        {
                            quantize_sequence(&mut draft.steps, quantum_ms);
                            ui.close();
                        }
                        if ui
                            .button(format!(
                                "{} Remove leading delay",
                                crate::ui::icons::SCISSORS
                            ))
                            .clicked()
                        {
                            strip_sequence_leading_delay(&mut draft.steps);
                            ui.close();
                        }
                    });
                });

            let nudge_enabled =
                !ui.ctx().egui_wants_keyboard_input() && !egui::Popup::is_any_open(ui.ctx());
            if nudge_enabled && ui.input(|input| input.key_pressed(egui::Key::ArrowLeft)) {
                let step = &mut draft.steps[*selected_index];
                step.at_us = step.at_us.saturating_sub(quantum_ms.saturating_mul(1_000));
            }
            if nudge_enabled && ui.input(|input| input.key_pressed(egui::Key::ArrowRight)) {
                let step = &mut draft.steps[*selected_index];
                step.at_us = step.at_us.saturating_add(quantum_ms.saturating_mul(1_000));
            }
        });

    ui.data_mut(|data| data.insert_persisted(snap_id, snap));
    if let Some((index, action)) = menu_action {
        *selected_index = index;
        match action {
            SequenceCueMenuAction::Edit => {
                ui.data_mut(|data| data.insert_temp(edit_request_id, true));
            }
            SequenceCueMenuAction::Duplicate => {
                let mut duplicate = draft.steps[index].clone();
                duplicate.at_us = duplicate
                    .at_us
                    .saturating_add(u64::from(duplicate.duration_ms.unwrap_or(100)).max(1) * 1_000);
                draft.steps.insert(index + 1, duplicate);
                *selected_index = index + 1;
            }
            SequenceCueMenuAction::Delete => {
                draft.steps.remove(index);
                *selected_index = (*selected_index).min(draft.steps.len().saturating_sub(1));
            }
        }
    }
}

fn draw_repeat_controls(
    ui: &mut egui::Ui,
    step: &mut crate::four_d::controller::HardwareMacroStep,
    human_readable_time_units: bool,
) {
    ui.label("Repeat");
    ui.horizontal(|ui| {
        let mut enabled = step.repeat_count.unwrap_or(1) > 1;
        if ui.checkbox(&mut enabled, "Loop cue").changed() {
            step.repeat_count = enabled.then_some(2);
            step.repeat_interval_ms =
                enabled.then_some(u32::from(step.duration_ms.unwrap_or(100).max(1)));
        }
        if enabled {
            ui.add(
                egui::DragValue::new(step.repeat_count.get_or_insert(2))
                    .range(2..=1_000)
                    .prefix("× "),
            );
        }
    });
    ui.end_row();
    if step.repeat_count.unwrap_or(1) > 1 {
        ui.label("Every");
        let interval = step
            .repeat_interval_ms
            .get_or_insert(u32::from(step.duration_ms.unwrap_or(100).max(1)));
        let mut value = u64::from(*interval);
        if ui
            .add(crate::duration::time_value_drag(
                &mut value,
                1..=3_600_000,
                10.0,
                human_readable_time_units,
            ))
            .changed()
        {
            *interval = value.min(u64::from(u32::MAX)) as u32;
        }
        ui.end_row();
    }
}

fn draw_timeline_authoring_fields(
    ui: &mut egui::Ui,
    step: &mut crate::four_d::controller::HardwareMacroStep,
    index: usize,
    human_readable_time_units: bool,
) {
    let supports_duration = matches!(
        step.kind.as_str(),
        "motion" | "relay" | "pwm" | "rgb" | "addressable"
    );
    let supports_curve = matches!(step.kind.as_str(), "pwm" | "rgb" | "addressable");

    if supports_duration {
        ui.label("Cue length");
        let mut duration = u64::from(step.duration_ms.unwrap_or(100).max(1));
        if ui
            .add(crate::duration::time_value_drag(
                &mut duration,
                1..=65_535,
                10.0,
                human_readable_time_units,
            ))
            .changed()
        {
            step.duration_ms = Some(duration.min(u64::from(u16::MAX)) as u16);
        }
        ui.end_row();
    }

    if step.kind == "pwm" {
        ui.label("Transition");
        ui.horizontal(|ui| {
            let mut enabled = step.to_value.is_some();
            if ui.checkbox(&mut enabled, "Fade to").changed() {
                step.to_value = enabled.then_some(step.value.unwrap_or_default());
            }
            if enabled {
                ui.add(egui::DragValue::new(step.to_value.get_or_insert(0)).range(0..=4095));
            }
        });
        ui.end_row();
    } else if matches!(step.kind.as_str(), "rgb" | "addressable") {
        ui.label("Transition");
        ui.horizontal(|ui| {
            let mut enabled = step.to_red.is_some();
            if ui.checkbox(&mut enabled, "Fade to").changed() {
                if enabled {
                    step.to_red = step.red;
                    step.to_green = step.green;
                    step.to_blue = step.blue;
                    step.to_brightness = step.brightness;
                } else {
                    step.to_red = None;
                    step.to_green = None;
                    step.to_blue = None;
                    step.to_brightness = None;
                }
            }
            if enabled {
                let mut color = egui::Color32::from_rgb(
                    step.to_red.unwrap_or_default(),
                    step.to_green.unwrap_or_default(),
                    step.to_blue.unwrap_or_default(),
                );
                if crate::ui::color_picker::color_button_srgba(ui, &mut color).changed() {
                    step.to_red = Some(color.r());
                    step.to_green = Some(color.g());
                    step.to_blue = Some(color.b());
                }
                ui.add(
                    egui::DragValue::new(step.to_brightness.get_or_insert(255))
                        .range(0..=255)
                        .prefix("Brightness "),
                );
            }
        });
        ui.end_row();
    }

    if supports_curve && (step.to_value.is_some() || step.to_red.is_some()) {
        ui.label("Easing");
        egui::ComboBox::from_id_salt(("sequence-easing", index))
            .selected_text(if step.easing.is_empty() {
                "Linear"
            } else {
                step.easing.as_str()
            })
            .show_ui(ui, |ui| {
                for (value, label) in [
                    ("linear", "Linear"),
                    ("ease-in", "Ease in"),
                    ("ease-out", "Ease out"),
                    ("ease-in-out", "Ease in/out"),
                ] {
                    ui.selectable_value(&mut step.easing, value.to_string(), label);
                }
            });
        ui.end_row();
        ui.label("Curve quality");
        ui.add(
            egui::Slider::new(step.sample_rate_hz.get_or_insert(30), 1..=60).suffix(" samples/s"),
        );
        ui.end_row();
    }

    draw_repeat_controls(ui, step, human_readable_time_units);
}

fn draw_sequence_step_editor(
    ui: &mut egui::Ui,
    draft: &mut ControllerEffectDraft,
    rtl_ui: bool,
    capabilities: Option<&crate::four_d::controller::HardwareCapabilities>,
    human_readable_time_units: bool,
    hide_cue_text_overflow: bool,
) {
    let selection_id = egui::Id::new((
        "effect-sequence-selected-cue",
        draft.reference.clone(),
        draft.id.clone(),
    ));
    let mut selected_index =
        ui.data_mut(|data| data.get_persisted::<usize>(selection_id).unwrap_or(0));
    draft.duration_ms = sequence_duration_ms(&draft.steps);
    draw_sequence_timeline(
        ui,
        draft,
        &mut selected_index,
        human_readable_time_units,
        hide_cue_text_overflow,
    );
    ui.data_mut(|data| data.insert_persisted(selection_id, selected_index));
    ui.add_space(8.0);

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

    selected_index = selected_index.min(draft.steps.len() - 1);
    let cue_heading = ui.heading("Selected cue");
    let edit_request_id = egui::Id::new((
        (
            "effect-sequence-timeline",
            draft.reference.clone(),
            draft.id.clone(),
        ),
        "edit-request",
    ));
    if ui.data_mut(|data| data.remove_temp::<bool>(edit_request_id).unwrap_or(false)) {
        cue_heading.scroll_to_me(Some(egui::Align::Center));
    }
    let mut move_step = None;
    let mut remove_step = None;
    for index in selected_index..=selected_index {
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
                                let kinds = sequence_step_kinds(capabilities);
                                if !step.kind.is_empty()
                                    && !kinds.iter().any(|kind| *kind == step.kind)
                                {
                                    let unavailable = step.kind.clone();
                                    ui.selectable_value(
                                        &mut step.kind,
                                        unavailable.clone(),
                                        format!("{unavailable} (unavailable)"),
                                    );
                                }
                                for kind in kinds {
                                    ui.selectable_value(&mut step.kind, kind.to_string(), kind);
                                }
                            });
                        if step.kind != previous_kind {
                            reset_step_kind(step, step.kind.clone());
                        }
                        ui.end_row();

                        ui.label("Time");
                        let mut at_us = step.at_us;
                        if ui
                            .add(crate::duration::time_value_us_drag(
                                &mut at_us,
                                0..=3_600_000_000,
                                1_000.0,
                                human_readable_time_units,
                            ))
                            .changed()
                        {
                            step.at_us = at_us;
                        }
                        ui.end_row();

                        match step.kind.as_str() {
                            "motion" => {
                                ui.label("Seat");
                                let target = step.target.get_or_insert(0);
                                let selected_name = motion_seat_name(capabilities, *target);
                                egui::ComboBox::from_id_salt(("motion-side", index))
                                    .selected_text(selected_name)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            target,
                                            0,
                                            motion_seat_name(capabilities, 0),
                                        );
                                        ui.selectable_value(
                                            target,
                                            1,
                                            motion_seat_name(capabilities, 1),
                                        );
                                    });
                                ui.end_row();
                                ui.label("Action");
                                let value = step.value.get_or_insert(0);
                                egui::ComboBox::from_id_salt(("motion-action", index))
                                    .selected_text(match *value {
                                        1 => "Up",
                                        2 => "Down",
                                        _ => "Stop",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(value, 0, "Stop");
                                        ui.selectable_value(value, 1, "Up");
                                        ui.selectable_value(value, 2, "Down");
                                    });
                                refresh_semantic_action(step);
                                ui.end_row();
                            }
                            "relay" => {
                                ui.label("Relay");
                                let target = step.target.get_or_insert(0);
                                let selected = capabilities
                                    .and_then(|value| {
                                        value
                                            .relays
                                            .iter()
                                            .find(|output| output.id.saturating_sub(1) == *target)
                                    })
                                    .map(|output| format!("R{} · {}", output.id, output.name))
                                    .unwrap_or_else(|| format!("R{}", target.saturating_add(1)));
                                egui::ComboBox::from_id_salt(("relay-target", index))
                                    .selected_text(selected)
                                    .show_ui(ui, |ui| {
                                        if let Some(capabilities) = capabilities {
                                            for output in &capabilities.relays {
                                                ui.selectable_value(
                                                    target,
                                                    output.id.saturating_sub(1),
                                                    format!("R{} · {}", output.id, output.name),
                                                );
                                            }
                                        }
                                    });
                                ui.end_row();
                                ui.label("State");
                                let value = step.value.get_or_insert(0);
                                egui::ComboBox::from_id_salt(("relay-state", index))
                                    .selected_text(if *value == 0 { "Off" } else { "On" })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(value, 0, "Off");
                                        ui.selectable_value(value, 1, "On");
                                    });
                                refresh_semantic_action(step);
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
                                let target = step.target.get_or_insert(0);
                                let selected = capabilities
                                    .and_then(|value| {
                                        value
                                            .pwm_channels
                                            .iter()
                                            .find(|output| output.id == *target)
                                    })
                                    .map(|output| format!("CH{} · {}", output.id, output.name))
                                    .unwrap_or_else(|| format!("CH{target}"));
                                egui::ComboBox::from_id_salt(("pwm-target", index))
                                    .selected_text(selected)
                                    .show_ui(ui, |ui| {
                                        if let Some(capabilities) = capabilities {
                                            for output in &capabilities.pwm_channels {
                                                ui.selectable_value(
                                                    target,
                                                    output.id,
                                                    format!("CH{} · {}", output.id, output.name),
                                                );
                                            }
                                        }
                                    });
                                ui.end_row();
                                ui.label("Value");
                                ui.add(
                                    egui::DragValue::new(step.value.get_or_insert(0))
                                        .range(0..=4095),
                                );
                                ui.end_row();
                            }
                            "pwm-off" | "relays-off" => {
                                ui.label("Action");
                                ui.label(if step.kind == "pwm-off" {
                                    "Turn every PWM output off"
                                } else {
                                    "Turn every relay off"
                                });
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
                                let text_align =
                                    crate::ui::i18n::input_alignment(rtl_ui, &step.text);
                                ui.add(
                                    crate::ui::dialog::singleline_text_edit(&mut step.text)
                                        .horizontal_align(text_align),
                                );
                                ui.end_row();
                                ui.label("Visible for");
                                let mut duration_ms =
                                    u64::from(*step.duration_ms.get_or_insert(1_500));
                                if ui
                                    .add(crate::duration::time_value_drag(
                                        &mut duration_ms,
                                        1..=65_535,
                                        10.0,
                                        human_readable_time_units,
                                    ))
                                    .changed()
                                {
                                    step.duration_ms = Some(duration_ms as u16);
                                }
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
                                let mut pulse_us = u64::from(*step.pulse_us.get_or_insert(350));
                                if ui
                                    .add(crate::duration::time_value_us_drag(
                                        &mut pulse_us,
                                        1..=65_535,
                                        10.0,
                                        human_readable_time_units,
                                    ))
                                    .changed()
                                {
                                    step.pulse_us = Some(pulse_us as u16);
                                }
                                ui.end_row();
                            }
                            "beep" => {
                                ui.label("Frequency");
                                ui.add(
                                    egui::DragValue::new(step.frequency_hz.get_or_insert(1_000))
                                        .range(0..=20_000)
                                        .suffix(" Hz"),
                                );
                                ui.end_row();
                                ui.label("Duration");
                                let mut duration_ms =
                                    u64::from(*step.duration_ms.get_or_insert(120));
                                if ui
                                    .add(crate::duration::time_value_drag(
                                        &mut duration_ms,
                                        1..=65_535,
                                        10.0,
                                        human_readable_time_units,
                                    ))
                                    .changed()
                                {
                                    step.duration_ms = Some(duration_ms as u16);
                                }
                                ui.end_row();
                            }
                            "rgb" => {
                                ui.label("Color");
                                let mut color = egui::Color32::from_rgb(
                                    step.red.unwrap_or_default(),
                                    step.green.unwrap_or_default(),
                                    step.blue.unwrap_or_default(),
                                );
                                if crate::ui::color_picker::color_button_srgba(ui, &mut color).changed() {
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
                            "addressable" => {
                                ui.label("Pixel");
                                let target = step.target.get_or_insert(0);
                                let maximum = capabilities
                                    .and_then(|value| value.strip_control.as_ref())
                                    .map(|strip| strip.maximum_pixels.min(u16::from(u8::MAX)))
                                    .unwrap_or(100)
                                    as u8;
                                egui::ComboBox::from_id_salt(("addressable-pixel", index))
                                    .selected_text(format!("Pixel {}", target.saturating_add(1)))
                                    .show_ui(ui, |ui| {
                                        for pixel in 0..maximum {
                                            ui.selectable_value(
                                                target,
                                                pixel,
                                                format!("Pixel {}", pixel + 1),
                                            );
                                        }
                                    });
                                ui.end_row();
                                ui.label("Color");
                                let mut color = egui::Color32::from_rgb(
                                    step.red.unwrap_or_default(),
                                    step.green.unwrap_or_default(),
                                    step.blue.unwrap_or_default(),
                                );
                                if crate::ui::color_picker::color_button_srgba(ui, &mut color).changed() {
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
                            "menu" => {
                                ui.label("Page ID");
                                ui.add(
                                    egui::DragValue::new(step.target.get_or_insert(0))
                                        .range(0..=255),
                                );
                                ui.end_row();
                            }
                            "menu-action" => {
                                ui.label("Front-panel action");
                                let target = step.target.get_or_insert(0);
                                egui::ComboBox::from_id_salt(("menu-action", index))
                                    .selected_text(match *target {
                                        0 => "Back",
                                        1 => "Enter",
                                        2 => "Decrease",
                                        _ => "Increase",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(target, 0, "Back");
                                        ui.selectable_value(target, 1, "Enter");
                                        ui.selectable_value(target, 2, "Decrease");
                                        ui.selectable_value(target, 3, "Increase");
                                    });
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
                                    crate::ui::dialog::singleline_text_edit(&mut step.payload_hex)
                                        .desired_width(280.0)
                                        .font(egui::TextStyle::Monospace),
                                );
                                ui.end_row();
                                ui.label("Description");
                                let text_align =
                                    crate::ui::i18n::input_alignment(rtl_ui, &step.text);
                                ui.add(
                                    crate::ui::dialog::singleline_text_edit(&mut step.text)
                                        .horizontal_align(text_align),
                                );
                                ui.end_row();
                            }
                            _ => {}
                        }

                        draw_timeline_authoring_fields(
                            ui,
                            step,
                            index,
                            human_readable_time_units,
                        );
                    });

                ui.add_space(5.0);
                ui.label(egui::RichText::new("Semantic actions").small().strong());
                let mut remove_action = None;
                for (action_index, action) in step.action_ids.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add(
                            crate::ui::dialog::singleline_text_edit(action)
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
        selected_index = to;
    }
    if let Some(index) = remove_step {
        draft.steps.remove(index);
        selected_index = selected_index.min(draft.steps.len().saturating_sub(1));
    }
    ui.data_mut(|data| data.insert_persisted(selection_id, selected_index));
    draft.duration_ms = sequence_duration_ms(&draft.steps);
}

fn motion_seat_name(
    capabilities: Option<&crate::four_d::controller::HardwareCapabilities>,
    target: u8,
) -> String {
    let (key, fallback) = if target == 0 {
        ("seat.a", "Seat A")
    } else {
        ("seat.b", "Seat B")
    };
    capabilities
        .and_then(|value| {
            value
                .controls
                .iter()
                .find(|control| control.key == key)
                .map(|control| control.name.trim())
                .filter(|name| !name.is_empty())
                .or_else(|| {
                    value
                        .peripheral_names
                        .get(key)
                        .map(|name| name.trim())
                        .filter(|name| !name.is_empty())
                })
        })
        .unwrap_or(fallback)
        .to_string()
}

#[derive(serde::Deserialize)]
struct RecordingColor {
    id: String,
    label: String,
    hex: String,
}

fn recording_colors() -> &'static [RecordingColor] {
    static COLORS: std::sync::OnceLock<Vec<RecordingColor>> = std::sync::OnceLock::new();
    COLORS.get_or_init(|| {
        serde_json::from_str(include_str!("../../assets/themes/recording-colors.json"))
            .expect("recording color palette must be valid")
    })
}

fn recording_color(value: &str) -> egui::Color32 {
    let id = if value == "purple" { "violet" } else { value };
    let hex = recording_colors()
        .iter()
        .find(|color| color.id == id)
        .unwrap_or(&recording_colors()[0])
        .hex
        .as_str();
    let rgb = crate::config::parse_rgb_hex(hex).expect("recording color must be RGB");
    egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

fn record_action_color() -> egui::Color32 {
    // Recording is a live-state affordance, not a preview of the effect's
    // timeline color. Keep it unmistakably red in every palette.
    egui::Color32::from_rgb(205, 42, 54)
}

fn paint_recording_swatch(ui: &egui::Ui, rect: egui::Rect, value: &str) {
    let center = egui::pos2(rect.left() + 12.0, rect.center().y);
    ui.painter()
        .circle_filled(center, 5.0, recording_color(value));
    // White must remain distinguishable on light popup surfaces too.
    ui.painter()
        .circle_stroke(center, 5.0, egui::Stroke::new(1.0, egui::Color32::GRAY));
}

fn recording_color_label(ui: &egui::Ui, label: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    // Match the painted swatch's fixed 24 px lane. Ordinary spaces vary with
    // font and selected/hover formatting, making the label visibly jump.
    job.append(
        label,
        28.0,
        egui::TextFormat::simple(
            egui::TextStyle::Button.resolve(ui.style()),
            ui.visuals().text_color(),
        ),
    );
    job
}

fn recording_color_picker(ui: &mut egui::Ui, value: &mut String) -> egui::Response {
    let selected = recording_colors()
        .iter()
        .find(|color| color.id == *value)
        .unwrap_or(&recording_colors()[0]);
    let response = egui::ComboBox::from_id_salt("effect_recording_color")
        .selected_text(recording_color_label(ui, &selected.label))
        .show_ui(ui, |ui| {
            for color in recording_colors() {
                let label = recording_color_label(ui, &color.label);
                let row = ui.selectable_value(value, color.id.clone(), label);
                paint_recording_swatch(ui, row.rect, &color.id);
            }
        })
        .response;
    paint_recording_swatch(ui, response.rect, value);
    response
}

fn paint_recording_icon(ui: &egui::Ui, rect: egui::Rect, value: &str) {
    if recording_color(value) == egui::Color32::WHITE && !ui.visuals().dark_mode {
        ui.painter().text(
            rect.center() + egui::vec2(1.0, 1.0),
            egui::Align2::CENTER_CENTER,
            crate::ui::icons::RECORD,
            egui::FontId::proportional(18.0),
            ui.visuals().weak_text_color(),
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        crate::ui::icons::RECORD,
        egui::FontId::proportional(18.0),
        recording_color(value),
    );
}

fn recording_preview_detail(step: &crate::four_d::controller::HardwareMacroStep) -> String {
    if !step.action_ids.is_empty() {
        return step.action_ids.join(" · ");
    }
    match (step.target, step.value) {
        (Some(target), Some(value)) => format!("target {target} · value {value}"),
        (Some(target), None) => format!("target {target}"),
        (None, Some(value)) => format!("value {value}"),
        (None, None) if !step.text.trim().is_empty() => step.text.replace('\n', " · "),
        _ => String::new(),
    }
}

pub(crate) fn draw_effect_capture_controls(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let hardware = app.advertised_hardware();
    let connected = hardware.as_ref().is_some_and(|hardware| hardware.board_connected);
    let recording = hardware.as_ref().map(|hardware| hardware.effect_recording.clone()).unwrap_or_default();
    let active = recording.active || app.hardware_effect_authoring.active;
    let busy = app.hardware_effect_authoring.pending_operation.is_some() || app.hardware_effect_authoring.pending_saved_macro_id.is_some();
    let target_matches = app.hardware_effect_authoring.append_target.map(|id| id.to_string())
        .as_deref() == Some(app.effect_library_draft.id.as_str());
    if active && target_matches && !recording.preview.is_empty() {
        app.effect_library_draft.steps = recording.preview.clone();
    }
    let mut start = false;
    let mut finish = false;
    let mut discard = false;
    let mut selected_melody = None;
    let mut refresh_melodies = false;
    ui.horizontal_wrapped(|ui| {
        ui.strong("Sequence steps");
        if ui.add_enabled(!active && !busy, egui::Button::new(format!("{} Add step", crate::ui::icons::PLUS))).clicked() {
            let draft = &mut app.effect_library_draft;
            let at_us = sequence_duration_ms(&draft.steps).saturating_mul(1000);
            let mut step = crate::four_d::controller::HardwareMacroStep { at_us, ..Default::default() };
            let kind = sequence_step_kinds(hardware.as_ref()).into_iter().next().unwrap_or("opcode");
            reset_step_kind(&mut step, kind.into());
            draft.steps.push(step);
            let selection_id = egui::Id::new(("effect-sequence-selected-cue", draft.reference.clone(), draft.id.clone()));
            ui.data_mut(|data| data.insert_persisted(selection_id, draft.steps.len() - 1));
        }
        ui.add_enabled_ui(!active && !busy && connected, |ui| {
            let has_melodies = hardware.as_ref().is_some_and(|value| !value.melodies.is_empty());
            let response = egui::ComboBox::from_id_salt("effect_add_melody")
                .selected_text(if has_melodies {
                    format!("{} Add melody", crate::ui::icons::MUSIC_NOTE)
                } else {
                    "No melodies".to_string()
                })
                .show_ui(ui, |ui| {
                    let Some(hardware) = hardware.as_ref() else {
                        ui.weak("PCController is unavailable");
                        return;
                    };
                    if hardware.melodies.is_empty() {
                        ui.weak("No configured melodies");
                    }
                    for melody in &hardware.melodies {
                        let duration = crate::duration::format_effect_duration_for_language(
                            app.language,
                            melody.duration_ms(),
                        );
                        if ui.selectable_label(
                            false,
                            format!("{}  {} · {duration}", crate::ui::icons::MUSIC_NOTE, melody.name),
                        ).clicked() {
                            selected_melody = Some(melody.clone());
                            ui.close();
                        }
                    }
                })
                .response;
            refresh_melodies = response.clicked();
        });
        ui.add_enabled_ui(!active && !busy, |ui| {
            egui::ComboBox::from_id_salt("effect_capture_clock").width(160.0)
                .selected_text(match app.hardware_effect_authoring.capture_mode.as_str() {
                    "device-clock" => "Device clock",
                    "board-retained" => "Board capture",
                    _ => "All live sources",
                }).show_ui(ui, |ui| {
                    for (value, label) in [("automatic", "All live sources"), ("device-clock", "Device clock"), ("board-retained", "Board capture")] {
                        ui.selectable_value(&mut app.hardware_effect_authoring.capture_mode, value.into(), label);
                    }
                });
        });
        if active {
            ui.label(egui::RichText::new(format!("{} {} actions", crate::ui::icons::RECORD, recording.steps))
                .color(record_action_color()));
            finish = ui.add_enabled(!busy, egui::Button::new(format!("{} Finish", crate::ui::icons::STOP_CIRCLE))).clicked();
            discard = ui.add_enabled(!busy, egui::Button::new(format!("{} Discard take", crate::ui::icons::TRASH)))
                .on_hover_text("Discard only this capture; existing sequence steps are retained").clicked();
        } else {
            start = ui.add_enabled(connected && !busy && !app.effect_library_draft.name.trim().is_empty(),
                egui::Button::new(egui::RichText::new(format!("{} Record", crate::ui::icons::RECORD))
                    .color(egui::Color32::WHITE))
                    .fill(record_action_color())
                    .stroke(egui::Stroke::new(1.0, record_action_color())))
                .on_hover_text("Publish the current sequence and capture at its end. Delete existing steps first to replace them.").clicked();
        }
        if busy { ui.spinner(); }
        if !active { ui.weak(format!("{} · {}", app.effect_library_draft.steps.len(), crate::duration::format_effect_duration_for_language(app.language, sequence_duration_ms(&app.effect_library_draft.steps)))); }
        if !recording.last_error.is_empty() { ui.colored_label(ui.visuals().error_fg_color, &recording.last_error); }
    });
    if refresh_melodies {
        // Events keep this catalog current in the background; opening the
        // picker is also an explicit freshness boundary for user choice.
        app.engine_handle.request_catalog_refresh();
    }
    if let Some(melody) = selected_melody
        && let Some(first) = append_melody_steps(&mut app.effect_library_draft.steps, &melody)
    {
        let selection_id = egui::Id::new((
            "effect-sequence-selected-cue",
            app.effect_library_draft.reference.clone(),
            app.effect_library_draft.id.clone(),
        ));
        ui.data_mut(|data| data.insert_persisted(selection_id, first));
    }
    let result = if start { app.start_hardware_effect_recording() }
        else if finish { app.save_hardware_effect_recording() }
        else if discard { app.discard_hardware_effect_recording() }
        else { Ok(()) };
    if let Err(error) = result { app.set_osd(error); }
}

pub fn draw_editor(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_effect_library_editor {
        return;
    }
    let mut open = app.show_effect_library_editor;
    let display_language = app.language;
    let human_readable_time_units = app.human_readable_time_units;
    let hide_cue_text_overflow = app.timeline_hide_cue_text_overflow;
    let capabilities = app.advertised_hardware();
    let capture_locked = app.hardware_effect_authoring.active
        || app.hardware_effect_authoring.pending_operation.is_some()
        || app.hardware_effect_authoring.pending_saved_macro_id.is_some()
        || capabilities.as_ref().is_some_and(|hardware| hardware.effect_recording.active);
    let sequences = capabilities
        .as_ref()
        .map(|value| value.macros.clone())
        .unwrap_or_default();
    let strips = capabilities
        .as_ref()
        .map(|value| value.strip_effects.clone())
        .unwrap_or_default();
    let groups = capabilities
        .as_ref()
        .map(|hardware| hardware.effect_groups.clone())
        .unwrap_or_default();
    let mut new_group_requested = false;
    let geometry = crate::ui::dialog::bounded_geometry(
        ui.ctx().content_rect(),
        20.0,
        egui::vec2(1_180.0, 780.0),
        egui::vec2(760.0, 520.0),
        egui::vec2(1_520.0, 980.0),
    );

    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::SPARKLE,
        app.tr("Effect properties")
    ))
    .id(egui::Id::new("controller_effect_library_dialog"))
    .open(&mut open)
    .default_rect(geometry.default_rect)
    .min_size(geometry.min_size)
    .max_size(geometry.max_size)
    .constrain_to(geometry.bounds)
    .resizable(true)
    .order(egui::Order::Foreground)
    .frame(crate::ui::dialog::opaque_window_frame(ui))
    .collapsible(false)
    .show(ui.ctx(), |ui| {
        let height = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(220.0_f32.min(ui.available_width() * 0.36), height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    if capture_locked { ui.disable(); }
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
                            if capture_locked { ui.disable(); }
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
                                app.tr("Group"),
                                app.tr("Icon"),
                                app.tr("Frame rate"),
                                app.tr("LED count"),
                                app.tr("Duration"),
                                app.tr("Execution"),
                            );
                            let icon_picker_labels = (
                                app.tr("Search icons..."),
                                app.tr("Presets"),
                                app.tr("No matching icons"),
                            );
                            let rtl_ui = app.rtl;
                            ui.add_enabled_ui(!capture_locked, |ui| {
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
                                        crate::ui::dialog::singleline_text_edit(&mut draft.id)
                                            .desired_width(260.0),
                                    );
                                    ui.end_row();
                                    ui.label(&labels.4);
                                    let name_align = crate::ui::i18n::input_alignment(
                                        rtl_ui,
                                        &draft.name,
                                    );
                                    ui.add(
                                        crate::ui::dialog::singleline_text_edit(&mut draft.name)
                                            .horizontal_align(name_align)
                                            .desired_width(320.0),
                                    );
                                    ui.end_row();
                                    ui.label(&labels.5);
                                    new_group_requested |= crate::ui::group_picker::effect_group_picker(
                                        ui,
                                        "effect-group",
                                        &mut draft.category,
                                        &groups,
                                        320.0,
                                        display_language,
                                    );
                                    ui.end_row();
                                    ui.label(&labels.6);
                                    crate::ui::icons::searchable_control_icon_picker(
                                        ui,
                                        "effect_icon_picker",
                                        &mut draft.icon,
                                        320.0,
                                        &icon_picker_labels.0,
                                        &icon_picker_labels.1,
                                        &icon_picker_labels.2,
                                        display_language,
                                    );
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
                                            human_readable_time_units,
                                        ));
                                    }
                                    ui.end_row();
                                    if draft.kind == "sequence" {
                                        ui.label("Color");
                                        recording_color_picker(ui, &mut draft.color);
                                        ui.end_row();
                                        ui.label(&labels.10);
                                        egui::ComboBox::from_id_salt("effect_sequence_engine")
                                            .selected_text(match draft.engine.as_str() {
                                                "mcu" => "Device clock (forced)",
                                                "host" => "Host clock (forced)",
                                                _ => "Automatic (recommended)",
                                            })
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(
                                                    &mut draft.engine,
                                                    "auto".to_string(),
                                                    "Automatic (recommended)",
                                                );
                                                ui.selectable_value(
                                                    &mut draft.engine,
                                                    "host".to_string(),
                                                    "Host clock (forced)",
                                                );
                                                ui.selectable_value(
                                                    &mut draft.engine,
                                                    "mcu".to_string(),
                                                    "Device clock (forced)",
                                                );
                                            });
                                        ui.end_row();
                                    }
                                });
                            });
                            if app.effect_library_draft.kind == "sequence" {
                                draw_effect_capture_controls(app, ui);
                                ui.add_space(10.0);
                                ui.add_enabled_ui(!capture_locked, |ui| draw_sequence_step_editor(
                                    ui,
                                    &mut app.effect_library_draft,
                                    rtl_ui,
                                    capabilities.as_ref(),
                                    human_readable_time_units,
                                    hide_cue_text_overflow,
                                ));
                                ui.add_space(8.0);
                                if crate::ui::icons::disclosure_header(
                                    ui,
                                    "effect_sequence_properties",
                                    "Sequence properties",
                                    false,
                                ) {
                                    egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                                        if capture_locked { ui.disable(); }
                                        egui::Grid::new("effect_sequence_properties_grid")
                                            .num_columns(2)
                                            .spacing([14.0, 8.0])
                                            .show(ui, |ui| {
                                                ui.label("Timing tolerance");
                                                let mut tolerance_us = u64::from(
                                                    app.effect_library_draft.timing_tolerance_us,
                                                );
                                                if ui
                                                    .add(crate::duration::time_value_us_drag(
                                                        &mut tolerance_us,
                                                        0..=5_000_000,
                                                        100.0,
                                                        human_readable_time_units,
                                                    ))
                                                    .changed()
                                                {
                                                    app.effect_library_draft.timing_tolerance_us =
                                                        tolerance_us as u32;
                                                }
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
                                                let label_align = crate::ui::i18n::input_alignment(
                                                    rtl_ui,
                                                    &app.effect_library_draft.label,
                                                );
                                                ui.add(
                                                    crate::ui::dialog::singleline_text_edit(
                                                        &mut app.effect_library_draft.label,
                                                    )
                                                    .horizontal_align(label_align),
                                                );
                                                ui.end_row();
                                                ui.label("LCD message");
                                                let lcd_align = crate::ui::i18n::input_alignment(
                                                    rtl_ui,
                                                    &app.effect_library_draft.lcd_message,
                                                );
                                                ui.add(
                                                    crate::ui::dialog::singleline_text_edit(
                                                        &mut app.effect_library_draft.lcd_message,
                                                    )
                                                    .horizontal_align(lcd_align),
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
                                let description_align = crate::ui::i18n::input_alignment(
                                    rtl_ui,
                                    &app.effect_library_draft.description,
                                );
                                ui.add_sized(
                                    [ui.available_width(), 72.0],
                                    egui::TextEdit::multiline(
                                        &mut app.effect_library_draft.description,
                                    )
                                    .horizontal_align(description_align)
                                    .desired_rows(3),
                                );
                            }
                            ui.add_space(12.0);
                            let reference = app.effect_library_draft.reference.clone();
                            let saved = !app.effect_library_draft.is_new;
                            let published = app.controller_effect_is_advertised(&reference);
                            let controller_reachable = app
                                .engine_handle
                                .is_connected
                                .load(std::sync::atomic::Ordering::Relaxed);
                            ui.add_enabled_ui(!capture_locked, |ui| { ui.horizontal_wrapped(|ui| {
                                if ui
                                    .button(if controller_reachable {
                                        format!(
                                            "{} {}",
                                            crate::ui::icons::FLOPPY_DISK,
                                            app.tr("Publish")
                                        )
                                    } else {
                                        format!(
                                            "{} {}",
                                            crate::ui::icons::FLOPPY_DISK,
                                            app.tr("Keep offline draft")
                                        )
                                    })
                                    .clicked()
                                {
                                    if controller_reachable {
                                        if let Err(error) = app.save_controller_effect() {
                                            app.set_osd(error);
                                        }
                                    } else {
                                        app.save_config();
                                        app.hardware_effect_authoring.status =
                                            "Offline working copy saved".to_string();
                                    }
                                }
                                if ui
                                    .add_enabled(
                                        published,
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
                                        published,
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
                            });
                            if saved && !published {
                                ui.label(
                                    egui::RichText::new(
                                        "This local draft is not currently published in PCController. Publish it to restore Run and Delete.",
                                    )
                                    .small()
                                    .weak(),
                                );
                            }
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
    if app.show_effect_library_editor && !open {
        // Closing the editor never throws away an offline working copy. The
        // catalog itself remains PCController-owned; only this single draft is
        // persisted by Pealayer until it can be published.
        app.save_config();
    }
    app.show_effect_library_editor = open;
    if new_group_requested {
        begin_new_group(app);
    }
    if app.effect_group_draft.is_some() {
        ui.ctx().move_to_top(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("effect_group_editor"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instantaneous_sequence_cues_are_move_only() {
        assert_eq!(
            sequence_cue_drag_mode(false, 10.0, 110.0, 10.0),
            crate::app::DragMode::Move,
        );
        assert_eq!(
            sequence_cue_drag_mode(false, 10.0, 110.0, 110.0),
            crate::app::DragMode::Move,
        );
        assert_eq!(
            sequence_cue_drag_mode(true, 10.0, 110.0, 10.0),
            crate::app::DragMode::ResizeLeft,
        );
        assert_eq!(
            sequence_cue_drag_mode(true, 10.0, 110.0, 110.0),
            crate::app::DragMode::ResizeRight,
        );
    }

    #[test]
    fn sequence_cue_menu_values_are_channel_specific() {
        assert_eq!(
            sequence_cue_value_options("motion"),
            &[(1, "Up"), (2, "Down"), (0, "Stop")]
        );
        assert_eq!(
            sequence_cue_value_options("relay"),
            &[(1, "On"), (0, "Off")]
        );
        assert!(sequence_cue_value_options("pwm").is_empty());
        assert!(sequence_cue_value_options("display").is_empty());
    }

    #[test]
    fn sequence_cue_right_click_targets_pointer_cue_and_duplicate_preserves_data() {
        fn text_positions(
            shape: &egui::epaint::Shape,
            text: &str,
            positions: &mut Vec<egui::Pos2>,
        ) {
            match shape {
                egui::epaint::Shape::Vec(shapes) => {
                    for shape in shapes {
                        text_positions(shape, text, positions);
                    }
                }
                egui::epaint::Shape::Text(t) if t.galley.job.text.contains(text) => {
                    positions.push(t.pos + t.galley.rect.center().to_vec2())
                }
                _ => {}
            }
        }
        for dark in [false, true] {
            let context = egui::Context::default();
            context.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            context.all_styles_mut(|style| style.animation_time = 0.0);
            let mut draft = ControllerEffectDraft::default();
            draft.steps = vec![
                crate::four_d::controller::HardwareMacroStep {
                    kind: "relay".into(),
                    target: Some(4),
                    value: Some(0),
                    ..Default::default()
                },
                crate::four_d::controller::HardwareMacroStep {
                    kind: "motion".into(),
                    target: Some(1),
                    value: Some(1),
                    at_us: 200_000,
                    duration_ms: Some(1000),
                    repeat_count: Some(3),
                    text: "Preserve metadata".into(),
                    ..Default::default()
                },
            ];
            let original = draft.steps[1].clone();
            let mut selected = 0;
            let render = |events, draft: &mut ControllerEffectDraft, selected: &mut usize| {
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(800.0, 600.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| draw_sequence_timeline(ui, draft, selected, true, true),
                );
                output.textures_delta.clear();
                output
            };
            render(vec![], &mut draft, &mut selected);
            let output = render(vec![], &mut draft, &mut selected);
            let mut positions = vec![];
            for s in &output.shapes {
                text_positions(&s.shape, "Up", &mut positions);
            }
            let cue = positions[0];
            let click = |pos, button, pressed| egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            render(
                vec![
                    egui::Event::PointerMoved(cue),
                    click(cue, egui::PointerButton::Secondary, true),
                ],
                &mut draft,
                &mut selected,
            );
            render(
                vec![egui::Event::PointerMoved(cue + egui::vec2(10.0, 0.0))],
                &mut draft,
                &mut selected,
            );
            assert_eq!(
                draft.steps[1], original,
                "secondary dragging must not move or resize the cue"
            );
            render(
                vec![click(
                    cue + egui::vec2(10.0, 0.0),
                    egui::PointerButton::Secondary,
                    false,
                )],
                &mut draft,
                &mut selected,
            );
            // A dragged secondary button is not a click: open the menu with a
            // subsequent stationary right click, as a user would.
            render(
                vec![
                    egui::Event::PointerMoved(cue),
                    click(cue, egui::PointerButton::Secondary, true),
                ],
                &mut draft,
                &mut selected,
            );
            render(
                vec![click(cue, egui::PointerButton::Secondary, false)],
                &mut draft,
                &mut selected,
            );
            let output = render(vec![], &mut draft, &mut selected);
            assert_eq!(
                selected, 1,
                "right click must select the motion cue, not the previously selected relay"
            );
            assert_eq!(
                draft.steps[1], original,
                "opening menu must not change the cue"
            );
            let mut positions = vec![];
            for s in &output.shapes {
                text_positions(&s.shape, "Duplicate", &mut positions);
            }
            let duplicate = *positions.first().expect("cue context menu did not open");
            render(
                vec![
                    egui::Event::PointerMoved(duplicate),
                    click(duplicate, egui::PointerButton::Primary, true),
                ],
                &mut draft,
                &mut selected,
            );
            render(
                vec![click(duplicate, egui::PointerButton::Primary, false)],
                &mut draft,
                &mut selected,
            );
            assert_eq!(draft.steps.len(), 3);
            assert_eq!(selected, 2);
            let mut expected = original.clone();
            expected.at_us += 1_000_000;
            assert_eq!(draft.steps[2], expected);
            assert_eq!(draft.steps[1], original);
        }
    }

    #[test]
    fn recording_color_swatches_match_the_shared_palette_in_both_themes() {
        for dark in [false, true] {
            let context = egui::Context::default();
            context.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            for color in recording_colors() {
                let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                    let rect = ui
                        .allocate_exact_size(egui::vec2(120.0, 24.0), egui::Sense::hover())
                        .0;
                    paint_recording_swatch(ui, rect, &color.id);
                });
                output.textures_delta.clear();
                assert!(output.shapes.iter().any(|shape| matches!(
                    &shape.shape, egui::epaint::Shape::Circle(circle)
                        if circle.fill == recording_color(&color.id) && circle.radius == 5.0
                )));
                assert!(output.shapes.iter().any(|shape| matches!(
                    &shape.shape, egui::epaint::Shape::Circle(circle)
                        if circle.stroke.width == 1.0 && circle.stroke.color == egui::Color32::GRAY
                )), "{} swatch lacks a light-theme outline", color.id);
            }
        }
        assert_eq!(recording_color("purple"), recording_color("violet"));
    }

    #[test]
    fn record_action_is_always_solid_red_instead_of_the_effect_color() {
        assert_eq!(record_action_color(), egui::Color32::from_rgb(205, 42, 54));
        assert_ne!(record_action_color(), recording_color("blue"));
        assert_ne!(record_action_color(), recording_color("green"));
    }

    #[test]
    fn recording_color_popup_selection_updates_header_in_the_same_frame() {
        fn flatten<'a>(shape: &'a egui::epaint::Shape, result: &mut Vec<&'a egui::epaint::Shape>) {
            if let egui::epaint::Shape::Vec(shapes) = shape {
                for shape in shapes {
                    flatten(shape, result);
                }
            } else {
                result.push(shape);
            }
        }
        let context = egui::Context::default();
        let mut color = "violet".to_string();
        // Popup fade opacity is irrelevant to checking the palette itself.
        context.all_styles_mut(|style| style.animation_time = 0.0);
        let render = |events, color: &mut String| {
            let mut picker = egui::Rect::NOTHING;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 360.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let icon = ui
                        .allocate_exact_size(egui::vec2(18.0, 24.0), egui::Sense::hover())
                        .0;
                    picker = recording_color_picker(ui, color).rect;
                    paint_recording_icon(ui, icon, color);
                },
            );
            output.textures_delta.clear();
            (output, picker)
        };
        let click = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        render(Vec::new(), &mut color);
        let (_, picker) = render(Vec::new(), &mut color);
        render(
            vec![
                egui::Event::PointerMoved(picker.center()),
                click(picker.center(), true),
            ],
            &mut color,
        );
        render(vec![click(picker.center(), false)], &mut color);
        let (output, _) = render(Vec::new(), &mut color);
        let mut shapes = Vec::new();
        for shape in &output.shapes {
            flatten(&shape.shape, &mut shapes);
        }
        for option in recording_colors() {
            assert!(shapes.iter().any(|shape| matches!(
                shape, egui::epaint::Shape::Circle(circle) if circle.fill == recording_color(&option.id)
            )), "popup is missing {} swatch; visible labels: {:?}", option.id,
                shapes.iter().filter_map(|shape| match shape {
                    egui::epaint::Shape::Text(text) => Some(&text.galley.job.text), _ => None,
                }).collect::<Vec<_>>());
        }
        let blue = shapes
            .iter()
            .find_map(|shape| match shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text.trim() == "Blue" => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                }
                _ => None,
            })
            .expect("Blue option is absent");
        render(
            vec![egui::Event::PointerMoved(blue), click(blue, true)],
            &mut color,
        );
        let (output, _) = render(vec![click(blue, false)], &mut color);
        assert_eq!(color, "blue");
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape, egui::epaint::Shape::Text(text)
                if text.galley.job.text == crate::ui::icons::RECORD
                    && text.galley.job.sections.iter().all(|section| section.format.color == recording_color("blue"))
        )), "header retained its old color after selection");
    }

    #[test]
    fn recording_panel_header_uses_the_draft_color_not_the_theme_accent() {
        let context = egui::Context::default();
        let mut app = PealayerApp::default();
        for color in ["green", "blue", "white"] {
            app.effect_library_draft.color = color.to_string();
            let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                draw_effect_capture_controls(&mut app, ui);
            });
            output.textures_delta.clear();
            assert!(output.shapes.iter().any(|shape| matches!(
                &shape.shape, egui::epaint::Shape::Text(text)
                    if text.galley.job.text.contains(crate::ui::icons::RECORD)
            )));
        }
    }

    #[test]
    fn effect_properties_use_one_searchable_icon_control() {
        let source = include_str!("effects_library.rs");
        assert!(source.contains("searchable_control_icon_picker"));
        assert!(!source.contains(concat!("effect_icon_", "preset")));
    }

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
        assert_eq!(app.effect_library_draft.engine, "auto");
    }

    #[test]
    fn new_group_is_separate_from_the_current_effect_draft() {
        let mut app = PealayerApp::default();
        app.effect_library_draft.name = "Existing draft".to_string();
        begin_new_group(&mut app);
        assert_eq!(app.effect_library_draft.name, "Existing draft");
        assert!(app.effect_group_draft.is_some());
        app.effect_group_draft.as_mut().unwrap().name = "  Cinema lighting  ".to_string();
        assert_eq!(app.effect_library_draft.name, "Existing draft");
        assert!(!app.show_effect_library_editor);
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

    #[test]
    fn sequence_duration_includes_repeated_cue_instances() {
        let steps = vec![crate::four_d::controller::HardwareMacroStep {
            at_us: 100_000,
            duration_ms: Some(250),
            repeat_count: Some(3),
            repeat_interval_ms: Some(400),
            ..Default::default()
        }];

        assert_eq!(sequence_duration_ms(&steps), 1_150);
    }

    #[test]
    fn named_melody_expands_notes_and_gaps_into_portable_beep_steps() {
        let melody = crate::four_d::controller::HardwareMelody {
            name: "attention".to_string(),
            notes: vec![
                crate::four_d::controller::HardwareMelodyNote {
                    frequency_hz: 880,
                    duration_ms: 100,
                    gap_ms: 25,
                },
                crate::four_d::controller::HardwareMelodyNote {
                    frequency_hz: 990,
                    duration_ms: 75,
                    gap_ms: 0,
                },
            ],
        };
        let mut steps = Vec::new();
        assert_eq!(append_melody_steps(&mut steps, &melody), Some(0));
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].at_us, 0);
        assert_eq!(steps[1].frequency_hz, Some(0));
        assert_eq!(steps[1].at_us, 100_000);
        assert_eq!(steps[2].at_us, 125_000);
        assert_eq!(sequence_duration_ms(&steps), 200);
    }

    #[test]
    fn strip_delay_preserves_relative_timing() {
        let mut steps = vec![
            crate::four_d::controller::HardwareMacroStep {
                at_us: 750_000,
                ..Default::default()
            },
            crate::four_d::controller::HardwareMacroStep {
                at_us: 1_250_000,
                ..Default::default()
            },
        ];

        strip_sequence_leading_delay(&mut steps);

        assert_eq!(steps[0].at_us, 0);
        assert_eq!(steps[1].at_us, 500_000);
    }

    #[test]
    fn quantize_applies_to_position_length_and_repeat_interval() {
        let mut steps = vec![crate::four_d::controller::HardwareMacroStep {
            at_us: 123_400,
            duration_ms: Some(267),
            repeat_interval_ms: Some(614),
            ..Default::default()
        }];

        quantize_sequence(&mut steps, 50);

        assert_eq!(steps[0].at_us, 100_000);
        assert_eq!(steps[0].duration_ms, Some(250));
        assert_eq!(steps[0].repeat_interval_ms, Some(600));
    }

    #[test]
    fn step_catalog_is_derived_from_the_live_board() {
        let mut capabilities = crate::four_d::controller::HardwareCapabilities {
            board_connected: true,
            supports_segment_display: true,
            supports_addressable_led: true,
            ..Default::default()
        };
        capabilities
            .relays
            .push(crate::four_d::controller::HardwareOutput {
                id: 1,
                key: "relay.1".to_string(),
                name: "Seat direction".to_string(),
                role: "motion-direction".to_string(),
                control: "seat-internal".to_string(),
            });
        let kinds = sequence_step_kinds(Some(&capabilities));
        assert!(kinds.contains(&"motion"));
        assert!(kinds.contains(&"relay"));
        assert!(kinds.contains(&"display"));
        assert!(kinds.contains(&"addressable"));
        assert!(!kinds.contains(&"rf"));
        assert!(sequence_step_kinds(None).is_empty());
    }

    #[test]
    fn semantic_actions_follow_the_edited_physical_command() {
        let mut step = crate::four_d::controller::HardwareMacroStep {
            kind: "motion".to_string(),
            target: Some(1),
            value: Some(2),
            ..Default::default()
        };
        refresh_semantic_action(&mut step);
        assert_eq!(step.action_ids, ["seat.b.down"]);

        step.kind = "relay".to_string();
        step.target = Some(4);
        step.value = Some(1);
        refresh_semantic_action(&mut step);
        assert_eq!(step.action_ids, ["relay.5.on"]);
    }

    #[test]
    fn retargeting_a_motion_cue_preserves_action_and_timing() {
        let mut step = crate::four_d::controller::HardwareMacroStep {
            at_us: 125_000,
            kind: "motion".to_string(),
            target: Some(0),
            value: Some(1),
            duration_ms: Some(750),
            action_ids: vec!["seat.a.up".to_string()],
            ..Default::default()
        };

        assert!(retarget_sequence_cue_to_lane(&mut step, "motion.1"));
        assert_eq!(step.target, Some(1));
        assert_eq!(step.value, Some(1));
        assert_eq!(step.at_us, 125_000);
        assert_eq!(step.duration_ms, Some(750));
        assert_eq!(step.action_ids, ["seat.b.up"]);

        let unchanged = step.clone();
        assert!(!retarget_sequence_cue_to_lane(&mut step, "relay.4"));
        assert_eq!(step, unchanged);
    }

    #[test]
    fn vertical_move_drag_retargets_a_motion_cue_to_the_destination_lane() {
        fn text_positions(
            shape: &egui::epaint::Shape,
            text: &str,
            positions: &mut Vec<egui::Pos2>,
        ) {
            match shape {
                egui::epaint::Shape::Vec(shapes) => {
                    for shape in shapes {
                        text_positions(shape, text, positions);
                    }
                }
                egui::epaint::Shape::Text(value) if value.galley.job.text.contains(text) => {
                    positions.push(value.pos + value.galley.rect.center().to_vec2());
                }
                _ => {}
            }
        }

        let context = egui::Context::default();
        context.all_styles_mut(|style| style.animation_time = 0.0);
        let mut draft = ControllerEffectDraft::default();
        draft.steps = vec![
            crate::four_d::controller::HardwareMacroStep {
                kind: "motion".to_string(),
                target: Some(0),
                value: Some(1),
                duration_ms: Some(500),
                action_ids: vec!["seat.a.up".to_string()],
                ..Default::default()
            },
            crate::four_d::controller::HardwareMacroStep {
                kind: "motion".to_string(),
                target: Some(1),
                value: Some(2),
                at_us: 750_000,
                duration_ms: Some(500),
                action_ids: vec!["seat.b.down".to_string()],
                ..Default::default()
            },
        ];
        let mut selected = 0;
        let render = |events, draft: &mut ControllerEffectDraft, selected: &mut usize| {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 600.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| draw_sequence_timeline(ui, draft, selected, true, true),
            );
            output.textures_delta.clear();
            output
        };
        render(vec![], &mut draft, &mut selected);
        let output = render(vec![], &mut draft, &mut selected);
        let mut up_positions = vec![];
        let mut seat_b_positions = vec![];
        for shape in &output.shapes {
            text_positions(&shape.shape, "Up", &mut up_positions);
            text_positions(&shape.shape, "Seat B", &mut seat_b_positions);
        }
        let cue = *up_positions.first().expect("Seat A Up cue was not painted");
        let seat_b = *seat_b_positions.first().expect("Seat B lane was not painted");
        let destination = egui::pos2(cue.x, seat_b.y);
        let pointer_button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };

        render(
            vec![egui::Event::PointerMoved(cue), pointer_button(cue, true)],
            &mut draft,
            &mut selected,
        );
        render(
            vec![egui::Event::PointerMoved(destination)],
            &mut draft,
            &mut selected,
        );
        render(
            vec![pointer_button(destination, false)],
            &mut draft,
            &mut selected,
        );

        assert_eq!(draft.steps[0].target, Some(1));
        assert_eq!(draft.steps[0].value, Some(1));
        assert_eq!(draft.steps[0].at_us, 0);
        assert_eq!(draft.steps[0].duration_ms, Some(500));
        assert_eq!(draft.steps[0].action_ids, ["seat.b.up"]);
    }

    #[test]
    fn effect_editor_uses_current_seat_presentation_names() {
        let capabilities = crate::four_d::controller::HardwareCapabilities {
            controls: vec![
                crate::four_d::controller::HardwareControl {
                    key: "seat.a".to_string(),
                    name: "Seat Left".to_string(),
                    ..Default::default()
                },
                crate::four_d::controller::HardwareControl {
                    key: "seat.b".to_string(),
                    name: "Seat Right".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        assert_eq!(motion_seat_name(Some(&capabilities), 0), "Seat Left");
        assert_eq!(motion_seat_name(Some(&capabilities), 1), "Seat Right");
        assert_eq!(motion_seat_name(None, 0), "Seat A");
        assert_eq!(motion_seat_name(None, 1), "Seat B");
    }
}
