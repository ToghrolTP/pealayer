use crate::app::PealayerApp;
use crate::four_d::controller::{HardwareAction, HardwareCapabilities, HardwareControl};
use eframe::egui;
use std::collections::BTreeSet;

fn selected_control(capabilities: &HardwareCapabilities, key: &str) -> Option<HardwareControl> {
    capabilities
        .controls
        .iter()
        .find(|control| control.key == key)
        .cloned()
        .or_else(|| {
            capabilities
                .relays
                .iter()
                .find(|output| output.key == key)
                .map(|output| HardwareControl {
                    key: output.key.clone(),
                    kind: "relay".to_string(),
                    name: output.name.clone(),
                    default_name: output.name.clone(),
                    control: output.control.clone(),
                    group: output.role.clone(),
                    actions: raw_relay_actions(&output.key),
                    ..Default::default()
                })
        })
        .or_else(|| {
            capabilities
                .pwm_channels
                .iter()
                .find(|output| output.key == key)
                .map(|output| HardwareControl {
                    key: output.key.clone(),
                    kind: "mosfet".to_string(),
                    name: output.name.clone(),
                    default_name: output.name.clone(),
                    control: output.control.clone(),
                    group: output.role.clone(),
                    ..Default::default()
                })
        })
}

fn raw_relay_actions(key: &str) -> Vec<HardwareAction> {
    [
        ("on", "On", crate::ui::icons::LIGHTNING),
        ("off", "Off", crate::ui::icons::STOP_CIRCLE),
    ]
    .into_iter()
    .map(|(verb, name, icon)| HardwareAction {
        id: format!("{key}.{verb}"),
        verb: verb.to_string(),
        name: name.to_string(),
        icon: icon.to_string(),
    })
    .collect()
}

fn relay_id(key: &str) -> Option<u8> {
    key.strip_prefix("relay.")?.parse().ok()
}

