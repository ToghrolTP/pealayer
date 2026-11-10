//! Capture extra buttons only on their owning visible surface. No global hook.
use crate::pointer_bindings::{Action, Buttons};
use eframe::egui;

pub(crate) fn install(ctx: &egui::Context, bindings: &crate::pointer_bindings::Bindings) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("pointer-bindings-policy"), bindings.clone()));
}
pub(crate) fn bindings(ctx: &egui::Context) -> crate::pointer_bindings::Bindings {
    ctx.data(|d| d.get_temp(egui::Id::new("pointer-bindings-policy")))
        .unwrap_or_default()
}

#[derive(Clone, Copy)]
struct Press {
    button: egui::PointerButton,
    origin: egui::Pos2,
    moved: bool,
    action: Action,
}
pub(crate) fn take_action(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    bindings: Buttons,
) -> Option<Action> {
    take_action_for(ui, rect, bindings, ui.id())
}
pub(crate) fn take_action_for(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    bindings: Buttons,
    owner: egui::Id,
) -> Option<Action> {
    let id = owner.with("extra-mouse-owner");
    let mut press = ui.ctx().data_mut(|d| d.get_temp::<Press>(id));
    if press.is_some_and(|p| {
        !ui.input(|i| i.pointer.button_down(p.button) || i.pointer.button_released(p.button))
    }) {
        press = None;
    }
    let events = ui.input(|i| i.events.clone());
    let hovered =
        ui.is_enabled() && !egui::Popup::is_any_open(ui.ctx()) && ui.rect_contains_pointer(rect);
    let mut claimed = Vec::new();
    let mut result = None;
    for (index, event) in events.iter().enumerate() {
        match event {
            egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers,
            } if matches!(
                button,
                egui::PointerButton::Extra1 | egui::PointerButton::Extra2
            ) =>
            {
                if *pressed
                    && hovered
                    && rect.contains(*pos)
                    && (press.is_none() || press.is_some_and(|p| p.button == *button))
                {
                    press = Some(Press {
                        button: *button,
                        origin: *pos,
                        moved: false,
                        action: bindings.resolve(
                            *button == egui::PointerButton::Extra2,
                            modifiers.ctrl || modifiers.command,
                            modifiers.alt,
                            modifiers.shift,
                        ),
                    });
                    claimed.push(index);
                } else if !pressed && press.is_some_and(|p| p.button == *button) {
                    let p = press.take().unwrap();
                    claimed.push(index);
                    if hovered && rect.contains(*pos) && !p.moved && p.origin.distance(*pos) < 6.0 {
                        result = Some(p.action);
                    }
                }
            }
            egui::Event::PointerMoved(pos) => {
                if let Some(p) = &mut press {
                    p.moved |= p.origin.distance(*pos) >= 6.0;
                }
            }
            egui::Event::PointerGone
            | egui::Event::WindowFocused(false)
            | egui::Event::Key {
                key: egui::Key::Escape,
                pressed: true,
                ..
            } => {
                press = None;
            }
            _ => {}
        }
    }
    if press.is_some_and(|p| !ui.input(|i| i.pointer.button_down(p.button))) {
        press = None;
    }
    ui.ctx().data_mut(|d| {
        if let Some(p) = press {
            d.insert_temp(id, p);
        } else {
            d.remove::<Press>(id);
        }
    });
    if !claimed.is_empty() {
        ui.input_mut(|i| {
            let mut n = 0;
            i.events.retain(|_| {
                let keep = !claimed.contains(&n);
                n += 1;
                keep
            });
        });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn button(pos: egui::Pos2, pressed: bool, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Extra1,
            pressed,
            modifiers,
        }
    }
    #[test]
    fn extra_capture_is_scope_owned_modifier_pinned_and_movement_cancelled() {
        let ctx = egui::Context::default();
        let mut time = 0.0;
        let left = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(300.0, 200.0));
        let right = left.translate(egui::vec2(330.0, 0.0));
        let mut render = |events, bindings| {
            time += 0.05;
            let mut actions = [None, None];
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    events,
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 500.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    actions[0] = take_action_for(ui, left, bindings, egui::Id::new("left-panel"));
                    actions[1] = take_action_for(ui, right, bindings, egui::Id::new("right-panel"));
                },
            );
            output.textures_delta.clear();
            actions
        };
        let b = Buttons {
            back: Action::PanLeft,
            ctrl_back: Action::ZoomOut,
            ..Default::default()
        };
        let a = left.center();
        let c = right.center();
        render(vec![], b);
        assert_eq!(
            render(
                vec![
                    egui::Event::PointerMoved(a),
                    button(a, true, egui::Modifiers::CTRL)
                ],
                b
            ),
            [None, None]
        );
        let changed = Buttons {
            ctrl_back: Action::None,
            ..b
        };
        assert_eq!(
            render(vec![button(a, false, egui::Modifiers::NONE)], changed),
            [Some(Action::ZoomOut), None]
        );
        render(vec![button(a, true, egui::Modifiers::NONE)], b);
        render(vec![egui::Event::PointerMoved(c)], b);
        assert_eq!(
            render(vec![button(c, false, egui::Modifiers::NONE)], b),
            [None, None]
        );
        render(
            vec![
                egui::Event::PointerMoved(c),
                button(c, true, egui::Modifiers::NONE),
            ],
            b,
        );
        assert_eq!(
            render(vec![button(c, false, egui::Modifiers::NONE)], b),
            [None, Some(Action::PanLeft)]
        );
        render(
            vec![
                egui::Event::PointerMoved(a),
                button(a, true, egui::Modifiers::NONE),
            ],
            b,
        );
        render(
            vec![egui::Event::PointerMoved(a + egui::vec2(20.0, 0.0))],
            b,
        );
        render(vec![egui::Event::PointerMoved(a)], b);
        assert_eq!(
            render(vec![button(a, false, egui::Modifiers::NONE)], b),
            [None, None]
        );
        render(vec![button(a, true, egui::Modifiers::NONE)], b);
        render(
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            b,
        );
        assert_eq!(
            render(vec![button(a, false, egui::Modifiers::NONE)], b),
            [None, None]
        );
    }
}
