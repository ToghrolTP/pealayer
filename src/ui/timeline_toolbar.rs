//! Fixed-geometry, panel-owned controls. Drag state is deliberately distinct
//! from hardware cards and cues; persistence never runs inside DockArea.
use crate::app::PealayerApp;
use crate::config::TimelineToolbarAction as Action;
use eframe::egui::{self, Align2, Color32, FontId, Id, Rect, Response, Sense, Stroke, StrokeKind};

const CELL: f32 = 26.0;

#[derive(Clone)]
struct ToolbarDrag(Action);

pub(super) struct Output {
    pub rect: Rect,
    pub action: Option<Action>,
    pub held_pan: f32,
}

pub(super) fn bounds(viewport: Rect, count: usize, pixels_per_point: f32) -> Rect {
    let snap = |value: f32| (value * pixels_per_point).round() / pixels_per_point;
    let right = snap(viewport.right());
    let top = snap(viewport.top());
    Rect::from_min_max(
        egui::pos2(
            snap((right - (count + 1) as f32 * CELL - 4.0).max(viewport.left())),
            top,
        ),
        egui::pos2(right, snap(top + CELL)),
    )
}

pub(super) fn held_pan_delta(direction: f32, viewport_width: f32, frame_seconds: f32) -> f32 {
    direction * viewport_width.max(200.0) * frame_seconds.clamp(0.0, 0.1)
}

fn move_action(order: &mut Vec<Action>, action: Action, target: Action, after: bool) -> bool {
    if action == target {
        return false;
    }
    let before = order.clone();
    order.retain(|candidate| *candidate != action);
    if let Some(index) = order.iter().position(|candidate| *candidate == target) {
        order.insert(index + usize::from(after), action);
    }
    *order != before
}

fn control(
    ui: &mut egui::Ui,
    rect: Rect,
    id: Id,
    icon: &str,
    label: &str,
    enabled: bool,
    selected: bool,
) -> Response {
    let response = ui.interact(rect, id, Sense::click_and_drag());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    let visuals = ui.style().interact_selectable(&response, selected);
    // Paint the border INSIDE an invariant hit box, including its transparent
    // inactive state. Hover/selection can never change the allocated size.
    let fill = if selected {
        ui.visuals().selection.bg_fill
    } else if response.hovered() || response.is_pointer_button_down_on() {
        visuals.weak_bg_fill
    } else {
        Color32::TRANSPARENT
    };
    let stroke = if selected {
        ui.visuals().selection.stroke
    } else {
        Stroke::new(1.0, Color32::TRANSPARENT)
    };
    ui.painter()
        .rect(rect.shrink(2.0), 4.0, fill, stroke, StrokeKind::Inside);
    let color = if enabled {
        visuals.text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        icon,
        FontId::proportional(14.0),
        color,
    );
    response.on_hover_cursor(if enabled {
        egui::CursorIcon::PointingHand
    } else {
        egui::CursorIcon::Default
    })
}