fn pwm_channel(capabilities: &HardwareCapabilities, key: &str) -> Option<u8> {
    capabilities
        .pwm_channels
        .iter()
        .find(|channel| channel.key == key)
        .map(|channel| channel.id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HardwareActionDispatch {
    Advertised(String),
    ControllerCommand(String),
}

fn action_dispatch(control: &HardwareControl, action: &HardwareAction) -> HardwareActionDispatch {
    if control.control.eq_ignore_ascii_case("raw-motion") {
        let side = match control.key.as_str() {
            "seat.a" => Some("left"),
            "seat.b" => Some("right"),
            _ => None,
        };
        let verb = action.verb.to_ascii_lowercase();
        if let Some(side) = side
            && matches!(verb.as_str(), "up" | "down" | "stop")
        {
            return HardwareActionDispatch::ControllerCommand(format!("relay side {side} {verb}"));
        }
    }
    // A raw relay remains directly controllable even while a board profile is
    // being configured. PCController advertises those controls so operators
    // can use them, but semantic action invocation is deliberately rejected
    // until a profile exists. Route the two stable relay verbs through the
    // controller command contract; profile-defined motion and every other
    // peripheral continue to use their advertised action IDs.
    if let Some(relay) = relay_id(&control.key) {
        match action.verb.to_ascii_lowercase().as_str() {
            "on" => {
                return HardwareActionDispatch::ControllerCommand(format!("relay {relay} on"));
            }
            "off" => {
                return HardwareActionDispatch::ControllerCommand(format!("relay {relay} off"));
            }
            _ => {}
        }
    }
    HardwareActionDispatch::Advertised(action.id.clone())
}

pub(crate) fn invoke_action(app: &PealayerApp, control: &HardwareControl, action: &HardwareAction) {
    let message = match action_dispatch(control, action) {
        HardwareActionDispatch::Advertised(action_id) => {
            crate::four_d::engine::EngineMessage::InvokeControllerAction { action_id }
        }
        HardwareActionDispatch::ControllerCommand(command) => {
            crate::four_d::engine::EngineMessage::ControllerCall {
                method: "controller.command.execute".to_string(),
                params: serde_json::json!({"command": command}),
            }
        }
    };
    let _ = app.engine_handle.sender.send(message);
}

/// Dispatch an advertised action through the same routing policy used by the
/// native Hardware Monitor. This is intentionally shared with the web/API
/// command path so raw relay and motion controls cannot drift back to semantic
/// action IDs that PCController rejects before a profile is configured.
pub(crate) fn invoke_action_by_id(
    app: &PealayerApp,
    capabilities: &HardwareCapabilities,
    action_id: &str,
) -> Result<(), String> {
    let controls = managed_controls(capabilities);
    let (control, action) = controls
        .iter()
        .find_map(|control| {
            control
                .actions
                .iter()
                .find(|action| action.id == action_id)
                .map(|action| (control, action))
        })
        .ok_or_else(|| format!("Hardware action {action_id:?} is not currently advertised"))?;
    invoke_action(app, control, action);
    Ok(())
}

fn set_relay(app: &PealayerApp, relay: u8, on: bool) {
    let _ = app
        .engine_handle
        .sender
        .send(crate::four_d::engine::EngineMessage::ControllerCall {
            method: "controller.command.execute".to_string(),
            params: serde_json::json!({
                "command": format!("relay {relay} {}", if on { "on" } else { "off" })
            }),
        });
}

pub(crate) fn set_pwm(app: &PealayerApp, channel: u8, percent: f64) {
    let raw = (percent.clamp(0.0, 100.0) * 4095.0 / 100.0).round() as u16;
    let _ = app
        .engine_handle
        .sender
        .send(crate::four_d::engine::EngineMessage::ControllerCall {
            method: "controller.pwm.set".to_string(),
            params: serde_json::json!({"channel": channel, "value": raw}),
        });
}

pub(crate) fn managed_controls(capabilities: &HardwareCapabilities) -> Vec<HardwareControl> {
    let mut controls = Vec::with_capacity(
        capabilities.controls.len() + capabilities.relays.len() + capabilities.pwm_channels.len(),
    );
    for control in &capabilities.controls {
        if !controls
            .iter()
            .any(|known: &HardwareControl| known.key == control.key)
        {
            controls.push(control.clone());
        }
    }
    for output in &capabilities.relays {
        if !controls.iter().any(|control| control.key == output.key) {
            controls.push(HardwareControl {
                key: output.key.clone(),
                kind: "relay".to_string(),
                name: output.name.clone(),
                default_name: output.name.clone(),
                control: output.control.clone(),
                group: output.role.clone(),
                actions: raw_relay_actions(&output.key),
                ..Default::default()
            });
        }
    }
    for output in &capabilities.pwm_channels {
        if !controls.iter().any(|control| control.key == output.key) {
            controls.push(HardwareControl {
                key: output.key.clone(),
                kind: "pwm".to_string(),
                name: output.name.clone(),
                default_name: output.name.clone(),
                control: output.control.clone(),
                group: output.role.clone(),
                ..Default::default()
            });
        }
    }
    controls.sort_by(|left, right| {
        channel_kind_rank(&left.kind)
            .cmp(&channel_kind_rank(&right.kind))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.order.cmp(&right.order))
            .then_with(|| left.key.cmp(&right.key))
    });
    controls
}

fn channel_kind_rank(kind: &str) -> u8 {
    match kind.to_ascii_lowercase().as_str() {
        "seat" | "motion" => 0,
        "relay" => 1,
        "pwm" | "mosfet" => 2,
        _ => 3,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChannelSelectionCommand {
    CheckAll,
    UncheckAll,
    Invert,
}

fn apply_channel_selection(
    selected: &mut BTreeSet<String>,
    available: impl IntoIterator<Item = String>,
    command: ChannelSelectionCommand,
) {
    let available = available.into_iter().collect::<BTreeSet<_>>();
    selected.retain(|key| available.contains(key));
    match command {
        ChannelSelectionCommand::CheckAll => selected.extend(available),
        ChannelSelectionCommand::UncheckAll => selected.clear(),
        ChannelSelectionCommand::Invert => {
            *selected = available.difference(selected).cloned().collect();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChannelBulkAction {
    ShowInMonitor,
    HideFromMonitor,
    Lock,
    Unlock,
    LinkTimeline,
    UnlinkTimeline,
    ShowTimelineTracks,
    HideTimelineTracks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindingEditorMode {
    Action,
    Toggle,
    Pwm,
    Hold,
}

fn binding_editor_mode(action: &crate::config::HardwareKeyBindingAction) -> BindingEditorMode {
    match action {
        crate::config::HardwareKeyBindingAction::Invoke { .. } => BindingEditorMode::Action,
        crate::config::HardwareKeyBindingAction::Toggle { .. } => BindingEditorMode::Toggle,
        crate::config::HardwareKeyBindingAction::SetPwm { .. } => BindingEditorMode::Pwm,
        crate::config::HardwareKeyBindingAction::Hold { .. } => BindingEditorMode::Hold,
    }
}

fn hardware_binding_action_label(
    binding: &crate::config::HardwareKeyBinding,
    app: &PealayerApp,
) -> String {
    match &binding.action {
        crate::config::HardwareKeyBindingAction::Invoke { label, action_id } => {
            if label.trim().is_empty() {
                action_id.clone()
            } else {
                label.clone()
            }
        }
        crate::config::HardwareKeyBindingAction::Toggle { .. } => app.tr("Toggle"),
        crate::config::HardwareKeyBindingAction::SetPwm { percent } => {
            format!("{} {:.1}%", app.tr("Set to"), percent)
        }
        crate::config::HardwareKeyBindingAction::Hold {
            press_label,
            release_label,
            ..
        } => format!(
            "{} → {} · {} → {}",
            app.tr("Press"),
            press_label,
            app.tr("Release"),
            release_label
        ),
    }
}

fn channel_binding_summary(app: &PealayerApp, channel_key: &str) -> Option<(String, String)> {
    let bindings = app
        .hardware_key_bindings
        .iter()
        .filter(|binding| binding.channel_key == channel_key && binding.enabled)
        .collect::<Vec<_>>();
    if bindings.is_empty() {
        return None;
    }
    let short = bindings
        .iter()
        .take(2)
        .map(|binding| binding.chord.display_name())
        .collect::<Vec<_>>()
        .join(" · ");
    let short = if bindings.len() > 2 {
        format!("{short} +{}", bindings.len() - 2)
    } else {
        short
    };
    let details = bindings
        .iter()
        .map(|binding| {
            format!(
                "{} — {}{}",
                binding.chord.display_name(),
                hardware_binding_action_label(binding, app),
                if binding.global {
                    format!(" ({})", app.tr("Global"))
                } else {
                    String::new()
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some((short, details))
}

fn selected_channel_controls(
    controls: &[HardwareControl],
    selected: &BTreeSet<String>,
) -> Vec<HardwareControl> {
    controls
        .iter()
        .filter(|control| selected.contains(&control.key))
        .cloned()
        .collect()
}

fn apply_channel_bulk_action(
    app: &mut PealayerApp,
    controls: &[HardwareControl],
    selected: &BTreeSet<String>,
    action: ChannelBulkAction,
) {
    let selected_controls = selected_channel_controls(controls, selected);
    if selected_controls.is_empty() {
        return;
    }
    match action {
        ChannelBulkAction::ShowInMonitor => {
            crate::ui::layout::update_control_presentation_flags_bulk(
                app,
                &selected_controls,
                Some(false),
                None,
            );
        }
        ChannelBulkAction::HideFromMonitor => {
            crate::ui::layout::update_control_presentation_flags_bulk(
                app,
                &selected_controls,
                Some(true),
                None,
            );
        }
        ChannelBulkAction::Lock => {
            crate::ui::layout::update_control_presentation_flags_bulk(
                app,
                &selected_controls,
                None,
                Some(true),
            );
        }
        ChannelBulkAction::Unlock => {
            crate::ui::layout::update_control_presentation_flags_bulk(
                app,
                &selected_controls,
                None,
                Some(false),
            );
        }
        ChannelBulkAction::LinkTimeline | ChannelBulkAction::UnlinkTimeline => {
            let linked = action == ChannelBulkAction::LinkTimeline;
            app.set_timeline_tracks_linked(
                selected_controls.iter().map(|control| {
                    crate::four_d::models::hardware_timeline_track_key(&control.key)
                }),
                linked,
            );
        }
        ChannelBulkAction::ShowTimelineTracks | ChannelBulkAction::HideTimelineTracks => {
            let visible = action == ChannelBulkAction::ShowTimelineTracks;
            app.set_timeline_tracks_visible(
                selected_controls.iter().map(|control| {
                    crate::four_d::models::hardware_timeline_track_key(&control.key)
                }),
                visible,
            );
        }
    }
    app.set_osd(format!(
        "{} {}",
        selected_controls.len(),
        app.tr("selected channels updated")
    ));
}

fn channel_kind_label(app: &PealayerApp, kind: &str) -> String {
    match kind.to_ascii_lowercase().as_str() {
        "seat" | "motion" => app.tr("Motion / seat controls"),
        "relay" => app.tr("Relay outputs"),
        "pwm" | "mosfet" => app.tr("PWM outputs"),
        _ => app.tr("Other controls"),
    }
}

fn channel_section_key(kind: &str) -> &'static str {
    match kind.to_ascii_lowercase().as_str() {
        "seat" | "motion" => "motion",
        "relay" => "relay",
        "pwm" | "mosfet" => "pwm",
        _ => "other",
    }
}

fn channel_identity(capabilities: &HardwareCapabilities, control: &HardwareControl) -> String {
    if let Some(relay) = relay_id(&control.key) {
        return format!("R{relay}");
    }
    if let Some(channel) = pwm_channel(capabilities, &control.key) {
        return format!("CH{channel}");
    }
    match control.key.as_str() {
        "seat.a" => "A".to_string(),
        "seat.b" => "B".to_string(),
        _ => control
            .key
            .rsplit_once('.')
            .map(|(_, tail)| tail.to_ascii_uppercase())
            .unwrap_or_else(|| control.key.clone()),
    }
}

pub(crate) fn channel_is_active(
    capabilities: &HardwareCapabilities,
    control: &HardwareControl,
) -> bool {
    if let Some(relay) = relay_id(&control.key) {
        return capabilities.active_relays.contains(&relay);
    }
    crate::ui::layout::motion_control_is_active(capabilities, control)
}

fn open_detail(
    app: &mut PealayerApp,
    capabilities: &HardwareCapabilities,
    control: &HardwareControl,
) {
    app.show_hardware_channels_dialog = true;
    app.hardware_channel_detail_active = true;
    app.hardware_control_dialog_key = Some(control.key.clone());
    app.hardware_control_name_draft = control.name.clone();
    app.hardware_control_group_draft = control.group.clone();
    app.hardware_control_icon_draft = control.icon.clone();
    app.hardware_control_color_draft = if control.color.trim().is_empty() {
        "#38D27A".to_string()
    } else {
        control.color.clone()
    };
    app.hardware_control_up_color_draft = if control.up_color.trim().is_empty() {
        "#F59E0B".to_string()
    } else {
        control.up_color.clone()
    };
    app.hardware_control_down_color_draft = if control.down_color.trim().is_empty() {
        "#3B82F6".to_string()
    } else {
        control.down_color.clone()
    };
    app.hardware_control_pwm_percent = capabilities
        .pwm_channels
        .iter()
        .find(|channel| channel.key == control.key)
        .and_then(|channel| {
            (capabilities.telemetry.pwm_channel == Some(channel.id))
                .then_some(capabilities.telemetry.pwm_value.unwrap_or(0))
        })
        .map(|value| f64::from(value) * 100.0 / 4095.0)
        .unwrap_or(0.0);
}

fn parse_display_order(value: &str, peer_count: usize) -> Option<u16> {
    let display_order = value.trim().parse::<usize>().ok()?;
    if peer_count == 0 || !(1..=peer_count).contains(&display_order) {
        return None;
    }
    u16::try_from(display_order - 1).ok()
}

fn is_motion_live_verb(verb: &str) -> bool {
    matches!(verb.to_ascii_lowercase().as_str(), "up" | "down" | "stop")
}

pub(crate) fn draw_manager_live_action(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities,
    control: &HardwareControl,
) {
    let enabled = !app.estop_active && !control.locked;
    let on = channel_is_active(capabilities, control);
    let is_motion = crate::ui::layout::is_motion_control(control);
    // Motion is directional, not boolean. It must never fall through to the
    // generic relay On/Off presentation merely because a profile maps the
    // semantic control onto relay-backed outputs. A timed motion cue means
    // Up or Down at its start and Stop at its end; the live manager exposes
    // those same semantic actions.
    if is_motion {
        let stop = control
            .actions
            .iter()
            .find(|action| action.verb.eq_ignore_ascii_case("stop"));
        for action in control.actions.iter().filter(|action| {
            is_motion_live_verb(&action.verb) && !action.verb.eq_ignore_ascii_case("stop")
        }) {
            let response = ui
                .add_enabled(
                    enabled,
                    egui::Button::new(format!(
                        "{} {}",
                        crate::ui::icons::action(&action.verb),
                        crate::ui::i18n::visual_text(app.language, &action.name)
                    ))
                    .min_size(egui::vec2(56.0, 25.0)),
                )
                .on_hover_text(crate::ui::i18n::visual_text(app.language, &action.name));
            if app.motion_control_mode == crate::config::MotionControlMode::Hold
                && let Some(stop) = stop
            {
                crate::ui::layout::update_held_motion_action(
                    app, ui, &response, control, action, stop,
                );
            } else if crate::ui::layout::hardware_control_activated(app, ui, &response) {
                invoke_action(app, control, action);
            }
        }
        if let Some(stop) = crate::ui::layout::contextual_stop_action(capabilities, control) {
            let response = ui
                .add_enabled(
                    enabled,
                    egui::Button::new(format!(
                        "{} {}",
                        crate::ui::icons::action(&stop.verb),
                        crate::ui::i18n::visual_text(app.language, &stop.name)
                    ))
                    .min_size(egui::vec2(56.0, 25.0)),
                )
                .on_hover_text(crate::ui::i18n::visual_text(app.language, &stop.name));
            if crate::ui::layout::hardware_control_activated(app, ui, &response) {
                invoke_action(app, control, stop);
            }
        }
        return;
    }
    let advertised = |verb: &str| {
        control
            .actions
            .iter()
            .find(|action| action.verb.eq_ignore_ascii_case(verb))
    };
    if let (Some(on_action), Some(off_action)) = (advertised("on"), advertised("off")) {
        for (selected, action) in [(on, on_action), (!on, off_action)] {
            let caption = if action.verb.eq_ignore_ascii_case("on") {
                app.tr("On")
            } else {
                app.tr("Off")
            };
            let response = ui
                .add_enabled(
                    enabled,
                    egui::Button::new(format!(
                        "{} {caption}",
                        crate::ui::icons::action(&action.verb)
                    ))
                    .selected(selected)
                    .min_size(egui::vec2(56.0, 25.0)),
                )
                .on_hover_text(crate::ui::i18n::visual_text(app.language, &action.name));
            if crate::ui::layout::hardware_control_activated(app, ui, &response) {
                invoke_action(app, control, action);
            }
        }
    } else if let Some(relay) = relay_id(&control.key) {
        for (state, icon, label) in [
            (true, crate::ui::icons::LIGHTNING, app.tr("Turn on")),
            (false, crate::ui::icons::STOP_CIRCLE, app.tr("Turn off")),
        ] {
            let response = ui
                .add_enabled(
                    enabled,
                    egui::Button::new(format!(
                        "{icon} {}",
                        if state { app.tr("On") } else { app.tr("Off") }
                    ))
                    .selected(on == state)
                    .min_size(egui::vec2(56.0, 25.0)),
                )
                .on_hover_text(label);
            if crate::ui::layout::hardware_control_activated(app, ui, &response) {
                set_relay(app, relay, state);
            }
        }
    } else if let Some(channel) = pwm_channel(capabilities, &control.key) {
        for (percent, icon, label) in [
            (100.0, crate::ui::icons::LIGHTNING, app.tr("Turn on")),
            (0.0, crate::ui::icons::STOP_CIRCLE, app.tr("Turn off")),
        ] {
            if ui
                .add_enabled(
                    enabled,
                    egui::Button::new(format!(
                        "{icon} {}",
                        if percent > 0.0 {
                            app.tr("On")
                        } else {
                            app.tr("Off")
                        }
                    ))
                    .min_size(egui::vec2(56.0, 25.0)),
                )
                .on_hover_text(label)
                .clicked()
            {
                set_pwm(app, channel, percent);
            }
        }
    } else {
        let stop = control
            .actions
            .iter()
            .find(|action| action.verb.eq_ignore_ascii_case("stop"));
        for action in control
            .actions
            .iter()
            .filter(|action| !action.verb.eq_ignore_ascii_case("stop"))
            .take(2)
        {
            let response = ui
                .add_enabled(
                    enabled,
                    egui::Button::new(format!(
                        "{} {}",
                        crate::ui::icons::action(&action.verb),
                        crate::ui::i18n::visual_text(app.language, &action.name)
                    ))
                    .min_size(egui::vec2(56.0, 25.0)),
                )
                .on_hover_text(crate::ui::i18n::visual_text(app.language, &action.name));
            if is_motion
                && app.motion_control_mode == crate::config::MotionControlMode::Hold
                && let Some(stop) = stop
            {
                crate::ui::layout::update_held_motion_action(
                    app, ui, &response, control, action, stop,
                );
            } else if crate::ui::layout::hardware_control_activated(app, ui, &response) {
                invoke_action(app, control, action);
            }
        }
        if let Some(stop) = crate::ui::layout::contextual_stop_action(capabilities, control) {
            let response = ui
                .add_enabled(
                    enabled,
                    egui::Button::new(format!(
                        "{} {}",
                        crate::ui::icons::action(&stop.verb),
                        crate::ui::i18n::visual_text(app.language, &stop.name)
                    ))
                    .min_size(egui::vec2(56.0, 25.0)),
                )
                .on_hover_text(crate::ui::i18n::visual_text(app.language, &stop.name));
            if crate::ui::layout::hardware_control_activated(app, ui, &response) {
                invoke_action(app, control, stop);
            }
        }
    }
}

/// Draw the compact, type-aware live control used by a Timeline track menu.
///
/// Timeline rows must retain the semantic shape of the advertised channel:
/// seats expose Up/Down/Stop, relays expose On/Off, and PWM channels expose a
/// continuous value editor.  Keeping this in the shared hardware-control
/// module prevents the Timeline from inventing a second (and previously
/// incorrect) generic boolean model for every peripheral.
pub(crate) fn draw_timeline_live_control(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities,
    control: &HardwareControl,
) {
    if let Some(channel) = pwm_channel(capabilities, &control.key) {
        let value_id = ui.make_persistent_id(("timeline_context_pwm_value", &control.key));
        let telemetry_raw = capabilities
            .telemetry
            .pwm_values
            .get(usize::from(channel))
            .copied()
            .flatten()
            .or_else(|| {
                (capabilities.telemetry.pwm_channel == Some(channel))
                    .then_some(capabilities.telemetry.pwm_value.unwrap_or(0))
            })
            .unwrap_or(0);
        let mut percent = ui
            .data_mut(|data| data.get_temp::<f64>(value_id))
            .unwrap_or_else(|| f64::from(telemetry_raw) * 100.0 / 4095.0);
        let response = crate::ui::layout::draw_pwm_editor_row(
            ui,
            &mut percent,
            !app.estop_active && !control.locked,
        );
        ui.data_mut(|data| data.insert_temp(value_id, percent));
        crate::ui::layout::transmit_pwm_editor_response(
            app,
            ui,
            channel,
            crate::ui::layout::pwm_raw(percent),
            response,
        );
        return;
    }

    ui.horizontal_wrapped(|ui| {
        draw_manager_live_action(app, ui, capabilities, control);
    });
}

fn draw_motion_mode_selector(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let previous = app.motion_control_mode;
    let hold = format!("{} {}", crate::ui::icons::HAND_TAP, app.tr("Push"));
    let toggle = format!("{} {}", crate::ui::icons::TOGGLE_RIGHT, app.tr("Toggle"));
    let hold_help = app.tr("Move only while the button is held");
    let toggle_help = app.tr("Keep moving until Stop is pressed");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.selectable_value(
            &mut app.motion_control_mode,
            crate::config::MotionControlMode::Toggle,
            toggle,
        )
        .on_hover_text(toggle_help);
        // This selector is hosted in a right-to-left action slot. Paint Toggle
        // first so the user-facing order remains Push, then Toggle.
        ui.selectable_value(
            &mut app.motion_control_mode,
            crate::config::MotionControlMode::Hold,
            hold,
        )
        .on_hover_text(hold_help);
    });
    if app.motion_control_mode != previous {
        app.save_config();
    }
}

const MANAGER_ROW_HEIGHT: f32 = 32.0;
const MANAGER_ROW_MARGIN: i8 = 10;
const MANAGER_ROW_VERTICAL_MARGIN: i8 = 1;
const MANAGER_NAME_HEIGHT: f32 = 27.0;
const MANAGER_NAME_FONT_SIZE: f32 = 13.0;
const MANAGER_NAME_ACTION_WIDTH: f32 = 28.0;
const MANAGER_NAME_GAP: f32 = 4.0;

fn manager_channel_is_dimmed(
    capabilities: &HardwareCapabilities,
    control: &HardwareControl,
    raw_visibility: crate::config::NonUserControlVisibility,
    track_state: Option<crate::four_d::models::TimelineTrackState>,
) -> bool {
    control.hidden
        || (raw_visibility != crate::config::NonUserControlVisibility::Shown
            && crate::ui::layout::is_non_user_control(capabilities, control))
        || track_state.is_some_and(|state| !state.linked || !state.visible)
}

/// Keep the selection frame and every child within the same fixed outer width.
/// The row hit target is registered first, so explicit child actions win clicks.
fn manager_channel_row<R>(
    ui: &mut egui::Ui,
    key: &str,
    layer_id: egui::LayerId,
    selected: bool,
    opacity: f32,
    render: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<(R, egui::Response)> {
    let width = ui.available_width();
    let content_width = (width - 2.0 * f32::from(MANAGER_ROW_MARGIN) - 2.0).max(1.0);
    let content_height = MANAGER_ROW_HEIGHT - 2.0 * f32::from(MANAGER_ROW_VERTICAL_MARGIN) - 2.0;
    let rect = egui::Rect::from_min_size(
        ui.next_widget_position(),
        egui::vec2(width, MANAGER_ROW_HEIGHT),
    );
    let id = ui.make_persistent_id(("manager-channel-row", key));
    ui.scope_builder(egui::UiBuilder::new().id(id).layer_id(layer_id), |ui| {
        ui.multiply_opacity(opacity);
        let background = ui.interact(rect, ui.id().with("manage"), egui::Sense::click());
        let content = egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(
                MANAGER_ROW_MARGIN,
                MANAGER_ROW_VERTICAL_MARGIN,
            ))
            .corner_radius(6.0)
            .fill(if selected {
                ui.visuals().selection.bg_fill.gamma_multiply(0.22)
            } else {
                egui::Color32::TRANSPARENT
            })
            .stroke(egui::Stroke::new(
                1.0,
                if selected {
                    ui.visuals().selection.bg_fill.gamma_multiply(0.82)
                } else {
                    egui::Color32::TRANSPARENT
                },
            ))
            .show(ui, |ui| {
                ui.set_width(content_width);
                ui.allocate_ui_with_layout(
                    egui::vec2(content_width, content_height),
                    egui::Layout::left_to_right(egui::Align::Center),
                    render,
                )
                .inner
            })
            .inner;
        (content, background)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagerNameAction {
    Manage,
    Rename,
    Confirm,
    Cancel,
}

/// Both modes own the same name-and-actions slot; Confirm and Cancel reduce
/// the edit's width rather than shifting the channel's remaining controls.
fn manager_channel_name(
    ui: &mut egui::Ui,
    key: &str,
    name: &str,
    draft: &mut String,
    editing: bool,
    request_focus: bool,
    width: f32,
    input_id: egui::Id,
    manage_help: &str,
    rename_help: &str,
    confirm_help: &str,
    cancel_help: &str,
) -> Option<ManagerNameAction> {
    let id = ui.make_persistent_id(("manager-channel-name-slot", key));
    ui.scope_builder(egui::UiBuilder::new().id(id), |ui| {
        ui.spacing_mut().item_spacing.x = MANAGER_NAME_GAP;
        ui.spacing_mut().interact_size.y = MANAGER_NAME_HEIGHT;
        ui.spacing_mut().button_padding = egui::vec2(4.0, 2.0);
        ui.allocate_ui_with_layout(
            egui::vec2(width, MANAGER_NAME_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let button = |ui: &mut egui::Ui, role: &str, icon: &str, tooltip: &str| {
                    let id = ui.make_persistent_id(role);
                    ui.scope_builder(egui::UiBuilder::new().id(id), |ui| {
                        ui.add_sized(
                            [MANAGER_NAME_ACTION_WIDTH, MANAGER_NAME_HEIGHT],
                            egui::Button::new(
                                egui::RichText::new(icon).size(MANAGER_NAME_FONT_SIZE),
                            ),
                        )
                        .on_hover_text(tooltip)
                    })
                    .inner
                };
                if editing {
                    let input_width =
                        (width - 2.0 * (MANAGER_NAME_ACTION_WIDTH + MANAGER_NAME_GAP)).max(1.0);
                    let edit = ui.add_sized(
                        [input_width, MANAGER_NAME_HEIGHT],
                        egui::TextEdit::singleline(draft)
                            .id(input_id)
                            .font(egui::FontId::proportional(MANAGER_NAME_FONT_SIZE))
                            .margin(egui::Margin::symmetric(4, 0))
                            .vertical_align(egui::Align::Center),
                    );
                    if request_focus {
                        edit.request_focus();
                    }
                    let confirm = button(ui, "confirm", crate::ui::icons::CHECK, confirm_help);
                    let cancel = button(ui, "cancel", crate::ui::icons::X, cancel_help);
                    if cancel.clicked()
                        || (edit.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)))
                    {
                        Some(ManagerNameAction::Cancel)
                    } else if confirm.clicked()
                        || ((edit.has_focus() || edit.lost_focus())
                            && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        Some(ManagerNameAction::Confirm)
                    } else {
                        None
                    }
                } else {
                    let caption_width =
                        (width - MANAGER_NAME_ACTION_WIDTH - MANAGER_NAME_GAP).max(1.0);
                    let caption_id = ui.make_persistent_id("caption");
                    let caption = ui
                        .scope_builder(egui::UiBuilder::new().id(caption_id), |ui| {
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(caption_width, MANAGER_NAME_HEIGHT),
                                egui::Sense::click(),
                            );
                            let galley = ui.painter().layout_no_wrap(
                                name.to_string(),
                                egui::FontId::proportional(MANAGER_NAME_FONT_SIZE),
                                ui.visuals().strong_text_color(),
                            );
                            ui.painter()
                                .with_clip_rect(rect.intersect(ui.clip_rect()))
                                .galley(
                                    egui::pos2(
                                        rect.left() + 4.0,
                                        rect.center().y - galley.size().y / 2.0,
                                    ),
                                    galley,
                                    ui.visuals().strong_text_color(),
                                );
                            response
                        })
                        .inner
                        .on_hover_text(manage_help);
                    let rename = button(ui, "rename", crate::ui::icons::PENCIL_SIMPLE, rename_help);
                    if rename.clicked() {
                        Some(ManagerNameAction::Rename)
                    } else if caption.clicked() {
                        Some(ManagerNameAction::Manage)
                    } else {
                        None
                    }
                }
            },
        )
        .inner
    })
    .inner
}

fn draw_channel_manager_page(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities,
) {
    let controls = managed_controls(&capabilities);
    let selection_id = ui.make_persistent_id("manager-channel-selection");
    let available_keys = controls
        .iter()
        .map(|control| control.key.clone())
        .collect::<BTreeSet<_>>();
    let mut selected = ui.data_mut(|data| {
        data.get_temp::<BTreeSet<String>>(selection_id)
            .unwrap_or_default()
    });
    selected.retain(|key| available_keys.contains(key));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(crate::ui::icons::CIRCUITRY)
                .size(28.0)
                .color(ui.visuals().selection.bg_fill),
        );
        ui.vertical(|ui| {
            ui.heading(crate::ui::i18n::visual_text(
                app.language,
                &capabilities.board_name,
            ));
            ui.label(
                egui::RichText::new(format!(
                    "{} · {}",
                    app.tr("All board channels"),
                    controls.len()
                ))
                .weak(),
            );
        });
    });
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    let mut pending_selection = None;
    let mut pending_bulk_action = None;
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "{} {}",
                crate::ui::icons::CHECK_SQUARE,
                if selected.is_empty() {
                    app.tr("No channels selected")
                } else {
                    format!("{} {}", selected.len(), app.tr("selected"))
                }
            ))
            .strong(),
        );
        ui.separator();
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::SELECTION_ALL,
                app.tr("Check all")
            ))
            .clicked()
        {
            pending_selection = Some(ChannelSelectionCommand::CheckAll);
        }
        if ui
            .add_enabled(
                !selected.is_empty(),
                egui::Button::new(format!("{} {}", crate::ui::icons::X, app.tr("Uncheck all"))),
            )
            .clicked()
        {
            pending_selection = Some(ChannelSelectionCommand::UncheckAll);
        }
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::ARROW_CLOCKWISE,
                app.tr("Invert selection")
            ))
            .clicked()
        {
            pending_selection = Some(ChannelSelectionCommand::Invert);
        }
        ui.add_enabled_ui(!selected.is_empty(), |ui| {
            egui::containers::menu::MenuButton::from_button(egui::Button::new(format!(
                "{} {}",
                crate::ui::icons::LIST_CHECKS,
                app.tr("Bulk actions")
            )))
            .ui(ui, |ui| {
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::EYE,
                        app.tr("Show in Hardware Monitor")
                    ))
                    .clicked()
                {
                    pending_bulk_action = Some(ChannelBulkAction::ShowInMonitor);
                    ui.close();
                }
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::EYE_SLASH,
                        app.tr("Hide from Hardware Monitor")
                    ))
                    .clicked()
                {
                    pending_bulk_action = Some(ChannelBulkAction::HideFromMonitor);
                    ui.close();
                }
                ui.separator();
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::LOCK,
                        app.tr("Lock channels")
                    ))
                    .clicked()
                {
                    pending_bulk_action = Some(ChannelBulkAction::Lock);
                    ui.close();
                }
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::POWER,
                        app.tr("Unlock channels")
                    ))
                    .clicked()
                {
                    pending_bulk_action = Some(ChannelBulkAction::Unlock);
                    ui.close();
                }
                ui.separator();
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::LINK,
                        app.tr("Link to timeline")
                    ))
                    .clicked()
                {
                    pending_bulk_action = Some(ChannelBulkAction::LinkTimeline);
                    ui.close();
                }
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::LINK_SIMPLE,
                        app.tr("Unlink from timeline")
                    ))
                    .clicked()
                {
                    pending_bulk_action = Some(ChannelBulkAction::UnlinkTimeline);
                    ui.close();
                }
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::EYE,
                        app.tr("Show timeline tracks")
                    ))
                    .clicked()
                {
                    pending_bulk_action = Some(ChannelBulkAction::ShowTimelineTracks);
                    ui.close();
                }
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::EYE_SLASH,
                        app.tr("Hide timeline tracks")
                    ))
                    .clicked()
                {
                    pending_bulk_action = Some(ChannelBulkAction::HideTimelineTracks);
                    ui.close();
                }
            });
        });
    });
    if let Some(command) = pending_selection {
        apply_channel_selection(&mut selected, available_keys.iter().cloned(), command);
    }
    if let Some(action) = pending_bulk_action {
        apply_channel_bulk_action(app, &controls, &selected, action);
    }
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(2.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut current_section = String::new();
            for control in &controls {
                let section = channel_section_key(&control.kind);
                if current_section != section {
                    if !current_section.is_empty() {
                        ui.add_space(7.0);
                    }
                    current_section = section.to_string();
                    let count = controls
                        .iter()
                        .filter(|candidate| channel_section_key(&candidate.kind) == section)
                        .count();
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(crate::ui::icons::control(
                                &control.kind,
                                &control.icon,
                            ))
                            .color(ui.visuals().selection.bg_fill),
                        );
                        ui.strong(channel_kind_label(app, &control.kind));
                        ui.label(egui::RichText::new(count.to_string()).small().weak());
                        if section == "motion" {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| draw_motion_mode_selector(app, ui),
                            );
                        }
                    });
                    ui.separator();
                }

                let edit_id = ui.make_persistent_id(("manager-channel-edit", &control.key));
                let draft_id = ui.make_persistent_id(("manager-channel-draft", &control.key));
                let focus_pending_id =
                    ui.make_persistent_id(("manager-channel-focus", &control.key));
                let text_edit_id = ui.make_persistent_id(("manager-channel-input", &control.key));
                let order_draft_id = ui.make_persistent_id(("manager-channel-order", &control.key));
                let order_edit_id =
                    ui.make_persistent_id(("manager-channel-order-input", &control.key));
                let timeline_track_key =
                    crate::four_d::models::hardware_timeline_track_key(&control.key);
                let binding_summary = channel_binding_summary(app, &control.key);
                let binding_width = if binding_summary.is_some() {
                    128.0
                } else {
                    0.0
                };
                let editing = ui.data_mut(|data| data.get_temp::<bool>(edit_id).unwrap_or(false));
                let row_height = MANAGER_ROW_HEIGHT;
                let row_width = ui.available_width();
                let predicted_rect = egui::Rect::from_min_size(
                    ui.next_widget_position(),
                    egui::vec2(row_width, row_height),
                );
                let row_hovered = ui
                    .ctx()
                    .pointer_hover_pos()
                    .is_some_and(|pointer| predicted_rect.contains(pointer));
                let row_selected = selected.contains(&control.key);
                let order_focused = ui.memory(|memory| memory.has_focus(order_edit_id));
                let order_alpha = ui.ctx().animate_bool(
                    egui::Id::new(("manager-order-visible", &control.key)),
                    row_hovered || order_focused,
                );
                let dragging = crate::ui::layout::hardware_channel_is_dragging(ui, &control.key);
                // Ordinary rows must stay on the modal's own layer. Painting
                // them on `Order::Middle` puts them behind this foreground
                // window, producing a list made only of blank separators.
                // Promote just the actively dragged row to its own layer so
                // its shapes can move without covering menus, row actions, or
                // the window resize grip during normal interaction.
                let parent_layer_id = ui.layer_id();
                let drag_layer_id = egui::LayerId::new(
                    parent_layer_id.order,
                    egui::Id::new(("hardware-channel-manager-drag", &control.key)),
                );
                let layer_id = if dragging {
                    drag_layer_id
                } else {
                    parent_layer_id
                };
                let dimmed = manager_channel_is_dimmed(
                    &capabilities,
                    control,
                    app.non_user_control_visibility,
                    app.timeline.track_states.get(&timeline_track_key).copied(),
                );
                let row = manager_channel_row(
                    ui,
                    &control.key,
                    layer_id,
                    row_selected,
                    if dimmed || dragging { 0.58 } else { 1.0 },
                    |ui| {
                        let mut checked = selected.contains(&control.key);
                        if ui
                            .checkbox(&mut checked, "")
                            .on_hover_text(app.tr("Select for bulk actions"))
                            .changed()
                        {
                            if checked {
                                selected.insert(control.key.clone());
                            } else {
                                selected.remove(&control.key);
                            }
                        }
                        let indicator_color = if channel_is_active(&capabilities, control) {
                            if matches!(control.kind.as_str(), "seat" | "motion") {
                                crate::ui::layout::motion_direction_color(
                                    control,
                                    crate::ui::layout::motion_control_direction(
                                        &capabilities,
                                        control,
                                    ),
                                )
                            } else {
                                egui::Color32::from_rgb(52, 211, 153)
                            }
                        } else {
                            ui.visuals().widgets.noninteractive.bg_stroke.color
                        };
                        let (indicator_rect, _) =
                            ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                        ui.painter()
                            .circle_filled(indicator_rect.center(), 4.5, indicator_color);
                        crate::ui::layout::hardware_channel_drag_handle(app, ui, control);
                        ui.label(crate::ui::icons::control(&control.kind, &control.icon));
                        ui.add_sized(
                            [38.0, 25.0],
                            egui::Label::new(
                                egui::RichText::new(channel_identity(&capabilities, control))
                                    .monospace()
                                    .weak(),
                            ),
                        )
                        .on_hover_text(app.tr("Board channel"));

                        let mut draft = ui.data_mut(|data| {
                            data.get_temp::<String>(draft_id)
                                .unwrap_or_else(|| control.name.clone())
                        });
                        let request_focus = editing
                            && ui.data_mut(|data| {
                                data.remove_temp::<bool>(focus_pending_id).unwrap_or(false)
                            });
                        // Reserve the maximum live-action width (including
                        // contextual motion Stop), order controls and menu.
                        let name_width = (ui.available_width() - 368.0 - binding_width).max(130.0);
                        let name = crate::ui::i18n::visual_text(app.language, &control.name);
                        let name_action = manager_channel_name(
                            ui,
                            &control.key,
                            &name,
                            &mut draft,
                            editing,
                            request_focus,
                            name_width,
                            text_edit_id,
                            &app.tr("Manage..."),
                            &app.tr("Rename"),
                            &app.tr("Confirm rename"),
                            &app.tr("Cancel"),
                        );
                        if editing {
                            ui.data_mut(|data| data.insert_temp(draft_id, draft.clone()));
                        }
                        match name_action {
                            Some(ManagerNameAction::Manage) => {
                                open_detail(app, &capabilities, control)
                            }
                            Some(ManagerNameAction::Confirm) => {
                                crate::ui::layout::update_control_name(
                                    app,
                                    &capabilities,
                                    control,
                                    draft,
                                );
                                ui.data_mut(|data| data.insert_temp(edit_id, false));
                            }
                            Some(ManagerNameAction::Cancel) => {
                                ui.data_mut(|data| data.insert_temp(edit_id, false));
                            }
                            Some(ManagerNameAction::Rename) => {
                                ui.data_mut(|data| {
                                    data.insert_temp(draft_id, control.name.clone());
                                    data.insert_temp(edit_id, true);
                                    data.insert_temp(focus_pending_id, true);
                                });
                            }
                            None => {}
                        }

                        if let Some((shortcut, details)) = binding_summary.as_ref()
                            && ui
                                .add_sized(
                                    [124.0, 25.0],
                                    egui::Button::new(format!(
                                        "{} {}",
                                        crate::ui::icons::KEYBOARD,
                                        shortcut
                                    )),
                                )
                                .on_hover_text(details)
                                .clicked()
                        {
                            app.open_hardware_bindings_for_channel(&control.key);
                        }

                        draw_manager_live_action(app, ui, &capabilities, control);
                        let peers = controls
                            .iter()
                            .filter(|candidate| candidate.kind == control.kind)
                            .collect::<Vec<_>>();
                        let position = peers
                            .iter()
                            .position(|candidate| candidate.key == control.key)
                            .unwrap_or_default();
                        ui.scope(|ui| {
                            ui.multiply_opacity(order_alpha);
                            ui.spacing_mut().item_spacing.x = 3.0;
                            let mut order_draft = ui.data_mut(|data| {
                                data.get_temp::<String>(order_draft_id)
                                    .unwrap_or_else(|| (control.order + 1).to_string())
                            });
                            let order_valid =
                                parse_display_order(&order_draft, peers.len()).is_some();
                            let order_edit = ui
                                .scope(|ui| {
                                    if !order_valid {
                                        ui.visuals_mut().widgets.active.bg_stroke =
                                            egui::Stroke::new(1.0, ui.visuals().error_fg_color);
                                    }
                                    ui.add_sized(
                                        [42.0, 25.0],
                                        egui::TextEdit::singleline(&mut order_draft)
                                            .id(order_edit_id)
                                            .horizontal_align(egui::Align::Center)
                                            .vertical_align(egui::Align::Center)
                                            .font(egui::TextStyle::Monospace)
                                            .char_limit(3),
                                    )
                                })
                                .inner
                                .on_hover_text(format!("{} · 1–{}", app.tr("Order"), peers.len()));
                            if order_edit.changed() {
                                ui.data_mut(|data| {
                                    data.insert_temp(order_draft_id, order_draft.clone())
                                });
                            }
                            let commit_order = order_edit.lost_focus()
                                || (order_edit.has_focus()
                                    && ui.input(|input| input.key_pressed(egui::Key::Enter)));
                            if commit_order {
                                if let Some(value) = parse_display_order(&order_draft, peers.len())
                                {
                                    crate::ui::layout::set_control_order(
                                        app,
                                        &capabilities,
                                        &control.key,
                                        value,
                                    );
                                    ui.data_mut(|data| {
                                        data.remove_temp::<String>(order_draft_id);
                                    });
                                } else {
                                    app.set_osd(format!(
                                        "{} 1–{}",
                                        app.tr("Order must be between"),
                                        peers.len()
                                    ));
                                    order_edit.request_focus();
                                }
                            }
                            if ui
                                .add_enabled(
                                    position > 0,
                                    egui::Button::new(crate::ui::icons::ARROW_UP)
                                        .min_size(egui::vec2(27.0, 25.0)),
                                )
                                .on_hover_text(app.tr("Move up"))
                                .clicked()
                            {
                                crate::ui::layout::move_control_by(
                                    app,
                                    &capabilities,
                                    &control.key,
                                    -1,
                                );
                            }
                            if ui
                                .add_enabled(
                                    position + 1 < peers.len(),
                                    egui::Button::new(crate::ui::icons::ARROW_DOWN)
                                        .min_size(egui::vec2(27.0, 25.0)),
                                )
                                .on_hover_text(app.tr("Move down"))
                                .clicked()
                            {
                                crate::ui::layout::move_control_by(
                                    app,
                                    &capabilities,
                                    &control.key,
                                    1,
                                );
                            }
                        });
                        egui::containers::menu::MenuButton::from_button(
                            egui::Button::new(crate::ui::icons::DOTS_THREE)
                                .min_size(egui::vec2(30.0, 25.0)),
                        )
                        .ui(ui, |ui| {
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::SLIDERS_HORIZONTAL,
                                    app.tr("Manage...")
                                ))
                                .clicked()
                            {
                                open_detail(app, &capabilities, control);
                                ui.close();
                            }
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::KEYBOARD,
                                    app.tr("Keyboard bindings...")
                                ))
                                .clicked()
                            {
                                app.open_hardware_bindings_for_channel(&control.key);
                                ui.close();
                            }
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::PUSH_PIN,
                                    app.tr("Pin to top")
                                ))
                                .clicked()
                            {
                                crate::ui::layout::set_control_order(
                                    app,
                                    &capabilities,
                                    &control.key,
                                    0,
                                );
                                ui.close();
                            }
                            let track_state = app.timeline.track_state(&timeline_track_key);
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::LINK,
                                    if track_state.linked {
                                        app.tr("Unlink from timeline")
                                    } else {
                                        app.tr("Link to timeline")
                                    }
                                ))
                                .clicked()
                            {
                                app.set_timeline_track_linked(
                                    &timeline_track_key,
                                    !track_state.linked,
                                );
                                ui.close();
                            }
                            if ui
                                .add_enabled(
                                    track_state.linked,
                                    egui::Button::new(format!(
                                        "{} {}",
                                        if track_state.visible {
                                            crate::ui::icons::EYE_SLASH
                                        } else {
                                            crate::ui::icons::EYE
                                        },
                                        if track_state.visible {
                                            app.tr("Hide timeline track")
                                        } else {
                                            app.tr("Show timeline track")
                                        }
                                    )),
                                )
                                .clicked()
                            {
                                app.set_timeline_track_visible(
                                    &timeline_track_key,
                                    !track_state.visible,
                                );
                                ui.close();
                            }
                            ui.separator();
                            if ui
                                .button(format!(
                                    "{} {}",
                                    if control.hidden {
                                        crate::ui::icons::EYE
                                    } else {
                                        crate::ui::icons::EYE_SLASH
                                    },
                                    if control.hidden {
                                        app.tr("Show in Hardware Monitor")
                                    } else {
                                        app.tr("Hide from Hardware Monitor")
                                    }
                                ))
                                .clicked()
                            {
                                crate::ui::layout::update_control_presentation_flags(
                                    app,
                                    &capabilities,
                                    control,
                                    Some(!control.hidden),
                                    None,
                                );
                                ui.close();
                            }
                            if ui
                                .button(format!(
                                    "{} {}",
                                    if control.locked {
                                        crate::ui::icons::LOCK
                                    } else {
                                        crate::ui::icons::POWER
                                    },
                                    if control.locked {
                                        app.tr("Unlock channel")
                                    } else {
                                        app.tr("Lock channel")
                                    }
                                ))
                                .clicked()
                            {
                                crate::ui::layout::update_control_presentation_flags(
                                    app,
                                    &capabilities,
                                    control,
                                    None,
                                    Some(!control.locked),
                                );
                                ui.close();
                            }
                        })
                        .0
                        .on_hover_text(app.tr("Channel actions"));
                    },
                );
                if row.inner.1.clicked() && !editing && !dragging {
                    open_detail(app, &capabilities, control);
                }
                let row_rect = row.response.rect;
                // If this frame started the drag, the row was still painted
                // on the parent layer. Transform an empty private layer for
                // that one frame rather than accidentally moving the entire
                // dialog. The next frame paints and moves the row on the
                // private drag layer.
                let transform_layer_id = if dragging {
                    drag_layer_id
                } else {
                    egui::LayerId::new(
                        parent_layer_id.order,
                        egui::Id::new(("hardware-channel-manager-drag-pending", &control.key)),
                    )
                };
                crate::ui::layout::finish_hardware_channel_card(
                    ui,
                    control,
                    row_rect,
                    transform_layer_id,
                );
                if let Some(drop) =
                    crate::ui::layout::hardware_channel_drop_target(ui, row_rect, control)
                {
                    crate::ui::layout::persist_channel_drop(app, &capabilities, drop);
                }
                ui.add(egui::Separator::default().spacing(0.0));
            }
        });
    ui.data_mut(|data| data.insert_temp(selection_id, selected));
    // A release over the source row, outside the window, or over an
    // incompatible channel is not a drop. Clear the transient drag in all of
    // those cases so a cancelled drag cannot leave a transformed layer above
    // the row controls and make subsequent actions appear dead.
    crate::ui::layout::clear_released_hardware_channel_drag(ui);
}

