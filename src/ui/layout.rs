use crate::app::{EffectDragPayload, PealayerApp};
use eframe::egui;
use egui_dock::TabViewer;

pub fn paint_dock_disclosure_icons(
    ui: &mut egui::Ui,
    dock_state: &egui_dock::DockState<PealayerTab>,
    tab_bar_height: f32,
) {
    let pointer = ui.ctx().pointer_hover_pos();
    for (_, leaf) in dock_state.iter_leaves() {
        if !leaf.rect.is_positive() || leaf.tabs.is_empty() {
            continue;
        }
        let button_rect = egui::Rect::from_min_size(
            leaf.rect.min,
            egui::vec2(24.0_f32.min(leaf.rect.width()), tab_bar_height),
        );
        let hovered = pointer.is_some_and(|position| button_rect.contains(position));
        let color = if hovered {
            ui.visuals().strong_text_color()
        } else {
            ui.visuals().text_color()
        };
        let icon = if leaf.collapsed {
            crate::ui::icons::CARET_RIGHT
        } else {
            crate::ui::icons::CARET_DOWN
        };
        ui.painter().text(
            button_rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(13.0),
            color,
        );
    }
}

fn drag_translation(
    pointer: egui::Pos2,
    source_min: egui::Pos2,
    grab_offset: egui::Vec2,
) -> egui::Vec2 {
    pointer - source_min - grab_offset
}

fn pwm_percent(raw: u16) -> f64 {
    f64::from(raw.min(4095)) * 100.0 / 4095.0
}

fn pwm_raw(percent: f64) -> u16 {
    (percent.clamp(0.0, 100.0) * 4095.0 / 100.0).round() as u16
}

fn begin_effect_drag(ctx: &egui::Context, payload: EffectDragPayload) {
    egui::DragAndDrop::set_payload(ctx, payload);
}

fn effect_drag_source<R>(
    ui: &mut egui::Ui,
    id: egui::Id,
    payload: EffectDragPayload,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let offset_id = id.with("pointer-offset");
    if ui.ctx().is_being_dragged(id) {
        egui::DragAndDrop::set_payload(ui.ctx(), payload);
        let layer_id = egui::LayerId::new(egui::Order::Tooltip, id);
        let response = ui.scope_builder(egui::UiBuilder::new().layer_id(layer_id), add_contents);
        if let Some(pointer) = ui.ctx().pointer_interact_pos() {
            let offset = ui
                .data_mut(|data| data.get_temp::<egui::Vec2>(offset_id))
                .unwrap_or_else(|| response.response.rect.size() * 0.5);
            let translation = drag_translation(pointer, response.response.rect.min, offset);
            ui.ctx().transform_layer_shapes(
                layer_id,
                egui::emath::TSTransform::from_translation(translation),
            );
        }
        response
    } else {
        let response = ui.scope(add_contents);
        let drag = ui
            .interact(response.response.rect, id, egui::Sense::drag())
            .on_hover_cursor(egui::CursorIcon::Grab);
        if drag.drag_started() {
            // Establish the payload in the same input frame in which egui
            // claims the drag. Waiting until the next paint left a race where
            // the pointer could enter (and even be released over) the timeline
            // before any drop payload existed, so the cards looked draggable
            // but every drop was ignored.
            begin_effect_drag(ui.ctx(), payload);
            if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                ui.data_mut(|data| {
                    data.insert_temp(offset_id, pointer - response.response.rect.min)
                });
            }
        }
        egui::InnerResponse::new(response.inner, drag | response.response)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TimelineTrackKind {
    Video,
    Audio,
    ControllerMacros,
    Relay(u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TimelineTrackRow {
    name: String,
    kind: TimelineTrackKind,
}

fn timeline_track_rows(app: &PealayerApp) -> Vec<TimelineTrackRow> {
    let mut rows = Vec::new();
    if app.current_video_path.is_some() {
        rows.push(TimelineTrackRow {
            name: app.tr("Video"),
            kind: TimelineTrackKind::Video,
        });
        if !app.audio_tracks.is_empty() && app.current_aid != "no" {
            let selected = app
                .current_aid
                .parse::<i64>()
                .ok()
                .and_then(|id| app.audio_tracks.iter().find(|track| track.id == id));
            let name = selected
                .and_then(|track| track.title.clone().or_else(|| track.lang.clone()))
                .unwrap_or_else(|| app.tr("Audio"));
            rows.push(TimelineTrackRow {
                name: app.display_text(&name),
                kind: TimelineTrackKind::Audio,
            });
        }
    }
    if let Some(capabilities) = app
        .advertised_hardware()
        .filter(|capabilities| capabilities.board_connected)
    {
        if !capabilities.macros.is_empty() || !capabilities.strip_effects.is_empty() {
            rows.push(TimelineTrackRow {
                name: app.tr("Controller effects"),
                kind: TimelineTrackKind::ControllerMacros,
            });
        }
        rows.extend(
            capabilities
                .relays
                .into_iter()
                .map(|relay| TimelineTrackRow {
                    name: app.display_text(&relay.name),
                    kind: TimelineTrackKind::Relay(relay.id),
                }),
        );
    }
    rows
}

fn relay_for_timeline_row(rows: &[TimelineTrackRow], row: i32) -> Option<u8> {
    rows.get(usize::try_from(row).ok()?)
        .and_then(|track| match track.kind {
            TimelineTrackKind::Relay(id) => Some(id),
            _ => None,
        })
}

fn timeline_row_for_relay(rows: &[TimelineTrackRow], relay_id: u8) -> Option<usize> {
    rows.iter()
        .position(|track| track.kind == TimelineTrackKind::Relay(relay_id))
}

fn relay_id_from_control_key(key: &str) -> Option<u8> {
    key.strip_prefix("relay.")?.parse().ok()
}

fn is_pwm_control(control: &crate::four_d::controller::HardwareControl) -> bool {
    matches!(control.kind.as_str(), "mosfet" | "pwm") || control.key.starts_with("pwm.")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ControlIndicatorState {
    Active,
    Available,
    Inactive,
}

fn control_indicator_state(
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
) -> ControlIndicatorState {
    if let Some(relay_id) = relay_id_from_control_key(&control.key) {
        return if capabilities.active_relays.contains(&relay_id) {
            ControlIndicatorState::Active
        } else {
            ControlIndicatorState::Inactive
        };
    }
    if is_pwm_control(control) {
        let channel = capabilities
            .pwm_channels
            .iter()
            .find(|channel| channel.key == control.key)
            .map(|channel| channel.id);
        return match channel {
            Some(channel) if capabilities.telemetry.pwm_channel == Some(channel) => {
                if capabilities.telemetry.pwm_value.unwrap_or(0) > 0 {
                    ControlIndicatorState::Active
                } else {
                    ControlIndicatorState::Inactive
                }
            }
            _ => ControlIndicatorState::Available,
        };
    }
    ControlIndicatorState::Available
}

fn control_grid_columns(available_width: f32) -> usize {
    if available_width >= 620.0 { 2 } else { 1 }
}

fn action_grid_columns(available_width: f32, action_count: usize) -> usize {
    match action_count {
        0 | 1 => 1,
        2 if available_width >= 230.0 => 2,
        3 if available_width >= 360.0 => 3,
        _ => 1,
    }
}

fn draw_control_indicator(
    app: &PealayerApp,
    ui: &mut egui::Ui,
    state: ControlIndicatorState,
    interactive: bool,
) -> egui::Response {
    let green = egui::Color32::from_rgb(34, 197, 94);
    let neutral = ui
        .visuals()
        .widgets
        .noninteractive
        .fg_stroke
        .color
        .gamma_multiply(0.42);
    let sense = if interactive {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), sense);
    let center = rect.center();
    match state {
        ControlIndicatorState::Active => {
            ui.painter()
                .circle_filled(center, 7.0, green.gamma_multiply(0.18));
            ui.painter().circle_filled(center, 4.5, green);
        }
        ControlIndicatorState::Available => {
            ui.painter()
                .circle_stroke(center, 5.5, egui::Stroke::new(1.8_f32, green));
            ui.painter()
                .circle_filled(center, 2.0, green.gamma_multiply(0.65));
        }
        ControlIndicatorState::Inactive => {
            ui.painter()
                .circle_stroke(center, 5.0, egui::Stroke::new(1.4_f32, neutral));
        }
    }
    let status = match state {
        ControlIndicatorState::Active => app.tr("Board reports ON"),
        ControlIndicatorState::Available => app.tr("Live board control"),
        ControlIndicatorState::Inactive => app.tr("Board reports OFF"),
    };
    response.on_hover_text(if interactive {
        format!("{status} · {}", app.tr("Click to toggle"))
    } else {
        status
    })
}

fn humanize_machine_label(value: &str) -> String {
    value
        .split(['-', '_', '.'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn update_control_name(
    app: &PealayerApp,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
    requested_name: String,
) {
    let requested_name = requested_name.trim().to_string();
    let restore_default = requested_name.is_empty()
        || (!control.default_name.is_empty() && requested_name == control.default_name);
    let mut fallback_names = capabilities.peripheral_names.clone();
    if restore_default {
        fallback_names.remove(&control.key);
    } else {
        fallback_names.insert(control.key.clone(), requested_name.clone());
    }
    let expected_revision = capabilities
        .board_profile
        .as_ref()
        .map(|profile| profile.revision.clone())
        .filter(|revision| !revision.is_empty());
    let _ = app.engine_handle.sender.send(
        crate::four_d::engine::EngineMessage::UpdatePeripheralPresentation {
            key: control.key.clone(),
            name: Some(if restore_default {
                String::new()
            } else {
                requested_name
            }),
            icon: None,
            group: None,
            expected_revision,
            fallback_names,
        },
    );
}

fn update_control_group(
    app: &PealayerApp,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
    requested_group: String,
) {
    let expected_revision = capabilities
        .board_profile
        .as_ref()
        .map(|profile| profile.revision.clone())
        .filter(|revision| !revision.is_empty());
    let _ = app.engine_handle.sender.send(
        crate::four_d::engine::EngineMessage::UpdatePeripheralPresentation {
            key: control.key.clone(),
            name: None,
            icon: None,
            group: Some(requested_group.trim().to_string()),
            expected_revision,
            fallback_names: capabilities.peripheral_names.clone(),
        },
    );
}

fn is_motion_control(control: &crate::four_d::controller::HardwareControl) -> bool {
    control.kind.eq_ignore_ascii_case("motion")
        || control.control.to_ascii_lowercase().contains("motion")
        || control.actions.iter().any(|action| {
            matches!(
                action.verb.to_ascii_lowercase().as_str(),
                "up" | "down" | "stop"
            )
        })
}

fn invoke_control_action(app: &PealayerApp, action_id: &str) {
    let _ = app.engine_handle.sender.send(
        crate::four_d::engine::EngineMessage::InvokeControllerAction {
            action_id: action_id.to_string(),
        },
    );
}

fn responsive_action_label(
    action: &crate::four_d::controller::HardwareAction,
    width: f32,
) -> String {
    let icon = crate::ui::icons::action(&action.verb);
    if width < 92.0 {
        icon.to_string()
    } else {
        format!("{icon} {}", action.name)
    }
}

fn draw_compact_control_card(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
) {
    let edit_id = ui.make_persistent_id(("control-name-editing", control.key.as_str()));
    let draft_id = ui.make_persistent_id(("control-name-draft", control.key.as_str()));
    let group_edit_id = ui.make_persistent_id(("control-group-editing", control.key.as_str()));
    let group_draft_id = ui.make_persistent_id(("control-group-draft", control.key.as_str()));
    let relay_id = relay_id_from_control_key(&control.key);
    let indicator_state = control_indicator_state(capabilities, control);
    let card = egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(9, 6))
        .corner_radius(7.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.set_max_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(crate::ui::icons::control(&control.kind, &control.icon))
                        .size(17.0),
                );
                let indicator = draw_control_indicator(app, ui, indicator_state, relay_id.is_some());
                if indicator.clicked() && let Some(id) = relay_id {
                    let turn_on = indicator_state != ControlIndicatorState::Active;
                    let _ = app.engine_handle.sender.send(
                        crate::four_d::engine::EngineMessage::ControllerCall {
                            method: "controller.command.execute".to_string(),
                            params: serde_json::json!({
                                "command": format!("relay {id} {}", if turn_on { "on" } else { "off" }),
                            }),
                        },
                    );
                }

                let editing = ui.data_mut(|data| data.get_temp::<bool>(edit_id).unwrap_or(false));
                let editing_group = ui
                    .data_mut(|data| data.get_temp::<bool>(group_edit_id).unwrap_or(false));
                if editing {
                    let mut draft = ui.data_mut(|data| {
                        data.get_temp::<String>(draft_id)
                            .unwrap_or_else(|| control.name.clone())
                    });
                    let edit_width = (ui.available_width() * 0.42).clamp(64.0, 190.0);
                    let edit = ui.add_sized(
                        [edit_width, 24.0],
                        egui::TextEdit::singleline(&mut draft).hint_text(&control.default_name),
                    );
                    if edit.changed() {
                        ui.data_mut(|data| data.insert_temp(draft_id, draft.clone()));
                    }
                    if ui.button(crate::ui::icons::FLOPPY_DISK).clicked()
                        || (edit.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                    {
                        update_control_name(app, capabilities, control, draft);
                        ui.data_mut(|data| data.insert_temp(edit_id, false));
                    }
                    if ui.button(crate::ui::icons::X).clicked() {
                        ui.data_mut(|data| data.insert_temp(edit_id, false));
                    }
                } else if editing_group {
                    let mut draft = ui.data_mut(|data| {
                        data.get_temp::<String>(group_draft_id)
                            .unwrap_or_else(|| control.group.clone())
                    });
                    let edit_width = (ui.available_width() * 0.42).clamp(64.0, 190.0);
                    let edit = ui.add_sized(
                        [edit_width, 24.0],
                        egui::TextEdit::singleline(&mut draft).hint_text(app.tr("No group")),
                    );
                    if edit.changed() {
                        ui.data_mut(|data| data.insert_temp(group_draft_id, draft.clone()));
                    }
                    if ui.button(crate::ui::icons::FLOPPY_DISK).clicked()
                        || (edit.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                    {
                        update_control_group(app, capabilities, control, draft);
                        ui.data_mut(|data| data.insert_temp(group_edit_id, false));
                    }
                    if ui.button(crate::ui::icons::X).clicked() {
                        ui.data_mut(|data| data.insert_temp(group_edit_id, false));
                    }
                } else {
                    let title = crate::ui::i18n::visual_text(app.language, &control.name);
                    let reserved = if is_pwm_control(control) {
                        150.0
                    } else if !control.actions.is_empty() {
                        (control.actions.len().min(3) as f32 * 34.0) + 8.0
                    } else if relay_id.is_some() {
                        76.0
                    } else {
                        8.0
                    };
                    let response = ui.add_sized(
                        [(ui.available_width() - reserved).max(48.0), 24.0],
                        egui::Label::new(egui::RichText::new(&title).strong())
                            .truncate()
                            .sense(egui::Sense::click()),
                    );
                    if response.clicked() {
                        ui.data_mut(|data| {
                            data.insert_temp(draft_id, control.name.clone());
                            data.insert_temp(edit_id, true);
                        });
                    }
                    response.on_hover_text(format!("{} — {}", title, app.tr("Rename")));
                }

                if is_pwm_control(control) {
                    if let Some(channel) = capabilities
                        .pwm_channels
                        .iter()
                        .find(|channel| channel.key == control.key)
                    {
                        let value_id = ui.make_persistent_id(("pwm_value", channel.id));
                        let raw = ui.data_mut(|data| {
                            data.get_temp::<u16>(value_id).unwrap_or_else(|| {
                                if capabilities.telemetry.pwm_channel == Some(channel.id) {
                                    capabilities.telemetry.pwm_value.unwrap_or(0)
                                } else {
                                    0
                                }
                            })
                        });
                        let mut percent = pwm_percent(raw);
                        let width = ui.available_width().max(96.0);
                        let response = ui.add_sized(
                            [width, 24.0],
                            egui::Slider::new(&mut percent, 0.0..=100.0)
                                .fixed_decimals(1)
                                .suffix("%"),
                        );
                        let raw = pwm_raw(percent);
                        ui.data_mut(|data| data.insert_temp(value_id, raw));
                        if response.drag_stopped() || response.lost_focus() {
                            let _ = app.engine_handle.sender.send(
                                crate::four_d::engine::EngineMessage::ControllerCall {
                                    method: "controller.pwm.set".to_string(),
                                    params: serde_json::json!({"channel": channel.id, "value": raw}),
                                },
                            );
                        }
                    }
                } else if !control.actions.is_empty() {
                    let is_motion = is_motion_control(control);
                    let stop = control
                        .actions
                        .iter()
                        .find(|action| action.verb.eq_ignore_ascii_case("stop"));
                    let show_stop = stop.is_some()
                        && (app.motion_control_mode == crate::config::MotionControlMode::Hold
                            || !capabilities.active_relays.is_empty());
                    let mut ordered = control
                        .actions
                        .iter()
                        .filter(|action| !action.verb.eq_ignore_ascii_case("stop"))
                        .collect::<Vec<_>>();
                    if show_stop && let Some(stop) = stop {
                        ordered.insert(ordered.len().min(1), stop);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for action in ordered.into_iter().rev() {
                            let response = ui
                                .add_enabled(
                                    !app.estop_active,
                                    egui::Button::new(crate::ui::icons::action(&action.verb))
                                        .min_size(egui::vec2(28.0, 26.0)),
                                )
                                .on_hover_text(crate::ui::i18n::visual_text(
                                    app.language,
                                    &action.name,
                                ));
                            if is_motion
                                && !action.verb.eq_ignore_ascii_case("stop")
                                && app.motion_control_mode == crate::config::MotionControlMode::Hold
                                && stop.is_some()
                            {
                                let held_id = ui.make_persistent_id((
                                    "compact-held-motion-action",
                                    action.id.as_str(),
                                ));
                                let was_held = ui.data_mut(|data| {
                                    data.get_temp::<bool>(held_id).unwrap_or(false)
                                });
                                let held = response.is_pointer_button_down_on();
                                if held && !was_held {
                                    invoke_control_action(app, &action.id);
                                } else if !held && was_held {
                                    invoke_control_action(app, &stop.expect("checked above").id);
                                }
                                ui.data_mut(|data| data.insert_temp(held_id, held));
                            } else if response.clicked() {
                                invoke_control_action(app, &action.id);
                            }
                        }
                    });
                } else if let Some(id) = relay_id {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for (state, icon, label) in [
                            (false, crate::ui::icons::STOP_CIRCLE, app.tr("OFF")),
                            (true, crate::ui::icons::LIGHTNING, app.tr("ON")),
                        ] {
                            if ui
                                .add_enabled(
                                    !app.estop_active,
                                    egui::Button::new(icon).min_size(egui::vec2(28.0, 26.0)),
                                )
                                .on_hover_text(label)
                                .clicked()
                            {
                                let _ = app.engine_handle.sender.send(
                                    crate::four_d::engine::EngineMessage::ControllerCall {
                                        method: "controller.command.execute".to_string(),
                                        params: serde_json::json!({
                                            "command": format!("relay {id} {}", if state { "on" } else { "off" })
                                        }),
                                    },
                                );
                            }
                        }
                    });
                }
            });
        });

    card.response.context_menu(|ui| {
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::PENCIL_SIMPLE,
                app.tr("Rename")
            ))
            .clicked()
        {
            ui.data_mut(|data| {
                data.insert_temp(draft_id, control.name.clone());
                data.insert_temp(edit_id, true);
            });
            ui.close();
        }
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::FOLDER_OPEN,
                app.tr("Change group")
            ))
            .clicked()
        {
            ui.data_mut(|data| {
                data.insert_temp(group_draft_id, control.group.clone());
                data.insert_temp(group_edit_id, true);
            });
            ui.close();
        }
        if !control.default_name.is_empty()
            && control.name != control.default_name
            && ui.button(app.tr("Restore default name")).clicked()
        {
            update_control_name(app, capabilities, control, String::new());
            ui.close();
        }
    });
}