pub(super) fn draw(
    ui: &mut egui::Ui,
    app: &mut PealayerApp,
    viewport: Rect,
    can_add_keyframe: bool,
) -> Output {
    let mut output = Output {
        rect: Rect::NOTHING,
        action: None,
        held_pan: 0.0,
    };
    if !app.timeline_toolbar_visible {
        return output;
    }
    let mut order = crate::config::normalize_timeline_toolbar_order(&app.timeline_toolbar_order);
    let visible: Vec<_> = order
        .iter()
        .copied()
        .filter(|action| !app.timeline_toolbar_hidden.contains(action))
        .collect();
    output.rect = bounds(viewport, visible.len(), ui.ctx().pixels_per_point());
    let layer = super::dialog::workspace_overlay_layer(ui, "timeline-ruler-toolbar-layer");
    let mut toolbar = ui.new_child(egui::UiBuilder::new().layer_id(layer).max_rect(output.rect));
    toolbar.set_clip_rect(output.rect.intersect(ui.clip_rect()));
    // Opaque and flush with the viewport edge: no tick can show behind the
    // controls, irrespective of horizontal pan or fractional scroll offsets.
    toolbar
        .painter()
        .rect_filled(output.rect, 0.0, ui.visuals().panel_fill);
    let pointer = ui.input(|input| input.pointer.interact_pos());
    let now = ui.input(|input| input.time);
    let mut changed = false;

    // Keep the existing right-to-left order: first configured action is next
    // to the overflow button. Hidden actions retain their ordering slots.
    for (index, action) in visible.iter().copied().enumerate() {
        let right = output.rect.right() - CELL * (index + 1) as f32;
        let rect = Rect::from_min_max(
            egui::pos2(right - CELL, output.rect.top()),
            egui::pos2(right, output.rect.bottom()),
        );
        if rect.left() < output.rect.left() {
            continue;
        }
        let has_selection = !app.selected_instance_ids.is_empty()
            || !app.selected_keyframes.is_empty()
            || app.selected_timeline_keyframe.is_some();
        let enabled = match action {
            Action::AddKeyframe => can_add_keyframe,
            Action::PreviousCue | Action::NextCue => !app.timeline.instances.is_empty(),
            Action::NudgeCueLeft
            | Action::NudgeCueRight
            | Action::DeleteSelection
            | Action::ClearSelection => has_selection,
            _ => true,
        };
        let label = app.tr(super::layout::timeline_toolbar_action_label(action));
        // An unavailable action can still be reordered/hidden, but not invoked.
        let response = control(
            &mut toolbar,
            rect,
            Id::new(("timeline-toolbar-action", action)),
            super::layout::timeline_toolbar_action_icon(action),
            &label,
            enabled,
            action == Action::FollowPlayhead && app.timeline_follow_playhead,
        );
        if response.drag_started_by(egui::PointerButton::Primary) {
            egui::DragAndDrop::set_payload(ui.ctx(), ToolbarDrag(action));
        }
        if let Some(drag) = response.dnd_hover_payload::<ToolbarDrag>() {
            let after = pointer.is_some_and(|pos| pos.x < rect.center().x);
            let x = if after { rect.left() } else { rect.right() };
            toolbar
                .painter()
                .vline(x, rect.y_range().shrink(4.0), ui.visuals().selection.stroke);
            if let Some(drag) = response.dnd_release_payload::<ToolbarDrag>() {
                changed |= move_action(&mut order, drag.0, action, after);
            } else if drag.0 != action {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }
        let hold_id = response.id.with("pan-hold");
        let holding = enabled
            && matches!(action, Action::PanLeft | Action::PanRight)
            && response.is_pointer_button_down_on()
            && !response.dragged();
        let hold_started = if holding {
            ui.data_mut(|data| *data.get_temp_mut_or_insert_with(hold_id, || now))
        } else {
            let started = ui.data(|data| data.get_temp::<f64>(hold_id));
            ui.data_mut(|data| data.remove_temp::<f64>(hold_id));
            started.unwrap_or(now)
        };
        if holding {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
            if now - hold_started >= 0.3 {
                output.held_pan = if action == Action::PanLeft { -1.0 } else { 1.0 };
            }
        }
        if enabled
            && response.clicked()
            && now - hold_started < 0.3
            && !egui::DragAndDrop::has_payload_of_type::<ToolbarDrag>(ui.ctx())
        {
            output.action = Some(action);
        }
        response.on_hover_text(format!(
            "{}\n{}\n{}",
            label,
            super::layout::timeline_toolbar_action_shortcut(action),
            app.tr("Drag to reorder or hide")
        ));
    }

    let drag = egui::DragAndDrop::payload::<ToolbarDrag>(ui.ctx());
    let drag_fade = ui.ctx().animate_bool_with_time(
        Id::new("timeline-toolbar-hide-fade"),
        drag.is_some(),
        0.15,
    );
    let menu_rect = Rect::from_min_max(
        egui::pos2(output.rect.right() - CELL, output.rect.top()),
        output.rect.max,
    );
    let response = control(
        &mut toolbar,
        menu_rect,
        Id::new("timeline-toolbar-menu"),
        "",
        &app.tr("Timeline toolbar"),
        true,
        false,
    );
    if drag_fade > 0.0 {
        toolbar.painter().rect_filled(
            menu_rect.shrink(2.0),
            4.0,
            ui.visuals().error_fg_color.gamma_multiply(drag_fade),
        );
    }
    for (icon, alpha) in [
        (super::icons::DOTS_THREE, 1.0 - drag_fade),
        (super::icons::TRASH, drag_fade),
    ] {
        let color = if icon == super::icons::TRASH {
            Color32::WHITE
        } else {
            ui.visuals().text_color()
        };
        toolbar.painter().text(
            menu_rect.center(),
            Align2::CENTER_CENTER,
            icon,
            FontId::proportional(14.0),
            color.gamma_multiply(alpha),
        );
    }
    if drag.is_some() {
        if let Some(drag) = response.dnd_release_payload::<ToolbarDrag>() {
            if !app.timeline_toolbar_hidden.contains(&drag.0) {
                app.timeline_toolbar_hidden.push(drag.0);
                changed = true;
            }
        }
        response
            .clone()
            .on_hover_text(app.tr("Drop here to hide this control"));
    } else {
        // Close only outside the popup. Checking several rows should not
        // repeatedly dismiss the menu or resize the dock beneath it.
        egui::Popup::menu(&response)
            .width(250.0)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_min_width(238.0);
                ui.set_max_width(250.0);
                ui.strong(app.tr("Timeline toolbar"));
                ui.separator();
                for (index, action) in order.clone().into_iter().enumerate() {
                    ui.horizontal(|ui| {
                        let mut visible = !app.timeline_toolbar_hidden.contains(&action);
                        let checkbox = ui.checkbox(
                            &mut visible,
                            egui::RichText::new(format!(
                                "{}  {}",
                                super::layout::timeline_toolbar_action_icon(action),
                                app.tr(super::layout::timeline_toolbar_action_label(action))
                            ))
                            .size(12.0),
                        );
                        if checkbox.changed() {
                            app.timeline_toolbar_hidden
                                .retain(|candidate| *candidate != action);
                            if !visible {
                                app.timeline_toolbar_hidden.push(action);
                            }
                            changed = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            for (can_move, icon, next, tip) in [
                                (
                                    index + 1 < order.len(),
                                    super::icons::ARROW_DOWN,
                                    (index + 1).min(order.len() - 1),
                                    "Move down",
                                ),
                                (
                                    index > 0,
                                    super::icons::ARROW_UP,
                                    index.saturating_sub(1),
                                    "Move up",
                                ),
                            ] {
                                if ui
                                    .add_enabled(can_move, egui::Button::new(icon).small())
                                    .on_hover_text(app.tr(tip))
                                    .clicked()
                                {
                                    order.swap(index, next);
                                    changed = true;
                                }
                            }
                        });
                    });
                }
                ui.separator();
                if ui
                    .button(format!(
                        "{} {}",
                        super::icons::ARROW_COUNTER_CLOCKWISE,
                        app.tr("Reset toolbar")
                    ))
                    .clicked()
                {
                    order = crate::config::default_timeline_toolbar_order();
                    app.timeline_toolbar_hidden = crate::config::default_timeline_toolbar_hidden();
                    changed = true;
                }
                if ui
                    .button(format!(
                        "{} {}",
                        super::icons::GEAR,
                        app.tr("Preferences...")
                    ))
                    .clicked()
                {
                    super::preferences::open(app, ui.ctx());
                    ui.close();
                }
            });
        response.on_hover_text(app.tr("More timeline controls and preferences"));
    }
    if let Some(drag) = drag
        && let Some(pos) = pointer
    {
        let ghost = Rect::from_center_size(pos - egui::vec2(0.0, 18.0), egui::vec2(CELL, CELL));
        let painter = ui.ctx().layer_painter(layer);
        painter.rect_filled(
            ghost,
            4.0,
            ui.visuals().widgets.hovered.bg_fill.gamma_multiply(0.8),
        );
        painter.text(
            ghost.center(),
            Align2::CENTER_CENTER,
            super::layout::timeline_toolbar_action_icon(drag.0),
            FontId::proportional(14.0),
            ui.visuals().text_color().gamma_multiply(0.8),
        );
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    if changed {
        app.timeline_toolbar_order = order;
        app.request_timeline_toolbar_save(ui.ctx());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_moves_only_the_requested_action_and_keeps_hidden_slots() {
        let mut order = crate::config::default_timeline_toolbar_order();
        assert!(move_action(
            &mut order,
            Action::ZoomIn,
            Action::PanLeft,
            true
        ));
        let target = order
            .iter()
            .position(|action| *action == Action::PanLeft)
            .unwrap();
        assert_eq!(order[target + 1], Action::ZoomIn);
        assert_eq!(order.len(), Action::ALL.len());
        assert!(!move_action(
            &mut order,
            Action::PanLeft,
            Action::PanLeft,
            true
        ));
    }

    #[test]
    fn toolbar_is_pixel_aligned_full_height_and_flush_right() {
        let viewport = Rect::from_min_size(egui::pos2(10.13, 20.12), egui::vec2(900.0, 400.0));
        let rect = bounds(viewport, 6, 1.5);
        assert_eq!(rect.height(), CELL);
        assert_eq!(rect.right(), (viewport.right() * 1.5).round() / 1.5);
        assert_eq!(rect.top(), (viewport.top() * 1.5).round() / 1.5);
    }

    #[test]
    fn held_pan_is_frame_rate_independent_and_bounded_after_a_stall() {
        let at_60 = held_pan_delta(1.0, 900.0, 1.0 / 60.0) * 60.0;
        let at_120 = held_pan_delta(1.0, 900.0, 1.0 / 120.0) * 120.0;
        assert!((at_60 - at_120).abs() < 0.01);
        assert_eq!(held_pan_delta(-1.0, 900.0, 5.0), -90.0);
        assert_eq!(held_pan_delta(0.0, 900.0, 0.016), 0.0);
    }

    #[test]
    fn selecting_a_button_changes_paint_not_geometry() {
        let context = egui::Context::default();
        let rect = Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(CELL, CELL));
        for selected in [false, true] {
            let mut actual = Rect::NOTHING;
            let output = context.run_ui(egui::RawInput::default(), |ui| {
                actual = control(
                    ui,
                    rect,
                    Id::new("fixed-button"),
                    super::super::icons::LOCK,
                    "Follow",
                    true,
                    selected,
                )
                .rect;
            });
            assert_eq!(actual, rect);
            drop(output);
        }
    }
}