fn channel_detail_section(
    ui: &mut egui::Ui,
    icon: &str,
    title: &str,
    body: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color.gamma_multiply(0.42))
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(format!("{icon}  {title}"))
                    .size(15.0)
                    .strong()
                    .color(ui.visuals().widgets.hovered.fg_stroke.color),
            );
            ui.add_space(10.0);
            body(ui);
        });
    ui.add_space(9.0);
}

fn hardware_binding_platform_label(app: &PealayerApp) -> String {
    if cfg!(target_os = "windows") {
        app.tr("Windows global hotkey")
    } else if cfg!(target_os = "macos") {
        app.tr("macOS global hotkey")
    } else {
        app.tr("X11 global hotkey")
    }
}

fn default_binding_action(
    control: &HardwareControl,
    mode: BindingEditorMode,
) -> crate::config::HardwareKeyBindingAction {
    let action_with_verb = |verb: &str| {
        control
            .actions
            .iter()
            .find(|action| action.verb.eq_ignore_ascii_case(verb))
    };
    let first = control.actions.first();
    match mode {
        BindingEditorMode::Action => first
            .map(|action| crate::config::HardwareKeyBindingAction::Invoke {
                action_id: action.id.clone(),
                label: action.name.clone(),
            })
            .unwrap_or_default(),
        BindingEditorMode::Toggle => {
            let on = action_with_verb("on");
            let off = action_with_verb("off");
            match (on, off) {
                (Some(on), Some(off)) => crate::config::HardwareKeyBindingAction::Toggle {
                    on_action_id: on.id.clone(),
                    off_action_id: off.id.clone(),
                },
                _ => default_binding_action(control, BindingEditorMode::Action),
            }
        }
        BindingEditorMode::Pwm => {
            crate::config::HardwareKeyBindingAction::SetPwm { percent: 100.0 }
        }
        BindingEditorMode::Hold => {
            let press = action_with_verb("on")
                .or_else(|| action_with_verb("up"))
                .or(first);
            let release = action_with_verb("off")
                .or_else(|| action_with_verb("stop"))
                .or(first);
            match (press, release) {
                (Some(press), Some(release)) => crate::config::HardwareKeyBindingAction::Hold {
                    press_action_id: press.id.clone(),
                    release_action_id: release.id.clone(),
                    press_label: press.name.clone(),
                    release_label: release.name.clone(),
                },
                _ => default_binding_action(control, BindingEditorMode::Action),
            }
        }
    }
}

