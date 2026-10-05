//! One single-value group selector for channel presentation and effect editors.
use crate::config::AppLanguage;
use eframe::egui;

pub(crate) fn group_options<'a>(
    groups: impl IntoIterator<Item = &'a str>,
    current: &str,
) -> Vec<String> {
    let mut options: Vec<String> = groups
        .into_iter()
        .map(str::trim)
        .filter(|group| !group.is_empty())
        .map(str::to_owned)
        .collect();
    if !current.trim().is_empty() {
        options.push(current.trim().to_owned());
    }
    options.sort();
    options.dedup();
    options
}

/// Editing search text never changes the draft. Only choosing an existing group,
/// Ungrouped, or Create group changes it; the caller retains its normal save path.
pub(crate) fn group_picker<'a>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    value: &mut String,
    groups: impl IntoIterator<Item = &'a str>,
    width: f32,
    language: AppLanguage,
) -> bool {
    let options = group_options(groups, value);
    let previous = value.clone();
    let id = ui.make_persistent_id(egui::IdSalt::new(&id_salt));
    let search_id = id.with("search");
    let focus_id = id.with("search-initialized");
    let mut initialized = ui.data_mut(|data| data.get_temp::<bool>(focus_id).unwrap_or(false));
    let mut search = ui.data_mut(|data| data.get_temp::<String>(search_id).unwrap_or_default());
    let ungrouped = crate::ui::i18n::tr(language, "Ungrouped");
    let width = width.min((ui.ctx().content_rect().width() - 24.0).max(80.0));
    egui::ComboBox::from_id_salt(&id_salt)
        .width(width)
        .height(280.0)
        .truncate()
        .selected_text(if value.trim().is_empty() {
            ungrouped.clone()
        } else {
            crate::ui::i18n::visual_text(language, value)
        })
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            ui.set_width(
                width
                    .max(180.0)
                    .min((ui.ctx().content_rect().width() - 24.0).max(80.0)),
            );
            let edit = ui.add(
                egui::TextEdit::singleline(&mut search)
                    .id(search_id)
                    .hint_text(crate::ui::i18n::tr(language, "Search or create group..."))
                    .desired_width(ui.available_width()),
            );
            if !initialized {
                edit.request_focus();
                initialized = true;
            }
            ui.separator();
            if ui
                .selectable_label(value.trim().is_empty(), &ungrouped)
                .clicked()
            {
                value.clear();
                ui.close();
            }
            let query = search.trim();
            let lower = query.to_lowercase();
            for group in options
                .iter()
                .filter(|group| group.to_lowercase().contains(&lower))
            {
                if ui
                    .selectable_label(
                        value.trim() == group,
                        crate::ui::i18n::visual_text(language, group),
                    )
                    .clicked()
                {
                    *value = group.clone();
                    ui.close();
                }
            }
            // Exact, case-sensitive names are preserved: PCController owns group identity.
            if !query.is_empty() && !options.iter().any(|group| group == query) {
                ui.separator();
                let create = format!(
                    "{} {}: {}",
                    crate::ui::icons::PLUS,
                    crate::ui::i18n::tr(language, "Create group"),
                    crate::ui::i18n::visual_text(language, query)
                );
                if ui.add(egui::Button::new(create).truncate()).clicked()
                    || ((edit.has_focus() || edit.lost_focus())
                        && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                {
                    *value = query.to_owned();
                    ui.close();
                }
            } else if !query.is_empty()
                && (edit.has_focus() || edit.lost_focus())
                && ui.input(|input| input.key_pressed(egui::Key::Enter))
            {
                *value = query.to_owned();
                ui.close();
            }
        });
    if egui::ComboBox::is_open(ui.ctx(), id) {
        ui.data_mut(|data| {
            data.insert_temp(search_id, search);
            data.insert_temp(focus_id, initialized);
        });
    } else {
        ui.data_mut(|data| {
            data.remove::<String>(search_id);
            data.remove::<bool>(focus_id);
        });
    }
    *value != previous
}