fn draw_control_card(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
) {
    if app.compact_hardware_controls {
        draw_compact_control_card(app, ui, capabilities, control);
        return;
    }
    let edit_id = ui.make_persistent_id(("control-name-editing", control.key.as_str()));
    let draft_id = ui.make_persistent_id(("control-name-draft", control.key.as_str()));
    let source_id = ui.make_persistent_id(("control-name-source", control.key.as_str()));
    let group_edit_id = ui.make_persistent_id(("control-group-editing", control.key.as_str()));
    let group_draft_id = ui.make_persistent_id(("control-group-draft", control.key.as_str()));
    let relay_id = relay_id_from_control_key(&control.key);
    let indicator_state = control_indicator_state(capabilities, control);

    let card = egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(12, 10))
        .corner_radius(8.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.set_max_width(ui.available_width());
            let mut editing = ui.data_mut(|data| data.get_temp::<bool>(edit_id).unwrap_or(false));
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(crate::ui::icons::control(&control.kind, &control.icon))
                        .size(18.0),
                );
                let indicator = draw_control_indicator(app, ui, indicator_state, relay_id.is_some());
                if indicator.clicked() && let Some(id) = relay_id {
                    let turn_on = indicator_state != ControlIndicatorState::Active;
                    let _ = app.engine_handle.sender.send(
                        crate::four_d::engine::EngineMessage::ControllerCall {
                            method: "controller.command.execute".to_string(),
                            params: serde_json::json!({
                                "command": format!("relay {id} {}", if turn_on { "on" } else { "off" }),
                            }),
                        },
                    );
                }

                if editing {
                    let mut draft = ui.data_mut(|data| {
                        let source = data.get_temp::<String>(source_id);
                        if source.as_deref() != Some(control.name.as_str()) {
                            data.insert_temp(source_id, control.name.clone());
                            data.insert_temp(draft_id, control.name.clone());
                        }
                        data.get_temp::<String>(draft_id).unwrap_or_else(|| control.name.clone())
                    });
                    let mut edit_response = None;
                    let mut save_clicked = false;
                    let mut cancel_clicked = false;
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        cancel_clicked = ui
                            .button(crate::ui::icons::X)
                            .on_hover_text(app.tr("Cancel"))
                            .clicked();
                        save_clicked = ui
                            .button(crate::ui::icons::FLOPPY_DISK)
                            .on_hover_text(app.tr("Save name"))
                            .clicked();
                        let width = ui.available_width().max(56.0);
                        edit_response = Some(ui.add_sized(
                            [width, 24.0],
                            egui::TextEdit::singleline(&mut draft).hint_text(&control.default_name),
                        ));
                    });
                    let edit = edit_response.expect("rename editor is always rendered");
                    if edit.changed() {
                        ui.data_mut(|data| data.insert_temp(draft_id, draft.clone()));
                    }
                    if save_clicked
                        || (edit.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                    {
                        update_control_name(app, capabilities, control, draft);
                        editing = false;
                    } else if cancel_clicked
                        || ui.input(|input| input.key_pressed(egui::Key::Escape))
                    {
                        editing = false;
                    }
                    ui.data_mut(|data| data.insert_temp(edit_id, editing));
                } else {
                    let title_text = crate::ui::i18n::visual_text(app.language, &control.name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(crate::ui::icons::PENCIL_SIMPLE)
                            .on_hover_text(app.tr("Rename")).clicked()
                        {
                            ui.data_mut(|data| data.insert_temp(edit_id, true));
                        }
                        let title = ui.add_sized(
                            [ui.available_width().max(52.0), 24.0],
                            egui::Label::new(
                                egui::RichText::new(&title_text).strong().size(13.0),
                            )
                            .truncate()
                            .sense(egui::Sense::click()),
                        );
                        if title.clicked() {
                            ui.data_mut(|data| data.insert_temp(edit_id, true));
                        }
                        title.on_hover_text(format!("{} — {}", title_text, app.tr("Rename")));
                    });
                }
            });

            let mut editing_group = ui
                .data_mut(|data| data.get_temp::<bool>(group_edit_id).unwrap_or(false));
            if editing_group {
                let mut draft = ui.data_mut(|data| {
                    data.get_temp::<String>(group_draft_id)
                        .unwrap_or_else(|| control.group.clone())
                });
                ui.horizontal(|ui| {
                    ui.label(app.tr("Group"));
                    let edit = ui.add_sized(
                        [ui.available_width().max(80.0) - 52.0, 24.0],
                        egui::TextEdit::singleline(&mut draft).hint_text(app.tr("No group")),
                    );
                    if edit.changed() {
                        ui.data_mut(|data| data.insert_temp(group_draft_id, draft.clone()));
                    }
                    if ui.button(crate::ui::icons::FLOPPY_DISK).clicked()
                        || (edit.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                    {
                        update_control_group(app, capabilities, control, draft);
                        editing_group = false;
                    }
                    if ui.button(crate::ui::icons::X).clicked()
                        || ui.input(|input| input.key_pressed(egui::Key::Escape))
                    {
                        editing_group = false;
                    }
                });
                ui.data_mut(|data| data.insert_temp(group_edit_id, editing_group));
            }

            if is_pwm_control(control) {
                if let Some(channel) = capabilities.pwm_channels.iter()
                    .find(|channel| channel.key == control.key)
                {
                    let value_id = ui.make_persistent_id(("pwm_value", channel.id));
                    let raw_value = ui.data_mut(|data| {
                        data.get_temp::<u16>(value_id).unwrap_or_else(|| {
                            if capabilities.telemetry.pwm_channel == Some(channel.id) {
                                capabilities.telemetry.pwm_value.unwrap_or(0)
                            } else { 0 }
                        })
                    });
                    let mut percent = pwm_percent(raw_value);
                    ui.add_space(if app.compact_hardware_controls { 3.0 } else { 8.0 });
                    let mut commit = false;
                    ui.horizontal(|ui| {
                        let number_width = 66.0;
                        let slider_width = (ui.available_width() - number_width - 8.0).max(72.0);
                        let slider = ui.add_sized(
                            [slider_width, 24.0],
                            egui::Slider::new(&mut percent, 0.0..=100.0).show_value(false),
                        );
                        let value = ui.add_sized(
                            [number_width, 24.0],
                            egui::DragValue::new(&mut percent)
                                .range(0.0..=100.0)
                                .speed(0.1)
                                .fixed_decimals(1)
                                .suffix("%"),
                        );
                        commit = slider.drag_stopped()
                            || value.lost_focus()
                            || (value.changed()
                                && ui.input(|input| input.key_pressed(egui::Key::Enter)));
                    });
                    let raw = pwm_raw(percent);
                    ui.data_mut(|data| data.insert_temp(value_id, raw));
                    if commit {
                        let _ = app.engine_handle.sender.send(
                            crate::four_d::engine::EngineMessage::ControllerCall {
                                method: "controller.pwm.set".to_string(),
                                params: serde_json::json!({"channel": channel.id, "value": raw}),
                            },
                        );
                    }
                }
            }

            if !control.actions.is_empty() {
                ui.add_space(if app.compact_hardware_controls { 3.0 } else { 8.0 });
                let is_motion = is_motion_control(control);
                let stop_action = control
                    .actions
                    .iter()
                    .find(|action| action.verb.eq_ignore_ascii_case("stop"));
                let directional = control
                    .actions
                    .iter()
                    .filter(|action| !action.verb.eq_ignore_ascii_case("stop"))
                    .collect::<Vec<_>>();
                let show_stop = stop_action.is_some()
                    && (app.motion_control_mode == crate::config::MotionControlMode::Hold
                        || capabilities.active_relays.iter().next().is_some());
                let action_rows = if is_motion { vec![directional] } else { vec![control.actions.iter().collect()] };
                for action_row in action_rows {
                    let columns = action_grid_columns(ui.available_width(), action_row.len());
                    ui.columns(columns, |uis| {
                        for (index, action) in action_row.into_iter().enumerate() {
                            let ui = &mut uis[index];
                            let visual_name = crate::ui::i18n::visual_text(app.language, &action.name);
                            let label = responsive_action_label(action, ui.available_width());
                            let verb = action.verb.to_ascii_lowercase();
                            let selected = relay_id.is_some()
                                && ((verb == "on" && indicator_state == ControlIndicatorState::Active)
                                    || (verb == "off" && indicator_state == ControlIndicatorState::Inactive));
                            let mut button = egui::Button::new(label)
                                .truncate()
                                .selected(selected);
                            if selected && verb == "on" {
                                button = button
                                    .fill(egui::Color32::from_rgb(22, 163, 74))
                                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(34, 197, 94)));
                            }
                            let response = ui.add_enabled_ui(!app.estop_active, |ui| {
                                ui.add_sized([ui.available_width(), 28.0], button)
                            }).inner.on_hover_text(visual_name);
                            if is_motion
                                && app.motion_control_mode == crate::config::MotionControlMode::Hold
                                && stop_action.is_some()
                            {
                                let held_id = ui.make_persistent_id(("held-motion-action", action.id.as_str()));
                                let was_held = ui.data_mut(|data| data.get_temp::<bool>(held_id).unwrap_or(false));
                                let held = response.is_pointer_button_down_on();
                                if held && !was_held {
                                    invoke_control_action(app, &action.id);
                                } else if !held && was_held {
                                    invoke_control_action(app, &stop_action.expect("checked above").id);
                                }
                                ui.data_mut(|data| data.insert_temp(held_id, held));
                            } else if response.clicked() {
                                invoke_control_action(app, &action.id);
                            }
                        }
                    });
                }
                if show_stop && let Some(stop) = stop_action {
                    ui.add_space(5.0);
                    ui.horizontal_centered(|ui| {
                        let width = ui.available_width().clamp(86.0, 160.0);
                        let label = responsive_action_label(stop, width);
                        if ui
                            .add_enabled(
                                !app.estop_active,
                                egui::Button::new(label).min_size(egui::vec2(width, 28.0)),
                            )
                            .on_hover_text(crate::ui::i18n::visual_text(app.language, &stop.name))
                            .clicked()
                        {
                            invoke_control_action(app, &stop.id);
                        }
                    });
                }
            } else if let Some(relay_id) = relay_id {
                ui.add_space(8.0);
                ui.columns(2, |uis| {
                    for (index, (state, label, icon)) in [
                        (true, app.tr("ON"), crate::ui::icons::LIGHTNING),
                        (false, app.tr("OFF"), crate::ui::icons::STOP_CIRCLE),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let selected = capabilities.active_relays.contains(&relay_id) == state;
                        let mut button = egui::Button::new(format!("{icon} {label}"))
                            .truncate()
                            .selected(selected);
                        if selected && state {
                            button = button
                                .fill(egui::Color32::from_rgb(22, 163, 74))
                                .stroke(egui::Stroke::new(
                                    1.0_f32,
                                    egui::Color32::from_rgb(34, 197, 94),
                                ));
                        }
                        if uis[index]
                            .add_enabled_ui(!app.estop_active, |ui| {
                                ui.add_sized([ui.available_width(), 28.0], button)
                            })
                            .inner
                            .clicked()
                        {
                            let _ = app.engine_handle.sender.send(
                                crate::four_d::engine::EngineMessage::ControllerCall {
                                    method: "controller.command.execute".to_string(),
                                    params: serde_json::json!({
                                        "command": format!(
                                            "relay {relay_id} {}",
                                            if state { "on" } else { "off" }
                                        )
                                    }),
                                },
                            );
                        }
                    }
                });
            }
        });

    // Use the frame's own response for the context menu. A second full-card
    // interaction layer would sit above and steal clicks from every child.
    card.response.context_menu(|ui| {
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::PENCIL_SIMPLE,
                app.tr("Rename")
            ))
            .clicked()
        {
            ui.data_mut(|data| data.insert_temp(edit_id, true));
            ui.close();
        }
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::FOLDER_OPEN,
                app.tr("Change group")
            ))
            .clicked()
        {
            ui.data_mut(|data| {
                data.insert_temp(group_draft_id, control.group.clone());
                data.insert_temp(group_edit_id, true);
            });
            ui.close();
        }
        if !control.default_name.is_empty()
            && control.name != control.default_name
            && ui.button(app.tr("Restore default name")).clicked()
        {
            update_control_name(app, capabilities, control, String::new());
            ui.close();
        }
    });
}

fn draw_control_card_grid(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    controls: &[crate::four_d::controller::HardwareControl],
) {
    let mut groups: Vec<(String, Vec<&crate::four_d::controller::HardwareControl>)> = Vec::new();
    for control in controls {
        let group = control.group.trim();
        if let Some((_, members)) = groups.iter_mut().find(|(name, _)| name == group) {
            members.push(control);
        } else {
            groups.push((group.to_string(), vec![control]));
        }
    }
    let show_group_headers = groups.iter().any(|(name, _)| !name.is_empty());
    for (group, members) in groups {
        if show_group_headers && !group.is_empty() {
            ui.horizontal(|ui| {
                ui.label(crate::ui::icons::FOLDER_OPEN);
                ui.label(
                    egui::RichText::new(crate::ui::i18n::visual_text(app.language, &group))
                        .small()
                        .strong(),
                );
                ui.separator();
            });
            ui.add_space(4.0);
        }
        let columns = control_grid_columns(ui.available_width());
        for row in members.chunks(columns) {
            ui.columns(columns, |uis| {
                for (index, control) in row.iter().enumerate() {
                    draw_control_card(app, &mut uis[index], capabilities, control);
                }
            });
            ui.add_space(8.0);
        }
        if show_group_headers && !group.is_empty() {
            ui.add_space(3.0);
        }
    }
}

#[cfg(test)]
mod timeline_row_tests {
    use super::*;

    #[test]
    fn dynamic_rows_map_only_explicit_relays() {
        let rows = vec![
            TimelineTrackRow {
                name: "Video".to_string(),
                kind: TimelineTrackKind::Video,
            },
            TimelineTrackRow {
                name: "Seat left".to_string(),
                kind: TimelineTrackKind::Relay(6),
            },
        ];
        assert_eq!(relay_for_timeline_row(&rows, 0), None);
        assert_eq!(relay_for_timeline_row(&rows, 1), Some(6));
        assert_eq!(timeline_row_for_relay(&rows, 6), Some(1));
        assert_eq!(timeline_row_for_relay(&rows, 1), None);
    }

    #[test]
    fn narrow_hardware_sidebars_use_single_column_cards_and_actions() {
        assert_eq!(control_grid_columns(420.0), 1);
        assert_eq!(action_grid_columns(210.0, 2), 1);
        assert_eq!(action_grid_columns(320.0, 3), 1);
        assert_eq!(control_grid_columns(720.0), 2);
        assert_eq!(action_grid_columns(280.0, 2), 2);
        assert_eq!(action_grid_columns(420.0, 3), 3);
    }

    #[test]
    fn pwm_editor_maps_full_raw_range_to_decimal_percentages() {
        assert_eq!(pwm_percent(0), 0.0);
        assert_eq!(pwm_percent(4095), 100.0);
        assert_eq!(pwm_raw(0.0), 0);
        assert_eq!(pwm_raw(100.0), 4095);
        assert_eq!(pwm_raw(12.5), 512);
    }

    #[test]
    fn effect_drag_translation_preserves_the_pointer_grab_offset() {
        let source_min = egui::pos2(100.0, 80.0);
        let grab_offset = egui::vec2(17.0, 11.0);
        let pointer = egui::pos2(430.0, 260.0);
        let translation = drag_translation(pointer, source_min, grab_offset);
        assert_eq!(source_min + translation + grab_offset, pointer);
    }

    #[test]
    fn effect_drag_start_publishes_the_payload_in_the_same_frame() {
        let context = egui::Context::default();
        let payload = EffectDragPayload {
            name: "Seat rise".to_string(),
            icon: String::new(),
            duration_ms: 750,
            target: crate::four_d::models::HardwareTarget::ControllerMacro,
            actions: Vec::new(),
            controller_macro: Some(crate::four_d::models::ControllerMacroCue {
                id: 7,
                mode: "mcu".to_string(),
            }),
            controller_strip_effect: None,
        };
        begin_effect_drag(&context, payload.clone());
        assert_eq!(
            egui::DragAndDrop::payload::<EffectDragPayload>(&context).as_deref(),
            Some(&payload)
        );
    }

    #[test]
    fn effect_card_completes_a_real_pointer_drag_and_drop_cycle() {
        let context = egui::Context::default();
        let payload = EffectDragPayload {
            name: "Seat rise".to_string(),
            icon: String::new(),
            duration_ms: 750,
            target: crate::four_d::models::HardwareTarget::ControllerMacro,
            actions: Vec::new(),
            controller_macro: Some(crate::four_d::models::ControllerMacroCue {
                id: 7,
                mode: "mcu".to_string(),
            }),
            controller_strip_effect: None,
        };
        let source = std::cell::Cell::new(egui::Rect::NOTHING);
        let target = std::cell::Cell::new(egui::Rect::NOTHING);
        let dropped = std::cell::Cell::new(false);
        let render = |events: Vec<egui::Event>| {
            let output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(480.0, 320.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    source.set(
                        effect_drag_source(
                            ui,
                            egui::Id::new("verified-effect-card"),
                            payload.clone(),
                            |ui| ui.add_sized([180.0, 40.0], egui::Label::new("Seat rise")),
                        )
                        .response
                        .rect,
                    );
                    ui.add_space(90.0);
                    let (zone, released) = ui
                        .dnd_drop_zone::<EffectDragPayload, _>(egui::Frame::NONE, |ui| {
                            ui.allocate_exact_size(egui::vec2(300.0, 70.0), egui::Sense::hover())
                        });
                    target.set(zone.response.rect);
                    dropped.set(dropped.get() || released.as_deref() == Some(&payload));
                },
            );
            drop(output);
        };

