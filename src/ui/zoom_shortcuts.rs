//! Default editor accelerators. The front dialog owns them, not the canvas
//! which happened to retain keyboard focus behind it.
use eframe::egui;

#[derive(Clone, Copy)]
struct ClaimedFrame(u64);

pub(crate) fn take(ui: &egui::Ui, enabled: bool) -> Option<f32> {
    let ctx = ui.ctx();
    if !enabled || !ui.is_enabled() || ctx.text_edit_focused() || egui::Popup::is_any_open(ctx) {
        return None;
    }
    let layer = ui.layer_id();
    let owns_input = ctx.memory(|memory| {
        if !memory.is_above_modal_layer(layer) {
            return false;
        }
        let areas = memory.areas();
        let root = areas.parent_layer(layer).unwrap_or(layer);
        let visible = areas.visible_layer_ids();
        let top_window = memory
            .layer_ids()
            .filter(|candidate| {
                matches!(
                    candidate.order,
                    egui::Order::Middle | egui::Order::Foreground
                ) && visible.contains(candidate)
                    && areas.parent_layer(*candidate).is_none()
            })
            .last();
        top_window.is_none_or(|top| root == top || root.order > top.order)
    });
    let frame = ctx.cumulative_frame_nr();
    let claim_id = egui::Id::new("contextual-editor-zoom-shortcut");
    if !owns_input
        || ctx.data(|data| {
            data.get_temp::<ClaimedFrame>(claim_id)
                .is_some_and(|claim| claim.0 == frame)
        })
    {
        return None;
    }
    let factor = ctx.input_mut(|input| {
        let (key, modifiers, factor) = input.events.iter().find_map(|event| {
            let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            else {
                return None;
            };
            if modifiers.ctrl || modifiers.command || modifiers.mac_cmd || modifiers.alt {
                return None;
            }
            match key {
                egui::Key::Plus | egui::Key::Equals => Some((*key, *modifiers, 1.25)),
                egui::Key::Minus if !modifiers.shift => Some((*key, *modifiers, 0.8)),
                _ => None,
            }
        })?;
        input.consume_key(modifiers, key).then_some(factor)
    })?;
    ctx.data_mut(|data| data.insert_temp(claim_id, ClaimedFrame(frame)));
    Some(factor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: egui::Key, modifiers: egui::Modifiers, repeat: bool) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat,
            modifiers,
        }
    }

    #[test]
    fn defaults_accept_plus_shift_equals_numpad_and_repeats_once_without_canvas_focus() {
        assert!(crate::config::AppConfig::default().keyboard_shortcuts_enabled);
        for (key, modifiers, factor) in [
            (egui::Key::Minus, egui::Modifiers::NONE, 0.8),
            (egui::Key::Plus, egui::Modifiers::NONE, 1.25),
            (egui::Key::Plus, egui::Modifiers::SHIFT, 1.25),
            (egui::Key::Equals, egui::Modifiers::NONE, 1.25),
            (egui::Key::Equals, egui::Modifiers::SHIFT, 1.25),
        ] {
            for repeat in [false, true] {
                let ctx = egui::Context::default();
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        events: vec![press(key, modifiers, repeat)],
                        ..Default::default()
                    },
                    |ui| {
                        assert_eq!(take(ui, true), Some(factor));
                        assert_eq!(take(ui, true), None, "one input must not zoom two editors");
                        assert!(!ctx.input(|input| input.key_pressed(key)));
                    },
                );
                output.textures_delta.clear();
            }
        }
    }

    #[test]
    fn front_window_owns_zoom_and_nonzoom_dialog_does_not_leak_to_timeline() {
        for modal in [false, true] {
            for zoomable in [false, true] {
                let ctx = egui::Context::default();
                for frame in 0..3 {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            events: if frame == 2 {
                                vec![press(egui::Key::Plus, egui::Modifiers::SHIFT, false)]
                            } else {
                                vec![]
                            },
                            ..Default::default()
                        },
                        |ui| {
                            assert_eq!(
                                take(ui, true),
                                None,
                                "background timeline stole the dialog shortcut"
                            );
                            let mut body = |ui: &mut egui::Ui| {
                                ui.label("Dialog");
                                if zoomable {
                                    assert_eq!(take(ui, true), (frame == 2).then_some(1.25));
                                }
                            };
                            if modal {
                                egui::Modal::new(egui::Id::new("zoom-owner-modal"))
                                    .show(&ctx, body);
                            } else {
                                egui::Window::new("Zoom owner")
                                    .order(egui::Order::Foreground)
                                    .show(&ctx, &mut body);
                            }
                        },
                    );
                    output.textures_delta.clear();
                }
            }
        }
    }

    #[test]
    fn text_fields_disabled_shortcuts_and_other_modifiers_keep_their_keys() {
        for (modifiers, enabled) in [
            (egui::Modifiers::CTRL, true),
            (egui::Modifiers::COMMAND, true),
            (egui::Modifiers::ALT, true),
            (egui::Modifiers::SHIFT, true),
            (egui::Modifiers::NONE, false),
        ] {
            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events: vec![press(egui::Key::Minus, modifiers, false)],
                    ..Default::default()
                },
                |ui| {
                    assert_eq!(take(ui, enabled), None);
                    assert!(ctx.input(|input| input.key_pressed(egui::Key::Minus)));
                },
            );
            output.textures_delta.clear();
        }
        let ctx = egui::Context::default();
        let mut value = "1".to_owned();
        for frame in 0..3 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events: if frame == 2 {
                        vec![
                            press(egui::Key::Plus, egui::Modifiers::SHIFT, false),
                            egui::Event::Text("+".into()),
                        ]
                    } else {
                        vec![]
                    },
                    ..Default::default()
                },
                |ui| {
                    if frame == 2 {
                        assert_eq!(take(ui, true), None);
                    }
                    ui.text_edit_singleline(&mut value).request_focus();
                },
            );
            output.textures_delta.clear();
        }
        assert!(value.contains('+'), "zoom stole normal field input");
    }
}