fn draw_binding_action_editor(
    app: &PealayerApp,
    ui: &mut egui::Ui,
    control: &HardwareControl,
    draft: &mut crate::config::HardwareKeyBinding,
) {
    let has_toggle = control
        .actions
        .iter()
        .any(|action| action.verb.eq_ignore_ascii_case("on"))
        && control
            .actions
            .iter()
            .any(|action| action.verb.eq_ignore_ascii_case("off"));
    let is_pwm = matches!(control.kind.as_str(), "pwm" | "mosfet");
    let has_hold_pair = control.actions.len() >= 2;
    let mut mode = binding_editor_mode(&draft.action);
    egui::ComboBox::from_id_salt(("hardware-binding-mode", &draft.id))
        .selected_text(match mode {
            BindingEditorMode::Action => app.tr("Run one action"),
            BindingEditorMode::Toggle => app.tr("Toggle On / Off"),
            BindingEditorMode::Pwm => app.tr("Set PWM level"),
            BindingEditorMode::Hold => app.tr("While key is held"),
        })
        .width(250.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(
                &mut mode,
                BindingEditorMode::Action,
                app.tr("Run one action"),
            );
            ui.add_enabled_ui(has_toggle, |ui| {
                ui.selectable_value(
                    &mut mode,
                    BindingEditorMode::Toggle,
                    app.tr("Toggle On / Off"),
                );
            });
            ui.add_enabled_ui(is_pwm, |ui| {
                ui.selectable_value(&mut mode, BindingEditorMode::Pwm, app.tr("Set PWM level"));
            });
            ui.add_enabled_ui(has_hold_pair, |ui| {
                ui.selectable_value(
                    &mut mode,
                    BindingEditorMode::Hold,
                    app.tr("While key is held"),
                );
            });
        });
    if mode != binding_editor_mode(&draft.action) {
        draft.action = default_binding_action(control, mode);
    }
    ui.add_space(8.0);

    match &mut draft.action {
        crate::config::HardwareKeyBindingAction::Invoke { action_id, label } => {
            let selected = control
                .actions
                .iter()
                .find(|action| action.id == *action_id)
                .map(|action| action.name.as_str())
                .unwrap_or(action_id);
            egui::ComboBox::from_id_salt(("hardware-binding-action", &draft.id))
                .selected_text(selected)
                .width(250.0)
                .show_ui(ui, |ui| {
                    for action in &control.actions {
                        if ui
                            .selectable_label(action.id == *action_id, &action.name)
                            .clicked()
                        {
                            *action_id = action.id.clone();
                            *label = action.name.clone();
                            ui.close();
                        }
                    }
                });
        }
        crate::config::HardwareKeyBindingAction::Toggle { .. } => {
            ui.label(
                app.tr(
                    "Each press chooses On or Off from the latest board-reported channel state.",
                ),
            );
        }
        crate::config::HardwareKeyBindingAction::SetPwm { percent } => {
            ui.horizontal(|ui| {
                ui.add(
                    egui::Slider::new(percent, 0.0..=100.0)
                        .suffix("%")
                        .fixed_decimals(1),
                );
                ui.add(
                    egui::DragValue::new(percent)
                        .range(0.0..=100.0)
                        .speed(0.1)
                        .suffix("%"),
                );
            });
        }
        crate::config::HardwareKeyBindingAction::Hold {
            press_action_id,
            release_action_id,
            press_label,
            release_label,
        } => {
            egui::Grid::new(("hardware-binding-hold-grid", &draft.id))
                .num_columns(2)
                .spacing(egui::vec2(12.0, 8.0))
                .show(ui, |ui| {
                    ui.label(app.tr("Key pressed"));
                    egui::ComboBox::from_id_salt(("binding-press", &draft.id))
                        .selected_text(press_label.as_str())
                        .width(220.0)
                        .show_ui(ui, |ui| {
                            for action in &control.actions {
                                if ui
                                    .selectable_label(action.id == *press_action_id, &action.name)
                                    .clicked()
                                {
                                    *press_action_id = action.id.clone();
                                    *press_label = action.name.clone();
                                    ui.close();
                                }
                            }
                        });
                    ui.end_row();
                    ui.label(app.tr("Key released"));
                    egui::ComboBox::from_id_salt(("binding-release", &draft.id))
                        .selected_text(release_label.as_str())
                        .width(220.0)
                        .show_ui(ui, |ui| {
                            for action in &control.actions {
                                if ui
                                    .selectable_label(action.id == *release_action_id, &action.name)
                                    .clicked()
                                {
                                    *release_action_id = action.id.clone();
                                    *release_label = action.name.clone();
                                    ui.close();
                                }
                            }
                        });
                    ui.end_row();
                });
        }
    }
}