        render(Vec::new());
        let source_point = source.get().center();
        render(vec![
            egui::Event::PointerMoved(source_point),
            egui::Event::PointerButton {
                pos: source_point,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        render(vec![egui::Event::PointerMoved(
            source_point + egui::vec2(18.0, 4.0),
        )]);
        let target_point = target.get().center();
        render(vec![egui::Event::PointerMoved(target_point)]);
        render(vec![egui::Event::PointerButton {
            pos: target_point,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);

        assert!(
            dropped.get(),
            "effect card payload was not released by the drop zone"
        );
    }

    #[test]
    fn indicators_use_authoritative_output_state() {
        let mut capabilities = crate::four_d::controller::HardwareCapabilities::default();
        capabilities.active_relays.insert(2);
        capabilities
            .pwm_channels
            .push(crate::four_d::controller::HardwareOutput {
                id: 0,
                key: "pwm.0".into(),
                name: "MOSFET 1".into(),
                role: "user-output".into(),
                control: "pwm-user".into(),
            });
        capabilities.telemetry.pwm_channel = Some(0);
        capabilities.telemetry.pwm_value = Some(1024);

        let control = |key: &str, kind: &str| crate::four_d::controller::HardwareControl {
            key: key.into(),
            kind: kind.into(),
            ..Default::default()
        };
        assert_eq!(
            control_indicator_state(&capabilities, &control("relay.2", "relay")),
            ControlIndicatorState::Active
        );
        assert_eq!(
            control_indicator_state(&capabilities, &control("relay.3", "relay")),
            ControlIndicatorState::Inactive
        );
        assert_eq!(
            control_indicator_state(&capabilities, &control("pwm.0", "mosfet")),
            ControlIndicatorState::Active
        );
        assert_eq!(
            control_indicator_state(&capabilities, &control("seat.left", "motion")),
            ControlIndicatorState::Available
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PealayerTab {
    ProgramMonitor,
    EffectControls,
    EffectsLibrary,
    HardwareMonitor,
    Timeline,
}

impl PealayerTab {
    pub const ALL: [PealayerTab; 5] = [
        PealayerTab::ProgramMonitor,
        PealayerTab::Timeline,
        PealayerTab::EffectControls,
        PealayerTab::EffectsLibrary,
        PealayerTab::HardwareMonitor,
    ];

    pub fn title(self, app: &crate::app::PealayerApp) -> String {
        match self {
            PealayerTab::ProgramMonitor => app.tr("Program Monitor"),
            PealayerTab::Timeline => app.tr("Timeline"),
            PealayerTab::EffectControls => app.tr("Effect Controls"),
            PealayerTab::EffectsLibrary => app.tr("Effects Library"),
            PealayerTab::HardwareMonitor => app.tr("Hardware Monitor"),
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            PealayerTab::ProgramMonitor => crate::ui::icons::MONITOR_PLAY,
            PealayerTab::Timeline => crate::ui::icons::WAVEFORM,
            PealayerTab::EffectControls => crate::ui::icons::SLIDERS_HORIZONTAL,
            PealayerTab::EffectsLibrary => crate::ui::icons::SPARKLE,
            PealayerTab::HardwareMonitor => crate::ui::icons::GAUGE,
        }
    }
}

pub fn visible_workspace_tab_count(app: &PealayerApp) -> usize {
    PealayerTab::ALL
        .into_iter()
        .filter(|tab| app.is_tab_open(*tab))
        .count()
}

pub fn draw_workspace_tab_menu(app: &mut PealayerApp, ui: &mut egui::Ui) {
    for tab in PealayerTab::ALL {
        let is_open = app.is_tab_open(tab);
        let mut visible = is_open;
        let visibility_icon = if is_open {
            crate::ui::icons::EYE
        } else {
            crate::ui::icons::EYE_SLASH
        };
        let label = format!("{visibility_icon}  {}  {}", tab.icon(), tab.title(app));
        if ui.checkbox(&mut visible, label).changed() {
            if visible && !is_open {
                app.open_or_focus_tab(tab);
            } else if !visible && is_open {
                app.toggle_tab(tab);
            }
            ui.close();
        }
    }
}

pub struct PealayerTabViewer<'a> {
    pub app: &'a mut PealayerApp,
}

impl<'a> TabViewer for PealayerTabViewer<'a> {
    type Tab = PealayerTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        format!("{} {}", tab.icon(), tab.title(self.app)).into()
    }

    fn context_menu(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab, _path: egui_dock::NodePath) {
        let title = tab.title(self.app);
        ui.label(egui::RichText::new(title).strong());
        ui.separator();
        match tab {
            PealayerTab::ProgramMonitor => {
                let is_fullscreen = self.app.fullscreen_intent(ui.ctx());
                let label = if is_fullscreen {
                    self.app.tr("Exit Fullscreen")
                } else {
                    self.app.tr("Fullscreen")
                };
                if ui
                    .button(format!("{} {label}", crate::ui::icons::ARROWS_OUT))
                    .clicked()
                {
                    self.app.toggle_fullscreen(ui.ctx());
                    ui.close();
                }
            }
            PealayerTab::Timeline => {
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::CHECK_SQUARE,
                        self.app.tr("Select all cues")
                    ))
                    .clicked()
                {
                    self.app.selected_instance_ids = self
                        .app
                        .timeline
                        .instances
                        .iter()
                        .map(|instance| instance.id)
                        .collect();
                    ui.close();
                }
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                        self.app.tr("Reset zoom")
                    ))
                    .clicked()
                {
                    self.app.timeline_zoom = 100.0;
                    ui.close();
                }
            }
            PealayerTab::HardwareMonitor => {
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::PLUG,
                        self.app.tr("Hardware preferences")
                    ))
                    .clicked()
                {
                    self.app.preferences_tab = 2;
                    self.app.show_preferences_dialog = true;
                    ui.close();
                }
            }
            _ => {}
        }
        ui.separator();
        ui.label(egui::RichText::new(self.app.tr("Panels")).strong());
        draw_workspace_tab_menu(self.app, ui);
        ui.separator();
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::GEAR,
                self.app.tr("Preferences...")
            ))
            .clicked()
        {
            self.app.show_preferences_dialog = true;
            ui.close();
        }
    }

    fn on_tab_button(&mut self, _tab: &mut Self::Tab, response: &egui::Response) {
        let id = egui::Id::new("workspace-tab-button-rects");
        response.ctx.data_mut(|data| {
            let mut rects = data.get_temp::<Vec<egui::Rect>>(id).unwrap_or_default();
            rects.push(response.rect);
            data.insert_temp(id, rects);
        });
    }

    fn scroll_bars(&self, tab: &Self::Tab) -> [bool; 2] {
        match tab {
            PealayerTab::EffectsLibrary | PealayerTab::HardwareMonitor => [false, true],
            _ => [true, true],
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let display_language = self.app.language;
        let replay_label = self.app.tr("Replay");
        let play_label = self.app.tr("Play");
        let pause_label = self.app.tr("Pause");
        let stop_label = self.app.tr("Stop");
        let delete_all_label = self.app.tr("Delete All Selected");
        let timeline_delete_cue_label = self.app.tr("Delete Cue");
        let estop_banner_label = self
            .app
            .tr("EMERGENCY STOP ACTIVE - ALL HARDWARE OUTPUTS DISABLED");
        let relay_mute_help = self
            .app
            .tr("Mute Track (M)\nMutes relay physical output during playback.");
        let relay_solo_help = self
            .app
            .tr("Solo Track (S)\nSolos this relay track output during playback.");
        let lock_help = self
            .app
            .tr("Lock Track (L)\nPrevents moving or modifying effects on this track.");
        let actuator_mute_help = self
            .app
            .tr("Mute Track (M)\nMutes actuator physical output during playback.");
        let record_arm_help = self
            .app
            .tr("Record Arm (R)\nArms this track for real-time motion capture gesture recording.");
        let live_fader_help = self
            .app
            .tr("Live Actuator Fader\nControl actuator intensity in real time (0% - 100%).");
        let add_keyframe_help = self
            .app
            .tr("Add Keyframe\nInserts a keyframe at the current playhead position.");
        let timeline_ruler_help = self
            .app
            .tr("Timeline Ruler\nClick or drag to scrub playhead. Scroll to zoom time.");
        let linear_label = self.app.tr("Linear");
        let smooth_label = self.app.tr("Smooth (Hermite)");
        let step_label = self.app.tr("Step");
        let delete_keyframe_label = self.app.tr("Delete Keyframe");
        let interpolation_label = self.app.tr("Interpolation");
        let time_label = self.app.tr("Time");
        let value_label = self.app.tr("Value");
        let analog_track_label = self.app.tr("Analog Track");
        let port_channel_label = self.app.tr("Port/Channel");
        // High density styling for text elements
        ui.style_mut().override_text_style = Some(egui::TextStyle::Body);

        egui::Frame::NONE
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
              ui.with_layout(crate::ui::i18n::vertical_layout(self.app.rtl), |ui| {
                match tab {
                    PealayerTab::ProgramMonitor => {
                        ui.vertical(|ui| {
                            // ponytail: reserve 35px at bottom for inline transport controls
                            let video_h = (ui.available_height() - 35.0).max(0.0);
                            let video_size = egui::vec2(ui.available_width(), video_h);
                            ui.allocate_ui_with_layout(video_size, egui::Layout::top_down(egui::Align::Center), |ui| {
                                crate::ui::video::draw(self.app, ui);
                            });

                            ui.add_space(5.0);

                            let has_video = self.app.current_video_path.is_some();
                            let can_seek = has_video
                                && self.app.is_seekable
                                && self.app.duration > 0.0;
                            ui.add_enabled_ui(has_video, |ui| {
                                ui.horizontal(|ui| {
                                    let play_icon = if self.app.is_playback_finished() {
                                        crate::ui::icons::ARROW_COUNTER_CLOCKWISE
                                    } else if self.app.is_paused {
                                        crate::ui::icons::PLAY
                                    } else {
                                        crate::ui::icons::PAUSE
                                    };
                                    let play_tooltip = if self.app.is_playback_finished() {
                                        &replay_label
                                    } else if self.app.is_paused {
                                        &play_label
                                    } else {
                                        &pause_label
                                    };
                                    let play_response = ui
                                        .add_sized([30.0, 22.0], egui::Button::new(play_icon))
                                        .on_hover_text(play_tooltip);
                                    play_response.context_menu(|ui| {
                                        crate::ui::controls::transport_context_menu(self.app, ui)
                                    });
                                    if play_response.clicked() {
                                        self.app.toggle_playback();
                                    }
                                    let stop_response = ui
                                        .add_sized(
                                            [30.0, 22.0],
                                            egui::Button::new(crate::ui::icons::STOP_CIRCLE),
                                        )
                                        .on_hover_text(&stop_label);
                                    stop_response.context_menu(|ui| {
                                        crate::ui::controls::transport_context_menu(self.app, ui)
                                    });
                                    if stop_response.clicked() {
                                        // Punch out on stop
                                        self.app.commit_recorded_samples();

                                        let _ = self.app.mpv.command("seek", &["0", "absolute+exact"]);
                                        let _ = self.app.mpv.set_property("pause", true);
                                        self.app.is_paused = true;
                                        self.app.is_eof = false;
                                        self.app.playback_time = 0.0;
                                        self.app.seek_pos = None;
                                    }
                                    let frame_back = ui
                                        .add_sized(
                                            [30.0, 22.0],
                                            egui::Button::new(crate::ui::icons::SKIP_BACK),
                                        )
                                        .on_hover_text(format!(
                                            "{} {} {} ([)",
                                            self.app.tr("Back"),
                                            self.app.frame_step_count,
                                            self.app.tr("frames")
                                        ));
                                    frame_back.context_menu(|ui| {
                                        crate::ui::controls::transport_context_menu(self.app, ui)
                                    });
                                    if frame_back.clicked() {
                                        self.app.step_frames(-1);
                                    }
                                    let frame_forward = ui
                                        .add_sized(
                                            [30.0, 22.0],
                                            egui::Button::new(crate::ui::icons::SKIP_FORWARD),
                                        )
                                        .on_hover_text(format!(
                                            "{} {} {} (])",
                                            self.app.tr("Forward"),
                                            self.app.frame_step_count,
                                            self.app.tr("frames")
                                        ));
                                    frame_forward.context_menu(|ui| {
                                        crate::ui::controls::transport_context_menu(self.app, ui)
                                    });
                                    if frame_forward.clicked() {
                                        self.app.step_frames(1);
                                    }
                                    ui.separator();

                                    let elapsed = self.app.seek_pos.unwrap_or(self.app.playback_time);
                                    let include_hours = self.app.duration >= 3600.0;
                                    let elapsed_response = crate::ui::controls::draw_elapsed_editor(
                                        self.app,
                                        ui,
                                        "nle-elapsed-editor",
                                        can_seek,
                                    );
                                    elapsed_response.context_menu(|ui| {
                                        crate::ui::controls::transport_context_menu(self.app, ui)
                                    });

                                    // seekbar
                                    let mut current_pos = if has_video {
                                        self.app.seek_pos.unwrap_or(self.app.playback_time)
                                    } else {
                                        0.0
                                    };
                                    let max_dur = if has_video && self.app.duration > 0.0 {
                                        self.app.duration
                                    } else {
                                        1.0
                                    };
                                    let slider = egui::Slider::new(&mut current_pos, 0.0..=max_dur)
                                        .show_value(false)
                                        .trailing_fill(true);

                                    let seekbar_w = (ui.available_width() - 180.0).max(50.0);
                                    let old_w = ui.spacing().slider_width;
                                    ui.spacing_mut().slider_width = seekbar_w;
                                    let response = ui.add_enabled(can_seek, slider);
                                    ui.spacing_mut().slider_width = old_w;
                                    response.context_menu(|ui| {
                                        crate::ui::controls::transport_context_menu(self.app, ui)
                                    });

                                    if let Some(buffered_until) = self.app.buffered_until() {
                                        let fraction = (buffered_until / self.app.duration)
                                            .clamp(0.0, 1.0)
                                            as f32;
                                        let buffered_rect = egui::Rect::from_min_max(
                                            egui::pos2(
                                                response.rect.left(),
                                                response.rect.bottom() - 2.0,
                                            ),
                                            egui::pos2(
                                                response.rect.left()
                                                    + response.rect.width() * fraction,
                                                response.rect.bottom(),
                                            ),
                                        );
                                        ui.painter().rect_filled(
                                            buffered_rect,
                                            1.0,
                                            ui.visuals()
                                                .selection
                                                .bg_fill
                                                .linear_multiply(0.55),
                                        );
                                    }

                                    if can_seek && response.dragged() {
                                        self.app.scrub_to(current_pos);
                                    }
                                    if can_seek && response.drag_stopped() {
                                        self.app.finish_scrub(current_pos);
                                    }

                                    let displayed_total = if self.app.show_remaining_time {
                                        -(self.app.duration - elapsed).max(0.0)
                                    } else {
                                        self.app.duration
                                    };
                                    let timeline_state = self.app.media_timeline_state();
                                    let (total_label, total_tooltip, finite_timeline) = match timeline_state {
                                        crate::media::MediaTimelineState::Determining => (
                                            format!(
                                                "{} {}",
                                                crate::ui::icons::HOURGLASS_MEDIUM,
                                                self.app.tr("Determining…")
                                            ),
                                            self.app.tr("MPV is still reading media metadata; duration and seeking will update when available."),
                                            false,
                                        ),
                                        crate::media::MediaTimelineState::Live => (
                                            format!(
                                                "{} {}",
                                                crate::ui::icons::BROADCAST,
                                                self.app.tr("LIVE")
                                            ),
                                            self.app.tr("This live or duration-less source has no fixed endpoint."),
                                            false,
                                        ),
                                        crate::media::MediaTimelineState::Finite { .. } => (
                                            crate::ui::controls::format_player_time(
                                                displayed_total,
                                                include_hours,
                                                self.app.show_subseconds,
                                            ),
                                            self.app.tr("Toggle duration / remaining time"),
                                            true,
                                        ),
                                        crate::media::MediaTimelineState::NoMedia => (
                                            format!("{} --:--", crate::ui::icons::CLOCK),
                                            self.app.tr("Open media to see its duration."),
                                            false,
                                        ),
                                    };
                                    let total_text = if finite_timeline {
                                        crate::ui::controls::timecode_text(total_label)
                                    } else {
                                        egui::RichText::new(total_label)
                                    };
                                    let total_response = ui
                                        .add(
                                            egui::Label::new(total_text).sense(if finite_timeline {
                                                egui::Sense::click()
                                            } else {
                                                egui::Sense::hover()
                                            }),
                                        )
                                        .on_hover_text(total_tooltip);
                                    total_response.context_menu(|ui| {
                                        crate::ui::controls::transport_context_menu(self.app, ui)
                                    });
                                    if finite_timeline && total_response.clicked() {
                                        self.app.show_remaining_time = !self.app.show_remaining_time;
                                        self.app.save_config();
                                    }
                                    let fullscreen_response = ui
                                        .button(crate::ui::icons::ARROWS_OUT)
                                        .on_hover_text(format!("{} (F)", self.app.tr("Fullscreen")));
                                    fullscreen_response.context_menu(|ui| {
                                        crate::ui::controls::transport_context_menu(self.app, ui)
                                    });
                                    if fullscreen_response.clicked() {
                                        self.app.set_fullscreen(ui.ctx(), true);
                                    }
                                });
                            });
                        });
                    }
                    PealayerTab::EffectControls => {
                        let advertised_relays = self.app
                            .advertised_hardware()
                            .filter(|capabilities| capabilities.board_connected)
                            .map(|capabilities| capabilities.relays)
                            .unwrap_or_default();
                        let selected_count = self.app.selected_instance_ids.len();

                        if selected_count == 1 {
                            let id = *self.app.selected_instance_ids.iter().next().unwrap();
                            let mut timeline_dirty = false;
                            let mut delete_cue = false;
                            let mut relocate_effect_id = None;

                            ui.heading(self.app.tr("Effect Controls"));
                            ui.add_space(8.0);

                            let mut instance_idx = None;
                            for (idx, inst) in self.app.timeline.instances.iter().enumerate() {
                                if inst.id == id {
                                    instance_idx = Some(idx);
                                    break;
                                }
                            }

                            let mut push_undo = false;
                            let mut isolate_instance = false;
                            let mut update_start_to = None;
                            let mut update_duration_to = None;
                            let mut update_relay_to = None;

                            if let Some(idx) = instance_idx {
                                let identity_label = self.app.tr("Identity");
                                let name_label = self.app.tr("Name:");
                                let timing_label = self.app.tr("Timing constraints");
                                let start_time_label = self.app.tr("Start time:");
                                let duration_label = self.app.tr("Duration:");
                                let hardware_target_label = self.app.tr("Hardware target");
                                let unavailable_output_label = self.app.tr("Unavailable output");
                                let unavailable_project_output_label =
                                    self.app.tr("Unavailable project output");
                                let connect_output_label = self
                                    .app
                                    .tr("Connect PCController to choose an available output.");
                                let target_output_label = self.app.tr("Target output:");
                                let delete_cue_label = self.app.tr("Delete Cue");
                                let max_secs = if self.app.duration > 0.0 {
                                    self.app.duration
                                } else {
                                    60.0
                                };
                                let instance = &mut self.app.timeline.instances[idx];

                                let mut template_idx = None;
                                for (t_idx, tmpl) in self.app.timeline.templates.iter().enumerate() {
                                    if tmpl.id == instance.effect_id {
                                        template_idx = Some(t_idx);
                                        break;
                                    }
                                }

                                if let Some(t_idx) = template_idx {
                                    let template = &mut self.app.timeline.templates[t_idx];

                                    ui.group(|ui| {
                                        ui.strong(&identity_label);
                                        ui.add_space(4.0);

                                        ui.horizontal(|ui| {
                                            ui.label(&name_label);
                                            if ui.text_edit_singleline(&mut template.name).changed() {
                                                timeline_dirty = true;
                                            }
                                        });

                                        ui.label(format!("Cue ID: {}", id));
                                        ui.label(format!("Template ID: {}", template.id));
                                    });

                                    ui.add_space(8.0);

                                    ui.group(|ui| {
                                        ui.strong(&timing_label);
                                        ui.add_space(4.0);

                                        let mut start_secs = instance.start_time_ms as f64 / 1000.0;
                                        ui.horizontal(|ui| {
                                            ui.label(&start_time_label);
                                            let slider = ui.add(egui::Slider::new(&mut start_secs, 0.0..=max_secs).suffix("s"));
                                            if slider.drag_started() || (slider.changed() && !slider.dragged()) {
                                                push_undo = true;
                                            }
                                            if slider.changed() {
                                                update_start_to = Some((start_secs * 1000.0) as u64);
                                                timeline_dirty = true;
                                            }
                                        });

                                        let mut duration_ms = template.duration_ms as f64;
                                        ui.horizontal(|ui| {
                                            ui.label(&duration_label);
                                            let slider = ui.add(egui::Slider::new(&mut duration_ms, 50.0..=60000.0).suffix("ms"));
                                            if slider.drag_started() || (slider.changed() && !slider.dragged()) {
                                                push_undo = true;
                                            }
                                            if slider.changed() {
                                                isolate_instance = true;
                                                update_duration_to = Some(duration_ms as u64);
                                                timeline_dirty = true;
                                            }
                                        });
                                    });

                                    ui.add_space(8.0);

                                    let is_controller_owned = template.controller_macro.is_some()
                                        || template.controller_strip_effect.is_some();
                                    let current_relay_id = template.actions.first().map(|a| a.relay_id).unwrap_or(0);
                                    if !is_controller_owned {
                                    let is_mismatched = !template.target.is_compatible_with_relay(current_relay_id);
                                    if is_mismatched {
                                        let configured_name = template
                                            .target
                                            .primary_relay_id()
                                            .and_then(|target_id| advertised_relays.iter().find(|relay| relay.id == target_id))
                                            .map(|relay| {
                                                crate::ui::i18n::visual_text(
                                                    display_language,
                                                    &relay.name,
                                                )
                                            })
                                            .unwrap_or_else(|| unavailable_output_label.clone());
                                        ui.group(|ui| {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(245, 158, 11),
                                                format!("{} Track mismatch: configured for {configured_name}", crate::ui::icons::WARNING),
                                            );
                                            if let Some(primary) = template.target.primary_relay_id() {
                                                if advertised_relays.iter().any(|relay| relay.id == primary)
                                                    && ui.button(format!("Relocate to {configured_name}")).clicked()
                                                {
                                                    relocate_effect_id = Some(template.id);
                                                }
                                            }
                                        });
                                        ui.add_space(8.0);
                                    }

                                    ui.group(|ui| {
                                        ui.strong(&hardware_target_label);
                                        ui.add_space(4.0);

                                        let mut selected_relay = current_relay_id;

                                        if advertised_relays.is_empty() {
                                            ui.label(&connect_output_label);
                                        } else {
                                          ui.horizontal(|ui| {
                                            ui.label(&target_output_label);
                                            egui::ComboBox::from_id_salt("relay_combo")
                                                .selected_text(
                                                    advertised_relays
                                                        .iter()
                                                        .find(|relay| relay.id == selected_relay)
                                                        .map(|relay| crate::ui::i18n::visual_text(display_language, &relay.name))
                                                        .unwrap_or_else(|| unavailable_project_output_label.clone())
                                                )
                                                .show_ui(ui, |ui| {
                                                    for relay in &advertised_relays {
                                                        ui.selectable_value(&mut selected_relay, relay.id, crate::ui::i18n::visual_text(display_language, &relay.name));
                                                    }
                                                });
                                          });
                                        }

                                        if selected_relay != current_relay_id {
                                            push_undo = true;
                                            isolate_instance = true;
                                            update_relay_to = Some(selected_relay);
                                            timeline_dirty = true;
                                        }
                                    });
                                    }

                                    ui.add_space(12.0);
                                    if ui.button(egui::RichText::new(format!("{} {delete_cue_label}", crate::ui::icons::X)).color(egui::Color32::from_rgb(231, 76, 60))).clicked() {
                                        delete_cue = true;
                                    }
                                }
                            }

                            if push_undo {
                                self.app.undo_stack.push(self.app.snapshot_timeline());
                            }

                            if isolate_instance {
                                self.app.isolate_template_for_instance(id);
                            }

                            if let Some(new_start) = update_start_to {
                                if let Some(inst) = self.app.timeline.instances.iter_mut().find(|i| i.id == id) {
                                    inst.start_time_ms = new_start;
                                }
                            }

                            if let Some(new_dur) = update_duration_to {
                                if let Some(inst) = self.app.timeline.instances.iter().find(|i| i.id == id) {
                                    let eff_id = inst.effect_id;
                                    if let Some(tmpl) = self.app.timeline.templates.iter_mut().find(|t| t.id == eff_id) {
                                        crate::app::update_effect_duration(tmpl, new_dur);
                                    }
                                }
                            }

                            if let Some(new_relay) = update_relay_to {
                                if let Some(inst) = self.app.timeline.instances.iter().find(|i| i.id == id) {
                                    let eff_id = inst.effect_id;
                                    if let Some(tmpl) = self.app.timeline.templates.iter_mut().find(|t| t.id == eff_id) {
                                        if tmpl.actions.is_empty() {
                                            tmpl.actions = crate::four_d::patterns::generate_constant(new_relay, true, tmpl.duration_ms);
                                        } else {
                                            for a in &mut tmpl.actions {
                                                a.relay_id = new_relay;
                                            }
                                        }
                                        tmpl.target = crate::four_d::models::HardwareTarget::Relay(new_relay);
                                    }
                                }
                            }

                            if let Some(eff_id) = relocate_effect_id {
                                self.app.relocate_effect_to_primary(eff_id);
                                ui.ctx().request_repaint();
                            }

                            if delete_cue {
                                self.app.undo_stack.push(self.app.snapshot_timeline());
                                self.app.timeline.instances.retain(|inst| inst.id != id);
                                self.app.selected_instance_ids.clear();
                                timeline_dirty = true;
                            }

                            if timeline_dirty {
                                self.app.sync_timeline_engine();
                                ui.ctx().request_repaint();
                            }
                        } else if selected_count > 1 {
                            ui.heading(self.app.tr("Bulk effect controls"));
                            ui.add_space(8.0);

                            ui.label(format!("Selected Cues: {}", selected_count));
                            ui.add_space(8.0);

                            let mut timeline_dirty = false;
                            let mut delete_all = false;

                            ui.group(|ui| {
                                ui.strong(self.app.tr("Bulk hardware target override"));
                                ui.add_space(8.0);

                                ui.horizontal_wrapped(|ui| {
                                    for relay in &advertised_relays {
                                        if ui.button(crate::ui::i18n::visual_text(display_language, &relay.name)).clicked() {
                                            // Apply the advertised target to all selected instances' templates.
                                            let selected_ids = &self.app.selected_instance_ids;
                                            for inst in &mut self.app.timeline.instances {
                                                    if selected_ids.contains(&inst.id) {
                                                        if let Some(template) = self.app.timeline.templates.iter_mut().find(|t| t.id == inst.effect_id) {
                                                            if template.controller_macro.is_some()
                                                                || template.controller_strip_effect.is_some()
                                                            {
                                                                continue;
                                                            }
                                                            template.actions = crate::four_d::patterns::generate_constant(relay.id, true, template.duration_ms);
                                                        template.target = crate::four_d::models::HardwareTarget::Relay(relay.id);
                                                    }
                                                }
                                            }
                                            timeline_dirty = true;
                                        }
                                    }
                                });
                            });

                            ui.add_space(8.0);

                            ui.group(|ui| {
                                ui.strong(self.app.tr("Bulk actions"));
                                ui.add_space(8.0);

                                if ui.button(egui::RichText::new(format!("× {delete_all_label}")).color(egui::Color32::from_rgb(231, 76, 60))).clicked() {
                                    delete_all = true;
                                }
                            });

                            if delete_all {
                                let selected_ids = &self.app.selected_instance_ids;
                                self.app.timeline.instances.retain(|inst| !selected_ids.contains(&inst.id));
                                self.app.selected_instance_ids.clear();
                                timeline_dirty = true;
                            }

                            if timeline_dirty {
                                self.app.sync_timeline_engine();
                            }
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.label(egui::RichText::new(self.app.tr("No Cue Selected")).weak().size(14.0));
                            });
                        }
                    }
                    PealayerTab::EffectsLibrary => {
                        ui.horizontal(|ui| {
                            ui.heading(self.app.tr("Effects Library"));
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::PENCIL_SIMPLE,
                                    self.app.tr("Manage effects")
                                ))
                                .clicked()
                            {
                                self.app.show_effect_library_editor = true;
                            }
                        });
                        ui.add_space(4.0);

                        // 1. Instant search edit field
                        ui.horizontal(|ui| {
                            ui.label(self.app.tr("Search"));
                            let search_hint = self.app.tr("Search effects...");
                            let res = ui.add(
                                egui::TextEdit::singleline(&mut self.app.effects_search_query)
                                    .hint_text(search_hint)
                            );
                            if res.changed() {
                                // Request repaint to filter instantly
                                ui.ctx().request_repaint();
                            }
                        });

                        ui.add_space(8.0);

                        // Filter presets based on query
                        let query = self.app.effects_search_query.trim().to_lowercase();
                        let advertised_presets = self.app.advertised_effect_presets();
                        let mut categorized: std::collections::BTreeMap<String, Vec<&crate::app::EffectPreset>> = std::collections::BTreeMap::new();

                        for preset in &advertised_presets {
                            if query.is_empty() || preset.effect.name.to_lowercase().contains(&query) {
                                categorized.entry(preset.category.clone()).or_default().push(preset);
                            }
                        }

                        let force_open = !query.is_empty();

                        if categorized.is_empty() {
                            ui.centered_and_justified(|ui| {
                                let source = self
                                    .app
                                    .connected_board_display_name()
                                    .unwrap_or_else(|| self.app.tr("connected hardware"));
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} {source}.",
                                        self.app.tr("No effects are advertised by")
                                    ))
                                    .weak()
                                    .size(12.0),
                                );
                            });
                        } else {
                            egui::ScrollArea::vertical()
                                .id_salt("effects_scroll")
                                .show(ui, |ui| {
                                    for (category, presets) in categorized {
                                        let group_id = ui.make_persistent_id(("effect-group", &category));
                                        let mut open = ui.data_mut(|data| {
                                            data.get_persisted::<bool>(group_id).unwrap_or(true)
                                        });
                                        if force_open {
                                            open = true;
                                        }
                                        let displayed_category = crate::ui::i18n::visual_text(
                                            display_language,
                                            &category,
                                        );
                                        let group_header = egui::Frame::new()
                                            .fill(ui.visuals().widgets.inactive.weak_bg_fill)
                                            .stroke(egui::Stroke::new(
                                                1.0_f32,
                                                ui.visuals().widgets.noninteractive.bg_stroke.color,
                                            ))
                                            .corner_radius(7.0)
                                            .inner_margin(egui::Margin::symmetric(9, 6))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(if open {
                                                        crate::ui::icons::CARET_DOWN
                                                    } else {
                                                        crate::ui::icons::CARET_RIGHT
                                                    });
                                                    ui.label(crate::ui::icons::FOLDER_OPEN);
                                                    ui.label(
                                                        egui::RichText::new(displayed_category)
                                                            .strong(),
                                                    );
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            egui::Frame::new()
                                                                .fill(
                                                                    ui.visuals()
                                                                        .selection
                                                                        .bg_fill
                                                                        .gamma_multiply(0.22),
                                                                )
                                                                .corner_radius(9.0)
                                                                .inner_margin(egui::Margin::symmetric(7, 2))
                                                                .show(ui, |ui| {
                                                                    ui.label(
                                                                        egui::RichText::new(
                                                                            presets.len().to_string(),
                                                                        )
                                                                        .small()
                                                                        .strong(),
                                                                    );
                                                                });
                                                        },
                                                    );
                                                });
                                            });
                                        let group_response = group_header.response.interact(egui::Sense::click());
                                        if group_response.clicked() {
                                            open = !open;
                                        }
                                        group_response.context_menu(|ui| {
                                            ui.strong(crate::ui::i18n::visual_text(
                                                display_language,
                                                &category,
                                            ));
                                            ui.separator();
                                            if ui
                                                .button(format!(
                                                    "{} {}",
                                                    crate::ui::icons::SPARKLE,
                                                    self.app.tr("New effect in this group")
                                                ))
                                                .clicked()
                                            {
                                                self.app.effect_library_selection = None;
                                                self.app.effect_library_draft = crate::app::ControllerEffectDraft {
                                                    category: category.clone(),
                                                    ..Default::default()
                                                };
                                                self.app.show_effect_library_editor = true;
                                                ui.close();
                                            }
                                            if ui
                                                .button(format!(
                                                    "{} {}",
                                                    crate::ui::icons::PENCIL_SIMPLE,
                                                    self.app.tr("Manage effects")
                                                ))
                                                .clicked()
                                            {
                                                self.app.show_effect_library_editor = true;
                                                ui.close();
                                            }
                                            ui.separator();
                                            let collapse_label = if open {
                                                self.app.tr("Collapse group")
                                            } else {
                                                self.app.tr("Expand group")
                                            };
                                            if ui.button(collapse_label).clicked() {
                                                open = !open;
                                                ui.close();
                                            }
                                        });
                                        ui.data_mut(|data| data.insert_persisted(group_id, open));

                                        if open {
                                            ui.add_space(5.0);
                                            for preset in presets {
                                                let item_id = ui.make_persistent_id((
                                                    "effect-card",
                                                    preset.effect.id,
                                                ));
                                                let payload = EffectDragPayload {
                                                    name: preset.effect.name.clone(),
                                                    icon: preset.effect.icon.clone(),
                                                    duration_ms: preset.effect.duration_ms,
                                                    target: preset.effect.target,
                                                    actions: preset.effect.actions.clone(),
                                                    controller_macro: preset.effect.controller_macro.clone(),
                                                    controller_strip_effect: preset.effect.controller_strip_effect.clone(),
                                                };
                                                let target_label = if preset
                                                    .effect
                                                    .controller_strip_effect
                                                    .is_some()
                                                {
                                                    self.app.tr("Host stream")
                                                } else if preset.effect.controller_macro.is_some() {
                                                    self.app.tr("Timed sequence")
                                                } else if let crate::four_d::models::HardwareTarget::Relay(id) =
                                                    preset.effect.target
                                                {
                                                    format!("{} {id}", self.app.tr("Relay"))
                                                } else {
                                                    self.app.tr("Effect")
                                                };
                                                let displayed_effect_name =
                                                    crate::ui::i18n::visual_text(
                                                        display_language,
                                                        &preset.effect.name,
                                                    );
                                                let source = preset.source;
                                                let response = effect_drag_source(
                                                    ui,
                                                    item_id,
                                                    payload,
                                                    |ui| {
                                                        egui::Frame::group(ui.style())
                                                            .inner_margin(egui::Margin::symmetric(9, 7))
                                                            .corner_radius(8.0)
                                                            .show(ui, |ui| {
                                                                ui.set_min_width(ui.available_width());
                                                                ui.set_max_width(ui.available_width());
                                                                ui.horizontal(|ui| {
                                                                    ui.label(
                                                                        egui::RichText::new(
                                                                            crate::ui::icons::SPARKLE,
                                                                        )
                                                                        .size(16.0),
                                                                    );
                                                                    ui.add(
                                                                        egui::Label::new(
                                                                            egui::RichText::new(
                                                                                &displayed_effect_name,
                                                                            )
                                                                            .strong(),
                                                                        )
                                                                        .truncate(),
                                                                    );
                                                                });
                                                                ui.add_space(5.0);
                                                                ui.horizontal(|ui| {
                                                                    for text in [
                                                                        target_label.clone(),
                                                                        format!(
                                                                            "{} ms",
                                                                            preset.effect.duration_ms
                                                                        ),
                                                                    ] {
                                                                        egui::Frame::new()
                                                                            .fill(
                                                                                ui.visuals()
                                                                                    .selection
                                                                                    .bg_fill
                                                                                    .gamma_multiply(0.18),
                                                                            )
                                                                            .corner_radius(8.0)
                                                                            .inner_margin(
                                                                                egui::Margin::symmetric(7, 2),
                                                                            )
                                                                            .show(ui, |ui| {
                                                                                ui.label(
                                                                                    egui::RichText::new(text)
                                                                                        .small(),
                                                                                );
                                                                            });
                                                                    }
                                                                });
                                                            })
                                                    },
                                                );
                                                if response.response.dragged() {
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                                }
                                                response.response.context_menu(|ui| {
                                                    ui.strong(&displayed_effect_name);
                                                    ui.separator();
                                                    if ui.button(format!("{} {}", crate::ui::icons::PENCIL_SIMPLE, self.app.tr("Properties and edit"))).clicked() {
                                                        if let Some(capabilities) = self.app.advertised_hardware() {
                                                            match source {
                                                                crate::app::EffectPresetSource::ControllerMacro(id) => {
                                                                    if let Some(effect) = capabilities.macros.iter().find(|effect| effect.id == id) {
                                                                        crate::ui::effects_library::select_sequence(self.app, effect);
                                                                    }
                                                                }
                                                                crate::app::EffectPresetSource::ControllerStrip => {
                                                                    if let Some(id) = preset.effect.controller_strip_effect.as_ref().map(|value| value.id.as_str())
                                                                        && let Some(effect) = capabilities.strip_effects.iter().find(|effect| effect.id == id)
                                                                    {
                                                                        crate::ui::effects_library::select_strip(self.app, effect);
                                                                    }
                                                                }
                                                            }
                                                            self.app.show_effect_library_editor = true;
                                                        }
                                                        ui.close();
                                                    }
                                                    let reference = match source {
                                                        crate::app::EffectPresetSource::ControllerMacro(id) => format!("sequence:{id}"),
                                                        crate::app::EffectPresetSource::ControllerStrip => format!("strip:{}", preset.effect.controller_strip_effect.as_ref().map(|value| value.id.as_str()).unwrap_or_default()),
                                                    };
                                                    if ui.button(format!("{} {}", crate::ui::icons::PLAY, self.app.tr("Run now"))).clicked() {
                                                        if let Err(error) = self.app.play_controller_effect(&reference) {
                                                            self.app.set_osd(error);
                                                        }
                                                        ui.close();
                                                    }
                                                });
                                                ui.add_space(5.0);
                                            }
                                        }
                                        ui.add_space(7.0);
                                    }
                                });
                        }
                    }
                    PealayerTab::HardwareMonitor => {
                        ui.heading(self.app.tr("Hardware Monitor Dashboard"));
                        ui.add_space(8.0);

                        if self.app.estop_active {
                            ui.horizontal(|ui| {
                                let time = ui.input(|i| i.time);
                                let is_flash = (time * 4.0).sin() > 0.0;
                                let color = if is_flash {
                                    egui::Color32::from_rgb(231, 76, 60) // Red
                                } else {
                                    egui::Color32::from_rgb(241, 196, 15) // Yellow
                                };
                                ui.colored_label(
                                    color,
                                    egui::RichText::new(format!("{} {estop_banner_label}", crate::ui::icons::WARNING))
                                        .strong()
                                        .size(13.0)
                                );
                            });
                            ui.add_space(8.0);
                        }

                        let capabilities = self.app.advertised_hardware();
                        if !self.app.is_connected {
                            ui.label(egui::RichText::new(
                                self.app.tr("Connect to PCController to discover live board controls.")
                            ).weak());
                        } else if crate::four_d::controller::is_controller_endpoint(&self.app.serial_port)
                            && capabilities.is_none()
                        {
                            if self.app.connection_notice.is_some() {
                                ui.horizontal_wrapped(|ui| {
                                    ui.colored_label(
                                        ui.visuals().warn_fg_color,
                                        crate::ui::icons::WARNING,
                                    );
                                    ui.label(
                                        egui::RichText::new(
                                            self.app.hardware_unavailable_detail(None),
                                        )
                                        .weak(),
                                    );
                                });
                            } else {
                                ui.label(egui::RichText::new(
                                    self.app.tr("PCController is connected; requesting the board capability catalog…")
                                ).weak());
                            }
                        } else if capabilities
                            .as_ref()
                            .is_some_and(|capabilities| !capabilities.board_connected)
                        {
                            ui.horizontal_wrapped(|ui| {
                                ui.colored_label(
                                    ui.visuals().warn_fg_color,
                                    crate::ui::icons::WARNING,
                                );
                                ui.label(
                                    egui::RichText::new(
                                        self.app.hardware_unavailable_detail(capabilities.as_ref()),
                                    )
                                    .weak(),
                                );
                            });
                        }

                        if let Some(capabilities) = capabilities
                            .filter(|capabilities| capabilities.board_connected)
                        {
                            let board_card = egui::Frame::group(ui.style())
                                .inner_margin(egui::Margin::symmetric(14, 12))
                                .corner_radius(10.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(crate::ui::icons::PLUG)
                                                .size(22.0),
                                        );
                                        ui.vertical(|ui| {
                                            let profile_name = capabilities
                                                .board_profile
                                                .as_ref()
                                                .map(|profile| humanize_machine_label(&profile.key))
                                                .filter(|name| !name.is_empty());
                                            let board_name = crate::ui::i18n::visual_text(
                                                display_language,
                                                &capabilities.board_name,
                                            );
                                            let name_response = ui.add(
                                                egui::Label::new(
                                                egui::RichText::new(if board_name.trim().is_empty() {
                                                    profile_name.clone().unwrap_or_else(|| self.app.tr("Connected board"))
                                                } else {
                                                    board_name
                                                })
                                                    .heading()
                                                    .strong(),
                                                )
                                                .sense(egui::Sense::click()),
                                            );
                                            if name_response
                                                .on_hover_text(self.app.tr("Rename board"))
                                                .clicked()
                                            {
                                                self.app.board_name_draft = capabilities.board_name.clone();
                                                self.app.board_info_tab = 0;
                                                self.app.show_board_info_dialog = true;
                                            }
                                            if let Some(profile_name) = profile_name {
                                                ui.label(
                                                    egui::RichText::new(profile_name)
                                                    .weak(),
                                                );
                                            }
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button(crate::ui::icons::INFO)
                                                    .on_hover_text(self.app.tr("Board information"))
                                                    .clicked()
                                                {
                                                    self.app.board_name_draft = capabilities.board_name.clone();
                                                    self.app.show_board_info_dialog = true;
                                                }
                                                let (rect, _) = ui.allocate_exact_size(
                                                    egui::vec2(14.0, 14.0),
                                                    egui::Sense::hover(),
                                                );
                                                ui.painter().circle_filled(
                                                    rect.center(),
                                                    5.0,
                                                    egui::Color32::from_rgb(52, 211, 153),
                                                );
                                                ui.label(self.app.tr("Connected"));
                                            },
                                        );
                                    });
                                });
                            board_card.response.context_menu(|ui| {
                                if ui
                                    .button(format!(
                                        "{} {}",
                                        crate::ui::icons::INFO,
                                        self.app.tr("Board information")
                                    ))
                                    .clicked()
                                {
                                    self.app.board_name_draft = capabilities.board_name.clone();
                                    self.app.show_board_info_dialog = true;
                                    ui.close();
                                }
                                if ui
                                    .button(format!(
                                        "{} {}",
                                        crate::ui::icons::PENCIL_SIMPLE,
                                        self.app.tr("Rename board")
                                    ))
                                    .clicked()
                                {
                                    self.app.board_name_draft = capabilities.board_name.clone();
                                    self.app.board_info_tab = 0;
                                    self.app.show_board_info_dialog = true;
                                    ui.close();
                                }
                            });

                            let can_record = capabilities.board_profile.as_ref().is_some_and(|profile| {
                                profile.attached && profile.configured
                            }) && !capabilities.controls.is_empty();
                            if can_record {
                                let record_title = self.app.tr("Record effect");
                                let name_label = self.app.tr("Name");
                                let name_hint = self.app.tr("Seat motion take");
                                let start_label = self.app.tr("Start recording");
                                let start_help = self.app.tr(
                                    "Anchor at the current video time and capture board-applied actions from every PCController surface.",
                                );
                                let refresh_label = self.app.tr("Refresh status");
                                let save_label = self.app.tr("Save and place");
                                let discard_label = self.app.tr("Discard");
                                let anchor_label = self.app.tr("Timeline anchor:");
                                let capture_engine_label = self.app.tr("Capture engine");
                                let host_engine_label = self.app.tr("PCController host");
                                let board_engine_label = self.app.tr("Board RAM");
                                let host_engine_help = self.app.tr("Captures relay, motion, MOSFET, display, buzzer, RF, and other coordinator actions.");
                                let board_engine_help = self.app.tr("Captures the board's bounded live relay snapshots, then imports them into PCController when saved.");
                                let record_open = self.app.hardware_effect_authoring.active
                                    || self.app.hardware_effect_authoring.pending_operation.is_some();
                                ui.add_space(8.0);
                                if crate::ui::icons::disclosure_header(
                                    ui,
                                    "hardware_effect_recording_panel_disclosure",
                                    &record_title,
                                    record_open,
                                ) {
                                    ui.indent("hardware_effect_recording_panel_content", |ui| {
                                        egui::Grid::new("hardware_effect_recording_name")
                                            .num_columns(2)
                                            .spacing([12.0, 8.0])
                                            .show(ui, |ui| {
                                                ui.label(&name_label);
                                                ui.add_enabled(
                                                    !self.app.hardware_effect_authoring.active
                                                        && self.app.hardware_effect_authoring.pending_operation.is_none(),
                                                    egui::TextEdit::singleline(&mut self.app.hardware_effect_authoring.name)
                                                        .desired_width(ui.available_width().min(280.0))
                                                        .hint_text(&name_hint),
                                                );
                                                ui.end_row();
                                                ui.label(&capture_engine_label);
                                                ui.horizontal(|ui| {
                                                    ui.selectable_value(
                                                        &mut self.app.hardware_effect_authoring.record_on_board,
                                                        false,
                                                        &host_engine_label,
                                                    ).on_hover_text(&host_engine_help);
                                                    ui.selectable_value(
                                                        &mut self.app.hardware_effect_authoring.record_on_board,
                                                        true,
                                                        &board_engine_label,
                                                    ).on_hover_text(&board_engine_help);
                                                });
                                                ui.end_row();
                                            });
                                        let pending = self.app.hardware_effect_authoring.pending_operation.is_some();
                                        ui.add_space(6.0);
                                        ui.columns(2, |uis| {
                                            if uis[0].add_enabled(
                                                !pending
                                                    && !self.app.hardware_effect_authoring.active
                                                    && !self.app.hardware_effect_authoring.name.trim().is_empty(),
                                                egui::Button::new(&start_label).truncate(),
                                            ).on_hover_text(&start_help).clicked()
                                            {
                                                if let Err(error) = self.app.start_hardware_effect_recording() {
                                                    self.app.set_osd(error);
                                                }
                                            }
                                            if uis[1].add_enabled(
                                                !pending && self.app.hardware_effect_authoring.active,
                                                egui::Button::new(&refresh_label).truncate(),
                                            ).clicked()
                                            {
                                                if let Err(error) = self.app.refresh_hardware_effect_recording() {
                                                    self.app.set_osd(error);
                                                }
                                            }
                                        });
                                        ui.columns(2, |uis| {
                                            if uis[0].add_enabled(
                                                !pending && self.app.hardware_effect_authoring.active,
                                                egui::Button::new(&save_label).truncate(),
                                            ).on_hover_text(format!(
                                                "{} {:.3}s", anchor_label,
                                                self.app.hardware_effect_authoring.anchor_ms as f64 / 1_000.0
                                            )).clicked()
                                            {
                                                if let Err(error) = self.app.save_hardware_effect_recording() {
                                                    self.app.set_osd(error);
                                                }
                                            }
                                            if uis[1].add_enabled(
                                                !pending && self.app.hardware_effect_authoring.active,
                                                egui::Button::new(&discard_label).truncate(),
                                            ).clicked()
                                            {
                                                if let Err(error) = self.app.discard_hardware_effect_recording() {
                                                    self.app.set_osd(error);
                                                }
                                            }
                                        });
                                        if !self.app.hardware_effect_authoring.status.is_empty() {
                                            ui.label(egui::RichText::new(
                                                self.app.hardware_effect_authoring.status.clone()
                                            ).weak().monospace());
                                        }
                                    });
                                }
                            }

                            if !capabilities.strip_effects.is_empty() {
                                let strip_title = self.app.tr("Host-rendered lighting effects");
                                let stop_label = self.app.tr("Stop preview");
                                let preview_label = self.app.tr("Preview");
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(&strip_title).strong(),
                                    );
                                    if ui
                                        .button(format!(
                                            "{} {}",
                                            crate::ui::icons::PENCIL_SIMPLE,
                                            self.app.tr("Manage effects")
                                        ))
                                        .clicked()
                                    {
                                        self.app.show_effect_library_editor = true;
                                    }
                                    if ui
                                        .add_enabled(
                                            self.app.hardware_effect_authoring.pending_operation.is_none(),
                                            egui::Button::new(&stop_label),
                                        )
                                        .clicked()
                                    {
                                        if let Err(error) = self.app.stop_strip_preview() {
                                            self.app.set_osd(error);
                                        }
                                    }
                                });
                                for strip_effect in &capabilities.strip_effects {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(
                                            egui::RichText::new(crate::ui::i18n::visual_text(
                                                display_language,
                                                &strip_effect.name,
                                            ))
                                            .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:.1} s",
                                                strip_effect.default_duration_ms.unwrap_or_default() as f64 / 1_000.0
                                            ))
                                            .weak(),
                                        );
                                        if ui
                                            .add_enabled(
                                                self.app.hardware_effect_authoring.pending_operation.is_none(),
                                                egui::Button::new(&preview_label),
                                            )
                                            .on_hover_text(format!("{} ms", strip_effect.default_duration_ms.unwrap_or_default()))
                                            .clicked()
                                        {
                                            if let Err(error) =
                                                self.app.preview_strip_effect(&strip_effect.id)
                                            {
                                                self.app.set_osd(error);
                                            }
                                        }
                                    });
                                }
                            }

                            let semantic_controls = capabilities
                                .board_profile
                                .as_ref()
                                .filter(|profile| profile.attached && profile.configured)
                                .map(|_| {
                                    capabilities.controls.iter()
                                        .filter(|control| {
                                            relay_id_from_control_key(&control.key).is_none()
                                                && !is_pwm_control(control)
                                        })
                                        .cloned()
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();
                            if !semantic_controls.is_empty() {
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new(self.app.tr("Semantic board controls")).strong());
                                ui.add_space(6.0);
                                draw_control_card_grid(self.app, ui, &capabilities, &semantic_controls);
                            }

                            if !capabilities.relays.is_empty() {
                                let relay_controls = capabilities.relays.iter()
                                    .map(|relay| {
                                        capabilities.controls.iter()
                                            .find(|control| control.key == relay.key)
                                            .cloned()
                                            .unwrap_or_else(|| crate::four_d::controller::HardwareControl {
                                                key: relay.key.clone(),
                                                kind: "relay".to_string(),
                                                name: relay.name.clone(),
                                                default_name: relay.name.clone(),
                                                control: relay.control.clone(),
                                                group: relay.role.clone(),
                                                ..Default::default()
                                            })
                                    })
                                    .collect::<Vec<_>>();
                                ui.label(egui::RichText::new(self.app.tr("Relay outputs")).strong());
                                ui.add_space(6.0);
                                draw_control_card_grid(self.app, ui, &capabilities, &relay_controls);
                            }

                            if !capabilities.pwm_channels.is_empty() {
                                let pwm_controls = capabilities.pwm_channels.iter()
                                    .map(|channel| {
                                        capabilities.controls.iter()
                                            .find(|control| control.key == channel.key)
                                            .cloned()
                                            .unwrap_or_else(|| crate::four_d::controller::HardwareControl {
                                                key: channel.key.clone(),
                                                kind: "mosfet".to_string(),
                                                name: channel.name.clone(),
                                                default_name: channel.name.clone(),
                                                control: channel.control.clone(),
                                                group: channel.role.clone(),
                                                ..Default::default()
                                            })
                                    })
                                    .collect::<Vec<_>>();
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new(self.app.tr("PWM / MOSFET outputs")).strong());
                                ui.add_space(6.0);
                                draw_control_card_grid(self.app, ui, &capabilities, &pwm_controls);
                            }

                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(self.app.tr("Advertised capabilities")).strong());
                            ui.horizontal_wrapped(|ui| {
                                if capabilities.supports_addressable_led {
                                    ui.label(format!("{} Addressable RGB strip", crate::ui::icons::LIGHTBULB));
                                }
                                if capabilities.supports_rf_transmit {
                                    ui.label(format!("{} RF transmitter", crate::ui::icons::RADIO));
                                }
                                if capabilities.supports_segment_display {
                                    ui.label(format!("{} Segment display", crate::ui::icons::GAUGE));
                                }
                                if capabilities.supports_lcd_display {
                                    ui.label(format!("{} LCD text display", crate::ui::icons::MONITOR_PLAY));
                                }
                            });

                            if capabilities.supports_segment_display || capabilities.supports_lcd_display {
                                let text_id = ui.make_persistent_id("hardware_display_text");
                                let mut text = ui.data_mut(|data| data.get_temp::<String>(text_id).unwrap_or_default());
                                ui.horizontal(|ui| {
                                    ui.label(self.app.tr("Display text"));
                                    if ui.text_edit_singleline(&mut text).changed() {
                                        ui.data_mut(|data| data.insert_temp(text_id, text.clone()));
                                    }
                                    if ui.add_enabled(!text.trim().is_empty(), egui::Button::new(format!("{} Send", crate::ui::icons::PAPER_PLANE_TILT))).clicked() {
                                        let target = if capabilities.supports_segment_display && capabilities.supports_lcd_display {
                                            "both"
                                        } else if capabilities.supports_lcd_display {
                                            "lcd"
                                        } else {
                                            "segments"
                                        };
                                        let _ = self.app.engine_handle.sender.send(
                                            crate::four_d::engine::EngineMessage::ControllerCall {
                                                method: "controller.display.send".to_string(),
                                                params: serde_json::json!({"target": target, "text": text, "duration_ms": 5000}),
                                            },
                                        );
                                    }
                                });
                            }

                            if capabilities.supports_rf_transmit {
                                let rf_code_id = ui.make_persistent_id("hardware_rf_code");
                                let mut code = ui.data_mut(|data| data.get_temp::<String>(rf_code_id).unwrap_or_default());
                                ui.horizontal(|ui| {
                                    ui.label(self.app.tr("RF code"));
                                    if ui.text_edit_singleline(&mut code).changed() {
                                        ui.data_mut(|data| data.insert_temp(rf_code_id, code.clone()));
                                    }
                                    let trimmed = code.trim();
                                    let parsed = trimmed
                                        .strip_prefix("0x")
                                        .or_else(|| trimmed.strip_prefix("0X"))
                                        .map(|hex| u32::from_str_radix(hex, 16).ok())
                                        .unwrap_or_else(|| trimmed.parse::<u32>().ok());
                                    if ui.add_enabled(parsed.is_some(), egui::Button::new(format!("{} Send 24-bit", crate::ui::icons::RADIO))).clicked() {
                                        let _ = self.app.engine_handle.sender.send(
                                            crate::four_d::engine::EngineMessage::ControllerCall {
                                                method: "controller.rf.transmit".to_string(),
                                                params: serde_json::json!({"code": parsed, "bits": 24, "protocol": 1}),
                                            },
                                        );
                                    }
                                });
                            }

                            if !capabilities.warnings.is_empty() {
                                ui.add_space(8.0);
                                for warning in &capabilities.warnings {
                                    ui.colored_label(
                                        ui.visuals().warn_fg_color,
                                        format!("{} {} — {}", crate::ui::icons::WARNING, warning.code, warning.message),
                                    );
                                }
                            }

                            if !capabilities.macros.is_empty() {
                                ui.add_space(8.0);
                                let board_name = self
                                    .app
                                    .connected_board_display_name()
                                    .unwrap_or_else(|| self.app.tr("Connected board"));
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{board_name} {}",
                                        self.app.tr("effect catalog")
                                    ))
                                    .strong(),
                                );
                                for hardware_macro in &capabilities.macros {
                                    ui.horizontal(|ui| {
                                        ui.label(crate::ui::i18n::visual_text(display_language, &hardware_macro.name));
                                        if ui.button(self.app.tr("Play")).clicked() {
                                            let command = format!("effect play sequence:{} {}", hardware_macro.id, hardware_macro.mode);
                                            let _ = self.app.engine_handle.sender.send(
                                                crate::four_d::engine::EngineMessage::ControllerCall {
                                                    method: "controller.command.execute".to_string(),
                                                    params: serde_json::json!({"command": command}),
                                                },
                                            );
                                        }
                                    });
                                }
                            }
                        }

                        // mpv's render callback requests frames while video is
                        // advancing. An unconditional repaint here turned the
                        // no-media state (which starts unpaused) into an
                        // unlimited GPU render loop when V-Sync was disabled.
                    }
                    PealayerTab::Timeline => {
                        let timeline_rows = timeline_track_rows(self.app);
                        if timeline_rows.is_empty() && self.app.timeline.analog_tracks.is_empty() {
                            let message = if self.app.advertised_hardware().is_none() {
                                self.app.tr("Open media or connect PCController to populate the timeline.")
                            } else {
                                self.app.tr("No media or advertised hardware tracks are available.")
                            };
                            ui.label(message);
                        }
                        ui.horizontal(|ui| {
                            // 1. Left column: Fixed Track Headers
                            ui.vertical(|ui| {
                                ui.set_width(250.0);

                                // 26px spacer to align with the right-side ruler
                                let (header_rect, _) = ui.allocate_exact_size(egui::vec2(250.0, 26.0), egui::Sense::hover());
                                ui.painter().rect_filled(header_rect, 0.0, ui.visuals().panel_fill);
                                ui.painter().line_segment(
                                    [egui::pos2(header_rect.min.x, header_rect.max.y), egui::pos2(header_rect.max.x, header_rect.max.y)],
                                    ui.visuals().widgets.noninteractive.bg_stroke,
                                );
                                ui.painter().text(
                                    header_rect.left_center() + egui::vec2(6.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    self.app.tr("Tracks"),
                                    egui::FontId::proportional(11.0),
                                    ui.visuals().weak_text_color(),
                                );

                                for track_row in &timeline_rows {
                                    let (rect, _response) = ui.allocate_exact_size(egui::vec2(250.0, 32.0), egui::Sense::hover());
                                    // Draw background with dark Premiere aesthetics
                                    ui.painter().rect_filled(rect, 0.0, ui.visuals().faint_bg_color);
                                    ui.painter().rect_stroke(rect, 0.0, ui.visuals().widgets.noninteractive.bg_stroke, egui::StrokeKind::Inside);

                                    // Create a nested UI at this rect to place buttons
                                    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(*ui.layout()));
                                    child_ui.horizontal(|ui| {
                                        ui.add_space(6.0);
                                        // Limit the track name label width
                                        ui.allocate_ui(egui::vec2(85.0, 20.0), |ui| {
                                            ui.label(egui::RichText::new(&track_row.name).size(11.0).strong());
                                        });

                                        if let TimelineTrackKind::Relay(relay_id) = track_row.kind {
                                            // Mute button (M)
                                            let muted = self.app.track_muted.contains(&relay_id);
                                            let m_btn = ui.selectable_label(muted, egui::RichText::new("M").strong().size(10.0))
                                                .on_hover_text(&relay_mute_help);
                                            if m_btn.clicked() {
                                                if muted {
                                                    self.app.track_muted.remove(&relay_id);
                                                } else {
                                                    self.app.track_muted.insert(relay_id);
                                                }
                                                let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                                let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                            }

                                            // Solo button (S)
                                            let soloed = self.app.track_soloed.contains(&relay_id);
                                            let s_btn = ui.selectable_label(soloed, egui::RichText::new("S").strong().size(10.0))
                                                .on_hover_text(&relay_solo_help);
                                            if s_btn.clicked() {
                                                if soloed {
                                                    self.app.track_soloed.remove(&relay_id);
                                                } else {
                                                    self.app.track_soloed.insert(relay_id);
                                                }
                                                let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                                let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                            }

                                            // Lock button (L)
                                            let locked = self.app.track_locked.contains(&relay_id);
                                            let l_btn = ui.selectable_label(locked, egui::RichText::new("L").strong().size(10.0))
                                                .on_hover_text(&lock_help);
                                            if l_btn.clicked() {
                                                if locked {
                                                    self.app.track_locked.remove(&relay_id);
                                                } else {
                                                    self.app.track_locked.insert(relay_id);
                                                }
                                            }
                                        }
                                    });
                                }

                                // Analog Curve Track Headers
                                let mut analog_tracks_changed = false;
                                for track in self.app.timeline.analog_tracks.iter_mut() {
                                    let (rect, _response) = ui.allocate_exact_size(egui::vec2(250.0, 40.0), egui::Sense::hover());
                                    ui.painter().rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
                                    ui.painter().rect_stroke(rect, 0.0, ui.visuals().widgets.noninteractive.bg_stroke, egui::StrokeKind::Inside);

                                    // Amplitude Y-axis tick labels on track header right margin
                                    let painter = ui.painter();
                                    painter.text(
                                        egui::pos2(rect.max.x - 3.0, rect.min.y + 5.0),
                                        egui::Align2::RIGHT_TOP,
                                        "100%",
                                        egui::FontId::monospace(7.5),
                                        egui::Color32::from_rgb(90, 90, 90),
                                    );
                                    painter.text(
                                        egui::pos2(rect.max.x - 3.0, rect.center().y),
                                        egui::Align2::RIGHT_CENTER,
                                        "50%",
                                        egui::FontId::monospace(7.5),
                                        egui::Color32::from_rgb(70, 70, 70),
                                    );
                                    painter.text(
                                        egui::pos2(rect.max.x - 3.0, rect.max.y - 5.0),
                                        egui::Align2::RIGHT_BOTTOM,
                                        "0%",
                                        egui::FontId::monospace(7.5),
                                        egui::Color32::from_rgb(90, 90, 90),
                                    );

                                    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(*ui.layout()));
                                    child_ui.horizontal(|ui| {
                                        ui.add_space(6.0);
                                        ui.allocate_ui(egui::vec2(85.0, 20.0), |ui| {
                                            let track_name = crate::ui::i18n::visual_text(display_language, &track.name);
                                            ui.label(egui::RichText::new(format!("P{}: {}", track.channel, track_name)).size(10.5).strong().color(egui::Color32::from_rgb(0, 220, 255)))
                                                .on_hover_text(format!("{analog_track_label}: {}\n{port_channel_label}: P{}", track_name, track.channel));
                                        });

                                        let m_btn = ui.selectable_label(track.muted, egui::RichText::new("M").strong().size(10.0))
                                            .on_hover_text(&actuator_mute_help);
                                        if m_btn.clicked() {
                                            track.muted = !track.muted;
                                            analog_tracks_changed = true;
                                        }

                                        // Record Arm Button [●]
                                        let arm_color = if track.armed {
                                            egui::Color32::from_rgb(255, 60, 60)
                                        } else {
                                            egui::Color32::from_rgb(120, 120, 120)
                                        };
                                        let arm_btn = ui.selectable_label(
                                            track.armed,
                                            egui::RichText::new("●").size(12.0).color(arm_color),
                                        )
                                        .on_hover_text(&record_arm_help);
                                        if arm_btn.clicked() {
                                            track.armed = !track.armed;
                                            analog_tracks_changed = true;

                                            if !track.armed && self.app.recording_session.sample_count(track.id) > 0 {
                                                self.app.recording_session.commit_to_track(
                                                    track,
                                                    0.015,
                                                    crate::four_d::curve::Interpolation::Smooth,
                                                );
                                            }
                                        }

                                        if track.armed {
                                            let mut val = self.app.input_capture.current_throttle;
                                            let slider = egui::Slider::new(&mut val, 0.0..=1.0)
                                                .show_value(false)
                                                .text("");
                                            if ui.add_sized([65.0, 16.0], slider)
                                                .on_hover_text(&live_fader_help)
                                                .changed() {
                                                self.app.input_capture.set_throttle(val);
                                                let byte_val = (val * 255.0).round() as u8;
                                                let _ = self.app.engine_handle.sender.send(
                                                    crate::four_d::engine::EngineMessage::LiveActuatorOverride {
                                                        channel: track.channel,
                                                        value: byte_val,
                                                    },
                                                );
                                            }
                                        }

                                        let add_btn = ui.button(egui::RichText::new("+").size(10.0))
                                            .on_hover_text(&add_keyframe_help);
                                        if add_btn.clicked() {
                                            let cur_ms = (self.app.playback_time * 1000.0) as u64;
                                            let cur_val = track.evaluate(cur_ms);
                                            track.add_keyframe(crate::four_d::curve::Keyframe::new(cur_ms, cur_val, crate::four_d::curve::Interpolation::Linear));
                                            analog_tracks_changed = true;
                                        }
                                    });
                                }
                                if analog_tracks_changed {
                                    let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.timeline.analog_tracks.clone()));
                                }
                            });

                            // 2. Right column: Scrollable Timeline Grid
                            let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
                            let zoom = self.app.timeline_zoom;
                            let px_per_ms = zoom / 1000.0;
                            let total_seconds = if self.app.duration > 0.0 { self.app.duration } else { 60.0 };
                            let total_width = (total_seconds * zoom as f64) as f32;
                            let num_analog = self.app.timeline.analog_tracks.len();
                            let track_area_height = timeline_rows.len() as f32 * 32.0;
                            let total_height = 26.0 + track_area_height + (num_analog as f32 * 40.0);

                            // Define dropping target zone
                            let drop_res = ui.dnd_drop_zone::<EffectDragPayload, _>(egui::Frame::NONE, |ui| {
                                egui::ScrollArea::both()
                                    .id_salt("timeline_scroll")
                                    .show(ui, |ui| {
                                        let size = egui::vec2(total_width, total_height);
                                        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

                                        let painter = ui.painter();

                                        // Draw timeline tracks background
                                        painter.rect_filled(rect, 0.0, ui.visuals().panel_fill);

                                        let tracks_top = rect.min.y + 26.0;

                                        // Allocate top 26px band of the timeline grid canvas as dedicated time ruler
                                        let ruler_rect = egui::Rect::from_min_max(
                                            egui::pos2(rect.min.x, rect.min.y),
                                            egui::pos2(rect.max.x, tracks_top),
                                        );

                                        let pointer_pos = ui.ctx().pointer_latest_pos();

                                        if let Some(pos) = pointer_pos {
                                            if rect.contains(pos) && scroll_delta.y != 0.0 {
                                                self.app.timeline_zoom = (self.app.timeline_zoom + scroll_delta.y * 0.2).clamp(20.0, 500.0);
                                                // The wheel gesture belongs to timeline zoom while
                                                // the pointer is over the canvas; prevent the parent
                                                // two-axis ScrollArea from also moving vertically.
                                                ui.ctx().input_mut(|input| {
                                                    input.smooth_scroll_delta.y = 0.0;
                                                });
                                            }
                                        }

                                        let ruler_response = ui.interact(ruler_rect, egui::Id::new("timeline_ruler"), egui::Sense::click_and_drag())
                                            .on_hover_text(&timeline_ruler_help);

                                        if let Some(pos) = pointer_pos {
                                            if (ruler_rect.contains(pos) || ruler_response.dragged())
                                                && self.app.active_drag.is_none()
                                                && self.app.lasso_origin.is_none()
                                                && self.app.active_keyframe_drag.is_none()
                                            {
                                                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                                if ui.input(|i| i.pointer.primary_down()) || ruler_response.dragged() {
                                                    let relative_x = (pos.x - rect.min.x).max(0.0);
                                                    let target_time = ((relative_x / zoom) as f64).clamp(0.0, total_seconds);
                                                    self.app.scrub_to(target_time);
                                                }
                                            }
                                        }

                                        if self.app.is_scrubbing && (!ui.input(|i| i.pointer.primary_down()) || ruler_response.drag_stopped()) {
                                            let current_target = self.app.seek_pos.unwrap_or(self.app.playback_time);
                                            self.app.finish_scrub(current_target);
                                        }

                                        // Draw grid lines
                                        // Major grid lines every second (zoom px)
                                        for i in 0..=(total_seconds.ceil() as i32) {
                                            let grid_x = rect.min.x + (i as f32 * zoom);
                                            if grid_x <= rect.max.x {
                                                painter.line_segment(
                                                    [egui::pos2(grid_x, rect.min.y), egui::pos2(grid_x, rect.max.y)],
                                                    ui.visuals().widgets.noninteractive.bg_stroke,
                                                );
                                            }
                                        }

                                        // Draw horizontal track separators and backgrounds
                                        for i in 0..=timeline_rows.len() {
                                            let grid_y = tracks_top + (i as f32 * 32.0);

                                            // Lock row background darkening
                                            if let Some(relay_id) = relay_for_timeline_row(&timeline_rows, i as i32) {
                                                if self.app.track_locked.contains(&relay_id) {
                                                    let track_rect = egui::Rect::from_min_max(
                                                        egui::pos2(rect.min.x, grid_y),
                                                        egui::pos2(rect.max.x, grid_y + 32.0),
                                                    );
                                                    painter.rect_filled(track_rect, 0.0, ui.visuals().faint_bg_color);
                                                }
                                            }

                                            painter.line_segment(
                                                [egui::pos2(rect.min.x, grid_y), egui::pos2(rect.max.x, grid_y)],
                                                ui.visuals().widgets.noninteractive.bg_stroke,
                                            );
                                        }

                                        // Horizontal separators for Analog tracks
                                        for (t_idx, _) in self.app.timeline.analog_tracks.iter().enumerate() {
                                            let grid_y = tracks_top + track_area_height + ((t_idx + 1) as f32 * 40.0);
                                            painter.line_segment(
                                                [egui::pos2(rect.min.x, grid_y), egui::pos2(rect.max.x, grid_y)],
                                                ui.visuals().widgets.noninteractive.bg_stroke,
                                            );
                                        }

                                        // Highlight target destination track row during active move drag
                                        if let Some(drag) = &self.app.active_drag {
                                            if drag.mode == crate::app::DragMode::Move {
                                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                                    let relative_y = mouse_pos.y - tracks_top;
                                                    let track_index = (relative_y / 32.0).floor() as i32;
                                                    if let Some(target_r) = relay_for_timeline_row(&timeline_rows, track_index) {
                                                        let target_compatible = self.app.timeline.instances.iter()
                                                            .find(|i| i.id == drag.instance_id)
                                                            .and_then(|inst| self.app.timeline.templates.iter().find(|t| t.id == inst.effect_id))
                                                            .map(|tmpl| tmpl.target.is_compatible_with_relay(target_r))
                                                            .unwrap_or(true);

                                                        if !self.app.track_locked.contains(&target_r) && target_compatible {
                                                            let row_y = tracks_top + (track_index as f32 * 32.0);
                                                            let dest_rect = egui::Rect::from_min_max(
                                                                egui::pos2(rect.min.x, row_y),
                                                                egui::pos2(rect.max.x, row_y + 32.0),
                                                            );
                                                            painter.rect_filled(dest_rect, 0.0, egui::Color32::from_rgba_unmultiplied(46, 204, 113, 25)); // Faint green highlight
                                                        } else if !target_compatible && !self.app.track_locked.contains(&target_r) {
                                                            let row_y = tracks_top + (track_index as f32 * 32.0);
                                                            let dest_rect = egui::Rect::from_min_max(
                                                                egui::pos2(rect.min.x, row_y),
                                                                egui::pos2(rect.max.x, row_y + 32.0),
                                                            );
                                                            painter.rect_filled(dest_rect, 0.0, egui::Color32::from_rgba_unmultiplied(231, 76, 60, 30)); // Faint red warning highlight for incompatible track
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if self.app.duration > 0.0 {
                                            for (row_index, track_row) in timeline_rows.iter().enumerate() {
                                                let (label, color) = match track_row.kind {
                                                    TimelineTrackKind::Video => (&track_row.name, egui::Color32::from_rgb(41, 128, 185)),
                                                    TimelineTrackKind::Audio => (&track_row.name, egui::Color32::from_rgb(39, 174, 96)),
                                                    TimelineTrackKind::ControllerMacros | TimelineTrackKind::Relay(_) => continue,
                                                };
                                                let row_y = tracks_top + row_index as f32 * 32.0;
                                                let clip_rect = egui::Rect::from_min_max(
                                                    egui::pos2(rect.min.x, row_y + 4.0),
                                                    egui::pos2(rect.min.x + total_width, row_y + 28.0),
                                                );
                                                painter.rect_filled(clip_rect, 4.0, color);
                                                painter.rect_stroke(clip_rect, 4.0, egui::Stroke::new(1.0_f32, egui::Color32::WHITE), egui::StrokeKind::Inside);
                                                painter.text(
                                                    clip_rect.left_center() + egui::vec2(10.0, 0.0),
                                                    egui::Align2::LEFT_CENTER,
                                                    label,
                                                    egui::FontId::proportional(11.0),
                                                    egui::Color32::WHITE,
                                                );
                                            }
                                        }

                                        let mut clicked_any_clip = false;
                                        let mut started_drag = None;
                                        let mut relocate_to_primary = None;
                                        let mut delete_cue_id = None;

                                        // 1st Pass: Draw all non-dragged clips
                                        let active_drag_id = self.app.active_drag.as_ref().map(|d| d.instance_id);
                                        let mut dragged_clip_data = None;

                                        for instance in &self.app.timeline.instances {
                                            if let Some(effect) = self.app.timeline.templates.iter().find(|t| t.id == instance.effect_id) {
                                                if effect.controller_macro.is_some()
                                                    || effect.controller_strip_effect.is_some()
                                                {
                                                    let Some(track_index) = timeline_rows
                                                        .iter()
                                                        .position(|row| row.kind == TimelineTrackKind::ControllerMacros)
                                                    else {
                                                        continue;
                                                    };
                                                    let track_y = tracks_top + track_index as f32 * 32.0;
                                                    let start_x = rect.min.x + (instance.start_time_ms as f32 * px_per_ms);
                                                    let end_x = start_x + (effect.duration_ms.max(1) as f32 * px_per_ms);
                                                    let clip_rect = egui::Rect::from_min_max(
                                                        egui::pos2(start_x, track_y + 4.0),
                                                        egui::pos2(end_x.max(start_x + 8.0), track_y + 28.0),
                                                    );
                                                    let response = ui.interact(
                                                        clip_rect,
                                                        egui::Id::new(instance.id),
                                                        egui::Sense::click(),
                                                    );
                                                    response.context_menu(|ui| {
                                                        if ui.button(egui::RichText::new(format!("× {timeline_delete_cue_label}")).color(egui::Color32::from_rgb(231, 76, 60))).clicked() {
                                                            delete_cue_id = Some(instance.id);
                                                            ui.close();
                                                        }
                                                    });
                                                    if response.clicked() {
                                                        clicked_any_clip = true;
                                                        self.app.selected_instance_ids.clear();
                                                        self.app.selected_instance_ids.insert(instance.id);
                                                    }
                                                    let selected = self.app.selected_instance_ids.contains(&instance.id);
                                                    painter.rect_filled(clip_rect, 4.0, egui::Color32::from_rgb(108, 76, 170));
                                                    painter.rect_stroke(
                                                        clip_rect,
                                                        4.0,
                                                        egui::Stroke::new(if selected { 2.0_f32 } else { 1.0_f32 }, egui::Color32::WHITE),
                                                        egui::StrokeKind::Inside,
                                                    );
                                                    painter.text(
                                                        clip_rect.left_center() + egui::vec2(8.0, 0.0),
                                                        egui::Align2::LEFT_CENTER,
                                                        &effect.name,
                                                        egui::FontId::proportional(10.0),
                                                        egui::Color32::WHITE,
                                                    );
                                                    continue;
                                                }
                                                // Find the relay used by this template's actions
                                                let Some(relay_id) = effect.actions.first().map(|a| a.relay_id) else {
                                                    continue;
                                                };
                                                let is_mismatched = !effect.target.is_compatible_with_relay(relay_id);
                                                let Some(track_index) = timeline_row_for_relay(&timeline_rows, relay_id) else {
                                                    continue;
                                                };
                                                let track_y = tracks_top + track_index as f32 * 32.0;

                                                let start_x = rect.min.x + (instance.start_time_ms as f32 * px_per_ms);
                                                let end_x = start_x + (effect.duration_ms as f32 * px_per_ms);

                                                let clip_rect = egui::Rect::from_min_max(
                                                    egui::pos2(start_x, track_y + 4.0),
                                                    egui::pos2(end_x, track_y + 28.0),
                                                );

                                                let clip_id = egui::Id::new(instance.id);
                                                let is_track_locked = self.app.track_locked.contains(&relay_id);

                                                let mut clip_response = if is_track_locked {
                                                    ui.interact(clip_rect, clip_id, egui::Sense::click())
                                                } else {
                                                    ui.interact(clip_rect, clip_id, egui::Sense::click_and_drag())
                                                };

                                                if is_mismatched {
                                                    let required_name = effect
                                                        .target
                                                        .primary_relay_id()
                                                        .and_then(|target_id| timeline_rows.iter().find(|row| row.kind == TimelineTrackKind::Relay(target_id)))
                                                        .map(|row| row.name.as_str())
                                                        .unwrap_or("unavailable project output");
                                                    let warn_msg = format!(
                                                        "Hardware target mismatch\nEffect: {}\nRequired output: {}\nRight-click to relocate when that output is available.",
                                                        effect.name, required_name
                                                    );
                                                    clip_response = clip_response.on_hover_text(warn_msg);
                                                }

                                                clip_response.context_menu(|ui| {
                                                    if is_mismatched {
                                                        if let Some(primary) = effect.target.primary_relay_id() {
                                                            if let Some(target_row) = timeline_rows.iter().find(|row| row.kind == TimelineTrackKind::Relay(primary))
                                                                && ui.button(format!("Relocate to {}", target_row.name)).clicked()
                                                            {
                                                                relocate_to_primary = Some(instance.effect_id);
                                                                ui.close();
                                                            }
                                                            ui.separator();
                                                        }
                                                    }
                                                    if ui.button(egui::RichText::new(format!("× {timeline_delete_cue_label}")).color(egui::Color32::from_rgb(231, 76, 60))).clicked() {
                                                        delete_cue_id = Some(instance.id);
                                                        ui.close();
                                                    }
                                                });

                                                let is_hovered = clip_response.hovered() && !is_track_locked;
                                                let mut hovered_handle = None;

                                                if is_hovered && self.app.active_drag.is_none() {
                                                    if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                                        let mode = crate::app::classify_clip_drag_mode(clip_rect.left(), clip_rect.right(), mouse_pos.x);
                                                        match mode {
                                                            crate::app::DragMode::ResizeLeft => {
                                                                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                                                hovered_handle = Some(crate::app::DragMode::ResizeLeft);
                                                            }
                                                            crate::app::DragMode::ResizeRight => {
                                                                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                                                hovered_handle = Some(crate::app::DragMode::ResizeRight);
                                                            }
                                                            crate::app::DragMode::Move => {
                                                                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                                            }
                                                        }
                                                    }
                                                }

                                                if clip_response.clicked() {
                                                    let is_ctrl = ui.ctx().input(|i| i.modifiers.command || i.modifiers.ctrl);
                                                    clicked_any_clip = true;
                                                    if is_ctrl {
                                                        if self.app.selected_instance_ids.contains(&instance.id) {
                                                            self.app.selected_instance_ids.remove(&instance.id);
                                                        } else {
                                                            self.app.selected_instance_ids.insert(instance.id);
                                                        }
                                                    } else {
                                                        if !self.app.selected_instance_ids.contains(&instance.id) {
                                                            self.app.selected_instance_ids.clear();
                                                            self.app.selected_instance_ids.insert(instance.id);
                                                        }
                                                    }
                                                }

                                                if clip_response.drag_started() && !is_track_locked {
                                                    clicked_any_clip = true;
                                                    let is_ctrl = ui.ctx().input(|i| i.modifiers.command || i.modifiers.ctrl);
                                                    if !self.app.selected_instance_ids.contains(&instance.id) {
                                                        if !is_ctrl {
                                                            self.app.selected_instance_ids.clear();
                                                        }
                                                        self.app.selected_instance_ids.insert(instance.id);
                                                    }

                                                    // Collect initial positions
                                                    let initial_positions: Vec<(uuid::Uuid, u64)> = self.app.timeline.instances.iter()
                                                        .filter(|inst| self.app.selected_instance_ids.contains(&inst.id))
                                                        .map(|inst| (inst.id, inst.start_time_ms))
                                                        .collect();

                                                    if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                                        let press_x = ui.ctx().input(|i| i.pointer.press_origin())
                                                            .or_else(|| clip_response.interact_pointer_pos())
                                                            .map(|p| p.x)
                                                            .unwrap_or(mouse_pos.x);

                                                        let drag_mode = crate::app::classify_clip_drag_mode(clip_rect.left(), clip_rect.right(), press_x);
                                                        started_drag = Some((instance.id, drag_mode, instance.start_time_ms, effect.duration_ms, mouse_pos.x, initial_positions));
                                                    }
                                                }

                                                if active_drag_id == Some(instance.id) {
                                                    // Save for 2nd pass
                                                    dragged_clip_data = Some((clip_rect, instance.id, effect.clone(), relay_id));
                                                    continue;
                                                }

                                                let is_selected = self.app.selected_instance_ids.contains(&instance.id);
                                                let stroke_color = if is_selected {
                                                    egui::Color32::from_rgb(255, 235, 59) // Selection Yellow outline
                                                } else if is_mismatched {
                                                    egui::Color32::from_rgb(245, 158, 11) // Warning amber border
                                                } else {
                                                    egui::Color32::WHITE
                                                };
                                                let stroke_width = if is_selected { 2.0_f32 } else if is_mismatched { 1.5_f32 } else { 1.0_f32 };

                                                let is_muted = self.app.track_muted.contains(&relay_id);
                                                let alpha = if is_muted { 128 } else { 255 };

                                                // Draw clip box
                                                painter.rect_filled(clip_rect, 4.0, egui::Color32::from_rgba_unmultiplied(142, 68, 173, alpha)); // Purple clip
                                                painter.rect_stroke(clip_rect, 4.0, egui::Stroke::new(stroke_width, stroke_color), egui::StrokeKind::Inside);

                                                // Visual handle grips
                                                let left_active = hovered_handle == Some(crate::app::DragMode::ResizeLeft);
                                                let right_active = hovered_handle == Some(crate::app::DragMode::ResizeRight);
                                                render_clip_handles(&painter, clip_rect, left_active, right_active, alpha);

                                                // Clip name label
                                                let displayed_effect_name = crate::ui::i18n::visual_text(display_language, &effect.name);
                                                let title = if is_mismatched {
                                                    format!("{} {} {}", crate::ui::icons::WARNING, crate::ui::icons::SPARKLE, displayed_effect_name)
                                                } else {
                                                    format!("{} {}", crate::ui::icons::SPARKLE, displayed_effect_name)
                                                };
                                                painter.text(
                                                    clip_rect.left_center() + egui::vec2(12.0, 0.0),
                                                    egui::Align2::LEFT_CENTER,
                                                    title,
                                                    egui::FontId::proportional(10.0),
                                                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                                                );
                                            }
                                        }

                                        // 2nd Pass: Draw the actively dragged clip on top with a shadow and brighter color
                                        if let Some((clip_rect, instance_id, effect, relay_id)) = dragged_clip_data {
                                            let is_mismatched = !effect.target.is_compatible_with_relay(relay_id);
                                            let is_selected = self.app.selected_instance_ids.contains(&instance_id);
                                            let stroke_color = if is_selected {
                                                egui::Color32::from_rgb(255, 235, 59)
                                            } else if is_mismatched {
                                                egui::Color32::from_rgb(245, 158, 11)
                                            } else {
                                                egui::Color32::WHITE
                                            };
                                            let stroke_width = if is_selected { 2.0_f32 } else if is_mismatched { 1.5_f32 } else { 1.0_f32 };

                                            // Draw drop shadow
                                            let shadow_rect = clip_rect.translate(egui::vec2(2.0, 3.0));
                                            painter.rect_filled(shadow_rect, 4.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 80));

                                            // Draw bright purple clip
                                            painter.rect_filled(clip_rect, 4.0, egui::Color32::from_rgb(172, 98, 203)); // Brighter purple
                                            painter.rect_stroke(clip_rect, 4.0, egui::Stroke::new(stroke_width, stroke_color), egui::StrokeKind::Inside);

                                            // Visual handle grips
                                            let left_active = self.app.active_drag.as_ref().map(|d| d.mode) == Some(crate::app::DragMode::ResizeLeft);
                                            let right_active = self.app.active_drag.as_ref().map(|d| d.mode) == Some(crate::app::DragMode::ResizeRight);
                                            render_clip_handles(&painter, clip_rect, left_active, right_active, 255);

                                            // Clip name label
                                            let displayed_effect_name = crate::ui::i18n::visual_text(display_language, &effect.name);
                                            let title = if is_mismatched {
                                                format!("{} {} {}", crate::ui::icons::WARNING, crate::ui::icons::SPARKLE, displayed_effect_name)
                                            } else {
                                                format!("{} {}", crate::ui::icons::SPARKLE, displayed_effect_name)
                                            };
                                            painter.text(
                                                clip_rect.left_center() + egui::vec2(12.0, 0.0),
                                                egui::Align2::LEFT_CENTER,
                                                title,
                                                egui::FontId::proportional(10.0),
                                                egui::Color32::WHITE,
                                            );
                                        }

                                        // Apply relocation or deletion from context menu outside borrow loop
                                        if let Some(effect_id) = relocate_to_primary {
                                            self.app.relocate_effect_to_primary(effect_id);
                                            ui.ctx().request_repaint();
                                        }
                                        if let Some(cue_id) = delete_cue_id {
                                            self.app.undo_stack.push(self.app.snapshot_timeline());
                                            self.app.timeline.instances.retain(|inst| inst.id != cue_id);
                                            self.app.selected_instance_ids.remove(&cue_id);
                                            self.app.sync_timeline_engine();
                                            ui.ctx().request_repaint();
                                        }

                                        // Apply selection or drag start outside the borrow loop
                                        if let Some((drag_id, mode, init_start, init_dur, start_x, init_positions)) = started_drag {
                                            // Push undo snapshot before mutating timeline
                                            self.app.undo_stack.push(self.app.snapshot_timeline());

                                            // If resizing, isolate template if shared by multiple instances
                                            if mode == crate::app::DragMode::ResizeLeft || mode == crate::app::DragMode::ResizeRight {
                                                self.app.isolate_template_for_instance(drag_id);
                                            }

                                            self.app.active_drag = Some(crate::app::ActiveDragState {
                                                instance_id: drag_id,
                                                mode,
                                                initial_start_time_ms: init_start,
                                                initial_duration_ms: init_dur,
                                                drag_start_x: start_x,
                                                initial_positions: init_positions,
                                            });
                                        }

                                        // Process active drag logic
                                        let mut drag_ended = false;
                                        let mut snap_line_x = None;

                                        if let Some(drag_state) = &self.app.active_drag {
                                            match drag_state.mode {
                                                crate::app::DragMode::Move => {
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                                }
                                                crate::app::DragMode::ResizeLeft | crate::app::DragMode::ResizeRight => {
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                                }
                                            }

                                            if ui.ctx().input(|i| i.pointer.any_released()) {
                                                drag_ended = true;
                                            } else {
                                                let delta_x = ui.ctx().pointer_latest_pos().map(|p| p.x).unwrap_or(drag_state.drag_start_x) - drag_state.drag_start_x;
                                                let delta_time_ms = (delta_x / px_per_ms) as i64;

                                                let snap_enabled = !ui.ctx().input(|i| i.modifiers.shift || i.modifiers.alt);

                                                // Build snap targets list
                                                let mut snap_targets = vec![0, (self.app.playback_time * 1000.0) as u64];
                                                for inst in &self.app.timeline.instances {
                                                    if inst.id == drag_state.instance_id || self.app.selected_instance_ids.contains(&inst.id) {
                                                        continue;
                                                    }
                                                    if let Some(tmpl) = self.app.timeline.templates.iter().find(|t| t.id == inst.effect_id) {
                                                        snap_targets.push(inst.start_time_ms);
                                                        snap_targets.push(inst.start_time_ms + tmpl.duration_ms);
                                                    }
                                                }

                                                let hud_text = match drag_state.mode {
                                                    crate::app::DragMode::Move => {
                                                        // Vertical track switching
                                                        let mut target_relay = None;
                                                        if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                                            let relative_y = mouse_pos.y - tracks_top;
                                                            let track_index = (relative_y / 32.0).floor() as i32;
                                                            if let Some(target_r) = relay_for_timeline_row(&timeline_rows, track_index) {
                                                                if !self.app.track_locked.contains(&target_r) {
                                                                    let is_compatible = self.app.timeline.instances.iter()
                                                                        .find(|i| i.id == drag_state.instance_id)
                                                                        .and_then(|inst| self.app.timeline.templates.iter().find(|t| t.id == inst.effect_id))
                                                                        .map(|tmpl| tmpl.target.is_compatible_with_relay(target_r))
                                                                        .unwrap_or(true);
                                                                    if is_compatible {
                                                                        target_relay = Some(target_r);
                                                                    }
                                                                }
                                                            }
                                                        }

                                                        let mut primary_new_start = (drag_state.initial_start_time_ms as i64 + delta_time_ms).max(0) as u64;

                                                        if snap_enabled {
                                                            // Check snap to start
                                                            for target in &snap_targets {
                                                                if (primary_new_start as i64 - *target as i64).abs() <= 100 {
                                                                    primary_new_start = *target;
                                                                    snap_line_x = Some(rect.min.x + (primary_new_start as f32 * px_per_ms));
                                                                    break;
                                                                }
                                                            }
                                                            // Check snap to end
                                                            if snap_line_x.is_none() {
                                                                let primary_new_end = primary_new_start + drag_state.initial_duration_ms;
                                                                for target in &snap_targets {
                                                                    if (primary_new_end as i64 - *target as i64).abs() <= 100 {
                                                                        primary_new_start = target.saturating_sub(drag_state.initial_duration_ms);
                                                                        snap_line_x = Some(rect.min.x + (*target as f32 * px_per_ms));
                                                                        break;
                                                                    }
                                                                }
                                                            }
                                                        }

                                                        let actual_delta_ms = primary_new_start as i64 - drag_state.initial_start_time_ms as i64;

                                                        for &(inst_id, init_start) in &drag_state.initial_positions {
                                                            if let Some(inst) = self.app.timeline.instances.iter_mut().find(|i| i.id == inst_id) {
                                                                let new_start = (init_start as i64 + actual_delta_ms).max(0) as u64;
                                                                inst.start_time_ms = new_start;

                                                                if inst_id == drag_state.instance_id {
                                                                    if let Some(r) = target_relay {
                                                                        if let Some(template) = self.app.timeline.templates.iter_mut().find(|t| t.id == inst.effect_id) {
                                                                            template.actions = crate::four_d::patterns::generate_constant(r, true, template.duration_ms);
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }

                                                        let start_secs = primary_new_start as f64 / 1000.0;
                                                        let track_name = target_relay.map(|r| format!("R{}", r)).unwrap_or_else(|| "Track".to_string());
                                                        format!("⏱ Start: {:.3}s | {}", start_secs, track_name)
                                                    }
                                                    crate::app::DragMode::ResizeRight => {
                                                        let mut new_end = (drag_state.initial_start_time_ms + drag_state.initial_duration_ms) as i64 + delta_time_ms;
                                                        if snap_enabled {
                                                            for target in &snap_targets {
                                                                if (new_end - *target as i64).abs() <= 100 {
                                                                    new_end = *target as i64;
                                                                    snap_line_x = Some(rect.min.x + (new_end as f32 * px_per_ms));
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                        let new_dur = (new_end - drag_state.initial_start_time_ms as i64).max(100) as u64;
                                                        let instance_id = drag_state.instance_id;
                                                        if let Some(instance) = self.app.timeline.instances.iter_mut().find(|inst| inst.id == instance_id) {
                                                            if let Some(template) = self.app.timeline.templates.iter_mut().find(|t| t.id == instance.effect_id) {
                                                                crate::app::update_effect_duration(template, new_dur);
                                                            }
                                                        }

                                                        let delta_ms = (new_dur as i64) - (drag_state.initial_duration_ms as i64);
                                                        let delta_str = if delta_ms >= 0 { format!("+{}ms", delta_ms) } else { format!("{}ms", delta_ms) };
                                                        let dur_secs = new_dur as f64 / 1000.0;
                                                        format!("⏱ Dur: {:.2}s ({})", dur_secs, delta_str)
                                                    }
                                                    crate::app::DragMode::ResizeLeft => {
                                                        let right_anchor = drag_state.initial_start_time_ms + drag_state.initial_duration_ms;
                                                        let mut new_start = (drag_state.initial_start_time_ms as i64 + delta_time_ms).max(0) as u64;
                                                        if snap_enabled {
                                                            for target in &snap_targets {
                                                                if (new_start as i64 - *target as i64).abs() <= 100 {
                                                                    new_start = *target;
                                                                    snap_line_x = Some(rect.min.x + (new_start as f32 * px_per_ms));
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                        new_start = new_start.min(right_anchor.saturating_sub(100));
                                                        let new_dur = right_anchor - new_start;
                                                        let instance_id = drag_state.instance_id;
                                                        if let Some(instance) = self.app.timeline.instances.iter_mut().find(|inst| inst.id == instance_id) {
                                                            instance.start_time_ms = new_start;
                                                            if let Some(template) = self.app.timeline.templates.iter_mut().find(|t| t.id == instance.effect_id) {
                                                                crate::app::update_effect_duration(template, new_dur);
                                                            }
                                                        }

                                                        let delta_ms = (new_dur as i64) - (drag_state.initial_duration_ms as i64);
                                                        let delta_str = if delta_ms >= 0 { format!("+{}ms", delta_ms) } else { format!("{}ms", delta_ms) };
                                                        let dur_secs = new_dur as f64 / 1000.0;
                                                        format!("⏱ Dur: {:.2}s ({})", dur_secs, delta_str)
                                                    }
                                                };

                                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                                    let galley = painter.layout_no_wrap(
                                                        hud_text.clone(),
                                                        egui::FontId::monospace(11.0),
                                                        egui::Color32::WHITE,
                                                    );
                                                    let padding = egui::vec2(8.0, 4.0);
                                                    let badge_size = galley.size() + padding * 2.0;
                                                    let hud_center = egui::pos2(mouse_pos.x, mouse_pos.y - 25.0);
                                                    let half_w = badge_size.x / 2.0;
                                                    let half_h = badge_size.y / 2.0;
                                                    let min_x = rect.min.x + half_w + 4.0;
                                                    let max_x = rect.max.x - half_w - 4.0;
                                                    let clamped_x = if min_x <= max_x { hud_center.x.clamp(min_x, max_x) } else { rect.center().x };

                                                    let min_y = rect.min.y + half_h + 4.0;
                                                    let max_y = rect.max.y - half_h - 4.0;
                                                    let clamped_y = if min_y <= max_y { hud_center.y.clamp(min_y, max_y) } else { rect.center().y };
                                                    let hud_rect = egui::Rect::from_center_size(egui::pos2(clamped_x, clamped_y), badge_size);

                                                    painter.rect_filled(
                                                        hud_rect,
                                                        4.0,
                                                        egui::Color32::from_black_alpha(220),
                                                    );
                                                    painter.rect_stroke(
                                                        hud_rect,
                                                        4.0,
                                                        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0, 220, 255)),
                                                        egui::StrokeKind::Inside,
                                                    );
                                                    painter.text(
                                                        hud_rect.center(),
                                                        egui::Align2::CENTER_CENTER,
                                                        hud_text,
                                                        egui::FontId::monospace(11.0),
                                                        egui::Color32::WHITE,
                                                    );
                                                }
                                            }
                                        }

                                        if drag_ended {
                                            self.app.active_drag = None;
                                            let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                            let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                        }

                                        if self.app.active_drag.is_some() {
                                            ui.ctx().request_repaint();
                                        }

                                        // Keyboard throttle update for Live Fader
                                        let up = ui.input(|i| i.key_down(egui::Key::W) || i.key_down(egui::Key::ArrowUp));
                                        let down = ui.input(|i| i.key_down(egui::Key::S) || i.key_down(egui::Key::ArrowDown));
                                        let dt = ui.input(|i| i.unstable_dt).min(0.05);
                                        self.app.input_capture.update_from_keyboard(up, down, dt);

                                        let is_playing_now = !self.app.is_paused && self.app.duration > 0.0;
                                        let was_recording = self.app.is_recording;
                                        let cur_time_ms = (self.app.playback_time * 1000.0) as u64;
                                        self.app.is_recording = false;

                                        if is_playing_now {
                                            for track in self.app.timeline.analog_tracks.iter_mut() {
                                                if track.armed {
                                                    self.app.is_recording = true;
                                                    let last_time = self.app.recording_session.get_live_samples(track.id).and_then(|s| s.last()).map(|s| s.0);
                                                    if last_time != Some(cur_time_ms) {
                                                        self.app.recording_session.record_sample(track.id, cur_time_ms, self.app.input_capture.current_throttle);
                                                    }
                                                    let byte_val = (self.app.input_capture.current_throttle * 255.0).round() as u8;
                                                    let _ = self.app.engine_handle.sender.send(
                                                        crate::four_d::engine::EngineMessage::LiveActuatorOverride {
                                                            channel: track.channel,
                                                            value: byte_val,
                                                        },
                                                    );
                                                }
                                            }
                                        }

                                        if was_recording && !is_playing_now {
                                            self.app.commit_recorded_samples();
                                        }

                                        if self.app.is_recording || (is_playing_now && self.app.timeline.analog_tracks.iter().any(|t| t.armed)) {
                                            ui.ctx().request_repaint();
                                        }

                                        // Render Analog Curve Tracks
                                        let mut clicked_any_keyframe = false;
                                        let mut curve_updated = false;
                                        let pointer_pos = ui.ctx().pointer_latest_pos();

                                        let mut started_drag_info = None;
                                        let mut kf_interp_change = None;
                                        let mut kf_to_remove = None;
                                        let mut pending_add_keyframe = None;

                                        for (t_idx, track) in self.app.timeline.analog_tracks.iter_mut().enumerate() {
                                            let row_y = tracks_top + track_area_height + (t_idx as f32 * 40.0);
                                            let row_rect = egui::Rect::from_min_max(
                                                egui::pos2(rect.min.x, row_y),
                                                egui::pos2(rect.max.x, row_y + 40.0),
                                            );

                                            // Row background shading
                                            painter.rect_filled(row_rect, 0.0, ui.visuals().extreme_bg_color);

                                            // Centerline guide (50% intensity)
                                            painter.line_segment(
                                                [egui::pos2(rect.min.x, row_y + 20.0), egui::pos2(rect.max.x, row_y + 20.0)],
                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(34, 42, 48)),
                                            );

                                            // Sample continuous curve along timeline
                                            let step_px = 6.0_f32;
                                            let mut points = Vec::new();
                                            let mut curr_x = rect.min.x;
                                            while curr_x <= rect.max.x {
                                                let t_ms = (((curr_x - rect.min.x) / zoom) * 1000.0).max(0.0) as u64;
                                                let norm_val = track.evaluate(t_ms);
                                                let py = (row_y + 36.0) - (norm_val * 32.0);
                                                points.push(egui::pos2(curr_x, py));
                                                curr_x += step_px;
                                            }

                                            // Draw translucent fill under curve (Curve Gradient Underlay)
                                            let fill_col = if track.muted {
                                                egui::Color32::from_rgba_unmultiplied(100, 100, 100, 20)
                                            } else {
                                                egui::Color32::from_rgba_unmultiplied(0, 220, 255, 25)
                                            };
                                            for window in points.windows(2) {
                                                let p1 = window[0];
                                                let p2 = window[1];
                                                let b1 = egui::pos2(p1.x, row_y + 36.0);
                                                let b2 = egui::pos2(p2.x, row_y + 36.0);
                                                painter.add(egui::Shape::convex_polygon(
                                                    vec![b1, p1, p2, b2],
                                                    fill_col,
                                                    egui::Stroke::NONE,
                                                ));
                                            }

                                            // Draw curve line
                                            let curve_color = if track.muted {
                                                egui::Color32::from_rgb(110, 110, 110)
                                            } else {
                                                egui::Color32::from_rgb(0, 220, 255)
                                            };
                                            painter.add(egui::Shape::line(
                                                points,
                                                egui::Stroke::new(1.8_f32, curve_color),
                                            ));

                                            // Draw recording ghost trail
                                            if track.armed {
                                                if let Some(live_samples) = self.app.recording_session.get_live_samples(track.id) {
                                                    if !live_samples.is_empty() {
                                                        let mut ghost_points = Vec::with_capacity(live_samples.len());
                                                        for s in live_samples {
                                                            let gx = rect.min.x + (s.0 as f32 * px_per_ms);
                                                            let gy = (row_y + 36.0) - (s.1 * 32.0);
                                                            ghost_points.push(egui::pos2(gx, gy));
                                                        }
                                                        painter.add(egui::Shape::line(
                                                            ghost_points,
                                                            egui::Stroke::new(2.5_f32, egui::Color32::from_rgb(255, 50, 50)),
                                                        ));
                                                    }
                                                }
                                            }

                                            // Keyframe markers and interactions
                                            for (k_idx, kf) in track.keyframes.iter().enumerate() {
                                                let kx = rect.min.x + (kf.time_ms as f32 * px_per_ms);
                                                let ky = (row_y + 36.0) - (kf.value * 32.0);
                                                let center = egui::pos2(kx, ky);
                                                let is_selected = self.app.selected_keyframes.contains(&(track.id, k_idx));

                                                // 16px Euclidean distance hitbox detection
                                                let hover_dist = 16.0;
                                                let is_hovered = pointer_pos.map_or(false, |pos| pos.distance(center) <= hover_dist);

                                                if is_hovered && self.app.active_keyframe_drag.is_none() {
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                                }

                                                let diamond = vec![
                                                    egui::pos2(center.x, center.y - 5.0),
                                                    egui::pos2(center.x + 5.0, center.y),
                                                    egui::pos2(center.x, center.y + 5.0),
                                                    egui::pos2(center.x - 5.0, center.y),
                                                ];
                                                let fill_diamond = if is_selected {
                                                    egui::Color32::from_rgb(255, 230, 0)
                                                } else {
                                                    curve_color
                                                };

                                                // Outline glows bright white/yellow on hover
                                                let stroke = if is_hovered {
                                                    egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 255, 180))
                                                } else {
                                                    egui::Stroke::new(1.2_f32, egui::Color32::WHITE)
                                                };

                                                if is_hovered {
                                                    let glow_diamond = vec![
                                                        egui::pos2(center.x, center.y - 7.5),
                                                        egui::pos2(center.x + 7.5, center.y),
                                                        egui::pos2(center.x, center.y + 7.5),
                                                        egui::pos2(center.x - 7.5, center.y),
                                                    ];
                                                    painter.add(egui::Shape::convex_polygon(
                                                        glow_diamond,
                                                        egui::Color32::from_rgba_unmultiplied(255, 255, 150, 45),
                                                        egui::Stroke::NONE,
                                                    ));
                                                }

                                                painter.add(egui::Shape::convex_polygon(
                                                    diamond,
                                                    fill_diamond,
                                                    stroke,
                                                ));

                                                // Keyframe interact widget for context menu and clicks
                                                let kf_rect = egui::Rect::from_center_size(center, egui::vec2(32.0, 32.0));
                                                let kf_id = egui::Id::new((track.id, k_idx, "kf_node"));
                                                let mut kf_response = ui.interact(kf_rect, kf_id, egui::Sense::click());

                                                // Native tooltip on hover showing timecode, value percentage, and interpolation mode
                                                if self.app.active_keyframe_drag.is_none() {
                                                    let interp_name = match kf.interpolation {
                                                        crate::four_d::curve::Interpolation::Step => &step_label,
                                                        crate::four_d::curve::Interpolation::Linear => &linear_label,
                                                        crate::four_d::curve::Interpolation::Smooth => &smooth_label,
                                                    };
                                                    kf_response = kf_response.on_hover_ui(|ui| {
                                                        ui.label(format!("{time_label}: {}", format_timecode(kf.time_ms as f64 / 1000.0)));
                                                        ui.label(format!("{value_label}: {:.1}%", kf.value * 100.0));
                                                        ui.label(format!("{interpolation_label}: {}", interp_name));
                                                    });
                                                }

                                                // Right-Click Context Menu
                                                kf_response.context_menu(|ui| {
                                                    crate::ui::icons::submenu(ui, interpolation_label.clone(), |ui| {
                                                        if ui.button(&linear_label).clicked() {
                                                            kf_interp_change = Some((track.id, k_idx, crate::four_d::curve::Interpolation::Linear));
                                                            ui.close();
                                                        }
                                                        if ui.button(&smooth_label).clicked() {
                                                            kf_interp_change = Some((track.id, k_idx, crate::four_d::curve::Interpolation::Smooth));
                                                            ui.close();
                                                        }
                                                        if ui.button(&step_label).clicked() {
                                                            kf_interp_change = Some((track.id, k_idx, crate::four_d::curve::Interpolation::Step));
                                                            ui.close();
                                                        }
                                                    });
                                                    ui.separator();
                                                    if ui.button(&delete_keyframe_label).clicked() {
                                                        kf_to_remove = Some((track.id, k_idx));
                                                        ui.close();
                                                    }
                                                });

                                                if is_hovered && ui.input(|i| i.pointer.secondary_clicked()) {
                                                    clicked_any_keyframe = true;
                                                }

                                                // Primary click: Selection & Active Drag Lock initialization on mouse press/down
                                                if is_hovered
                                                    && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary) || i.pointer.primary_down())
                                                    && self.app.active_keyframe_drag.is_none()
                                                {
                                                    if let Some(pos) = pointer_pos {
                                                        started_drag_info = Some((track.id, k_idx, pos, kf.time_ms, kf.value));
                                                        clicked_any_keyframe = true;
                                                    }
                                                }
                                            }

                                            if response.double_clicked() && !clicked_any_keyframe {
                                                if let Some(pos) = response.interact_pointer_pos() {
                                                    if row_rect.contains(pos) {
                                                        let new_t = (((pos.x - rect.min.x) / zoom) * 1000.0).max(0.0) as u64;
                                                        let new_v = ((row_y + 36.0 - pos.y) / 32.0).clamp(0.0, 1.0);
                                                        pending_add_keyframe = Some((track.id, new_t, new_v));
                                                        clicked_any_keyframe = true;
                                                    }
                                                }
                                            }
                                        }

                                        if let Some((tid, new_t, new_v)) = pending_add_keyframe {
                                            let pre_snap = self.app.snapshot_timeline();
                                            self.app.undo_stack.push(pre_snap);
                                            if let Some(track) = self.app.timeline.analog_tracks.iter_mut().find(|t| t.id == tid) {
                                                track.add_keyframe(crate::four_d::curve::Keyframe::new(
                                                    new_t,
                                                    new_v,
                                                    crate::four_d::curve::Interpolation::Linear,
                                                ));
                                            }
                                            curve_updated = true;
                                        }

                                        if let Some((tid, kid, new_interp)) = kf_interp_change {
                                            let pre_snap = self.app.snapshot_timeline();
                                            self.app.undo_stack.push(pre_snap);
                                            if let Some(track) = self.app.timeline.analog_tracks.iter_mut().find(|t| t.id == tid) {
                                                if let Some(kf) = track.keyframes.get_mut(kid) {
                                                    kf.interpolation = new_interp;
                                                }
                                            }
                                            curve_updated = true;
                                        }

                                        if let Some((tid, kid)) = kf_to_remove {
                                            let pre_snap = self.app.snapshot_timeline();
                                            self.app.undo_stack.push(pre_snap);
                                            if let Some(track) = self.app.timeline.analog_tracks.iter_mut().find(|t| t.id == tid) {
                                                if kid < track.keyframes.len() {
                                                    track.keyframes.remove(kid);
                                                }
                                            }
                                            self.app.selected_keyframes.clear();
                                            curve_updated = true;
                                        }

                                        if let Some((track_id, k_idx, pos, orig_t, orig_v)) = started_drag_info {
                                            if !self.app.selected_keyframes.contains(&(track_id, k_idx)) {
                                                if !ui.input(|i| i.modifiers.shift || i.modifiers.ctrl) {
                                                    self.app.selected_keyframes.clear();
                                                }
                                                self.app.selected_keyframes.insert((track_id, k_idx));
                                            }
                                            let mut group_originals = Vec::new();
                                            for &(tid, kid) in &self.app.selected_keyframes {
                                                if let Some(t) = self.app.timeline.analog_tracks.iter().find(|t| t.id == tid) {
                                                    if let Some(k) = t.keyframes.get(kid) {
                                                        group_originals.push((tid, kid, k.time_ms, k.value));
                                                    }
                                                }
                                            }
                                            if !group_originals.iter().any(|&(tid, kid, _, _)| tid == track_id && kid == k_idx) {
                                                group_originals.push((track_id, k_idx, orig_t, orig_v));
                                            }
                                            self.app.active_keyframe_drag = Some(crate::app::KeyframeDragState {
                                                track_id,
                                                keyframe_index: k_idx,
                                                start_pointer_pos: pos,
                                                original_time_ms: orig_t,
                                                original_value: orig_v,
                                                group_originals,
                                            });
                                        }

                                        // Active Keyframe Drag Processing & Drag Lock
                                        if let Some(drag) = self.app.active_keyframe_drag.clone() {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                            ui.ctx().request_repaint();

                                            let drag_ended = ui.ctx().input(|i| i.pointer.any_released());

                                            if drag_ended {
                                                // Reconstruct pre-drag snapshot
                                                let mut pre_snap = self.app.snapshot_timeline();
                                                for &(tid, kid, orig_t, orig_v) in &drag.group_originals {
                                                    if let Some(t) = pre_snap.analog_tracks.iter_mut().find(|t| t.id == tid) {
                                                        if let Some(k) = t.keyframes.get_mut(kid) {
                                                            k.time_ms = orig_t;
                                                            k.value = orig_v;
                                                        }
                                                    }
                                                }
                                                for t in &mut pre_snap.analog_tracks {
                                                    t.keyframes.sort_by_key(|k| k.time_ms);
                                                }
                                                let any_changed = drag.group_originals.iter().any(|&(tid, kid, orig_t, orig_v)| {
                                                    self.app.timeline.analog_tracks.iter().find(|t| t.id == tid)
                                                        .and_then(|t| t.keyframes.get(kid))
                                                        .map_or(false, |k| k.time_ms != orig_t || (k.value - orig_v).abs() > 1e-4)
                                                });
                                                if any_changed {
                                                    self.app.undo_stack.push(pre_snap);
                                                }

                                                let mut target_keyframes = Vec::new();
                                                for &(tid, kid) in &self.app.selected_keyframes {
                                                    if let Some(t) = self.app.timeline.analog_tracks.iter().find(|t| t.id == tid) {
                                                        if let Some(k) = t.keyframes.get(kid) {
                                                            target_keyframes.push((tid, k.time_ms, (k.value * 10000.0).round() as i64));
                                                        }
                                                    }
                                                }

                                                // Sort keyframes on mouse release
                                                for track in self.app.timeline.analog_tracks.iter_mut() {
                                                    track.keyframes.sort_by_key(|k| k.time_ms);
                                                }

                                                // Restore selected_keyframes with updated indices
                                                let mut new_selection = std::collections::HashSet::new();
                                                for (tid, target_t, target_v) in target_keyframes {
                                                    if let Some(t) = self.app.timeline.analog_tracks.iter().find(|t| t.id == tid) {
                                                        if let Some(new_idx) = t.keyframes.iter().enumerate().position(|(idx, k)| {
                                                            !new_selection.contains(&(tid, idx))
                                                                && k.time_ms == target_t
                                                                && ((k.value * 10000.0).round() as i64 - target_v).abs() <= 1
                                                        }) {
                                                            new_selection.insert((tid, new_idx));
                                                        }
                                                    }
                                                }
                                                self.app.selected_keyframes = new_selection;

                                                self.app.active_keyframe_drag = None;
                                                curve_updated = true;
                                            } else if let Some(pos) = pointer_pos {
                                                let delta_x = pos.x - drag.start_pointer_pos.x;
                                                let delta_time_ms = (delta_x / px_per_ms) as i64;
                                                let raw_new_t = (drag.original_time_ms as i64 + delta_time_ms).max(0) as u64;

                                                // Magnetic Snapping (within 5px of playhead or 1s grid mark)
                                                let current_playhead_time = self.app.seek_pos.unwrap_or(self.app.playback_time);
                                                let playhead_ms = (current_playhead_time * 1000.0).round() as u64;
                                                let nearest_sec = ((raw_new_t as f64 / 1000.0).round() as u64) * 1000;

                                                let play_x = rect.min.x + (playhead_ms as f32 * px_per_ms);
                                                let grid_x = rect.min.x + (nearest_sec as f32 * px_per_ms);
                                                let kf_x = rect.min.x + (raw_new_t as f32 * px_per_ms);

                                                let snap_enabled = !ui.input(|i| i.modifiers.shift || i.modifiers.alt);
                                                let mut new_t = raw_new_t;
                                                if snap_enabled {
                                                    let dist_play = (kf_x - play_x).abs();
                                                    let dist_grid = (kf_x - grid_x).abs();
                                                    if dist_play <= 5.0 && dist_play <= dist_grid {
                                                        new_t = playhead_ms;
                                                        snap_line_x = Some(play_x);
                                                    } else if dist_grid <= 5.0 {
                                                        new_t = nearest_sec;
                                                        snap_line_x = Some(grid_x);
                                                    }
                                                }

                                                let delta_y = pos.y - drag.start_pointer_pos.y;
                                                let val_delta = -delta_y / 32.0;
                                                let new_v = (drag.original_value + val_delta).clamp(0.0, 1.0);

                                                let time_delta = new_t as i64 - drag.original_time_ms as i64;

                                                for &(tid, kid, orig_t, orig_v) in &drag.group_originals {
                                                    let k_new_t = (orig_t as i64 + time_delta).max(0) as u64;
                                                    let k_new_v = (orig_v + val_delta).clamp(0.0, 1.0);
                                                    if let Some(t) = self.app.timeline.analog_tracks.iter_mut().find(|t| t.id == tid) {
                                                        if let Some(k) = t.keyframes.get_mut(kid) {
                                                            k.time_ms = k_new_t;
                                                            k.value = k_new_v;
                                                        }
                                                    }
                                                }
                                                curve_updated = true;

                                                // Floating HUD Badge
                                                let hud_pos = egui::pos2(pos.x, pos.y - 20.0);
                                                let hud_text = format!("{} | {:.1}%", format_timecode(new_t as f64 / 1000.0), new_v * 100.0);
                                                painter.rect_filled(
                                                    egui::Rect::from_center_size(hud_pos, egui::vec2(100.0, 18.0)),
                                                    4.0,
                                                    egui::Color32::from_black_alpha(200),
                                                );
                                                painter.text(hud_pos, egui::Align2::CENTER_CENTER, hud_text, egui::FontId::monospace(10.0), egui::Color32::WHITE);
                                            }
                                        }

                                        if curve_updated {
                                            let _ = self.app.engine_handle.sender.send(
                                                crate::four_d::engine::EngineMessage::UpdateAnalogTracks(
                                                    self.app.timeline.analog_tracks.clone(),
                                                ),
                                            );
                                        }

                                        // Render Dedicated Time Ruler Bar Header
                                        painter.rect_filled(ruler_rect, 0.0, ui.visuals().panel_fill);
                                        painter.line_segment(
                                            [egui::pos2(ruler_rect.min.x, ruler_rect.max.y), egui::pos2(ruler_rect.max.x, ruler_rect.max.y)],
                                            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(50, 50, 50)),
                                        );

                                        let label_step = if zoom < 30.0 {
                                            5
                                        } else if zoom < 60.0 {
                                            2
                                        } else {
                                            1
                                        };

                                        for i in 0..=(total_seconds.ceil() as i32) {
                                            let grid_x = rect.min.x + (i as f32 * zoom);
                                            if grid_x <= rect.max.x - 8.0 {
                                                // Major second tick
                                                painter.line_segment(
                                                    [egui::pos2(grid_x, ruler_rect.max.y - 8.0), egui::pos2(grid_x, ruler_rect.max.y)],
                                                    egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(100, 100, 100)),
                                                );
                                                // Sub-second frame notches
                                                if zoom >= 50.0 {
                                                    for sub in 1..10 {
                                                        let sub_x = grid_x + (sub as f32 * (zoom / 10.0));
                                                        if sub_x <= rect.max.x - 8.0 {
                                                            let notch_h = if sub == 5 { 5.0 } else { 3.0 };
                                                            painter.line_segment(
                                                                [egui::pos2(sub_x, ruler_rect.max.y - notch_h), egui::pos2(sub_x, ruler_rect.max.y)],
                                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(60, 60, 60)),
                                                            );
                                                        }
                                                    }
                                                }
                                                // Time label inside ruler
                                                if i % label_step == 0 {
                                                    let mut label_x = grid_x + 4.0;
                                                    // Ensure label doesn't clip against right margin
                                                    if label_x + 20.0 > rect.max.x - 8.0 {
                                                        label_x = rect.max.x - 28.0;
                                                    }
                                                    painter.text(
                                                        egui::pos2(label_x, ruler_rect.min.y + 13.0),
                                                        egui::Align2::LEFT_CENTER,
                                                        format!("{}s", i),
                                                        egui::FontId::monospace(9.0),
                                                        egui::Color32::from_rgb(140, 140, 140),
                                                    );
                                                }
                                            }
                                        }

                                        // Draw Playhead
                                        let current_playhead_time = self.app.seek_pos.unwrap_or(self.app.playback_time);
                                        let playhead_x = rect.min.x + (current_playhead_time as f32 * zoom);
                                        if playhead_x <= rect.max.x {
                                            // Vertical line
                                            painter.line_segment(
                                                [egui::pos2(playhead_x, rect.min.y), egui::pos2(playhead_x, rect.max.y)],
                                                egui::Stroke::new(1.5_f32, egui::Color32::RED),
                                            );
                                            // Playhead handle (triangle at top in ruler bar, 14px width)
                                            let points = vec![
                                                egui::pos2(playhead_x - 7.0, rect.min.y),
                                                egui::pos2(playhead_x + 7.0, rect.min.y),
                                                egui::pos2(playhead_x, rect.min.y + 12.0),
                                            ];
                                            painter.add(egui::Shape::convex_polygon(points, egui::Color32::RED, egui::Stroke::NONE));
                                        }

                                        // Draw Snap line if active
                                        if let Some(x) = snap_line_x {
                                            painter.line_segment(
                                                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0, 255, 255)), // Cyan snap line
                                            );
                                        }

                                        // Target highlighting during active drag
                                        if let Some(payload) = egui::DragAndDrop::payload::<EffectDragPayload>(ui.ctx()) {
                                            if let Some(mouse_pos) = ui.ctx().pointer_hover_pos() {
                                                if rect.contains(mouse_pos) {
                                                    let relative_y = mouse_pos.y - tracks_top;
                                                    let hovered_track_index = (relative_y / 32.0).floor() as i32;

                                                    for (i, track_row) in timeline_rows.iter().enumerate() {
                                                        if payload.controller_macro.is_some()
                                                            || payload.controller_strip_effect.is_some()
                                                        {
                                                            if track_row.kind == TimelineTrackKind::ControllerMacros {
                                                                let row_y = tracks_top + (i as f32 * 32.0);
                                                                let track_rect = egui::Rect::from_min_max(
                                                                    egui::pos2(rect.min.x, row_y),
                                                                    egui::pos2(rect.max.x, row_y + 32.0),
                                                                );
                                                                if hovered_track_index == i as i32 {
                                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Copy);
                                                                    painter.rect_filled(track_rect, 0.0, egui::Color32::from_rgba_unmultiplied(108, 76, 170, 55));
                                                                    painter.rect_stroke(track_rect, 0.0, egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(170, 130, 255)), egui::StrokeKind::Inside);
                                                                }
                                                            }
                                                            continue;
                                                        }
                                                        let TimelineTrackKind::Relay(relay_id) = track_row.kind else {
                                                            continue;
                                                        };
                                                        let i = i as i32;
                                                        let is_compatible = payload.target.is_compatible_with_relay(relay_id);
                                                        let is_locked = self.app.track_locked.contains(&relay_id);
                                                        let is_primary = payload.target.primary_relay_id() == Some(relay_id);
                                                        let row_y = tracks_top + (i as f32 * 32.0);
                                                        let track_rect = egui::Rect::from_min_max(
                                                            egui::pos2(rect.min.x, row_y),
                                                            egui::pos2(rect.max.x, row_y + 32.0),
                                                        );

                                                        if hovered_track_index == i {
                                                            if is_locked || !is_compatible {
                                                                ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
                                                                painter.rect_filled(track_rect, 0.0, egui::Color32::from_rgba_unmultiplied(255, 70, 70, 45));
                                                                painter.rect_stroke(track_rect, 0.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(255, 70, 70)), egui::StrokeKind::Inside);

                                                                let reason = if is_locked {
                                                                    "Track is locked".to_string()
                                                                } else {
                                                                    let required = payload
                                                                        .target
                                                                        .primary_relay_id()
                                                                        .and_then(|target_id| timeline_rows.iter().find(|row| row.kind == TimelineTrackKind::Relay(target_id)))
                                                                        .map(|row| row.name.as_str())
                                                                        .unwrap_or("an unavailable output");
                                                                    format!("'{}' targets {}", payload.name, required)
                                                                };
                                                                egui::Tooltip::always_open(
                                                                    ui.ctx().clone(),
                                                                    ui.layer_id(),
                                                                    egui::Id::new("drag_incompat_tip"),
                                                                    egui::PopupAnchor::Pointer,
                                                                )
                                                                .show(|ui| {
                                                                    ui.label(reason);
                                                                });
                                                            } else {
                                                                ui.ctx().set_cursor_icon(egui::CursorIcon::Copy);
                                                                painter.rect_filled(track_rect, 0.0, egui::Color32::from_rgba_unmultiplied(0, 255, 136, 45));
                                                                painter.rect_stroke(track_rect, 0.0, egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(0, 255, 136)), egui::StrokeKind::Inside);

                                                                let tooltip_text = format!(
                                                                    "Place '{}' on {}",
                                                                    crate::ui::i18n::visual_text(display_language, &payload.name),
                                                                    track_row.name,
                                                                );
                                                                egui::Tooltip::always_open(
                                                                    ui.ctx().clone(),
                                                                    ui.layer_id(),
                                                                    egui::Id::new("drag_compat_tip"),
                                                                    egui::PopupAnchor::Pointer,
                                                                )
                                                                .show(|ui| {
                                                                    ui.label(tooltip_text);
                                                                });
                                                            }
                                                        } else if is_primary && !is_locked {
                                                            // Subtle beacon highlight on primary track
                                                            painter.rect_stroke(
                                                                track_rect,
                                                                0.0,
                                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(0, 255, 136, 120)),
                                                                egui::StrokeKind::Inside,
                                                            );
                                                        }
                                                    }

                                                    if timeline_rows
                                                        .get(usize::try_from(hovered_track_index).unwrap_or(usize::MAX))
                                                        .is_some_and(|row| {
                                                            if payload.controller_macro.is_some()
                                                                || payload.controller_strip_effect.is_some()
                                                            {
                                                                row.kind != TimelineTrackKind::ControllerMacros
                                                            } else {
                                                                !matches!(row.kind, TimelineTrackKind::Relay(_))
                                                            }
                                                        })
                                                    {
                                                        ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
                                                        let row_y = tracks_top + (hovered_track_index as f32 * 32.0);
                                                        let track_rect = egui::Rect::from_min_max(
                                                            egui::pos2(rect.min.x, row_y),
                                                            egui::pos2(rect.max.x, row_y + 32.0),
                                                        );
                                                        painter.rect_filled(track_rect, 0.0, egui::Color32::from_rgba_unmultiplied(255, 70, 70, 45));
                                                        painter.rect_stroke(track_rect, 0.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(255, 70, 70)), egui::StrokeKind::Inside);
                                                    }
                                                }
                                            }
                                        }

                                        // Lasso selection drawing
                                        if let Some(lasso_rect) = self.app.lasso_rect {
                                            painter.rect(
                                                lasso_rect,
                                                2.0,
                                                egui::Color32::from_rgba_unmultiplied(52, 152, 219, 30),
                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(52, 152, 219)),
                                                egui::StrokeKind::Inside,
                                            );
                                        }

                                        ((rect, response), clicked_any_clip, clicked_any_keyframe)
                                    })
                            });

                            let ((rect, response), clicked_any_clip, clicked_any_keyframe) = drop_res.0.inner.inner;

                            let tracks_top = rect.min.y + 26.0;

                            // Successful drop logic
                            if let Some(payload) = &drop_res.1 {
                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                    if rect.contains(mouse_pos) {
                                        let relative_y = mouse_pos.y - tracks_top;
                                        let visible_row = (relative_y / 32.0).floor() as i32;
                                        let relative_x = mouse_pos.x - rect.min.x;
                                        let drop_time_secs = (relative_x / zoom) as f64;

                                        self.app.handle_effect_drop(payload, visible_row, drop_time_secs);
                                    }
                                }
                            }

                            // Background click, seek, or lasso selection logic
                            let ruler_bottom = tracks_top;

                            if response.drag_started() && !clicked_any_clip && !clicked_any_keyframe && self.app.active_drag.is_none() && self.app.active_keyframe_drag.is_none() {
                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                    if mouse_pos.y >= ruler_bottom {
                                        self.app.lasso_origin = Some(mouse_pos);
                                    }
                                }
                            }

                            if response.dragged() && self.app.lasso_origin.is_some() {
                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                    let origin = self.app.lasso_origin.unwrap();
                                    let lasso_rect = egui::Rect::from_two_pos(origin, mouse_pos);
                                    self.app.lasso_rect = Some(lasso_rect);

                                    let shift_held = ui.ctx().input(|i| i.modifiers.shift);
                                    let mut new_instance_selection = if shift_held { self.app.selected_instance_ids.clone() } else { std::collections::HashSet::new() };
                                    let mut new_keyframe_selection = if shift_held { self.app.selected_keyframes.clone() } else { std::collections::HashSet::new() };

                                    // Instances
                                    for instance in &self.app.timeline.instances {
                                        if let Some(effect) = self.app.timeline.templates.iter().find(|t| t.id == instance.effect_id) {
                                            let Some(relay_id) = effect.actions.first().map(|a| a.relay_id) else {
                                                continue;
                                            };
                                            let Some(track_index) = timeline_row_for_relay(&timeline_rows, relay_id) else {
                                                continue;
                                            };
                                            let track_y = tracks_top + track_index as f32 * 32.0;
                                            let start_x = rect.min.x + (instance.start_time_ms as f32 * px_per_ms);
                                            let end_x = start_x + (effect.duration_ms as f32 * px_per_ms);
                                            let clip_rect = egui::Rect::from_min_max(
                                                egui::pos2(start_x, track_y + 4.0),
                                                egui::pos2(end_x, track_y + 28.0),
                                            );
                                            if lasso_rect.intersects(clip_rect) {
                                                new_instance_selection.insert(instance.id);
                                            }
                                        }
                                    }

                                    // Keyframes
                                    for (t_idx, track) in self.app.timeline.analog_tracks.iter().enumerate() {
                                        let t_y = tracks_top + track_area_height + (t_idx as f32 * 40.0);
                                        for (k_idx, kf) in track.keyframes.iter().enumerate() {
                                            let k_x = rect.min.x + (kf.time_ms as f32 * px_per_ms);
                                            let k_y = (t_y + 36.0) - (kf.value * 32.0);
                                            let k_pos = egui::pos2(k_x, k_y);
                                            if lasso_rect.contains(k_pos) {
                                                new_keyframe_selection.insert((track.id, k_idx));
                                            }
                                        }
                                    }

                                    self.app.selected_instance_ids = new_instance_selection;
                                    self.app.selected_keyframes = new_keyframe_selection;
                                }
                            }

                            let mut lasso_ended = false;
                            if ui.ctx().input(|i| i.pointer.any_released()) {
                                lasso_ended = true;
                            }

                            if lasso_ended {
                                self.app.lasso_origin = None;
                                self.app.lasso_rect = None;
                            }

                            if response.clicked() && !clicked_any_clip && !clicked_any_keyframe && self.app.active_drag.is_none() && self.app.active_keyframe_drag.is_none() {
                                if let Some(mouse_pos) = response.interact_pointer_pos() {
                                    if mouse_pos.y >= tracks_top && mouse_pos.y < tracks_top + track_area_height {
                                        self.app.selected_instance_ids.clear();
                                        let relative_x = mouse_pos.x - rect.min.x;
                                        let seek_time = (relative_x / zoom) as f64;
                                        let target_time = seek_time.clamp(0.0, total_seconds);
                                        // Punch out on seek
                                        self.app.commit_recorded_samples();

                                        let _ = self.app.mpv.command("seek", &[&target_time.to_string(), "absolute"]);
                                    }
                                }
                            }

                            response.context_menu(|ui| {
                                ui.label(egui::RichText::new(self.app.tr("Timeline")).strong());
                                ui.separator();
                                if ui.button(format!("{} {}", crate::ui::icons::SELECTION_ALL, self.app.tr("Select all cues"))).clicked() {
                                    self.app.selected_instance_ids = self.app.timeline.instances.iter().map(|instance| instance.id).collect();
                                    self.app.selected_keyframes.clear();
                                    for track in &self.app.timeline.analog_tracks {
                                        for (index, _) in track.keyframes.iter().enumerate() {
                                            self.app.selected_keyframes.insert((track.id, index));
                                        }
                                    }
                                    ui.close();
                                }
                                if ui.button(format!("{} {}", crate::ui::icons::ERASER, self.app.tr("Clear selection"))).clicked() {
                                    self.app.selected_instance_ids.clear();
                                    self.app.selected_keyframes.clear();
                                    ui.close();
                                }
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label(format!("{} {}", crate::ui::icons::MAGNIFYING_GLASS, self.app.tr("Zoom")));
                                    ui.add(egui::Slider::new(&mut self.app.timeline_zoom, 20.0..=500.0).suffix(" px/s"));
                                });
                                if ui.button(format!("{} {}", crate::ui::icons::ARROW_COUNTER_CLOCKWISE, self.app.tr("Reset zoom"))).clicked() {
                                    self.app.timeline_zoom = 100.0;
                                    ui.close();
                                }
                                ui.separator();
                                if ui.button(format!("{} {}", crate::ui::icons::GEAR, self.app.tr("Preferences..."))).clicked() {
                                    self.app.show_preferences_dialog = true;
                                    ui.close();
                                }
                            });

                            if !ui.ctx().egui_wants_keyboard_input() {
                                let delete_pressed = ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace));
                                let undo_pressed = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift);
                                let redo_pressed = ui.input(|i| (i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) || (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z)));
                                let select_all_pressed = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::A));
                                let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

                                let num_modifier_free = ui.input(|i| !i.modifiers.ctrl && !i.modifiers.alt && !i.modifiers.command && !i.modifiers.shift);
                                let key_1_pressed = ui.input(|i| i.key_pressed(egui::Key::Num1)) && num_modifier_free;
                                let key_2_pressed = ui.input(|i| i.key_pressed(egui::Key::Num2)) && num_modifier_free;
                                let key_3_pressed = ui.input(|i| i.key_pressed(egui::Key::Num3)) && num_modifier_free;

                                if undo_pressed {
                                    let current = self.app.snapshot_timeline();
                                    if let Some(prev) = self.app.undo_stack.undo(current) {
                                        self.app.restore_timeline_snapshot(prev);
                                        self.app.selected_instance_ids.clear();
                                        self.app.selected_keyframes.clear();
                                        self.app.sync_timeline_engine();
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.timeline.analog_tracks.clone()));
                                    }
                                } else if redo_pressed {
                                    let current = self.app.snapshot_timeline();
                                    if let Some(next) = self.app.undo_stack.redo(current) {
                                        self.app.restore_timeline_snapshot(next);
                                        self.app.selected_instance_ids.clear();
                                        self.app.selected_keyframes.clear();
                                        let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.timeline.analog_tracks.clone()));
                                    }
                                } else if select_all_pressed {
                                    self.app.selected_instance_ids = self.app.timeline.instances.iter().map(|i| i.id).collect();
                                    self.app.selected_keyframes.clear();
                                    for track in &self.app.timeline.analog_tracks {
                                        for (k_idx, _) in track.keyframes.iter().enumerate() {
                                            self.app.selected_keyframes.insert((track.id, k_idx));
                                        }
                                    }
                                } else if escape_pressed {
                                    self.app.selected_instance_ids.clear();
                                    self.app.selected_keyframes.clear();
                                } else if delete_pressed {
                                    if !self.app.selected_instance_ids.is_empty() || !self.app.selected_keyframes.is_empty() {
                                        self.app.undo_stack.push(self.app.snapshot_timeline());

                                        self.app.timeline.instances.retain(|inst| !self.app.selected_instance_ids.contains(&inst.id));

                                        for track in &mut self.app.timeline.analog_tracks {
                                            let mut k_indices: Vec<usize> = self.app.selected_keyframes.iter()
                                                .filter(|(t_id, _)| *t_id == track.id)
                                                .map(|(_, k_idx)| *k_idx)
                                                .collect();
                                            k_indices.sort_unstable();
                                            for idx in k_indices.into_iter().rev() {
                                                if idx < track.keyframes.len() {
                                                    track.keyframes.remove(idx);
                                                }
                                            }
                                        }

                                        self.app.selected_instance_ids.clear();
                                        self.app.selected_keyframes.clear();

                                        let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.timeline.analog_tracks.clone()));
                                    }
                                } else if key_1_pressed || key_2_pressed || key_3_pressed {
                                    if !self.app.selected_keyframes.is_empty() {
                                        self.app.undo_stack.push(self.app.snapshot_timeline());
                                        let interp = if key_1_pressed {
                                            crate::four_d::curve::Interpolation::Step
                                        } else if key_2_pressed {
                                            crate::four_d::curve::Interpolation::Linear
                                        } else {
                                            crate::four_d::curve::Interpolation::Smooth
                                        };
                                        for track in &mut self.app.timeline.analog_tracks {
                                            for (k_idx, kf) in track.keyframes.iter_mut().enumerate() {
                                                if self.app.selected_keyframes.contains(&(track.id, k_idx)) {
                                                    kf.interpolation = interp.clone();
                                                }
                                            }
                                        }
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.timeline.analog_tracks.clone()));
                                    }
                                }
                            }
                        });
                    }
                }
              });
            });
    }
}