/// Effects select only authoritative groups. Creating a group is a separate,
/// persistent operation, never a side effect of typing in the selector.
pub(crate) fn effect_group_picker(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    value: &mut String,
    groups: &[crate::four_d::controller::HardwareEffectGroup],
    width: f32,
    language: AppLanguage,
) -> bool {
    let mut new_group = false;
    egui::ComboBox::from_id_salt(id_salt)
        .width(width)
        .height(280.0)
        .truncate()
        .selected_text(if value.is_empty() {
            crate::ui::i18n::tr(language, "Select group")
        } else {
            crate::ui::i18n::visual_text(language, value)
        })
        .show_ui(ui, |ui| {
            ui.set_width(width.min((ui.ctx().content_rect().width() - 24.0).max(80.0)));
            for group in groups {
                if ui.add_sized([ui.available_width(), 26.0], egui::Button::new(
                    crate::ui::i18n::visual_text(language, &group.name))
                    .selected(*value == group.name).truncate()).on_hover_text(&group.name).clicked() {
                    *value = group.name.clone();
                    ui.close();
                }
            }
            ui.separator();
            if ui
                .button(format!(
                    "{} {}",
                    crate::ui::icons::PLUS,
                    crate::ui::i18n::tr(language, "New...")
                ))
                .clicked()
            {
                new_group = true;
                ui.close();
            }
        });
    new_group
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        context: &egui::Context,
        value: &mut String,
        events: Vec<egui::Event>,
    ) -> (egui::FullOutput, egui::Rect) {
        let mut rect = egui::Rect::NOTHING;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 500.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                rect = ui
                    .scope(|ui| {
                        group_picker(
                            ui,
                            "interaction-groups",
                            value,
                            ["Lighting", "Seats"],
                            260.0,
                            AppLanguage::English,
                        );
                    })
                    .response
                    .rect;
            },
        );
        output.textures_delta.clear();
        (output, rect)
    }

    fn click(context: &egui::Context, value: &mut String, pos: egui::Pos2) -> egui::FullOutput {
        frame(
            context,
            value,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        frame(
            context,
            value,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )
        .0
    }

    fn text_rect(output: &egui::FullOutput, text: &str) -> egui::Rect {
        fn find(shape: &egui::epaint::Shape, text: &str) -> Option<egui::Rect> {
            match shape {
                egui::epaint::Shape::Text(shape) if shape.galley.job.text == text => {
                    Some(shape.galley.rect.translate(shape.pos.to_vec2()))
                }
                egui::epaint::Shape::Vec(shapes) => {
                    shapes.iter().find_map(|shape| find(shape, text))
                }
                _ => None,
            }
        }
        output
            .shapes
            .iter()
            .find_map(|shape| find(&shape.shape, text))
            .unwrap_or_else(|| panic!("Missing group option {text}"))
    }

    #[test]
    fn group_picker_pointer_selection_search_creation_and_clear_work() {
        for dark in [false, true] {
            let context = egui::Context::default();
            context.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            let mut value = "Seats".to_owned();
            frame(&context, &mut value, vec![]);
            let (_, rect) = frame(&context, &mut value, vec![]);
            click(&context, &mut value, rect.center());
            let output = frame(&context, &mut value, vec![]).0;
            click(
                &context,
                &mut value,
                text_rect(&output, "Lighting").center(),
            );
            assert_eq!(value, "Lighting");
            let (_, rect) = frame(&context, &mut value, vec![]);
            click(&context, &mut value, rect.center());
            frame(&context, &mut value, vec![]);
            frame(
                &context,
                &mut value,
                vec![egui::Event::Text("Custom group".to_owned())],
            );
            assert_eq!(value, "Lighting", "Searching must not change the draft");
            frame(
                &context,
                &mut value,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert_eq!(value, "Custom group");
            let (_, rect) = frame(&context, &mut value, vec![]);
            click(&context, &mut value, rect.center());
            let output = frame(&context, &mut value, vec![]).0;
            assert!(
                output
                    .shapes
                    .iter()
                    .any(|shape| matches!(shape.shape, egui::epaint::Shape::Text(_)))
            );
            click(
                &context,
                &mut value,
                text_rect(&output, "Ungrouped").center(),
            );
            assert!(value.is_empty());
        }
    }

    #[test]
    fn group_picker_options_are_live_deduplicated_and_preserve_custom_names() {
        assert_eq!(
            group_options([" Seats ", "Lighting", "Seats", "", "چراغ‌ها"], "Custom"),
            ["Custom", "Lighting", "Seats", "چراغ‌ها"]
        );
        assert!(group_options([], "  ").is_empty());
        assert_eq!(
            group_options(["seats", "Seats"], "Seats"),
            ["Seats", "seats"]
        );
    }

    #[test]
    fn effect_group_dropdown_does_not_accept_arbitrary_typing_and_has_new_action() {
        let context = egui::Context::default();
        let mut value = "Lighting".to_owned();
        let groups = vec![crate::four_d::controller::HardwareEffectGroup {
            name: "Lighting".to_owned(),
            icon: String::new(),
        }];
        let mut render = |events: Vec<egui::Event>| {
            let mut requested = false;
            let mut rect = egui::Rect::NOTHING;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(600.0, 400.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    rect = ui
                        .scope(|ui| {
                            requested = effect_group_picker(
                                ui,
                                "saved-groups",
                                &mut value,
                                &groups,
                                260.0,
                                AppLanguage::English,
                            );
                        })
                        .response
                        .rect;
                },
            );
            output.textures_delta.clear();
            (output, rect, requested)
        };
        render(vec![]);
        let (_, rect, _) = render(vec![]);
        let pointer = |pos, pressed| {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        render(pointer(rect.center(), true));
        render(pointer(rect.center(), false));
        let (output, _, _) = render(vec![egui::Event::Text("Arbitrary input".to_owned())]);
        let pos = text_rect(&output, &format!("{} New...", crate::ui::icons::PLUS)).center();
        render(pointer(pos, true));
        let (_, _, requested) = render(pointer(pos, false));
        assert!(requested);
        drop(render);
        assert_eq!(value, "Lighting");
    }

    #[test]
    fn group_picker_closed_control_preserves_draft_and_bounds_width_in_both_themes() {
        for dark in [false, true] {
            let context = egui::Context::default();
            context.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            let mut value = "Unlisted custom group".to_owned();
            let mut changed = true;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 400.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    changed = group_picker(
                        ui,
                        "groups",
                        &mut value,
                        ["Seats", "Lighting"],
                        260.0,
                        AppLanguage::English,
                    );
                    assert!(ui.min_rect().width() <= 400.0);
                },
            );
            output.textures_delta.clear();
            assert!(!changed);
            assert_eq!(value, "Unlisted custom group");
        }
    }
}
