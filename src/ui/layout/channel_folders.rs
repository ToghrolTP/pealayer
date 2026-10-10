use super::*;
use crate::four_d::controller::{HardwareCapabilities, HardwareControl};

pub(super) fn kind(value: &str) -> &'static str {
    match value {
        "seat" | "side" | "motion" => "motion",
        "relay" => "relay",
        "pwm" | "mosfet" => "pwm",
        _ => "board",
    }
}

pub(crate) fn channel_folder_names<'a>(
    capabilities: &'a HardwareCapabilities,
    control_kind: &str,
) -> Vec<&'a str> {
    let section = kind(control_kind);
    capabilities
        .channel_folders
        .iter()
        .filter(|folder| folder.kind == section)
        .map(|folder| folder.name.as_str())
        .chain(
            capabilities
                .controls
                .iter()
                .filter(|control| kind(&control.kind) == section)
                .map(|control| control.group.as_str()),
        )
        .filter(|name| !name.is_empty())
        .collect()
}

pub(super) fn control_move_menu(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities,
    control: &HardwareControl,
) {
    if raw_relay(capabilities, control) {
        return;
    }
    move_menu(
        app,
        ui,
        capabilities,
        kind(&control.kind),
        &control.group,
        &[control.key.clone()],
    );
}

pub(crate) fn raw_relay(capabilities: &HardwareCapabilities, control: &HardwareControl) -> bool {
    capabilities.relays.iter().any(|output| {
        output.key == control.key
            && output.id <= 4
            && (output.control == "seat-internal"
                || matches!(output.role.as_str(), "motion-direction" | "motion-enable"))
    })
}

pub(super) struct FolderGroup<'a> {
    pub name: String,
    pub raw: bool,
    pub members: Vec<&'a HardwareControl>,
}

pub(super) fn groups<'a>(
    app: &PealayerApp,
    capabilities: &HardwareCapabilities,
    controls: &'a [HardwareControl],
) -> Vec<FolderGroup<'a>> {
    let section = controls
        .first()
        .map(|control| kind(&control.kind))
        .unwrap_or("board");
    let mut groups = capabilities
        .channel_folders
        .iter()
        .filter(|folder| folder.kind == section)
        .map(|folder| FolderGroup {
            name: folder.name.clone(),
            raw: false,
            members: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut ordered = controls
        .iter()
        .filter(|control| !control.hidden && global_control_is_visible(app, capabilities, control))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|control| (control.order, &control.key));
    for control in ordered {
        let raw = raw_relay(capabilities, control);
        let name = if raw {
            "Raw relays"
        } else {
            control.group.trim()
        };
        if let Some(group) = groups
            .iter_mut()
            .find(|group| group.raw == raw && group.name.eq_ignore_ascii_case(name))
        {
            group.members.push(control);
        } else {
            groups.push(FolderGroup {
                name: name.to_string(),
                raw,
                members: vec![control],
            });
        }
    }
    // Named folders never visually swallow the ungrouped channels that follow.
    if groups
        .iter()
        .any(|group| !group.raw && !group.name.is_empty())
        && !groups.iter().any(|group| group.name.is_empty())
    {
        groups.push(FolderGroup {
            name: String::new(),
            raw: false,
            members: Vec::new(),
        });
    }
    groups.sort_by_key(|group| (group.name.is_empty(), !group.raw, group.name.to_lowercase()));
    groups
}