fn draw_hardware_binding_dialog(
    app: &mut PealayerApp,
    ctx: &egui::Context,
    capabilities: &HardwareCapabilities,
) {
    let Some(channel_key) = app.hardware_binding_dialog_channel.clone() else {
        return;
    };
    let Some(control) = managed_controls(capabilities)
        .into_iter()
        .find(|control| control.key == channel_key)
    else {
        app.hardware_binding_dialog_channel = None;
        app.hardware_binding_draft = None;
        return;
    };

    let mut draft = app.hardware_binding_draft.clone();
    if app.hardware_binding_capturing {
        let events = ctx.input(|input| input.events.clone());
        if let Some(chord) = events
            .iter()
            .find_map(crate::hardware_shortcuts::chord_from_egui_event)
            && let Some(binding) = draft.as_mut()
        {
            binding.chord = chord;
            app.hardware_binding_capturing = false;
        }
    }

    let mut close_dialog = false;
    let mut add_binding = false;
    let mut edit_binding = None;
    let mut delete_binding = None;
    let mut toggle_binding = None;
    let mut save_binding = false;
    let mut cancel_edit = false;
    let geometry = crate::ui::dialog::bounded_geometry(
        ctx.content_rect(),
        28.0,
        egui::vec2(720.0, 570.0),
        egui::vec2(560.0, 420.0),
        egui::vec2(900.0, 760.0),
    );
    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::KEYBOARD,
        app.tr("Keyboard bindings")
    ))
    .id(egui::Id::new("hardware_keyboard_binding_dialog_v1"))
    .default_rect(geometry.default_rect)
    .min_size(geometry.min_size)
    .max_size(geometry.max_size)
    .constrain_to(geometry.bounds)
    .resizable(true)
    .collapsible(false)
    .title_bar(false)
    .order(egui::Order::Foreground)
    .frame(crate::ui::dialog::opaque_window_frame_from_context(ctx))
    .show(ctx, |ui| {
        let accent = ui.visuals().selection.bg_fill;
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color.gamma_multiply(0.42))
            .corner_radius(9.0)
            .inner_margin(egui::Margin::symmetric(12, 9))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(crate::ui::icons::KEYBOARD)
                            .size(22.0)
                            .color(accent),
                    );
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(app.tr("Keyboard bindings"))
                                .size(16.0)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(crate::ui::i18n::visual_text(
                                app.language,
                                &control.name,
                            ))
                            .small()
                            .weak(),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(crate::ui::icons::X)
                                    .frame(false)
                                    .min_size(egui::vec2(30.0, 30.0)),
                            )
                            .on_hover_text(app.tr("Close"))
                            .clicked()
                        {
                            close_dialog = true;
                        }
                    });
                });
            });
        ui.add_space(10.0);

        if let Some(binding) = draft.as_mut() {
            ui.horizontal(|ui| {
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                        app.tr("All bindings")
                    ))
                    .clicked()
                {
                    cancel_edit = true;
                }
                ui.label(egui::RichText::new(app.tr("Edit binding")).strong());
            });
            ui.add_space(8.0);
            crate::ui::dialog::scroll_column(ui, "hardware_binding_editor", None, |ui| {
                channel_detail_section(
                    ui,
                    crate::ui::icons::KEYBOARD,
                    &app.tr("Keyboard shortcut or hotkey"),
                    |ui| {
                        ui.horizontal_wrapped(|ui| {
                            let label = if app.hardware_binding_capturing {
                                app.tr("Press a key or key combination…")
                            } else {
                                binding.chord.display_name()
                            };
                            let button = egui::Button::new(
                                egui::RichText::new(label).size(16.0).strong(),
                            )
                            .min_size(egui::vec2(270.0, 42.0));
                            if ui.add(button).clicked() {
                                app.hardware_binding_capturing = true;
                            }
                            if app.hardware_binding_capturing
                                && ui
                                    .button(format!(
                                        "{} {}",
                                        crate::ui::icons::X,
                                        app.tr("Cancel recording")
                                    ))
                                    .clicked()
                            {
                                app.hardware_binding_capturing = false;
                            }
                        });
                        ui.label(
                            egui::RichText::new(app.tr(
                                "Choose the field, then press the exact key combination to record it.",
                            ))
                            .small()
                            .weak(),
                        );
                    },
                );
                ui.add_space(9.0);
                channel_detail_section(
                    ui,
                    crate::ui::icons::LIGHTNING,
                    &app.tr("Channel action"),
                    |ui| draw_binding_action_editor(app, ui, &control, binding),
                );
                ui.add_space(9.0);
                channel_detail_section(
                    ui,
                    crate::ui::icons::GLOBE,
                    &app.tr("Availability"),
                    |ui| {
                        ui.checkbox(&mut binding.enabled, app.tr("Binding enabled"));
                        ui.checkbox(
                            &mut binding.global,
                            format!(
                                "{} ({})",
                                app.tr("Work when Pealayer is not in the foreground"),
                                hardware_binding_platform_label(app)
                            ),
                        );
                        ui.label(
                            egui::RichText::new(if binding.global {
                                app.tr("The operating system registers and delivers this hotkey to Pealayer.")
                            } else {
                                app.tr("This shortcut is active only while Pealayer has keyboard focus.")
                            })
                            .small()
                            .weak(),
                        );
                        if binding.global
                            && !binding.chord.control
                            && !binding.chord.alt
                            && !binding.chord.shift
                            && !binding.chord.super_key
                        {
                            ui.colored_label(
                                ui.visuals().warn_fg_color,
                                app.tr("A global single-key binding may conflict with normal typing or another application."),
                            );
                        }
                        if let Some(error) = app.hardware_hotkey_runtime.errors.get(&binding.id) {
                            ui.colored_label(ui.visuals().error_fg_color, error);
                        }
                    },
                );
            });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(7.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::ui::dialog::primary_action_button(
                    ui,
                    crate::ui::icons::FLOPPY_DISK,
                    &app.tr("Save binding"),
                )
                .clicked()
                {
                    save_binding = true;
                }
                if crate::ui::dialog::action_button(
                    ui,
                    crate::ui::icons::X,
                    &app.tr("Cancel"),
                )
                .clicked()
                {
                    cancel_edit = true;
                }
            });
        } else {
            let bindings = app
                .hardware_key_bindings
                .iter()
                .filter(|binding| binding.channel_key == control.key)
                .cloned()
                .collect::<Vec<_>>();
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} · {}",
                        app.tr("Assigned bindings"),
                        bindings.len()
                    ))
                    .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(format!(
                            "{} {}",
                            crate::ui::icons::PLUS,
                            app.tr("Add binding")
                        ))
                        .clicked()
                    {
                        add_binding = true;
                    }
                });
            });
            ui.add_space(7.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if bindings.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                egui::RichText::new(app.tr(
                                    "No keyboard bindings are assigned to this channel.",
                                ))
                                .weak(),
                            );
                        });
                    }
                    for binding in bindings {
                        egui::Frame::new()
                            .fill(ui.visuals().faint_bg_color.gamma_multiply(0.45))
                            .stroke(egui::Stroke::new(
                                1.0,
                                ui.visuals().widgets.noninteractive.bg_stroke.color,
                            ))
                            .corner_radius(8.0)
                            .inner_margin(egui::Margin::symmetric(11, 8))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    let mut enabled = binding.enabled;
                                    if ui.checkbox(&mut enabled, "").changed() {
                                        toggle_binding = Some((binding.id.clone(), enabled));
                                    }
                                    ui.label(
                                        egui::RichText::new(binding.chord.display_name())
                                            .strong()
                                            .color(accent),
                                    );
                                    ui.label(hardware_binding_action_label(&binding, app));
                                    if binding.global {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} {}",
                                                crate::ui::icons::GLOBE,
                                                app.tr("Global")
                                            ))
                                            .small(),
                                        );
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .button(crate::ui::icons::TRASH)
                                                .on_hover_text(app.tr("Delete binding"))
                                                .clicked()
                                            {
                                                delete_binding = Some(binding.id.clone());
                                            }
                                            if ui
                                                .button(crate::ui::icons::PENCIL_SIMPLE)
                                                .on_hover_text(app.tr("Edit binding"))
                                                .clicked()
                                            {
                                                edit_binding = Some(binding.id.clone());
                                            }
                                        },
                                    );
                                });
                            });
                        ui.add_space(6.0);
                    }
                });
        }
    });

    if close_dialog {
        app.hardware_binding_dialog_channel = None;
        app.hardware_binding_draft = None;
        app.hardware_binding_capturing = false;
    } else if cancel_edit {
        app.hardware_binding_draft = None;
        app.hardware_binding_capturing = false;
    } else if let Some(id) = delete_binding {
        app.delete_hardware_binding(&id);
    } else if let Some((id, enabled)) = toggle_binding {
        if let Some(binding) = app
            .hardware_key_bindings
            .iter_mut()
            .find(|binding| binding.id == id)
        {
            binding.enabled = enabled;
            app.save_config();
        }
    } else if add_binding {
        app.open_hardware_binding_editor(&control, None);
    } else if let Some(id) = edit_binding {
        app.open_hardware_binding_editor(&control, Some(&id));
    } else if save_binding {
        app.hardware_binding_draft = draft;
        if let Err(error) = app.save_hardware_binding_draft() {
            app.set_osd(error);
        }
    } else {
        app.hardware_binding_draft = draft;
    }
}

fn channel_metadata_tile(ui: &mut egui::Ui, icon: &str, label: &str, value: &str) {
    egui::Frame::new()
        .fill(ui.visuals().widgets.noninteractive.weak_bg_fill)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(icon)
                        .size(16.0)
                        .color(ui.visuals().selection.bg_fill),
                );
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(label).small().weak());
                    ui.add(egui::Label::new(value).truncate());
                });
            });
        });
}

fn channel_option_toggle(
    ui: &mut egui::Ui,
    enabled: bool,
    icon: &str,
    title: &str,
    detail: &str,
    value: &mut bool,
) -> bool {
    egui::Frame::new()
        .fill(ui.visuals().widgets.noninteractive.weak_bg_fill)
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add_enabled_ui(enabled, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(icon)
                            .size(16.0)
                            .color(ui.visuals().selection.bg_fill),
                    );
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(title).strong());
                        ui.label(egui::RichText::new(detail).small().weak());
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.checkbox(value, "")
                    })
                    .inner
                })
                .inner
            })
            .inner
        })
        .inner
        .changed()
}

fn channel_color_editor(ui: &mut egui::Ui, draft: &mut String, fallback: [u8; 3]) {
    ui.horizontal(|ui| {
        let mut rgb = crate::config::parse_rgb_hex(draft).unwrap_or(fallback);
        if ui.color_edit_button_srgb(&mut rgb).changed() {
            *draft = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
        }
        ui.add_sized(
            [104.0, 28.0],
            egui::TextEdit::singleline(draft)
                .char_limit(7)
                .hint_text("#38D27A"),
        );
    });
}

