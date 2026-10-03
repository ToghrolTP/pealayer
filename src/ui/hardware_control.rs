use crate::app::PealayerApp;
use crate::four_d::controller::{HardwareAction, HardwareCapabilities, HardwareControl};
use eframe::egui;

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
    let (control, action) = capabilities
        .controls
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

fn set_pwm(app: &PealayerApp, channel: u8, percent: f64) {
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
    let mut controls = capabilities.controls.clone();
    for output in &capabilities.relays {
        if !controls.iter().any(|control| control.key == output.key) {
            controls.push(HardwareControl {
                key: output.key.clone(),
                kind: "relay".to_string(),
                name: output.name.clone(),
                default_name: output.name.clone(),
                control: output.control.clone(),
                group: output.role.clone(),
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
    app.hardware_control_icon_search.clear();
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

fn draw_manager_live_action(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities,
    control: &HardwareControl,
) {
    let enabled = !app.estop_active && !control.locked;
    let on = channel_is_active(capabilities, control);
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
        let is_motion = crate::ui::layout::is_motion_control(control);
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

fn draw_channel_manager_page(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities,
) {
    let controls = managed_controls(&capabilities);
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
                let editing = ui.data_mut(|data| data.get_temp::<bool>(edit_id).unwrap_or(false));
                let row_height = 32.0;
                let row_width = ui.available_width();
                let predicted_rect = egui::Rect::from_min_size(
                    ui.next_widget_position(),
                    egui::vec2(row_width, row_height),
                );
                let row_hovered = ui
                    .ctx()
                    .pointer_hover_pos()
                    .is_some_and(|pointer| predicted_rect.contains(pointer));
                let order_focused = ui.memory(|memory| memory.has_focus(order_edit_id));
                let order_alpha = ui.ctx().animate_bool(
                    egui::Id::new(("manager-order-visible", &control.key)),
                    row_hovered || order_focused,
                );
                let dragging = crate::ui::layout::hardware_channel_is_dragging(ui, &control.key);
                // Keep ordinary rows in the same interaction order as the
                // containing window. A permanently-Foreground child layer
                // sits above the window resize grip and menu popups, which
                // made the row look correct while swallowing its controls.
                // The row is still transformed as one unit while dragging.
                let layer_id = egui::LayerId::new(
                    egui::Order::Middle,
                    egui::Id::new(("hardware-channel-manager-row", &control.key)),
                );
                let row = ui.scope_builder(egui::UiBuilder::new().layer_id(layer_id), |ui| {
                    if dragging {
                        ui.set_opacity(0.58);
                    }
                    ui.allocate_ui_with_layout(
                        egui::vec2(row_width, row_height),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
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
                            let (indicator_rect, _) = ui
                                .allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                            ui.painter().circle_filled(
                                indicator_rect.center(),
                                4.5,
                                indicator_color,
                            );
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

                            if editing {
                                let mut draft = ui.data_mut(|data| {
                                    data.get_temp::<String>(draft_id)
                                        .unwrap_or_else(|| control.name.clone())
                                });
                                let edit = ui.add_sized(
                                    [(ui.available_width() - 324.0).max(130.0), 27.0],
                                    egui::TextEdit::singleline(&mut draft).id(text_edit_id),
                                );
                                if ui.data_mut(|data| {
                                    data.remove_temp::<bool>(focus_pending_id).unwrap_or(false)
                                }) {
                                    edit.request_focus();
                                }
                                if edit.changed() {
                                    ui.data_mut(|data| data.insert_temp(draft_id, draft.clone()));
                                }
                                if ui.button(crate::ui::icons::CHECK).clicked()
                                    || (edit.lost_focus()
                                        && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                                {
                                    crate::ui::layout::update_control_name(
                                        app,
                                        &capabilities,
                                        control,
                                        draft,
                                    );
                                    ui.data_mut(|data| data.insert_temp(edit_id, false));
                                }
                                if ui.button(crate::ui::icons::X).clicked() {
                                    ui.data_mut(|data| data.insert_temp(edit_id, false));
                                }
                            } else {
                                let name =
                                    crate::ui::i18n::visual_text(app.language, &control.name);
                                let response = crate::ui::layout::left_aligned_click_label(
                                    ui,
                                    &name,
                                    (ui.available_width() - 324.0).max(130.0),
                                    27.0,
                                    13.0,
                                );
                                if response.clicked()
                                    || ui
                                        .button(crate::ui::icons::PENCIL_SIMPLE)
                                        .on_hover_text(app.tr("Rename"))
                                        .clicked()
                                {
                                    ui.data_mut(|data| {
                                        data.insert_temp(draft_id, control.name.clone());
                                        data.insert_temp(edit_id, true);
                                        data.insert_temp(focus_pending_id, true);
                                    });
                                }
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
                                ui.set_opacity(order_alpha);
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
                                    .on_hover_text(format!(
                                        "{} · 1–{}",
                                        app.tr("Order"),
                                        peers.len()
                                    ));
                                if order_edit.changed() {
                                    ui.data_mut(|data| {
                                        data.insert_temp(order_draft_id, order_draft.clone())
                                    });
                                }
                                let commit_order = order_edit.lost_focus()
                                    || (order_edit.has_focus()
                                        && ui.input(|input| input.key_pressed(egui::Key::Enter)));
                                if commit_order {
                                    if let Some(value) =
                                        parse_display_order(&order_draft, peers.len())
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
                    )
                });
                let row_rect = row.response.rect;
                crate::ui::layout::finish_hardware_channel_card(ui, control, row_rect, layer_id);
                if let Some(drop) =
                    crate::ui::layout::hardware_channel_drop_target(ui, row_rect, control)
                {
                    crate::ui::layout::persist_channel_drop(app, &capabilities, drop);
                }
                ui.add(egui::Separator::default().spacing(0.0));
            }
        });
    // A release over the source row, outside the window, or over an
    // incompatible channel is not a drop. Clear the transient drag in all of
    // those cases so a cancelled drag cannot leave a transformed layer above
    // the row controls and make subsequent actions appear dead.
    crate::ui::layout::clear_released_hardware_channel_drag(ui);
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
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(crate::ui::icons::control(&control.kind, &control.icon))
                        .size(28.0),
                );
                ui.vertical(|ui| {
                    ui.heading(crate::ui::i18n::visual_text(app.language, &control.name));
                    ui.label(egui::RichText::new(&control.key).monospace().weak());
                });
            });
            ui.add_space(10.0);

            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(10))
                .corner_radius(8.0)
                .show(ui, |ui| {
                    egui::Grid::new("hardware_control_identity")
                        .num_columns(2)
                        .spacing([16.0, 7.0])
                        .show(ui, |ui| {
                            ui.label(app.tr("Board"));
                            ui.label(crate::ui::i18n::visual_text(
                                app.language,
                                &capabilities.board_name,
                            ));
                            ui.end_row();
                            ui.label(app.tr("Type"));
                            ui.label(&control.kind);
                            ui.end_row();
                            ui.label(app.tr("Control"));
                            ui.label(&control.control);
                            ui.end_row();
                            ui.label(app.tr("Stable key"));
                            ui.label(egui::RichText::new(&control.key).monospace());
                            ui.end_row();
                        });
                });

            ui.add_space(10.0);
            ui.strong(app.tr("Presentation"));
            let timeline_track_key =
                crate::four_d::models::hardware_timeline_track_key(&control.key);
            egui::Grid::new("hardware_control_presentation")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(app.tr("Name"));
                    let name_align =
                        crate::ui::i18n::input_alignment(app.rtl, &app.hardware_control_name_draft);
                    ui.add(
                        egui::TextEdit::singleline(&mut app.hardware_control_name_draft)
                            .horizontal_align(name_align)
                            .desired_width(ui.available_width().max(180.0)),
                    );
                    ui.end_row();
                    ui.label(app.tr("Group"));
                    let group_align = crate::ui::i18n::input_alignment(
                        app.rtl,
                        &app.hardware_control_group_draft,
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut app.hardware_control_group_draft)
                            .horizontal_align(group_align)
                            .desired_width(ui.available_width().max(180.0)),
                    );
                    ui.end_row();
                    ui.label(app.tr("Icon"));
                    let selected_icon =
                        crate::ui::icons::named_control_icon(&app.hardware_control_icon_draft)
                            .unwrap_or_else(|| crate::ui::icons::control(&control.kind, ""));
                    let selected_name =
                        crate::ui::icons::control_icon_name(&app.hardware_control_icon_draft)
                            .unwrap_or("Use channel default");
                    egui::ComboBox::from_id_salt(("hardware-control-icon", &control.key))
                        .width(ui.available_width().max(180.0))
                        .selected_text(format!("{selected_icon}  {}", app.tr(selected_name)))
                        .show_ui(ui, |ui| {
                            let search_hint = app.tr("Search icons");
                            ui.add(
                                egui::TextEdit::singleline(&mut app.hardware_control_icon_search)
                                    .hint_text(search_hint)
                                    .desired_width(ui.available_width()),
                            );
                            if ui
                                .selectable_label(
                                    app.hardware_control_icon_draft.is_empty(),
                                    app.tr("Use channel default"),
                                )
                                .clicked()
                            {
                                app.hardware_control_icon_draft.clear();
                            }
                            ui.separator();
                            let query =
                                app.hardware_control_icon_search.trim().to_ascii_lowercase();
                            for (key, label, glyph) in crate::ui::icons::CONTROL_ICON_PRESETS {
                                if !query.is_empty()
                                    && !key.contains(&query)
                                    && !label.to_ascii_lowercase().contains(&query)
                                {
                                    continue;
                                }
                                if ui
                                    .selectable_label(
                                        app.hardware_control_icon_draft == *key,
                                        format!("{glyph}  {}", app.tr(label)),
                                    )
                                    .clicked()
                                {
                                    app.hardware_control_icon_draft = (*key).to_string();
                                }
                            }
                        });
                    ui.end_row();
                    if matches!(control.kind.as_str(), "mosfet" | "pwm") {
                        ui.label(app.tr("Indicator color"));
                        ui.horizontal(|ui| {
                            let mut rgb =
                                crate::config::parse_rgb_hex(&app.hardware_control_color_draft)
                                    .unwrap_or([56, 210, 122]);
                            if ui.color_edit_button_srgb(&mut rgb).changed() {
                                app.hardware_control_color_draft =
                                    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
                            }
                            ui.add(
                                egui::TextEdit::singleline(&mut app.hardware_control_color_draft)
                                    .desired_width(92.0)
                                    .char_limit(7)
                                    .hint_text("#38D27A"),
                            );
                        });
                        ui.end_row();
                    }
                    if matches!(control.kind.as_str(), "seat" | "motion") {
                        let moving_up_color = app.tr("Moving up color");
                        let moving_down_color = app.tr("Moving down color");
                        for (label, draft, fallback) in [
                            (
                                moving_up_color,
                                &mut app.hardware_control_up_color_draft,
                                [245, 158, 11],
                            ),
                            (
                                moving_down_color,
                                &mut app.hardware_control_down_color_draft,
                                [59, 130, 246],
                            ),
                        ] {
                            ui.label(label);
                            ui.horizontal(|ui| {
                                let mut rgb =
                                    crate::config::parse_rgb_hex(draft).unwrap_or(fallback);
                                if ui.color_edit_button_srgb(&mut rgb).changed() {
                                    *draft = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
                                }
                                ui.add(
                                    egui::TextEdit::singleline(draft)
                                        .desired_width(92.0)
                                        .char_limit(7),
                                );
                            });
                            ui.end_row();
                        }
                    }
                    if relay_id(&control.key).is_some()
                        || matches!(control.kind.as_str(), "mosfet" | "pwm")
                    {
                        let mut locked = control.locked;
                        ui.label(app.tr("Lock"));
                        if ui
                            .checkbox(&mut locked, app.tr("Prevent live control"))
                            .changed()
                        {
                            crate::ui::layout::update_control_presentation_flags(
                                app,
                                &capabilities,
                                &control,
                                None,
                                Some(locked),
                            );
                        }
                        ui.end_row();
                        let mut visible = !control.hidden;
                        ui.label(app.tr("Visibility"));
                        if ui
                            .checkbox(&mut visible, app.tr("Show in Hardware Monitor"))
                            .changed()
                        {
                            crate::ui::layout::update_control_presentation_flags(
                                app,
                                &capabilities,
                                &control,
                                Some(!visible),
                                None,
                            );
                        }
                        ui.end_row();
                    }
                    let track_state = app.timeline.track_state(&timeline_track_key);
                    let mut linked = track_state.linked;
                    ui.label(app.tr("Timeline"));
                    if ui
                        .checkbox(&mut linked, app.tr("Link channel to timeline"))
                        .changed()
                    {
                        app.set_timeline_track_linked(&timeline_track_key, linked);
                    }
                    ui.end_row();
                    let mut timeline_visible = track_state.visible;
                    ui.label(app.tr("Timeline visibility"));
                    if ui
                        .add_enabled_ui(linked, |ui| {
                            ui.checkbox(
                                &mut timeline_visible,
                                app.tr("Show channel timeline track"),
                            )
                        })
                        .inner
                        .changed()
                    {
                        app.set_timeline_track_visible(&timeline_track_key, timeline_visible);
                    }
                    ui.end_row();
                });
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::FLOPPY_DISK,
                        app.tr("Save presentation")
                    ))
                    .clicked()
                {
                    let requested_name = app.hardware_control_name_draft.trim();
                    let name =
                        if requested_name.is_empty() || requested_name == control.default_name {
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
                        fields["color"] = serde_json::Value::String(
                            app.hardware_control_color_draft.trim().to_string(),
                        );
                    }
                    if matches!(control.kind.as_str(), "seat" | "motion") {
                        fields["up_color"] = serde_json::Value::String(
                            app.hardware_control_up_color_draft.trim().to_string(),
                        );
                        fields["down_color"] = serde_json::Value::String(
                            app.hardware_control_down_color_draft.trim().to_string(),
                        );
                    }
                    crate::ui::layout::update_control_presentation(
                        app,
                        &capabilities,
                        &control,
                        "presentation-manage",
                        fields,
                    );
                }
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                        app.tr("Restore defaults")
                    ))
                    .clicked()
                {
                    app.hardware_control_name_draft = control.default_name.clone();
                    app.hardware_control_group_draft.clear();
                    app.hardware_control_icon_draft.clear();
                    app.hardware_control_icon_search.clear();
                    app.hardware_control_color_draft.clear();
                    app.hardware_control_up_color_draft.clear();
                    app.hardware_control_down_color_draft.clear();
                    crate::ui::layout::update_control_presentation(
                        app,
                        &capabilities,
                        &control,
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
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            ui.strong(app.tr("Live control"));

            if !control.actions.is_empty() {
                ui.add_space(6.0);
                let columns = if ui.available_width() >= 420.0 { 3 } else { 1 };
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
                            || crate::ui::layout::contextual_stop_action(&capabilities, &control)
                                .is_some()
                    })
                    .collect::<Vec<_>>();
                for row in actions.chunks(columns) {
                    ui.columns(columns, |uis| {
                        for (index, action) in row.iter().enumerate() {
                            let response = uis[index].add_enabled(
                                !app.estop_active && !control.locked,
                                egui::Button::new(format!(
                                    "{} {}",
                                    crate::ui::icons::action(&action.verb),
                                    crate::ui::i18n::visual_text(app.language, &action.name)
                                ))
                                .min_size(egui::vec2(uis[index].available_width(), 32.0)),
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
                                    &control,
                                    action,
                                    stop,
                                );
                                continue;
                            }
                            let activated = if is_motion || matches!(verb.as_str(), "on" | "off") {
                                crate::ui::layout::hardware_control_activated(
                                    app,
                                    &uis[index],
                                    &response,
                                )
                            } else {
                                response.clicked()
                            };
                            if activated {
                                invoke_action(app, &control, action);
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
                            egui::Button::new(format!("{icon} {label}"))
                                .min_size(egui::vec2(uis[index].available_width(), 34.0)),
                        );
                        if crate::ui::layout::hardware_control_activated(
                            app,
                            &uis[index],
                            &response,
                        ) {
                            set_relay(app, relay, on);
                        }
                    }
                });
            } else if let Some(channel) = pwm_channel(&capabilities, &control.key) {
                ui.add_space(6.0);
                let pwm_response = crate::ui::layout::draw_pwm_editor_row(
                    ui,
                    &mut app.hardware_control_pwm_percent,
                    !control.locked,
                );
                if pwm_response.should_transmit(app.live_pwm_updates) {
                    set_pwm(app, channel, app.hardware_control_pwm_percent);
                }
                ui.horizontal_wrapped(|ui| {
                    for percent in [0.0, 25.0, 50.0, 75.0, 100.0] {
                        if ui
                            .add_enabled(
                                !control.locked,
                                egui::Button::new(format!("{percent:.0}%")),
                            )
                            .clicked()
                        {
                            app.hardware_control_pwm_percent = percent;
                            set_pwm(app, channel, percent);
                        }
                    }
                });
            } else {
                ui.label(
                    egui::RichText::new(app.tr("This item has no live actions advertised.")).weak(),
                );
            }
        });
}

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if !app.show_hardware_channels_dialog && app.hardware_control_dialog_key.is_none() {
        return;
    }
    let Some(capabilities) = app
        .advertised_hardware()
        .filter(|capabilities| capabilities.board_connected)
    else {
        app.show_hardware_channels_dialog = false;
        app.hardware_control_dialog_key = None;
        app.hardware_channel_detail_active = false;
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
        egui::vec2(860.0, 720.0),
        egui::vec2(600.0, 420.0),
        egui::vec2(1_100.0, 820.0),
    );
    let window_fill = ui.visuals().window_fill();
    let opaque_window_fill =
        egui::Color32::from_rgb(window_fill.r(), window_fill.g(), window_fill.b());
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
    .frame(egui::Frame::window(ui.style()).fill(opaque_window_fill))
    .constrain_to(geometry.bounds)
    .title_bar(false)
    .collapsible(false)
    .show(ui.ctx(), |ui| {
        ui.horizontal(|ui| {
            ui.strong(format!(
                "{} {}",
                crate::ui::icons::SLIDERS_HORIZONTAL,
                app.tr("Manage channels")
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(crate::ui::icons::X)
                    .on_hover_text(app.tr("Close"))
                    .clicked()
                {
                    close_requested = true;
                }
            });
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let on_channels = !app.hardware_channel_detail_active;
            if ui
                .selectable_label(
                    on_channels,
                    format!(
                        "{} {}",
                        crate::ui::icons::LIST_CHECKS,
                        app.tr("All channels")
                    ),
                )
                .clicked()
            {
                app.hardware_channel_detail_active = false;
            }
            if let Some(control) = selected.as_ref() {
                let label = format!(
                    "{} {}",
                    crate::ui::icons::control(&control.kind, &control.icon),
                    crate::ui::i18n::visual_text(app.language, &control.name)
                );
                if ui
                    .selectable_label(app.hardware_channel_detail_active, label)
                    .clicked()
                {
                    app.hardware_channel_detail_active = true;
                    app.hardware_control_dialog_key = Some(control.key.clone());
                }
            }
        });
        ui.separator();
        ui.add_space(4.0);

        if app.hardware_channel_detail_active
            && let Some(control) = selected.as_ref()
        {
            draw_channel_detail_page(app, ui, &capabilities, control);
        } else {
            draw_channel_manager_page(app, ui, &capabilities);
        }
    });

    if close_requested {
        app.show_hardware_channels_dialog = false;
        app.hardware_control_dialog_key = None;
        app.hardware_channel_detail_active = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            controls: vec![HardwareControl {
                key: "relay.5".to_string(),
                kind: "relay".to_string(),
                name: "User relay".to_string(),
                order: 1,
                ..Default::default()
            }],
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
