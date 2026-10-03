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

pub fn draw(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let Some(key) = app.hardware_control_dialog_key.clone() else {
        return;
    };
    let Some(capabilities) = app
        .advertised_hardware()
        .filter(|capabilities| capabilities.board_connected)
    else {
        app.hardware_control_dialog_key = None;
        return;
    };
    let Some(control) = selected_control(&capabilities, &key) else {
        app.hardware_control_dialog_key = None;
        return;
    };

    let mut open = true;
    let bounds = ui.ctx().content_rect().shrink(20.0);
    let default_size = egui::vec2(bounds.width().min(560.0), bounds.height().min(620.0));
    egui::Window::new(format!(
        "{} {}",
        crate::ui::icons::control(&control.kind, &control.icon),
        app.tr("Hardware control")
    ))
    .id(egui::Id::new("hardware_control_dialog"))
    .open(&mut open)
    .default_size(default_size)
    .max_size(egui::vec2(bounds.width().min(700.0), bounds.height()))
    .constrain_to(bounds)
    .resizable(true)
    .collapsible(false)
    .show(ui.ctx(), |ui| {
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
        egui::Grid::new("hardware_control_presentation")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label(app.tr("Name"));
                ui.add(
                    egui::TextEdit::singleline(&mut app.hardware_control_name_draft)
                        .desired_width(ui.available_width().max(180.0)),
                );
                ui.end_row();
                ui.label(app.tr("Group"));
                ui.add(
                    egui::TextEdit::singleline(&mut app.hardware_control_group_draft)
                        .desired_width(ui.available_width().max(180.0)),
                );
                ui.end_row();
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
                crate::ui::layout::update_control_name(
                    app,
                    &capabilities,
                    &control,
                    app.hardware_control_name_draft.clone(),
                );
                crate::ui::layout::update_control_group(
                    app,
                    &capabilities,
                    &control,
                    app.hardware_control_group_draft.clone(),
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
                crate::ui::layout::update_control_name(app, &capabilities, &control, String::new());
                crate::ui::layout::update_control_group(
                    app,
                    &capabilities,
                    &control,
                    String::new(),
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
                        let activated = if matches!(verb.as_str(), "on" | "off") {
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
                    if crate::ui::layout::hardware_control_activated(app, &uis[index], &response) {
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
                        .add_enabled(!control.locked, egui::Button::new(format!("{percent:.0}%")))
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

    if !open {
        app.hardware_control_dialog_key = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