/// Helper function to build the initial DockState layout
pub fn create_initial_layout() -> egui_dock::DockState<PealayerTab> {
    use egui_dock::NodeIndex;

    // Start with a Timeline tab as the initial tab in the root node
    let mut dock_state = egui_dock::DockState::new(vec![PealayerTab::Timeline]);

    // Split the root node: split_above creates a top node (70% height) for monitors,
    // leaving the Timeline at the bottom (30% height).
    let [_timeline_node, top_node] = dock_state.main_surface_mut().split_above(
        NodeIndex::root(),
        0.7,
        vec![PealayerTab::ProgramMonitor],
    );

    // Split the top node horizontally: split_left creates a left column (25% width)
    // for Effect Controls and Hardware Monitor, leaving Program Monitor in the center/right.
    let [center_node, _left_node] = dock_state.main_surface_mut().split_left(
        top_node,
        0.25,
        vec![PealayerTab::EffectControls, PealayerTab::HardwareMonitor],
    );

    // Split the remaining center node horizontally: split_right creates a right column
    // (30% of the remaining width, ~22.5% of total width) for Effects Library,
    // leaving the Program Monitor in the center (~52.5% width).
    let [_center_node, _right_node] = dock_state.main_surface_mut().split_right(
        center_node,
        0.7,
        vec![PealayerTab::EffectsLibrary],
    );

    sanitize_dock_rects(&mut dock_state);

    dock_state
}