pub(crate) fn request_channel_folder_update(
    app: &mut PealayerApp,
    capabilities: &HardwareCapabilities,
    mut fields: serde_json::Value,
) -> Result<(), String> {
    if app.channel_folder_pending {
        return Err(app.tr("A folder change is still pending"));
    }
    let profile = capabilities
        .board_profile
        .as_ref()
        .filter(|profile| profile.attached && profile.configured)
        .ok_or_else(|| app.tr("Configure the attached board before editing folders"))?;
    let params = fields.as_object_mut().ok_or("Invalid folder operation")?;
    params.insert(
        "expected_revision".into(),
        serde_json::json!(profile.revision),
    );
    app.engine_handle.request_controller_call(
        format!(
            "presentation-folder:{}",
            params
                .get("kind")
                .and_then(|value| value.as_str())
                .unwrap_or("board")
        ),
        "controller.peripheral.folder.update",
        fields,
    )?;
    app.channel_folder_result = None;
    app.channel_folder_sequence = app.channel_folder_sequence.saturating_add(1);
    app.channel_folder_error = None;
    app.channel_folder_pending = true;
    Ok(())
}

fn update(app: &mut PealayerApp, capabilities: &HardwareCapabilities, fields: serde_json::Value) {
    if let Err(error) = request_channel_folder_update(app, capabilities, fields) {
        app.set_osd(error);
    }
}

fn folder_id(
    capabilities: &HardwareCapabilities,
    section: &str,
    name: &str,
    raw: bool,
) -> egui::Id {
    egui::Id::new((
        "channel-folder-open",
        capabilities
            .board_profile
            .as_ref()
            .map(|profile| &profile.board_identity),
        section,
        name,
        raw,
    ))
}

#[derive(Clone, Default)]
struct FolderDraft {
    board_identity: String,
    kind: String,
    original: Option<String>,
    name: String,
    icon: String,
    error: String,
    focus: bool,
    delete: bool,
    pending: bool,
}
fn editor_id() -> egui::Id {
    egui::Id::new("channel-folder-editor-state")
}
fn manager_id() -> egui::Id {
    egui::Id::new("channel-folder-manager-state")
}

fn edit(ui: &egui::Ui, capabilities: &HardwareCapabilities, section: &str, name: Option<&str>) {
    let folder = capabilities
        .channel_folders
        .iter()
        .find(|folder| folder.kind == section && Some(folder.name.as_str()) == name);
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            editor_id(),
            FolderDraft {
                board_identity: capabilities
                    .board_profile
                    .as_ref()
                    .map(|profile| profile.board_identity.clone())
                    .unwrap_or_default(),
                kind: section.into(),
                original: name.map(str::to_owned),
                name: name.unwrap_or_default().into(),
                icon: folder.map(|folder| folder.icon.clone()).unwrap_or_default(),
                error: String::new(),
                focus: true,
                delete: false,
                pending: false,
            },
        )
    });
}

pub(super) fn section_menu(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    response: &egui::Response,
    capabilities: &HardwareCapabilities,
    controls: &[HardwareControl],
) {
    let section = controls
        .first()
        .map(|control| kind(&control.kind))
        .unwrap_or("board");
    let open = secondary_click_inside(ui.ctx(), response.rect);
    egui::Popup::menu(response)
        .id(response.id.with("section-context"))
        .at_pointer_fixed()
        .open_memory(open.then_some(egui::SetOpenCommand::Bool(true)))
        .show(|ui| {
            if ui.button(app.tr("Manage channels")).clicked() {
                app.show_hardware_channels_dialog = true;
                app.hardware_channel_detail_active = false;
                ui.close();
            }
            if ui.button(app.tr("Manage folders...")).clicked() {
                ui.ctx()
                    .data_mut(|data| data.insert_temp(manager_id(), section.to_string()));
                ui.close();
            }
            if ui.button(app.tr("New folder...")).clicked() {
                edit(ui, capabilities, section, None);
                ui.close();
            }
            if !capabilities
                .channel_folders
                .iter()
                .any(|folder| folder.kind == section)
            {
                return;
            }
            ui.separator();
            for (label, open) in [("Expand folders", true), ("Collapse folders", false)] {
                if ui.button(app.tr(label)).clicked() {
                    for group in groups(app, capabilities, controls) {
                        let id = folder_id(capabilities, section, &group.name, group.raw);
                        ui.ctx().data_mut(|data| data.insert_persisted(id, open));
                    }
                    ui.close();
                }
            }
        });
}