fn save_channel_presentation(
    app: &mut PealayerApp,
    capabilities: &HardwareCapabilities,
    control: &HardwareControl,
) {
    let requested_name = app.hardware_control_name_draft.trim();
    let name = if requested_name.is_empty() || requested_name == control.default_name {
        String::new()
    } else {
        requested_name.to_string()
    };
    let mut fields = serde_json::json!({
        "name": name,
        "group": app.hardware_control_group_draft.trim(),
        "icon": app.hardware_control_icon_draft.trim(),
    });
    if matches!(control.kind.as_str(), "mosfet" | "pwm") {
        fields["color"] =
            serde_json::Value::String(app.hardware_control_color_draft.trim().to_string());
    }
    if matches!(control.kind.as_str(), "seat" | "motion") {
        fields["up_color"] =
            serde_json::Value::String(app.hardware_control_up_color_draft.trim().to_string());
        fields["down_color"] =
            serde_json::Value::String(app.hardware_control_down_color_draft.trim().to_string());
    }
    crate::ui::layout::update_control_presentation(
        app,
        capabilities,
        control,
        "presentation-manage",
        fields,
    );
}

fn restore_channel_presentation(
    app: &mut PealayerApp,
    capabilities: &HardwareCapabilities,
    control: &HardwareControl,
) {
    app.hardware_control_name_draft = control.default_name.clone();
    app.hardware_control_group_draft.clear();
    app.hardware_control_icon_draft.clear();
    app.hardware_control_color_draft = "#38D27A".to_string();
    app.hardware_control_up_color_draft = "#F59E0B".to_string();
    app.hardware_control_down_color_draft = "#3B82F6".to_string();
    crate::ui::layout::update_control_presentation(
        app,
        capabilities,
        control,
        "presentation-restore",
        serde_json::json!({
            "name": "",
            "group": "",
            "icon": "",
            "color": "",
            "up_color": "",
            "down_color": "",
        }),
    );
}

fn draw_channel_live_control(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities,
    control: &HardwareControl,
) {
    if !control.actions.is_empty() {
        let columns = if ui.available_width() >= 520.0 { 3 } else { 1 };
        let is_motion = crate::ui::layout::is_motion_control(control);
        let stop_action = control
            .actions
            .iter()
            .find(|action| action.verb.eq_ignore_ascii_case("stop"));
        let actions = control
            .actions
            .iter()
            .filter(|action| {
                !action.verb.eq_ignore_ascii_case("stop")
                    || crate::ui::layout::contextual_stop_action(capabilities, control).is_some()
            })
            .collect::<Vec<_>>();
        for row in actions.chunks(columns) {
            ui.columns(columns, |uis| {
                for (index, action) in row.iter().enumerate() {
                    let response = uis[index].add_enabled(
                        !app.estop_active && !control.locked,
                        egui::Button::new(format!(
                            "{}  {}",
                            crate::ui::icons::action(&action.verb),
                            crate::ui::i18n::visual_text(app.language, &action.name)
                        ))
                        .corner_radius(7.0)
                        .min_size(egui::vec2(uis[index].available_width(), 34.0)),
                    );
                    let verb = action.verb.to_ascii_lowercase();
                    if is_motion
                        && !verb.eq("stop")
                        && app.motion_control_mode == crate::config::MotionControlMode::Hold
                        && let Some(stop) = stop_action
                    {
                        crate::ui::layout::update_held_motion_action(
                            app,
                            &uis[index],
                            &response,
                            control,
                            action,
                            stop,
                        );
                        continue;
                    }
                    let activated = if is_motion || matches!(verb.as_str(), "on" | "off") {
                        crate::ui::layout::hardware_control_activated(app, &uis[index], &response)
                    } else {
                        response.clicked()
                    };
                    if activated {
                        invoke_action(app, control, action);
                    }
                }
            });
        }
    } else if let Some(relay) = relay_id(&control.key) {
        ui.columns(2, |uis| {
            for (index, (on, icon, label)) in [
                (true, crate::ui::icons::LIGHTNING, app.tr("Turn on")),
                (false, crate::ui::icons::STOP_CIRCLE, app.tr("Turn off")),
            ]
            .into_iter()
            .enumerate()
            {
                let response = uis[index].add_enabled(
                    !app.estop_active && !control.locked,
                    egui::Button::new(format!("{icon}  {label}"))
                        .corner_radius(7.0)
                        .min_size(egui::vec2(uis[index].available_width(), 36.0)),
                );
                if crate::ui::layout::hardware_control_activated(app, &uis[index], &response) {
                    set_relay(app, relay, on);
                }
            }
        });
    } else if let Some(channel) = pwm_channel(capabilities, &control.key) {
        let pwm_response = crate::ui::layout::draw_pwm_editor_row(
            ui,
            &mut app.hardware_control_pwm_percent,
            !control.locked,
        );
        crate::ui::layout::transmit_pwm_editor_response(
            app,
            ui,
            channel,
            super::layout::pwm_raw(app.hardware_control_pwm_percent),
            pwm_response,
        );
        ui.add_space(7.0);
        ui.horizontal_wrapped(|ui| {
            for percent in [0.0, 25.0, 50.0, 75.0, 100.0] {
                if ui
                    .add_enabled(
                        !control.locked,
                        egui::Button::new(format!("{percent:.0}%")).corner_radius(6.0),
                    )
                    .clicked()
                {
                    app.hardware_control_pwm_percent = percent;
                    set_pwm(app, channel, percent);
                }
            }
        });
    } else {
        ui.label(egui::RichText::new(app.tr("This item has no live actions advertised.")).weak());
    }
}