/// Sanitizes all node rectangles and viewports in a `DockState` to finite values (`Rect::ZERO`),
/// preventing non-finite floats (e.g. `Rect::NOTHING` where Pos2 is +/-INFINITY) from serializing
/// as `null` in JSON formats such as `serde_json`.
pub fn sanitize_dock_rects<Tab>(dock_state: &mut egui_dock::DockState<Tab>) {
    for (_path, node) in dock_state.iter_all_nodes_mut() {
        if let Some(rect) = node.rect()
            && !rect.is_finite()
        {
            node.set_rect(egui::Rect::ZERO);
        }
        if let Some(leaf) = node.get_leaf_mut()
            && !leaf.viewport.is_finite()
        {
            leaf.viewport = egui::Rect::ZERO;
        }
    }
}

/// Restores a tab to its canonical dock location if it is closed, respecting sibling groupings and anchors.
pub fn restore_tab_to_canonical_slot(
    dock_state: &mut egui_dock::DockState<PealayerTab>,
    tab: PealayerTab,
) {
    if dock_state.find_tab(&tab).is_some() {
        return;
    }

    if dock_state.iter_all_tabs().count() == 0 {
        *dock_state = egui_dock::DockState::new(vec![tab]);
        sanitize_dock_rects(dock_state);
        return;
    }

    match tab {
        PealayerTab::EffectControls => {
            if let Some(sibling_path) = dock_state.find_tab(&PealayerTab::HardwareMonitor) {
                let node_path = sibling_path.node_path();
                if let Ok(leaf) = dock_state.leaf_mut(node_path) {
                    leaf.tabs.push(tab);
                    sanitize_dock_rects(dock_state);
                    return;
                }
            }
            if let Some(anchor_path) = dock_state
                .find_tab(&PealayerTab::ProgramMonitor)
                .or_else(|| dock_state.find_tab(&PealayerTab::Timeline))
            {
                let node_index = anchor_path.node;
                dock_state
                    .main_surface_mut()
                    .split_left(node_index, 0.25, vec![tab]);
                sanitize_dock_rects(dock_state);
                return;
            }
        }
        PealayerTab::HardwareMonitor => {
            if let Some(sibling_path) = dock_state.find_tab(&PealayerTab::EffectControls) {
                let node_path = sibling_path.node_path();
                if let Ok(leaf) = dock_state.leaf_mut(node_path) {
                    leaf.tabs.push(tab);
                    sanitize_dock_rects(dock_state);
                    return;
                }
            }
            if let Some(anchor_path) = dock_state
                .find_tab(&PealayerTab::ProgramMonitor)
                .or_else(|| dock_state.find_tab(&PealayerTab::Timeline))
            {
                let node_index = anchor_path.node;
                dock_state
                    .main_surface_mut()
                    .split_left(node_index, 0.25, vec![tab]);
                sanitize_dock_rects(dock_state);
                return;
            }
        }
        PealayerTab::Timeline => {
            if let Some(anchor_path) = dock_state
                .find_tab(&PealayerTab::ProgramMonitor)
                .or_else(|| dock_state.find_tab(&PealayerTab::EffectControls))
                .or_else(|| dock_state.find_tab(&PealayerTab::EffectsLibrary))
                .or_else(|| dock_state.find_tab(&PealayerTab::HardwareMonitor))
            {
                let node_index = anchor_path.node;
                dock_state
                    .main_surface_mut()
                    .split_below(node_index, 0.7, vec![tab]);
                sanitize_dock_rects(dock_state);
                return;
            }
        }
        PealayerTab::EffectsLibrary => {
            if let Some(anchor_path) = dock_state
                .find_tab(&PealayerTab::ProgramMonitor)
                .or_else(|| dock_state.find_tab(&PealayerTab::Timeline))
            {
                let node_index = anchor_path.node;
                dock_state
                    .main_surface_mut()
                    .split_right(node_index, 0.75, vec![tab]);
                sanitize_dock_rects(dock_state);
                return;
            }
        }
        PealayerTab::ProgramMonitor => {
            if let Some(anchor_path) = dock_state.find_tab(&PealayerTab::Timeline) {
                let node_index = anchor_path.node;
                dock_state
                    .main_surface_mut()
                    .split_above(node_index, 0.7, vec![tab]);
                sanitize_dock_rects(dock_state);
                return;
            }
            if let Some(anchor_path) = dock_state
                .find_tab(&PealayerTab::EffectControls)
                .or_else(|| dock_state.find_tab(&PealayerTab::HardwareMonitor))
            {
                let node_index = anchor_path.node;
                dock_state
                    .main_surface_mut()
                    .split_right(node_index, 0.5, vec![tab]);
                sanitize_dock_rects(dock_state);
                return;
            }
        }
    }

    dock_state.push_to_first_leaf(tab);
    sanitize_dock_rects(dock_state);
}

