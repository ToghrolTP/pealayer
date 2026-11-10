//! Fixed-geometry, panel-owned controls. Drag state is deliberately distinct
//! from hardware cards and cues; persistence never runs inside DockArea.
use crate::app::PealayerApp;
use crate::config::TimelineToolbarAction as Action;
use eframe::egui::{self, Align2, Color32, FontId, Id, Rect, Response, Stroke, StrokeKind};

const CELL: f32 = super::layout::TIMELINE_RULER_HEIGHT;

#[derive(Clone, Debug)]
struct ToolbarDrag {
    action: Action,
    source_rect: Rect,
    grab_offset: egui::Vec2,
}

fn register_drag(response: &Response, action: Action, moved_for_drag: bool) {
    super::action_customization::register_drag(response, moved_for_drag, |grab_offset| ToolbarDrag {
        action, source_rect: response.rect, grab_offset,
    });
}
fn moved_for_drag(ctx: &egui::Context) -> bool { super::action_customization::moved_for_drag(ctx) }
fn drop_payload(response: &Response) -> Option<std::sync::Arc<ToolbarDrag>> {
    super::action_customization::drop_payload(response)
}
fn hide_action(hidden: &mut Vec<Action>, action: Action) -> bool {
    super::action_customization::hide_action(hidden, action)
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
    super::action_customization::outline(visuals,selected,pressed)
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
    super::action_customization::move_action(order, action, target, after)
}
fn control(ui:&mut egui::Ui,rect:Rect,id:Id,icon:&str,label:&str,enabled:bool,selected:bool)->Response {
    super::action_customization::control(ui,rect,id,icon,label,enabled,selected,false)
}
pub(super) fn draw(
    ui: &mut egui::Ui,
    app: &mut PealayerApp,
    viewport: Rect,
    can_add_keyframe: bool,
    playhead_in_view: bool,
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
            Action::BringPlayheadIntoView => !playhead_in_view,
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
    let hide_progress = crate::ui::motion::hover_alpha(ui,
        Id::new("timeline-toolbar-removal-preview"),
        hide_hovered,
    );
    let hidden = super::action_customization::menu_hide_target::<ToolbarDrag>(
        &mut toolbar, &response, drag.is_some(), FontId::proportional(14.0),
    );
    if drag.is_some() {
        if let Some(drag) = hidden {
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
                let language = app.language;
                changed |= customization(ui, &mut order, &mut app.timeline_toolbar_hidden,
                    &|key| super::i18n::tr(language, key));
                if ui
                    .button(format!(
                        "{} {}",
                        super::icons::GEAR,
                        app.tr("Preferences...")
                    ))
                    .clicked()
                {
                    super::preferences::navigate(app, ui.ctx(),
                        crate::preferences_contract::PreferencesNavigation::timeline_toolbar());
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

/// The overflow menu and Preferences edit the same ordered/hidden action model.
pub(super) fn customization(
    ui: &mut egui::Ui,
    order: &mut Vec<Action>,
    hidden: &mut Vec<Action>,
    tr: &impl Fn(&'static str) -> String,
) -> bool {
    let mut changed = false;
    for (index, action) in order.clone().into_iter().enumerate() {
        ui.push_id(action, |ui| ui.horizontal(|ui| {
            let mut visible = !hidden.contains(&action);
            if ui.checkbox(&mut visible, egui::RichText::new(format!("{}  {}",
                super::layout::timeline_toolbar_action_icon(action),
                tr(super::layout::timeline_toolbar_action_label(action)))).size(12.0)).changed() {
                hidden.retain(|candidate| *candidate != action);
                if !visible { hidden.push(action); }
                changed = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                for (enabled, icon, next, tip) in [
                    (index + 1 < order.len(), super::icons::ARROW_DOWN, (index + 1).min(order.len() - 1), "Move down"),
                    (index > 0, super::icons::ARROW_UP, index.saturating_sub(1), "Move up"),
                ] {
                    if ui.add_enabled(enabled, egui::Button::new(icon).small())
                        .on_hover_text(tr(tip)).clicked() {
                        order.swap(index, next);
                        changed = true;
                    }
                }
            });
        }));
    }
    ui.separator();
    if ui.button(format!("{} {}", super::icons::ARROW_COUNTER_CLOCKWISE, tr("Reset toolbar"))).clicked() {
        *order = crate::config::default_timeline_toolbar_order();
        *hidden = crate::config::default_timeline_toolbar_hidden();
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_playhead_disables_and_dims_reveal_but_not_customization() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = PealayerApp::default();
        app.timeline_toolbar_hidden = Action::ALL.into_iter()
            .filter(|action| *action != Action::BringPlayheadIntoView).collect();
        let ctx = egui::Context::default();
        super::super::motion::configure(&ctx, crate::config::UiAnimationMode::Disabled);
        let viewport = Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(900.0, 600.0));
        let point = egui::pos2(viewport.right() - CELL * 1.5, viewport.top() + CELL * 0.5);
        let mut frame = |events, in_view| {
            let mut action = None;
            let output = ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| {
                action = draw(ui, &mut app, viewport, false, in_view).action;
            });
            let color = output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.text() == super::super::icons::TARGET =>
                    Some(text.fallback_color),
                _ => None,
            });
            output.drop_without_applying_deltas();
            (action, color)
        };
        let (_, dimmed) = frame(vec![], true);
        frame(vec![egui::Event::PointerMoved(point), pointer_button(point, egui::PointerButton::Primary, true)], true);
        assert_eq!(frame(vec![pointer_button(point, egui::PointerButton::Primary, false)], true).0, None);
        let (_, available) = frame(vec![egui::Event::PointerMoved(egui::pos2(0.0, 0.0))], false);
        assert!(dimmed.is_some() && available.is_some());
        assert_ne!(dimmed, available, "unavailable action must be visibly dimmed");
        frame(vec![egui::Event::PointerMoved(point), pointer_button(point, egui::PointerButton::Primary, true)], false);
        assert_eq!(frame(vec![pointer_button(point, egui::PointerButton::Primary, false)], false).0, Some(Action::BringPlayheadIntoView));
    }

    #[test]
    fn reveal_availability_fades_without_delaying_its_click_guard_or_moving_the_icon() {
        let _guard = crate::app::tests::lock_app_tests();
        for animated in [false, true] {
            let ctx = egui::Context::default();
            super::super::motion::configure(&ctx, if animated {
                crate::config::UiAnimationMode::Enabled
            } else { crate::config::UiAnimationMode::Disabled });
            let mut app = PealayerApp::default();
            app.engine_handle = crate::four_d::engine::memory_only_test_engine();
            app.timeline_toolbar_hidden = Action::ALL.into_iter()
                .filter(|action| *action != Action::BringPlayheadIntoView).collect();
            let viewport = Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(900.0, 600.0));
            let point = egui::pos2(viewport.right() - CELL * 1.5, viewport.top() + CELL * 0.5);
            let mut time = 0.0;
            let mut frame = |visible, events| {
                let mut action = None;
                let mut result = None;
                let mut output = ctx.run_ui(egui::RawInput {
                    time: Some(time), predicted_dt: 0.01, events, ..Default::default()
                }, |ui| {
                    action = draw(ui, &mut app, viewport, false, visible).action;
                });
                time += 0.01;
                for shape in &output.shapes {
                    if let egui::Shape::Text(text) = &shape.shape
                        && text.galley.text() == super::super::icons::TARGET {
                        result = Some((text.fallback_color, text.pos));
                    }
                }
                output.textures_delta.clear();
                (action, result.expect("reveal icon paint"))
            };
            let (_, (available, position)) = frame(false, vec![]);
            let (_, (first, _)) = frame(true, vec![]);
            let (_, (middle, _)) = frame(true, vec![]);
            let mut colors = vec![first, middle];
            for index in 0..20 {
                let (action, (color, current_position)) = frame(true, vec![
                    egui::Event::PointerMoved(point),
                    pointer_button(point, egui::PointerButton::Primary, index % 2 == 0),
                ]);
                assert_eq!(action, None, "Click must be blocked even during the fade");
                assert_eq!(position, current_position, "Availability cannot shift the glyph");
                colors.push(color);
            }
            let dimmed = *colors.last().unwrap();
            assert_ne!(available, dimmed);
            if animated {
                assert_ne!(first, available);
                assert_ne!(middle, dimmed, "State change must not snap immediately to its final color");
                assert!(colors.windows(2).all(|pair| pair[0].r() >= pair[1].r()), "Fade must be monotonic");
            } else { assert!(colors.iter().all(|color| *color == dimmed), "Reduced motion applies immediately"); }
            frame(false, vec![egui::Event::PointerMoved(point), pointer_button(point, egui::PointerButton::Primary, true)]);
            assert_eq!(frame(false, vec![pointer_button(point, egui::PointerButton::Primary, false)]).0,
                Some(Action::BringPlayheadIntoView), "Re-enabling must not wait for the fade");
            for _ in 0..20 { frame(false, vec![egui::Event::PointerMoved(egui::Pos2::ZERO)]); }
            assert_eq!(frame(false, vec![]).1, (available, position));
        }
    }

    #[test]
    fn overflow_preferences_click_requests_exact_toolbar_page() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = PealayerApp::default();
        app.preferences_tab = 1;
        let ctx = egui::Context::default();
        let viewport = Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(900.0, 600.0));
        let mut frame = |events| {
            ctx.run_ui(egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 900.0))),
                events, ..Default::default()
            }, |ui| { draw(ui, &mut app, viewport, false, true); })
        };
        fn text_point(shape: &egui::epaint::Shape) -> Option<egui::Pos2> {
            match shape {
                egui::epaint::Shape::Text(text) if text.galley.text().contains("Preferences...") =>
                    Some(text.pos + text.galley.size() * 0.5),
                egui::epaint::Shape::Vec(shapes) => shapes.iter().find_map(text_point),
                _ => None,
            }
        }
        let menu = egui::pos2(viewport.right() - CELL * 0.5, viewport.top() + CELL * 0.5);
        frame(vec![]).drop_without_applying_deltas();
        frame(vec![egui::Event::PointerMoved(menu), pointer_button(menu, egui::PointerButton::Primary, true)]).drop_without_applying_deltas();
        frame(vec![pointer_button(menu, egui::PointerButton::Primary, false)]).drop_without_applying_deltas();
        let output = frame(vec![]);
        let preferences = output.shapes.iter().find_map(|shape| text_point(&shape.shape)).expect("overflow Preferences action visible");
        output.drop_without_applying_deltas();
        frame(vec![egui::Event::PointerMoved(preferences), pointer_button(preferences, egui::PointerButton::Primary, true)]).drop_without_applying_deltas();
        frame(vec![pointer_button(preferences, egui::PointerButton::Primary, false)]).drop_without_applying_deltas();
        drop(frame);
        assert!(app.show_preferences_dialog);
        assert!(app.preferences_draft.is_some());
        app.native_dialog_windows = false;
        ctx.run_ui(egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 900.0))),
            ..Default::default()
        }, |ui| super::super::preferences::draw(&mut app, ui)).drop_without_applying_deltas();
        assert_eq!(crate::preferences_contract::preference_sections()[app.preferences_tab].id, "timeline_toolbar");
    }

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