fn draw_channel_detail_page(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities,
    control: &HardwareControl,
) {
    egui::ScrollArea::vertical()
        .id_salt("hardware-channel-detail-page")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let content_width = ui.available_width().min(920.0);
            let leading_space = ((ui.available_width() - content_width) * 0.5).max(0.0);
            ui.horizontal(|ui| {
                ui.add_space(leading_space);
                ui.vertical(|ui| {
                    ui.set_width(content_width);
                    let accent = ui.visuals().selection.bg_fill;
                    egui::Frame::new()
                        .fill(accent.gamma_multiply(0.10))
                        .stroke(egui::Stroke::new(1.0, accent.gamma_multiply(0.42)))
                        .corner_radius(11.0)
                        .inner_margin(egui::Margin::same(14))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                egui::Frame::new()
                                    .fill(accent.gamma_multiply(0.20))
                                    .corner_radius(10.0)
                                    .inner_margin(egui::Margin::same(11))
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new(crate::ui::icons::control(
                                                &control.kind,
                                                &control.icon,
                                            ))
                                            .size(28.0)
                                            .color(accent),
                                        );
                                    });
                                ui.add_space(4.0);
                                ui.vertical(|ui| {
                                    ui.heading(crate::ui::i18n::visual_text(
                                        app.language,
                                        &control.name,
                                    ));
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(
                                            egui::RichText::new(channel_kind_label(
                                                app,
                                                &control.kind,
                                            ))
                                            .weak(),
                                        );
                                        ui.label(egui::RichText::new("·").weak());
                                        ui.label(
                                            egui::RichText::new(channel_identity(
                                                capabilities,
                                                control,
                                            ))
                                            .monospace()
                                            .color(accent),
                                        );
                                    });
                                });
                            });
                        });
                    ui.add_space(9.0);

                    let board_name =
                        crate::ui::i18n::visual_text(app.language, &capabilities.board_name);
                    let kind_name = channel_kind_label(app, &control.kind);
                    let channel_name = channel_identity(capabilities, control);
                    let control_name = control.control.replace('-', " ");
                    let kind_value = if control_name.is_empty() {
                        kind_name
                    } else {
                        format!("{kind_name} · {control_name}")
                    };
                    let metadata = [
                        (crate::ui::icons::CIRCUITRY, app.tr("Board"), board_name),
                        (crate::ui::icons::GAUGE, app.tr("Channel"), channel_name),
                        (
                            crate::ui::icons::SLIDERS_HORIZONTAL,
                            app.tr("Type"),
                            kind_value,
                        ),
                        (
                            crate::ui::icons::KEYBOARD,
                            app.tr("Stable key"),
                            control.key.clone(),
                        ),
                    ];
                    let metadata_columns = if content_width >= 760.0 {
                        4
                    } else if content_width >= 430.0 {
                        2
                    } else {
                        1
                    };
                    for row in metadata.chunks(metadata_columns) {
                        ui.columns(metadata_columns, |uis| {
                            for (index, (icon, label, value)) in row.iter().enumerate() {
                                channel_metadata_tile(&mut uis[index], icon, label, value);
                            }
                        });
                        ui.add_space(6.0);
                    }

                    channel_detail_section(
                        ui,
                        crate::ui::icons::PALETTE,
                        &app.tr("Presentation"),
                        |ui| {
                            let field_width = (ui.available_width() - 170.0).clamp(220.0, 620.0);
                            let icon_search_hint = app.tr("Search icons...");
                            let icon_presets_label = app.tr("Presets");
                            let icon_no_matches_label = app.tr("No matching icons");
                            let icon_default_label = app.tr("Use channel default");
                            egui::Grid::new(("hardware-control-presentation", &control.key))
                                .num_columns(2)
                                .spacing([18.0, 9.0])
                                .show(ui, |ui| {
                                    ui.label(format!(
                                        "{}  {}",
                                        crate::ui::icons::PENCIL_SIMPLE,
                                        app.tr("Name")
                                    ));
                                    let align = crate::ui::i18n::input_alignment(
                                        app.rtl,
                                        &app.hardware_control_name_draft,
                                    );
                                    ui.add_sized(
                                        [field_width, 28.0],
                                        egui::TextEdit::singleline(
                                            &mut app.hardware_control_name_draft,
                                        )
                                        .horizontal_align(align),
                                    );
                                    ui.end_row();

                                    ui.label(format!(
                                        "{}  {}",
                                        crate::ui::icons::FOLDER_OPEN,
                                        app.tr("Group")
                                    ));
                                    let align = crate::ui::i18n::input_alignment(
                                        app.rtl,
                                        &app.hardware_control_group_draft,
                                    );
                                    ui.add_sized(
                                        [field_width, 28.0],
                                        egui::TextEdit::singleline(
                                            &mut app.hardware_control_group_draft,
                                        )
                                        .horizontal_align(align),
                                    );
                                    ui.end_row();

                                    ui.label(format!(
                                        "{}  {}",
                                        crate::ui::icons::SPARKLE,
                                        app.tr("Icon")
                                    ));
                                    crate::ui::icons::searchable_icon_picker(
                                        ui,
                                        ("hardware-control-icon", &control.key),
                                        &mut app.hardware_control_icon_draft,
                                        crate::ui::icons::IconPickerConfig {
                                            language: app.language,
                                            presets: crate::ui::icons::CONTROL_ICON_PRESETS,
                                            fallback_glyph: crate::ui::icons::control(
                                                &control.kind,
                                                "",
                                            ),
                                            fallback_name: &icon_default_label,
                                            width: field_width,
                                            show_selected_name: true,
                                            search_hint: &icon_search_hint,
                                            presets_label: &icon_presets_label,
                                            no_matches_label: &icon_no_matches_label,
                                            clear_label: Some(&icon_default_label),
                                        },
                                    );
                                    ui.end_row();

                                    if matches!(control.kind.as_str(), "mosfet" | "pwm") {
                                        ui.label(format!(
                                            "{}  {}",
                                            crate::ui::icons::PALETTE,
                                            app.tr("Indicator color")
                                        ));
                                        channel_color_editor(
                                            ui,
                                            &mut app.hardware_control_color_draft,
                                            [56, 210, 122],
                                        );
                                        ui.end_row();
                                    }
                                    if matches!(control.kind.as_str(), "seat" | "motion") {
                                        let moving_up_color = app.tr("Moving up color");
                                        let moving_down_color = app.tr("Moving down color");
                                        for (icon, label, draft, fallback) in [
                                            (
                                                crate::ui::icons::ARROW_UP,
                                                moving_up_color,
                                                &mut app.hardware_control_up_color_draft,
                                                [245, 158, 11],
                                            ),
                                            (
                                                crate::ui::icons::ARROW_DOWN,
                                                moving_down_color,
                                                &mut app.hardware_control_down_color_draft,
                                                [59, 130, 246],
                                            ),
                                        ] {
                                            ui.label(format!("{icon}  {label}"));
                                            channel_color_editor(ui, draft, fallback);
                                            ui.end_row();
                                        }
                                    }
                                });

                            ui.add_space(10.0);
                            let indicator_color = if control.color.trim().is_empty() {
                                "#38D27A"
                            } else {
                                control.color.trim()
                            };
                            let up_color = if control.up_color.trim().is_empty() {
                                "#F59E0B"
                            } else {
                                control.up_color.trim()
                            };
                            let down_color = if control.down_color.trim().is_empty() {
                                "#3B82F6"
                            } else {
                                control.down_color.trim()
                            };
                            let dirty = app.hardware_control_name_draft.trim() != control.name
                                || app.hardware_control_group_draft.trim() != control.group
                                || app.hardware_control_icon_draft.trim() != control.icon
                                || (matches!(control.kind.as_str(), "mosfet" | "pwm")
                                    && app.hardware_control_color_draft.trim() != indicator_color)
                                || (matches!(control.kind.as_str(), "seat" | "motion")
                                    && (app.hardware_control_up_color_draft.trim() != up_color
                                        || app.hardware_control_down_color_draft.trim()
                                            != down_color));
                            let customized = control.name != control.default_name
                                || !control.group.is_empty()
                                || !control.icon.is_empty()
                                || !control.color.is_empty()
                                || !control.up_color.is_empty()
                                || !control.down_color.is_empty();
                            let mut restore_clicked = false;
                            let mut save_clicked = false;
                            crate::ui::dialog::action_bar(
                                ui,
                                app.rtl,
                                |ui| {
                                    restore_clicked = ui
                                        .add_enabled_ui(customized, |ui| {
                                            crate::ui::dialog::action_button(
                                                ui,
                                                crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                                                &app.tr("Restore defaults"),
                                            )
                                        })
                                        .inner
                                        .clicked();
                                },
                                |ui| {
                                    save_clicked = ui
                                        .add_enabled_ui(dirty, |ui| {
                                            crate::ui::dialog::primary_action_button(
                                                ui,
                                                crate::ui::icons::FLOPPY_DISK,
                                                &app.tr("Save presentation"),
                                            )
                                        })
                                        .inner
                                        .clicked();
                                },
                            );
                            if restore_clicked {
                                restore_channel_presentation(app, capabilities, control);
                            }
                            if save_clicked {
                                save_channel_presentation(app, capabilities, control);
                            }
                        },
                    );

                    channel_detail_section(
                        ui,
                        crate::ui::icons::SLIDERS_HORIZONTAL,
                        &app.tr("Channel behavior"),
                        |ui| {
                            let timeline_key =
                                crate::four_d::models::hardware_timeline_track_key(&control.key);
                            let timeline_state = app.timeline.track_state(&timeline_key);
                            let mut locked = control.locked;
                            let mut visible = !control.hidden;
                            let mut linked = timeline_state.linked;
                            let mut timeline_visible = timeline_state.visible;
                            let supports_policy = relay_id(&control.key).is_some()
                                || matches!(control.kind.as_str(), "mosfet" | "pwm");
                            let columns = if ui.available_width() >= 620.0 { 2 } else { 1 };

                            ui.columns(columns, |uis| {
                                if channel_option_toggle(
                                    &mut uis[0],
                                    supports_policy,
                                    crate::ui::icons::LOCK,
                                    &app.tr("Lock channel"),
                                    &app.tr("Prevent live control"),
                                    &mut locked,
                                ) {
                                    crate::ui::layout::update_control_presentation_flags(
                                        app,
                                        capabilities,
                                        control,
                                        None,
                                        Some(locked),
                                    );
                                }
                                if columns > 1
                                    && channel_option_toggle(
                                        &mut uis[1],
                                        supports_policy,
                                        crate::ui::icons::EYE,
                                        &app.tr("Hardware Monitor"),
                                        &app.tr("Show in Hardware Monitor"),
                                        &mut visible,
                                    )
                                {
                                    crate::ui::layout::update_control_presentation_flags(
                                        app,
                                        capabilities,
                                        control,
                                        Some(!visible),
                                        None,
                                    );
                                }
                            });
                            if columns == 1
                                && channel_option_toggle(
                                    ui,
                                    supports_policy,
                                    crate::ui::icons::EYE,
                                    &app.tr("Hardware Monitor"),
                                    &app.tr("Show in Hardware Monitor"),
                                    &mut visible,
                                )
                            {
                                crate::ui::layout::update_control_presentation_flags(
                                    app,
                                    capabilities,
                                    control,
                                    Some(!visible),
                                    None,
                                );
                            }
                            ui.add_space(6.0);
                            ui.columns(columns, |uis| {
                                if channel_option_toggle(
                                    &mut uis[0],
                                    true,
                                    crate::ui::icons::LINK,
                                    &app.tr("Timeline link"),
                                    &app.tr("Link channel to timeline"),
                                    &mut linked,
                                ) {
                                    app.set_timeline_track_linked(&timeline_key, linked);
                                }
                                if columns > 1
                                    && channel_option_toggle(
                                        &mut uis[1],
                                        linked,
                                        crate::ui::icons::EYE,
                                        &app.tr("Timeline visibility"),
                                        &app.tr("Show channel timeline track"),
                                        &mut timeline_visible,
                                    )
                                {
                                    app.set_timeline_track_visible(&timeline_key, timeline_visible);
                                }
                            });
                            if columns == 1
                                && channel_option_toggle(
                                    ui,
                                    linked,
                                    crate::ui::icons::EYE,
                                    &app.tr("Timeline visibility"),
                                    &app.tr("Show channel timeline track"),
                                    &mut timeline_visible,
                                )
                            {
                                app.set_timeline_track_visible(&timeline_key, timeline_visible);
                            }
                        },
                    );

                    channel_detail_section(
                        ui,
                        crate::ui::icons::KEYBOARD,
                        &app.tr("Keyboard bindings"),
                        |ui| {
                            let bindings = app
                                .hardware_key_bindings
                                .iter()
                                .filter(|binding| binding.channel_key == control.key)
                                .cloned()
                                .collect::<Vec<_>>();
                            ui.horizontal_wrapped(|ui| {
                                if bindings.is_empty() {
                                    ui.label(
                                        egui::RichText::new(app.tr(
                                            "No keyboard shortcut or global hotkey is assigned.",
                                        ))
                                        .weak(),
                                    );
                                } else {
                                    for binding in &bindings {
                                        let state = if binding.enabled {
                                            binding.chord.display_name()
                                        } else {
                                            format!(
                                                "{} ({})",
                                                binding.chord.display_name(),
                                                app.tr("disabled")
                                            )
                                        };
                                        ui.label(
                                            egui::RichText::new(state)
                                                .strong()
                                                .color(ui.visuals().selection.bg_fill),
                                        )
                                        .on_hover_text(hardware_binding_action_label(binding, app));
                                    }
                                }
                                if ui
                                    .button(format!(
                                        "{} {}",
                                        crate::ui::icons::KEYBOARD,
                                        app.tr("Manage bindings…")
                                    ))
                                    .clicked()
                                {
                                    app.open_hardware_bindings_for_channel(&control.key);
                                }
                            });
                        },
                    );

                    channel_detail_section(
                        ui,
                        crate::ui::icons::LIGHTNING,
                        &app.tr("Live control"),
                        |ui| draw_channel_live_control(app, ui, capabilities, control),
                    );
                });
            });
        });
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_hardware_channels_dialog
        && app.hardware_control_dialog_key.is_none()
        && app.hardware_binding_dialog_channel.is_none()
    {
        return;
    }
    let Some(capabilities) = app
        .advertised_hardware()
        .filter(|capabilities| capabilities.board_connected)
    else {
        // Capability discovery is asynchronous during startup and reconnect.
        // Keep the requested dialog state intact so an open channel manager
        // returns as soon as the same board becomes available instead of
        // silently destroying the user's persisted workspace state.
        return;
    };
    let selected = app
        .hardware_control_dialog_key
        .as_deref()
        .and_then(|key| selected_control(&capabilities, key));
    if app.hardware_control_dialog_key.is_some() && selected.is_none() {
        app.hardware_control_dialog_key = None;
        app.hardware_channel_detail_active = false;
    }
    app.show_hardware_channels_dialog = true;

    let mut close_requested = false;
    let geometry = crate::ui::dialog::bounded_geometry(
        ui.ctx().content_rect(),
        24.0,
        egui::vec2(940.0, 760.0),
        egui::vec2(680.0, 500.0),
        egui::vec2(1_100.0, 820.0),
    );
    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::SLIDERS_HORIZONTAL,
        app.tr("Manage channels")
    ))
    .id(egui::Id::new("hardware_channels_dialog_v2"))
    .default_rect(geometry.default_rect)
    .min_size(geometry.min_size)
    .max_size(geometry.max_size)
    .resizable(true)
    .order(egui::Order::Foreground)
    .frame(crate::ui::dialog::opaque_window_frame(ui))
    .constrain_to(geometry.bounds)
    .title_bar(false)
    .collapsible(false)
    .show(ui.ctx(), |ui| {
        let accent = ui.visuals().selection.bg_fill;
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color.gamma_multiply(0.38))
            .corner_radius(9.0)
            .inner_margin(egui::Margin::symmetric(11, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    egui::Frame::new()
                        .fill(accent.gamma_multiply(0.18))
                        .corner_radius(7.0)
                        .inner_margin(egui::Margin::same(7))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(crate::ui::icons::SLIDERS_HORIZONTAL)
                                    .size(18.0)
                                    .color(accent),
                            );
                        });
                    ui.add_space(2.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(app.tr("Manage channels"))
                                .size(16.0)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(crate::ui::i18n::visual_text(
                                app.language,
                                &capabilities.board_name,
                            ))
                            .small()
                            .weak(),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(crate::ui::icons::X)
                                    .frame(false)
                                    .min_size(egui::vec2(30.0, 30.0)),
                            )
                            .on_hover_text(app.tr("Close"))
                            .clicked()
                        {
                            close_requested = true;
                        }
                    });
                });
            });
        ui.add_space(7.0);
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color.gamma_multiply(0.22))
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(5))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let on_channels = !app.hardware_channel_detail_active;
                    if crate::ui::dialog::navigation_button(
                        ui,
                        on_channels,
                        crate::ui::icons::LIST_CHECKS,
                        &app.tr("All channels"),
                        150.0,
                    )
                    .clicked()
                    {
                        app.hardware_channel_detail_active = false;
                    }
                    if let Some(control) = selected.as_ref() {
                        let label = format!(
                            "{}",
                            crate::ui::i18n::visual_text(app.language, &control.name)
                        );
                        if crate::ui::dialog::navigation_button(
                            ui,
                            app.hardware_channel_detail_active,
                            crate::ui::icons::control(&control.kind, &control.icon),
                            &label,
                            (ui.available_width() - 6.0).clamp(180.0, 320.0),
                        )
                        .clicked()
                        {
                            app.hardware_channel_detail_active = true;
                            app.hardware_control_dialog_key = Some(control.key.clone());
                        }
                    }
                });
            });
        ui.add_space(7.0);

        if app.hardware_channel_detail_active
            && let Some(control) = selected.as_ref()
        {
            draw_channel_detail_page(app, ui, &capabilities, control);
        } else {
            draw_channel_manager_page(app, ui, &capabilities);
        }
    });

    draw_hardware_binding_dialog(app, ui.ctx(), &capabilities);

    if close_requested {
        app.show_hardware_channels_dialog = false;
        app.hardware_control_dialog_key = None;
        app.hardware_channel_detail_active = false;
        app.hardware_binding_dialog_channel = None;
        app.hardware_binding_draft = None;
        app.hardware_binding_capturing = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manager_rows_inset_selection_children_and_preserve_opacity() {
        for dark in [false, true] {
            for width in [600.0, 980.0] {
                let context = egui::Context::default();
                context.set_visuals(if dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
                let mut rectangles = None;
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1100.0, 200.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        ui.set_width(width);
                        let layer_id = ui.layer_id();
                        let row = manager_channel_row(ui, "relay.5", layer_id, true, 0.58, |ui| {
                            let mut selected = true;
                            let checkbox = ui.checkbox(&mut selected, "").rect;
                            let remaining = ui.available_width();
                            ui.allocate_exact_size(
                                egui::vec2(remaining - 36.0, 27.0),
                                egui::Sense::hover(),
                            );
                            let opacity = ui.opacity();
                            let (last, order_opacity) = ui
                                .scope(|ui| {
                                    ui.multiply_opacity(0.5);
                                    (
                                        ui.add_sized([28.0, 27.0], egui::Button::new("...")).rect,
                                        ui.opacity(),
                                    )
                                })
                                .inner;
                            (checkbox, last, opacity, order_opacity)
                        });
                        rectangles = Some((row.response.rect, row.inner.0));
                    },
                );
                output.textures_delta.clear();
                let (row, (checkbox, last, opacity, order_opacity)) = rectangles.unwrap();
                assert!(
                    (row.width() - width).abs() < 0.1,
                    "row must not expand past its parent: {row:?}"
                );
                assert!(
                    checkbox.left() - row.left() >= 10.0,
                    "checkbox needs an inset: {checkbox:?} in {row:?}"
                );
                assert!(
                    row.right() - last.right() >= 10.0,
                    "last action needs a right inset: {last:?} in {row:?}"
                );
                assert!((opacity - 0.58).abs() < 0.001);
                assert!(
                    (order_opacity - 0.29).abs() < 0.001,
                    "hover fade must preserve row dimming"
                );
            }
        }
    }

    #[test]
    fn manager_rename_keeps_caption_font_position_and_action_slot_width() {
        let mut layouts = Vec::new();
        for editing in [false, true] {
            let context = egui::Context::default();
            let mut following = egui::Rect::NOTHING;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(600.0, 120.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    ui.horizontal(|ui| {
                        let mut draft = "Cinema output".to_string();
                        manager_channel_name(
                            ui,
                            "relay.5",
                            "Cinema output",
                            &mut draft,
                            editing,
                            false,
                            240.0,
                            egui::Id::new("rename-test"),
                            "Manage",
                            "Rename",
                            "Confirm",
                            "Cancel",
                        );
                        following = ui.button("Next action").rect;
                    });
                },
            );
            output.textures_delta.clear();
            let text = output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::epaint::Shape::Text(text) = &shape.shape
                        && text.galley.job.text == "Cinema output"
                    {
                        Some((text.pos, text.galley.job.sections[0].format.font_id.clone()))
                    } else {
                        None
                    }
                })
                .expect("caption text must be rendered in both modes");
            layouts.push((following, text));
        }
        assert_eq!(
            layouts[0].0, layouts[1].0,
            "Confirm/Cancel must fit inside the original name slot"
        );
        assert!(
            (layouts[0].1.0.x - layouts[1].1.0.x).abs() < 0.1,
            "text must not shift horizontally"
        );
        assert!(
            (layouts[0].1.0.y - layouts[1].1.0.y).abs() < 0.1,
            "text must not shift vertically"
        );
        assert_eq!(
            layouts[0].1.1, layouts[1].1.1,
            "display and edit must use the same font"
        );
    }

    #[test]
    fn manager_channel_dimming_tracks_monitor_raw_and_timeline_visibility() {
        use crate::config::NonUserControlVisibility as Visibility;
        use crate::four_d::models::TimelineTrackState;
        let capabilities = HardwareCapabilities::default();
        let mut control = HardwareControl {
            key: "relay.5".to_string(),
            kind: "relay".to_string(),
            ..Default::default()
        };
        assert!(!manager_channel_is_dimmed(
            &capabilities,
            &control,
            Visibility::Dimmed,
            None
        ));
        control.hidden = true;
        assert!(manager_channel_is_dimmed(
            &capabilities,
            &control,
            Visibility::Shown,
            None
        ));
        control.hidden = false;
        for state in [
            TimelineTrackState {
                linked: true,
                visible: false,
            },
            TimelineTrackState {
                linked: false,
                visible: true,
            },
        ] {
            assert!(manager_channel_is_dimmed(
                &capabilities,
                &control,
                Visibility::Shown,
                Some(state)
            ));
        }
        control.key = "relay.1".to_string();
        assert!(manager_channel_is_dimmed(
            &capabilities,
            &control,
            Visibility::Dimmed,
            None
        ));
        assert!(manager_channel_is_dimmed(
            &capabilities,
            &control,
            Visibility::Hidden,
            None
        ));
        assert!(!manager_channel_is_dimmed(
            &capabilities,
            &control,
            Visibility::Shown,
            None
        ));
    }

    #[test]
    fn manager_row_click_manages_without_stealing_checkbox_or_rename_actions() {
        let context = egui::Context::default();
        let mut selected = false;
        let mut editing = false;
        let mut draft = "Cinema output".to_string();
        let mut checkbox_rect = egui::Rect::NOTHING;
        let mut name_rect = egui::Rect::NOTHING;
        let mut row_rect = egui::Rect::NOTHING;
        let mut actions = Vec::new();
        let mut render = |events| {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(600.0, 180.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    ui.set_width(520.0);
                    let layer = ui.layer_id();
                    let row = manager_channel_row(ui, "relay.5", layer, selected, 1.0, |ui| {
                        checkbox_rect = ui.checkbox(&mut selected, "").rect;
                        let start = ui.next_widget_position();
                        name_rect = egui::Rect::from_min_size(
                            start,
                            egui::vec2(240.0, MANAGER_NAME_HEIGHT),
                        );
                        if let Some(action) = manager_channel_name(
                            ui,
                            "relay.5",
                            "Cinema output",
                            &mut draft,
                            editing,
                            false,
                            240.0,
                            egui::Id::new("pointer-rename-test"),
                            "Manage",
                            "Rename",
                            "Confirm",
                            "Cancel",
                        ) {
                            actions.push(action);
                            editing = action == ManagerNameAction::Rename;
                        }
                    });
                    row_rect = row.response.rect;
                    if row.inner.1.clicked() && !editing {
                        actions.push(ManagerNameAction::Manage);
                    }
                },
            );
            output.textures_delta.clear();
            (
                checkbox_rect,
                name_rect,
                row_rect,
                selected,
                actions.clone(),
            )
        };
        let click_events = |point, pressed| {
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        render(Vec::new());
        let (checkbox, name, row, _, _) = render(Vec::new());
        let point = checkbox.center();
        render(click_events(point, true));
        let (_, _, _, checked, actions) = render(click_events(point, false));
        assert!(checked);
        assert!(actions.is_empty(), "bulk selection must not open Manage");
        let point = egui::pos2(name.left() + 35.0, row.center().y);
        render(click_events(point, true));
        let (_, _, _, _, actions) = render(click_events(point, false));
        assert_eq!(actions, vec![ManagerNameAction::Manage]);
        let point = egui::pos2(row.right() - 20.0, row.center().y);
        render(click_events(point, true));
        let (_, _, _, _, actions) = render(click_events(point, false));
        assert_eq!(
            actions,
            vec![ManagerNameAction::Manage; 2],
            "empty row space opens Manage"
        );
        let point = egui::pos2(
            name.right() - MANAGER_NAME_ACTION_WIDTH / 2.0,
            row.center().y,
        );
        render(click_events(point, true));
        let (_, _, _, _, actions) = render(click_events(point, false));
        assert_eq!(
            actions,
            vec![
                ManagerNameAction::Manage,
                ManagerNameAction::Manage,
                ManagerNameAction::Rename
            ]
        );
        render(Vec::new());
        let point = egui::pos2(
            name.right() - MANAGER_NAME_ACTION_WIDTH * 1.5 - MANAGER_NAME_GAP,
            row.center().y,
        );
        render(click_events(point, true));
        let (_, _, _, _, actions) = render(click_events(point, false));
        assert_eq!(actions.last(), Some(&ManagerNameAction::Confirm));
    }

    #[test]
    fn channel_bulk_selection_handles_all_none_invert_and_stale_keys() {
        let available = || {
            ["seat.a", "relay.5", "pwm.0"]
                .into_iter()
                .map(str::to_string)
        };
        let mut selected = BTreeSet::from(["stale.channel".to_string()]);

        apply_channel_selection(
            &mut selected,
            available(),
            ChannelSelectionCommand::CheckAll,
        );
        assert_eq!(selected.len(), 3);
        assert!(!selected.contains("stale.channel"));

        selected.remove("relay.5");
        apply_channel_selection(&mut selected, available(), ChannelSelectionCommand::Invert);
        assert_eq!(selected, BTreeSet::from(["relay.5".to_string()]));

        apply_channel_selection(
            &mut selected,
            available(),
            ChannelSelectionCommand::UncheckAll,
        );
        assert!(selected.is_empty());
    }

    #[test]
    fn channel_bulk_actions_target_only_checked_stable_keys() {
        let controls = vec![
            HardwareControl {
                key: "relay.5".to_string(),
                ..Default::default()
            },
            HardwareControl {
                key: "pwm.0".to_string(),
                ..Default::default()
            },
            HardwareControl {
                key: "seat.a".to_string(),
                ..Default::default()
            },
        ];
        let selected = BTreeSet::from(["seat.a".to_string(), "pwm.0".to_string()]);

        let targeted = selected_channel_controls(&controls, &selected);
        assert_eq!(
            targeted
                .iter()
                .map(|control| control.key.as_str())
                .collect::<Vec<_>>(),
            vec!["pwm.0", "seat.a"]
        );
    }

    #[test]
    fn motion_live_controls_never_reuse_generic_boolean_verbs() {
        assert!(is_motion_live_verb("up"));
        assert!(is_motion_live_verb("Down"));
        assert!(is_motion_live_verb("STOP"));
        assert!(!is_motion_live_verb("on"));
        assert!(!is_motion_live_verb("off"));
    }

    #[test]
    fn binding_modes_derive_actions_from_advertised_channel_contract() {
        let relay = HardwareControl {
            key: "relay.5".to_string(),
            kind: "relay".to_string(),
            actions: vec![
                HardwareAction {
                    id: "relay.5.on".to_string(),
                    verb: "on".to_string(),
                    name: "On".to_string(),
                    ..Default::default()
                },
                HardwareAction {
                    id: "relay.5.off".to_string(),
                    verb: "off".to_string(),
                    name: "Off".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert!(matches!(
            default_binding_action(&relay, BindingEditorMode::Toggle),
            crate::config::HardwareKeyBindingAction::Toggle {
                on_action_id,
                off_action_id
            } if on_action_id == "relay.5.on" && off_action_id == "relay.5.off"
        ));

        let motion = HardwareControl {
            key: "seat.a".to_string(),
            kind: "motion".to_string(),
            actions: vec![
                HardwareAction {
                    id: "seat.a.up".to_string(),
                    verb: "up".to_string(),
                    name: "Up".to_string(),
                    ..Default::default()
                },
                HardwareAction {
                    id: "seat.a.stop".to_string(),
                    verb: "stop".to_string(),
                    name: "Stop".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert!(matches!(
            default_binding_action(&motion, BindingEditorMode::Hold),
            crate::config::HardwareKeyBindingAction::Hold {
                press_action_id,
                release_action_id,
                ..
            } if press_action_id == "seat.a.up" && release_action_id == "seat.a.stop"
        ));
    }

    #[test]
    fn display_order_accepts_only_editable_one_based_values() {
        assert_eq!(parse_display_order("1", 4), Some(0));
        assert_eq!(parse_display_order(" 3 ", 4), Some(2));
        assert_eq!(parse_display_order("4", 4), Some(3));
        assert_eq!(parse_display_order("0", 4), None);
        assert_eq!(parse_display_order("5", 4), None);
        assert_eq!(parse_display_order("two", 4), None);
        assert_eq!(parse_display_order("1", 0), None);
    }

    #[test]
    fn selected_control_uses_advertised_metadata_before_output_fallbacks() {
        let capabilities = HardwareCapabilities {
            controls: vec![HardwareControl {
                key: "relay.5".to_string(),
                kind: "motion".to_string(),
                name: "Seat left".to_string(),
                actions: vec![Default::default()],
                ..Default::default()
            }],
            relays: vec![crate::four_d::controller::HardwareOutput {
                id: 5,
                key: "relay.5".to_string(),
                name: "Raw relay 5".to_string(),
                role: String::new(),
                control: "toggle".to_string(),
            }],
            ..Default::default()
        };

        let control = selected_control(&capabilities, "relay.5").expect("control");
        assert_eq!(control.kind, "motion");
        assert_eq!(control.name, "Seat left");
        assert_eq!(control.actions.len(), 1);
    }

    #[test]
    fn raw_outputs_still_receive_a_complete_control_dialog() {
        let capabilities = HardwareCapabilities {
            pwm_channels: vec![crate::four_d::controller::HardwareOutput {
                id: 2,
                key: "pwm.2".to_string(),
                name: "Aisle light".to_string(),
                role: "Lighting".to_string(),
                control: "level".to_string(),
            }],
            ..Default::default()
        };

        let control = selected_control(&capabilities, "pwm.2").expect("fallback control");
        assert_eq!(control.kind, "mosfet");
        assert_eq!(control.group, "Lighting");
        assert_eq!(pwm_channel(&capabilities, "pwm.2"), Some(2));
    }

    #[test]
    fn channel_manager_includes_advertised_and_raw_board_channels_once() {
        let capabilities = HardwareCapabilities {
            controls: vec![
                HardwareControl {
                    key: "relay.5".to_string(),
                    kind: "relay".to_string(),
                    name: "User relay".to_string(),
                    order: 1,
                    ..Default::default()
                },
                HardwareControl {
                    key: "relay.5".to_string(),
                    kind: "relay".to_string(),
                    name: "Duplicate advertised control".to_string(),
                    order: 99,
                    ..Default::default()
                },
            ],
            relays: vec![
                crate::four_d::controller::HardwareOutput {
                    id: 5,
                    key: "relay.5".to_string(),
                    name: "Duplicate source".to_string(),
                    role: String::new(),
                    control: "toggle".to_string(),
                },
                crate::four_d::controller::HardwareOutput {
                    id: 6,
                    key: "relay.6".to_string(),
                    name: "Raw relay".to_string(),
                    role: String::new(),
                    control: "toggle".to_string(),
                },
            ],
            pwm_channels: vec![crate::four_d::controller::HardwareOutput {
                id: 0,
                key: "pwm.0".to_string(),
                name: "House light".to_string(),
                role: "lighting".to_string(),
                control: "level".to_string(),
            }],
            ..Default::default()
        };
        let controls = managed_controls(&capabilities);
        assert_eq!(controls.len(), 3);
        assert_eq!(
            controls.iter().filter(|item| item.key == "relay.5").count(),
            1
        );
        assert!(controls.iter().any(|item| item.key == "relay.6"));
        assert!(controls.iter().any(|item| item.key == "pwm.0"));
    }

    #[test]
    fn relay_keys_are_parsed_without_guessing_other_peripherals() {
        assert_eq!(relay_id("relay.5"), Some(5));
        assert_eq!(relay_id("pwm.5"), None);
        assert_eq!(relay_id("relay.left"), None);
    }

    #[test]
    fn advertised_raw_relay_actions_use_the_profile_independent_command_path() {
        let control = HardwareControl {
            key: "relay.5".to_string(),
            kind: "relay".to_string(),
            ..Default::default()
        };
        let on = HardwareAction {
            id: "relay.5.on".to_string(),
            verb: "on".to_string(),
            name: "On".to_string(),
            ..Default::default()
        };
        assert_eq!(
            action_dispatch(&control, &on),
            HardwareActionDispatch::ControllerCommand("relay 5 on".to_string())
        );
    }

    #[test]
    fn semantic_controls_keep_their_advertised_action_contract() {
        let control = HardwareControl {
            key: "seat.a".to_string(),
            kind: "motion".to_string(),
            ..Default::default()
        };
        let up = HardwareAction {
            id: "seat.a.up".to_string(),
            verb: "up".to_string(),
            name: "Up".to_string(),
            ..Default::default()
        };
        assert_eq!(
            action_dispatch(&control, &up),
            HardwareActionDispatch::Advertised("seat.a.up".to_string())
        );
    }

    #[test]
    fn unconfigured_raw_motion_routes_through_interlocked_side_command() {
        let control = HardwareControl {
            key: "seat.a".to_string(),
            kind: "motion".to_string(),
            control: "raw-motion".to_string(),
            ..HardwareControl::default()
        };
        let action = HardwareAction {
            id: "seat.a.up".to_string(),
            verb: "up".to_string(),
            name: "Up".to_string(),
            ..HardwareAction::default()
        };
        assert_eq!(
            action_dispatch(&control, &action),
            HardwareActionDispatch::ControllerCommand("relay side left up".to_string())
        );
    }
}