fn format_timecode(t: f64) -> String {
    let secs = t.floor() as i64;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    let f = ((t - t.floor()) * 24.0).round() as i64;
    format!("{:02}:{:02}:{:02}:{:02}", h, m, s, f)
}

impl PealayerApp {
    /// Locks or unlocks a capability-advertised output track.
    pub fn lock_track(&mut self, relay_id: u8, locked: bool) {
        if locked {
            self.track_locked.insert(relay_id);
        } else {
            self.track_locked.remove(&relay_id);
        }
    }

    /// Returns the latest OSD message text, if any.
    pub fn last_osd_message(&self) -> Option<String> {
        self.osd_message.as_ref().map(|(msg, _)| msg.clone())
    }

    /// Handles dropping an effect payload onto the timeline canvas.
    ///
    /// - Case A: Dropped on a capability-advertised relay track:
    ///   Validates track lock and target compatibility with `payload.target`.
    ///   Rejects drop with warning message and OSD update if incompatible or locked.
    /// - Case B: Dropped on empty grid space: auto-routes only when the payload's
    ///   exact output is currently advertised and unlocked. Media rows reject.
    ///
    /// Returns true if a clip instance was successfully created.
    pub fn handle_effect_drop(
        &mut self,
        payload: &EffectDragPayload,
        track_index: i32,
        drop_time_secs: f64,
    ) -> bool {
        let timeline_rows = timeline_track_rows(self);
        if payload.controller_macro.is_some() || payload.controller_strip_effect.is_some() {
            let valid_track = usize::try_from(track_index)
                .ok()
                .and_then(|index| timeline_rows.get(index))
                .is_some_and(|row| row.kind == TimelineTrackKind::ControllerMacros);
            if !valid_track {
                self.set_osd(format!(
                    "Place '{}' on the Controller effects track",
                    payload.name
                ));
                return false;
            }
            let mut snapped_secs = drop_time_secs;
            if (snapped_secs - self.playback_time).abs() < 0.15 {
                snapped_secs = self.playback_time;
            } else {
                snapped_secs = (snapped_secs * 10.0).round() / 10.0;
            }
            self.undo_stack.push(self.snapshot_timeline());
            let template_id = if let Some(existing) =
                self.timeline.templates.iter().find(|effect| {
                    effect.controller_macro == payload.controller_macro
                        && effect.controller_strip_effect == payload.controller_strip_effect
                }) {
                existing.id
            } else {
                let effect = if let Some(controller_macro) = payload.controller_macro.as_ref() {
                    crate::four_d::models::Effect::controller_macro(
                        payload.name.clone(),
                        payload.icon.clone(),
                        payload.duration_ms,
                        controller_macro.id,
                        controller_macro.mode.clone(),
                    )
                } else {
                    crate::four_d::models::Effect::controller_strip_effect(
                        payload.name.clone(),
                        payload.duration_ms,
                        payload
                            .controller_strip_effect
                            .as_ref()
                            .expect("controller effect payload has one durable reference")
                            .id
                            .clone(),
                    )
                };
                let id = effect.id;
                self.timeline.templates.push(effect);
                id
            };
            let instance = crate::four_d::models::EffectInstance::new(
                template_id,
                (snapped_secs.max(0.0) * 1000.0) as u64,
            );
            self.selected_instance_ids.clear();
            self.selected_instance_ids.insert(instance.id);
            self.timeline.instances.push(instance);
            self.sync_timeline_engine();
            return true;
        }
        let target_relay = if let Some(track_row) = usize::try_from(track_index)
            .ok()
            .and_then(|index| timeline_rows.get(index))
        {
            let TimelineTrackKind::Relay(relay) = track_row.kind else {
                self.set_osd(format!(
                    "Cannot place effect '{}' on a media track",
                    payload.name
                ));
                return false;
            };
            if self.track_locked.contains(&relay) {
                self.set_osd(format!(
                    "Cannot place '{}': {} is locked",
                    payload.name, track_row.name
                ));
                return false;
            }
            if !payload.target.is_compatible_with_relay(relay) {
                self.set_osd(format!(
                    "Placement rejected: '{}' targets a different output",
                    payload.name
                ));
                return false;
            }
            relay
        } else {
            if let Some(primary) = payload.target.primary_relay_id() {
                let Some(target_row) = timeline_rows
                    .iter()
                    .find(|row| row.kind == TimelineTrackKind::Relay(primary))
                else {
                    self.set_osd(format!(
                        "Cannot place '{}': its output is not currently available",
                        payload.name
                    ));
                    return false;
                };
                if self.track_locked.contains(&primary) {
                    self.set_osd(format!(
                        "Cannot place '{}': {} is locked",
                        payload.name, target_row.name
                    ));
                    return false;
                }
                self.set_osd(format!("Placed '{}' on {}", payload.name, target_row.name));
                primary
            } else {
                return false;
            }
        };

        let mut snapped_secs = drop_time_secs;
        // Playhead/grid snapping
        if (snapped_secs - self.playback_time).abs() < 0.15 {
            snapped_secs = self.playback_time;
        } else {
            snapped_secs = (snapped_secs * 10.0).round() / 10.0;
        }
        let start_time_ms = (snapped_secs.max(0.0) * 1000.0) as u64;

        // Record undo snapshot before mutating timeline
        self.undo_stack.push(self.snapshot_timeline());

        // Check or create template targeting target_relay with payload.target
        let template_id = if let Some(existing) = self.timeline.templates.iter().find(|t| {
            t.name == payload.name
                && t.duration_ms == payload.duration_ms
                && t.target == payload.target
                && t.actions.first().map(|a| a.relay_id) == Some(target_relay)
        }) {
            existing.id
        } else {
            let actions = if !payload.actions.is_empty() {
                let mut acts = payload.actions.clone();
                for a in &mut acts {
                    a.relay_id = target_relay;
                }
                acts
            } else {
                crate::four_d::patterns::generate_constant(target_relay, true, payload.duration_ms)
            };
            let new_effect = crate::four_d::models::Effect::with_target(
                payload.name.clone(),
                payload.icon.clone(),
                payload.duration_ms,
                payload.target,
                actions,
            );
            let id = new_effect.id;
            self.timeline.templates.push(new_effect);
            id
        };

        // Instantiate and select
        let new_instance = crate::four_d::models::EffectInstance::new(template_id, start_time_ms);
        let new_instance_id = new_instance.id;
        self.timeline.instances.push(new_instance);
        self.selected_instance_ids.clear();
        self.selected_instance_ids.insert(new_instance_id);
        self.selected_keyframes.clear();

        // Recompile timeline
        self.sync_timeline_engine();

        true
    }

