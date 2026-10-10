//! Fixed-geometry, panel-owned controls. Drag state is deliberately distinct
//! from hardware cards and cues; persistence never runs inside DockArea.
use crate::app::PealayerApp;
use crate::config::TimelineToolbarAction as Action;
use eframe::egui::{self, Align2, Color32, FontId, Id, Rect, Response, Sense, Stroke, StrokeKind};

const CELL: f32 = super::layout::TIMELINE_RULER_HEIGHT;

#[derive(Clone, Debug)]
struct ToolbarDrag {
    action: Action,
    source_rect: Rect,
    grab_offset: egui::Vec2,
}

fn register_drag(response: &Response, action: Action, moved_for_drag: bool) {
    let offset_id = response.id.with("pointer-offset");
    let started_id = response.id.with("payload-started");
    let fresh_press = response.ctx.input(|input| {
        input.pointer.button_pressed(egui::PointerButton::Primary)
            && input
                .pointer
                .press_origin()
                .is_some_and(|pos| response.rect.contains(pos))
    });
    if fresh_press {
        response
            .ctx
            .data_mut(|data| data.remove_temp::<bool>(started_id));
    }
    let started = response
        .ctx
        .data(|data| data.get_temp::<bool>(started_id))
        .unwrap_or(false);
    // Use the same mouse-down capture as effect cards, before egui promotes
    // the gesture. Freeze source geometry in the payload for the whole drag.
    super::layout::remember_drag_offset_on_press(&response.ctx, response.rect, offset_id);
    if response.dragged_by(egui::PointerButton::Primary)
        && moved_for_drag
        && !started
        && !egui::DragAndDrop::has_any_payload(&response.ctx)
    {
        let grab_offset = response
            .ctx
            .data(|data| data.get_temp(offset_id))
            .unwrap_or_else(|| response.rect.size() * 0.5);
        egui::DragAndDrop::set_payload(
            &response.ctx,
            ToolbarDrag {
                action,
                source_rect: response.rect,
                grab_offset,
            },
        );
        // Escape clears egui's payload at frame start. Do not recreate it
        // while the same mouse gesture remains held after cancellation.
        response
            .ctx
            .data_mut(|data| data.insert_temp(started_id, true));
    }
    if response
        .ctx
        .input(|input| input.pointer.button_released(egui::PointerButton::Primary))
    {
        response
            .ctx
            .data_mut(|data| data.remove_temp::<bool>(started_id));
    }
}

fn moved_for_drag(ctx: &egui::Context) -> bool {
    let distance = ctx.options(|options| options.input_options.max_click_dist);
    ctx.input(|input| {
        input
            .pointer
            .press_origin()
            .zip(input.pointer.interact_pos())
            .is_some_and(|(start, current)| start.distance(current) > distance)
    })
}

fn drop_payload(response: &Response) -> Option<std::sync::Arc<ToolbarDrag>> {
    response
        .ctx
        .input(|input| input.pointer.button_released(egui::PointerButton::Primary))
        .then(|| response.dnd_release_payload::<ToolbarDrag>())
        .flatten()
}

fn hide_action(hidden: &mut Vec<Action>, action: Action) -> bool {
    if hidden.contains(&action) {
        false
    } else {
        hidden.push(action);
        true
    }
}

fn ghost_rect(drag: &ToolbarDrag, pointer: egui::Pos2) -> Rect {
    drag.source_rect.translate(super::layout::drag_translation(
        pointer,
        drag.source_rect.min,
        drag.grab_offset,
    ))
}

fn control_outline(visuals: &egui::Visuals, selected: bool, pressed: bool) -> Stroke {
    // selection.stroke is contrast ink, NOT the accent outline color.
    Stroke::new(
        1.0,
        if selected || pressed {
            visuals.selection.bg_fill
        } else {
            Color32::TRANSPARENT
        },
    )
}