fn move_menu(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities,
    section: &str,
    current: &str,
    keys: &[String],
) {
    if keys.is_empty() {
        return;
    }
    ui.menu_button(app.tr("Move to folder"), |ui| {
        for name in std::iter::once("").chain(
            capabilities
                .channel_folders
                .iter()
                .filter(|folder| folder.kind == section)
                .map(|folder| folder.name.as_str()),
        ) {
            if name == current {
                continue;
            }
            let label = if name.is_empty() {
                app.tr("Ungrouped")
            } else {
                name.to_string()
            };
            if ui.button(label).clicked() {
                update(
                    app,
                    capabilities,
                    serde_json::json!({"operation":"move","kind":section,"name":name,"keys":keys}),
                );
                ui.close();
            }
        }
    });
}

pub(super) fn draw_header(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities,
    section: &str,
    group: &FolderGroup<'_>,
) -> bool {
    let id = folder_id(capabilities, section, &group.name, group.raw);
    let mut open = ui
        .ctx()
        .data_mut(|data| data.get_persisted::<bool>(id).unwrap_or(true));
    let folder = capabilities
        .channel_folders
        .iter()
        .find(|folder| folder.kind == section && folder.name == group.name);
    let icon = folder
        .filter(|folder| !folder.icon.is_empty())
        .map(|folder| crate::ui::icons::control("folder", &folder.icon))
        .unwrap_or(crate::ui::icons::FOLDER_OPEN);
    let label = if group.name.is_empty() {
        app.tr("Ungrouped")
    } else if group.raw {
        app.tr("Raw relays")
    } else {
        crate::ui::i18n::visual_text(app.language, &group.name)
    };
    let header = ui.horizontal(|ui| {
        let caret = if group.members.is_empty() {
            ""
        } else if open {
            crate::ui::icons::CARET_DOWN
        } else {
            crate::ui::icons::CARET_RIGHT
        };
        let text = egui::RichText::new(format!("{caret} {icon} {label} ({})", group.members.len()))
            .small()
            .strong();
        let response = ui.add(
            egui::Button::new(if group.members.is_empty() {
                text.weak()
            } else {
                text
            })
            .frame(false),
        );
        if response.clicked() && !group.members.is_empty() {
            open = !open;
        }
        let (line, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().line_segment(
            [line.left_center(), line.right_center()],
            ui.visuals().widgets.noninteractive.bg_stroke,
        );
    });
    ui.ctx().data_mut(|data| data.insert_persisted(id, open));
    let response = &header.response;
    if !group.raw {
        folder_drop_target(app, ui, capabilities, response.rect, section, &group.name);
        let popup = secondary_click_inside(ui.ctx(), response.rect);
        egui::Popup::menu(response)
            .id(id.with("context"))
            .at_pointer_fixed()
            .open_memory(popup.then_some(egui::SetOpenCommand::Bool(true)))
            .show(|ui| {
                if ui.button(app.tr("New folder...")).clicked() {
                    edit(ui, capabilities, section, None);
                    ui.close();
                }
                if !group.name.is_empty() && ui.button(app.tr("Manage folder...")).clicked() {
                    edit(ui, capabilities, section, Some(&group.name));
                    ui.close();
                }
                let keys = capabilities
                    .controls
                    .iter()
                    .filter(|control| {
                        kind(&control.kind) == section
                            && control.group == group.name
                            && !raw_relay(capabilities, control)
                    })
                    .map(|control| control.key.clone())
                    .collect::<Vec<_>>();
                move_menu(app, ui, capabilities, section, &group.name, &keys);
            });
    }
    open
}

pub(super) fn folder_drop_target(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities,
    rect: egui::Rect,
    section: &str,
    name: &str,
) {
    if let Some(fields) = take_folder_drop(ui, capabilities, rect, section, name) {
        update(app, capabilities, fields);
    }
}

fn take_folder_drop(
    ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities,
    rect: egui::Rect,
    section: &str,
    name: &str,
) -> Option<serde_json::Value> {
    let Some((drag_id, drag)) = [
        HardwareChannelDragSurface::Monitor,
        HardwareChannelDragSurface::Manager,
    ]
    .into_iter()
    .find_map(|surface| {
        let id = hardware_channel_drag_id(ui, surface);
        ui.ctx()
            .data(|data| data.get_temp::<HardwareChannelDrag>(id))
            .map(|drag| (id, drag))
    }) else {
        return None;
    };
    let Some(source) = capabilities
        .controls
        .iter()
        .find(|control| control.key == drag.key)
    else {
        return None;
    };
    if kind(&source.kind) != section || raw_relay(capabilities, source) || source.group == name {
        return None;
    }
    if !ui
        .ctx()
        .pointer_hover_pos()
        .is_some_and(|pointer| rect.intersect(ui.clip_rect()).contains(pointer))
    {
        return None;
    }
    ui.painter().rect_stroke(
        rect.expand(2.0),
        4.0,
        ui.visuals().selection.stroke,
        egui::StrokeKind::Inside,
    );
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    if ui.input(|input| input.pointer.button_released(egui::PointerButton::Primary)) {
        ui.ctx()
            .data_mut(|data| data.remove_temp::<HardwareChannelDrag>(drag_id));
        return Some(
            serde_json::json!({"operation":"move","kind":section,"name":name,"keys":[source.key]}),
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::four_d::controller::{HardwareChannelFolder, HardwareOutput};

    fn run_ui(context: &egui::Context, input: egui::RawInput, body: impl FnMut(&mut egui::Ui)) {
        let mut output = context.run_ui(input, body);
        output.textures_delta.clear();
    }

    #[test]
    fn hardware_folder_section_context_menu_works_on_collapsed_headers() {
        for kind in ["seat", "relay", "pwm"] {
            let context = egui::Context::default();
            let mut app = PealayerApp::default();
            let controls = vec![HardwareControl {
                key: format!("{kind}.1"),
                kind: kind.into(),
                ..Default::default()
            }];
            let caps = HardwareCapabilities {
                controls: controls.clone(),
                ..Default::default()
            };
            let rect = std::cell::Cell::new(egui::Rect::NOTHING);
            let popup = std::cell::Cell::new(egui::Id::NULL);
            let render = |ui: &mut egui::Ui, app: &mut PealayerApp| {
                let response = hardware_section(ui, "test-section", "", kind, false, |_| {
                    panic!("collapsed body rendered")
                });
                rect.set(response.rect);
                popup.set(response.id.with("section-context"));
                section_menu(app, ui, &response, &caps, &controls);
            };
            run_ui(&context, egui::RawInput::default(), |ui| {
                render(ui, &mut app)
            });
            let point = rect.get().center();
            run_ui(
                &context,
                egui::RawInput {
                    events: vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Secondary,
                            pressed: true,
                            modifiers: egui::Modifiers::NONE,
                        },
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Secondary,
                            pressed: false,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                },
                |ui| render(ui, &mut app),
            );
            assert!(
                egui::Popup::is_id_open(&context, popup.get()),
                "missing {kind} section menu"
            );
        }
    }

    #[test]
    fn hardware_folder_drop_consumes_only_a_valid_primary_release() {
        for (section, button, expected) in [
            ("pwm", egui::PointerButton::Primary, true),
            ("relay", egui::PointerButton::Primary, false),
            ("pwm", egui::PointerButton::Secondary, false),
        ] {
            let context = egui::Context::default();
            let control = HardwareControl {
                key: "pwm.0".into(),
                kind: "pwm".into(),
                ..Default::default()
            };
            let caps = HardwareCapabilities {
                controls: vec![control.clone()],
                ..Default::default()
            };
            let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(220.0, 28.0));
            let point = rect.center();
            run_ui(
                &context,
                egui::RawInput {
                    events: vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button,
                            pressed: true,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                },
                |ui| {
                    let id = hardware_channel_drag_id(ui, HardwareChannelDragSurface::Monitor);
                    ui.data_mut(|data| {
                        data.insert_temp(
                            id,
                            HardwareChannelDrag {
                                key: control.key.clone(),
                                kind: control.kind.clone(),
                                ..Default::default()
                            },
                        )
                    });
                },
            );
            run_ui(
                &context,
                egui::RawInput {
                    events: vec![egui::Event::PointerButton {
                        pos: point,
                        button,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ui| {
                    let fields = take_folder_drop(ui, &caps, rect, section, "Empty");
                    assert_eq!(fields.is_some(), expected);
                    if let Some(fields) = fields {
                        assert_eq!(
                            fields,
                            serde_json::json!({"operation":"move","kind":"pwm","name":"Empty","keys":["pwm.0"]})
                        );
                    }
                    assert_eq!(
                        hardware_channel_is_dragging(
                            ui,
                            "pwm.0",
                            HardwareChannelDragSurface::Monitor
                        ),
                        !expected
                    );
                },
            );
        }
    }

    #[test]
    fn hardware_folder_editor_retains_failed_drafts_and_closes_on_ack_or_board_change() {
        let context = egui::Context::default();
        let mut app = PealayerApp::default();
        let mut caps = HardwareCapabilities {
            board_profile: Some(crate::four_d::controller::HardwareBoardProfile {
                board_identity: "board-a".into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        context.data_mut(|data| {
            data.insert_temp(
                editor_id(),
                FolderDraft {
                    board_identity: "board-a".into(),
                    kind: "pwm".into(),
                    name: "Preserved draft".into(),
                    icon: "lightbulb".into(),
                    pending: true,
                    ..Default::default()
                },
            )
        });
        app.channel_folder_result = Some(Err("stale revision".into()));
        run_ui(&context, egui::RawInput::default(), |ui| {
            draw_dialogs(&mut app, ui, &caps)
        });
        let draft = context
            .data(|data| data.get_temp::<FolderDraft>(editor_id()))
            .unwrap();
        assert_eq!(draft.name, "Preserved draft");
        assert_eq!(draft.icon, "lightbulb");
        assert_eq!(draft.error, "stale revision");
        assert!(!draft.pending);
        context.data_mut(|data| {
            data.insert_temp(
                editor_id(),
                FolderDraft {
                    pending: true,
                    ..draft.clone()
                },
            )
        });
        app.channel_folder_result = Some(Ok(()));
        run_ui(&context, egui::RawInput::default(), |ui| {
            draw_dialogs(&mut app, ui, &caps)
        });
        assert!(
            context
                .data(|data| data.get_temp::<FolderDraft>(editor_id()))
                .is_none()
        );
        context.data_mut(|data| data.insert_temp(editor_id(), draft));
        caps.board_profile.as_mut().unwrap().board_identity = "board-b".into();
        run_ui(&context, egui::RawInput::default(), |ui| {
            draw_dialogs(&mut app, ui, &caps)
        });
        assert!(
            context
                .data(|data| data.get_temp::<FolderDraft>(editor_id()))
                .is_none()
        );
    }

    #[test]
    fn hardware_folders_partition_only_advertised_members_and_keep_empty_drop_targets() {
        let app = PealayerApp::default();
        let mut caps = HardwareCapabilities::default();
        caps.channel_folders = vec![
            HardwareChannelFolder {
                kind: "pwm".into(),
                name: "Cinema lighting".into(),
                icon: "lightbulb".into(),
            },
            HardwareChannelFolder {
                kind: "pwm".into(),
                name: "Unused".into(),
                icon: String::new(),
            },
        ];
        let controls = (0..16)
            .map(|id| HardwareControl {
                key: format!("pwm.{id}"),
                kind: "pwm".into(),
                group: if id == 0 {
                    "Cinema lighting".into()
                } else {
                    String::new()
                },
                hidden: id >= 13,
                ..Default::default()
            })
            .collect::<Vec<_>>();
        let folders = groups(&app, &caps, &controls);
        assert_eq!(
            folders
                .iter()
                .find(|folder| folder.name == "Cinema lighting")
                .unwrap()
                .members
                .iter()
                .map(|control| control.key.as_str())
                .collect::<Vec<_>>(),
            ["pwm.0"]
        );
        assert_eq!(
            folders
                .iter()
                .find(|folder| folder.name.is_empty())
                .unwrap()
                .members
                .len(),
            12
        );
        assert!(
            folders
                .iter()
                .find(|folder| folder.name == "Unused")
                .unwrap()
                .members
                .is_empty()
        );
        let all_grouped = vec![controls[0].clone()];
        assert!(
            groups(&app, &caps, &all_grouped)
                .iter()
                .any(|folder| folder.name.is_empty() && folder.members.is_empty())
        );

        let relays = (1..=8)
            .map(|id| HardwareControl {
                key: format!("relay.{id}"),
                kind: "relay".into(),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        caps.relays = (1..=8)
            .map(|id| HardwareOutput {
                id,
                key: format!("relay.{id}"),
                control: if id <= 4 {
                    "seat-internal".into()
                } else {
                    "relay".into()
                },
                name: String::new(),
                role: String::new(),
            })
            .collect();
        let folders = groups(&app, &caps, &relays);
        assert_eq!(
            folders
                .iter()
                .find(|folder| folder.raw)
                .unwrap()
                .members
                .iter()
                .map(|control| control.key.as_str())
                .collect::<Vec<_>>(),
            ["relay.1", "relay.2", "relay.3", "relay.4"]
        );
        assert_eq!(
            folders
                .iter()
                .find(|folder| !folder.raw)
                .unwrap()
                .members
                .len(),
            4
        );
        caps.relays[0].control = "relay".into();
        assert!(
            !raw_relay(&caps, &relays[0]),
            "an ordinary R1 is not inferred to be seat wiring"
        );
    }
}

pub(super) fn draw_dialogs(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &HardwareCapabilities,
) {
    if let Some(section) = ui.ctx().data(|data| data.get_temp::<String>(manager_id())) {
        let mut open = true;
        egui::Window::new(app.tr("Manage folders"))
            .id(manager_id().with("window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(360.0)
            .show(ui.ctx(), |ui| {
                if ui
                    .button(format!(
                        "{} {}",
                        crate::ui::icons::PLUS,
                        app.tr("New folder...")
                    ))
                    .clicked()
                {
                    edit(ui, capabilities, &section, None);
                }
                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .show(ui, |ui| {
                        for folder in capabilities
                            .channel_folders
                            .iter()
                            .filter(|folder| folder.kind == section)
                        {
                            ui.horizontal(|ui| {
                                let count = capabilities
                                    .controls
                                    .iter()
                                    .filter(|control| {
                                        kind(&control.kind) == section
                                            && control.group == folder.name
                                    })
                                    .count();
                                ui.label(format!("{} ({count})", folder.name));
                                if ui.button(app.tr("Manage")).clicked() {
                                    edit(ui, capabilities, &section, Some(&folder.name));
                                }
                                let rect = ui.min_rect();
                                folder_drop_target(
                                    app,
                                    ui,
                                    capabilities,
                                    rect,
                                    &section,
                                    &folder.name,
                                );
                            });
                        }
                    });
            });
        if !open {
            ui.ctx()
                .data_mut(|data| data.remove_temp::<String>(manager_id()));
        }
    }
    let Some(mut draft) = ui
        .ctx()
        .data(|data| data.get_temp::<FolderDraft>(editor_id()))
    else {
        return;
    };
    if draft.pending {
        if let Some(result) = app.channel_folder_result.take() {
            match result {
                Ok(()) => {
                    ui.ctx()
                        .data_mut(|data| data.remove_temp::<FolderDraft>(editor_id()));
                    return;
                }
                Err(error) => {
                    draft.pending = false;
                    draft.error = error;
                }
            }
        }
    }
    if capabilities
        .board_profile
        .as_ref()
        .map(|profile| profile.board_identity.as_str())
        != Some(draft.board_identity.as_str())
    {
        ui.ctx()
            .data_mut(|data| data.remove_temp::<FolderDraft>(editor_id()));
        return;
    }
    let mut open = true;
    let mut save = false;
    let mut delete = false;
    let mut cancel = false;
    let title = app.tr(if draft.original.is_some() {
        "Manage folder"
    } else {
        "New folder"
    });
    egui::Window::new(title)
        .id(editor_id().with("window"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(340.0)
        .show(ui.ctx(), |ui| {
            ui.label(app.tr("Name"));
            let response = ui.add(crate::ui::dialog::singleline_text_edit(&mut draft.name));
            if draft.focus {
                response.request_focus();
                draft.focus = false;
            }
            ui.label(app.tr("Icon"));
            let search_hint = app.tr("Search icons...");
            let presets = app.tr("Presets");
            let no_matches = app.tr("No matching icons");
            let default = app.tr("Folder");
            crate::ui::icons::searchable_icon_picker(
                ui,
                "channel-folder-icon",
                &mut draft.icon,
                crate::ui::icons::IconPickerConfig {
                    language: app.language,
                    presets: crate::ui::icons::CONTROL_ICON_PRESETS,
                    fallback_glyph: crate::ui::icons::FOLDER_OPEN,
                    fallback_name: &default,
                    width: 300.0,
                    show_selected_name: true,
                    search_hint: &search_hint,
                    presets_label: &presets,
                    no_matches_label: &no_matches,
                    clear_label: Some(&default),
                },
            );
            if !draft.error.is_empty() {
                ui.colored_label(ui.visuals().error_fg_color, &draft.error);
            }
            ui.horizontal(|ui| {
                save = ui
                    .add_enabled(
                        !draft.pending
                            && !draft.name.trim().is_empty()
                            && draft.name.chars().count() <= 64,
                        egui::Button::new(app.tr("Save")),
                    )
                    .clicked();
                if ui.button(app.tr("Cancel")).clicked() {
                    cancel = true;
                }
                if draft.original.is_some() && ui.button(app.tr("Delete folder")).clicked() {
                    draft.delete = true;
                }
            });
            if draft.delete {
                ui.label(app.tr("Channels will be moved to Ungrouped."));
                delete = ui
                    .add_enabled(!draft.pending, egui::Button::new(app.tr("Confirm delete")))
                    .clicked();
            }
        });
    if cancel || ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        open = false;
    }
    if save || delete {
        let fields = if delete {
            serde_json::json!({"operation":"delete","kind":draft.kind,"name":draft.original})
        } else if let Some(original) = &draft.original {
            serde_json::json!({"operation":"update","kind":draft.kind,"name":original,"next_name":draft.name.trim(),"icon":draft.icon})
        } else {
            serde_json::json!({"operation":"create","kind":draft.kind,"name":draft.name.trim(),"icon":draft.icon})
        };
        match request_channel_folder_update(app, capabilities, fields) {
            Ok(()) => draft.pending = true,
            Err(error) => draft.error = error,
        }
    }
    ui.ctx().data_mut(|data| {
        if open {
            data.insert_temp(editor_id(), draft);
        } else {
            data.remove_temp::<FolderDraft>(editor_id());
        }
    });
}