    /// 1-Click Relocation: Automatically reassigns an effect's template actions to its primary relay track,
    /// pushes an undo snapshot to the undo stack, recompiles the timeline, and updates the engine queue.
    /// Preserves internal pattern sequences (e.g. pulsing) while updating the target relay.
    /// Returns true if relocation was successfully performed.
    pub fn relocate_effect_to_primary(&mut self, effect_id: uuid::Uuid) -> bool {
        let (primary, duration_ms, effect_name) =
            if let Some(tmpl) = self.timeline.templates.iter().find(|t| t.id == effect_id) {
                if let Some(primary) = tmpl.target.primary_relay_id() {
                    (primary, tmpl.duration_ms, tmpl.name.clone())
                } else {
                    return false;
                }
            } else {
                return false;
            };
        let Some(display_name) = self
            .advertised_hardware()
            .filter(|capabilities| capabilities.board_connected)
            .and_then(|capabilities| {
                capabilities
                    .relays
                    .into_iter()
                    .find(|relay| relay.id == primary)
            })
            .map(|relay| relay.name)
        else {
            return false;
        };

        // Record undo snapshot before mutating timeline
        self.undo_stack.push(self.snapshot_timeline());

        if let Some(t) = self
            .timeline
            .templates
            .iter_mut()
            .find(|t| t.id == effect_id)
        {
            if t.actions.is_empty() {
                t.actions = crate::four_d::patterns::generate_constant(primary, true, duration_ms);
            } else {
                for a in &mut t.actions {
                    a.relay_id = primary;
                }
            }
        }

        let compiled = crate::four_d::engine::compile_timeline(
            &self.timeline,
            &self.track_muted,
            &self.track_soloed,
        );
        let _ = self
            .engine_handle
            .sender
            .send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));

        self.set_osd(format!("Relocated '{}' to {}", effect_name, display_name));
        true
    }
}