fn paint_drag_preview(
    ui: &egui::Ui,
    drag: &ToolbarDrag,
    pointer: egui::Pos2,
    hide_progress: f32,
) -> Rect {
    let ghost = ghost_rect(drag, pointer);
    // Painter-only tooltip layer: visible outside the toolbar, without stealing
    // hit-testing from the drop target underneath the preview.
    let layer = egui::LayerId::new(
        egui::Order::Tooltip,
        Id::new("timeline-toolbar-drag-preview"),
    );
    let painter = ui
        .ctx()
        .layer_painter(layer)
        .with_clip_rect(ui.ctx().content_rect());
    let progress = hide_progress.clamp(0.0, 1.0);
    let alpha = 0.8 - 0.48 * progress;
    let error = ui.visuals().error_fg_color;
    painter.rect(
        ghost.shrink(2.0),
        4.0,
        ui.visuals()
            .widgets
            .hovered
            .bg_fill
            .lerp_to_gamma(error, 0.25 * progress)
            .gamma_multiply(alpha),
        Stroke::new(
            1.0,
            ui.visuals()
                .widgets
                .hovered
                .bg_stroke
                .color
                .lerp_to_gamma(error, progress),
        ),
        StrokeKind::Inside,
    );
    painter.text(
        ghost.center(),
        Align2::CENTER_CENTER,
        super::layout::timeline_toolbar_action_icon(drag.action),
        FontId::proportional(14.0),
        ui.visuals()
            .text_color()
            .lerp_to_gamma(error, progress)
            .gamma_multiply(alpha),
    );
    if progress > 0.0 {
        painter.line_segment(
            [
                ghost.left_top() + egui::vec2(7.0, 7.0),
                ghost.right_bottom() - egui::vec2(7.0, 7.0),
            ],
            Stroke::new(1.5, error.gamma_multiply(progress)),
        );
    }
    ghost
}

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
    let stroke = control_outline(
        ui.visuals(),
        selected,
        enabled && response.is_pointer_button_down_on(),
    );
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
    // egui also marks a stationary long press as a drag after its click
    // timeout. Reordering needs real movement, otherwise held Pan buttons
    // would turn into a drag payload and stop panning after 800 ms.
    let moved_for_drag = moved_for_drag(ui.ctx());
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
        register_drag(&response, action, moved_for_drag);
        if let Some(drag) = response.dnd_hover_payload::<ToolbarDrag>() {
            let after = pointer.is_some_and(|pos| pos.x < rect.center().x);
            let x = if after { rect.left() } else { rect.right() };
            toolbar.painter().vline(
                x,
                rect.y_range().shrink(4.0),
                Stroke::new(1.0, ui.visuals().selection.bg_fill),
            );
            if let Some(drag) = drop_payload(&response) {
                changed |= move_action(&mut order, drag.action, action, after);
            } else if drag.action != action {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
        }
        let hold_id = response.id.with("pan-hold");
        let holding = enabled
            && matches!(action, Action::PanLeft | Action::PanRight)
            && response.is_pointer_button_down_on()
            && !egui::DragAndDrop::has_any_payload(ui.ctx());
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
    // Hover is a prospective removal only. Persist nothing until an actual
    // primary-button drop; leaving Trash reverses the preview transition.
    let hide_hovered = response.dnd_hover_payload::<ToolbarDrag>().is_some();
    let hide_progress = ui.ctx().animate_bool_with_time(
        Id::new("timeline-toolbar-removal-preview"),
        hide_hovered,
        0.15,
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
        if let Some(drag) = drop_payload(&response) {
            changed |= hide_action(&mut app.timeline_toolbar_hidden, drag.action);
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
        paint_drag_preview(ui, &drag, pos, hide_progress);
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

    fn pointer_button(pos: egui::Pos2, button: egui::PointerButton, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn gesture_frame(
        context: &egui::Context,
        source: Rect,
        events: Vec<egui::Event>,
        hidden: &mut Vec<Action>,
    ) -> bool {
        let mut over_trash = false;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(480.0, 160.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                let response = control(
                    ui,
                    source,
                    Id::new("gesture-source"),
                    super::super::icons::LOCK,
                    "Follow",
                    true,
                    false,
                );
                register_drag(&response, Action::FollowPlayhead, moved_for_drag(context));
                let trash = control(
                    ui,
                    Rect::from_min_size(egui::pos2(220.0, 20.0), egui::vec2(CELL, CELL)),
                    Id::new("gesture-trash"),
                    super::super::icons::TRASH,
                    "Hide",
                    true,
                    false,
                );
                over_trash = trash.dnd_hover_payload::<ToolbarDrag>().is_some();
                if let Some(drag) = drop_payload(&trash) {
                    hide_action(hidden, drag.action);
                }
            },
        );
        output.textures_delta.clear();
        over_trash
    }

    #[test]
    fn drag_captures_noncentral_mouse_down_and_keeps_it_after_source_reflow() {
        let context = egui::Context::default();
        let source = Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(CELL, CELL));
        let mut hidden = Vec::new();
        gesture_frame(&context, source, vec![], &mut hidden);
        let press = source.min + egui::vec2(5.0, 13.0);
        gesture_frame(
            &context,
            source,
            vec![
                egui::Event::PointerMoved(press),
                pointer_button(press, egui::PointerButton::Primary, true),
            ],
            &mut hidden,
        );
        assert_eq!(
            context.data(|data| data
                .get_temp::<egui::Vec2>(Id::new("gesture-source").with("pointer-offset"))),
            Some(press - source.min)
        );
        assert!(egui::DragAndDrop::payload::<ToolbarDrag>(&context).is_none());
        let moved = press + egui::vec2(60.0, 35.0);
        gesture_frame(
            &context,
            source,
            vec![egui::Event::PointerMoved(moved)],
            &mut hidden,
        );
        let drag =
            egui::DragAndDrop::payload::<ToolbarDrag>(&context).expect("actual primary drag");
        assert_eq!(ghost_rect(&drag, moved).min + drag.grab_offset, moved);
        assert_eq!(drag.grab_offset, press - source.min);
        let before = ghost_rect(&drag, moved);
        gesture_frame(
            &context,
            source.translate(egui::vec2(31.0, 7.0)),
            vec![],
            &mut hidden,
        );
        let after = egui::DragAndDrop::payload::<ToolbarDrag>(&context).unwrap();
        assert_eq!(after.source_rect, source);
        assert_eq!(ghost_rect(&after, moved), before);
        assert!(hidden.is_empty());
    }

    #[test]
    fn trash_hover_is_reversible_and_only_a_drop_hides_the_control() {
        let context = egui::Context::default();
        let source = Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(CELL, CELL));
        let mut hidden = Vec::new();
        let press = source.min + egui::vec2(6.0, 12.0);
        gesture_frame(&context, source, vec![], &mut hidden);
        gesture_frame(
            &context,
            source,
            vec![
                egui::Event::PointerMoved(press),
                pointer_button(press, egui::PointerButton::Primary, true),
            ],
            &mut hidden,
        );
        gesture_frame(
            &context,
            source,
            vec![egui::Event::PointerMoved(press + egui::vec2(65.0, 20.0))],
            &mut hidden,
        );
        let trash_center = egui::pos2(220.0 + CELL * 0.5, 20.0 + CELL * 0.5);
        assert!(gesture_frame(
            &context,
            source,
            vec![egui::Event::PointerMoved(trash_center)],
            &mut hidden
        ));
        assert!(hidden.is_empty(), "hover must not persist a removal");
        assert!(!gesture_frame(
            &context,
            source,
            vec![egui::Event::PointerMoved(egui::pos2(320.0, 90.0))],
            &mut hidden
        ));
        assert!(
            hidden.is_empty(),
            "leaving Trash must restore the prospective state"
        );
        gesture_frame(
            &context,
            source,
            vec![egui::Event::PointerMoved(trash_center)],
            &mut hidden,
        );
        gesture_frame(
            &context,
            source,
            vec![pointer_button(
                trash_center,
                egui::PointerButton::Primary,
                false,
            )],
            &mut hidden,
        );
        assert_eq!(hidden, vec![Action::FollowPlayhead]);
        assert!(!hide_action(&mut hidden, Action::FollowPlayhead));
        assert!(egui::DragAndDrop::payload::<ToolbarDrag>(&context).is_none());
    }

    #[test]
    fn secondary_movement_does_not_create_a_toolbar_drag() {
        let context = egui::Context::default();
        let source = Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(CELL, CELL));
        let mut hidden = Vec::new();
        let press = source.center();
        gesture_frame(&context, source, vec![], &mut hidden);
        gesture_frame(
            &context,
            source,
            vec![
                egui::Event::PointerMoved(press),
                pointer_button(press, egui::PointerButton::Secondary, true),
            ],
            &mut hidden,
        );
        gesture_frame(
            &context,
            source,
            vec![egui::Event::PointerMoved(press + egui::vec2(70.0, 15.0))],
            &mut hidden,
        );
        assert!(egui::DragAndDrop::payload::<ToolbarDrag>(&context).is_none());
        assert!(hidden.is_empty());
    }

    #[test]
    fn escape_over_trash_cancels_without_republishing_or_hiding() {
        let context = egui::Context::default();
        let source = Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(CELL, CELL));
        let mut hidden = Vec::new();
        let press = source.min + egui::vec2(6.0, 12.0);
        gesture_frame(&context, source, vec![], &mut hidden);
        gesture_frame(
            &context,
            source,
            vec![
                egui::Event::PointerMoved(press),
                pointer_button(press, egui::PointerButton::Primary, true),
            ],
            &mut hidden,
        );
        gesture_frame(
            &context,
            source,
            vec![egui::Event::PointerMoved(press + egui::vec2(65.0, 20.0))],
            &mut hidden,
        );
        let trash_center = egui::pos2(220.0 + CELL * 0.5, 20.0 + CELL * 0.5);
        gesture_frame(
            &context,
            source,
            vec![egui::Event::PointerMoved(trash_center)],
            &mut hidden,
        );
        gesture_frame(
            &context,
            source,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: Some(egui::Key::Escape),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut hidden,
        );
        assert!(egui::DragAndDrop::payload::<ToolbarDrag>(&context).is_none());
        gesture_frame(&context, source, vec![], &mut hidden);
        assert!(egui::DragAndDrop::payload::<ToolbarDrag>(&context).is_none());
        gesture_frame(
            &context,
            source,
            vec![pointer_button(
                trash_center,
                egui::PointerButton::Primary,
                false,
            )],
            &mut hidden,
        );
        assert!(hidden.is_empty());
    }

    #[test]
    fn active_outlines_use_accent_not_contrast_text_and_keep_width() {
        for dark in [false, true] {
            for palette in [
                crate::config::ColorPalette::Native,
                crate::config::ColorPalette::Studio,
            ] {
                let mut visuals = if dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                };
                super::super::palette::apply(&mut visuals, palette);
                visuals.selection.bg_fill = Color32::from_rgb(24, 170, 105);
                visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
                for (selected, pressed) in [(true, false), (false, true), (true, true)] {
                    let stroke = control_outline(&visuals, selected, pressed);
                    assert_eq!(stroke.color, visuals.selection.bg_fill);
                    assert_ne!(stroke.color, visuals.selection.stroke.color);
                    assert_eq!(stroke.width, 1.0);
                }
                assert_eq!(control_outline(&visuals, false, false).width, 1.0);
                assert_eq!(
                    control_outline(&visuals, false, false).color,
                    Color32::TRANSPARENT
                );
            }
        }
    }

    #[test]
    fn removal_preview_adds_a_red_strike_without_moving_the_grab_point() {
        let context = egui::Context::default();
        let drag = ToolbarDrag {
            action: Action::ZoomIn,
            source_rect: Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(CELL, CELL)),
            grab_offset: egui::vec2(5.0, 13.0),
        };
        let pointer = egui::pos2(300.0, 100.0);
        let mut regular_ink = Color32::TRANSPARENT;
        for progress in [0.0, 1.0, 0.0] {
            let mut error = Color32::TRANSPARENT;
            let mut actual = Rect::NOTHING;
            let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                error = ui.visuals().error_fg_color;
                actual = paint_drag_preview(ui, &drag, pointer, progress);
            });
            output.textures_delta.clear();
            assert_eq!(actual.min + drag.grab_offset, pointer);
            let strike = output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::epaint::Shape::LineSegment { stroke, .. } => Some(*stroke),
                _ => None,
            });
            let ink = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.fallback_color),
                    _ => None,
                })
                .expect("drag glyph");
            if progress == 0.0 {
                assert!(strike.is_none());
                if regular_ink == Color32::TRANSPARENT {
                    regular_ink = ink;
                }
                assert_eq!(ink, regular_ink, "leaving Trash restores normal ink");
            } else {
                assert_eq!(strike.unwrap().color, error);
                assert!(ink.a() < regular_ink.a(), "removal preview fades the glyph");
            }
        }
    }

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
            let mut output = context.run_ui(egui::RawInput::default(), |ui| {
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
            // `control` renders an icon glyph, so this headless test owns an
            // unapplied font-atlas texture update. Discard it explicitly just
            // as an integration would upload it before dropping `FullOutput`.
            output.textures_delta.clear();
        }
    }
}