fn render_clip_handles(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    left_active: bool,
    right_active: bool,
    alpha: u8,
) {
    if clip_rect.width() < 14.0 {
        return;
    }
    let handle_w = (clip_rect.width() * 0.35).min(10.0);
    let left_handle_rect = egui::Rect::from_min_max(
        clip_rect.left_top(),
        egui::pos2(clip_rect.left() + handle_w, clip_rect.bottom()),
    );
    let right_handle_rect = egui::Rect::from_min_max(
        egui::pos2(clip_rect.right() - handle_w, clip_rect.top()),
        clip_rect.right_bottom(),
    );

    let alpha_scale = alpha as f32 / 255.0;
    let cyan = egui::Color32::from_rgb(0, 220, 255);

    // Left handle highlight & edge
    if left_active {
        painter.rect_filled(
            left_handle_rect,
            egui::CornerRadius {
                nw: 4,
                sw: 4,
                ne: 0,
                se: 0,
            },
            egui::Color32::from_rgba_unmultiplied(0, 220, 255, (50.0 * alpha_scale) as u8),
        );
        painter.line_segment(
            [
                clip_rect.left_top() + egui::vec2(1.0, 1.0),
                egui::pos2(clip_rect.left() + 1.0, clip_rect.bottom() - 1.0),
            ],
            egui::Stroke::new(
                2.0_f32,
                egui::Color32::from_rgba_unmultiplied(cyan.r(), cyan.g(), cyan.b(), alpha),
            ),
        );
    }

    // Right handle highlight & edge
    if right_active {
        painter.rect_filled(
            right_handle_rect,
            egui::CornerRadius {
                nw: 0,
                sw: 0,
                ne: 4,
                se: 4,
            },
            egui::Color32::from_rgba_unmultiplied(0, 220, 255, (50.0 * alpha_scale) as u8),
        );
        painter.line_segment(
            [
                egui::pos2(clip_rect.right() - 1.0, clip_rect.top() + 1.0),
                egui::pos2(clip_rect.right() - 1.0, clip_rect.bottom() - 1.0),
            ],
            egui::Stroke::new(
                2.0_f32,
                egui::Color32::from_rgba_unmultiplied(cyan.r(), cyan.g(), cyan.b(), alpha),
            ),
        );
    }

    // Draw grip affordance notches (2 subtle vertical lines in center of each handle)
    let notch_y_top = clip_rect.top() + 5.0;
    let notch_y_bot = clip_rect.bottom() - 5.0;
    if notch_y_bot > notch_y_top {
        // Left handle grip notches
        let left_cx = left_handle_rect.center().x;
        let left_notch_color = if left_active {
            egui::Color32::from_rgba_unmultiplied(cyan.r(), cyan.g(), cyan.b(), alpha)
        } else {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, (70.0 * alpha_scale) as u8)
        };
        painter.line_segment(
            [
                egui::pos2(left_cx - 1.5, notch_y_top),
                egui::pos2(left_cx - 1.5, notch_y_bot),
            ],
            egui::Stroke::new(1.0_f32, left_notch_color),
        );
        painter.line_segment(
            [
                egui::pos2(left_cx + 1.5, notch_y_top),
                egui::pos2(left_cx + 1.5, notch_y_bot),
            ],
            egui::Stroke::new(1.0_f32, left_notch_color),
        );

        // Right handle grip notches
        let right_cx = right_handle_rect.center().x;
        let right_notch_color = if right_active {
            egui::Color32::from_rgba_unmultiplied(cyan.r(), cyan.g(), cyan.b(), alpha)
        } else {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, (70.0 * alpha_scale) as u8)
        };
        painter.line_segment(
            [
                egui::pos2(right_cx - 1.5, notch_y_top),
                egui::pos2(right_cx - 1.5, notch_y_bot),
            ],
            egui::Stroke::new(1.0_f32, right_notch_color),
        );
        painter.line_segment(
            [
                egui::pos2(right_cx + 1.5, notch_y_top),
                egui::pos2(right_cx + 1.5, notch_y_bot),
            ],
            egui::Stroke::new(1.0_f32, right_notch_color),
        );
    }
}
