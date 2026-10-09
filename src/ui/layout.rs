use crate::app::{EffectDragPayload, PealayerApp};
use crate::config::TimelineToolbarAction;
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

fn hardware_monitor_scroll<R>(
    ui: &mut egui::Ui,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::scroll_area::ScrollAreaOutput<R> {
    // Dock scrolling is disabled for this panel: it owns one bounded vertical
    // viewport so cards cannot grow the panel or strand lower sections.
    egui::ScrollArea::vertical()
        .id_salt("hardware-monitor-scroll")
        .max_height(ui.available_height())
        .auto_shrink([false, false])
        .show(ui, body)
}

pub(crate) fn left_aligned_click_label(
    ui: &mut egui::Ui,
    text: &str,
    width: f32,
    height: f32,
    size: f32,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width.max(1.0), height), egui::Sense::click());
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(size),
        ui.visuals().strong_text_color(),
    );
    ui.painter()
        .with_clip_rect(rect.intersect(ui.clip_rect()))
        .galley(
            egui::pos2(rect.left(), rect.center().y - galley.size().y * 0.5),
            galley,
            ui.visuals().strong_text_color(),
        );
    response
}

const EFFECTS_PANEL_RIGHT_GUTTER: f32 = 10.0;
// Grip, icon, three compact actions and a readable title need this minimum.
const EFFECT_CARD_MIN_WIDTH: f32 = 160.0;
const EFFECT_CARD_HORIZONTAL_MARGIN: i8 = 9;
const EFFECT_CARD_STROKE_WIDTH: f32 = 1.0;
const EFFECT_CARD_ACTION_GUTTER: f32 = 132.0;
const EFFECT_CARD_ACTION_BUTTONS_WIDTH: f32 = 96.0;
const HARDWARE_CARD_STROKE_WIDTH: f32 = 1.0;
const BOARD_IDENTITY_TWO_LINE_HEIGHT: f32 = 42.0;
const BOARD_IDENTITY_LINE_GAP: f32 = 0.0;
const EFFECT_CONTROLS_RIGHT_GUTTER: f32 = 8.0;
const EFFECT_CONTROLS_CARD_MARGIN: i8 = 10;
const TIMELINE_TRACK_HEADER_WIDTH: f32 = 250.0;
pub(super) const TIMELINE_RULER_HEIGHT: f32 = 26.0;
const TIMELINE_COMPACT_TRACK_ROW_HEIGHT: f32 = 32.0;
const TIMELINE_COMFORTABLE_TRACK_ROW_HEIGHT: f32 = 40.0;
const TIMELINE_COMPACT_ANALOG_ROW_HEIGHT: f32 = 40.0;
const TIMELINE_COMFORTABLE_ANALOG_ROW_HEIGHT: f32 = 48.0;
const TIMELINE_TRACK_STATE_BUTTON_SIZE: f32 = 22.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimelineTrackStateKind {
    Muted,
    Soloed,
    Locked,
}

fn timeline_track_state_color(kind: TimelineTrackStateKind) -> egui::Color32 {
    match kind {
        // Do not inherit the application accent: these colors communicate
        // suppressed output, isolated output, and constrained editing.
        TimelineTrackStateKind::Muted => egui::Color32::from_rgb(224, 88, 88),
        TimelineTrackStateKind::Soloed => egui::Color32::from_rgb(245, 184, 65),
        TimelineTrackStateKind::Locked => egui::Color32::from_rgb(125, 146, 160),
    }
}

fn timeline_track_visual_opacity(muted: bool, soloed: bool, locked: bool, any_soloed: bool) -> f32 {
    if muted {
        0.42
    } else if any_soloed && !soloed {
        0.36
    } else if locked {
        0.72
    } else {
        1.0
    }
}

fn timeline_track_cue_alpha(muted: bool, soloed: bool, locked: bool, any_soloed: bool) -> u8 {
    (255.0 * timeline_track_visual_opacity(muted, soloed, locked, any_soloed)).round() as u8
}

fn timeline_track_state_button(
    ui: &mut egui::Ui,
    active: bool,
    kind: TimelineTrackStateKind,
    icon: &str,
    help: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(TIMELINE_TRACK_STATE_BUTTON_SIZE, TIMELINE_TRACK_STATE_BUTTON_SIZE),
        egui::Sense::click(),
    );
    let semantic = timeline_track_state_color(kind);
    let hovered = response.hovered();
    let fill = if active {
        semantic.gamma_multiply(if hovered { 0.30 } else { 0.20 })
    } else if hovered {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        egui::Color32::TRANSPARENT
    };
    let stroke_color = if active {
        semantic
    } else if hovered {
        ui.visuals().widgets.hovered.bg_stroke.color
    } else {
        egui::Color32::TRANSPARENT
    };
    let icon_color = if active {
        semantic
    } else if hovered {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().weak_text_color()
    };

    // The exact rectangle and inside stroke never change. Hover therefore
    // cannot alter layout or nudge the glyph as a selectable label did.
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, fill);
    painter.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, stroke_color),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(13.0),
        icon_color,
    );
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.on_hover_text(help)
}
pub const TIMELINE_MIN_ZOOM: f32 = 0.5;
pub const TIMELINE_MAX_ZOOM: f32 = 2000.0;

fn timeline_track_row_height(compact: bool) -> f32 {
    if compact {
        TIMELINE_COMPACT_TRACK_ROW_HEIGHT
    } else {
        TIMELINE_COMFORTABLE_TRACK_ROW_HEIGHT
    }
}

fn timeline_analog_row_height(compact: bool) -> f32 {
    if compact {
        TIMELINE_COMPACT_ANALOG_ROW_HEIGHT
    } else {
        TIMELINE_COMFORTABLE_ANALOG_ROW_HEIGHT
    }
}

fn timeline_track_row_top(timeline_top: f32, row_index: usize, track_row_height: f32) -> f32 {
    timeline_top + TIMELINE_RULER_HEIGHT + row_index as f32 * track_row_height
}

/// Freeze only the ruler's vertical origin. Its horizontal origin remains tied
/// to content, so ticks, markers and the playhead stay aligned while panning.
fn timeline_frozen_ruler_rect(content: egui::Rect, viewport: egui::Rect) -> egui::Rect {
    let top = content.top() + viewport.top();
    egui::Rect::from_min_max(
        egui::pos2(content.left(), top),
        egui::pos2(content.right(), top + TIMELINE_RULER_HEIGHT),
    )
}

fn timeline_content_height(
    track_rows: usize,
    analog_rows: usize,
    track_row_height: f32,
    analog_row_height: f32,
) -> f32 {
    TIMELINE_RULER_HEIGHT
        + track_rows as f32 * track_row_height
        + analog_rows as f32 * analog_row_height
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InlineEffectIdentityEdit {
    name: String,
    icon: String,
}

fn request_effect_card_rename(ctx: &egui::Context, item_id: egui::Id) {
    ctx.data_mut(|data| data.insert_temp(item_id.with("rename-request"), true));
    ctx.request_repaint();
}

fn effect_card_rename_name_width(available_after_icon: f32, spacing: f32) -> f32 {
    (available_after_icon - 48.0 - spacing * 2.0).max(1.0)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct HardwareChannelDrag {
    key: String,
    kind: String,
    grab_offset: egui::Vec2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HardwareChannelDrop {
    source_key: String,
    target_key: String,
    before: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct TimelineTrackDrag {
    key: String,
    kind: String,
}

fn timeline_track_drag_id() -> egui::Id {
    egui::Id::new("timeline-track-order-drag")
}

fn timeline_track_handle_hovered(
    pointer: Option<egui::Pos2>,
    row_rect: egui::Rect,
    active: bool,
) -> bool {
    active || pointer.is_some_and(|position| row_rect.contains(position))
}

fn timeline_track_drag_handle(
    ui: &mut egui::Ui,
    control: &crate::four_d::controller::HardwareControl,
    row_rect: egui::Rect,
    help: &str,
) -> egui::Response {
    let active = ui
        .data_mut(|data| data.get_temp::<TimelineTrackDrag>(timeline_track_drag_id()))
        .is_some_and(|drag| drag.key == control.key);
    // Child widgets (including this handle) take hover ownership away from the
    // row's background `Response`. Use geometric containment instead so the
    // handle cannot make itself fade out when the pointer reaches it.
    let hovered = timeline_track_handle_hovered(ui.ctx().pointer_hover_pos(), row_rect, active);
    let alpha = ui.ctx().animate_bool_with_time(
        egui::Id::new(("timeline-track-order-handle", control.key.as_str())),
        hovered,
        0.12,
    );
    let response = ui
        .add(
            egui::Label::new(
                egui::RichText::new(crate::ui::icons::DOTS_SIX_VERTICAL)
                    .size(14.0)
                    .color(ui.visuals().weak_text_color().gamma_multiply(alpha)),
            )
            .sense(egui::Sense::drag()),
        )
        .on_hover_text(help);
    if response.drag_started_by(egui::PointerButton::Primary) {
        ui.data_mut(|data| {
            data.insert_temp(
                timeline_track_drag_id(),
                TimelineTrackDrag {
                    key: control.key.clone(),
                    kind: control.kind.clone(),
                },
            );
        });
    }
    if response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    response
}

fn timeline_track_drop_target(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    control: &crate::four_d::controller::HardwareControl,
) -> Option<HardwareChannelDrop> {
    let drag = ui.data_mut(|data| data.get_temp::<TimelineTrackDrag>(timeline_track_drag_id()))?;
    if drag.key == control.key || drag.kind != control.kind {
        return None;
    }
    let pointer = ui.ctx().pointer_hover_pos()?;
    if !rect.contains(pointer) {
        return None;
    }
    let before = pointer.y < rect.center().y;
    let y = if before { rect.top() } else { rect.bottom() };
    ui.painter().line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
    );
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    if ui.input(|input| input.pointer.any_released()) {
        ui.data_mut(|data| data.remove_temp::<TimelineTrackDrag>(timeline_track_drag_id()));
        return Some(HardwareChannelDrop {
            source_key: drag.key,
            target_key: control.key.clone(),
            before,
        });
    }
    None
}

fn clear_released_timeline_track_drag(ui: &mut egui::Ui) {
    if ui.input(|input| input.pointer.any_released()) {
        ui.data_mut(|data| data.remove_temp::<TimelineTrackDrag>(timeline_track_drag_id()));
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum HardwareChannelDragSurface {
    Monitor,
    Manager,
}

fn hardware_channel_drag_id(ui: &egui::Ui, surface: HardwareChannelDragSurface) -> egui::Id {
    egui::Id::new(("hardware-channel-drag", surface, ui.ctx().viewport_id()))
}

fn hardware_channel_source_rect_id(
    ui: &egui::Ui,
    surface: HardwareChannelDragSurface,
    key: &str,
) -> egui::Id {
    egui::Id::new(("hardware-channel-source-rect", surface, ui.ctx().viewport_id(), key))
}

pub(crate) fn clear_released_hardware_channel_drag(
    ui: &mut egui::Ui,
    surface: HardwareChannelDragSurface,
) {
    if ui.input(|input| input.pointer.button_released(egui::PointerButton::Primary)) {
        let id = hardware_channel_drag_id(ui, surface);
        ui.data_mut(|data| {
            data.remove_temp::<HardwareChannelDrag>(id);
        });
    }
}

fn hardware_channel_handle_hovered(
    pointer: Option<egui::Pos2>,
    card_rect: Option<egui::Rect>,
    current_row_rect: egui::Rect,
    active: bool,
) -> bool {
    active
        || pointer.is_some_and(|pointer| {
            card_rect.is_some_and(|rect| rect.contains(pointer))
                || current_row_rect.contains(pointer)
        })
}

pub(crate) fn hardware_channel_is_dragging(
    ui: &mut egui::Ui,
    key: &str,
    surface: HardwareChannelDragSurface,
) -> bool {
    let id = hardware_channel_drag_id(ui, surface);
    ui.data_mut(|data| data.get_temp::<HardwareChannelDrag>(id))
        .is_some_and(|drag| drag.key == key)
}

pub(crate) fn hardware_channel_drag_handle(
    app: &PealayerApp,
    ui: &mut egui::Ui,
    control: &crate::four_d::controller::HardwareControl,
    surface: HardwareChannelDragSurface,
) -> egui::Response {
    let source_rect_id = hardware_channel_source_rect_id(ui, surface, &control.key);
    let drag_id = hardware_channel_drag_id(ui, surface);
    let active = ui
        .data_mut(|data| data.get_temp::<HardwareChannelDrag>(drag_id))
        .is_some_and(|drag| drag.key == control.key);
    // `ui.max_rect()` inside a horizontal row begins at the current cursor and
    // therefore excludes the icon/indicator area that users naturally hover.
    // Use the complete card/row rectangle recorded on the preceding frame so
    // the affordance reacts anywhere over the channel, with the current row as
    // a first-frame fallback.
    let source_rect = ui.data_mut(|data| data.get_temp::<egui::Rect>(source_rect_id));
    let pointer = ui.ctx().pointer_hover_pos();
    let hovered = hardware_channel_handle_hovered(pointer, source_rect, ui.max_rect(), active);
    let alpha = ui.ctx().animate_bool_with_time(
        source_rect_id.with("handle-visible"),
        hovered,
        0.12,
    );
    let color = ui.visuals().weak_text_color().gamma_multiply(alpha);
    let response = ui
        .add_sized(
            [18.0, 24.0],
            egui::Label::new(
                egui::RichText::new(crate::ui::icons::DOTS_SIX_VERTICAL)
                    .color(color)
                    .size(15.0),
            )
            .sense(egui::Sense::drag()),
        )
        .on_hover_text(app.tr("Drag to reorder channel"));
    if response.drag_started() {
        let source_rect = source_rect.unwrap_or(response.rect);
        let grab_offset = ui
            .ctx()
            .pointer_interact_pos()
            .unwrap_or(source_rect.center())
            - source_rect.min;
        ui.data_mut(|data| {
            data.insert_temp(
                drag_id,
                HardwareChannelDrag {
                    key: control.key.clone(),
                    kind: control.kind.clone(),
                    grab_offset,
                },
            );
        });
    }
    if response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    response
}

pub(crate) fn finish_hardware_channel_card(
    ui: &mut egui::Ui,
    control: &crate::four_d::controller::HardwareControl,
    card_rect: egui::Rect,
    layer_id: egui::LayerId,
    surface: HardwareChannelDragSurface,
) {
    let source_rect_id = hardware_channel_source_rect_id(ui, surface, &control.key);
    let drag_id = hardware_channel_drag_id(ui, surface);
    ui.data_mut(|data| data.insert_temp(source_rect_id, card_rect));
    let drag = ui.data_mut(|data| data.get_temp::<HardwareChannelDrag>(drag_id));
    if let Some(drag) = drag.filter(|drag| drag.key == control.key)
        && let Some(pointer) = ui.ctx().pointer_interact_pos()
    {
        let translation = drag_translation(pointer, card_rect.min, drag.grab_offset);
        ui.ctx().transform_layer_shapes(
            layer_id,
            egui::emath::TSTransform::from_translation(translation),
        );
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
}

pub(crate) fn hardware_channel_drop_target(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    control: &crate::four_d::controller::HardwareControl,
    surface: HardwareChannelDragSurface,
) -> Option<HardwareChannelDrop> {
    let drag_id = hardware_channel_drag_id(ui, surface);
    let drag =
        ui.data_mut(|data| data.get_temp::<HardwareChannelDrag>(drag_id))?;
    if drag.key == control.key || drag.kind != control.kind {
        return None;
    }
    let pointer = ui.ctx().pointer_hover_pos()?;
    if !rect.contains(pointer) {
        return None;
    }
    let before = pointer.y < rect.center().y;
    let y = if before { rect.top() } else { rect.bottom() };
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 5.0, y),
            egui::pos2(rect.right() - 5.0, y),
        ],
        egui::Stroke::new(2.0, ui.visuals().selection.stroke.color),
    );
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    if ui.input(|input| input.pointer.button_released(egui::PointerButton::Primary)) {
        ui.data_mut(|data| {
            data.remove_temp::<HardwareChannelDrag>(drag_id);
        });
        return Some(HardwareChannelDrop {
            source_key: drag.key,
            target_key: control.key.clone(),
            before,
        });
    }
    None
}

pub(crate) fn timeline_keyboard_focus_id() -> egui::Id {
    egui::Id::new("timeline-keyboard-focus")
}

fn delete_selected_cues_without_timeline_focus(
    app: &mut PealayerApp,
    ui: &egui::Ui,
) -> bool {
    if app.selected_instance_ids.is_empty()
        || app.active_drag.is_some()
        || ui.ctx().text_edit_focused()
        || egui::Popup::is_any_open(ui.ctx())
        || ui
            .ctx()
            .memory(|memory| memory.has_focus(timeline_keyboard_focus_id()))
        || !ui.input(|input| input.key_pressed(egui::Key::Delete))
    {
        return false;
    }

    let removed = app.delete_selected_timeline_cues();
    if removed > 0 {
        ui.ctx().request_repaint();
        true
    } else {
        false
    }
}

fn pan_timeline_offset(
    offset: egui::Vec2,
    pointer_delta: egui::Vec2,
    content_size: egui::Vec2,
    viewport_size: egui::Vec2,
) -> egui::Vec2 {
    let max_offset = egui::vec2(
        (content_size.x - viewport_size.x).max(0.0),
        (content_size.y - viewport_size.y).max(0.0),
    );
    egui::vec2(
        (offset.x - pointer_delta.x).clamp(0.0, max_offset.x),
        (offset.y - pointer_delta.y).clamp(0.0, max_offset.y),
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TimelineWheelAction {
    Zoom(f32),
    HorizontalScroll(f32),
    VerticalScroll(f32),
    MultiGesture {
        zoom_factor: f32,
        translation: egui::Vec2,
    },
}

fn timeline_wheel_action(
    delta: egui::Vec2,
    behavior: crate::config::TimelineWheelBehavior,
) -> Option<TimelineWheelAction> {
    // Never reinterpret a physical horizontal gesture as zoom, even with modifiers.
    if delta.x != 0.0 {
        return Some(TimelineWheelAction::HorizontalScroll(delta.x));
    }
    if delta.y == 0.0 { return None; }
    match behavior {
        crate::config::TimelineWheelBehavior::Zoom => Some(TimelineWheelAction::Zoom(delta.y)),
        crate::config::TimelineWheelBehavior::HorizontalScroll => Some(TimelineWheelAction::HorizontalScroll(delta.y)),
        crate::config::TimelineWheelBehavior::VerticalScroll => Some(TimelineWheelAction::VerticalScroll(delta.y)),
        crate::config::TimelineWheelBehavior::None => None,
    }
}

fn timeline_wheel_behavior(app: &PealayerApp, modifiers: egui::Modifiers) -> crate::config::TimelineWheelBehavior {
    if modifiers.shift { app.timeline_shift_wheel_action }
    else if modifiers.ctrl || modifiers.command { app.timeline_ctrl_wheel_action }
    else if modifiers.alt { app.timeline_alt_wheel_action }
    else { app.timeline_plain_wheel_action }
}

fn timeline_wheel_modifiers(input: &egui::InputState) -> egui::Modifiers {
    input.events.iter().rev().find_map(|event| match event {
        egui::Event::MouseWheel { modifiers, .. } => Some(*modifiers),
        _ => None,
    }).unwrap_or(input.modifiers)
}

fn timeline_wheel_over_surface(ui: &egui::Ui, surface: egui::Rect,
    behavior: crate::config::TimelineWheelBehavior) -> Option<(TimelineWheelAction, f32)> {
    if !ui.rect_contains_pointer(surface) { return None; }
    // Surface-class touchscreens expose pan and pinch together. Preserve both
    // axes and keep the content under the center of the fingers.
    if let Some(touch) = ui.input(|input| input.multi_touch())
        && surface.contains(touch.center_pos)
        && ((touch.zoom_delta - 1.0).abs() > f32::EPSILON
            || touch.translation_delta != egui::Vec2::ZERO)
    {
        ui.ctx().input_mut(|input| input.smooth_scroll_delta = egui::Vec2::ZERO);
        return Some((TimelineWheelAction::MultiGesture {
            zoom_factor: touch.zoom_delta.max(0.01),
            translation: touch.translation_delta,
        }, touch.center_pos.x));
    }
    // Precision-trackpad pinch is native Event::Zoom on Windows and macOS.
    if let Some(zoom_factor) = ui.input(|input| timeline_pinch_factor(&input.events)) {
        ui.ctx().input_mut(|input| {
            input.smooth_scroll_delta = egui::Vec2::ZERO;
            input.events.retain(|event| !matches!(event, egui::Event::Zoom(_)));
        });
        return Some((TimelineWheelAction::MultiGesture {
            zoom_factor,
            translation: egui::Vec2::ZERO,
        }, ui.ctx().pointer_latest_pos().map_or(surface.center().x, |pos| pos.x)));
    }
    let line_speed = ui.ctx().options(|options| options.input_options.line_scroll_speed);
    let delta = ui.input(|input| timeline_wheel_delta(&input.events, line_speed, surface.height()));
    // None also consumes the gesture, rather than falling through to ScrollArea.
    ui.ctx().input_mut(|input| input.smooth_scroll_delta = egui::Vec2::ZERO);
    timeline_wheel_action(delta, behavior).map(|action| (action,
        ui.ctx().pointer_latest_pos().map_or(surface.left(), |pos| pos.x)))
}

fn timeline_pinch_factor(events: &[egui::Event]) -> Option<f32> {
    let factor = events.iter().filter_map(|event| match event {
        egui::Event::Zoom(factor) if factor.is_finite() && *factor > 0.0 => Some(*factor),
        _ => None,
    }).product::<f32>();
    ((factor - 1.0).abs() > f32::EPSILON).then_some(factor)
}

// Read the original axes for every modifier: egui otherwise converts Ctrl to zoom
// and Shift to horizontal scrolling before configurable routing can inspect it.
fn timeline_wheel_delta(events: &[egui::Event], line_speed: f32, page_height: f32) -> egui::Vec2 {
    events.iter().filter_map(|event| match event {
        egui::Event::MouseWheel { unit, delta, .. } => {
                let scale = match unit {
                    egui::MouseWheelUnit::Point => 1.0,
                    egui::MouseWheelUnit::Line => line_speed,
                    egui::MouseWheelUnit::Page => page_height,
                };
                Some(*delta * scale)
            }
        _ => None,
    }).fold(egui::Vec2::ZERO, |sum, delta| sum + delta)
}

fn timeline_ruler_scroll_steps_from_events(events: &[egui::Event]) -> i32 {
    let mut steps: i32 = 0;
    for event in events {
        if let egui::Event::MouseWheel { delta, unit, .. } = event {
            let count = match unit {
                egui::MouseWheelUnit::Line | egui::MouseWheelUnit::Page => {
                    if delta.y > 0.0 {
                        1
                    } else if delta.y < 0.0 {
                        -1
                    } else {
                        0
                    }
                }
                egui::MouseWheelUnit::Point => {
                    if delta.y >= 1.0 {
                        1
                    } else if delta.y <= -1.0 {
                        -1
                    } else {
                        0
                    }
                }
            };
            steps += count;
        }
    }
    steps
}

fn timeline_ruler_scroll_steps(ui: &egui::Ui) -> i32 {
    let steps = ui.input(|input| {
        timeline_ruler_scroll_steps_from_events(&input.events)
    });

    let has_wheel = ui.input(|input| {
        input.smooth_scroll_delta != egui::Vec2::ZERO
            || input.events.iter().any(|e| matches!(e, egui::Event::MouseWheel { .. }))
    });

    if has_wheel {
        ui.ctx().input_mut(|input| {
            input.smooth_scroll_delta = egui::Vec2::ZERO;
            input.events.retain(|event| !matches!(event, egui::Event::MouseWheel { .. }));
        });
    }

    steps
}

fn timeline_zoom_from_wheel(current_zoom: f32, wheel_delta: f32) -> f32 {
    // Multiplicative zoom feels uniform at both ends of the range. A 120-unit
    // Windows wheel notch changes the scale by roughly 27%, while precision
    // touchpads still produce small, smooth increments.
    (current_zoom * (wheel_delta * 0.002).exp()).clamp(TIMELINE_MIN_ZOOM, TIMELINE_MAX_ZOOM)
}

fn timeline_offset_for_pointer_zoom(
    current_offset: f32,
    pointer_x_in_viewport: f32,
    current_zoom: f32,
    next_zoom: f32,
    duration_seconds: f64,
    viewport_width: f32,
) -> f32 {
    if current_zoom <= 0.0 || !current_zoom.is_finite() || !next_zoom.is_finite() {
        return current_offset;
    }
    let pointer_x_in_viewport = pointer_x_in_viewport.clamp(0.0, viewport_width.max(0.0));
    let seconds_under_pointer = (current_offset + pointer_x_in_viewport) / current_zoom;
    let next_content_width = (duration_seconds.max(0.0) as f32 * next_zoom).max(0.0);
    let max_offset = (next_content_width - viewport_width).max(0.0);
    (seconds_under_pointer * next_zoom - pointer_x_in_viewport).clamp(0.0, max_offset)
}

fn constrain_middle_pan_delta(
    pointer_delta: egui::Vec2,
    shift: bool,
    command_or_ctrl: bool,
    axis_lock_modifiers: bool,
) -> egui::Vec2 {
    if !axis_lock_modifiers || (shift && command_or_ctrl) {
        pointer_delta
    } else if shift {
        egui::vec2(pointer_delta.x, 0.0)
    } else if command_or_ctrl {
        egui::vec2(0.0, pointer_delta.y)
    } else {
        pointer_delta
    }
}

fn timeline_offset_to_reveal_x(
    current_offset: f32,
    target_x: f32,
    content_width: f32,
    viewport_width: f32,
) -> f32 {
    let max_offset = (content_width - viewport_width).max(0.0);
    let margin = 24.0_f32.min(viewport_width * 0.2);
    let visible_left = current_offset + margin;
    let visible_right = current_offset + viewport_width - margin;

    let revealed = if target_x < visible_left {
        target_x - margin
    } else if target_x > visible_right {
        target_x - viewport_width + margin
    } else {
        current_offset
    };

    revealed.clamp(0.0, max_offset)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineZoomAnchor {
    pub pointer_x_in_viewport: f32,
    pub media_time_seconds: f64,
    pub duration_seconds: f64,
    pub viewport_width: f32,
}

fn timeline_follow_target_offset(
    current_offset: f32,
    playhead_x: f32,
    content_width: f32,
    viewport_width: f32,
) -> Option<f32> {
    if viewport_width <= 1.0 || content_width <= viewport_width {
        return None;
    }
    let position_in_view = playhead_x - current_offset;
    // Preserve useful look-ahead instead of waiting for the playhead to touch
    // the edge. A backward seek outside the viewport is recovered as well.
    let target = if position_in_view >= viewport_width * 0.82 {
        playhead_x - viewport_width * 0.62
    } else if position_in_view < 0.0 {
        playhead_x - viewport_width * 0.20
    } else {
        return None;
    };
    Some(target.clamp(0.0, (content_width - viewport_width).max(0.0)))
}

pub(super) fn timeline_toolbar_action_label(action: TimelineToolbarAction) -> &'static str {
    match action {
        TimelineToolbarAction::ZoomIn => "Zoom in",
        TimelineToolbarAction::ZoomOut => "Zoom out",
        TimelineToolbarAction::PanLeft => "Pan left",
        TimelineToolbarAction::PanRight => "Pan right",
        TimelineToolbarAction::BringPlayheadIntoView => "Bring playhead into view",
        TimelineToolbarAction::FollowPlayhead => "Keep playhead in view",
        TimelineToolbarAction::AddKeyframe => "Add keyframe at playhead",
        TimelineToolbarAction::PreviousCue => "Select previous cue",
        TimelineToolbarAction::NextCue => "Select next cue",
        TimelineToolbarAction::NudgeCueLeft => "Nudge selected cues left",
        TimelineToolbarAction::NudgeCueRight => "Nudge selected cues right",
        TimelineToolbarAction::SelectAll => "Select all cues",
        TimelineToolbarAction::ClearSelection => "Clear selection",
        TimelineToolbarAction::DeleteSelection => "Delete selection",
    }
}

pub(super) fn timeline_toolbar_action_icon(action: TimelineToolbarAction) -> &'static str {
    match action {
        TimelineToolbarAction::ZoomIn => crate::ui::icons::PLUS,
        TimelineToolbarAction::ZoomOut => crate::ui::icons::MINUS,
        TimelineToolbarAction::PanLeft => crate::ui::icons::CARET_LEFT,
        TimelineToolbarAction::PanRight => crate::ui::icons::CARET_RIGHT,
        TimelineToolbarAction::BringPlayheadIntoView => crate::ui::icons::TARGET,
        TimelineToolbarAction::FollowPlayhead => crate::ui::icons::LOCK,
        TimelineToolbarAction::AddKeyframe => crate::ui::icons::DIAMOND,
        TimelineToolbarAction::PreviousCue => crate::ui::icons::SKIP_BACK,
        TimelineToolbarAction::NextCue => crate::ui::icons::SKIP_FORWARD,
        TimelineToolbarAction::NudgeCueLeft => crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
        TimelineToolbarAction::NudgeCueRight => crate::ui::icons::ARROW_CLOCKWISE,
        TimelineToolbarAction::SelectAll => crate::ui::icons::SELECTION_ALL,
        TimelineToolbarAction::ClearSelection => crate::ui::icons::ERASER,
        TimelineToolbarAction::DeleteSelection => crate::ui::icons::TRASH,
    }
}

pub(super) fn timeline_toolbar_action_shortcut(action: TimelineToolbarAction) -> &'static str {
    match action {
        TimelineToolbarAction::ZoomIn => "+ / =",
        TimelineToolbarAction::ZoomOut => "-",
        TimelineToolbarAction::PanLeft => "Shift+Left",
        TimelineToolbarAction::PanRight => "Shift+Right",
        TimelineToolbarAction::BringPlayheadIntoView => "C",
        TimelineToolbarAction::FollowPlayhead => "Ctrl+Shift+L",
        TimelineToolbarAction::AddKeyframe => "K",
        TimelineToolbarAction::PreviousCue => "Shift+Tab",
        TimelineToolbarAction::NextCue => "Tab",
        TimelineToolbarAction::NudgeCueLeft => "Alt+Left",
        TimelineToolbarAction::NudgeCueRight => "Alt+Right",
        TimelineToolbarAction::SelectAll => "Ctrl+A",
        TimelineToolbarAction::ClearSelection => "Esc",
        TimelineToolbarAction::DeleteSelection => "Delete",
    }
}

fn timeline_two_line_menu_label(
    ui: &egui::Ui,
    icon: &str,
    label: &str,
    detail: &str,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &format!("{icon} {label}"),
        0.0,
        egui::TextFormat {
            font_id: egui::TextStyle::Button.resolve(ui.style()),
            color: ui.visuals().text_color(),
            ..Default::default()
        },
    );
    job.append(
        &format!("\n   {detail}"),
        0.0,
        egui::TextFormat {
            font_id: egui::TextStyle::Small.resolve(ui.style()),
            color: ui.visuals().weak_text_color(),
            ..Default::default()
        },
    );
    job
}

fn timeline_pointer_time_ms(
    pointer_x: f32,
    timeline_left: f32,
    px_per_ms: f32,
    duration_ms: u64,
) -> u64 {
    if !pointer_x.is_finite()
        || !timeline_left.is_finite()
        || !px_per_ms.is_finite()
        || px_per_ms <= 0.0
    {
        return 0;
    }
    (((pointer_x - timeline_left).max(0.0) / px_per_ms).round() as u64).min(duration_ms)
}

fn timeline_ruler_context_time_id() -> egui::Id {
    egui::Id::new("timeline_ruler_context_time_ms")
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct TimelineNavigationTransition {
    start_offset: egui::Vec2,
    target_offset: egui::Vec2,
    start_zoom: f32,
    target_zoom: f32,
    started_at_seconds: f64,
    duration_seconds: f32,
    anchor: Option<TimelineZoomAnchor>,
}

fn timeline_ruler_owns_pointer(
    ruler: egui::Rect,
    toolbar: egui::Rect,
    pointer: egui::Pos2,
    dragging_ruler: bool,
) -> bool {
    dragging_ruler || (ruler.contains(pointer) && !toolbar.contains(pointer))
}

fn start_timeline_navigation_transition(
    ui: &egui::Ui,
    id: egui::Id,
    start: (egui::Vec2, f32),
    target: (egui::Vec2, f32),
    duration_ms: u32,
) {
    start_anchored_timeline_navigation_transition(ui, id, start, target, duration_ms, None);
}

fn start_anchored_timeline_navigation_transition(
    ui: &egui::Ui,
    id: egui::Id,
    start: (egui::Vec2, f32),
    target: (egui::Vec2, f32),
    duration_ms: u32,
    anchor: Option<TimelineZoomAnchor>,
) {
    // All Context accessors use the same non-reentrant lock. Snapshot input
    // before the data transaction; never read input or request repaint inside it.
    let transition = TimelineNavigationTransition {
        start_offset: start.0,
        target_offset: target.0,
        start_zoom: start.1,
        target_zoom: target.1,
        started_at_seconds: ui.input(|input| input.time),
        duration_seconds: duration_ms as f32 / 1_000.0,
        anchor,
    };
    ui.data_mut(|data| data.insert_temp(id, transition));
    ui.ctx().request_repaint();
}

fn sample_timeline_navigation_transition(
    transition: TimelineNavigationTransition,
    now_seconds: f64,
) -> (egui::Vec2, f32, bool) {
    let duration = f64::from(transition.duration_seconds.max(f32::EPSILON));
    let elapsed = (now_seconds - transition.started_at_seconds).max(0.0);
    let complete = elapsed + 1.0e-6 >= duration;
    let progress = if complete {
        1.0
    } else {
        (elapsed / duration).clamp(0.0, 1.0)
    };
    // Smoothstep starts and stops gently while remaining deterministic and
    // short enough that navigation never feels disconnected from its action.
    let eased = progress * progress * (3.0 - 2.0 * progress);
    let interpolate = |start: f32, target: f32| {
        (f64::from(start) + f64::from(target - start) * eased) as f32
    };
    let eased_zoom = interpolate(transition.start_zoom, transition.target_zoom);
    let eased_offset_y = interpolate(transition.start_offset.y, transition.target_offset.y);
    let eased_offset_x = if let Some(anchor) = transition.anchor {
        let next_content_width = (anchor.duration_seconds.max(0.0) as f32 * eased_zoom).max(0.0);
        let max_offset = (next_content_width - anchor.viewport_width).max(0.0);
        let desired = anchor.media_time_seconds as f32 * eased_zoom - anchor.pointer_x_in_viewport;
        desired.clamp(0.0, max_offset)
    } else {
        interpolate(transition.start_offset.x, transition.target_offset.x)
    };
    (
        egui::vec2(eased_offset_x, eased_offset_y),
        eased_zoom,
        complete,
    )
}

fn fit_timeline_to_window(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    viewport_width: f32,
    total_seconds: f64,
    timeline_scroll_state: &mut egui::scroll_area::State,
    navigation_transition_id: egui::Id,
) {
    if total_seconds > 0.0 && viewport_width > 0.0 {
        let fit_zoom = (viewport_width / total_seconds as f32)
            .clamp(TIMELINE_MIN_ZOOM, TIMELINE_MAX_ZOOM);
        if app.timeline_animated_navigation {
            let transition = TimelineNavigationTransition {
                start_offset: timeline_scroll_state.offset,
                target_offset: egui::vec2(0.0, timeline_scroll_state.offset.y),
                start_zoom: app.timeline_zoom,
                target_zoom: fit_zoom,
                started_at_seconds: ui.input(|input| input.time),
                duration_seconds: app.timeline_navigation_transition_ms as f32 / 1_000.0,
                anchor: None,
            };
            ui.data_mut(|data| {
                data.insert_temp(navigation_transition_id, transition);
            });
            ui.ctx().request_repaint();
        } else {
            app.timeline_zoom = fit_zoom;
            timeline_scroll_state.offset.x = 0.0;
        }
    }
}

fn timeline_frame_step_ms(media_fps: f64, frame_count: u32) -> u64 {
    if media_fps.is_finite() && media_fps > 0.0 {
        ((1_000.0 * f64::from(frame_count.max(1))) / media_fps)
            .round()
            .max(1.0) as u64
    } else {
        // Unknown frame rate still needs an exact, deterministic nudge.
        u64::from(frame_count.max(1))
    }
}

fn timeline_snap_tolerance_ms(px_per_ms: f32) -> u64 {
    if px_per_ms <= f32::EPSILON {
        return 100;
    }
    (8.0 / px_per_ms).round().clamp(1.0, 250.0) as u64
}

fn timeline_snap_targets(
    timeline: &crate::four_d::models::Timeline,
    selected_instances: &std::collections::HashSet<uuid::Uuid>,
    playhead_ms: u64,
) -> Vec<u64> {
    let mut targets = vec![0, playhead_ms];
    targets.extend(timeline.keyframes.iter().map(|keyframe| keyframe.time_ms));
    targets.extend(
        timeline
            .analog_tracks
            .iter()
            .flat_map(|track| track.keyframes.iter().map(|keyframe| keyframe.time_ms)),
    );
    for instance in &timeline.instances {
        if selected_instances.contains(&instance.id) {
            continue;
        }
        if let Some(effect) = timeline
            .templates
            .iter()
            .find(|effect| effect.id == instance.effect_id)
        {
            targets.push(instance.start_time_ms);
            if !effect.is_state_marker() {
                targets.push(instance.start_time_ms.saturating_add(effect.duration_ms));
            }
        }
    }
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn nearest_snap_time(value: u64, targets: &[u64], tolerance_ms: u64) -> Option<u64> {
    targets
        .iter()
        .copied()
        .filter_map(|target| {
            let distance = value.abs_diff(target);
            (distance <= tolerance_ms).then_some((distance, target))
        })
        .min_by_key(|(distance, target)| (*distance, *target))
        .map(|(_, target)| target)
}

fn adjacent_timeline_time(current: u64, forwards: bool, targets: &[u64]) -> Option<u64> {
    if forwards {
        targets.iter().copied().find(|target| *target > current)
    } else {
        targets
            .iter()
            .rev()
            .copied()
            .find(|target| *target < current)
    }
}

fn move_selected_cues(
    timeline: &mut crate::four_d::models::Timeline,
    selected: &std::collections::HashSet<uuid::Uuid>,
    delta_ms: i64,
) -> bool {
    let Some(first_start) = timeline
        .instances
        .iter()
        .filter(|instance| selected.contains(&instance.id))
        .map(|instance| instance.start_time_ms)
        .min()
    else {
        return false;
    };
    let applied_delta = delta_ms.max(-(first_start as i64));
    if applied_delta == 0 {
        return false;
    }
    for instance in &mut timeline.instances {
        if selected.contains(&instance.id) {
            instance.start_time_ms = (instance.start_time_ms as i64 + applied_delta).max(0) as u64;
        }
    }
    true
}

fn sorted_cue_ids(timeline: &crate::four_d::models::Timeline) -> Vec<uuid::Uuid> {
    let mut cues: Vec<_> = timeline
        .instances
        .iter()
        .map(|instance| (instance.start_time_ms, instance.id))
        .collect();
    cues.sort_unstable();
    cues.into_iter().map(|(_, id)| id).collect()
}

fn effects_panel_content_width(available_width: f32) -> f32 {
    (available_width - EFFECTS_PANEL_RIGHT_GUTTER).max(EFFECT_CARD_MIN_WIDTH)
}

fn effects_frame_content_width(outer_width: f32) -> f32 {
    (outer_width - (f32::from(EFFECT_CARD_HORIZONTAL_MARGIN) + EFFECT_CARD_STROKE_WIDTH) * 2.0)
        .max(1.0)
}

fn effect_card_header_widths(available_after_icon: f32, item_spacing: f32) -> (f32, f32) {
    // Four 24 px action buttons plus the three gaps between them. The title and
    // action container are separate horizontal-layout items, so their own gap
    // must also be removed from the title budget. Omitting that final gap made
    // every child card one spacing unit wider than its group header.
    let actions_width = (EFFECT_CARD_ACTION_BUTTONS_WIDTH + item_spacing * 3.0)
        .min((available_after_icon - item_spacing - 12.0).max(1.0));
    let title_width = (available_after_icon - actions_width - item_spacing).max(1.0);
    (title_width, actions_width)
}

struct EffectCardHeaderResponse {
    grip: egui::Response,
    icon: egui::Response,
    title: egui::Response,
    rename: egui::Response,
    run: egui::Response,
    place: egui::Response,
    more: egui::Response,
}

fn effect_library_card_header(
    ui: &mut egui::Ui,
    id: egui::Id,
    icon: &str,
    title: &str,
    primary_action_icon: &str,
    primary_action_enabled: bool,
    tooltips: [&str; 7],
) -> EffectCardHeaderResponse {
    ui.horizontal(|ui| {
        // Compact spacing is local to a narrow card, never a global theme change.
        if ui.available_width() < 180.0 {
            ui.spacing_mut().item_spacing.x = ui.spacing().item_spacing.x.min(2.0);
        }
        let source_rect = ui.data(|data| data.get_temp::<egui::Rect>(id.with("source-rect")));
        let hovered = hardware_channel_handle_hovered(
            ui.ctx().pointer_hover_pos(),
            source_rect,
            ui.max_rect(),
            primary_effect_drag_active(ui.ctx(), id),
        );
        let hover = ui
            .ctx()
            .animate_bool_with_time(id.with("grip-hover"), hovered, 0.12);
        // Discoverable even at rest; hover emphasis must not change row geometry.
        let grip = ui
            .add_sized(
                [14.0, 24.0],
                egui::Label::new(
                    egui::RichText::new(crate::ui::icons::DOTS_SIX_VERTICAL)
                        .size(14.0)
                        .color(
                            ui.visuals()
                                .weak_text_color()
                                .gamma_multiply(0.45 + 0.55 * hover),
                        ),
                )
                .sense(egui::Sense::hover()),
            )
            .on_hover_text(tooltips[0])
            .on_hover_cursor(egui::CursorIcon::Grab);
        // The existing whole-card primary drag surface owns the gesture; adding
        // a competing drag ID on this glyph would break payload/offset handling.
        let icon = ui
            .add_sized(
                [20.0, 24.0],
                egui::Button::new(egui::RichText::new(icon).size(16.0)).frame(false),
            )
            .on_hover_text(tooltips[1]);
        let (title_width, actions_width) =
            effect_card_header_widths(ui.available_width(), ui.spacing().item_spacing.x);
        let action_button_width = ((actions_width - ui.spacing().item_spacing.x * 3.0) / 4.0).max(1.0);
        let title = ui
            .allocate_ui_with_layout(
                egui::vec2(title_width, 24.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    // allocate_ui_with_layout is content-sized unless we hold this
                    // slot open. A short caption otherwise pulls all actions left.
                    ui.set_min_width(title_width);
                    ui.add(
                        egui::Label::new(egui::RichText::new(title).strong())
                            .truncate()
                            .sense(egui::Sense::click()),
                    )
                },
            )
            .inner
            .on_hover_text(tooltips[2]);
        let (more, place, run, rename) = ui
            .allocate_ui_with_layout(
                egui::vec2(actions_width, 24.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.set_min_width(actions_width);
                    let more = ui
                        .add_sized(
                            [action_button_width, 24.0],
                            egui::Button::new(crate::ui::icons::DOTS_THREE).frame(false),
                        )
                        .on_hover_text(tooltips[5]);
                    let place = ui
                        .add_sized(
                            [action_button_width, 24.0],
                            egui::Button::new(crate::ui::icons::PLUS).frame(false),
                        )
                        .on_hover_text(tooltips[4]);
                    let run = ui
                        .add_enabled_ui(primary_action_enabled, |ui| {
                            ui.add_sized(
                                [action_button_width, 24.0],
                                egui::Button::new(primary_action_icon).frame(false),
                            )
                        })
                        .inner
                        .on_hover_text(tooltips[3]);
                    let rename = ui
                        .add_sized(
                            [action_button_width, 24.0],
                            egui::Button::new(crate::ui::icons::PENCIL_SIMPLE).frame(false),
                        )
                        .on_hover_text(tooltips[6]);
                    (more, place, run, rename)
                },
            )
            .inner;
        EffectCardHeaderResponse {
            grip,
            icon,
            title,
            rename,
            run,
            place,
            more,
        }
    })
    .inner
}

fn effect_card<R>(
    ui: &mut egui::Ui,
    outer_width: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    // The Effects panel owns the card width. Deriving the width again from the
    // Frame child feeds its margins back into egui's sizing pass, which made
    // cards grow on successive paints and resized the drag preview.
    ui.set_width(outer_width);
    let stroke_color = ui.visuals().widgets.noninteractive.bg_stroke.color;
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(EFFECT_CARD_HORIZONTAL_MARGIN, 7))
        .stroke(egui::Stroke::new(EFFECT_CARD_STROKE_WIDTH, stroke_color))
        .corner_radius(8.0)
        .show(ui, |ui| {
            let content_width = effects_frame_content_width(outer_width);
            ui.set_width(content_width);
            add_contents(ui)
        })
}

fn effect_group_header<R>(
    ui: &mut egui::Ui,
    outer_width: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    // Category rows and their child cards are peers in the Effects Library,
    // so they deliberately use the same outer/content width contract. Keep
    // this separate from `effect_card` because the header has a lighter fill
    // and tighter vertical padding, not because it is a different width.
    ui.set_width(outer_width);
    let fill = ui.visuals().widgets.inactive.weak_bg_fill;
    let stroke_color = ui.visuals().widgets.noninteractive.bg_stroke.color;
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(EFFECT_CARD_STROKE_WIDTH, stroke_color))
        .corner_radius(7.0)
        .inner_margin(egui::Margin::symmetric(EFFECT_CARD_HORIZONTAL_MARGIN, 6))
        .show(ui, |ui| {
            let content_width = effects_frame_content_width(outer_width);
            ui.set_width(content_width);
            add_contents(ui)
    })
}

fn effect_group_disclosure_icon(open: bool, count: usize) -> Option<&'static str> {
    (count > 0).then_some(if open {
        crate::ui::icons::CARET_DOWN
    } else {
        crate::ui::icons::CARET_RIGHT
    })
}

fn effect_group_action_header(
    ui: &mut egui::Ui,
    outer_width: f32,
    title: &str,
    icon: &str,
    open: bool,
    count: usize,
    add_tooltip: &str,
) -> (egui::Response, egui::Response) {
    let empty = count == 0;
    effect_group_header(ui, outer_width, |ui| {
        ui.horizontal(|ui| {
            let spacing = ui.spacing().item_spacing.x;
            let count_text = count.to_string();
            let count_width = ui.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap(
                        count_text.clone(),
                        egui::TextStyle::Small.resolve(ui.style()),
                        ui.visuals().text_color(),
                    )
                    .size()
                    .x
            }) + 16.0;
            let title_width = (ui.available_width() - 24.0 - count_width - spacing * 2.0).max(1.0);
            let title_response = ui
                .allocate_ui_with_layout(
                    egui::vec2(title_width, 24.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.style_mut().interaction.selectable_labels = false;
                        ui.set_width(title_width);
                        if let Some(disclosure) = effect_group_disclosure_icon(open, count) {
                            ui.label(disclosure);
                        }
                        let icon_text = egui::RichText::new(icon);
                        ui.label(if empty { icon_text.weak() } else { icon_text });
                        let title_text = egui::RichText::new(title).strong();
                        ui.add(
                            egui::Label::new(if empty {
                                title_text.weak()
                            } else {
                                title_text
                            })
                            .truncate(),
                        );
                    },
                )
                .response
                .interact(if empty {
                    egui::Sense::hover()
                } else {
                    egui::Sense::click()
                });
            let add_response = ui
                .add_sized([24.0, 24.0], egui::Button::new(crate::ui::icons::PLUS))
                .on_hover_text(add_tooltip);
            egui::Frame::new()
                .fill(ui.visuals().widgets.inactive.weak_bg_fill)
                .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                .corner_radius(9.0)
                .inner_margin(egui::Margin::symmetric(7, 2))
                .show(ui, |ui| {
                    let count_text = egui::RichText::new(count_text).small().strong();
                    ui.label(if empty { count_text.weak() } else { count_text });
                });
            (title_response, add_response)
        })
        .inner
    })
    .inner
}

/// Neutral metadata never inherits the accent selection fill. Reserve the
/// duration first so changing labels cannot move it away from the card's edge.
fn effect_library_metadata(
    ui: &mut egui::Ui,
    kind: &str,
    duration: &str,
) -> (egui::Rect, egui::Rect) {
    ui.horizontal(|ui| {
        let duration_text = format!("{} {duration}", crate::ui::icons::CLOCK);
        let duration_width = ui.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(
                    duration_text.clone(),
                    egui::TextStyle::Small.resolve(ui.style()),
                    ui.visuals().weak_text_color(),
                )
                .size()
                .x
        });
        let kind_width =
            (ui.available_width() - duration_width - ui.spacing().item_spacing.x).max(1.0);
        let kind_rect = ui
            .allocate_ui_with_layout(
                egui::vec2(kind_width, 22.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_width(kind_width);
                    egui::Frame::new()
                        .fill(ui.visuals().widgets.inactive.weak_bg_fill)
                        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                        .corner_radius(11.0)
                        .inner_margin(egui::Margin::symmetric(7, 2))
                        .show(ui, |ui| {
                            ui.set_max_width((kind_width - 16.0).max(1.0));
                            ui.add(
                                egui::Label::new(egui::RichText::new(kind).small().weak())
                                    .truncate(),
                            );
                        })
                        .response
                        .rect
                },
            )
            .inner;
        let duration_rect = ui
            .add_sized(
                [duration_width, 22.0],
                egui::Label::new(egui::RichText::new(duration_text).small().weak()),
            )
            .rect;
        (kind_rect, duration_rect)
    })
    .inner
}

fn effect_controls_content_width(available_width: f32) -> f32 {
    (available_width - EFFECT_CONTROLS_RIGHT_GUTTER).max(1.0)
}

fn effect_controls_frame_content_width(outer_width: f32) -> f32 {
    (outer_width - f32::from(EFFECT_CONTROLS_CARD_MARGIN) * 2.0 - 2.0).max(1.0)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct EffectControlsHeaderResponse {
    previous: bool,
    next: bool,
}

fn effect_controls_panel_header(
    ui: &mut egui::Ui,
    outer_width: f32,
    selected_count: usize,
    cue_count: usize,
    title: &str,
    subtitle: &str,
    selection_label: &str,
) -> EffectControlsHeaderResponse {
    let mut response = EffectControlsHeaderResponse::default();
    let can_navigate = cue_count > 1 || (cue_count == 1 && selected_count == 0);
    egui::Frame::new()
        .fill(ui.visuals().widgets.noninteractive.weak_bg_fill)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(11, 9))
        .show(ui, |ui| {
            ui.set_width(effect_controls_frame_content_width(outer_width));
            ui.horizontal(|ui| {
                egui::Frame::new()
                    .fill(ui.visuals().selection.bg_fill.gamma_multiply(0.16))
                    .corner_radius(7.0)
                    .inner_margin(egui::Margin::same(7))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(crate::ui::icons::SLIDERS_HORIZONTAL)
                                .size(18.0)
                                .color(ui.visuals().selection.bg_fill),
                        );
                    });
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(title).strong().size(15.0));
                    ui.label(egui::RichText::new(subtitle).small().weak());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    effect_controls_badge(
                        ui,
                        crate::ui::icons::SELECTION_ALL,
                        selection_label,
                    );
                    response.next = ui
                        .add_enabled(
                            can_navigate,
                            egui::Button::new(crate::ui::icons::SKIP_FORWARD)
                                .frame(false)
                                .min_size(egui::vec2(28.0, 28.0)),
                        )
                        .on_hover_text("Select next cue (Tab)")
                        .clicked();
                    response.previous = ui
                        .add_enabled(
                            can_navigate,
                            egui::Button::new(crate::ui::icons::SKIP_BACK)
                                .frame(false)
                                .min_size(egui::vec2(28.0, 28.0)),
                        )
                        .on_hover_text("Select previous cue (Shift+Tab)")
                        .clicked();
                });
            });
        });
    response
}

fn effect_controls_timing_overview(
    ui: &mut egui::Ui,
    start_time_ms: u64,
    duration_ms: u64,
    total_time_ms: u64,
    playhead_time_ms: u64,
) -> egui::Response {
    let total_time_ms = total_time_ms.max(start_time_ms.saturating_add(duration_ms)).max(1);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 30.0),
        egui::Sense::hover(),
    );
    let track = egui::Rect::from_min_max(
        egui::pos2(rect.left(), rect.center().y - 4.0),
        egui::pos2(rect.right(), rect.center().y + 4.0),
    );
    ui.painter().rect_filled(track, 4.0, ui.visuals().widgets.inactive.bg_fill);
    let x_for_time = |time_ms: u64| {
        egui::lerp(
            track.left()..=track.right(),
            (time_ms.min(total_time_ms) as f32 / total_time_ms as f32).clamp(0.0, 1.0),
        )
    };
    let cue_start = x_for_time(start_time_ms);
    let cue_end = x_for_time(start_time_ms.saturating_add(duration_ms));
    let cue_rect = egui::Rect::from_min_max(
        egui::pos2(cue_start, track.top() - 3.0),
        egui::pos2((cue_end.max(cue_start + 3.0)).min(track.right()), track.bottom() + 3.0),
    );
    ui.painter().rect_filled(
        cue_rect,
        5.0,
        ui.visuals().selection.bg_fill.gamma_multiply(0.82),
    );
    let playhead_x = x_for_time(playhead_time_ms);
    ui.painter().line_segment(
        [egui::pos2(playhead_x, rect.top() + 2.0), egui::pos2(playhead_x, rect.bottom() - 2.0)],
        egui::Stroke::new(1.5, egui::Color32::from_rgb(231, 76, 60)),
    );
    response.on_hover_text(format!(
        "{} → {} · {}",
        crate::duration::format_time_value_ms(start_time_ms),
        crate::duration::format_time_value_ms(start_time_ms.saturating_add(duration_ms)),
        crate::duration::format_time_value_ms(duration_ms),
    ))
}

fn effect_controls_relay_state(
    ui: &mut egui::Ui,
    value: &mut u16,
    on_label: &str,
    off_label: &str,
) {
    let gap = ui.spacing().item_spacing.x;
    let button_width = ((ui.available_width() - gap) / 2.0).max(70.0);
    ui.horizontal(|ui| {
        let on = *value >= 5_000;
        let on_color = egui::Color32::from_rgb(38, 166, 91);
        let off_color = egui::Color32::from_rgb(125, 146, 160);
        if ui
            .add_sized(
                [button_width, 30.0],
                egui::Button::new(
                    egui::RichText::new(format!("{}  {on_label}", crate::ui::icons::POWER))
                        .color(if on { egui::Color32::WHITE } else { ui.visuals().text_color() }),
                )
                .fill(if on { on_color } else { ui.visuals().widgets.inactive.weak_bg_fill })
                .stroke(egui::Stroke::new(1.0, if on { on_color } else { ui.visuals().widgets.inactive.bg_stroke.color })),
            )
            .clicked()
        {
            *value = 10_000;
        }
        if ui
            .add_sized(
                [button_width, 30.0],
                egui::Button::new(
                    egui::RichText::new(format!("{}  {off_label}", crate::ui::icons::STOP_CIRCLE))
                        .color(if !on { egui::Color32::WHITE } else { ui.visuals().text_color() }),
                )
                .fill(if !on { off_color } else { ui.visuals().widgets.inactive.weak_bg_fill })
                .stroke(egui::Stroke::new(1.0, if !on { off_color } else { ui.visuals().widgets.inactive.bg_stroke.color })),
            )
            .clicked()
        {
            *value = 0;
        }
    });
}

fn effect_controls_card<R>(
    ui: &mut egui::Ui,
    outer_width: f32,
    icon: &str,
    title: &str,
    subtitle: Option<&str>,
    emphasized: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    effect_controls_card_with_ping(
        ui,
        outer_width,
        icon,
        title,
        subtitle,
        emphasized,
        0.0,
        add_contents,
    )
}

fn effect_controls_card_with_ping<R>(
    ui: &mut egui::Ui,
    outer_width: f32,
    icon: &str,
    title: &str,
    subtitle: Option<&str>,
    emphasized: bool,
    ping_strength: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.set_width(outer_width);
    let visuals = ui.visuals();
    let fill = if ping_strength > 0.0 {
        let base_mult = if visuals.dark_mode { 0.18 } else { 0.10 };
        visuals
            .selection
            .bg_fill
            .gamma_multiply((base_mult + 0.18 * ping_strength).min(1.0))
    } else if emphasized {
        visuals
            .selection
            .bg_fill
            .gamma_multiply(if visuals.dark_mode { 0.18 } else { 0.10 })
    } else {
        visuals.widgets.noninteractive.bg_fill
    };
    let stroke = if ping_strength > 0.0 {
        let stroke_width = 1.0 + 2.0 * ping_strength;
        let stroke_alpha = (0.72 + 0.28 * ping_strength).min(1.0);
        egui::Stroke::new(
            stroke_width,
            visuals.selection.bg_fill.gamma_multiply(stroke_alpha),
        )
    } else if emphasized {
        egui::Stroke::new(1.0_f32, visuals.selection.bg_fill.gamma_multiply(0.72))
    } else {
        visuals.widgets.noninteractive.bg_stroke
    };
    let card = egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(EFFECT_CONTROLS_CARD_MARGIN, 11))
        .show(ui, |ui| {
            ui.set_width(effect_controls_frame_content_width(outer_width));
            ui.horizontal(|ui| {
                let icon_color = if ping_strength > 0.0 || emphasized {
                    ui.visuals().selection.bg_fill
                } else {
                    ui.visuals().strong_text_color()
                };
                egui::Frame::new()
                    .fill(icon_color.gamma_multiply(if emphasized { 0.18 } else { 0.10 }))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::same(6))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(icon).size(17.0).color(icon_color));
                    });
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(title).strong().size(14.0));
                    if let Some(subtitle) = subtitle.filter(|value| !value.trim().is_empty()) {
                        ui.label(egui::RichText::new(subtitle).small().weak());
                    }
                });
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(7.0);
            add_contents(ui)
        });
    if emphasized {
        let stripe = egui::Rect::from_min_max(
            card.response.rect.min,
            egui::pos2(card.response.rect.min.x + 3.0, card.response.rect.max.y),
        );
        ui.painter().rect_filled(stripe, 10.0, ui.visuals().selection.bg_fill);
    }
    card
}

fn effect_controls_badge(ui: &mut egui::Ui, icon: &str, text: impl Into<String>) {
    let visuals = ui.visuals();
    egui::Frame::new()
        .fill(visuals.widgets.inactive.weak_bg_fill)
        .stroke(visuals.widgets.noninteractive.bg_stroke)
        .corner_radius(20.0)
        .inner_margin(egui::Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(format!("{icon}  {}", text.into())).small());
        });
}

fn effect_controls_kind(
    effect: &crate::four_d::models::Effect,
) -> (&'static str, &'static str, &'static str) {
    if let Some(direct) = effect.direct_control.as_ref() {
        (
            if direct.control_key.starts_with("relay.") {
                crate::ui::icons::PLUG
            } else {
                crate::ui::icons::SLIDERS_HORIZONTAL
            },
            "Direct channel cue",
            "Timeline effect",
        )
    } else if effect.controller_strip_effect.is_some() {
        (
            crate::ui::icons::SPARKLE,
            "Addressable lighting",
            "PCController effect",
        )
    } else if effect.controller_macro.is_some() {
        (
            crate::ui::icons::WAVEFORM,
            "Hardware macro",
            "PCController effect",
        )
    } else {
        (crate::ui::icons::PLUG, "Relay sequence", "Timeline effect")
    }
}

fn hardware_frame_content_width(outer_width: f32, horizontal_margin: i8) -> f32 {
    (outer_width - (f32::from(horizontal_margin) + HARDWARE_CARD_STROKE_WIDTH) * 2.0).max(1.0)
}

/// Keep controls visibly distinct from the nearly-white Hardware Monitor card
/// surface. The global light palette deliberately stays subtle, but its
/// inactive button fill is otherwise only one RGB level away from a window
/// card and visually disappears. Scope this stronger surface to hardware cards
/// so the rest of the application's control palette is unaffected.
fn configure_hardware_card_controls(ui: &mut egui::Ui) {
    configure_hardware_card_visuals(ui.visuals_mut());
}

fn configure_hardware_card_visuals(visuals: &mut egui::Visuals) {
    if visuals.dark_mode {
        return;
    }

    // A restrained cool-gray ramp keeps every hardware action visibly
    // clickable on the almost-white card surface without making the sidebar
    // look heavy. Cover every widget state: leaving `active` or `open` on the
    // global palette caused the surface to disappear or jump to an unrelated
    // accent treatment as focus and menus changed.
    visuals.button_frame = true;
    let widgets = &mut visuals.widgets;
    widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(227, 231, 236);
    widgets.inactive.bg_fill = egui::Color32::from_rgb(227, 231, 236);
    widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(184, 192, 202));
    widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(217, 223, 230);
    widgets.hovered.bg_fill = egui::Color32::from_rgb(217, 223, 230);
    widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(144, 156, 169));
    widgets.active.weak_bg_fill = egui::Color32::from_rgb(206, 214, 223);
    widgets.active.bg_fill = egui::Color32::from_rgb(206, 214, 223);
    widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(120, 134, 150));
    widgets.open = widgets.active;
    widgets.noninteractive.weak_bg_fill = egui::Color32::from_rgb(238, 240, 243);
    widgets.noninteractive.bg_fill = egui::Color32::from_rgb(238, 240, 243);
    widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(207, 212, 218));
}

fn pwm_percent(raw: u16) -> f64 {
    f64::from(raw.min(4095)) * 100.0 / 4095.0
}

pub(crate) fn pwm_raw(percent: f64) -> u16 {
    (percent.clamp(0.0, 100.0) * 4095.0 / 100.0).round() as u16
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PwmEditorResponse {
    changed: bool,
    committed: bool,
    dragging: bool,
}

fn pwm_editor_widths(row_width: f32, gap: f32) -> (f32, f32) {
    let number_width = 76.0_f32.min((row_width - gap - 56.0).max(52.0));
    let slider_width = (row_width - number_width - gap).max(1.0);
    (slider_width, number_width)
}

impl PwmEditorResponse {
    pub(crate) fn should_transmit(self, live_updates: bool) -> bool {
        if live_updates {
            self.changed
        } else {
            self.committed
        }
    }
}

fn pwm_wheel_steps(ui: &egui::Ui, response: &egui::Response) -> f64 {
    if !response.hovered() {
        return 0.0;
    }
    ui.ctx().input_mut(|input| {
        let (steps, consume_scroll) = pwm_wheel_gesture(&input.events, input.smooth_scroll_delta);
        if consume_scroll {
            // The hovered PWM slider owns the complete wheel gesture, including
            // egui's smoothed tail on frames that contain no new MouseWheel
            // event. Otherwise that tail leaks into the enclosing Hardware
            // Monitor ScrollArea and moves the panel after changing the value.
            input.smooth_scroll_delta = egui::Vec2::ZERO;
        }
        steps
    })
}

fn pwm_wheel_gesture(events: &[egui::Event], smooth_delta: egui::Vec2) -> (f64, bool) {
    let steps = events
        .iter()
        .filter_map(|event| match event {
            egui::Event::MouseWheel { delta, .. } => {
                let delta = if delta.y.abs() >= delta.x.abs() {
                    delta.y
                } else {
                    delta.x
                };
                (delta != 0.0).then_some(f64::from(delta.signum()))
            }
            _ => None,
        })
        .sum::<f64>();
    (steps, steps != 0.0 || smooth_delta != egui::Vec2::ZERO)
}

pub(crate) fn draw_pwm_editor_row(
    ui: &mut egui::Ui,
    percent: &mut f64,
    enabled: bool,
) -> PwmEditorResponse {
    draw_pwm_editor_row_sized(ui, percent, enabled, ui.available_width())
}

fn draw_pwm_editor_row_sized(
    ui: &mut egui::Ui,
    percent: &mut f64,
    enabled: bool,
    row_width: f32,
) -> PwmEditorResponse {
    let mut outcome = PwmEditorResponse::default();
    ui.allocate_ui_with_layout(
        egui::vec2(row_width, 26.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            let gap = ui.spacing().item_spacing.x;
            let (slider_width, number_width) = pwm_editor_widths(row_width, gap);
            let slider = ui
                .scope(|ui| {
                    // `Slider` uses the style's slider width even when wrapped in
                    // `add_sized`, so scope the computed card width here instead
                    // of changing every slider in the application.
                    ui.spacing_mut().slider_width = slider_width;
                    if !ui.visuals().dark_mode {
                        let visuals = ui.visuals_mut();
                        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(209, 213, 219);
                        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(191, 199, 210);
                        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(167, 177, 191);
                    }
                    ui.add_enabled(
                        enabled,
                        egui::Slider::new(percent, 0.0..=100.0)
                            .show_value(false)
                            .step_by(0.1),
                    )
                })
                .inner;
            let wheel_steps = if enabled {
                pwm_wheel_steps(ui, &slider)
            } else {
                0.0
            };
            if wheel_steps != 0.0 {
                *percent = (*percent + wheel_steps).clamp(0.0, 100.0);
            }
            let value = ui
                .add_enabled_ui(enabled, |ui| {
                    ui.add_sized(
                        [number_width, 24.0],
                        egui::DragValue::new(percent)
                            .range(0.0..=100.0)
                            .speed(0.1)
                            .fixed_decimals(1)
                            .suffix("%")
                            .max_decimals(1),
                    )
                })
                .inner;
            outcome.changed = slider.changed() || value.changed() || wheel_steps != 0.0;
            outcome.dragging = slider.dragged();
            outcome.committed = slider.drag_stopped()
                || value.lost_focus()
                || (value.changed() && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                || wheel_steps != 0.0;
        },
    );
    outcome
}

pub(crate) fn hardware_control_activated(
    app: &PealayerApp,
    ui: &egui::Ui,
    response: &egui::Response,
) -> bool {
    hardware_control_activation(
        app.hardware_actions_on_press,
        response.is_pointer_button_down_on(),
        ui.input(|input| input.pointer.button_pressed(egui::PointerButton::Primary)),
        response.clicked(),
    )
}

fn hardware_control_activation(
    activate_on_press: bool,
    pointer_down_on_control: bool,
    primary_pressed_this_frame: bool,
    clicked_on_release: bool,
) -> bool {
    if activate_on_press {
        pointer_down_on_control && primary_pressed_this_frame
    } else {
        clicked_on_release
    }
}

fn send_pwm_raw_with(
    sender: &std::sync::mpsc::Sender<crate::four_d::engine::EngineMessage>,
    channel: u8,
    raw: u16,
) {
    let percent = f64::from(raw.min(4095)) * 100.0 / 4095.0;
    let _ = sender.send(crate::four_d::engine::EngineMessage::CoalescedControllerIntent {
        created:std::time::Instant::now(),
        control_key: format!("pwm.{channel}"),
        method: "controller.pwm.set".to_string(),
        params: serde_json::json!({"channel": channel, "percent": percent}),
        refresh_catalog: false,
    });
}

const PWM_LIVE_INTERVAL: std::time::Duration = std::time::Duration::from_micros(33_334);
const PWM_STALE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(300);

fn pwm_transmit_due(
    response: PwmEditorResponse,
    live_updates: bool,
    elapsed: Option<std::time::Duration>,
    value_changed: bool,
) -> bool {
    if response.committed && !response.dragging {
        return true;
    }
    if !live_updates || !response.changed {
        return false;
    }
    let elapsed = elapsed.unwrap_or(PWM_STALE_INTERVAL);
    (value_changed && elapsed >= PWM_LIVE_INTERVAL) || elapsed >= PWM_STALE_INTERVAL
}

pub(crate) fn transmit_pwm_editor_response(
    app: &PealayerApp,
    ui: &mut egui::Ui,
    channel: u8,
    raw: u16,
    response: PwmEditorResponse,
) {
    transmit_pwm_editor_response_with(
        &app.engine_handle.sender,
        app.live_pwm_updates,
        ui,
        channel,
        raw,
        response,
    );
}

fn transmit_pwm_editor_response_with(
    sender: &std::sync::mpsc::Sender<crate::four_d::engine::EngineMessage>,
    live_updates: bool,
    ui: &mut egui::Ui,
    channel: u8,
    raw: u16,
    response: PwmEditorResponse,
) {
    let sent_id = ui.make_persistent_id(("pwm_sent_value", channel));
    let last_send_id = ui.make_persistent_id(("pwm_last_send", channel));
    let now = std::time::Instant::now();
    let last_sent = ui.data_mut(|data| data.get_temp::<u16>(sent_id));
    let last_send = ui.data_mut(|data| data.get_temp::<std::time::Instant>(last_send_id));
    let due = pwm_transmit_due(
        response,
        live_updates,
        last_send.map(|last| now.duration_since(last)),
        last_sent != Some(raw),
    );
    // A release/commit is deliberately exempt from throttling, even when the
    // rounded value matches the last live tick. That final acknowledged write
    // makes the board converge on exactly what the user sees.
    if due {
        send_pwm_raw_with(sender, channel, raw);
        ui.data_mut(|data| {
            data.insert_temp(sent_id, raw);
            data.insert_temp(last_send_id, now);
        });
    } else if response.changed {
        ui.ctx().request_repaint_after(PWM_LIVE_INTERVAL);
    }
}

fn draw_pwm_card_editor(
    app: &PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
    channel: &crate::four_d::controller::HardwareOutput,
    row_width: f32,
) {
    let value_id = ui.make_persistent_id(("pwm_value", channel.id));
    let sent_id = ui.make_persistent_id(("pwm_sent_value", channel.id));
    let last_send_id = ui.make_persistent_id(("pwm_last_send", channel.id));
    let telemetry_raw = capabilities
        .telemetry
        .pwm_values
        .get(usize::from(channel.id))
        .copied()
        .flatten()
        .or_else(|| {
            (capabilities.telemetry.pwm_channel == Some(channel.id))
                .then_some(capabilities.telemetry.pwm_value.unwrap_or(0))
        })
        .unwrap_or(0);
    let local_raw = ui.data_mut(|data| data.get_temp::<u16>(value_id));
    let last_sent = ui.data_mut(|data| data.get_temp::<u16>(sent_id));
    let last_send = ui.data_mut(|data| data.get_temp::<std::time::Instant>(last_send_id));
    let awaiting_readback = last_sent.is_some_and(|sent| sent != telemetry_raw)
        && last_send.is_some_and(|sent_at| sent_at.elapsed() < std::time::Duration::from_secs(1));
    let displayed_raw = if awaiting_readback {
        local_raw.unwrap_or(telemetry_raw)
    } else {
        telemetry_raw
    };
    let mut percent = if awaiting_readback {
        pwm_percent(displayed_raw)
    } else {
        capabilities.pwm_percent(channel.id, displayed_raw)
    };
    let response = draw_pwm_editor_row_sized(ui, &mut percent, !control.locked, row_width);
    let raw = pwm_raw(percent);
    ui.data_mut(|data| data.insert_temp(value_id, raw));
    if response.changed {
        ui.ctx().request_repaint();
    }
    transmit_pwm_editor_response(app, ui, channel.id, raw, response);
}

fn begin_effect_drag(ctx: &egui::Context, payload: EffectDragPayload) {
    egui::DragAndDrop::set_payload(ctx, payload);
}

fn take_effect_drop_on_rect(
    ctx: &egui::Context,
    rect: egui::Rect,
) -> Option<std::sync::Arc<EffectDragPayload>> {
    let released_inside = ctx.input(|input| {
        input.pointer.button_released(egui::PointerButton::Primary)
            && input
                .pointer
                .latest_pos()
                .is_some_and(|position| rect.contains(position))
    });
    released_inside.then(|| egui::DragAndDrop::take_payload::<EffectDragPayload>(ctx))?
}

fn effect_preset_reference(preset: &crate::app::EffectPreset) -> Option<String> {
    match preset.source {
        crate::app::EffectPresetSource::ControllerMacro(id) => Some(format!("effect:{id}")),
        crate::app::EffectPresetSource::ControllerStrip => preset
            .effect
            .controller_strip_effect
            .as_ref()
            .map(|effect| effect.id.trim())
            .filter(|id| !id.is_empty())
            .map(|id| format!("effect:{id}")),
    }
}

fn secondary_click_inside(ctx: &egui::Context, rect: egui::Rect) -> bool {
    ctx.input(|input| {
        input
            .pointer
            .button_released(egui::PointerButton::Secondary)
            && input
                .pointer
                .latest_pos()
                .is_some_and(|position| rect.contains(position))
    })
}

fn primary_effect_drag_active(ctx: &egui::Context, id: egui::Id) -> bool {
    ctx.is_being_dragged(id) && ctx.input(|input| input.pointer.primary_down())
}

fn remember_effect_drag_offset_on_press(
    ctx: &egui::Context,
    rect: egui::Rect,
    offset_id: egui::Id,
) {
    let press_origin = ctx.input(|input| {
        input
            .pointer
            .primary_down()
            .then(|| input.pointer.press_origin())
            .flatten()
            .filter(|position| rect.contains(*position))
    });
    if let Some(position) = press_origin {
        ctx.data_mut(|data| data.insert_temp(offset_id, position - rect.min));
    }
}

fn effect_drag_source<R>(
    ui: &mut egui::Ui,
    id: egui::Id,
    payload: EffectDragPayload,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    effect_drag_source_with_action_gutter(ui, id, payload, 0.0, add_contents)
}

fn effect_drag_source_with_action_gutter<R>(
    ui: &mut egui::Ui,
    id: egui::Id,
    payload: EffectDragPayload,
    action_gutter: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let offset_id = id.with("pointer-offset");
    let source_rect_id = id.with("source-rect");
    // A `Sense::drag` response may be promoted to the active drag before this
    // function gets another chance to lay out the normal card. Keep the last
    // laid-out source rect so the press frame can always record the grab point
    // before branching on `is_being_dragged`.
    if let Some(source_rect) = ui.data_mut(|data| data.get_temp::<egui::Rect>(source_rect_id)) {
        remember_effect_drag_offset_on_press(ui.ctx(), source_rect, offset_id);
    }
    // `Context::is_being_dragged` is deliberately button-agnostic. A moved
    // secondary click can therefore make it true as well, but right-click is
    // reserved exclusively for the effect context menu. Only a held primary
    // button may enter the preview/payload path.
    if primary_effect_drag_active(ui.ctx(), id) {
        egui::DragAndDrop::set_payload(ui.ctx(), payload);
        let layer_id = egui::LayerId::new(egui::Order::Tooltip, id);
        let response = ui.scope_builder(egui::UiBuilder::new().layer_id(layer_id), |ui| {
            // A drag preview must remain visible after leaving the narrow
            // Effects Library ScrollArea.
            ui.set_clip_rect(ui.ctx().content_rect());
            add_contents(ui)
        });
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
        // Preserve the original, untransformed source rectangle for the whole
        // gesture. Replacing it with the preview rectangle makes the stored
        // grab offset drift toward the pointer and causes the visible jump.
        response
    } else {
        let response = ui.scope(add_contents);
        // Capture the exact grab point on mouse-down, before egui promotes the
        // gesture to a drag on a later frame. Recording this only from
        // `drag_started()` is too late: by then this function can already be in
        // the active-drag branch and the preview falls back to its center.
        remember_effect_drag_offset_on_press(ui.ctx(), response.response.rect, offset_id);
        ui.data_mut(|data| data.insert_temp(source_rect_id, response.response.rect));
        // Action buttons are real child widgets and must own their hover,
        // click, keyboard, and tooltip behavior. Keep the card drag target out
        // of their right-hand strip instead of overlaying a competing drag
        // response over them.
        let mut drag_rect = response.response.rect;
        drag_rect.max.x = (drag_rect.max.x - action_gutter).max(drag_rect.min.x);
        let drag = ui
            .interact(drag_rect, id, egui::Sense::drag())
            .on_hover_cursor(egui::CursorIcon::Grab);
        if drag.drag_started_by(egui::PointerButton::Primary) {
            // Establish the payload in the same input frame in which egui
            // claims the drag. Waiting until the next paint left a race where
            // the pointer could enter (and even be released over) the timeline
            // before any drop payload existed.
            begin_effect_drag(ui.ctx(), payload);
        }
        egui::InnerResponse::new(response.inner, drag | response.response)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TimelineTrackKind {
    Video(Option<i64>),
    Audio(i64),
    Subtitle(i64),
    ControllerEffect(crate::four_d::models::ControllerEffectLane),
    Relay(u8),
    Hardware(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TimelineTrackRow {
    key: String,
    name: String,
    detail: Option<String>,
    active: bool,
    enabled: bool,
    linked: bool,
    visible: bool,
    icon: String,
    control_key: Option<String>,
    relay_ids: Vec<u8>,
    dimmed: bool,
    kind: TimelineTrackKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimelineCueDraftAction {
    Relay { enabled: bool },
    Pwm { value_basis_points: u16 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TimelineCueDraft {
    track_name: String,
    control_key: String,
    start_time_ms: u64,
    duration_ms: u64,
    behavior: crate::four_d::models::DirectCueBehavior,
    end_value_basis_points: u16,
    action: TimelineCueDraftAction,
    error: Option<String>,
}

impl TimelineCueDraft {
    fn value_basis_points(&self) -> u16 {
        match self.action {
            TimelineCueDraftAction::Relay { enabled } => {
                if enabled { 10_000 } else { 0 }
            }
            TimelineCueDraftAction::Pwm { value_basis_points } => value_basis_points,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TimelineCuePlacement {
    track_index: usize,
    relay_id: Option<u8>,
    analog_index: Option<usize>,
    selected_track_key: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimelineMarqueeMode {
    /// CAD-style window selection: the cue must be completely enclosed.
    Window,
    /// CAD-style crossing selection: touching any part of the cue is enough.
    Crossing,
}

fn timeline_marquee_mode(origin: egui::Pos2, current: egui::Pos2) -> TimelineMarqueeMode {
    if current.x >= origin.x {
        TimelineMarqueeMode::Window
    } else {
        TimelineMarqueeMode::Crossing
    }
}

fn timeline_marquee_selects_rect(
    marquee: egui::Rect,
    candidate: egui::Rect,
    mode: TimelineMarqueeMode,
) -> bool {
    match mode {
        TimelineMarqueeMode::Window => {
            marquee.contains(candidate.min) && marquee.contains(candidate.max)
        }
        TimelineMarqueeMode::Crossing => marquee.intersects(candidate),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TimelineTrackPatch {
    pub linked: Option<bool>,
    pub visible: Option<bool>,
    pub muted: Option<bool>,
    pub soloed: Option<bool>,
    pub locked: Option<bool>,
    pub selected: Option<bool>,
}

fn timeline_track_row_id(key: &str) -> egui::Id {
    egui::Id::new(("timeline_track_row", key))
}

fn timeline_analog_track_row_id(key: &str) -> egui::Id {
    egui::Id::new(("timeline_analog_track_row", key))
}

fn timeline_track_bring_into_view_id() -> egui::Id {
    egui::Id::new("timeline_track_bring_into_view")
}

fn timeline_track_picker_item_is_dimmed(row: &TimelineTrackRow) -> bool {
    !row.linked || !row.visible
}

fn request_timeline_track_into_view(
    app: &mut PealayerApp,
    ctx: &egui::Context,
    filter_id: egui::Id,
    row: &TimelineTrackRow,
) {
    // "Bring into view" is useful even for a currently hidden or unlinked
    // track. Make the track renderable first, clear a filter that could still
    // exclude it, then let the next layout pass scroll its real row into view.
    if !row.linked {
        app.set_timeline_track_linked(&row.key, true);
    }
    if !row.visible {
        app.set_timeline_track_visible(&row.key, true);
    }
    ctx.data_mut(|data| {
        data.insert_temp(filter_id, String::new());
        data.insert_temp(timeline_track_bring_into_view_id(), row.key.clone());
    });
    ctx.request_repaint();
}

fn scroll_requested_timeline_track_into_view(
    ui: &mut egui::Ui,
    key: &str,
    rect: egui::Rect,
) -> bool {
    let requested = ui.ctx().data_mut(|data| {
        data.get_temp::<String>(timeline_track_bring_into_view_id())
            .is_some_and(|requested| requested == key)
    });
    if requested {
        // This row lives inside egui_dock's scrollable tab body. Scrolling the
        // allocated row rectangle keeps the track caption and its canvas lane
        // aligned instead of guessing an offset from the item index.
        ui.scroll_to_rect(rect, Some(egui::Align::Center));
        ui.ctx().data_mut(|data| {
            data.remove_temp::<String>(timeline_track_bring_into_view_id());
        });
    }
    requested
}

fn manage_timeline_track(app: &mut PealayerApp, row: &TimelineTrackRow) {
    if let Some(control_key) = row.control_key.as_deref() {
        if let Some(capabilities) = app.advertised_hardware() {
            if let Some(control) = crate::ui::hardware_control::managed_controls(&capabilities)
                .into_iter()
                .find(|control| control.key == control_key)
            {
                open_control_dialog(app, &capabilities, &control);
                return;
            }
        }
    }

    match &row.kind {
        TimelineTrackKind::Audio(_) => app.show_audio_settings = true,
        TimelineTrackKind::Subtitle(_) => app.show_sub_settings = true,
        TimelineTrackKind::ControllerEffect(_) => {
            app.open_or_focus_tab(PealayerTab::EffectControls)
        }
        TimelineTrackKind::Video(_) => app.open_or_focus_tab(PealayerTab::ProgramMonitor),
        TimelineTrackKind::Relay(_) | TimelineTrackKind::Hardware(_) => {}
    }
}

pub(crate) fn manage_timeline_track_by_key(
    app: &mut PealayerApp,
    key: &str,
) -> Result<(), String> {
    let row = all_timeline_track_rows(app)
        .into_iter()
        .find(|row| row.key == key)
        .ok_or_else(|| format!("Timeline track '{key}' is no longer available"))?;
    manage_timeline_track(app, &row);
    Ok(())
}

pub(crate) fn update_timeline_track(
    app: &mut PealayerApp,
    key: &str,
    patch: TimelineTrackPatch,
) -> Result<(), String> {
    let row = all_timeline_track_rows(app)
        .into_iter()
        .find(|row| row.key == key)
        .ok_or_else(|| format!("Timeline track '{key}' is no longer available"))?;

    let analog_channel = row
        .control_key
        .as_deref()
        .and_then(|control_key| control_key.strip_prefix("pwm."))
        .and_then(|channel| channel.parse::<u8>().ok());
    let supports_audio_mute = matches!(row.kind, TimelineTrackKind::Audio(_));
    let supports_channel_state = !row.relay_ids.is_empty() || analog_channel.is_some();

    if patch.muted.is_some() && !supports_channel_state && !supports_audio_mute {
        return Err(format!("'{}' does not support mute", row.name));
    }
    if (patch.soloed.is_some() || patch.locked.is_some()) && !supports_channel_state {
        return Err(format!("'{}' does not support solo or lock", row.name));
    }

    if let Some(linked) = patch.linked {
        app.set_timeline_track_linked(key, linked);
    }
    if let Some(visible) = patch.visible {
        app.set_timeline_track_visible(key, visible);
    }
    if patch.selected == Some(true) {
        app.selected_timeline_track = Some(key.to_string());
    } else if patch.selected == Some(false)
        && app.selected_timeline_track.as_deref() == Some(key)
    {
        app.selected_timeline_track = None;
    }

    let mut engine_changed = false;
    if let Some(muted) = patch.muted {
        if supports_audio_mute {
            app.set_audio_muted(muted);
        }
        for relay in &row.relay_ids {
            if muted {
                app.track_muted.insert(*relay);
            } else {
                app.track_muted.remove(relay);
            }
            engine_changed = true;
        }
        if let Some(channel) = analog_channel
            && let Some(track) = app
                .timeline
                .analog_tracks
                .iter_mut()
                .find(|track| track.channel == channel)
        {
            track.muted = muted;
            engine_changed = true;
        }
    }
    if let Some(soloed) = patch.soloed {
        for relay in &row.relay_ids {
            if soloed {
                app.track_soloed.insert(*relay);
            } else {
                app.track_soloed.remove(relay);
            }
            engine_changed = true;
        }
        if let Some(channel) = analog_channel
            && let Some(track) = app
                .timeline
                .analog_tracks
                .iter_mut()
                .find(|track| track.channel == channel)
        {
            track.soloed = soloed;
            engine_changed = true;
        }
    }
    if let Some(locked) = patch.locked {
        for relay in &row.relay_ids {
            if locked {
                app.track_locked.insert(*relay);
            } else {
                app.track_locked.remove(relay);
            }
        }
        if let Some(channel) = analog_channel
            && let Some(track) = app
                .timeline
                .analog_tracks
                .iter_mut()
                .find(|track| track.channel == channel)
        {
            track.locked = locked;
            if locked {
                track.armed = false;
            }
            engine_changed = true;
        }
    }
    if engine_changed {
        app.sync_timeline_engine();
    }
    Ok(())
}

fn media_track_key(row: &TimelineTrackRow) -> Option<crate::app::MediaTrackKey> {
    let (kind, id) = match row.kind {
        TimelineTrackKind::Video(Some(id)) => (crate::app::MediaTrackType::Video, id),
        TimelineTrackKind::Audio(id) => (crate::app::MediaTrackType::Audio, id),
        TimelineTrackKind::Subtitle(id) => (crate::app::MediaTrackType::Subtitle, id),
        TimelineTrackKind::Video(None)
        | TimelineTrackKind::ControllerEffect(_)
        | TimelineTrackKind::Relay(_)
        | TimelineTrackKind::Hardware(_) => return None,
    };
    Some(crate::app::MediaTrackKey { kind, id })
}

fn open_media_track_properties(app: &mut PealayerApp, row: &TimelineTrackRow) -> bool {
    let Some(selection) = media_track_key(row) else {
        return false;
    };
    crate::ui::media_track_properties::open(app, selection.kind, selection.id);
    true
}

fn default_hardware_track_state(
    app: &PealayerApp,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
    key: &str,
) -> crate::four_d::models::TimelineTrackState {
    app.timeline.track_states.get(key).copied().unwrap_or(
        crate::four_d::models::TimelineTrackState {
            linked: true,
            // Raw diagnostic channels remain available from the track picker,
            // while semantic controls are the default authoring surface.
            visible: !is_non_user_control(capabilities, control),
        },
    )
}

fn control_timeline_relay_ids(control: &crate::four_d::controller::HardwareControl) -> Vec<u8> {
    if let Some(relay) = relay_id_from_control_key(&control.key) {
        return vec![relay];
    }
    if is_motion_control(control) {
        let key = control.key.to_ascii_lowercase();
        if key.contains("left") || key.ends_with(".a") {
            return vec![1, 2];
        }
        if key.contains("right") || key.ends_with(".b") {
            return vec![3, 4];
        }
    }
    Vec::new()
}

fn timeline_track_matches_filter(row: &TimelineTrackRow, filter: &str) -> bool {
    let filter = filter.trim().to_lowercase();
    filter.is_empty()
        || row.name.to_lowercase().contains(&filter)
        || row.key.to_lowercase().contains(&filter)
        || row
            .detail
            .as_deref()
            .is_some_and(|detail| detail.to_lowercase().contains(&filter))
}

fn can_add_timeline_keyframe(
    rows: &[TimelineTrackRow],
    visible_analog_tracks: usize,
    playback_time: f64,
) -> bool {
    (!rows.is_empty() || visible_analog_tracks > 0)
        && playback_time.is_finite()
        && playback_time >= 0.0
}

fn timeline_keyframe_marker_center(ruler_rect: egui::Rect, marker_x: f32) -> egui::Pos2 {
    // Reserve the lower 12 px of the ruler for the red playhead handle. An
    // exact keyframe inserted at the playhead then remains independently
    // visible and clickable instead of sitting underneath the handle.
    egui::pos2(marker_x, ruler_rect.min.y + 6.0)
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum TimelineKeyframeTarget {
    Marker(uuid::Uuid),
    Analog(uuid::Uuid, usize),
}

impl TimelineKeyframeTarget {
    fn menu_id(self) -> egui::Id {
        egui::Id::new(("timeline-keyframe-menu", self))
    }
}

fn nearest_timeline_keyframe(
    pointer: Option<egui::Pos2>,
    candidates: &[(TimelineKeyframeTarget, egui::Pos2)],
) -> Option<TimelineKeyframeTarget> {
    let pointer = pointer?;
    candidates.iter()
        .map(|(target, center)| (*target, pointer.distance_sq(*center)))
        .filter(|(_, distance)| *distance <= 16.0 * 16.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(target, _)| target)
}

fn keyframe_context_menu(
    ui: &egui::Ui,
    response: &egui::Response,
    target: TimelineKeyframeTarget,
    targeted: bool,
    body: impl FnOnce(&mut egui::Ui),
) -> bool {
    // The canvas/ruler also cover this area. Use the nearest marker's explicit
    // hit-test, not whichever overlapping Response claimed the right click.
    let open = targeted && ui.rect_contains_pointer(response.rect)
        && ui.input(|input| input.pointer.button_released(egui::PointerButton::Secondary));
    egui::Popup::menu(response)
        .id(target.menu_id())
        .at_pointer_fixed()
        .open_memory(open.then_some(egui::SetOpenCommand::Bool(true)))
        .show(body)
        .is_some()
}

fn clear_unfocused_timeline_keyframes(
    app: &mut PealayerApp,
    ui: &egui::Ui,
    keyframe_hit: bool,
    popup_was_open: bool,
) {
    if app.selected_timeline_keyframe.is_none() && app.selected_keyframes.is_empty() {
        return;
    }
    if popup_was_open || egui::Popup::is_any_open(ui.ctx()) || app.active_keyframe_drag.is_some() {
        return;
    }
    let (click_away, escape) = ui.input(|input| (
        input.pointer.button_pressed(egui::PointerButton::Primary)
            && !keyframe_hit && !input.modifiers.ctrl && !input.modifiers.command,
        input.key_pressed(egui::Key::Escape),
    ));
    let focus = ui.ctx().memory(|memory| memory.focused());
    let escape = escape && (focus.is_none() || focus == Some(timeline_keyboard_focus_id()));
    if click_away || escape {
        app.selected_timeline_keyframe = None;
        app.selected_keyframes.clear();
        ui.ctx().request_repaint();
        if escape {
            app.selected_instance_ids.clear();
            ui.ctx().memory_mut(|memory| memory.surrender_focus(timeline_keyboard_focus_id()));
        }
    }
}

fn media_timeline_track_label(
    app: &PealayerApp,
    kind: &'static str,
    ordinal: usize,
    title: Option<&str>,
    language: Option<&str>,
) -> (String, Option<String>) {
    let title = title
        .map(str::trim)
        .filter(|value| !value.is_empty())
        // Some containers expose a generic title copied from the wrong
        // stream type (for example an audio stream titled "Video"). Such a
        // value is not a useful track identity and must not make two
        // different media rows appear to be the same channel.
        .filter(|value| {
            !["video", "audio", "subtitle", "subtitles"]
                .iter()
                .any(|generic| value.eq_ignore_ascii_case(generic))
                || value.eq_ignore_ascii_case(kind)
        });
    let language = language.map(str::trim).filter(|value| !value.is_empty());
    let name = title
        .map(|value| app.display_text(value))
        .unwrap_or_else(|| {
            if ordinal == 0 {
                app.tr(kind)
            } else {
                format!("{} {}", app.tr(kind), ordinal + 1)
            }
        });
    let detail = if title.is_some() {
        match language {
            Some(value) => format!("{} · {}", app.tr(kind), app.display_text(value)),
            None => app.tr(kind),
        }
    } else {
        match language {
            Some(value) => format!(
                "{} {} · {}",
                app.tr("Track"),
                ordinal + 1,
                app.display_text(value)
            ),
            None => format!("{} {}", app.tr("Track"), ordinal + 1),
        }
    };
    (name, Some(detail))
}

fn timeline_track_rows(app: &PealayerApp) -> Vec<TimelineTrackRow> {
    all_timeline_track_rows(app)
        .into_iter()
        .filter(|row| row.linked && row.visible && !hardware_row_has_analog_track(app, row))
        .collect()
}

fn hardware_row_has_analog_track(app: &PealayerApp, row: &TimelineTrackRow) -> bool {
    let TimelineTrackKind::Hardware(control_key) = &row.kind else {
        return false;
    };
    let Some(channel) = control_key
        .strip_prefix("pwm.")
        .and_then(|value| value.parse::<u8>().ok())
    else {
        return false;
    };
    app.timeline
        .analog_tracks
        .iter()
        .any(|track| track.channel == channel)
}

fn all_timeline_track_rows(app: &PealayerApp) -> Vec<TimelineTrackRow> {
    let mut rows = Vec::new();
    if app.current_video_path.is_some() {
        if app.video_tracks.is_empty() {
            let key = "media:video".to_string();
            let state = app.timeline.track_state(&key);
            let (name, detail) = media_timeline_track_label(app, "Video", 0, None, None);
            rows.push(TimelineTrackRow {
                key,
                name,
                detail,
                active: true,
                enabled: true,
                linked: state.linked,
                visible: state.visible,
                icon: crate::ui::icons::FILE_VIDEO.to_string(),
                control_key: None,
                relay_ids: Vec::new(),
                dimmed: false,
                kind: TimelineTrackKind::Video(None),
            });
        } else {
            for (ordinal, track) in app.video_tracks.iter().enumerate() {
                let key = if ordinal == 0 {
                    "media:video".to_string()
                } else {
                    format!("media:video:{}", track.id)
                };
                let state = app.timeline.track_state(&key);
                let active = app.current_vid == track.id.to_string();
                let (name, detail) = media_timeline_track_label(
                    app,
                    "Video",
                    ordinal,
                    track.title.as_deref(),
                    track.lang.as_deref(),
                );
                rows.push(TimelineTrackRow {
                    key,
                    name,
                    detail,
                    active,
                    enabled: active,
                    linked: state.linked,
                    visible: state.visible,
                    icon: crate::ui::icons::FILE_VIDEO.to_string(),
                    control_key: None,
                    relay_ids: Vec::new(),
                    dimmed: false,
                    kind: TimelineTrackKind::Video(Some(track.id)),
                });
            }
        }
        for (ordinal, track) in app.audio_tracks.iter().enumerate() {
            let key = format!("media:audio:{}", track.id);
            let state = app.timeline.track_state(&key);
            let active = app.current_aid == track.id.to_string();
            let (name, detail) = media_timeline_track_label(
                app,
                "Audio",
                ordinal,
                track.title.as_deref(),
                track.lang.as_deref(),
            );
            rows.push(TimelineTrackRow {
                key,
                name,
                detail,
                active,
                enabled: active,
                linked: state.linked,
                visible: state.visible,
                icon: crate::ui::icons::SPEAKER_HIGH.to_string(),
                control_key: None,
                relay_ids: Vec::new(),
                dimmed: false,
                kind: TimelineTrackKind::Audio(track.id),
            });
        }
        for (ordinal, track) in app.sub_tracks.iter().enumerate() {
            let key = format!("media:subtitle:{}", track.id);
            let state = app.timeline.track_state(&key);
            let active = app.current_sid == track.id.to_string();
            let (name, detail) = media_timeline_track_label(
                app,
                "Subtitles",
                ordinal,
                track.title.as_deref(),
                track.lang.as_deref(),
            );
            rows.push(TimelineTrackRow {
                key,
                name,
                detail,
                active,
                enabled: active && app.sub_visibility,
                linked: state.linked,
                visible: state.visible,
                icon: crate::ui::icons::SUBTITLES.to_string(),
                control_key: None,
                relay_ids: Vec::new(),
                dimmed: false,
                kind: TimelineTrackKind::Subtitle(track.id),
            });
        }
    }
    let capabilities = app
        .advertised_hardware()
        .filter(|capabilities| capabilities.board_connected);
    let mut effect_lanes = app
        .timeline
        .templates
        .iter()
        .filter_map(|effect| effect.controller_lane)
        .collect::<std::collections::BTreeSet<_>>();
    if let Some(capabilities) = capabilities.as_ref() {
        effect_lanes.extend(
            capabilities
                .macros
                .iter()
                .map(crate::app::controller_macro_lane),
        );
        if !capabilities.strip_effects.is_empty() {
            effect_lanes.insert(crate::four_d::models::ControllerEffectLane::Lighting);
        }
    }
    rows.extend(effect_lanes.into_iter().map(|lane| {
        let key = crate::four_d::models::controller_effect_timeline_track_key(lane);
        let state = app.timeline.track_state(&key);
        TimelineTrackRow {
            key,
            name: controller_effect_lane_label(app, lane),
            detail: Some(app.tr("Effects")),
            active: true,
            enabled: true,
            linked: state.linked,
            visible: state.visible,
            icon: controller_effect_lane_icon(lane).to_string(),
            control_key: None,
            relay_ids: Vec::new(),
            dimmed: false,
            kind: TimelineTrackKind::ControllerEffect(lane),
        }
    }));
    if let Some(capabilities) = capabilities.as_ref() {
        rows.extend(
            crate::ui::hardware_control::managed_controls(capabilities)
                .into_iter()
                .map(|control| {
                    let key = crate::four_d::models::hardware_timeline_track_key(&control.key);
                    let state = default_hardware_track_state(app, capabilities, &control, &key);
                    let relay_id = relay_id_from_control_key(&control.key);
                    let dimmed = app.non_user_control_visibility
                        == crate::config::NonUserControlVisibility::Dimmed
                        && is_non_user_control(capabilities, &control);
                    TimelineTrackRow {
                        key,
                        name: app.display_text(&control.name),
                        detail: (!control.group.trim().is_empty())
                            .then(|| app.display_text(&control.group)),
                        active: crate::ui::hardware_control::channel_is_active(
                            capabilities,
                            &control,
                        ),
                        enabled: !control.locked,
                        linked: state.linked,
                        visible: state.visible,
                        icon: crate::ui::icons::control(&control.kind, &control.icon).to_string(),
                        control_key: Some(control.key.clone()),
                        relay_ids: control_timeline_relay_ids(&control),
                        dimmed,
                        kind: relay_id.map_or_else(
                            || TimelineTrackKind::Hardware(control.key.clone()),
                            TimelineTrackKind::Relay,
                        ),
                    }
                }),
        );
    }
    rows
}

/// Project the native timeline's authoritative row model into the shared Web
/// status contract. Keeping this beside `all_timeline_track_rows` prevents the
/// two surfaces from independently inventing track visibility or ordering.
pub(crate) fn web_timeline_tracks(
    app: &PealayerApp,
) -> Vec<crate::platform::interop::WebTimelineTrack> {
    all_timeline_track_rows(app)
        .into_iter()
        .map(|row| {
            let analog_track = row
                .control_key
                .as_deref()
                .and_then(|control_key| control_key.strip_prefix("pwm."))
                .and_then(|channel| channel.parse::<u8>().ok())
                .and_then(|channel| {
                    app.timeline
                        .analog_tracks
                        .iter()
                        .find(|track| track.channel == channel)
                });
            let is_audio = matches!(&row.kind, TimelineTrackKind::Audio(_));
            let supports_channel_state = !row.relay_ids.is_empty() || analog_track.is_some();
            let muted = if is_audio {
                app.is_muted
            } else if !row.relay_ids.is_empty() {
                row.relay_ids
                    .iter()
                    .all(|relay| app.track_muted.contains(relay))
            } else {
                analog_track.is_some_and(|track| track.muted)
            };
            let soloed = if !row.relay_ids.is_empty() {
                row.relay_ids
                    .iter()
                    .all(|relay| app.track_soloed.contains(relay))
            } else {
                analog_track.is_some_and(|track| track.soloed)
            };
            let locked = if !row.relay_ids.is_empty() {
                row.relay_ids
                    .iter()
                    .all(|relay| app.track_locked.contains(relay))
            } else {
                analog_track.is_some_and(|track| track.locked)
            };
            let manageable = row.control_key.is_some()
                || matches!(
                    &row.kind,
                    TimelineTrackKind::Video(_)
                        | TimelineTrackKind::Audio(_)
                        | TimelineTrackKind::Subtitle(_)
                        | TimelineTrackKind::ControllerEffect(_)
                );
            let selected = app.selected_timeline_track.as_deref() == Some(row.key.as_str());
            let (kind, lane) = match &row.kind {
                TimelineTrackKind::Video(_) => ("video".to_string(), None),
                TimelineTrackKind::Audio(_) => ("audio".to_string(), None),
                TimelineTrackKind::Subtitle(_) => ("subtitle".to_string(), None),
                TimelineTrackKind::ControllerEffect(lane) => (
                    "effect".to_string(),
                    Some(match *lane {
                        crate::four_d::models::ControllerEffectLane::Motion => "motion",
                        crate::four_d::models::ControllerEffectLane::Relay => "relay",
                        crate::four_d::models::ControllerEffectLane::Pwm => "pwm",
                        crate::four_d::models::ControllerEffectLane::Lighting => "lighting",
                        crate::four_d::models::ControllerEffectLane::Display => "display",
                        crate::four_d::models::ControllerEffectLane::Rf => "rf",
                        crate::four_d::models::ControllerEffectLane::Audio => "audio",
                        crate::four_d::models::ControllerEffectLane::Sequence => "sequence",
                        crate::four_d::models::ControllerEffectLane::Composite => "composite",
                    }.to_string()),
                ),
                TimelineTrackKind::Relay(_) | TimelineTrackKind::Hardware(_) => {
                    ("hardware".to_string(), None)
                }
            };
            crate::platform::interop::WebTimelineTrack {
                key: row.key,
                name: row.name,
                detail: row.detail,
                kind,
                lane,
                control_key: row.control_key,
                active: row.active,
                enabled: row.enabled,
                linked: row.linked,
                visible: row.visible,
                dimmed: row.dimmed,
                selected,
                muted,
                soloed,
                locked,
                supports_mute: supports_channel_state || is_audio,
                supports_solo: supports_channel_state,
                supports_lock: supports_channel_state,
                manageable,
            }
        })
        .collect()
}

fn controller_effect_lane_label(
    app: &PealayerApp,
    lane: crate::four_d::models::ControllerEffectLane,
) -> String {
    use crate::four_d::models::ControllerEffectLane;
    app.tr(match lane {
        ControllerEffectLane::Motion => "Motion effects",
        ControllerEffectLane::Relay => "Relay effects",
        ControllerEffectLane::Pwm => "PWM effects",
        ControllerEffectLane::Lighting => "Lighting effects",
        ControllerEffectLane::Display => "Display effects",
        ControllerEffectLane::Rf => "RF effects",
        ControllerEffectLane::Audio => "Audio effects",
        ControllerEffectLane::Sequence => "General sequences",
        ControllerEffectLane::Composite => "Composite effects",
    })
}

fn controller_effect_lane_icon(lane: crate::four_d::models::ControllerEffectLane) -> &'static str {
    use crate::four_d::models::ControllerEffectLane;
    match lane {
        ControllerEffectLane::Motion => crate::ui::icons::SEAT,
        ControllerEffectLane::Relay => crate::ui::icons::PLUG,
        ControllerEffectLane::Pwm => crate::ui::icons::SLIDERS_HORIZONTAL,
        ControllerEffectLane::Lighting => crate::ui::icons::SPARKLE,
        ControllerEffectLane::Display => crate::ui::icons::APP_WINDOW,
        ControllerEffectLane::Rf => crate::ui::icons::RADIO,
        ControllerEffectLane::Audio => crate::ui::icons::SPEAKER_HIGH,
        ControllerEffectLane::Sequence => crate::ui::icons::WAVEFORM,
        ControllerEffectLane::Composite => crate::ui::icons::SELECTION_ALL,
    }
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

fn timeline_cue_placement(
    app: &PealayerApp,
    effect: &crate::four_d::models::Effect,
    rows: &[TimelineTrackRow],
    visible_analog_track_ids: &std::collections::BTreeSet<uuid::Uuid>,
) -> Option<TimelineCuePlacement> {
    if effect.controller_macro.is_some() || effect.controller_strip_effect.is_some() {
        let lane = effect
            .controller_lane
            .unwrap_or(crate::four_d::models::ControllerEffectLane::Sequence);
        return rows.iter()
            .position(|row| row.kind == TimelineTrackKind::ControllerEffect(lane))
            .map(|track_index| TimelineCuePlacement {
                track_index,
                relay_id: None,
                analog_index: None,
                selected_track_key: None,
            });
    }

    if let Some(direct) = effect.direct_control.as_ref()
        && let Some(channel) = direct.control_key.strip_prefix("pwm.")
            .and_then(|value| value.parse::<u8>().ok())
    {
        if let Some(analog_index) = app.timeline.analog_tracks.iter()
            .filter(|track| visible_analog_track_ids.contains(&track.id))
            .position(|track| track.channel == channel)
        {
            return Some(TimelineCuePlacement {
                track_index: usize::MAX,
                relay_id: None,
                analog_index: Some(analog_index),
                selected_track_key: Some(
                    crate::four_d::models::hardware_timeline_track_key(&direct.control_key),
                ),
            });
        }

        return rows.iter()
            .position(|row| row.kind == TimelineTrackKind::Hardware(direct.control_key.clone()))
            .map(|track_index| TimelineCuePlacement {
                track_index,
                relay_id: None,
                analog_index: None,
                selected_track_key: None,
            });
    }

    let relay_id = effect.actions.first().map(|action| action.relay_id)?;
    timeline_row_for_relay(rows, relay_id).map(|track_index| TimelineCuePlacement {
        track_index,
        relay_id: Some(relay_id),
        analog_index: None,
        selected_track_key: None,
    })
}

fn relay_id_from_control_key(key: &str) -> Option<u8> {
    key.strip_prefix("relay.")?.parse().ok()
}

fn relay_identifier_label(relay_id: u8) -> String {
    format!("R{relay_id}")
}

fn is_pwm_control(control: &crate::four_d::controller::HardwareControl) -> bool {
    matches!(control.kind.as_str(), "mosfet" | "pwm") || control.key.starts_with("pwm.")
}

fn timeline_cue_dialog_id() -> egui::Id {
    egui::Id::new("timeline-add-cue-dialog")
}

fn direct_cue_behavior_editor(ui: &mut egui::Ui, behavior: &mut crate::four_d::models::DirectCueBehavior,
    end: &mut u16, pwm: bool) {
    use crate::four_d::models::DirectCueBehavior;
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(behavior, DirectCueBehavior::SetKeep, "Set and keep");
        ui.selectable_value(behavior, DirectCueBehavior::Hold, "Timed hold");
        if pwm { ui.selectable_value(behavior, DirectCueBehavior::Ramp, "PWM ramp"); }
    });
    if *behavior != DirectCueBehavior::SetKeep {
        ui.horizontal(|ui| {
            ui.label(if *behavior == DirectCueBehavior::Ramp { "Ramp to" } else { "On exit" });
            if pwm {
                ui.add(egui::Slider::new(end, 0..=10_000)
                    .custom_formatter(|v, _| format!("{:.1}%", v / 100.0))
                    .custom_parser(|text| text.trim().trim_end_matches('%').trim().parse::<f64>().ok()
                        .filter(|value| value.is_finite())
                        .map(|value| (value.clamp(0.0, 100.0) * 100.0).round())));
            } else {
                ui.selectable_value(end, 0, "Off");
                ui.selectable_value(end, 10_000, "On");
            }
        });
    }
}

fn direct_cue_marker_caption(effect: &crate::four_d::models::Effect) -> String {
    let Some(cue) = &effect.direct_control else { return effect.name.clone(); };
    let value = if cue.control_key.starts_with("relay.") {
        if cue.value_basis_points >= 5_000 { "On".to_string() } else { "Off".to_string() }
    } else { format!("{:.1}%", f32::from(cue.value_basis_points) / 100.0) };
    format!("{value}  → ∞")
}

fn paint_hardware_cue(painter: &egui::Painter, rect: egui::Rect, effect: &crate::four_d::models::Effect,
    alpha: u8, stroke: egui::Stroke) {
    let color = egui::Color32::from_rgba_unmultiplied(65, 111, 161, alpha);
    painter.rect_filled(rect, if effect.is_state_marker() { 9.0 } else { 4.0 }, color);
    if let Some(direct) = &effect.direct_control {
        if direct.behavior == crate::four_d::models::DirectCueBehavior::Ramp {
            for index in 0..32 {
                let value = direct.value_at(index * 100, 3100) as f32 / 10_000.0;
                let strip = egui::Rect::from_min_max(
                    egui::pos2(rect.left() + rect.width() * index as f32 / 32.0, rect.top() + 2.0),
                    egui::pos2(rect.left() + rect.width() * (index + 1) as f32 / 32.0, rect.bottom() - 2.0));
                painter.rect_filled(strip, 0.0, egui::Color32::from_rgba_unmultiplied(
                    35, (60.0 + value * 100.0) as u8, (90.0 + value * 110.0) as u8, alpha));
            }
            painter.line_segment([egui::pos2(rect.left() + 5.0, rect.bottom() - 4.0 - rect.height() * 0.6 * direct.value_basis_points as f32 / 10_000.0),
                egui::pos2(rect.right() - 5.0, rect.bottom() - 4.0 - rect.height() * 0.6 * direct.end_value_basis_points as f32 / 10_000.0)], egui::Stroke::new(1.0, egui::Color32::WHITE));
        }
    }
    painter.rect_stroke(rect, if effect.is_state_marker() { 9.0 } else { 4.0 }, stroke, egui::StrokeKind::Inside);
}

fn timeline_cue_draft_for_row(
    app: &PealayerApp,
    row: &TimelineTrackRow,
    start_time_ms: u64,
) -> Result<TimelineCueDraft, String> {
    if !row.linked || !row.visible {
        return Err(format!("Track '{}' is not linked and visible", row.name));
    }
    if !row.enabled {
        return Err(format!("Track '{}' is locked by PCController", row.name));
    }
    let control_key = row
        .control_key
        .as_deref()
        .ok_or_else(|| format!("Track '{}' does not accept direct cues", row.name))?;
    let control = app
        .advertised_hardware()
        .and_then(|capabilities| {
            crate::ui::hardware_control::managed_controls(&capabilities)
                .into_iter()
                .find(|control| control.key == control_key)
        })
        .ok_or_else(|| format!("Channel '{control_key}' is not advertised by PCController"))?;

    if row
        .relay_ids
        .iter()
        .any(|relay| app.track_locked.contains(relay))
    {
        return Err(format!("Track '{}' is locked", row.name));
    }
    let analog_locked = control_key
        .strip_prefix("pwm.")
        .and_then(|channel| channel.parse::<u8>().ok())
        .and_then(|channel| {
            app.timeline
                .analog_tracks
                .iter()
                .find(|track| track.channel == channel)
        })
        .is_some_and(|track| track.locked);
    if analog_locked {
        return Err(format!("Track '{}' is locked", row.name));
    }

    let action = if relay_id_from_control_key(control_key).is_some() {
        TimelineCueDraftAction::Relay { enabled: true }
    } else if is_pwm_control(&control) {
        TimelineCueDraftAction::Pwm {
            value_basis_points: 5_000,
        }
    } else {
        return Err(format!(
            "Track '{}' does not support directly-authored cues",
            row.name
        ));
    };

    Ok(TimelineCueDraft {
        track_name: row.name.clone(),
        control_key: control_key.to_string(),
        start_time_ms,
        duration_ms: 1_000,
        behavior: crate::four_d::models::DirectCueBehavior::SetKeep,
        end_value_basis_points: 0,
        action,
        error: None,
    })
}

fn request_timeline_cue_dialog(
    app: &mut PealayerApp,
    context: &egui::Context,
    track_key: &str,
    start_time_ms: u64,
) {
    let row = all_timeline_track_rows(app)
        .into_iter()
        .find(|row| row.key == track_key);
    let Some(row) = row else {
        let message = app.tr("Select a relay or PWM timeline track first");
        app.set_osd(message);
        return;
    };
    app.selected_timeline_track = Some(row.key.clone());
    match timeline_cue_draft_for_row(app, &row, start_time_ms) {
        Ok(draft) => {
            context
                .data_mut(|data| data.insert_temp(timeline_cue_dialog_id(), draft));
            context.request_repaint();
        }
        Err(error) => app.set_osd(error),
    }
}

fn draw_timeline_cue_dialog(app: &mut PealayerApp, context: &egui::Context) {
    let Some(mut draft) =
        context.data_mut(|data| data.get_temp::<TimelineCueDraft>(timeline_cue_dialog_id()))
    else {
        return;
    };
    let mut open = true;
    let mut cancel = false;
    let mut submit = false;
    crate::ui::sync_elegance_theme(context);
    elegance::Modal::new("timeline-add-cue-dialog", &mut open)
        .heading(app.tr("Add cue"))
        .subtitle(draft.track_name.clone())
        .header_icon(crate::ui::icons::PLUS)
        .max_width(440.0)
        .footer(|ui| {
            if ui.add(elegance::Button::new(app.tr("Add cue")).accent(elegance::Accent::Green)).clicked() {
                submit = true;
            }
            if ui.add(elegance::Button::new(app.tr("Cancel")).outline()).clicked() {
                cancel = true;
            }
        })
        .show(context, |ui| {
            ui.add(elegance::Badge::new(draft.control_key.as_str(), elegance::BadgeTone::Neutral).preserve_case());
            ui.add_space(10.0);
            direct_cue_behavior_editor(ui, &mut draft.behavior, &mut draft.end_value_basis_points,
                matches!(draft.action, TimelineCueDraftAction::Pwm { .. }));
            elegance::Card::new().heading(app.tr("Timing")).show(ui, |ui| {
                egui::Grid::new("timeline-add-cue-timing")
                    .num_columns(2)
                    .spacing([14.0, 10.0])
                    .show(ui, |ui| {
                        ui.label(app.tr("Start time"));
                        ui.add(crate::duration::time_value_drag(
                            &mut draft.start_time_ms, 0..=86_400_000, 10.0,
                            app.human_readable_time_units,
                        ));
                        ui.end_row();
                        if draft.behavior != crate::four_d::models::DirectCueBehavior::SetKeep {
                        ui.label(app.tr("Duration"));
                        ui.add(crate::duration::time_value_drag(
                            &mut draft.duration_ms, 100..=86_400_000, 10.0,
                            app.human_readable_time_units,
                        ));
                        ui.end_row();
                        }
                    });
            });
            ui.add_space(8.0);
            elegance::Card::new().heading(app.tr("Action")).show(ui, |ui| match &mut draft.action {
                TimelineCueDraftAction::Relay { enabled } => {
                    let mut selected = usize::from(*enabled);
                    if ui.add(elegance::SegmentedControl::new(&mut selected, [app.tr("Off"), app.tr("On")])).changed() {
                        *enabled = selected == 1;
                    }
                }
                TimelineCueDraftAction::Pwm { value_basis_points } => {
                    ui.add(egui::Slider::new(value_basis_points, 0..=10_000)
                        .custom_formatter(|value, _| format!("{:.2}%", value / 100.0))
                        .custom_parser(|text| text.trim().trim_end_matches('%').trim().parse::<f64>().ok()
                            .map(|percent| (percent.clamp(0.0, 100.0) * 100.0).round())));
                }
            });
            if let Some(error) = draft.error.as_deref() {
                ui.add_space(8.0);
                elegance::Callout::new(elegance::CalloutTone::Danger).body(error).show(ui, |_| {});
            }
        });

    if submit {
        match app.add_configured_direct_control_cue(
            &draft.control_key,
            draft.value_basis_points(),
            draft.start_time_ms,
            draft.duration_ms,
            draft.behavior,
            draft.end_value_basis_points,
        ) {
            Ok(_) => {
                let cue_added = app.tr("Cue added");
                app.set_osd(format!("{cue_added}: {}", draft.track_name));
                open = false;
            }
            Err(error) => draft.error = Some(error),
        }
    }
    if !open || cancel {
        context.data_mut(|data| {
            data.remove::<TimelineCueDraft>(timeline_cue_dialog_id());
        });
    } else {
        context.data_mut(|data| data.insert_temp(timeline_cue_dialog_id(), draft));
    }
}

fn pwm_channel_for<'a>(
    capabilities: &'a crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
) -> Option<&'a crate::four_d::controller::HardwareOutput> {
    capabilities
        .pwm_channels
        .iter()
        .find(|channel| channel.key == control.key)
}

fn status_rgb_component(
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
) -> Option<(u8, egui::Color32)> {
    let role = pwm_channel_for(capabilities, control)?
        .role
        .to_ascii_lowercase();
    let status = capabilities.status_led.as_ref()?;
    if role.contains("status") && role.contains("red") {
        Some((status.red, egui::Color32::from_rgb(239, 68, 68)))
    } else if role.contains("status") && role.contains("green") {
        Some((status.green, egui::Color32::from_rgb(34, 197, 94)))
    } else if role.contains("status") && role.contains("blue") {
        Some((status.blue, egui::Color32::from_rgb(59, 130, 246)))
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ControlIndicatorState {
    Active,
    Inactive,
    Unknown,
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
        if let Some((value, _)) = status_rgb_component(capabilities, control) {
            return if value > 0 {
                ControlIndicatorState::Active
            } else {
                ControlIndicatorState::Inactive
            };
        }
        let Some(channel) = pwm_channel_for(capabilities, control).map(|channel| channel.id) else {
            return ControlIndicatorState::Unknown;
        };
        return capabilities
            .telemetry
            .pwm_values
            .get(usize::from(channel))
            .copied()
            .flatten()
            .or_else(|| {
                (capabilities.telemetry.pwm_channel == Some(channel))
                    .then_some(capabilities.telemetry.pwm_value.unwrap_or(0))
            })
            .map_or(ControlIndicatorState::Unknown, |value| {
                if value > 0 {
                    ControlIndicatorState::Active
                } else {
                    ControlIndicatorState::Inactive
                }
            });
    }
    if is_motion_control(control) {
        return if motion_control_is_active(capabilities, control) {
            ControlIndicatorState::Active
        } else {
            ControlIndicatorState::Inactive
        };
    }
    ControlIndicatorState::Unknown
}

pub(crate) fn is_non_user_control(
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
) -> bool {
    if relay_id_from_control_key(&control.key).is_some_and(|relay| relay <= 4) {
        return true;
    }
    if is_pwm_control(control) {
        return capabilities
            .pwm_channels
            .iter()
            .find(|channel| channel.key == control.key)
            .is_some_and(|channel| channel.control != "pwm-user" || channel.role != "user-output");
    }
    false
}

fn global_control_is_visible(
    app: &PealayerApp,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
) -> bool {
    !is_non_user_control(capabilities, control)
        || app.non_user_control_visibility != crate::config::NonUserControlVisibility::Hidden
}

fn draw_non_user_visibility_choices(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let mut changed = false;
    for (value, icon, label, help) in [
        (
            crate::config::NonUserControlVisibility::Hidden,
            crate::ui::icons::EYE_SLASH,
            "Hide non-user controls",
            "Keep diagnostic relays and system PWM channels out of the monitor",
        ),
        (
            crate::config::NonUserControlVisibility::Dimmed,
            crate::ui::icons::EYE,
            "Dim non-user controls",
            "Show diagnostic and system channels with reduced emphasis",
        ),
        (
            crate::config::NonUserControlVisibility::Shown,
            crate::ui::icons::GAUGE,
            "Show non-user controls",
            "Show diagnostic and system channels like user outputs",
        ),
    ] {
        let option_label = format!("{icon} {}", app.tr(label));
        let option_help = app.tr(help);
        changed |= ui
            .radio_value(&mut app.non_user_control_visibility, value, option_label)
            .on_hover_text(option_help)
            .changed();
    }
    if changed {
        app.save_config();
    }
}

fn pwm_control_intensity(
    ui: &egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
) -> Option<f32> {
    if let Some((value, _)) = status_rgb_component(capabilities, control) {
        return Some(f32::from(value) / 255.0);
    }
    let channel = pwm_channel_for(capabilities, control)?;
    let value_id = ui.make_persistent_id(("pwm_value", channel.id));
    let local = ui.data_mut(|data| data.get_temp::<u16>(value_id));
    let authoritative = capabilities
        .telemetry
        .pwm_values
        .get(usize::from(channel.id))
        .copied()
        .flatten()
        .or_else(|| {
            (capabilities.telemetry.pwm_channel == Some(channel.id))
                .then_some(capabilities.telemetry.pwm_value.unwrap_or(0))
        });
    let sent_id = ui.make_persistent_id(("pwm_sent_value", channel.id));
    let last_send_id = ui.make_persistent_id(("pwm_last_send", channel.id));
    let last_sent = ui.data_mut(|data| data.get_temp::<u16>(sent_id));
    let last_send = ui.data_mut(|data| data.get_temp::<std::time::Instant>(last_send_id));
    let awaiting_readback = last_sent
        .zip(authoritative)
        .is_some_and(|(sent, observed)| sent != observed)
        && last_send.is_some_and(|sent_at| sent_at.elapsed() < std::time::Duration::from_secs(1));
    let raw = if awaiting_readback {
        local.or(authoritative)
    } else {
        authoritative.or(local)
    }?;
    let percent = if awaiting_readback {
        pwm_percent(raw)
    } else {
        capabilities.pwm_percent(channel.id, raw)
    };
    Some((percent / 100.0) as f32)
}

fn control_indicator_color(control: &crate::four_d::controller::HardwareControl) -> egui::Color32 {
    crate::config::parse_rgb_hex(&control.color)
        .map(|[red, green, blue]| egui::Color32::from_rgb(red, green, blue))
        .unwrap_or_else(|| egui::Color32::from_rgb(34, 197, 94))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MotionDirectionState {
    Stopped,
    Up,
    Down,
    Unknown,
}

pub(crate) fn motion_control_direction(
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
) -> MotionDirectionState {
    if !is_motion_control(control) {
        return MotionDirectionState::Unknown;
    }
    let key = control.key.to_ascii_lowercase();
    let side = if key.contains("left") || key.ends_with(".a") {
        Some((1_u8, 2_u8, ["left", "motion-a", "seat-a"], true))
    } else if key.contains("right") || key.ends_with(".b") {
        Some((3_u8, 4_u8, ["right", "motion-b", "seat-b"], false))
    } else {
        None
    };
    let Some((_, _, _, is_left)) = side else {
        return MotionDirectionState::Unknown;
    };

    // PCController owns semantic intent and reconciles it with every physical
    // relay edge. During the board's mandatory break-before-make interval,
    // display the requested direction. Raw relay feedback remains available
    // for diagnostics and recording, but it is not a second API contract for
    // reconstructing semantic state.
    if let Some(motion) = &capabilities.motion {
        let state = if is_left { &motion.left } else { &motion.right };
        let presented = if state.transitioning {
            state.requested.as_str()
        } else {
            state.applied.as_str()
        };
        match presented {
            "up" => return MotionDirectionState::Up,
            "down" => return MotionDirectionState::Down,
            "stop" => return MotionDirectionState::Stopped,
            _ => {}
        }
    }

    MotionDirectionState::Unknown
}

pub(crate) fn motion_direction_color(
    control: &crate::four_d::controller::HardwareControl,
    direction: MotionDirectionState,
) -> egui::Color32 {
    let configured = match direction {
        MotionDirectionState::Up => &control.up_color,
        MotionDirectionState::Down => &control.down_color,
        MotionDirectionState::Stopped | MotionDirectionState::Unknown => "",
    };
    let fallback = match direction {
        MotionDirectionState::Up => [245, 158, 11],
        MotionDirectionState::Down => [59, 130, 246],
        MotionDirectionState::Stopped | MotionDirectionState::Unknown => [34, 197, 94],
    };
    let [red, green, blue] = crate::config::parse_rgb_hex(configured).unwrap_or(fallback);
    egui::Color32::from_rgb(red, green, blue)
}

fn active_control_indicator_color(
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
) -> egui::Color32 {
    if is_motion_control(control) {
        motion_direction_color(control, motion_control_direction(capabilities, control))
    } else if let Some((_, color)) = status_rgb_component(capabilities, control) {
        color
    } else {
        control_indicator_color(control)
    }
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
    color: egui::Color32,
    intensity: Option<f32>,
) -> egui::Response {
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
            let intensity = intensity.unwrap_or(1.0).clamp(0.0, 1.0);
            ui.painter()
                .circle_filled(center, 7.0, color.gamma_multiply(0.10 + intensity * 0.18));
            ui.painter()
                .circle_filled(center, 4.5, color.gamma_multiply(0.28 + intensity * 0.72));
        }
        ControlIndicatorState::Unknown => {
            ui.painter()
                .circle_stroke(center, 5.0, egui::Stroke::new(1.4_f32, neutral));
            ui.painter().line_segment(
                [center - egui::vec2(2.2, 0.0), center + egui::vec2(2.2, 0.0)],
                egui::Stroke::new(1.3_f32, neutral),
            );
        }
        ControlIndicatorState::Inactive => {
            ui.painter()
                .circle_stroke(center, 5.0, egui::Stroke::new(1.4_f32, neutral));
        }
    }
    let status = match (state, intensity) {
        (ControlIndicatorState::Active, Some(intensity)) => {
            format!("{} {:.1}%", app.tr("Output level"), intensity * 100.0)
        }
        (ControlIndicatorState::Active, None) => app.tr("Board reports ON"),
        (ControlIndicatorState::Unknown, _) => app.tr("State not sampled by the board"),
        (ControlIndicatorState::Inactive, Some(_)) => {
            format!("{} 0%", app.tr("Output level"))
        }
        (ControlIndicatorState::Inactive, None) => app.tr("Board reports OFF"),
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

fn board_card_labels(
    language: crate::config::AppLanguage,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    fallback: String,
) -> (String, Option<String>) {
    let profile = capabilities
        .board_profile
        .as_ref()
        .map(|profile| humanize_machine_label(&profile.key))
        .filter(|name| !name.trim().is_empty());
    let advertised = crate::ui::i18n::visual_text(language, &capabilities.board_name);
    let title = if advertised.trim().is_empty() {
        profile.clone().unwrap_or(fallback)
    } else {
        advertised
    };
    let subtitle = profile.filter(|profile| !profile.trim().eq_ignore_ascii_case(title.trim()));
    (title, subtitle)
}

fn board_identity_text_positions(
    rect: egui::Rect,
    title_height: f32,
    subtitle_height: Option<f32>,
) -> (egui::Pos2, Option<egui::Pos2>) {
    let gap = subtitle_height
        .map(|_| BOARD_IDENTITY_LINE_GAP)
        .unwrap_or_default();
    let stack_height = title_height + subtitle_height.unwrap_or_default() + gap;
    let top = rect.center().y - stack_height * 0.5;
    let title = egui::pos2(rect.left(), top);
    let subtitle = subtitle_height.map(|_| egui::pos2(rect.left(), top + title_height + gap));
    (title, subtitle)
}

fn draw_board_identity_block(
    ui: &mut egui::Ui,
    width: f32,
    height: f32,
    title: &str,
    subtitle: Option<&str>,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width.max(1.0), height), egui::Sense::click());
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    let title_color = ui.visuals().strong_text_color();
    let subtitle_color = ui.visuals().weak_text_color();
    let title_galley = painter.layout_no_wrap(
        title.to_owned(),
        egui::TextStyle::Heading.resolve(ui.style()),
        title_color,
    );
    let subtitle_galley = subtitle.map(|subtitle| {
        painter.layout_no_wrap(
            subtitle.to_owned(),
            egui::TextStyle::Small.resolve(ui.style()),
            subtitle_color,
        )
    });
    let (title_pos, subtitle_pos) = board_identity_text_positions(
        rect,
        title_galley.size().y,
        subtitle_galley.as_ref().map(|galley| galley.size().y),
    );
    painter.galley(title_pos, title_galley, title_color);
    if let (Some(galley), Some(position)) = (subtitle_galley, subtitle_pos) {
        painter.galley(position, galley, subtitle_color);
    }
    response
}

fn open_board_information(
    app: &mut PealayerApp,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    tab: usize,
) {
    app.board_name_draft = capabilities.board_identity.stored_name.clone();
    app.board_info_tab = tab;
    if tab == 2 {
        app.front_panel_refresh_attempted = false;
    }
    app.show_board_info_dialog = true;
}

fn reconfigure_hardware_connection(app: &mut PealayerApp, connect: bool) -> Result<(), String> {
    let endpoint = app.serial_port.trim().to_owned();
    if connect && endpoint.is_empty() {
        return Err(app.tr("Enter a hardware endpoint before connecting."));
    }
    app.connection_notice = None;
    app.engine_handle
        .sender
        .send(crate::four_d::engine::EngineMessage::ReconfigureEndpoint { endpoint, connect })
        .map_err(|_| "hardware engine is unavailable".to_string())
}

fn draw_board_card_context_menu(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    title: &str,
    subtitle: Option<&str>,
) {
    ui.strong(title);
    if let Some(subtitle) = subtitle {
        ui.label(egui::RichText::new(subtitle).small().weak());
    }
    ui.separator();

    for (tab, icon, label) in [
        (0, crate::ui::icons::INFO, "Board information"),
        (1, crate::ui::icons::CIRCUITRY, "Capabilities"),
        (2, crate::ui::icons::APP_WINDOW, "Front panel"),
        (3, crate::ui::icons::SLIDERS_HORIZONTAL, "Board settings"),
    ] {
        if ui.button(format!("{icon} {}", app.tr(label))).clicked() {
            open_board_information(app, capabilities, tab);
            ui.close();
        }
    }
    if ui
        .button(format!(
            "{} {}",
            crate::ui::icons::PENCIL_SIMPLE,
            app.tr("Rename board")
        ))
        .clicked()
    {
        open_board_information(app, capabilities, 0);
        ui.close();
    }
    if ui
        .button(format!(
            "{} {}",
            crate::ui::icons::ARROW_CLOCKWISE,
            app.tr("Refresh live status")
        ))
        .clicked()
    {
        app.engine_handle.request_catalog_refresh();
        ui.close();
    }

    ui.separator();
    crate::ui::icons::submenu(
        ui,
        format!("{} {}", crate::ui::icons::PLUG, app.tr("Connection")),
        |ui| {
            ui.label(
                egui::RichText::new(crate::media::redact_media_target(&app.serial_port))
                    .small()
                    .weak(),
            );
            ui.separator();
            if ui
                .button(format!(
                    "{} {}",
                    crate::ui::icons::ARROW_CLOCKWISE,
                    app.tr("Reconnect")
                ))
                .clicked()
            {
                if let Err(error) = reconfigure_hardware_connection(app, true) {
                    app.set_osd(error);
                }
                ui.close();
            }
            if ui
                .button(format!("{} {}", crate::ui::icons::X, app.tr("Disconnect")))
                .clicked()
            {
                if let Err(error) = reconfigure_hardware_connection(app, false) {
                    app.set_osd(error);
                }
                ui.close();
            }
        },
    );
    crate::ui::icons::submenu(
        ui,
        format!("{} {}", crate::ui::icons::COPY, app.tr("Copy")),
        |ui| {
            for (label, value) in [
                (app.tr("Board name"), capabilities.board_name.as_str()),
                (
                    app.tr("Profile"),
                    capabilities
                        .board_profile
                        .as_ref()
                        .map(|profile| profile.key.as_str())
                        .unwrap_or_default(),
                ),
                (app.tr("Hardware endpoint"), app.serial_port.as_str()),
            ] {
                if ui
                    .add_enabled(
                        !value.trim().is_empty(),
                        egui::Button::new(format!("{} {label}", crate::ui::icons::COPY)),
                    )
                    .clicked()
                {
                    ui.ctx().copy_text(value.to_string());
                    ui.close();
                }
            }
        },
    );

    crate::ui::icons::submenu(
        ui,
        format!("{} {}", crate::ui::icons::EYE, app.tr("View options")),
        |ui| {
            let compact_label = app.tr("Compact controls");
            let relay_prefix_label = app.tr("Prefix relay identifiers");
            let mut changed = false;
            changed |= ui
                .checkbox(&mut app.compact_hardware_controls, compact_label)
                .changed();
            changed |= ui
                .checkbox(&mut app.prefix_relay_identifiers, relay_prefix_label)
                .changed();
            if changed {
                app.save_config();
            }
            ui.separator();
            ui.label(egui::RichText::new(app.tr("Non-user controls")).strong());
            draw_non_user_visibility_choices(app, ui);
        },
    );
    crate::ui::icons::submenu(
        ui,
        format!(
            "{} {}",
            crate::ui::icons::GEAR,
            app.tr("Connection behavior")
        ),
        |ui| {
            let auto_connect_label = app.tr("Connect automatically");
            let pause_disconnect_label = app.tr("Pause playback on disconnect");
            let mut changed = false;
            changed |= ui
                .checkbox(&mut app.auto_connect_hardware, auto_connect_label)
                .changed();
            changed |= ui
                .checkbox(
                    &mut app.pause_on_hardware_disconnect,
                    pause_disconnect_label,
                )
                .changed();
            if changed {
                app.save_config();
            }
        },
    );

    ui.separator();
    let mut estop = app.estop_active;
    if ui
        .checkbox(
            &mut estop,
            format!("{} {}", crate::ui::icons::WARNING, app.tr("E-STOP")),
        )
        .changed()
    {
        app.request_emergency_stop_change(estop);
        ui.close();
    }
}

pub(crate) fn update_control_name(
    app: &mut PealayerApp,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
    requested_name: String,
) {
    let requested_name = requested_name.trim().to_string();
    let restore_default = requested_name.is_empty()
        || (!control.default_name.is_empty() && requested_name == control.default_name);
    update_control_presentation(
        app,
        capabilities,
        control,
        "presentation-name",
        serde_json::json!({
            "name": if restore_default { String::new() } else { requested_name },
        }),
    );
}

pub(crate) fn update_control_group(
    app: &mut PealayerApp,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
    requested_group: String,
) {
    update_control_presentation(
        app,
        capabilities,
        control,
        "presentation-group",
        serde_json::json!({"group": requested_group.trim()}),
    );
}

pub(crate) fn update_control_presentation_flags(
    app: &mut PealayerApp,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
    hidden: Option<bool>,
    locked: Option<bool>,
) {
    let mut fields = serde_json::Map::new();
    if let Some(hidden) = hidden {
        fields.insert("hidden".to_string(), serde_json::Value::Bool(hidden));
    }
    if let Some(locked) = locked {
        fields.insert("locked".to_string(), serde_json::Value::Bool(locked));
    }
    update_control_presentation(
        app,
        capabilities,
        control,
        "presentation-policy",
        serde_json::Value::Object(fields),
    );
}

/// Apply the same presentation policy to several stable channel keys. Bulk
/// edits intentionally omit an optimistic profile revision: the controller
/// processes these tracked calls serially, and reusing the one revision from
/// the initial catalog would make every request after the first look stale.
pub(crate) fn update_control_presentation_flags_bulk(
    app: &mut PealayerApp,
    controls: &[crate::four_d::controller::HardwareControl],
    hidden: Option<bool>,
    locked: Option<bool>,
) {
    for control in controls {
        let mut params = serde_json::Map::new();
        params.insert(
            "key".to_string(),
            serde_json::Value::String(control.key.clone()),
        );
        if let Some(hidden) = hidden {
            params.insert("hidden".to_string(), serde_json::Value::Bool(hidden));
        }
        if let Some(locked) = locked {
            params.insert("locked".to_string(), serde_json::Value::Bool(locked));
        }
        if let Err(error) = app.engine_handle.request_controller_call(
            format!("presentation-bulk-policy:{}", control.key),
            "controller.peripheral.presentation.update",
            serde_json::Value::Object(params),
        ) {
            app.set_osd(error);
            break;
        }
    }
}

pub(crate) fn update_control_presentation(
    app: &mut PealayerApp,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
    operation: &str,
    fields: serde_json::Value,
) {
    let params = control_presentation_update_params(capabilities, control, fields);
    if let Err(error) = app.engine_handle.request_controller_call(
        format!("{operation}:{}", control.key),
        "controller.peripheral.presentation.update",
        params,
    ) {
        app.set_osd(error);
    }
}

fn draw_control_icon_picker(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
    size: f32,
) {
    let search_hint = app.tr("Search icons...");
    let presets_label = app.tr("Presets");
    let no_matches_label = app.tr("No matching icons");
    let default_label = app.tr("Use channel default");
    let tooltip = app.tr("Choose channel icon");
    let mut selected = control.icon.clone();
    if crate::ui::icons::searchable_icon_button(
        ui,
        ("hardware-card-icon-picker", control.key.as_str()),
        &mut selected,
        size,
        &tooltip,
        crate::ui::icons::IconPickerConfig {
            language: app.language,
            presets: crate::ui::icons::CONTROL_ICON_PRESETS,
            fallback_glyph: crate::ui::icons::control(&control.kind, ""),
            fallback_name: &default_label,
            width: 260.0,
            show_selected_name: true,
            search_hint: &search_hint,
            presets_label: &presets_label,
            no_matches_label: &no_matches_label,
            clear_label: Some(&default_label),
        },
    ) {
        update_control_presentation(
            app,
            capabilities,
            control,
            "presentation-icon",
            serde_json::json!({"icon": selected}),
        );
    }
}

fn reordered_channel_rank(
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    drop: &HardwareChannelDrop,
) -> Option<u16> {
    let source = capabilities
        .controls
        .iter()
        .find(|control| control.key == drop.source_key)?;
    let target = capabilities
        .controls
        .iter()
        .find(|control| control.key == drop.target_key)?;
    if source.kind != target.kind {
        return None;
    }
    let mut peers = capabilities
        .controls
        .iter()
        .filter(|control| control.kind == source.kind)
        .collect::<Vec<_>>();
    peers.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then_with(|| left.key.cmp(&right.key))
    });
    let source_index = peers
        .iter()
        .position(|control| control.key == drop.source_key)?;
    peers.remove(source_index);
    let target_index = peers
        .iter()
        .position(|control| control.key == drop.target_key)?;
    let insertion = target_index + usize::from(!drop.before);
    u16::try_from(insertion).ok()
}

pub(crate) fn persist_channel_drop(
    app: &mut PealayerApp,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    drop: HardwareChannelDrop,
) {
    let Some(order) = reordered_channel_rank(capabilities, &drop) else {
        app.set_osd(app.tr("Unable to reorder these channels"));
        return;
    };
    let Some(control) = capabilities
        .controls
        .iter()
        .find(|control| control.key == drop.source_key)
    else {
        return;
    };
    let params = control_presentation_update_params(
        capabilities,
        control,
        serde_json::json!({"order": order}),
    );
    if let Err(error) = app.engine_handle.request_controller_call(
        format!("presentation-order:{}", control.key),
        "controller.peripheral.presentation.update",
        params,
    ) {
        app.set_osd(error);
        return;
    }
    let optimistic_error = app
        .engine_handle
        .hardware_capabilities
        .lock()
        .ok()
        .and_then(|mut current| {
            current
                .as_mut()
                .and_then(|current| current.apply_control_reorder(&control.key, order).err())
        });
    if let Some(error) = optimistic_error {
        app.set_osd(error);
        app.engine_handle.request_catalog_refresh();
        return;
    }
    app.set_osd(app.tr("Saving channel order..."));
}

pub(crate) fn move_control_by(
    app: &mut PealayerApp,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    key: &str,
    delta: i32,
) {
    let Some(source) = capabilities
        .controls
        .iter()
        .find(|control| control.key == key)
    else {
        return;
    };
    let mut peers = capabilities
        .controls
        .iter()
        .filter(|control| control.kind == source.kind)
        .collect::<Vec<_>>();
    peers.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then_with(|| left.key.cmp(&right.key))
    });
    let Some(index) = peers.iter().position(|control| control.key == key) else {
        return;
    };
    let target_index =
        (index as i32 + delta).clamp(0, peers.len().saturating_sub(1) as i32) as usize;
    if target_index == index {
        return;
    }
    persist_channel_drop(
        app,
        capabilities,
        HardwareChannelDrop {
            source_key: key.to_string(),
            target_key: peers[target_index].key.clone(),
            before: target_index < index,
        },
    );
}

pub(crate) fn set_control_order(
    app: &mut PealayerApp,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    key: &str,
    requested_order: u16,
) {
    let Some(control) = capabilities
        .controls
        .iter()
        .find(|control| control.key == key)
    else {
        return;
    };
    let peer_count = capabilities
        .controls
        .iter()
        .filter(|candidate| candidate.kind == control.kind)
        .count();
    if peer_count == 0 {
        return;
    }
    let order = requested_order.min(u16::try_from(peer_count - 1).unwrap_or(u16::MAX));
    if order == control.order {
        return;
    }
    let params = control_presentation_update_params(
        capabilities,
        control,
        serde_json::json!({"order": order}),
    );
    if let Err(error) = app.engine_handle.request_controller_call(
        format!("presentation-order:{}", control.key),
        "controller.peripheral.presentation.update",
        params,
    ) {
        app.set_osd(error);
        return;
    }
    if let Ok(mut current) = app.engine_handle.hardware_capabilities.lock()
        && let Some(current) = current.as_mut()
    {
        let _ = current.apply_control_reorder(&control.key, order);
    }
    app.set_osd(app.tr("Saving channel order..."));
}

fn control_presentation_update_params(
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
    fields: serde_json::Value,
) -> serde_json::Value {
    let mut params = fields.as_object().cloned().unwrap_or_default();
    params.insert(
        "key".to_string(),
        serde_json::Value::String(control.key.clone()),
    );
    let expected_revision = capabilities
        .board_profile
        .as_ref()
        .map(|profile| profile.revision.clone())
        .filter(|revision| !revision.is_empty());
    if let Some(revision) = expected_revision {
        params.insert(
            "expected_revision".to_string(),
            serde_json::Value::String(revision),
        );
    }
    serde_json::Value::Object(params)
}

fn control_supports_presentation_policy(
    control: &crate::four_d::controller::HardwareControl,
) -> bool {
    relay_id_from_control_key(&control.key).is_some() || is_pwm_control(control)
}

pub(crate) fn is_motion_control(control: &crate::four_d::controller::HardwareControl) -> bool {
    matches!(
        control.kind.to_ascii_lowercase().as_str(),
        "motion" | "seat"
    ) || control.control.to_ascii_lowercase().contains("motion")
        || control.actions.iter().any(|action| {
            matches!(
                action.verb.to_ascii_lowercase().as_str(),
                "up" | "down" | "stop"
            )
        })
}

fn card_control_actions<'a>(
    control: &'a crate::four_d::controller::HardwareControl,
) -> Vec<&'a crate::four_d::controller::HardwareAction> {
    if !is_motion_control(control) {
        return control.actions.iter().collect();
    }
    control
        .actions
        .iter()
        .filter(|action| !action.verb.eq_ignore_ascii_case("stop"))
        .collect()
}

pub(crate) fn contextual_stop_action<'a>(
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &'a crate::four_d::controller::HardwareControl,
) -> Option<&'a crate::four_d::controller::HardwareAction> {
    (is_motion_control(control) && motion_control_is_active(capabilities, control))
        .then(|| {
            control
                .actions
                .iter()
                .find(|action| action.verb.eq_ignore_ascii_case("stop"))
        })
        .flatten()
}

fn invoke_advertised_action(app: &PealayerApp, control_key: &str, action_id: &str) {
    let _ = app.engine_handle.queue_controller_intent(
        control_key,
        "controller.action.invoke",
        serde_json::json!({"action_id": action_id}),
        true,
    );
}

pub(crate) fn invoke_held_motion_action(
    app: &PealayerApp,
    control_key: &str,
    action_id: &str,
) {
    let mut parts = action_id.split('.');
    if parts.next() == Some("raw-motion")
        && let (Some(side), Some(verb), None) = (parts.next(), parts.next(), parts.next())
        && matches!(side, "left" | "right")
        && matches!(verb, "up" | "down" | "stop")
    {
        let _ = app.engine_handle.queue_controller_intent(
            control_key,
            "controller.command.execute",
            serde_json::json!({"command": format!("relay side {side} {verb}")}),
            false,
        );
        return;
    }
    invoke_advertised_action(app, control_key, action_id);
}

pub(crate) fn update_held_motion_action(
    app: &mut PealayerApp,
    ui: &egui::Ui,
    response: &egui::Response,
    control: &crate::four_d::controller::HardwareControl,
    action: &crate::four_d::controller::HardwareAction,
    stop: &crate::four_d::controller::HardwareAction,
) {
    let action_id = action.id.as_str();
    let primary_down = ui.input(|input| input.pointer.primary_down());
    match hold_motion_transition(
        app.held_motion_action
            .as_ref()
            .map(|value| value.0.as_str()),
        action_id,
        response.is_pointer_button_down_on(),
        primary_down,
    ) {
        HoldMotionTransition::Start => {
            if let Some((_, previous_stop, previous_control_key)) =
                app.held_motion_action.take()
            {
                invoke_held_motion_action(app, &previous_control_key, &previous_stop);
            }
            crate::ui::hardware_control::invoke_action(app, control, action);
            app.held_motion_action = Some((
                action_id.to_string(),
                stop.id.clone(),
                control.key.clone(),
            ));
        }
        HoldMotionTransition::Stop => {
            if let Some((_, stop, control_key)) = app.held_motion_action.take() {
                invoke_held_motion_action(app, &control_key, &stop);
            }
        }
        HoldMotionTransition::None => {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoldMotionTransition {
    None,
    Start,
    Stop,
}

fn hold_motion_transition(
    active_action: Option<&str>,
    action_id: &str,
    pointer_down_on_button: bool,
    primary_down: bool,
) -> HoldMotionTransition {
    if pointer_down_on_button && primary_down && active_action != Some(action_id) {
        HoldMotionTransition::Start
    } else if !primary_down && active_action == Some(action_id) {
        HoldMotionTransition::Stop
    } else {
        HoldMotionTransition::None
    }
}

pub(crate) fn motion_control_is_active(
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
) -> bool {
    matches!(
        motion_control_direction(capabilities, control),
        MotionDirectionState::Up | MotionDirectionState::Down
    )
}

fn open_control_dialog(
    app: &mut PealayerApp,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
) {
    app.show_hardware_channels_dialog = true;
    app.hardware_channel_detail_active = true;
    app.hardware_control_dialog_key = Some(control.key.clone());
    app.hardware_control_name_draft = control.name.clone();
    app.hardware_control_group_draft = control.group.clone();
    app.hardware_control_icon_draft = control.icon.clone();
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
        .map(|raw| {
            let channel = control
                .key
                .strip_prefix("pwm.")
                .and_then(|value| value.parse().ok())
                .unwrap_or_default();
            capabilities.pwm_percent(channel, raw)
        })
        .unwrap_or(0.0);
}

fn draw_control_context_menu(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
    edit_id: egui::Id,
    draft_id: egui::Id,
    focus_pending_id: egui::Id,
    group_edit_id: egui::Id,
    group_draft_id: egui::Id,
) {
    ui.horizontal(|ui| {
        ui.label(crate::ui::icons::control(&control.kind, &control.icon));
        ui.strong(crate::ui::i18n::visual_text(app.language, &control.name));
    });
    ui.label(egui::RichText::new(&control.key).monospace().weak().small());
    ui.separator();

    if ui
        .button(format!(
            "{} {}",
            crate::ui::icons::SLIDERS_HORIZONTAL,
            app.tr("Manage...")
        ))
        .clicked()
    {
        open_control_dialog(app, capabilities, control);
        ui.close();
    }

    if !control.actions.is_empty() {
        crate::ui::icons::submenu(
            ui,
            format!("{} {}", crate::ui::icons::PLAY, app.tr("Actions")),
            |ui| {
                for action in &control.actions {
                    if ui
                        .add_enabled(
                            !app.estop_active && !control.locked,
                            egui::Button::new(format!(
                                "{} {}",
                                crate::ui::icons::action(&action.verb),
                                crate::ui::i18n::visual_text(app.language, &action.name)
                            )),
                        )
                        .clicked()
                    {
                        crate::ui::hardware_control::invoke_action(app, control, action);
                        ui.close();
                    }
                }
            },
        );
    } else if let Some(relay_id) = relay_id_from_control_key(&control.key) {
        for (state, icon, label) in [
            (true, crate::ui::icons::LIGHTNING, app.tr("Turn on")),
            (false, crate::ui::icons::STOP_CIRCLE, app.tr("Turn off")),
        ] {
            if ui
                .add_enabled(
                    !app.estop_active && !control.locked,
                    egui::Button::new(format!("{icon} {label}")),
                )
                .clicked()
            {
                crate::ui::hardware_control::set_relay(app, relay_id, state);
                ui.close();
            }
        }
    }

    if is_motion_control(control) {
        ui.separator();
        let hold_label = app.tr("Run only while held");
        let toggle_label = app.tr("Toggle on press");
        let hold_help = app.tr("Move only while the button is held");
        let toggle_help = app.tr("Keep moving until Stop is pressed");
        crate::ui::icons::submenu(
            ui,
            format!(
                "{} {}",
                crate::ui::icons::SLIDERS_HORIZONTAL,
                app.tr("Button behavior")
            ),
            |ui| {
                let mut changed = false;
                changed |= ui
                    .radio_value(
                        &mut app.motion_control_mode,
                        crate::config::MotionControlMode::Hold,
                        &hold_label,
                    )
                    .on_hover_text(&hold_help)
                    .changed();
                changed |= ui
                    .radio_value(
                        &mut app.motion_control_mode,
                        crate::config::MotionControlMode::Toggle,
                        &toggle_label,
                    )
                    .on_hover_text(&toggle_help)
                    .changed();
                if changed {
                    app.save_config();
                }
            },
        );
        crate::ui::icons::submenu(
            ui,
            format!("{} {}", crate::ui::icons::EYE, app.tr("Non-user controls")),
            |ui| draw_non_user_visibility_choices(app, ui),
        );
    }

    ui.separator();
    if control_supports_presentation_policy(control) {
        let mut locked = control.locked;
        if ui
            .checkbox(
                &mut locked,
                format!("{} {}", crate::ui::icons::LOCK, app.tr("Lock channel")),
            )
            .on_hover_text(app.tr("Prevent control changes until this channel is unlocked"))
            .changed()
        {
            update_control_presentation_flags(app, capabilities, control, None, Some(locked));
            ui.close();
        }
        if ui
            .button(format!(
                "{} {}",
                crate::ui::icons::EYE_SLASH,
                app.tr("Hide channel")
            ))
            .on_hover_text(app.tr("Hide this channel from the Hardware Monitor"))
            .clicked()
        {
            update_control_presentation_flags(app, capabilities, control, Some(true), None);
            ui.close();
        }
        ui.separator();
    }
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
            data.insert_temp(focus_pending_id, true);
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
        && ui
            .button(format!(
                "{} {}",
                crate::ui::icons::ARROW_COUNTER_CLOCKWISE,
                app.tr("Restore default name")
            ))
            .clicked()
    {
        update_control_name(app, capabilities, control, String::new());
        ui.close();
    }
}

fn control_context_popup(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    card_response: &egui::Response,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
    edit_id: egui::Id,
    draft_id: egui::Id,
    focus_pending_id: egui::Id,
    group_edit_id: egui::Id,
    group_draft_id: egui::Id,
) {
    // Read the pointer directly instead of relying on the frame response. Child
    // buttons and labels own their own responses, so a normal context_menu on
    // the frame only worked in its empty padding and appeared to be missing.
    let open = secondary_click_inside(ui.ctx(), card_response.rect);
    egui::Popup::menu(card_response)
        .id(ui.make_persistent_id(("hardware-control-context", control.key.as_str())))
        .at_pointer_fixed()
        .open_memory(open.then_some(egui::SetOpenCommand::Bool(true)))
        .show(|ui| {
            draw_control_context_menu(
                app,
                ui,
                capabilities,
                control,
                edit_id,
                draft_id,
                focus_pending_id,
                group_edit_id,
                group_draft_id,
            );
        });
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

fn hardware_section(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    icon: &str,
    title: &str,
    default_open: bool,
    body: impl FnOnce(&mut egui::Ui),
) {
    let id = ui.make_persistent_id(id_salt);
    let mut open = ui.data_mut(|data| data.get_persisted::<bool>(id).unwrap_or(default_open));
    ui.add_space(7.0);
    ui.horizontal(|ui| {
        let caret = if open {
            crate::ui::icons::CARET_DOWN
        } else {
            crate::ui::icons::CARET_RIGHT
        };
        let response = ui.add(
            egui::Button::new(egui::RichText::new(format!("{caret}  {icon}  {title}")).strong())
                .frame(false),
        );
        if response.clicked() {
            open = !open;
        }
        let (line, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().line_segment(
            [line.left_center(), line.right_center()],
            ui.visuals().widgets.noninteractive.bg_stroke,
        );
    });
    ui.data_mut(|data| data.insert_persisted(id, open));
    if open {
        ui.indent(id.with("content"), |ui| {
            ui.add_space(5.0);
            body(ui);
        });
    }
}

fn parse_rf_code(value: &str) -> Option<u32> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .map(|hex| u32::from_str_radix(hex, 16).ok())
        .unwrap_or_else(|| trimmed.parse::<u32>().ok())
}

fn board_tool_card(ui: &mut egui::Ui, icon: &str, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    let outer_width = ui.available_width();
    ui.set_width(outer_width);
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(12, 10))
        .stroke(egui::Stroke::new(
            HARDWARE_CARD_STROKE_WIDTH,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(9.0)
        .show(ui, |ui| {
            configure_hardware_card_controls(ui);
            ui.set_width(hardware_frame_content_width(outer_width, 12));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(icon).size(18.0));
                ui.label(egui::RichText::new(title).strong());
            });
            ui.add_space(7.0);
            body(ui);
        });
}

fn draw_buzzer_tool(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
) {
    let melody_id = ui.make_persistent_id("hardware_buzzer_melody");
    let repeats_id = ui.make_persistent_id("hardware_buzzer_repeats");
    let frequency_id = ui.make_persistent_id("hardware_buzzer_frequency");
    let duration_id = ui.make_persistent_id("hardware_buzzer_duration");
    let mut melody = ui.data_mut(|data| {
        data.get_persisted::<String>(melody_id)
            .filter(|name| capabilities.melodies.iter().any(|item| item.name == *name))
            .or_else(|| capabilities.melodies.first().map(|item| item.name.clone()))
            .unwrap_or_default()
    });
    let mut repeats = ui.data_mut(|data| {
        data.get_persisted::<u8>(repeats_id)
            .unwrap_or(1)
            .min(20)
    });
    let mut frequency_hz = ui.data_mut(|data| {
        data.get_persisted::<u16>(frequency_id)
            .unwrap_or(880)
            .clamp(20, 20_000)
    });
    let mut duration_ms = ui.data_mut(|data| {
        data.get_persisted::<u16>(duration_id)
            .unwrap_or(180)
            .clamp(1, 60_000)
    });
    let pending = app.board_operation.is_some();
    let can_start = !pending && !app.estop_active;
    let can_stop = !pending;
    let playing = capabilities.buzzer.is_playing();
    let board_silent = capabilities
        .settings
        .as_ref()
        .is_some_and(|settings| settings.silent);
    let mut refresh_catalog = false;
    let mut play_melody = false;
    let mut play_tone = false;
    let mut stop = false;

    board_tool_card(
        ui,
        crate::ui::icons::SPEAKER_HIGH,
        &app.tr("Buzzer & melodies"),
        |ui| {
            ui.horizontal_wrapped(|ui| {
                let state_color = if playing {
                    egui::Color32::from_rgb(34, 197, 94)
                } else {
                    ui.visuals().weak_text_color()
                };
                ui.colored_label(state_color, crate::ui::icons::DOT_OUTLINE);
                if playing {
                    let name = capabilities.buzzer.melody_name.trim();
                    ui.strong(if name.is_empty() {
                        app.tr("Playing melody")
                    } else {
                        format!("{}: {name}", app.tr("Playing"))
                    });
                } else {
                    ui.label(app.tr("Idle"));
                }
                ui.separator();
                let route_icon = if board_silent {
                    crate::ui::icons::SPEAKER_SLASH
                } else {
                    crate::ui::icons::SPEAKER_HIGH
                };
                let route_color = if board_silent {
                    ui.visuals().warn_fg_color
                } else {
                    ui.visuals().weak_text_color()
                };
                ui.colored_label(
                    route_color,
                    format!(
                        "{route_icon} {}",
                        app.tr(if board_silent { "Board muted" } else { "Board audible" })
                    ),
                );
                if pending {
                    ui.spinner();
                }
            });

            ui.add_space(8.0);
            ui.label(egui::RichText::new(app.tr("Configured melody")).strong());
            ui.horizontal_wrapped(|ui| {
                let selected_text = if melody.is_empty() {
                    app.tr("No configured melodies")
                } else {
                    melody.clone()
                };
                let response = egui::ComboBox::from_id_salt("hardware_buzzer_melody_combo")
                    .width(210.0)
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        if capabilities.melodies.is_empty() {
                            ui.weak(app.tr("No configured melodies"));
                        }
                        for item in &capabilities.melodies {
                            let duration = crate::duration::format_effect_duration_for_language(
                                app.language,
                                item.duration_ms(),
                            );
                            let detail = format!(
                                "{} · {} {}",
                                duration,
                                item.notes.len(),
                                app.tr("notes")
                            );
                            ui.selectable_value(
                                &mut melody,
                                item.name.clone(),
                                format!("{}  {}  ·  {detail}", crate::ui::icons::MUSIC_NOTE, item.name),
                            );
                        }
                    })
                    .response;
                refresh_catalog |= response.clicked();
                if ui
                    .small_button(crate::ui::icons::ARROW_CLOCKWISE)
                    .on_hover_text(app.tr("Refresh melody catalog"))
                    .clicked()
                {
                    refresh_catalog = true;
                }
            });

            ui.horizontal_wrapped(|ui| {
                let mut loop_until_stopped = repeats == 0;
                if ui
                    .toggle_value(&mut loop_until_stopped, app.tr("Loop until stopped"))
                    .changed()
                {
                    repeats = if loop_until_stopped { 0 } else { 1 };
                }
                if !loop_until_stopped {
                    ui.label(app.tr("Repeats"));
                    ui.add(egui::DragValue::new(&mut repeats).range(1..=20));
                }
            });

            ui.horizontal_wrapped(|ui| {
                play_melody = ui
                    .add_enabled(
                        can_start && !melody.is_empty(),
                        egui::Button::new(format!(
                            "{} {}",
                            crate::ui::icons::PLAY,
                            app.tr("Play melody")
                        )),
                    )
                    .clicked();
                stop = ui
                    .add_enabled(
                        can_stop,
                        egui::Button::new(format!(
                            "{} {}",
                            crate::ui::icons::STOP_CIRCLE,
                            app.tr("Stop buzzer")
                        )),
                    )
                    .clicked();
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(egui::RichText::new(app.tr("Tone test")).strong());
            egui::Grid::new("hardware_buzzer_tone_grid")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label(app.tr("Frequency"));
                    ui.add(
                        egui::DragValue::new(&mut frequency_hz)
                            .range(20..=20_000)
                            .suffix(" Hz"),
                    );
                    ui.end_row();
                    ui.label(app.tr("Duration"));
                    ui.add(
                        egui::DragValue::new(&mut duration_ms)
                            .range(1..=60_000)
                            .suffix(" ms"),
                    );
                    ui.end_row();
                });
            play_tone = ui
                .add_enabled(
                    can_start,
                    egui::Button::new(format!(
                        "{} {}",
                        crate::ui::icons::PLAY,
                        app.tr("Play tone")
                    )),
                )
                .clicked();
            if board_silent {
                ui.add_space(4.0);
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    format!(
                        "{} {}",
                        crate::ui::icons::WARNING,
                        app.tr("The physical board is muted; host routing may still be audible.")
                    ),
                );
            }
        },
    );

    ui.data_mut(|data| {
        data.insert_persisted(melody_id, melody.clone());
        data.insert_persisted(repeats_id, repeats);
        data.insert_persisted(frequency_id, frequency_hz);
        data.insert_persisted(duration_id, duration_ms);
    });
    if refresh_catalog {
        // Push events keep this catalog current; opening or explicitly
        // refreshing the picker is also a user-visible freshness boundary.
        app.engine_handle.request_catalog_refresh();
    }
    let result = if stop {
        app.stop_buzzer()
    } else if play_melody {
        app.play_buzzer_melody(&melody, repeats)
    } else if play_tone {
        app.play_buzzer_tone(frequency_hz, duration_ms)
    } else {
        Ok(())
    };
    if let Err(error) = result {
        app.set_osd(error);
    }
}

fn addressable_strip_gradient(
    pixels: u16,
    start: egui::Color32,
    end: egui::Color32,
    brightness: u8,
) -> Vec<u8> {
    let scale = f32::from(brightness) / 255.0;
    let denominator = f32::from(pixels.saturating_sub(1).max(1));
    let mut frame = Vec::with_capacity(usize::from(pixels) * 3);
    for index in 0..pixels {
        let amount = f32::from(index) / denominator;
        for (from, to) in [
            (start.r(), end.r()),
            (start.g(), end.g()),
            (start.b(), end.b()),
        ] {
            let channel = f32::from(from) + (f32::from(to) - f32::from(from)) * amount;
            frame.push((channel * scale).round().clamp(0.0, 255.0) as u8);
        }
    }
    frame
}

fn draw_addressable_strip_tool(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
) {
    let Some(strip) = capabilities.strip_control.as_ref() else {
        return;
    };
    let pixels_id = ui.make_persistent_id("hardware_strip_pixels");
    let fps_id = ui.make_persistent_id("hardware_strip_fps");
    let mode_id = ui.make_persistent_id("hardware_strip_mode");
    let color_id = ui.make_persistent_id("hardware_strip_color");
    let second_color_id = ui.make_persistent_id("hardware_strip_second_color");
    let brightness_id = ui.make_persistent_id("hardware_strip_brightness");
    let pixel_id = ui.make_persistent_id("hardware_strip_pixel");
    let effect_selection_id = ui.make_persistent_id("hardware_strip_effect");

    let mut pixels = ui.data_mut(|data| {
        data.get_persisted::<u16>(pixels_id)
            .unwrap_or(strip.default_pixels)
            .clamp(strip.minimum_pixels, strip.maximum_pixels)
    });
    let mut fps = ui.data_mut(|data| {
        data.get_persisted::<u8>(fps_id)
            .unwrap_or(strip.default_fps)
            .clamp(strip.minimum_fps, strip.maximum_fps)
    });
    let available_modes = ["solid", "pixel", "frame", "rainbow", "effect"]
        .into_iter()
        .filter(|mode| strip.supports(mode))
        .collect::<Vec<_>>();
    let mut mode = ui.data_mut(|data| {
        data.get_persisted::<String>(mode_id)
            .filter(|mode| available_modes.contains(&mode.as_str()))
            .or_else(|| available_modes.first().map(ToString::to_string))
            .unwrap_or_default()
    });
    let mut color = ui.data_mut(|data| {
        data.get_persisted::<egui::Color32>(color_id)
            .unwrap_or(egui::Color32::WHITE)
    });
    let mut second_color = ui.data_mut(|data| {
        data.get_persisted::<egui::Color32>(second_color_id)
            .unwrap_or(egui::Color32::from_rgb(0, 96, 255))
    });
    let mut brightness = ui.data_mut(|data| data.get_persisted::<u8>(brightness_id).unwrap_or(255));
    let mut pixel = ui.data_mut(|data| {
        data.get_persisted::<u16>(pixel_id)
            .unwrap_or(0)
            .min(pixels.saturating_sub(1))
    });
    let mut effect_id = ui.data_mut(|data| {
        data.get_persisted::<String>(effect_selection_id)
            .filter(|id| {
                capabilities
                    .strip_effects
                    .iter()
                    .any(|effect| &effect.id == id)
            })
            .or_else(|| {
                capabilities
                    .strip_effects
                    .first()
                    .map(|effect| effect.id.clone())
            })
            .unwrap_or_default()
    });
    let pending = app.hardware_effect_authoring.pending_operation.is_some();
    let mutation_enabled = !pending && !app.estop_active;
    let live = strip.running || app.hardware_effect_authoring.preview_active;

    board_tool_card(
        ui,
        crate::ui::icons::SPARKLE,
        &app.tr("Addressable LED strip"),
        |ui| {
            ui.horizontal_wrapped(|ui| {
                let state_color = if live {
                    egui::Color32::from_rgb(34, 197, 94)
                } else {
                    ui.visuals().weak_text_color()
                };
                ui.colored_label(state_color, crate::ui::icons::DOT_OUTLINE);
                ui.label(if live {
                    if strip.active_name.trim().is_empty() {
                        app.tr("Streaming")
                    } else {
                        strip.active_name.clone()
                    }
                } else {
                    app.tr("Idle")
                });
                ui.separator();
                ui.weak(format!(
                    "{}–{} {} · {}–{} FPS",
                    strip.minimum_pixels,
                    strip.maximum_pixels,
                    app.tr("pixels"),
                    strip.minimum_fps,
                    strip.maximum_fps
                ));
            });
            ui.add_space(7.0);

            egui::Grid::new("hardware_strip_configuration")
                .num_columns(2)
                .spacing([10.0, 7.0])
                .show(ui, |ui| {
                    ui.label(app.tr("Pixel count"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut pixels)
                                .range(strip.minimum_pixels..=strip.maximum_pixels),
                        );
                        if ui
                            .add_enabled(
                                mutation_enabled,
                                egui::Button::new(format!(
                                    "{} {}",
                                    crate::ui::icons::GEAR,
                                    app.tr("Configure")
                                )),
                            )
                            .clicked()
                        {
                            if let Err(error) = app.configure_addressable_strip(pixels) {
                                app.set_osd(error);
                            }
                        }
                    });
                    ui.end_row();

                    ui.label(app.tr("Mode"));
                    egui::ComboBox::from_id_salt("hardware_strip_mode_combo")
                        .selected_text(match mode.as_str() {
                            "solid" => app.tr("Solid color"),
                            "pixel" => app.tr("Single pixel"),
                            "frame" => app.tr("Color frame"),
                            "rainbow" => app.tr("Rainbow"),
                            "effect" => app.tr("Effect"),
                            _ => app.tr("Unavailable"),
                        })
                        .show_ui(ui, |ui| {
                            for advertised in &available_modes {
                                let label = match *advertised {
                                    "solid" => app.tr("Solid color"),
                                    "pixel" => app.tr("Single pixel"),
                                    "frame" => app.tr("Color frame"),
                                    "rainbow" => app.tr("Rainbow"),
                                    "effect" => app.tr("Effect"),
                                    _ => (*advertised).to_string(),
                                };
                                ui.selectable_value(&mut mode, (*advertised).to_string(), label);
                            }
                        });
                    ui.end_row();
                });

            ui.add_space(7.0);
            match mode.as_str() {
                "solid" => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(app.tr("Color"));
                        ui.color_edit_button_srgba(&mut color);
                        ui.label(app.tr("Brightness"));
                        ui.add(egui::Slider::new(&mut brightness, 0..=255).show_value(true));
                    });
                    if ui
                        .add_enabled(
                            mutation_enabled,
                            egui::Button::new(format!(
                                "{} {}",
                                crate::ui::icons::PALETTE,
                                app.tr("Fill strip")
                            )),
                        )
                        .clicked()
                    {
                        if let Err(error) =
                            app.fill_addressable_strip(color.r(), color.g(), color.b(), brightness)
                        {
                            app.set_osd(error);
                        }
                    }
                }
                "pixel" => {
                    pixel = pixel.min(pixels.saturating_sub(1));
                    ui.horizontal_wrapped(|ui| {
                        ui.label(app.tr("Pixel"));
                        ui.add(
                            egui::DragValue::new(&mut pixel).range(0..=pixels.saturating_sub(1)),
                        );
                        ui.label(app.tr("Color"));
                        ui.color_edit_button_srgba(&mut color);
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(app.tr("Brightness"));
                        ui.add(egui::Slider::new(&mut brightness, 0..=255).show_value(true));
                        if ui
                            .add_enabled(
                                mutation_enabled,
                                egui::Button::new(format!(
                                    "{} {}",
                                    crate::ui::icons::PAPER_PLANE_TILT,
                                    app.tr("Apply pixel")
                                )),
                            )
                            .clicked()
                        {
                            if let Err(error) = app.set_addressable_strip_pixel(
                                pixel,
                                pixels,
                                color.r(),
                                color.g(),
                                color.b(),
                                brightness,
                            ) {
                                app.set_osd(error);
                            }
                        }
                    });
                }
                "frame" => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(app.tr("Start color"));
                        ui.color_edit_button_srgba(&mut color);
                        ui.label(app.tr("End color"));
                        ui.color_edit_button_srgba(&mut second_color);
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(app.tr("Brightness"));
                        ui.add(egui::Slider::new(&mut brightness, 0..=255).show_value(true));
                        if ui
                            .add_enabled(
                                mutation_enabled,
                                egui::Button::new(format!(
                                    "{} {}",
                                    crate::ui::icons::PAPER_PLANE_TILT,
                                    app.tr("Send color frame")
                                )),
                            )
                            .clicked()
                        {
                            let frame =
                                addressable_strip_gradient(pixels, color, second_color, brightness);
                            if let Err(error) = app.send_addressable_strip_frame(pixels, &frame) {
                                app.set_osd(error);
                            }
                        }
                    });
                }
                "rainbow" => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(app.tr("Frame rate"));
                        ui.add(
                            egui::DragValue::new(&mut fps)
                                .range(strip.minimum_fps..=strip.maximum_fps)
                                .suffix(" FPS"),
                        );
                        if ui
                            .add_enabled(
                                mutation_enabled,
                                egui::Button::new(format!(
                                    "{} {}",
                                    crate::ui::icons::PLAY,
                                    app.tr("Start rainbow")
                                )),
                            )
                            .clicked()
                        {
                            if let Err(error) = app.start_addressable_strip_rainbow(pixels, fps) {
                                app.set_osd(error);
                            }
                        }
                    });
                }
                "effect" => {
                    if capabilities.strip_effects.is_empty() {
                        ui.weak(app.tr("No addressable LED effects are advertised"));
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            egui::ComboBox::from_id_salt("hardware_strip_effect_combo")
                                .selected_text(
                                    capabilities
                                        .strip_effects
                                        .iter()
                                        .find(|effect| effect.id == effect_id)
                                        .map(|effect| app.display_text(&effect.name))
                                        .unwrap_or_else(|| app.tr("Select effect")),
                                )
                                .show_ui(ui, |ui| {
                                    for effect in &capabilities.strip_effects {
                                        ui.selectable_value(
                                            &mut effect_id,
                                            effect.id.clone(),
                                            app.display_text(&effect.name),
                                        );
                                    }
                                });
                            ui.add(
                                egui::DragValue::new(&mut fps)
                                    .range(strip.minimum_fps..=strip.maximum_fps)
                                    .suffix(" FPS"),
                            );
                            if ui
                                .add_enabled(
                                    mutation_enabled && !effect_id.is_empty(),
                                    egui::Button::new(format!(
                                        "{} {}",
                                        crate::ui::icons::PLAY,
                                        app.tr("Start effect")
                                    )),
                                )
                                .clicked()
                            {
                                if let Err(error) =
                                    app.start_addressable_strip_effect(&effect_id, pixels, fps)
                                {
                                    app.set_osd(error);
                                }
                            }
                        });
                    }
                }
                _ => {}
            }

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        !pending,
                        egui::Button::new(format!(
                            "{} {}",
                            crate::ui::icons::ERASER,
                            app.tr("Clear strip")
                        )),
                    )
                    .clicked()
                {
                    if let Err(error) = app.clear_addressable_strip() {
                        app.set_osd(error);
                    }
                }
                if live
                    && ui
                        .add_enabled(
                            !pending,
                            egui::Button::new(format!(
                                "{} {}",
                                crate::ui::icons::STOP_CIRCLE,
                                app.tr("Stop stream")
                            )),
                        )
                        .clicked()
                {
                    if let Err(error) = app.stop_addressable_strip() {
                        app.set_osd(error);
                    }
                }
                if ui
                    .add_enabled(
                        !pending,
                        egui::Button::new(format!(
                            "{} {}",
                            crate::ui::icons::ARROW_CLOCKWISE,
                            app.tr("Refresh status")
                        )),
                    )
                    .clicked()
                {
                    if let Err(error) = app.refresh_addressable_strip_status() {
                        app.set_osd(error);
                    }
                }
            });
            if !app.hardware_effect_authoring.status.is_empty() {
                ui.add_space(5.0);
                ui.weak(&app.hardware_effect_authoring.status);
            }
        },
    );

    ui.data_mut(|data| {
        data.insert_persisted(pixels_id, pixels);
        data.insert_persisted(fps_id, fps);
        data.insert_persisted(mode_id, mode);
        data.insert_persisted(color_id, color);
        data.insert_persisted(second_color_id, second_color);
        data.insert_persisted(brightness_id, brightness);
        data.insert_persisted(pixel_id, pixel);
        data.insert_persisted(effect_selection_id, effect_id);
    });
}

fn default_display_text_target(segments_available: bool, lcd_available: bool) -> &'static str {
    if segments_available {
        "segments"
    } else if lcd_available {
        "lcd"
    } else {
        "segments"
    }
}

fn draw_display_text_tool(
    app: &PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
) {
    let text_id = ui.make_persistent_id("hardware_display_text");
    let target_id = ui.make_persistent_id("hardware_display_target");
    let duration_id = ui.make_persistent_id("hardware_display_duration");
    let mut text = ui.data_mut(|data| data.get_temp::<String>(text_id).unwrap_or_default());
    // The front-panel segments are the primary board-facing display. Keep LCD
    // and the combined target available, but never surprise a new Display Text
    // action by broadcasting to both displays.
    let default_target = default_display_text_target(
        capabilities.supports_segment_display,
        capabilities.supports_lcd_display,
    );
    let mut target = ui.data_mut(|data| {
        data.get_temp::<String>(target_id)
            .unwrap_or_else(|| default_target.to_string())
    });
    if target == "both"
        && !(capabilities.supports_segment_display && capabilities.supports_lcd_display)
    {
        target = default_target.to_string();
    }
    let mut duration_ms = ui.data_mut(|data| data.get_temp::<u64>(duration_id).unwrap_or(5_000));

    board_tool_card(
        ui,
        crate::ui::icons::MONITOR_PLAY,
        &app.tr("Display text"),
        |ui| {
            let text_align = crate::ui::i18n::input_alignment(app.rtl, &text);
            let response = ui.add_sized(
                [ui.available_width(), 54.0],
                egui::TextEdit::multiline(&mut text)
                    .desired_rows(2)
                    .horizontal_align(text_align)
                    .hint_text(app.tr("Message for the board displays")),
            );
            if response.changed() {
                ui.data_mut(|data| data.insert_temp(text_id, text.clone()));
            }
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if capabilities.supports_segment_display && capabilities.supports_lcd_display {
                    egui::ComboBox::from_id_salt("hardware_display_target_combo")
                        .selected_text(match target.as_str() {
                            "segments" => app.tr("Segments"),
                            "lcd" => app.tr("LCD"),
                            _ => app.tr("Both displays"),
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut target,
                                "both".to_string(),
                                app.tr("Both displays"),
                            );
                            ui.selectable_value(
                                &mut target,
                                "segments".to_string(),
                                app.tr("Segments"),
                            );
                            ui.selectable_value(&mut target, "lcd".to_string(), app.tr("LCD"));
                        });
                } else {
                    ui.label(
                        egui::RichText::new(if target == "lcd" {
                            app.tr("LCD")
                        } else {
                            app.tr("Segments")
                        })
                        .weak(),
                    );
                }
                ui.label(app.tr("Duration"));
                ui.add(crate::duration::time_value_drag(
                    &mut duration_ms,
                    250..=60_000,
                    250.0,
                    app.human_readable_time_units,
                ));
                if ui
                    .add_enabled(
                        !text.trim().is_empty(),
                        egui::Button::new(format!(
                            "{} {}",
                            crate::ui::icons::PAPER_PLANE_TILT,
                            app.tr("Send")
                        )),
                    )
                    .clicked()
                {
                    let _ = app.engine_handle.sender.send(
                        crate::four_d::engine::EngineMessage::ControllerCall {
                            method: "controller.display.send".to_string(),
                            params: serde_json::json!({
                                "target": target,
                                "text": text.trim(),
                                "duration_ms": duration_ms,
                            }),
                        },
                    );
                }
            });
        },
    );
    ui.data_mut(|data| {
        data.insert_temp(target_id, target);
        data.insert_temp(duration_id, duration_ms);
    });
}

fn draw_rf_code_tool(app: &mut PealayerApp, ui: &mut egui::Ui) {
    if ui.button(format!("{} {}", crate::ui::icons::RADIO, app.tr("Manage RF…"))).clicked() {
        app.rf.open = true;
        if let Err(error) = app.request_rf("catalog", serde_json::json!({"read_board":true})) { app.rf.error = error; }
    }
    let code_id = ui.make_persistent_id("hardware_rf_code");
    let bits_id = ui.make_persistent_id("hardware_rf_bits");
    let protocol_id = ui.make_persistent_id("hardware_rf_protocol");
    let mut code = ui.data_mut(|data| data.get_temp::<String>(code_id).unwrap_or_default());
    let mut bits = ui.data_mut(|data| data.get_temp::<u8>(bits_id).unwrap_or(24));
    let mut protocol = ui.data_mut(|data| data.get_temp::<u8>(protocol_id).unwrap_or(1));
    let parsed = parse_rf_code(&code);

    board_tool_card(ui, crate::ui::icons::RADIO, &app.tr("RF code"), |ui| {
        let response = ui.add_sized(
            [ui.available_width(), 28.0],
            crate::ui::dialog::singleline_text_edit(&mut code)
                .hint_text("0x12AB34")
                .font(egui::TextStyle::Monospace),
        );
        if response.changed() {
            ui.data_mut(|data| data.insert_temp(code_id, code.clone()));
        }
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(app.tr("Bits"));
            ui.add(egui::DragValue::new(&mut bits).range(1..=32));
            ui.label(app.tr("Protocol"));
            ui.add(egui::DragValue::new(&mut protocol).range(1..=12));
            let valid = parsed.is_some();
            ui.colored_label(
                if valid {
                    egui::Color32::from_rgb(34, 197, 94)
                } else {
                    ui.visuals().weak_text_color()
                },
                if code.trim().is_empty() {
                    app.tr("Hex or decimal")
                } else if valid {
                    app.tr("Ready")
                } else {
                    app.tr("Invalid code")
                },
            );
            if ui
                .add_enabled(
                    valid,
                    egui::Button::new(format!(
                        "{} {}",
                        crate::ui::icons::PAPER_PLANE_TILT,
                        app.tr("Transmit")
                    )),
                )
                .clicked()
            {
                let _ = app.engine_handle.sender.send(
                    crate::four_d::engine::EngineMessage::ControllerCall {
                        method: "controller.rf.transmit".to_string(),
                        params: serde_json::json!({
                            "code": parsed.expect("button is enabled only for parsed codes"),
                            "bits": bits,
                            "protocol": protocol,
                        }),
                    },
                );
            }
        });
    });
    ui.data_mut(|data| {
        data.insert_temp(bits_id, bits);
        data.insert_temp(protocol_id, protocol);
    });
}

/// Stop can disappear between mouse-down and mouse-up. Explicit child IDs
/// prevent its captured gesture being inherited by the next caption/button.
/// `push_id` alone is insufficient: egui's auto widget IDs still depend on the
/// child's position unless its UI has an explicit ID independent of that slot.
fn hardware_header_widget<R>(
    ui: &mut egui::Ui,
    channel_key: &str,
    role: &str,
    render: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let id = ui.make_persistent_id(("hardware-header-widget", channel_key, role));
    ui.scope_builder(egui::UiBuilder::new().id(id), render)
        .inner
}

/// Keep transformable card layers attached to their workspace panel rather
/// than as independent Middle windows that can paint over floating dialogs.
fn hardware_card_layer(ui: &egui::Ui, key: &str) -> egui::LayerId {
    crate::ui::dialog::workspace_overlay_layer(ui, ("hardware-channel-card", key))
}

fn draw_compact_control_card(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
) -> Option<HardwareChannelDrop> {
    let edit_id = ui.make_persistent_id(("control-name-editing", control.key.as_str()));
    let draft_id = ui.make_persistent_id(("control-name-draft", control.key.as_str()));
    let focus_pending_id = ui.make_persistent_id(("control-name-focus", control.key.as_str()));
    let text_edit_id = ui.make_persistent_id(("control-name-input", control.key.as_str()));
    let group_edit_id = ui.make_persistent_id(("control-group-editing", control.key.as_str()));
    let group_draft_id = ui.make_persistent_id(("control-group-draft", control.key.as_str()));
    let relay_id = relay_id_from_control_key(&control.key);
    let pwm_id = pwm_channel_for(capabilities, control).map(|channel| channel.id);
    let indicator_intensity = pwm_control_intensity(ui, capabilities, control);
    let indicator_state = indicator_intensity.map_or_else(
        || control_indicator_state(capabilities, control),
        |intensity| {
            if intensity > 0.0 {
                ControlIndicatorState::Active
            } else {
                ControlIndicatorState::Inactive
            }
        },
    );
    let card_outer_width = ui.available_width();
    let card_content_width = hardware_frame_content_width(card_outer_width, 9);
    ui.set_width(card_outer_width);
    let layer_id = hardware_card_layer(ui, &control.key);
    let dragging_source = hardware_channel_is_dragging(ui, &control.key, HardwareChannelDragSurface::Monitor);
    let card = ui.scope_builder(egui::UiBuilder::new().layer_id(layer_id), |ui| {
        if dragging_source {
            ui.set_opacity(0.58);
        }
        egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(9, 6))
        .stroke(egui::Stroke::new(
            HARDWARE_CARD_STROKE_WIDTH,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(7.0)
        .show(ui, |ui| {
            configure_hardware_card_controls(ui);
            ui.set_width(card_content_width);
            ui.horizontal(|ui| {
                draw_control_icon_picker(app, ui, capabilities, control, 17.0);
                let indicator = draw_control_indicator(
                    app,
                    ui,
                    indicator_state,
                    relay_id.is_some() && !control.locked,
                    active_control_indicator_color(capabilities, control),
                    indicator_intensity,
                );
                if hardware_control_activated(app, ui, &indicator)
                    && !control.locked
                    && let Some(id) = relay_id
                {
                    let turn_on = indicator_state != ControlIndicatorState::Active;
                    crate::ui::hardware_control::set_relay(app, id, turn_on);
                }
                if app.prefix_relay_identifiers && let Some(id) = relay_id {
                    ui.label(
                        egui::RichText::new(relay_identifier_label(id))
                            .monospace()
                            .weak(),
                    );
                }
                if let Some(id) = pwm_id {
                    ui.label(egui::RichText::new(format!("P{}", id + 1)).monospace().weak());
                }
                if control.locked {
                    ui.label(egui::RichText::new(crate::ui::icons::LOCK).weak())
                        .on_hover_text(app.tr("Channel is locked in PCController"));
                }
				hardware_channel_drag_handle(app, ui, control, HardwareChannelDragSurface::Monitor);

                let editing = ui.data_mut(|data| data.get_temp::<bool>(edit_id).unwrap_or(false));
                let editing_group = ui
                    .data_mut(|data| data.get_temp::<bool>(group_edit_id).unwrap_or(false));
                if editing {
                    let mut draft = ui.data_mut(|data| {
                        data.get_temp::<String>(draft_id)
                            .unwrap_or_else(|| control.name.clone())
                    });
                    let edit_align = crate::ui::i18n::input_alignment(app.rtl, &draft);
                    let edit_width = (ui.available_width() * 0.42).clamp(64.0, 190.0);
                    let edit = ui.add_sized(
                        [edit_width, 24.0],
                        crate::ui::dialog::singleline_text_edit(&mut draft)
                            .id(text_edit_id)
                            .horizontal_align(edit_align)
                            .hint_text(&control.default_name),
                    );
                    if ui.data_mut(|data| data.remove_temp::<bool>(focus_pending_id))
                        == Some(true)
                    {
                        edit.request_focus();
                    }
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
                    let changed = crate::ui::group_picker::group_picker(
                        ui,
                        group_draft_id,
                        &mut draft,
                        capabilities.controls.iter().map(|item| item.group.as_str()),
                        edit_width,
                        app.language,
                    );
                    if changed {
                        ui.data_mut(|data| data.insert_temp(group_draft_id, draft.clone()));
                    }
                    if ui.button(crate::ui::icons::FLOPPY_DISK).clicked() {
                        update_control_group(app, capabilities, control, draft);
                        ui.data_mut(|data| data.insert_temp(group_edit_id, false));
                    }
                    if ui.button(crate::ui::icons::X).clicked() {
                        ui.data_mut(|data| data.insert_temp(group_edit_id, false));
                    }
                } else {
                    let title = crate::ui::i18n::visual_text(app.language, &control.name);
                    let is_motion = is_motion_control(control);
                    let stop_visible = contextual_stop_action(capabilities, control).is_some();
                    let reserved = if is_pwm_control(control) {
                        150.0
                    } else if !control.actions.is_empty() {
                        (card_control_actions(control).len().min(3) as f32 * 34.0)
                            + if is_motion { 34.0 } else { 0.0 }
                            + if stop_visible { 34.0 } else { 0.0 }
                            + 8.0
                    } else if relay_id.is_some() {
                        76.0
                    } else {
                        8.0
                    };
                    let response = hardware_header_widget(ui, &control.key, "caption", |ui| {
                        left_aligned_click_label(ui, &title, (ui.available_width() - reserved).max(48.0), 24.0, 13.0)
                    });
                    if response.clicked() {
                        ui.data_mut(|data| {
                            data.insert_temp(draft_id, control.name.clone());
                            data.insert_temp(edit_id, true);
                            data.insert_temp(focus_pending_id, true);
                        });
                    }
                    response.on_hover_text(format!("{} — {}", title, app.tr("Rename")));
                    if is_motion {
                        if let Some(stop) = contextual_stop_action(capabilities, control) {
                            let response = hardware_header_widget(ui, &control.key, "stop", |ui| ui
                                .add_enabled(
                                    !app.estop_active && !control.locked,
                                    egui::Button::new(crate::ui::icons::action(&stop.verb))
                                        .min_size(egui::vec2(28.0, 26.0)),
                                )
                                .on_hover_text(crate::ui::i18n::visual_text(
                                    app.language,
                                    &stop.name,
                                )));
                            if hardware_control_activated(app, ui, &response) {
                                crate::ui::hardware_control::invoke_action(app, control, stop);
                            }
                        }
                        if hardware_header_widget(ui, &control.key, "rename", |ui| ui
                            .button(crate::ui::icons::PENCIL_SIMPLE)
                            .on_hover_text(app.tr("Rename")))
                            .clicked()
                        {
                            ui.data_mut(|data| {
                                data.insert_temp(draft_id, control.name.clone());
                                data.insert_temp(edit_id, true);
                                data.insert_temp(focus_pending_id, true);
                            });
                        }
                    }
                }

                if is_pwm_control(control) {
                    if let Some(channel) = capabilities
                        .pwm_channels
                        .iter()
                        .find(|channel| channel.key == control.key)
                    {
                        draw_pwm_card_editor(
                            app,
                            ui,
                            capabilities,
                            control,
                            channel,
                            ui.available_width(),
                        );
                    }
                } else if !control.actions.is_empty() {
                    let is_motion = is_motion_control(control);
                    let stop = control
                        .actions
                        .iter()
                        .find(|action| action.verb.eq_ignore_ascii_case("stop"));
                    let ordered = card_control_actions(control);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for action in ordered.into_iter().rev() {
                            let response = ui
                                .add_enabled(
                                    !app.estop_active && !control.locked,
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
                                update_held_motion_action(
                                    app,
                                    ui,
                                    &response,
                                    control,
                                    action,
                                    stop.expect("checked above"),
                                );
                            } else if ((is_motion
                                || matches!(
                                    action.verb.to_ascii_lowercase().as_str(),
                                    "on" | "off"
                                )) && hardware_control_activated(app, ui, &response))
                                || (!is_motion
                                    && !matches!(
                                        action.verb.to_ascii_lowercase().as_str(),
                                        "on" | "off"
                                    )
                                    && response.clicked())
                            {
                                crate::ui::hardware_control::invoke_action(app, control, action);
                            }
                        }
                    });
                } else if let Some(id) = relay_id {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for (state, icon, label) in [
                            (false, crate::ui::icons::STOP_CIRCLE, app.tr("OFF")),
                            (true, crate::ui::icons::LIGHTNING, app.tr("ON")),
                        ] {
                            let response = ui
                                .add_enabled(
                                    !app.estop_active && !control.locked,
                                    egui::Button::new(icon).min_size(egui::vec2(28.0, 26.0)),
                                )
                                .on_hover_text(label);
                            if hardware_control_activated(app, ui, &response) {
                                crate::ui::hardware_control::set_relay(app, id, state);
                            }
                        }
                    });
                }
            });
        })
    }).inner;

    finish_hardware_channel_card(ui, control, card.response.rect, layer_id, HardwareChannelDragSurface::Monitor);
    let drop = hardware_channel_drop_target(ui, card.response.rect, control, HardwareChannelDragSurface::Monitor);
    control_context_popup(
        app,
        ui,
        &card.response,
        capabilities,
        control,
        edit_id,
        draft_id,
        focus_pending_id,
        group_edit_id,
        group_draft_id,
    );
    drop
}

fn draw_control_card(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    control: &crate::four_d::controller::HardwareControl,
) -> Option<HardwareChannelDrop> {
    if app.compact_hardware_controls {
        return draw_compact_control_card(app, ui, capabilities, control);
    }
    let edit_id = ui.make_persistent_id(("control-name-editing", control.key.as_str()));
    let draft_id = ui.make_persistent_id(("control-name-draft", control.key.as_str()));
    let focus_pending_id = ui.make_persistent_id(("control-name-focus", control.key.as_str()));
    let text_edit_id = ui.make_persistent_id(("control-name-input", control.key.as_str()));
    let source_id = ui.make_persistent_id(("control-name-source", control.key.as_str()));
    let group_edit_id = ui.make_persistent_id(("control-group-editing", control.key.as_str()));
    let group_draft_id = ui.make_persistent_id(("control-group-draft", control.key.as_str()));
    let relay_id = relay_id_from_control_key(&control.key);
    let pwm_id = pwm_channel_for(capabilities, control).map(|channel| channel.id);
    let indicator_intensity = pwm_control_intensity(ui, capabilities, control);
    let indicator_state = indicator_intensity.map_or_else(
        || control_indicator_state(capabilities, control),
        |intensity| {
            if intensity > 0.0 {
                ControlIndicatorState::Active
            } else {
                ControlIndicatorState::Inactive
            }
        },
    );
    let card_outer_width = ui.available_width();
    let card_content_width = hardware_frame_content_width(card_outer_width, 12);
    ui.set_width(card_outer_width);
    let layer_id = hardware_card_layer(ui, &control.key);
    let dragging_source = hardware_channel_is_dragging(ui, &control.key, HardwareChannelDragSurface::Monitor);
    let card = ui
        .scope_builder(egui::UiBuilder::new().layer_id(layer_id), |ui| {
            if dragging_source {
                ui.set_opacity(0.58);
            }
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(12, 10))
                .stroke(egui::Stroke::new(
                    HARDWARE_CARD_STROKE_WIDTH,
                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                ))
                .corner_radius(8.0)
                .show(ui, |ui| {
                    configure_hardware_card_controls(ui);
                    ui.set_width(card_content_width);
                    let mut editing =
                        ui.data_mut(|data| data.get_temp::<bool>(edit_id).unwrap_or(false));
                    ui.horizontal(|ui| {
                        draw_control_icon_picker(app, ui, capabilities, control, 18.0);
                        let indicator = draw_control_indicator(
                            app,
                            ui,
                            indicator_state,
                            relay_id.is_some() && !control.locked,
                            active_control_indicator_color(capabilities, control),
                            indicator_intensity,
                        );
                        if hardware_control_activated(app, ui, &indicator)
                            && !control.locked
                            && let Some(id) = relay_id
                        {
                            let turn_on = indicator_state != ControlIndicatorState::Active;
                            crate::ui::hardware_control::set_relay(app, id, turn_on);
                        }
                        if app.prefix_relay_identifiers
                            && let Some(id) = relay_id
                        {
                            ui.label(
                                egui::RichText::new(relay_identifier_label(id))
                                    .monospace()
                                    .weak(),
                            );
                        }
                        if let Some(id) = pwm_id {
                            ui.label(
                                egui::RichText::new(format!("P{}", id + 1))
                                    .monospace()
                                    .weak(),
                            );
                        }
                        if control.locked {
                            ui.label(egui::RichText::new(crate::ui::icons::LOCK).weak())
                                .on_hover_text(app.tr("Channel is locked in PCController"));
                        }
                        hardware_channel_drag_handle(app, ui, control, HardwareChannelDragSurface::Monitor);

                        if editing {
                            let mut draft = ui.data_mut(|data| {
                                let source = data.get_temp::<String>(source_id);
                                if source.as_deref() != Some(control.name.as_str()) {
                                    data.insert_temp(source_id, control.name.clone());
                                    data.insert_temp(draft_id, control.name.clone());
                                }
                                data.get_temp::<String>(draft_id)
                                    .unwrap_or_else(|| control.name.clone())
                            });
                            let edit_align = crate::ui::i18n::input_alignment(app.rtl, &draft);
                            let mut edit_response = None;
                            let mut save_clicked = false;
                            let mut cancel_clicked = false;
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    cancel_clicked = ui
                                        .button(crate::ui::icons::X)
                                        .on_hover_text(app.tr("Cancel"))
                                        .clicked();
                                    save_clicked = ui
                                        .button(crate::ui::icons::FLOPPY_DISK)
                                        .on_hover_text(app.tr("Save name"))
                                        .clicked();
                                    let width = ui.available_width().max(56.0);
                                    edit_response = Some(
                                        ui.add_sized(
                                            [width, 24.0],
                                            crate::ui::dialog::singleline_text_edit(&mut draft)
                                                .id(text_edit_id)
                                                .horizontal_align(edit_align)
                                                .hint_text(&control.default_name),
                                        ),
                                    );
                                },
                            );
                            let edit = edit_response.expect("rename editor is always rendered");
                            if ui.data_mut(|data| data.remove_temp::<bool>(focus_pending_id))
                                == Some(true)
                            {
                                edit.request_focus();
                            }
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
                            let title_text =
                                crate::ui::i18n::visual_text(app.language, &control.name);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if hardware_header_widget(ui, &control.key, "rename", |ui| {
                                        ui.button(crate::ui::icons::PENCIL_SIMPLE)
                                            .on_hover_text(app.tr("Rename"))
                                    })
                                    .clicked()
                                    {
                                        ui.data_mut(|data| {
                                            data.insert_temp(draft_id, control.name.clone());
                                            data.insert_temp(edit_id, true);
                                            data.insert_temp(focus_pending_id, true);
                                        });
                                    }
                                    if let Some(stop) =
                                        contextual_stop_action(capabilities, control)
                                    {
                                        let response = hardware_header_widget(
                                            ui,
                                            &control.key,
                                            "stop",
                                            |ui| {
                                                ui.add_enabled(
                                                    !app.estop_active && !control.locked,
                                                    egui::Button::new(crate::ui::icons::action(
                                                        &stop.verb,
                                                    )),
                                                )
                                                .on_hover_text(crate::ui::i18n::visual_text(
                                                    app.language,
                                                    &stop.name,
                                                ))
                                            },
                                        );
                                        if hardware_control_activated(app, ui, &response) {
                                            crate::ui::hardware_control::invoke_action(
                                                app, control, stop,
                                            );
                                        }
                                    }
                                    let title =
                                        hardware_header_widget(ui, &control.key, "caption", |ui| {
                                            left_aligned_click_label(
                                                ui,
                                                &title_text,
                                                ui.available_width().max(52.0),
                                                24.0,
                                                13.0,
                                            )
                                        });
                                    if title.clicked() {
                                        ui.data_mut(|data| {
                                            data.insert_temp(draft_id, control.name.clone());
                                            data.insert_temp(edit_id, true);
                                            data.insert_temp(focus_pending_id, true);
                                        });
                                    }
                                    title.on_hover_text(format!(
                                        "{} — {}",
                                        title_text,
                                        app.tr("Rename")
                                    ));
                                },
                            );
                        }
                    });

                    let mut editing_group =
                        ui.data_mut(|data| data.get_temp::<bool>(group_edit_id).unwrap_or(false));
                    if editing_group {
                        let mut draft = ui.data_mut(|data| {
                            data.get_temp::<String>(group_draft_id)
                                .unwrap_or_else(|| control.group.clone())
                        });
                        ui.horizontal(|ui| {
                            ui.label(app.tr("Group"));
                            let changed = crate::ui::group_picker::group_picker(
                                ui,
                                group_draft_id,
                                &mut draft,
                                capabilities.controls.iter().map(|item| item.group.as_str()),
                                ui.available_width().max(80.0) - 52.0,
                                app.language,
                            );
                            if changed {
                                ui.data_mut(|data| data.insert_temp(group_draft_id, draft.clone()));
                            }
                            if ui.button(crate::ui::icons::FLOPPY_DISK).clicked() {
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
                        if let Some(channel) = capabilities
                            .pwm_channels
                            .iter()
                            .find(|channel| channel.key == control.key)
                        {
                            ui.add_space(if app.compact_hardware_controls {
                                3.0
                            } else {
                                8.0
                            });
                            draw_pwm_card_editor(
                                app,
                                ui,
                                capabilities,
                                control,
                                channel,
                                card_content_width,
                            );
                        }
                    }

                    if !control.actions.is_empty() {
                        ui.add_space(if app.compact_hardware_controls {
                            3.0
                        } else {
                            8.0
                        });
                        let is_motion = is_motion_control(control);
                        let stop_action = control
                            .actions
                            .iter()
                            .find(|action| action.verb.eq_ignore_ascii_case("stop"));
                        let actions = card_control_actions(control);
                        let action_rows = vec![actions];
                        for action_row in action_rows {
                            let columns =
                                action_grid_columns(ui.available_width(), action_row.len());
                            for row in action_row.chunks(columns) {
                                ui.columns(columns, |uis| {
                                    for (index, action) in row.iter().enumerate() {
                                        let ui = &mut uis[index];
                                        let visual_name = crate::ui::i18n::visual_text(
                                            app.language,
                                            &action.name,
                                        );
                                        let label =
                                            responsive_action_label(action, ui.available_width());
                                        let verb = action.verb.to_ascii_lowercase();
                                        let selected = relay_id.is_some()
                                            && ((verb == "on"
                                                && indicator_state
                                                    == ControlIndicatorState::Active)
                                                || (verb == "off"
                                                    && indicator_state
                                                        == ControlIndicatorState::Inactive));
                                        let mut button =
                                            egui::Button::new(label).truncate().selected(selected);
                                        if selected && verb == "on" {
                                            button = button
                                                .fill(egui::Color32::from_rgb(22, 163, 74))
                                                .stroke(egui::Stroke::new(
                                                    1.0_f32,
                                                    egui::Color32::from_rgb(34, 197, 94),
                                                ));
                                        }
                                        let response = ui
                                            .add_enabled_ui(
                                                !app.estop_active && !control.locked,
                                                |ui| {
                                                    ui.add_sized(
                                                        [ui.available_width(), 28.0],
                                                        button,
                                                    )
                                                },
                                            )
                                            .inner
                                            .on_hover_text(visual_name);
                                        if is_motion
                                            && app.motion_control_mode
                                                == crate::config::MotionControlMode::Hold
                                            && stop_action.is_some()
                                        {
                                            update_held_motion_action(
                                                app,
                                                ui,
                                                &response,
                                                control,
                                                action,
                                                stop_action.expect("checked above"),
                                            );
                                        } else if ((is_motion
                                            || matches!(verb.as_str(), "on" | "off"))
                                            && hardware_control_activated(app, ui, &response))
                                            || (!is_motion
                                                && !matches!(verb.as_str(), "on" | "off")
                                                && response.clicked())
                                        {
                                            crate::ui::hardware_control::invoke_action(
                                                app, control, action,
                                            );
                                        }
                                    }
                                });
                            }
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
                                let selected =
                                    capabilities.active_relays.contains(&relay_id) == state;
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
                                let response = uis[index]
                                    .add_enabled_ui(!app.estop_active && !control.locked, |ui| {
                                        ui.add_sized([ui.available_width(), 28.0], button)
                                    })
                                    .inner;
                                if hardware_control_activated(app, &uis[index], &response) {
                                    crate::ui::hardware_control::set_relay(
                                        app, relay_id, state,
                                    );
                                }
                            }
                        });
                    }
                })
        })
        .inner;

    finish_hardware_channel_card(ui, control, card.response.rect, layer_id, HardwareChannelDragSurface::Monitor);
    let drop = hardware_channel_drop_target(ui, card.response.rect, control, HardwareChannelDragSurface::Monitor);
    control_context_popup(
        app,
        ui,
        &card.response,
        capabilities,
        control,
        edit_id,
        draft_id,
        focus_pending_id,
        group_edit_id,
        group_draft_id,
    );
    drop
}

fn draw_compact_relay_group(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    group: &str,
    controls: &[&crate::four_d::controller::HardwareControl],
) -> Option<HardwareChannelDrop> {
    let mut pending_drop = None;
    let outer_width = ui.available_width();
    ui.set_width(outer_width);
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(9, 6))
        .stroke(egui::Stroke::new(
            HARDWARE_CARD_STROKE_WIDTH,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(7.0)
        .show(ui, |ui| {
            configure_hardware_card_controls(ui);
            ui.set_width(hardware_frame_content_width(outer_width, 9));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(crate::ui::icons::PLUG).size(17.0));
                let title = if group.trim().is_empty() {
                    app.tr("Relay outputs")
                } else {
                    humanize_machine_label(group)
                };
                let controls_width = controls.len() as f32 * 64.0;
                ui.add_sized(
                    [(ui.available_width() - controls_width).max(52.0), 26.0],
                    egui::Label::new(egui::RichText::new(title).strong()).truncate(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for control in controls.iter().rev() {
                        let Some(relay_id) = relay_id_from_control_key(&control.key) else {
                            continue;
                        };
                        let active = capabilities.active_relays.contains(&relay_id);
                        let mut button = egui::Button::new(relay_id.to_string())
                            .min_size(egui::vec2(42.0, 26.0))
                            .selected(active);
                        if active {
                            button = button.fill(egui::Color32::from_rgb(22, 163, 74)).stroke(
                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(34, 197, 94)),
                            );
                        }
                        let handle = hardware_channel_drag_handle(app, ui, control, HardwareChannelDragSurface::Monitor);
                        let response = ui
                            .add_enabled(!app.estop_active && !control.locked, button)
                            .on_hover_text(format!(
                                "{} · {}{}",
                                crate::ui::i18n::visual_text(app.language, &control.name),
                                app.tr(if active {
                                    "Board reports ON"
                                } else {
                                    "Board reports OFF"
                                }),
                                if control.locked {
                                    format!(" · {}", app.tr("Channel is locked in PCController"))
                                } else {
                                    String::new()
                                }
                            ));
                        if let Some(drop) = hardware_channel_drop_target(
                            ui,
                            handle.rect.union(response.rect),
                            control,
                            HardwareChannelDragSurface::Monitor,
                        ) {
                            pending_drop = Some(drop);
                        }
                        if response.clicked() {
                            crate::ui::hardware_control::set_relay(app, relay_id, !active);
                        }
                        response.context_menu(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(crate::ui::icons::PLUG);
                                ui.strong(crate::ui::i18n::visual_text(
                                    app.language,
                                    &control.name,
                                ));
                            });
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} · {}",
                                    relay_identifier_label(relay_id),
                                    control.key
                                ))
                                .monospace()
                                .weak()
                                .small(),
                            );
                            ui.separator();
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::SLIDERS_HORIZONTAL,
                                    app.tr("Manage...")
                                ))
                                .clicked()
                            {
                                open_control_dialog(app, capabilities, control);
                                ui.close();
                            }
                            let mut locked = control.locked;
                            if ui
                                .checkbox(
                                    &mut locked,
                                    format!(
                                        "{} {}",
                                        crate::ui::icons::LOCK,
                                        app.tr("Lock channel")
                                    ),
                                )
                                .changed()
                            {
                                update_control_presentation_flags(
                                    app,
                                    capabilities,
                                    control,
                                    None,
                                    Some(locked),
                                );
                                ui.close();
                            }
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::EYE_SLASH,
                                    app.tr("Hide channel")
                                ))
                                .clicked()
                            {
                                update_control_presentation_flags(
                                    app,
                                    capabilities,
                                    control,
                                    Some(true),
                                    None,
                                );
                                ui.close();
                            }
                            if ui
                                .button(format!(
                                    "{} {}",
                                    if active {
                                        crate::ui::icons::STOP_CIRCLE
                                    } else {
                                        crate::ui::icons::LIGHTNING
                                    },
                                    app.tr(if active { "Turn off" } else { "Turn on" })
                                ))
                                .clicked()
                            {
                                crate::ui::hardware_control::set_relay(app, relay_id, !active);
                                ui.close();
                            }
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::PENCIL_SIMPLE,
                                    app.tr("Rename...")
                                ))
                                .clicked()
                            {
                                open_control_dialog(app, capabilities, control);
                                ui.close();
                            }
                        });
                    }
                });
            });
        });
    pending_drop
}

fn draw_control_card_grid(
    app: &mut PealayerApp,
    ui: &mut egui::Ui,
    capabilities: &crate::four_d::controller::HardwareCapabilities,
    controls: &[crate::four_d::controller::HardwareControl],
) {
    let mut pending_drop = None;
    let hidden = controls
        .iter()
        .filter(|control| control.hidden && control_supports_presentation_policy(control))
        .collect::<Vec<_>>();
    if !hidden.is_empty() {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.menu_button(
                format!(
                    "{} {} {}",
                    crate::ui::icons::EYE_SLASH,
                    hidden.len(),
                    app.tr("hidden")
                ),
                |ui| {
                    ui.label(egui::RichText::new(app.tr("Hidden channels")).strong());
                    ui.separator();
                    for control in &hidden {
                        if ui
                            .button(format!(
                                "{} {}",
                                crate::ui::icons::EYE,
                                crate::ui::i18n::visual_text(app.language, &control.name)
                            ))
                            .on_hover_text(app.tr("Show channel"))
                            .clicked()
                        {
                            update_control_presentation_flags(
                                app,
                                capabilities,
                                control,
                                Some(false),
                                None,
                            );
                            ui.close();
                        }
                    }
                },
            );
        });
        ui.add_space(4.0);
    }
    let mut ordered_controls = controls.iter().collect::<Vec<_>>();
    ordered_controls.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then_with(|| left.key.cmp(&right.key))
    });
    let mut groups: Vec<(String, Vec<&crate::four_d::controller::HardwareControl>)> = Vec::new();
    for control in ordered_controls
        .into_iter()
        .filter(|control| !control.hidden)
        .filter(|control| global_control_is_visible(app, capabilities, control))
    {
        let group = control.group.trim();
        if let Some((_, members)) = groups.iter_mut().find(|(name, _)| name == group) {
            members.push(control);
        } else {
            groups.push((group.to_string(), vec![control]));
        }
    }
    if groups.is_empty() {
        ui.label(egui::RichText::new(app.tr("All channels are hidden")).weak());
        return;
    }
    let show_group_headers = groups.iter().any(|(name, _)| !name.is_empty());
    for (group, members) in groups {
        if app.compact_hardware_controls
            && members.len() > 1
            && members
                .iter()
                .all(|control| relay_id_from_control_key(&control.key).is_some())
        {
            let dimmed = app.non_user_control_visibility
                == crate::config::NonUserControlVisibility::Dimmed
                && members
                    .iter()
                    .all(|control| is_non_user_control(capabilities, control));
            ui.scope(|ui| {
                if dimmed {
                    ui.set_opacity(0.58);
                }
                if let Some(drop) =
                    draw_compact_relay_group(app, ui, capabilities, &group, &members)
                {
                    pending_drop = Some(drop);
                }
            });
            ui.add_space(6.0);
            continue;
        }
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
                    let dimmed = app.non_user_control_visibility
                        == crate::config::NonUserControlVisibility::Dimmed
                        && is_non_user_control(capabilities, control);
                    uis[index].scope(|ui| {
                        if dimmed {
                            ui.set_opacity(0.58);
                        }
                        if let Some(drop) = draw_control_card(app, ui, capabilities, control) {
                            pending_drop = Some(drop);
                        }
                    });
                }
            });
            ui.add_space(8.0);
        }
        if show_group_headers && !group.is_empty() {
            ui.add_space(3.0);
        }
    }
    if let Some(drop) = pending_drop {
        persist_channel_drop(app, capabilities, drop);
    } else {
        clear_released_hardware_channel_drag(ui, HardwareChannelDragSurface::Monitor);
    }
}

#[cfg(test)]
mod timeline_row_tests {
    use super::*;

    #[test]
    fn timeline_track_state_semantics_have_distinct_colors_and_opacity() {
        assert_ne!(timeline_track_state_color(TimelineTrackStateKind::Muted), timeline_track_state_color(TimelineTrackStateKind::Soloed));
        assert_ne!(timeline_track_state_color(TimelineTrackStateKind::Soloed), timeline_track_state_color(TimelineTrackStateKind::Locked));
        assert_eq!(timeline_track_visual_opacity(true, false, false, false), 0.42);
        assert_eq!(timeline_track_visual_opacity(false, false, false, true), 0.36);
        assert_eq!(timeline_track_visual_opacity(false, true, false, true), 1.0);
        assert_eq!(timeline_track_visual_opacity(false, false, true, false), 0.72);
        assert_eq!(timeline_track_cue_alpha(true, false, false, false), 107);
        assert_eq!(timeline_track_cue_alpha(false, false, false, true), 92);
    }

    #[test]
    fn timeline_track_state_button_always_reserves_the_same_square() {
        let context = egui::Context::default();
        let mut size = egui::Vec2::ZERO;
        discard_ui_output(context.run_ui(egui::RawInput::default(), |ui| {
            size = timeline_track_state_button(
                ui,
                false,
                TimelineTrackStateKind::Muted,
                crate::ui::icons::PROHIBIT,
                "Mute",
            )
            .rect
            .size();
        }));
        assert_eq!(size, egui::vec2(TIMELINE_TRACK_STATE_BUTTON_SIZE, TIMELINE_TRACK_STATE_BUTTON_SIZE));
    }

    #[test]
    fn middle_button_timeline_pan_tracks_the_grab_offset_on_both_axes() {
        let offset = pan_timeline_offset(
            egui::vec2(120.0, 70.0),
            egui::vec2(-35.0, 20.0),
            egui::vec2(1_000.0, 600.0),
            egui::vec2(400.0, 300.0),
        );

        assert_eq!(offset, egui::vec2(155.0, 50.0));
    }

    #[test]
    fn timeline_marquee_direction_matches_cad_window_and_crossing_conventions() {
        let origin = egui::pos2(100.0, 50.0);

        assert_eq!(
            timeline_marquee_mode(origin, egui::pos2(180.0, 80.0)),
            TimelineMarqueeMode::Window
        );
        assert_eq!(
            timeline_marquee_mode(origin, egui::pos2(20.0, 80.0)),
            TimelineMarqueeMode::Crossing
        );
    }

    #[test]
    fn timeline_window_requires_full_enclosure_while_crossing_accepts_an_overlap() {
        let marquee = egui::Rect::from_min_max(egui::pos2(10.0, 10.0), egui::pos2(50.0, 50.0));
        let enclosed = egui::Rect::from_min_max(
            egui::pos2(20.0, 20.0),
            egui::pos2(40.0, 40.0),
        );
        let crossing = egui::Rect::from_min_max(
            egui::pos2(40.0, 20.0),
            egui::pos2(70.0, 40.0),
        );

        assert!(timeline_marquee_selects_rect(
            marquee,
            enclosed,
            TimelineMarqueeMode::Window
        ));
        assert!(!timeline_marquee_selects_rect(
            marquee,
            crossing,
            TimelineMarqueeMode::Window
        ));
        assert!(timeline_marquee_selects_rect(
            marquee,
            crossing,
            TimelineMarqueeMode::Crossing
        ));
    }

    #[test]
    fn controller_macro_cues_resolve_to_the_same_lane_used_for_rendering() {
        let app = PealayerApp::default();
        let lane = crate::four_d::models::ControllerEffectLane::Sequence;
        let rows = vec![TimelineTrackRow {
            key: crate::four_d::models::controller_effect_timeline_track_key(lane),
            name: "General sequences".to_string(),
            detail: Some("Effects".to_string()),
            active: true,
            enabled: true,
            linked: true,
            visible: true,
            icon: crate::ui::icons::WAVEFORM.to_string(),
            control_key: None,
            relay_ids: Vec::new(),
            dimmed: false,
            kind: TimelineTrackKind::ControllerEffect(lane),
        }];
        let effect = crate::four_d::models::Effect::controller_macro(
            "Motion".to_string(),
            crate::ui::icons::SEAT.to_string(),
            1_000,
            7,
            "mcu".to_string(),
        );

        let placement = timeline_cue_placement(
            &app,
            &effect,
            &rows,
            &std::collections::BTreeSet::new(),
        )
        .expect("controller cue should resolve to its visible timeline lane");
        assert_eq!(placement.track_index, 0);
        assert_eq!(placement.analog_index, None);
        assert_eq!(placement.relay_id, None);
    }

    #[test]
    fn middle_button_timeline_pan_clamps_to_scrollable_content() {
        assert_eq!(
            pan_timeline_offset(
                egui::vec2(5.0, 10.0),
                egui::vec2(100.0, 100.0),
                egui::vec2(1_000.0, 600.0),
                egui::vec2(400.0, 300.0),
            ),
            egui::Vec2::ZERO
        );
        assert_eq!(
            pan_timeline_offset(
                egui::vec2(590.0, 290.0),
                egui::vec2(-100.0, -100.0),
                egui::vec2(1_000.0, 600.0),
                egui::vec2(400.0, 300.0),
            ),
            egui::vec2(600.0, 300.0)
        );
    }

    #[test]
    fn timeline_zoom_keeps_the_media_time_under_the_pointer() {
        let next_offset = timeline_offset_for_pointer_zoom(200.0, 300.0, 100.0, 200.0, 20.0, 800.0);

        assert_eq!(next_offset, 700.0);
        let old_time = (200.0 + 300.0) / 100.0;
        let new_time = (next_offset + 300.0) / 200.0;
        assert_eq!(old_time, new_time);
    }

    #[test]
    fn timeline_wheel_modifiers_route_zoom_and_horizontal_scroll() {
        use crate::config::TimelineWheelBehavior as Wheel;
        let delta = egui::vec2(0.0, 120.0);
        assert_eq!(timeline_wheel_action(delta, Wheel::Zoom), Some(TimelineWheelAction::Zoom(120.0)));
        assert_eq!(timeline_wheel_action(delta, Wheel::VerticalScroll), Some(TimelineWheelAction::VerticalScroll(120.0)));
        assert_eq!(timeline_wheel_action(delta, Wheel::HorizontalScroll), Some(TimelineWheelAction::HorizontalScroll(120.0)));
        assert_eq!(timeline_wheel_action(delta, Wheel::None), None);
        assert_eq!(timeline_wheel_action(egui::Vec2::ZERO, Wheel::Zoom), None);
        for behavior in [Wheel::Zoom, Wheel::VerticalScroll, Wheel::HorizontalScroll, Wheel::None] {
            for delta in [egui::vec2(-45.0, 0.0), egui::vec2(-45.0, 120.0)] {
                assert_eq!(timeline_wheel_action(delta, behavior), Some(TimelineWheelAction::HorizontalScroll(-45.0)));
            }
        }
    }

    #[test]
    fn timeline_wheel_reaches_empty_surface_and_consumes_ctrl_zoom_input() {
        use crate::config::TimelineWheelBehavior as Wheel;
        for (modifiers, delta, behavior, expected) in [
            (egui::Modifiers::CTRL, egui::vec2(0.0, -80.0), Wheel::VerticalScroll, Some(TimelineWheelAction::VerticalScroll(-80.0))),
            (egui::Modifiers::SHIFT, egui::vec2(-50.0, 0.0), Wheel::Zoom, Some(TimelineWheelAction::HorizontalScroll(-50.0))),
            (egui::Modifiers::ALT, egui::vec2(0.0, -40.0), Wheel::HorizontalScroll, Some(TimelineWheelAction::HorizontalScroll(-40.0))),
            (egui::Modifiers::NONE, egui::vec2(0.0, -40.0), Wheel::None, None),
        ] {
            let context = egui::Context::default();
            // Warm up hit testing, then send an actual egui raw wheel event over
            // empty space well below a small child widget (the previous exclusion).
            for frame in 0..2 {
                let output = context.run_ui(egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 300.0))),
                    events: if frame == 0 { vec![] } else { vec![
                        egui::Event::PointerMoved(egui::pos2(240.0, 230.0)),
                        egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta,
                            modifiers, phase: egui::TouchPhase::Move },
                    ] },
                    ..Default::default()
                }, |ui| {
                    let surface = ui.clip_rect();
                    let (_, child) = ui.allocate_space(egui::vec2(40.0, 20.0));
                    if frame == 1 {
                        assert!(!child.contains(egui::pos2(240.0, 230.0)));
                        assert_eq!(ui.input(timeline_wheel_modifiers), modifiers);
                        assert_eq!(timeline_wheel_over_surface(ui, surface, behavior).map(|(action, _)| action), expected);
                        assert_eq!(ui.input(|input| input.smooth_scroll_delta), egui::Vec2::ZERO);
                    }
                });
                discard_ui_output(output);
            }
        }
    }

    #[test]
    fn timeline_wheel_uses_original_axes_and_native_units_for_all_modifiers() {
        let wheel = |unit, delta, modifiers| egui::Event::MouseWheel {
            unit, delta, modifiers, phase: egui::TouchPhase::Move,
        };
        let events = [
            wheel(egui::MouseWheelUnit::Line, egui::vec2(0.0, -3.0), egui::Modifiers::CTRL),
            wheel(egui::MouseWheelUnit::Point, egui::vec2(0.0, -12.0), egui::Modifiers::COMMAND),
            wheel(egui::MouseWheelUnit::Point, egui::vec2(0.0, -100.0), egui::Modifiers::NONE),
        ];
        assert_eq!(timeline_wheel_delta(&events, 20.0, 200.0), egui::vec2(0.0, -172.0));
        assert_eq!(timeline_wheel_delta(&[wheel(egui::MouseWheelUnit::Page, egui::vec2(0.0, -1.0), egui::Modifiers::CTRL)], 20.0, 200.0), egui::vec2(0.0, -200.0));
        assert_eq!(timeline_wheel_delta(&[wheel(egui::MouseWheelUnit::Line, egui::vec2(2.0, 0.0), egui::Modifiers::ALT)], 20.0, 200.0), egui::vec2(40.0, 0.0));
    }

    #[test]
    fn timeline_ruler_scroll_steps_converts_vertical_wheel_to_frame_steps() {
        let wheel = |unit, delta| egui::Event::MouseWheel {
            unit,
            delta,
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        };
        // Scrolling upwards (positive y) advances forward (+1 frame per notch)
        assert_eq!(
            timeline_ruler_scroll_steps_from_events(&[wheel(egui::MouseWheelUnit::Line, egui::vec2(0.0, 1.0))]),
            1
        );
        // High magnitude line delta (e.g. Windows multi-line notch 3.0 or 120.0) still produces exactly 1 frame step
        assert_eq!(
            timeline_ruler_scroll_steps_from_events(&[wheel(egui::MouseWheelUnit::Line, egui::vec2(0.0, 3.0))]),
            1
        );
        assert_eq!(
            timeline_ruler_scroll_steps_from_events(&[wheel(egui::MouseWheelUnit::Line, egui::vec2(0.0, 120.0))]),
            1
        );
        // Scrolling downwards (negative y) steps backward (-1 frame per notch)
        assert_eq!(
            timeline_ruler_scroll_steps_from_events(&[wheel(egui::MouseWheelUnit::Line, egui::vec2(0.0, -1.0))]),
            -1
        );
        assert_eq!(
            timeline_ruler_scroll_steps_from_events(&[wheel(egui::MouseWheelUnit::Line, egui::vec2(0.0, -3.0))]),
            -1
        );
        // Multiple separate notch events in a single frame accumulate
        assert_eq!(
            timeline_ruler_scroll_steps_from_events(&[
                wheel(egui::MouseWheelUnit::Line, egui::vec2(0.0, 1.0)),
                wheel(egui::MouseWheelUnit::Line, egui::vec2(0.0, 1.0)),
                wheel(egui::MouseWheelUnit::Line, egui::vec2(0.0, 1.0)),
            ]),
            3
        );
        // Point unit (trackpad / smooth wheel) resolves directionally
        assert_eq!(
            timeline_ruler_scroll_steps_from_events(&[wheel(egui::MouseWheelUnit::Point, egui::vec2(0.0, 15.0))]),
            1
        );
        assert_eq!(
            timeline_ruler_scroll_steps_from_events(&[wheel(egui::MouseWheelUnit::Point, egui::vec2(0.0, -25.0))]),
            -1
        );
        // Sub-pixel jitter under 1.0 point is ignored
        assert_eq!(
            timeline_ruler_scroll_steps_from_events(&[wheel(egui::MouseWheelUnit::Point, egui::vec2(0.0, 0.4))]),
            0
        );
        // Empty events produce 0 steps
        assert_eq!(timeline_ruler_scroll_steps_from_events(&[]), 0);
    }

    #[test]
    fn timeline_combines_native_precision_trackpad_pinch_factors() {
        assert_eq!(timeline_pinch_factor(&[]), None);
        assert_eq!(timeline_pinch_factor(&[
            egui::Event::Zoom(1.1), egui::Event::Zoom(1.2)]), Some(1.32));
        assert_eq!(timeline_pinch_factor(&[egui::Event::Zoom(f32::NAN)]), None);
        assert_eq!(timeline_pinch_factor(&[egui::Event::Zoom(0.0)]), None);
    }

    #[test]
    fn timeline_ruler_is_frozen_while_tracks_and_horizontal_origin_scroll() {
        for offset in [egui::Vec2::ZERO, egui::vec2(140.0, 90.0), egui::vec2(400.0, 600.0)] {
            let content = egui::Rect::from_min_size(egui::pos2(300.0, 80.0) - offset, egui::vec2(1800.0, 2000.0));
            let viewport = egui::Rect::from_min_size(offset.to_pos2(), egui::vec2(500.0, 260.0));
            let ruler = timeline_frozen_ruler_rect(content, viewport);
            assert_eq!(ruler.top(), 80.0);
            assert_eq!(ruler.bottom(), 80.0 + TIMELINE_RULER_HEIGHT);
            assert_eq!(ruler.left(), 300.0 - offset.x);
            assert_eq!(timeline_track_row_top(content.top(), 0, 40.0), 80.0 + TIMELINE_RULER_HEIGHT - offset.y);
            assert!(ruler.contains(timeline_keyframe_marker_center(ruler, 350.0)));
        }
    }

    #[test]
    fn timeline_frozen_ruler_uses_real_scroll_viewport_and_separate_paint_clip() {
        let context = egui::Context::default();
        let mut observed = Vec::new();
        for offset in [0.0, 120.0, 420.0] {
            let output = context.run_ui(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 240.0))),
                ..Default::default()
            }, |ui| {
                let scroll = egui::ScrollArea::both()
                    .id_salt("test-frozen-timeline")
                    .content_margin(egui::Margin::ZERO)
                    .vertical_scroll_offset(offset)
                    .show_viewport(ui, |ui, viewport| {
                        let (_, content) = ui.allocate_space(egui::vec2(1400.0, 1600.0));
                        let ruler = timeline_frozen_ruler_rect(content, viewport);
                        let clip = ui.clip_rect();
                        let body_clip = egui::Rect::from_min_max(egui::pos2(clip.left(), ruler.bottom()), clip.max).intersect(clip);
                        let body_painter = ui.painter().with_clip_rect(body_clip);
                        let ruler_painter = ui.painter().with_clip_rect(ruler.intersect(clip));
                        assert!(ruler_painter.clip_rect().height() > 0.0);
                        assert!(body_painter.clip_rect().top() >= ruler.bottom());
                        observed.push((ruler.top(), timeline_track_row_top(content.top(), 0, 40.0)));
                    });
                assert_eq!(scroll.state.offset.y, offset);
            });
            discard_ui_output(output);
        }
        assert_eq!(observed[0].0, observed[1].0);
        assert_eq!(observed[1].0, observed[2].0);
        assert_eq!(observed[0].1 - observed[1].1, 120.0);
        assert_eq!(observed[0].1 - observed[2].1, 420.0);
    }

    #[test]
    fn middle_pan_axis_modifiers_preserve_free_and_constrained_modes() {
        let delta = egui::vec2(15.0, -9.0);
        assert_eq!(constrain_middle_pan_delta(delta, false, false, true), delta);
        assert_eq!(
            constrain_middle_pan_delta(delta, true, false, true),
            egui::vec2(15.0, 0.0)
        );
        assert_eq!(
            constrain_middle_pan_delta(delta, false, true, true),
            egui::vec2(0.0, -9.0)
        );
        assert_eq!(constrain_middle_pan_delta(delta, true, false, false), delta);
    }

    #[test]
    fn bring_playhead_into_view_moves_only_when_outside_the_viewport() {
        assert_eq!(
            timeline_offset_to_reveal_x(200.0, 350.0, 1_000.0, 400.0),
            200.0
        );
        assert_eq!(
            timeline_offset_to_reveal_x(200.0, 800.0, 1_000.0, 400.0),
            424.0
        );
        assert_eq!(
            timeline_offset_to_reveal_x(400.0, 50.0, 1_000.0, 400.0),
            26.0
        );
    }

    #[test]
    fn playhead_follow_waits_near_the_edge_and_preserves_look_ahead() {
        assert_eq!(
            timeline_follow_target_offset(200.0, 450.0, 2_000.0, 500.0),
            None
        );
        assert_eq!(
            timeline_follow_target_offset(200.0, 620.0, 2_000.0, 500.0),
            Some(310.0)
        );
        assert_eq!(
            timeline_follow_target_offset(700.0, 500.0, 2_000.0, 500.0),
            Some(400.0)
        );
        assert_eq!(
            timeline_follow_target_offset(0.0, 450.0, 400.0, 500.0),
            None
        );
    }

    #[test]
    fn timeline_toolbar_stays_under_add_cue_backdrop_and_cannot_be_clicked_through() {
        for dark in [false, true] {
            let context = egui::Context::default();
            context.set_visuals(if dark { egui::Visuals::dark() } else { egui::Visuals::light() });
            let mut app = PealayerApp::default();
            let toolbar_rect = egui::Rect::from_min_size(egui::pos2(750.0, 12.0), egui::vec2(180.0, 28.0));
            context.data_mut(|data| data.insert_temp(timeline_cue_dialog_id(), TimelineCueDraft {
                track_name: "Relay 5".into(), control_key: "relay.5".into(),
                start_time_ms: 1000, duration_ms: 1000,
                behavior: crate::four_d::models::DirectCueBehavior::SetKeep, end_value_basis_points: 0,
                action: TimelineCueDraftAction::Relay { enabled: true }, error: None,
            }));
            let mut clicked = false;
            // Area fades its painter on opening. Advance a deterministic clock
            // beyond that animation before asserting the final backdrop color.
            let frame_time = std::cell::Cell::new(0.0);
            let mut render = |events| context.run_ui(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 800.0))),
                time: Some({
                    let time = frame_time.get();
                    frame_time.set(time + 1.0);
                    time
                }),
                events, ..Default::default()
            }, |ui| {
                let layer = crate::ui::dialog::workspace_overlay_layer(ui, "timeline-ruler-toolbar-layer");
                assert_eq!(layer.order, ui.layer_id().order);
                let mut toolbar = ui.new_child(egui::UiBuilder::new().layer_id(layer).max_rect(toolbar_rect));
                clicked |= toolbar.button("Ruler control sentinel").clicked();
                draw_timeline_cue_dialog(&mut app, &context);
            });
            for _ in 0..3 { discard_ui_output(render(Vec::new())); }
            let mut output = render(vec![egui::Event::PointerMoved(toolbar_rect.center())]);
            let control = output.shapes.iter().position(|shape| matches!(&shape.shape,
                egui::epaint::Shape::Text(text) if text.galley.job.text == "Ruler control sentinel"
            )).expect("toolbar not painted");
            let backdrop = output.shapes.iter().position(|shape| matches!(&shape.shape,
                egui::epaint::Shape::Rect(rect) if rect.rect.contains_rect(toolbar_rect)
                    && rect.fill == egui::Color32::from_rgba_premultiplied(0, 0, 0, 150)
            )).expect("full viewport backdrop missing");
            assert!(control < backdrop, "toolbar paints above the modal backdrop; dark={dark}");
            assert_eq!(context.layer_id_at(toolbar_rect.center()).unwrap().order, egui::Order::Foreground);
            output.textures_delta.clear();
            for pressed in [true, false] {
                discard_ui_output(render(vec![egui::Event::PointerButton {
                    pos: toolbar_rect.center(), button: egui::PointerButton::Primary,
                    pressed, modifiers: egui::Modifiers::NONE,
                }]));
            }
            assert!(!clicked, "modal backdrop allowed a toolbar action; dark={dark}");
        }
    }

    #[test]
    fn timeline_ruler_does_not_steal_toolbar_cursor_or_seek() {
        let ruler = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(900.0, 30.0));
        let toolbar = egui::Rect::from_min_max(egui::pos2(650.0, 1.0), egui::pos2(897.0, 29.0));
        assert!(!timeline_ruler_owns_pointer(ruler, toolbar, toolbar.center(), false));
        assert!(timeline_ruler_owns_pointer(ruler, toolbar, egui::pos2(200.0, 15.0), false));
        // A seek already started on the ruler may continue across the toolbar.
        assert!(timeline_ruler_owns_pointer(ruler, toolbar, toolbar.center(), true));
        assert!(!timeline_ruler_owns_pointer(ruler, toolbar, egui::pos2(200.0, 60.0), false));
    }

    #[test]
    fn timeline_navigation_transaction_does_not_reenter_egui_context() {
        let context = egui::Context::default();
        let id = egui::Id::new("navigation-lock-regression");
        let mut output = context.run_ui(
            egui::RawInput { time: Some(10.0), ..Default::default() },
            |ui| {
                // Toolbar navigation, follow-playhead, wheel and reveal all
                // use this path. It must return with the context unlocked.
                for target_zoom in [120.0, 150.0, 100.0, 200.0] {
                    start_timeline_navigation_transition(
                        ui, id, (egui::vec2(10.0, 20.0), 100.0),
                        (egui::vec2(300.0, 40.0), target_zoom), 220,
                    );
                    let transition = ui.data(|data| {
                        data.get_temp::<TimelineNavigationTransition>(id).unwrap()
                    });
                    assert_eq!(transition.started_at_seconds, 10.0);
                    assert_eq!(transition.target_zoom, target_zoom);
                    assert_eq!(transition.duration_seconds, 0.22);
                }
            },
        );
        output.textures_delta.clear();
    }

    #[test]
    fn timeline_navigation_transition_eases_between_exact_endpoints() {
        let transition = TimelineNavigationTransition {
            start_offset: egui::vec2(100.0, 20.0),
            target_offset: egui::vec2(500.0, 60.0),
            start_zoom: 100.0,
            target_zoom: 200.0,
            started_at_seconds: 10.0,
            duration_seconds: 0.2,
            anchor: None,
        };

        assert_eq!(
            sample_timeline_navigation_transition(transition, 10.0),
            (egui::vec2(100.0, 20.0), 100.0, false)
        );
        assert_eq!(
            sample_timeline_navigation_transition(transition, 10.1),
            (egui::vec2(300.0, 40.0), 150.0, false)
        );
        assert_eq!(
            sample_timeline_navigation_transition(transition, 10.2),
            (egui::vec2(500.0, 60.0), 200.0, true)
        );
    }

    #[test]
    fn timeline_navigation_transition_preserves_anchor_focal_point_during_ease() {
        let anchor = TimelineZoomAnchor {
            pointer_x_in_viewport: 250.0,
            media_time_seconds: 3.5,
            duration_seconds: 100.0,
            viewport_width: 800.0,
        };
        let start_zoom: f32 = 100.0;
        let target_zoom: f32 = 250.0;
        let start_offset_x = (3.5_f32 * start_zoom - 250.0).max(0.0);
        let target_offset_x = (3.5_f32 * target_zoom - 250.0).max(0.0);
        let transition = TimelineNavigationTransition {
            start_offset: egui::vec2(start_offset_x, 0.0),
            target_offset: egui::vec2(target_offset_x, 0.0),
            start_zoom,
            target_zoom,
            started_at_seconds: 0.0,
            duration_seconds: 1.0,
            anchor: Some(anchor),
        };

        for step in 0..=10 {
            let t = step as f64 * 0.1;
            let (offset, zoom, _complete) = sample_timeline_navigation_transition(transition, t);
            let time_under_pointer = (offset.x + 250.0) / zoom;
            assert!(
                (time_under_pointer - 3.5).abs() < 1e-4,
                "Drift detected at t={t}: got {time_under_pointer}, expected 3.5"
            );
        }
    }

    #[test]
    fn timeline_zoom_range_clamped_to_min_and_max() {
        assert_eq!(timeline_zoom_from_wheel(1.0, -1000.0), TIMELINE_MIN_ZOOM);
        assert_eq!(timeline_zoom_from_wheel(1500.0, 1000.0), TIMELINE_MAX_ZOOM);
    }

    #[test]
    fn timeline_context_menu_identity_is_stable_per_channel_and_kind() {
        assert_ne!(
            timeline_track_row_id("hardware:relay.5"),
            timeline_track_row_id("hardware:pwm.0")
        );
        assert_ne!(
            timeline_track_row_id("hardware:pwm.0"),
            timeline_analog_track_row_id("hardware:pwm.0")
        );
    }

    #[test]
    fn timeline_header_rows_and_canvas_lanes_share_one_vertical_geometry() {
        let timeline_top = 73.0;

        for compact in [true, false] {
            let track_height = timeline_track_row_height(compact);
            let analog_height = timeline_analog_row_height(compact);
            assert_eq!(
                timeline_track_row_top(timeline_top, 0, track_height),
                timeline_top + TIMELINE_RULER_HEIGHT
            );
            assert_eq!(
                timeline_track_row_top(timeline_top, 3, track_height),
                timeline_top + TIMELINE_RULER_HEIGHT + 3.0 * track_height
            );
            assert_eq!(
                timeline_content_height(3, 2, track_height, analog_height),
                TIMELINE_RULER_HEIGHT + 3.0 * track_height + 2.0 * analog_height
            );
        }
    }

    #[test]
    fn timeline_fixed_rows_have_no_hidden_gap_against_canvas_lanes() {
        for compact in [true, false] {
            let track_height = timeline_track_row_height(compact);
            let analog_height = timeline_analog_row_height(compact);
            let context = egui::Context::default();
            let fixed_rects = std::cell::RefCell::new(Vec::<egui::Rect>::new());
            let canvas_rect = std::cell::Cell::new(egui::Rect::NOTHING);
            let output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(700.0, 220.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(TIMELINE_TRACK_HEADER_WIDTH);
                            ui.spacing_mut().item_spacing.y = 0.0;
                            for height in [TIMELINE_RULER_HEIGHT, track_height, track_height] {
                                fixed_rects.borrow_mut().push(
                                    ui.allocate_exact_size(
                                        egui::vec2(TIMELINE_TRACK_HEADER_WIDTH, height),
                                        egui::Sense::hover(),
                                    )
                                    .0,
                                );
                            }
                        });
                        canvas_rect.set(
                            ui.allocate_exact_size(
                                egui::vec2(
                                    360.0,
                                    timeline_content_height(2, 0, track_height, analog_height),
                                ),
                                egui::Sense::hover(),
                            )
                            .0,
                        );
                    });
                },
            );
            discard_ui_output(output);

            let fixed = fixed_rects.borrow();
            let canvas = canvas_rect.get();
            assert_eq!(fixed[0].top(), canvas.top());
            assert_eq!(fixed[0].bottom(), fixed[1].top());
            assert_eq!(fixed[1].bottom(), fixed[2].top());
            assert_eq!(
                fixed[1].top(),
                timeline_track_row_top(canvas.top(), 0, track_height)
            );
            assert_eq!(
                fixed[2].top(),
                timeline_track_row_top(canvas.top(), 1, track_height)
            );
            assert_eq!(fixed[2].bottom(), canvas.bottom());
        }
    }

    #[test]
    fn compact_timeline_density_changes_row_size_without_changing_alignment() {
        assert!(timeline_track_row_height(true) < timeline_track_row_height(false));
        assert!(timeline_analog_row_height(true) < timeline_analog_row_height(false));
        for compact in [true, false] {
            let track_height = timeline_track_row_height(compact);
            let analog_height = timeline_analog_row_height(compact);
            let top = 11.0;
            assert_eq!(
                timeline_track_row_top(top, 2, track_height),
                top + TIMELINE_RULER_HEIGHT + 2.0 * track_height
            );
            assert_eq!(
                timeline_content_height(2, 1, track_height, analog_height),
                TIMELINE_RULER_HEIGHT + 2.0 * track_height + analog_height
            );
        }
    }

    fn discard_ui_output(mut output: egui::FullOutput) {
        output.textures_delta.clear();
    }

    #[test]
    fn timeline_drag_indicator_keeps_hover_at_the_handle_edge() {
        let row = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(376.0, 34.0));

        // The handle is the right-most child widget. Its owning response must
        // not make the row-level fade target false when the pointer reaches it.
        assert!(timeline_track_handle_hovered(
            Some(egui::pos2(380.0, 37.0)),
            row,
            false,
        ));
        assert!(!timeline_track_handle_hovered(
            Some(egui::pos2(392.0, 37.0)),
            row,
            false,
        ));
        assert!(timeline_track_handle_hovered(None, row, true));
    }

    #[test]
    fn hardware_drag_indicator_uses_the_whole_card_hover_area() {
        let card = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(320.0, 72.0));
        // The current horizontal row starts after the leading icon/indicator.
        let remaining_row =
            egui::Rect::from_min_size(egui::pos2(92.0, 20.0), egui::vec2(238.0, 32.0));

        assert!(hardware_channel_handle_hovered(
            Some(egui::pos2(28.0, 42.0)),
            Some(card),
            remaining_row,
            false,
        ));
        assert!(!hardware_channel_handle_hovered(
            Some(egui::pos2(400.0, 42.0)),
            Some(card),
            remaining_row,
            false,
        ));
        assert!(hardware_channel_handle_hovered(
            None,
            Some(card),
            remaining_row,
            true,
        ));
    }

    #[test]
    fn hardware_monitor_scroll_reaches_lower_cards() {
        let context = egui::Context::default();
        let mut app = PealayerApp::default();
        let capabilities = crate::four_d::controller::HardwareCapabilities::default();
        let offset = std::cell::Cell::new(0.0);
        let viewport_height = std::cell::Cell::new(0.0);
        let content_height = std::cell::Cell::new(0.0);
        let first_card = std::cell::Cell::new(egui::Rect::NOTHING);
        let mut render = |events| {
            let output = context.run_ui(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(420.0, 300.0))),
                events,
                ..Default::default()
            }, |ui| {
                let scroll = hardware_monitor_scroll(ui, |ui| {
                    for relay in 1..=14 {
                        let control = crate::four_d::controller::HardwareControl {
                            key: format!("relay.{relay}"), kind: "relay".to_string(),
                            name: format!("Channel {relay}"), ..Default::default()
                        };
                        let top = ui.next_widget_position();
                        draw_control_card(&mut app, ui, &capabilities, &control);
                        if relay == 1 {
                            first_card.set(egui::Rect::from_min_size(top, egui::vec2(300.0, 40.0)));
                        }
                        ui.add_space(8.0);
                    }
                });
                offset.set(scroll.state.offset.y);
                viewport_height.set(scroll.inner_rect.height());
                content_height.set(scroll.content_size.y);
            });
            discard_ui_output(output);
        };
        render(Vec::new());
        render(vec![egui::Event::PointerMoved(first_card.get().center())]);
        render(vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            phase: egui::TouchPhase::Move,
            delta: egui::vec2(0.0, -100.0), modifiers: egui::Modifiers::NONE,
        }]);
        for _ in 0..8 { render(Vec::new()); }
        assert!(content_height.get() > viewport_height.get());
        assert!(viewport_height.get() <= 300.0);
        assert!(offset.get() > 0.0, "wheel over a card did not scroll the Hardware Monitor");
    }

    #[test]
    fn hardware_cards_paint_below_subtitle_settings_and_keep_panel_clip() {
        for dark in [false, true] {
            for compact in [false, true] {
                let context = egui::Context::default();
                context.set_visuals(if dark { egui::Visuals::dark() } else { egui::Visuals::light() });
                let mut app = PealayerApp::default();
                app.show_sub_settings = true;
                app.compact_hardware_controls = compact;
                let capabilities = crate::four_d::controller::HardwareCapabilities::default();
                let control = crate::four_d::controller::HardwareControl { key: "relay.5".into(), kind: "relay".into(), name: "Hardware sentinel".into(), ..Default::default() };
                let clip = egui::Rect::from_min_max(egui::pos2(8.0, 8.0), egui::pos2(388.0, 160.0));
                let mut final_output = None;
                for _ in 0..3 {
                    let mut output = context.run_ui(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 800.0))), ..Default::default() }, |ui| {
                        ui.scope(|ui| {
                            ui.set_width(380.0);
                            ui.set_clip_rect(clip);
                            draw_control_card(&mut app, ui, &capabilities, &control);
                        });
                        crate::ui::subtitles::draw_settings_dialog(&mut app, ui);
                    });
                    output.textures_delta.clear();
                    final_output = Some(output);
                }
                let output = final_output.unwrap();
                let card_index = output.shapes.iter().position(|s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.job.text == "Hardware sentinel")).expect("card not painted");
                let dialog_index = output.shapes.iter().position(|s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.job.text.contains("Visibility"))).expect("subtitle dialog not painted");
                assert!(card_index < dialog_index, "hardware card paints over Subtitle settings; dark={dark}, compact={compact}");
                assert!(clip.contains_rect(output.shapes[card_index].clip_rect), "card painting escaped the Hardware Monitor clip");
                let dialog_rect = context.memory(|memory| memory.area_rect(egui::Id::new("subtitle_settings_dialog_professional_v4"))).expect("subtitle window bounds missing");
                assert!(dialog_rect.expand(8.0).contains_rect(output.shapes[dialog_index].clip_rect), "subtitle contents escaped their window clip");
                assert_eq!(context.layer_id_at(dialog_rect.center()).unwrap().order, egui::Order::Foreground, "workspace must not own dialog pointer interactions");
            }
        }
    }

    #[test]
    fn hardware_drag_manager_coordinates_and_release_are_not_owned_by_monitor() {
        let context = egui::Context::default();
        let monitor_rect = egui::Rect::from_min_size(egui::pos2(12.0, 20.0), egui::vec2(280.0, 80.0));
        let manager_rect = egui::Rect::from_min_size(egui::pos2(350.0, 150.0), egui::vec2(540.0, 36.0));
        let control = crate::four_d::controller::HardwareControl {
            key: "relay.5".to_string(), kind: "relay".to_string(), ..Default::default()
        };
        let target = crate::four_d::controller::HardwareControl {
            key: "relay.6".to_string(), kind: "relay".to_string(), ..Default::default()
        };
        let target_rect = manager_rect.translate(egui::vec2(0.0, 60.0));
        let grab_offset = egui::vec2(46.0, 18.0);
        let output = context.run_ui(egui::RawInput::default(), |ui| {
            let empty_layer = egui::LayerId::new(egui::Order::Middle, egui::Id::new("test-drag-layer"));
            finish_hardware_channel_card(ui, &control, monitor_rect, empty_layer, HardwareChannelDragSurface::Monitor);
            finish_hardware_channel_card(ui, &control, manager_rect, empty_layer, HardwareChannelDragSurface::Manager);
            let monitor_id = hardware_channel_source_rect_id(ui, HardwareChannelDragSurface::Monitor, &control.key);
            let manager_id = hardware_channel_source_rect_id(ui, HardwareChannelDragSurface::Manager, &control.key);
            assert_ne!(monitor_id, manager_id);
            assert_eq!(ui.data_mut(|data| data.get_temp::<egui::Rect>(monitor_id)), Some(monitor_rect));
            assert_eq!(ui.data_mut(|data| data.get_temp::<egui::Rect>(manager_id)), Some(manager_rect));
            let drag_id = hardware_channel_drag_id(ui, HardwareChannelDragSurface::Manager);
            ui.data_mut(|data| data.insert_temp(drag_id, HardwareChannelDrag {
                key: control.key.clone(), kind: control.kind.clone(), grab_offset,
            }));
        });
        discard_ui_output(output);
        // A real primary release is rendered by the panel before the modal.
        let point = target_rect.center();
        let output = context.run_ui(egui::RawInput {
            events: vec![egui::Event::PointerMoved(point), egui::Event::PointerButton {
                pos: point, button: egui::PointerButton::Primary, pressed: true,
                modifiers: egui::Modifiers::NONE,
            }], ..Default::default()
        }, |_| {});
        discard_ui_output(output);
        let output = context.run_ui(egui::RawInput {
            events: vec![egui::Event::PointerButton {
                pos: point, button: egui::PointerButton::Primary, pressed: false,
                modifiers: egui::Modifiers::NONE,
            }], ..Default::default()
        }, |ui| {
            assert!(!hardware_channel_is_dragging(ui, &control.key, HardwareChannelDragSurface::Monitor));
            assert!(hardware_channel_drop_target(ui, target_rect, &target, HardwareChannelDragSurface::Monitor).is_none());
            clear_released_hardware_channel_drag(ui, HardwareChannelDragSurface::Monitor);
            assert!(hardware_channel_is_dragging(ui, &control.key, HardwareChannelDragSurface::Manager));
            let drag_id = hardware_channel_drag_id(ui, HardwareChannelDragSurface::Manager);
            let drag = ui.data_mut(|data| data.get_temp::<HardwareChannelDrag>(drag_id)).unwrap();
            assert_eq!(drag_translation(manager_rect.min + grab_offset, manager_rect.min, drag.grab_offset), egui::Vec2::ZERO);
            let drop = hardware_channel_drop_target(ui, target_rect, &target, HardwareChannelDragSurface::Manager).unwrap();
            assert_eq!(drop.source_key, control.key);
            assert_eq!(drop.target_key, target.key);
            assert!(!hardware_channel_is_dragging(ui, &control.key, HardwareChannelDragSurface::Manager));
        });
        discard_ui_output(output);
    }

    #[test]
    fn hardware_drag_pointer_start_and_drop_work_in_each_surface() {
        use std::cell::{Cell, RefCell};
        for origin in [HardwareChannelDragSurface::Monitor, HardwareChannelDragSurface::Manager] {
            let context = egui::Context::default();
            let app = PealayerApp::default();
            let control = crate::four_d::controller::HardwareControl {
                key: "relay.5".to_string(), kind: "relay".to_string(), ..Default::default()
            };
            let target = crate::four_d::controller::HardwareControl {
                key: "relay.6".to_string(), kind: "relay".to_string(), ..Default::default()
            };
            let source_handle = Cell::new(egui::Rect::NOTHING);
            let source_rect = Cell::new(egui::Rect::NOTHING);
            let drop_rect = Cell::new(egui::Rect::NOTHING);
            let started_drag = RefCell::new(None::<HardwareChannelDrag>);
            let dropped = Cell::new(false);
            let render = |events| {
                let output = context.run_ui(egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(780.0, 400.0))),
                    events, ..Default::default()
                }, |ui| {
                    ui.horizontal_top(|ui| {
                        for surface in [HardwareChannelDragSurface::Monitor, HardwareChannelDragSurface::Manager] {
                            ui.push_id(surface, |ui| {
                                ui.allocate_ui(egui::vec2(300.0, 250.0), |ui| {
                                    for row in [&control, &target] {
                                        let frame = egui::Frame::group(ui.style()).show(ui, |ui| {
                                            ui.set_width(260.0);
                                            // Distinct card and modal geometry deliberately
                                            // shares the same underlying channel stable key.
                                            if surface == HardwareChannelDragSurface::Manager { ui.add_space(12.0); }
                                            ui.horizontal(|ui| {
                                                let handle = hardware_channel_drag_handle(&app, ui, row, surface);
                                                if surface == origin && row.key == control.key {
                                                    source_handle.set(handle.rect);
                                                    if handle.drag_started() {
                                                        let id = hardware_channel_drag_id(ui, surface);
                                                        *started_drag.borrow_mut() = ui.data_mut(|data| data.get_temp::<HardwareChannelDrag>(id));
                                                    }
                                                }
                                                ui.label(&row.key);
                                            });
                                        });
                                        if surface == origin && row.key == control.key { source_rect.set(frame.response.rect); }
                                        if surface == origin && row.key == target.key { drop_rect.set(frame.response.rect); }
                                        let empty_layer = egui::LayerId::new(egui::Order::Middle, egui::Id::new(("test-empty-drag-layer", surface, &row.key)));
                                        finish_hardware_channel_card(ui, row, frame.response.rect, empty_layer, surface);
                                        if let Some(drop) = hardware_channel_drop_target(ui, frame.response.rect, row, surface) {
                                            assert_eq!(surface, origin);
                                            assert_eq!(drop.source_key, control.key);
                                            assert_eq!(drop.target_key, target.key);
                                            dropped.set(true);
                                        }
                                    }
                                    clear_released_hardware_channel_drag(ui, surface);
                                });
                            });
                        }
                    });
                });
                discard_ui_output(output);
            };
            render(Vec::new());
            let press = source_handle.get().center();
            let start = press + egui::vec2(18.0, 2.0);
            render(vec![egui::Event::PointerMoved(press), egui::Event::PointerButton {
                pos: press, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE,
            }]);
            render(vec![egui::Event::PointerMoved(start)]);
            let captured = started_drag.borrow().clone().expect("hardware drag did not start from its handle");
            assert_eq!(captured.grab_offset, start - source_rect.get().min, "grab offset came from the other surface");
            let destination = drop_rect.get().center();
            render(vec![egui::Event::PointerMoved(destination)]);
            render(vec![egui::Event::PointerButton {
                pos: destination, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE,
            }]);
            assert!(dropped.get(), "hardware drag did not drop in {origin:?}");
        }
    }

    #[test]
    fn channel_drop_computes_a_kind_local_rank() {
        let control =
            |key: &str, kind: &str, order: u16| crate::four_d::controller::HardwareControl {
                key: key.to_string(),
                kind: kind.to_string(),
                order,
                ..Default::default()
            };
        let capabilities = crate::four_d::controller::HardwareCapabilities {
            controls: vec![
                control("relay.5", "relay", 0),
                control("relay.6", "relay", 1),
                control("relay.7", "relay", 2),
                control("pwm.0", "mosfet", 0),
            ],
            ..Default::default()
        };
        assert_eq!(
            reordered_channel_rank(
                &capabilities,
                &HardwareChannelDrop {
                    source_key: "relay.7".to_string(),
                    target_key: "relay.5".to_string(),
                    before: true,
                },
            ),
            Some(0)
        );
        assert_eq!(
            reordered_channel_rank(
                &capabilities,
                &HardwareChannelDrop {
                    source_key: "relay.5".to_string(),
                    target_key: "relay.6".to_string(),
                    before: false,
                },
            ),
            Some(1)
        );
        assert_eq!(
            reordered_channel_rank(
                &capabilities,
                &HardwareChannelDrop {
                    source_key: "relay.5".to_string(),
                    target_key: "pwm.0".to_string(),
                    before: true,
                },
            ),
            None
        );
    }

    #[test]
    fn magnetic_targets_include_exact_and_automation_keyframes() {
        let mut timeline = crate::four_d::models::Timeline::default();
        timeline.add_keyframe(1_250);
        let mut track = crate::four_d::curve::AnalogTrack::new("PWM", 1);
        track.add_keyframe(crate::four_d::curve::Keyframe::new(
            1_500,
            0.5,
            crate::four_d::curve::Interpolation::Linear,
        ));
        timeline.analog_tracks.push(track);
        let effect =
            crate::four_d::models::Effect::new("Cue".to_string(), String::new(), 500, Vec::new());
        let effect_id = effect.id;
        timeline.templates.push(effect);
        let instance = crate::four_d::models::EffectInstance::new(effect_id, 2_000);
        let instance_id = instance.id;
        timeline.instances.push(instance);

        let targets = timeline_snap_targets(&timeline, &std::collections::HashSet::new(), 750);
        assert_eq!(targets, vec![0, 750, 1_250, 1_500, 2_000, 2_500]);

        let selected = std::collections::HashSet::from([instance_id]);
        let targets = timeline_snap_targets(&timeline, &selected, 750);
        assert_eq!(targets, vec![0, 750, 1_250, 1_500]);
    }

    #[test]
    fn magnetic_snap_chooses_nearest_target_within_visual_tolerance() {
        assert_eq!(nearest_snap_time(1_040, &[1_000, 1_100], 50), Some(1_000));
        assert_eq!(nearest_snap_time(1_060, &[1_000, 1_100], 50), Some(1_100));
        assert_eq!(nearest_snap_time(1_060, &[1_000, 1_100], 30), None);
        assert_eq!(timeline_snap_tolerance_ms(0.1), 80);
    }

    #[test]
    fn keyboard_nudge_preserves_group_spacing_and_clamps_at_zero() {
        let mut timeline = crate::four_d::models::Timeline::default();
        let effect =
            crate::four_d::models::Effect::new("Cue".to_string(), String::new(), 100, Vec::new());
        let effect_id = effect.id;
        timeline.templates.push(effect);
        let first = crate::four_d::models::EffectInstance::new(effect_id, 20);
        let second = crate::four_d::models::EffectInstance::new(effect_id, 70);
        let selected = std::collections::HashSet::from([first.id, second.id]);
        timeline.instances.extend([first, second]);

        assert!(move_selected_cues(&mut timeline, &selected, -50));
        assert_eq!(timeline.instances[0].start_time_ms, 0);
        assert_eq!(timeline.instances[1].start_time_ms, 50);
        assert_eq!(timeline_frame_step_ms(25.0, 1), 40);
        assert_eq!(timeline_frame_step_ms(0.0, 2), 2);
    }

    #[test]
    fn cue_manage_reveals_expands_and_focuses_effect_controls() {
        let mut dock_state = create_initial_layout();
        let controls_path = dock_state
            .find_tab(&PealayerTab::EffectControls)
            .expect("effect controls starts in the canonical workspace");
        dock_state
            .leaf_mut(controls_path.node_path())
            .expect("effect controls belongs to a leaf")
            .collapsed = true;

        let timeline_path = dock_state
            .find_tab(&PealayerTab::Timeline)
            .expect("timeline starts in the canonical workspace");
        dock_state.set_focused_node_and_surface(timeline_path.node_path());

        assert!(reveal_and_focus_tab(
            &mut dock_state,
            PealayerTab::EffectControls
        ));
        let revealed_path = dock_state
            .find_tab(&PealayerTab::EffectControls)
            .expect("effect controls remains open");
        let leaf = dock_state
            .leaf(revealed_path.node_path())
            .expect("revealed tab belongs to a leaf");
        assert!(!leaf.collapsed);
        assert_eq!(leaf.active, revealed_path.tab);
        assert_eq!(dock_state.focused_leaf(), Some(revealed_path.node_path()));

        assert!(!reveal_and_focus_tab(
            &mut dock_state,
            PealayerTab::EffectControls
        ));
    }

    #[test]
    fn cue_reveal_triggers_effect_controls_ping_and_calculates_decay() {
        let mut app = PealayerApp::default();
        assert_eq!(app.effect_controls_ping_strength(0.0), 0.0);

        app.trigger_effect_controls_ping(10.0);
        assert_eq!(app.effect_controls_ping_at, Some(10.0));
        assert!((app.effect_controls_ping_strength(10.0) - 1.0).abs() < 1e-4);
        assert!((app.effect_controls_ping_strength(10.6) - 0.5).abs() < 1e-2);
        assert_eq!(app.effect_controls_ping_strength(11.2), 0.0);
        assert_eq!(app.effect_controls_ping_strength(12.0), 0.0);
    }

    #[test]
    fn cue_reveal_restores_closed_effect_controls_tab() {
        let mut app = PealayerApp::default();
        let path = app.dock_state.find_tab(&PealayerTab::EffectControls).expect("starts open");
        app.dock_state.remove_tab(path);
        assert!(app.dock_state.find_tab(&PealayerTab::EffectControls).is_none());

        app.open_or_focus_tab(PealayerTab::EffectControls);
        app.trigger_effect_controls_ping(5.0);

        let restored = app.dock_state.find_tab(&PealayerTab::EffectControls).expect("restored tab");
        let leaf = app.dock_state.leaf(restored.node_path()).expect("leaf");
        assert_eq!(leaf.active, restored.tab);
    }

    #[test]
    fn open_or_focus_tab_during_dock_render_persists_restored_tab() {
        let mut app = PealayerApp::default();
        let path = app.dock_state.find_tab(&PealayerTab::EffectControls).expect("starts open");
        app.dock_state.remove_tab(path);
        assert!(app.dock_state.find_tab(&PealayerTab::EffectControls).is_none());

        // Simulate dock render: swap out dock_state like src/app.rs:1565
        let dock_state = std::mem::replace(&mut app.dock_state, egui_dock::DockState::new(vec![]));

        // Tab viewer calls open_or_focus_tab while swapped out
        app.open_or_focus_tab(PealayerTab::EffectControls);

        // Put back dock_state and process pending reveals like src/app.rs:1581
        app.dock_state = dock_state;
        for tab in std::mem::take(&mut app.pending_tab_reveals) {
            reveal_and_focus_tab(&mut app.dock_state, tab);
        }

        assert!(
            app.dock_state.find_tab(&PealayerTab::EffectControls).is_some(),
            "Restored tab must not be lost when called during dock render"
        );
    }

    #[test]
    fn display_text_prefers_segments_when_both_displays_are_available() {
        assert_eq!(default_display_text_target(true, true), "segments");
        assert_eq!(default_display_text_target(true, false), "segments");
        assert_eq!(default_display_text_target(false, true), "lcd");
    }

    #[test]
    fn addressable_strip_frames_preserve_endpoints_and_apply_brightness() {
        let frame = addressable_strip_gradient(
            3,
            egui::Color32::from_rgb(200, 0, 0),
            egui::Color32::from_rgb(0, 0, 100),
            128,
        );
        assert_eq!(frame.len(), 9);
        assert_eq!(&frame[0..3], &[100, 0, 0]);
        assert_eq!(&frame[3..6], &[50, 0, 25]);
        assert_eq!(&frame[6..9], &[0, 0, 50]);
    }

    #[test]
    fn hardware_buzzer_card_exposes_live_state_catalog_and_tone_controls() {
        let context = egui::Context::default();
        let mut app = PealayerApp::default();
        let capabilities = crate::four_d::controller::HardwareCapabilities {
            board_connected: true,
            buzzer: crate::four_d::controller::HardwareBuzzerState {
                melody_id: 9,
                melody_name: "attention".to_string(),
            },
            melodies: vec![crate::four_d::controller::HardwareMelody {
                name: "attention".to_string(),
                notes: vec![crate::four_d::controller::HardwareMelodyNote {
                    frequency_hz: 880,
                    duration_ms: 125,
                    gap_ms: 25,
                }],
            }],
            settings: Some(crate::four_d::controller::HardwareBoardSettings {
                silent: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(520.0, 600.0),
                )),
                ..Default::default()
            },
            |ui| {
                ui.set_width(440.0);
                draw_buzzer_tool(&mut app, ui, &capabilities);
            },
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("Buzzer & melodies"));
        assert!(text.contains("Playing: attention"));
        assert!(text.contains("Board muted"));
        assert!(text.contains("Play melody"));
        assert!(text.contains("Stop buzzer"));
        assert!(text.contains("Tone test"));
        output.textures_delta.clear();
    }

    #[test]
    fn dynamic_rows_map_only_explicit_relays() {
        let rows = vec![
            TimelineTrackRow {
                key: "media:video".to_string(),
                name: "Video".to_string(),
                detail: None,
                active: true,
                enabled: true,
                linked: true,
                visible: true,
                icon: crate::ui::icons::FILE_VIDEO.to_string(),
                control_key: None,
                relay_ids: Vec::new(),
                dimmed: false,
                kind: TimelineTrackKind::Video(Some(1)),
            },
            TimelineTrackRow {
                key: "media:audio:1".to_string(),
                name: "English".to_string(),
                detail: Some("Audio".to_string()),
                active: true,
                enabled: true,
                linked: true,
                visible: true,
                icon: crate::ui::icons::SPEAKER_HIGH.to_string(),
                control_key: None,
                relay_ids: Vec::new(),
                dimmed: false,
                kind: TimelineTrackKind::Audio(1),
            },
            TimelineTrackRow {
                key: "media:subtitle:2".to_string(),
                name: "Farsi".to_string(),
                detail: Some("Subtitles".to_string()),
                active: false,
                enabled: false,
                linked: true,
                visible: true,
                icon: crate::ui::icons::SUBTITLES.to_string(),
                control_key: None,
                relay_ids: Vec::new(),
                dimmed: false,
                kind: TimelineTrackKind::Subtitle(2),
            },
            TimelineTrackRow {
                key: "hardware:relay.6".to_string(),
                name: "Seat left".to_string(),
                detail: None,
                active: false,
                enabled: true,
                linked: true,
                visible: true,
                icon: crate::ui::icons::PLUG.to_string(),
                control_key: Some("relay.6".to_string()),
                relay_ids: vec![6],
                dimmed: false,
                kind: TimelineTrackKind::Relay(6),
            },
        ];
        assert_eq!(relay_for_timeline_row(&rows, 0), None);
        assert_eq!(relay_for_timeline_row(&rows, 1), None);
        assert_eq!(relay_for_timeline_row(&rows, 2), None);
        assert_eq!(relay_for_timeline_row(&rows, 3), Some(6));
        assert_eq!(timeline_row_for_relay(&rows, 6), Some(3));
        assert_eq!(timeline_row_for_relay(&rows, 1), None);
    }

    #[test]
    fn timeline_lists_every_real_audio_and_subtitle_track_in_media_order() {
        let mut app = PealayerApp::default();
        app.current_video_path = Some(std::path::PathBuf::from("feature.mkv"));
        app.current_vid = "1".to_string();
        app.current_aid = "7".to_string();
        app.current_sid = "12".to_string();
        app.sub_visibility = true;
        app.video_tracks = vec![crate::app::VideoTrack {
            id: 1,
            title: Some("Main picture".to_string()),
            lang: Some("und".to_string()),
        }];
        app.audio_tracks = vec![
            crate::app::AudioTrack {
                id: 7,
                title: Some("Main mix".to_string()),
                lang: Some("en".to_string()),
            },
            crate::app::AudioTrack {
                id: 8,
                title: Some("Commentary".to_string()),
                lang: None,
            },
        ];
        app.sub_tracks = vec![
            crate::app::SubtitleTrack {
                id: 11,
                title: Some("English SDH".to_string()),
                lang: Some("en".to_string()),
            },
            crate::app::SubtitleTrack {
                id: 12,
                title: Some("فارسی".to_string()),
                lang: Some("fa".to_string()),
            },
        ];

        let rows = timeline_track_rows(&app);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].kind, TimelineTrackKind::Video(Some(1)));
        assert_eq!(rows[1].kind, TimelineTrackKind::Audio(7));
        assert_eq!(rows[2].kind, TimelineTrackKind::Audio(8));
        assert_eq!(rows[3].kind, TimelineTrackKind::Subtitle(11));
        assert_eq!(rows[4].kind, TimelineTrackKind::Subtitle(12));
        assert!(rows[1].active);
        assert!(!rows[2].active);
        assert!(!rows[3].active);
        assert!(rows[4].active && rows[4].enabled);
        assert_eq!(rows[0].name, "Main picture");
        assert_eq!(rows[0].detail.as_deref(), Some("Video · und"));
        assert_eq!(rows[1].detail.as_deref(), Some("Audio · en"));
        assert_eq!(rows[4].detail.as_deref(), Some("Subtitles · fa"));
    }

    #[test]
    fn media_track_labels_are_two_line_and_reject_a_wrong_generic_type() {
        let app = PealayerApp::default();
        let (audio_name, audio_detail) =
            media_timeline_track_label(&app, "Audio", 0, Some("Video"), Some("en"));
        assert_eq!(audio_name, "Audio");
        assert_eq!(audio_detail.as_deref(), Some("Track 1 · en"));

        let (video_name, video_detail) = media_timeline_track_label(&app, "Video", 0, None, None);
        assert_eq!(video_name, "Video");
        assert_eq!(video_detail.as_deref(), Some("Track 1"));
    }

    #[test]
    fn timeline_exposes_each_advertised_hardware_channel_and_honors_track_policy() {
        let mut app = PealayerApp::default();
        app.update_hardware_capabilities(Some(crate::four_d::controller::HardwareCapabilities {
            board_connected: true,
            controls: vec![
                crate::four_d::controller::HardwareControl {
                    key: "seat.a".to_string(),
                    kind: "seat".to_string(),
                    name: "Seat A".to_string(),
                    ..Default::default()
                },
                crate::four_d::controller::HardwareControl {
                    key: "relay.5".to_string(),
                    kind: "relay".to_string(),
                    name: "User relay".to_string(),
                    ..Default::default()
                },
                crate::four_d::controller::HardwareControl {
                    key: "relay.1".to_string(),
                    kind: "relay".to_string(),
                    name: "Raw direction relay".to_string(),
                    ..Default::default()
                },
                crate::four_d::controller::HardwareControl {
                    key: "pwm.12".to_string(),
                    kind: "pwm".to_string(),
                    name: "Enclosure light".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }));

        let rows = all_timeline_track_rows(&app);
        assert!(rows.iter().any(|row| {
            row.key == "hardware:seat.a"
                && row.kind == TimelineTrackKind::Hardware("seat.a".to_string())
                && row.relay_ids == [1, 2]
        }));
        assert!(rows.iter().any(|row| {
            row.key == "hardware:relay.5" && row.kind == TimelineTrackKind::Relay(5)
        }));
        assert!(rows.iter().any(|row| {
            row.key == "hardware:pwm.12"
                && row.kind == TimelineTrackKind::Hardware("pwm.12".to_string())
        }));
        assert!(
            rows.iter()
                .any(|row| row.key == "hardware:relay.1" && !row.visible)
        );

        app.timeline.set_track_visible("hardware:pwm.12", false);
        app.timeline.set_track_linked("hardware:relay.5", false);
        let visible_rows = timeline_track_rows(&app);
        assert!(!visible_rows.iter().any(|row| row.key == "hardware:pwm.12"));
        assert!(!visible_rows.iter().any(|row| row.key == "hardware:relay.5"));
        assert!(visible_rows.iter().any(|row| row.key == "hardware:seat.a"));

        app.set_timeline_track_visible("hardware:relay.1", true);
        assert!(
            timeline_track_rows(&app)
                .iter()
                .any(|row| row.key == "hardware:relay.1")
        );

        app.non_user_control_visibility = crate::config::NonUserControlVisibility::Dimmed;
        assert!(
            all_timeline_track_rows(&app)
                .iter()
                .any(|row| row.key == "hardware:relay.1" && row.dimmed)
        );
    }

    #[test]
    fn cue_dialog_drafts_valid_actions_for_relay_and_pwm_tracks() {
        let mut app = PealayerApp::default();
        app.update_hardware_capabilities(Some(crate::four_d::controller::HardwareCapabilities {
            board_connected: true,
            controls: vec![
                crate::four_d::controller::HardwareControl {
                    key: "relay.5".to_string(),
                    kind: "relay".to_string(),
                    name: "Fog relay".to_string(),
                    ..Default::default()
                },
                crate::four_d::controller::HardwareControl {
                    key: "pwm.12".to_string(),
                    kind: "pwm".to_string(),
                    name: "House light".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }));
        let rows = all_timeline_track_rows(&app);
        let relay = rows
            .iter()
            .find(|row| row.key == "hardware:relay.5")
            .expect("relay track should be advertised");
        let pwm = rows
            .iter()
            .find(|row| row.key == "hardware:pwm.12")
            .expect("PWM track should be advertised");

        let relay_draft = timeline_cue_draft_for_row(&app, relay, 2_750)
            .expect("relay track should accept a direct cue");
        assert_eq!(relay_draft.start_time_ms, 2_750);
        assert_eq!(relay_draft.duration_ms, 1_000);
        assert_eq!(relay_draft.value_basis_points(), 10_000);

        let pwm_draft = timeline_cue_draft_for_row(&app, pwm, 4_000)
            .expect("PWM track should accept a direct cue");
        assert_eq!(pwm_draft.start_time_ms, 4_000);
        assert_eq!(pwm_draft.value_basis_points(), 5_000);
    }

    #[test]
    fn cue_dialog_rejects_a_locked_track() {
        let mut app = PealayerApp::default();
        app.update_hardware_capabilities(Some(crate::four_d::controller::HardwareCapabilities {
            board_connected: true,
            controls: vec![crate::four_d::controller::HardwareControl {
                key: "relay.6".to_string(),
                kind: "relay".to_string(),
                name: "Locked relay".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }));
        app.track_locked.insert(6);
        let row = all_timeline_track_rows(&app)
            .into_iter()
            .find(|row| row.key == "hardware:relay.6")
            .expect("relay track should be advertised");

        assert!(timeline_cue_draft_for_row(&app, &row, 0).is_err());
    }

    #[test]
    fn timeline_track_filter_searches_caption_detail_and_stable_key() {
        let row = TimelineTrackRow {
            key: "hardware:seat.a".to_string(),
            name: "Left cinema seat".to_string(),
            detail: Some("Motion controls".to_string()),
            active: false,
            enabled: true,
            linked: true,
            visible: true,
            icon: crate::ui::icons::SEAT.to_string(),
            control_key: Some("seat.a".to_string()),
            relay_ids: vec![1, 2],
            dimmed: false,
            kind: TimelineTrackKind::Hardware("seat.a".to_string()),
        };
        assert!(timeline_track_matches_filter(&row, "cinema"));
        assert!(timeline_track_matches_filter(&row, "motion"));
        assert!(timeline_track_matches_filter(&row, "seat.a"));
        assert!(!timeline_track_matches_filter(&row, "right"));
        assert!(can_add_timeline_keyframe(&[row], 0, 0.0));
        assert!(!can_add_timeline_keyframe(&[], 0, 0.0));
        assert!(!can_add_timeline_keyframe(&[], 1, f64::NAN));
    }

    #[test]
    fn exact_keyframe_insertion_selects_and_deduplicates_the_durable_model() {
        let mut app = PealayerApp::default();
        app.playback_time = 8.75;
        app.seek_pos = Some(8.75);
        let (first, inserted) = app.insert_timeline_keyframe(1_250);
        assert!(inserted);
        assert_eq!(app.timeline.keyframes.len(), 1);
        assert_eq!(app.timeline.keyframes[0].time_ms, 1_250);
        assert_eq!(app.selected_timeline_keyframe, Some(first));
        assert_eq!(app.playback_time, 8.75, "inserting a keyframe must not seek playback");
        assert_eq!(app.seek_pos, Some(8.75), "inserting a keyframe must not change the pending seek");

        let (duplicate, inserted) = app.insert_timeline_keyframe(1_250);
        assert!(!inserted);
        assert_eq!(duplicate, first);
        assert_eq!(app.timeline.keyframes.len(), 1);
    }

    #[test]
    fn exact_keyframe_context_time_is_stable_and_clamped() {
        let captured = timeline_pointer_time_ms(350.0, 100.0, 0.25, 8_000);
        assert_eq!(captured, 1_000);
        assert_eq!(timeline_pointer_time_ms(4_000.0, 100.0, 0.25, 8_000), 8_000);
        assert_eq!(timeline_pointer_time_ms(50.0, 100.0, 0.25, 8_000), 0);
    }

    #[test]
    fn exact_keyframe_insertion_is_written_to_the_media_sidecar() {
        let temp_dir = std::env::temp_dir().join(format!(
            "pealayer-keyframe-persistence-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir(&temp_dir).expect("temporary test directory should be created");
        let media_path = temp_dir.join("sample.mkv");
        let sidecar_path = temp_dir.join("sample.4d.json");

        let mut app = PealayerApp::default();
        app.current_video_path = Some(media_path);
        let (_, inserted) = app.insert_timeline_keyframe(2_750);
        assert!(inserted);
        let restored = crate::four_d::models::Timeline::load_from_file(&sidecar_path)
            .expect("insertion should persist a readable timeline sidecar");
        assert_eq!(restored.keyframes.len(), 1);
        assert_eq!(restored.keyframes[0].time_ms, 2_750);

        std::fs::remove_file(&sidecar_path).expect("test sidecar should be removable");
        std::fs::remove_dir(&temp_dir).expect("empty test directory should be removable");
    }

    #[test]
    fn exact_keyframe_marker_does_not_overlap_the_playhead_handle() {
        let ruler = egui::Rect::from_min_max(egui::pos2(10.0, 20.0), egui::pos2(310.0, 46.0));
        let marker = timeline_keyframe_marker_center(ruler, 80.0);
        let marker_bottom = marker.y + 5.0;
        let playhead_handle_top = ruler.max.y - 12.0;
        assert!(marker_bottom < playhead_handle_top);
    }

    #[test]
    fn keyframe_near_hit_chooses_one_closest_marker() {
        let first = TimelineKeyframeTarget::Marker(uuid::Uuid::new_v4());
        let second = TimelineKeyframeTarget::Analog(uuid::Uuid::new_v4(), 0);
        let candidates = [(first, egui::pos2(60.0, 25.0)), (second, egui::pos2(72.0, 25.0))];
        assert_eq!(nearest_timeline_keyframe(Some(egui::pos2(62.0, 30.0)), &candidates), Some(first));
        assert_eq!(nearest_timeline_keyframe(Some(egui::pos2(76.0, 30.0)), &candidates), Some(second));
        assert_eq!(nearest_timeline_keyframe(Some(egui::pos2(100.0, 30.0)), &candidates), None);
        assert_eq!(nearest_timeline_keyframe(None, &candidates), None);
        assert_ne!(first.menu_id(), second.menu_id());
    }

    #[test]
    fn keyframe_context_menu_wins_over_canvas_and_ruler_near_the_diamond() {
        for target in [TimelineKeyframeTarget::Marker(uuid::Uuid::new_v4()),
            TimelineKeyframeTarget::Analog(uuid::Uuid::new_v4(), 0)]
        {
            let context = egui::Context::default();
            context.all_styles_mut(|style| style.animation_time = 0.0);
            let center = egui::pos2(100.0, 40.0);
            let render = |events| {
                let output = context.run_ui(egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 300.0))),
                    events, ..Default::default()
                }, |ui| {
                    let canvas_rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 200.0));
                    let canvas = ui.interact(canvas_rect, timeline_keyboard_focus_id(), egui::Sense::click_and_drag());
                    let ruler = ui.interact(canvas_rect, egui::Id::new("test-ruler"), egui::Sense::click());
                    let hit = nearest_timeline_keyframe(ui.ctx().pointer_latest_pos(), &[(target, center)]);
                    let owned = hit.is_some() || egui::Popup::is_id_open(ui.ctx(), target.menu_id());
                    if !owned { ruler.context_menu(|ui| { ui.label("Wrong ruler menu"); }); }
                    let marker = ui.interact(egui::Rect::from_center_size(center, egui::vec2(32.0, 32.0)),
                        egui::Id::new(("test-keyframe", target)), egui::Sense::click());
                    keyframe_context_menu(ui, &marker, target, hit == Some(target), |ui| {
                        ui.label("Dedicated keyframe menu");
                        ui.button("Delete keyframe").on_hover_text("Only this keyframe");
                    });
                    if !owned { canvas.context_menu(|ui| { ui.label("Wrong canvas menu"); }); }
                });
                discard_ui_output(output);
            };
            render(Vec::new());
            render(Vec::new());
            // Outside the painted 5px diamond, inside the forgiving near hit.
            let point = center + egui::vec2(12.0, 5.0);
            render(vec![egui::Event::PointerMoved(point), egui::Event::PointerButton {
                pos: point, button: egui::PointerButton::Secondary, pressed: true, modifiers: egui::Modifiers::NONE,
            }]);
            render(vec![egui::Event::PointerButton {
                pos: point, button: egui::PointerButton::Secondary, pressed: false, modifiers: egui::Modifiers::NONE,
            }]);
            assert!(egui::Popup::is_id_open(&context, target.menu_id()), "near right click opened the wrong menu");
            render(vec![egui::Event::PointerMoved(egui::pos2(210.0, 85.0))]);
            assert!(egui::Popup::is_id_open(&context, target.menu_id()), "menu lost its keyframe when pointer left marker");
        }
    }

    #[test]
    fn keyframe_highlight_clears_on_click_away_or_escape_but_not_menu_interaction() {
        let context = egui::Context::default();
        let mut app = PealayerApp::default();
        let (id, _) = app.insert_timeline_keyframe(1_000);
        let track = uuid::Uuid::new_v4();
        app.selected_keyframes.insert((track, 0));
        let render = |events, hit, popup, app: &mut PealayerApp| {
            let output = context.run_ui(egui::RawInput { events, ..Default::default() }, |ui| {
                // Register the real focus widget before requesting focus, preserving
                // the Windows accessibility crash fix for empty timelines.
                ui.interact(ui.max_rect(), timeline_keyboard_focus_id(), egui::Sense::click());
                ui.ctx().memory_mut(|memory| memory.request_focus(timeline_keyboard_focus_id()));
                clear_unfocused_timeline_keyframes(app, ui, hit, popup);
            });
            discard_ui_output(output);
        };
        let press = |pressed| egui::Event::PointerButton {
            pos: egui::pos2(150.0, 80.0), button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE,
        };
        render(vec![press(true)], true, false, &mut app);
        assert_eq!(app.selected_timeline_keyframe, Some(id));
        render(vec![press(false)], true, false, &mut app);
        render(vec![press(true)], false, true, &mut app);
        assert_eq!(app.selected_timeline_keyframe, Some(id), "popup interaction cleared its target");
        render(vec![press(false)], false, true, &mut app);
        render(vec![press(true)], false, false, &mut app);
        assert!(app.selected_timeline_keyframe.is_none());
        assert!(app.selected_keyframes.is_empty());
        render(vec![press(false)], false, false, &mut app);
        app.selected_timeline_keyframe = Some(id);
        app.selected_keyframes.insert((track, 0));
        render(vec![egui::Event::Key { key: egui::Key::Escape, physical_key: None,
            pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }], false, false, &mut app);
        assert!(app.selected_timeline_keyframe.is_none());
        assert!(app.selected_keyframes.is_empty());
        assert!(!context.memory(|memory| memory.has_focus(timeline_keyboard_focus_id())));
        assert_eq!(app.timeline.keyframes.len(), 1, "blur must not delete keyframes");
    }

    #[test]
    fn delete_key_removes_a_selected_cue_without_timeline_focus_but_not_during_text_editing() {
        let context = egui::Context::default();
        let mut app = PealayerApp::default();
        let effect = crate::four_d::models::Effect::new(
            "Delete shortcut".to_string(),
            String::new(),
            1_000,
            Vec::new(),
        );
        let effect_id = effect.id;
        app.timeline.templates.push(effect);
        let selected = crate::four_d::models::EffectInstance::new(effect_id, 1_000);
        app.selected_instance_ids.insert(selected.id);
        app.timeline.instances.push(selected);
        let delete_event = || egui::Event::Key {
            key: egui::Key::Delete,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };

        let output = context.run_ui(
            egui::RawInput {
                events: vec![delete_event()],
                ..Default::default()
            },
            |ui| {
                assert!(!ui
                    .ctx()
                    .memory(|memory| memory.has_focus(timeline_keyboard_focus_id())));
                assert!(delete_selected_cues_without_timeline_focus(&mut app, ui));
            },
        );
        discard_ui_output(output);
        assert!(app.timeline.instances.is_empty());

        let selected = crate::four_d::models::EffectInstance::new(effect_id, 2_000);
        app.selected_instance_ids.insert(selected.id);
        app.timeline.instances.push(selected);
        let mut text = String::from("keep editing");
        let output = context.run_ui(egui::RawInput::default(), |ui| {
            ui.text_edit_singleline(&mut text).request_focus();
        });
        discard_ui_output(output);
        let output = context.run_ui(
            egui::RawInput {
                events: vec![delete_event()],
                ..Default::default()
            },
            |ui| {
                ui.text_edit_singleline(&mut text);
                assert!(ui.ctx().text_edit_focused());
                assert!(!delete_selected_cues_without_timeline_focus(&mut app, ui));
            },
        );
        discard_ui_output(output);
        assert_eq!(app.timeline.instances.len(), 1);
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
    fn relay_identifiers_are_compact_for_raw_and_general_outputs() {
        assert_eq!(relay_identifier_label(1), "R1");
        assert_eq!(relay_identifier_label(4), "R4");
        assert_eq!(relay_identifier_label(5), "R5");
        assert_eq!(relay_identifier_label(8), "R8");
    }

    #[test]
    fn presentation_update_uses_canonical_seat_key_and_profile_revision() {
        let capabilities = crate::four_d::controller::HardwareCapabilities {
            board_profile: Some(crate::four_d::controller::HardwareBoardProfile {
                revision: "profile-revision".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let control = crate::four_d::controller::HardwareControl {
            key: "seat.a".to_string(),
            name: "Seat A".to_string(),
            ..Default::default()
        };

        assert_eq!(
            control_presentation_update_params(
                &capabilities,
                &control,
                serde_json::json!({
                    "name": "VIP Seat A",
                    "icon": "seat",
                    "color": "#A142F4",
                }),
            ),
            serde_json::json!({
                "key": "seat.a",
                "name": "VIP Seat A",
                "icon": "seat",
                "color": "#A142F4",
                "expected_revision": "profile-revision",
            })
        );
    }

    #[test]
    fn non_user_controls_include_raw_relays_and_role_specific_pwm() {
        let capabilities = crate::four_d::controller::HardwareCapabilities {
            pwm_channels: vec![crate::four_d::controller::HardwareOutput {
                id: 11,
                key: "pwm.11".to_string(),
                name: "Enclosure light".to_string(),
                role: "illumination".to_string(),
                control: "role-specific".to_string(),
            }],
            ..Default::default()
        };
        let raw_relay = crate::four_d::controller::HardwareControl {
            key: "relay.1".to_string(),
            kind: "relay".to_string(),
            ..Default::default()
        };
        let enclosure = crate::four_d::controller::HardwareControl {
            key: "pwm.11".to_string(),
            kind: "mosfet".to_string(),
            ..Default::default()
        };
        let user_pwm = crate::four_d::controller::HardwareControl {
            key: "pwm.0".to_string(),
            kind: "mosfet".to_string(),
            ..Default::default()
        };
        assert!(is_non_user_control(&capabilities, &raw_relay));
        assert!(is_non_user_control(&capabilities, &enclosure));
        assert!(!is_non_user_control(&capabilities, &user_pwm));
    }

    #[test]
    fn pwm_indicator_reads_the_live_slider_value_and_custom_color() {
        let context = egui::Context::default();
        let capabilities = crate::four_d::controller::HardwareCapabilities {
            pwm_channels: vec![crate::four_d::controller::HardwareOutput {
                id: 11,
                key: "pwm.11".to_string(),
                name: "Enclosure light".to_string(),
                role: "illumination".to_string(),
                control: "role-specific".to_string(),
            }],
            ..Default::default()
        };
        let control = crate::four_d::controller::HardwareControl {
            key: "pwm.11".to_string(),
            kind: "mosfet".to_string(),
            color: "#A142F4".to_string(),
            ..Default::default()
        };
        let sampled = std::cell::Cell::new(None);
        discard_ui_output(context.run_ui(Default::default(), |ui| {
            let value_id = ui.make_persistent_id(("pwm_value", 11_u8));
            ui.data_mut(|data| data.insert_temp(value_id, 2048_u16));
            sampled.set(pwm_control_intensity(ui, &capabilities, &control));
        }));
        assert!((sampled.get().unwrap() - 0.5).abs() < 0.001);
        assert_eq!(
            control_indicator_color(&control),
            egui::Color32::from_rgb(0xA1, 0x42, 0xF4)
        );
    }

    #[test]
    fn hardware_cards_keep_one_exact_width_across_cards_and_frames() {
        let context = egui::Context::default();
        let expected_width = 318.0;

        for _ in 0..4 {
            let mut widths = Vec::new();
            let output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(420.0, 480.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    ui.set_width(expected_width);
                    for (horizontal_margin, vertical_margin, corner_radius) in
                        [(14, 12, 10.0), (12, 10, 8.0), (9, 6, 7.0)]
                    {
                        ui.set_width(expected_width);
                        let card = egui::Frame::group(ui.style())
                            .inner_margin(egui::Margin::symmetric(
                                horizontal_margin,
                                vertical_margin,
                            ))
                            .stroke(egui::Stroke::new(
                                HARDWARE_CARD_STROKE_WIDTH,
                                ui.visuals().widgets.noninteractive.bg_stroke.color,
                            ))
                            .corner_radius(corner_radius)
                            .show(ui, |ui| {
                                ui.set_width(hardware_frame_content_width(
                                    expected_width,
                                    horizontal_margin,
                                ));
                                ui.horizontal(|ui| {
                                    ui.label(crate::ui::icons::PLUG);
                                    ui.label("Hardware control");
                                });
                            });
                        widths.push(card.response.rect.width());
                    }
                },
            );
            discard_ui_output(output);

            assert_eq!(widths.len(), 3);
            for width in widths {
                assert!(
                    (width - expected_width).abs() <= 0.1,
                    "hardware card requested {width}px instead of {expected_width}px"
                );
            }
        }
    }

    #[test]
    fn hardware_card_content_budget_includes_both_margins_and_strokes() {
        let outer_width = 318.0;
        for margin in [9, 12, 14] {
            let content = hardware_frame_content_width(outer_width, margin);
            let reconstructed = content + 2.0 * (f32::from(margin) + HARDWARE_CARD_STROKE_WIDTH);
            assert_eq!(reconstructed, outer_width);
        }
    }

    #[test]
    fn light_hardware_card_buttons_have_visible_surfaces_and_outlines() {
        let mut visuals = egui::Visuals::light();
        configure_hardware_card_visuals(&mut visuals);
        let widgets = visuals.widgets;
        assert!(visuals.button_frame);
        assert_eq!(
            widgets.inactive.weak_bg_fill,
            egui::Color32::from_rgb(227, 231, 236)
        );
        assert_eq!(
            widgets.hovered.weak_bg_fill,
            egui::Color32::from_rgb(217, 223, 230)
        );
        assert_eq!(
            widgets.active.weak_bg_fill,
            egui::Color32::from_rgb(206, 214, 223)
        );
        assert_eq!(widgets.open, widgets.active);
        assert_eq!(widgets.inactive.bg_stroke.width, 1.0);
        assert_ne!(widgets.inactive.bg_stroke.color, egui::Color32::TRANSPARENT);
        assert!(
            widgets.noninteractive.weak_bg_fill.r() > widgets.inactive.weak_bg_fill.r()
                && widgets.inactive.weak_bg_fill.r() > widgets.hovered.weak_bg_fill.r()
                && widgets.hovered.weak_bg_fill.r() > widgets.active.weak_bg_fill.r(),
            "light hardware buttons must progress from muted disabled to visibly pressed"
        );
        assert_ne!(
            widgets.noninteractive.weak_bg_fill, widgets.inactive.weak_bg_fill,
            "disabled and enabled buttons must remain visually distinguishable"
        );

        let mut dark = egui::Visuals::dark();
        let original_dark_widgets = dark.widgets.clone();
        configure_hardware_card_visuals(&mut dark);
        assert_eq!(dark.widgets, original_dark_widgets);
    }

    #[test]
    fn board_card_uses_one_centered_line_when_profile_is_the_only_identity() {
        let capabilities = crate::four_d::controller::HardwareCapabilities {
            board_profile: Some(crate::four_d::controller::HardwareBoardProfile {
                key: "cafe-cinema".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let (title, subtitle) = board_card_labels(
            crate::config::AppLanguage::English,
            &capabilities,
            "Connected board".to_string(),
        );

        assert_eq!(title, "Cafe Cinema");
        assert_eq!(subtitle, None);
    }

    #[test]
    fn board_identity_multiline_block_is_tight_and_vertically_centered() {
        let text_height = 20.0 + BOARD_IDENTITY_LINE_GAP + 15.0;
        let top_inset = (BOARD_IDENTITY_TWO_LINE_HEIGHT - text_height) / 2.0;
        let bottom_inset = BOARD_IDENTITY_TWO_LINE_HEIGHT - text_height - top_inset;
        assert_eq!(BOARD_IDENTITY_LINE_GAP, 0.0);
        assert!(top_inset > 0.0);
        assert_eq!(top_inset, bottom_inset);
    }

    #[test]
    fn board_identity_lines_share_the_exact_same_left_origin() {
        let rect = egui::Rect::from_min_size(egui::pos2(23.5, 10.0), egui::vec2(240.0, 42.0));
        let (title, subtitle) = board_identity_text_positions(rect, 20.0, Some(15.0));
        let subtitle = subtitle.expect("the two-line identity has a subtitle position");

        assert_eq!(title.x, rect.left());
        assert_eq!(subtitle.x, rect.left());
        assert_eq!(title.x, subtitle.x);
        assert_eq!(subtitle.y, title.y + 20.0 + BOARD_IDENTITY_LINE_GAP);
    }

    #[test]
    fn board_card_uses_distinct_profile_as_a_muted_second_line() {
        let mut capabilities = crate::four_d::controller::HardwareCapabilities {
            board_name: "Cinema controller".to_string(),
            board_profile: Some(crate::four_d::controller::HardwareBoardProfile {
                key: "cafe-cinema".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let (title, subtitle) = board_card_labels(
            crate::config::AppLanguage::English,
            &capabilities,
            "Connected board".to_string(),
        );
        assert_eq!(title, "Cinema controller");
        assert_eq!(subtitle.as_deref(), Some("Cafe Cinema"));

        capabilities.board_name = "Cafe Cinema".to_string();
        let (_, duplicate_subtitle) = board_card_labels(
            crate::config::AppLanguage::English,
            &capabilities,
            "Connected board".to_string(),
        );
        assert_eq!(duplicate_subtitle, None);
    }

    #[test]
    fn rf_codes_accept_hex_and_decimal_without_guessing_invalid_input() {
        assert_eq!(parse_rf_code("0x12AB34"), Some(0x12AB34));
        assert_eq!(parse_rf_code("1223476"), Some(1_223_476));
        assert_eq!(parse_rf_code(""), None);
        assert_eq!(parse_rf_code("not-a-code"), None);
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
    fn pwm_slider_and_input_fill_the_card_row_without_a_trailing_gap() {
        let row_width = 420.0;
        let gap = 8.0;
        let (slider, input) = pwm_editor_widths(row_width, gap);
        assert_eq!(slider + gap + input, row_width);
        assert_eq!(input, 76.0);
    }

    #[test]
    fn pwm_live_mode_transmits_changes_while_deferred_mode_waits_for_commit() {
        let changing = PwmEditorResponse {
            changed: true,
            committed: false,
            dragging: true,
        };
        assert!(changing.should_transmit(true));
        assert!(!changing.should_transmit(false));

        let committed = PwmEditorResponse {
            changed: false,
            committed: true,
            dragging: false,
        };
        assert!(!committed.should_transmit(true));
        assert!(committed.should_transmit(false));
    }

    #[test]
    fn pwm_drag_is_capped_at_thirty_hz_with_an_unthrottled_final_write() {
        let dragging = PwmEditorResponse {
            changed: true,
            committed: false,
            dragging: true,
        };
        assert!(!pwm_transmit_due(
            dragging,
            true,
            Some(std::time::Duration::from_millis(10)),
            true,
        ));
        assert!(pwm_transmit_due(
            dragging,
            true,
            Some(PWM_LIVE_INTERVAL),
            true,
        ));
        assert!(pwm_transmit_due(
            dragging,
            true,
            Some(PWM_STALE_INTERVAL),
            false,
        ));
        let release = PwmEditorResponse {
            changed: false,
            committed: true,
            dragging: false,
        };
        assert!(pwm_transmit_due(
            release,
            false,
            Some(std::time::Duration::ZERO),
            false,
        ));
    }

    #[test]
    fn status_rgb_channels_use_live_components_and_primary_indicator_colors() {
        let mut capabilities = crate::four_d::controller::HardwareCapabilities {
            status_led: Some(crate::four_d::controller::HardwareStatusLed {
                red: 18,
                green: 52,
                blue: 86,
                brightness: 120,
                effect: 0,
                condition: 0,
            }),
            ..Default::default()
        };
        capabilities.pwm_channels = [
            (13, "pwm.13", "status-red"),
            (14, "pwm.14", "status-green"),
            (15, "pwm.15", "status-blue"),
        ]
        .into_iter()
        .map(
            |(id, key, role)| crate::four_d::controller::HardwareOutput {
                id,
                key: key.to_string(),
                name: role.to_string(),
                role: role.to_string(),
                control: "role-specific".to_string(),
            },
        )
        .collect();
        for (key, value, color) in [
            ("pwm.13", 18, egui::Color32::from_rgb(239, 68, 68)),
            ("pwm.14", 52, egui::Color32::from_rgb(34, 197, 94)),
            ("pwm.15", 86, egui::Color32::from_rgb(59, 130, 246)),
        ] {
            let control = crate::four_d::controller::HardwareControl {
                key: key.to_string(),
                kind: "pwm".to_string(),
                ..Default::default()
            };
            assert_eq!(
                status_rgb_component(&capabilities, &control),
                Some((value, color))
            );
        }
    }

    #[test]
    fn pwm_wheel_consumes_raw_steps_and_the_smoothed_scroll_tail() {
        let wheel_event = egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, 12.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        };
        assert_eq!(
            pwm_wheel_gesture(&[wheel_event], egui::vec2(0.0, 12.0)),
            (1.0, true)
        );
        assert_eq!(
            pwm_wheel_gesture(&[], egui::vec2(0.0, 3.0)),
            (0.0, true),
            "a hovered PWM slider must also consume egui's residual smooth scroll"
        );
        assert_eq!(pwm_wheel_gesture(&[], egui::Vec2::ZERO), (0.0, false));
    }

    #[test]
    fn output_activation_selects_press_or_release_without_double_firing() {
        assert!(hardware_control_activation(true, true, true, false));
        assert!(!hardware_control_activation(true, false, false, true));
        assert!(!hardware_control_activation(false, true, true, false));
        assert!(hardware_control_activation(false, false, false, true));
    }

    #[test]
    fn hold_motion_captures_press_and_stops_only_on_physical_release() {
        assert_eq!(
            hold_motion_transition(None, "seat.a.up", true, true),
            HoldMotionTransition::Start
        );
        assert_eq!(
            hold_motion_transition(Some("seat.a.up"), "seat.a.up", false, true),
            HoldMotionTransition::None
        );
        assert_eq!(
            hold_motion_transition(Some("seat.a.up"), "seat.a.up", false, false),
            HoldMotionTransition::Stop
        );
    }

    #[test]
    fn seat_stop_visibility_requires_semantic_motion_state() {
        let mut capabilities = crate::four_d::controller::HardwareCapabilities::default();
        capabilities.relays = vec![
            crate::four_d::controller::HardwareOutput {
                id: 1,
                key: "relay.1".into(),
                name: "Left up".into(),
                role: "motion-left-up".into(),
                control: "relay".into(),
            },
            crate::four_d::controller::HardwareOutput {
                id: 3,
                key: "relay.3".into(),
                name: "Right up".into(),
                role: "motion-right-up".into(),
                control: "relay".into(),
            },
            crate::four_d::controller::HardwareOutput {
                id: 5,
                key: "relay.5".into(),
                name: "Aux".into(),
                role: "user-output".into(),
                control: "relay".into(),
            },
        ];
        let left = crate::four_d::controller::HardwareControl {
            key: "seat.a".into(),
            kind: "motion".into(),
            ..Default::default()
        };
        capabilities.active_relays.insert(3);
        assert!(!motion_control_is_active(&capabilities, &left));
        capabilities.active_relays.clear();
        capabilities.active_relays.insert(5);
        assert!(!motion_control_is_active(&capabilities, &left));
        capabilities.active_relays.insert(1);
        assert!(!motion_control_is_active(&capabilities, &left));
        capabilities.motion = Some(crate::four_d::controller::HardwareMotionState {
            left: crate::four_d::controller::HardwareMotionSide {
                requested: "up".into(),
                applied: "up".into(),
                transitioning: false,
                revision: 1,
            },
            ..Default::default()
        });
        assert!(motion_control_is_active(&capabilities, &left));
    }

    #[test]
    fn seat_indicator_does_not_infer_semantics_from_raw_relays() {
        let control = crate::four_d::controller::HardwareControl {
            key: "seat.a".into(),
            kind: "seat".into(),
            up_color: "#FF8800".into(),
            down_color: "#0088FF".into(),
            ..Default::default()
        };
        let mut capabilities = crate::four_d::controller::HardwareCapabilities::default();
        assert_eq!(
            motion_control_direction(&capabilities, &control),
            MotionDirectionState::Unknown
        );
        capabilities.active_relays.insert(2);
        assert_eq!(
            motion_control_direction(&capabilities, &control),
            MotionDirectionState::Unknown
        );
        capabilities.active_relays.insert(1);
        assert_eq!(
            motion_control_direction(&capabilities, &control),
            MotionDirectionState::Unknown
        );
    }

    #[test]
    fn seat_indicator_keeps_requested_direction_during_safe_reversal() {
        let control = crate::four_d::controller::HardwareControl {
            key: "seat.a".into(),
            kind: "seat".into(),
            ..Default::default()
        };
        let mut capabilities = crate::four_d::controller::HardwareCapabilities::default();
        capabilities.motion = Some(crate::four_d::controller::HardwareMotionState {
            left: crate::four_d::controller::HardwareMotionSide {
                requested: "down".into(),
                applied: "stop".into(),
                transitioning: true,
                revision: 12,
            },
            ..Default::default()
        });

        // The raw enable relay is deliberately off during break-before-make,
        // but the indicator must remain on the coordinator-confirmed intent.
        assert!(capabilities.active_relays.is_empty());
        assert_eq!(
            motion_control_direction(&capabilities, &control),
            MotionDirectionState::Down
        );

        let left = &mut capabilities.motion.as_mut().unwrap().left;
        left.applied = "down".into();
        left.transitioning = false;
        left.revision += 1;
        assert_eq!(
            motion_control_direction(&capabilities, &control),
            MotionDirectionState::Down
        );
    }

    #[test]
    fn motion_stop_removal_does_not_activate_caption_or_rename_button() {
        for compact in [false, true] {
            let context = egui::Context::default();
            let show_stop = std::cell::Cell::new(true);
            let stop_rect = std::cell::Cell::new(egui::Rect::NOTHING);
            let caption_rect = std::cell::Cell::new(egui::Rect::NOTHING);
            let rename_rect = std::cell::Cell::new(egui::Rect::NOTHING);
            let stop_count = std::cell::Cell::new(0);
            let rename_count = std::cell::Cell::new(0);
            let render = |events| {
                discard_ui_output(context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(380.0, 100.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        ui.set_width(280.0);
                        let caption = |ui: &mut egui::Ui, width| {
                            let response = hardware_header_widget(ui, "seat.a", "caption", |ui| {
                                left_aligned_click_label(ui, "Seat A", width, 24.0, 13.0)
                            });
                            caption_rect.set(response.rect);
                            if response.clicked() {
                                rename_count.set(rename_count.get() + 1);
                            }
                        };
                        let rename = |ui: &mut egui::Ui| {
                            let response = hardware_header_widget(ui, "seat.a", "rename", |ui| {
                                ui.button("Edit")
                            });
                            rename_rect.set(response.rect);
                            if response.clicked() {
                                rename_count.set(rename_count.get() + 1);
                            }
                        };
                        let stop = |ui: &mut egui::Ui| {
                            if show_stop.get() {
                                let response = hardware_header_widget(ui, "seat.a", "stop", |ui| {
                                    ui.button("Stop")
                                });
                                stop_rect.set(response.rect);
                                if hardware_control_activation(
                                    true,
                                    response.is_pointer_button_down_on(),
                                    ui.input(|input| {
                                        input.pointer.button_pressed(egui::PointerButton::Primary)
                                    }),
                                    response.clicked(),
                                ) {
                                    stop_count.set(stop_count.get() + 1);
                                }
                            }
                        };
                        ui.horizontal(|ui| {
                            if compact {
                                caption(ui, 120.0);
                                stop(ui);
                                rename(ui);
                            } else {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        rename(ui);
                                        stop(ui);
                                        caption(ui, ui.available_width());
                                    },
                                );
                            }
                        });
                    },
                ));
            };
            render(Vec::new());
            render(Vec::new());
            let point = stop_rect.get().center();
            render(vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
            assert_eq!(stop_count.get(), 1, "Stop must dispatch on press");
            // Board acknowledgment removes Stop before the mouse is released.
            show_stop.set(false);
            render(vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }]);
            assert_eq!(
                rename_count.get(),
                0,
                "compact={compact}: Stop release must not rename"
            );
            assert_eq!(stop_count.get(), 1);
            render(Vec::new());
            let point = caption_rect.get().center();
            render(vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
            render(vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }]);
            assert_eq!(rename_count.get(), 1, "Caption rename must still work");
            render(Vec::new());
            let point = rename_rect.get().center();
            render(vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
            render(vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }]);
            assert_eq!(rename_count.get(), 2, "Rename button must still work");
        }
    }

    #[test]
    fn semantic_seat_stop_is_inline_and_only_present_while_that_seat_is_active() {
        let mut capabilities = crate::four_d::controller::HardwareCapabilities::default();
        let control = crate::four_d::controller::HardwareControl {
            key: "seat.a".into(),
            kind: "seat".into(),
            control: "seat".into(),
            actions: ["up", "down", "stop"]
                .into_iter()
                .map(|verb| crate::four_d::controller::HardwareAction {
                    verb: verb.into(),
                    name: verb.into(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };

        assert_eq!(
            card_control_actions(&control)
                .into_iter()
                .map(|action| action.verb.as_str())
                .collect::<Vec<_>>(),
            ["up", "down"]
        );
        assert!(contextual_stop_action(&capabilities, &control).is_none());

        capabilities.motion = Some(crate::four_d::controller::HardwareMotionState {
            left: crate::four_d::controller::HardwareMotionSide {
                requested: "up".into(),
                applied: "up".into(),
                transitioning: false,
                revision: 1,
            },
            ..Default::default()
        });
        assert_eq!(
            card_control_actions(&control)
                .into_iter()
                .map(|action| action.verb.as_str())
                .collect::<Vec<_>>(),
            ["up", "down"]
        );
        assert_eq!(
            contextual_stop_action(&capabilities, &control).map(|action| action.verb.as_str()),
            Some("stop")
        );
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
    fn controller_effects_keep_distinct_timeline_lanes_while_offline() {
        use crate::four_d::models::{ControllerEffectLane, Effect};

        let mut app = PealayerApp::default();
        let mut motion = Effect::controller_macro(
            "Seat rise".to_string(),
            String::new(),
            1_000,
            1,
            "host".to_string(),
        );
        motion.controller_lane = Some(ControllerEffectLane::Motion);
        let lighting =
            Effect::controller_strip_effect("Police".to_string(), 3_000, "police".to_string());
        app.timeline.templates.extend([motion, lighting]);

        let rows = timeline_track_rows(&app);
        assert!(rows.iter().any(|row| {
            row.kind == TimelineTrackKind::ControllerEffect(ControllerEffectLane::Motion)
        }));
        assert!(rows.iter().any(|row| {
            row.kind == TimelineTrackKind::ControllerEffect(ControllerEffectLane::Lighting)
        }));
        assert!(!rows.iter().any(|row| {
            row.kind == TimelineTrackKind::ControllerEffect(ControllerEffectLane::Sequence)
        }));
    }

    #[test]
    fn effect_library_metadata_is_neutral_right_aligned_and_stable() {
        for dark in [false, true] {
            for width in [220.0, 340.0] {
                let context = egui::Context::default();
                context.set_visuals(if dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
                let mut previous_sizes = None;
                for _ in 0..3 {
                    let mut sizes = Vec::new();
                    let mut badge_fill = egui::Color32::TRANSPARENT;
                    let output = context.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(500.0, 260.0),
                            )),
                            ..Default::default()
                        },
                        |ui| {
                            badge_fill = ui.visuals().widgets.inactive.weak_bg_fill;
                            ui.set_width(width);
                            for kind in
                                ["Timed sequence", "A long user-facing effect classification"]
                            {
                                let card = effect_card(ui, width, |ui| {
                                    let right = ui.max_rect().right();
                                    let (badge, duration) =
                                        effect_library_metadata(ui, kind, "1 min 5 sec");
                                    assert!(
                                        (duration.right() - right).abs() < 0.1,
                                        "duration={duration:?}, right={right}, badge={badge:?}"
                                    );
                                    assert!(badge.right() < duration.left());
                                });
                                assert!((card.response.rect.width() - width).abs() < 0.1);
                                sizes.push(card.response.rect.size());
                            }
                        },
                    );
                    assert_eq!(sizes[0], sizes[1]);
                    if let Some(previous) = previous_sizes.as_ref() {
                        assert_eq!(&sizes, previous);
                    }
                    previous_sizes = Some(sizes);
                    let mut rects = Vec::new();
                    for shape in &output.shapes {
                        if let egui::Shape::Rect(rect) = &shape.shape {
                            rects.push(rect);
                        }
                    }
                    let badges: Vec<_> = rects
                        .iter()
                        .filter(|rect| rect.corner_radius.nw == 11)
                        .collect();
                    assert_eq!(badges.len(), 2);
                    assert!(badges.iter().all(|rect| rect.fill == badge_fill));
                    discard_ui_output(output);
                }
            }
        }
    }

    #[test]
    fn effect_cards_keep_identical_geometry_across_rows_and_frames() {
        let context = egui::Context::default();
        let payload = EffectDragPayload {
            name: "Seat rise".to_string(),
            icon: String::new(),
            duration_ms: 750,
            target: crate::four_d::models::HardwareTarget::ControllerMacro,
            actions: Vec::new(),
            controller_macro: None,
            controller_strip_effect: None,
            controller_lane: None,
        };
        let previous = std::cell::RefCell::new(None::<Vec<egui::Vec2>>);

        for _ in 0..4 {
            let sizes = std::cell::RefCell::new(Vec::new());
            let output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(360.0, 320.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    let width = effects_panel_content_width(ui.available_width());
                    ui.set_width(width);
                    for row in 0..3 {
                        let response = effect_drag_source(
                            ui,
                            egui::Id::new(("stable-effect-card", row)),
                            payload.clone(),
                            |ui| {
                                effect_card(ui, width, |ui| {
                                    ui.add_sized(
                                        [ui.available_width(), 44.0],
                                        egui::Label::new("Seat rise"),
                                    )
                                })
                            },
                        );
                        sizes.borrow_mut().push(response.response.rect.size());
                        ui.add_space(5.0);
                    }
                },
            );
            discard_ui_output(output);

            let sizes = sizes.into_inner();
            assert_eq!(sizes.len(), 3);
            assert!(sizes.windows(2).all(|pair| pair[0] == pair[1]));
            if let Some(previous) = previous.borrow().as_ref() {
                assert_eq!(&sizes, previous);
            }
            previous.replace(Some(sizes));
        }
    }

    #[test]
    fn effect_group_headers_and_cards_share_the_same_outer_width() {
        let outer = effects_panel_content_width(327.0);
        let content = effects_frame_content_width(outer);
        assert_eq!(outer, 317.0);
        assert_eq!(content, 297.0);
        assert_eq!(
            content + (f32::from(EFFECT_CARD_HORIZONTAL_MARGIN) + EFFECT_CARD_STROKE_WIDTH) * 2.0,
            outer
        );

        let context = egui::Context::default();
        let painted_widths = std::cell::Cell::new((0.0_f32, 0.0_f32));
        let output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(327.0, 180.0),
                )),
                ..Default::default()
            },
            |ui| {
                let header = effect_group_header(ui, outer, |ui| {
                    ui.label("Lighting");
                });
                let card = effect_card(ui, outer, |ui| {
                    ui.add_sized(
                        [ui.available_width(), 24.0],
                        egui::Label::new("White thunder").halign(egui::Align::Min),
                    );
                });
                painted_widths.set((header.response.rect.width(), card.response.rect.width()));
            },
        );
        discard_ui_output(output);
        let (header_width, card_width) = painted_widths.get();
        assert_eq!(header_width, outer);
        assert_eq!(card_width, outer);
        assert_eq!(header_width, card_width);
    }

    #[test]
    fn empty_effect_group_has_no_disclosure_chevron() {
        assert_eq!(effect_group_disclosure_icon(true, 0), None);
        assert_eq!(effect_group_disclosure_icon(false, 0), None);
        assert_eq!(
            effect_group_disclosure_icon(true, 1),
            Some(crate::ui::icons::CARET_DOWN)
        );
        assert_eq!(
            effect_group_disclosure_icon(false, 1),
            Some(crate::ui::icons::CARET_RIGHT)
        );
    }

    #[test]
    fn effect_group_add_button_click_does_not_collapse_the_group() {
        for dark in [false, true] {
            let context = egui::Context::default();
            context.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            let frame = |events| {
                let actions =
                    std::cell::Cell::new((false, false, egui::Rect::NOTHING, egui::Rect::NOTHING));
                let output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(400.0, 200.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        let (title, add) = effect_group_action_header(
                            ui,
                            317.0,
                            "Cinema lighting with a long caption",
                            crate::ui::icons::FOLDER_OPEN,
                            true,
                            12,
                            "New effect in this group",
                        );
                        actions.set((title.clicked(), add.clicked(), title.rect, add.rect));
                        assert!(ui.min_rect().width() <= 317.0 + f32::EPSILON);
                    },
                );
                discard_ui_output(output);
                actions.get()
            };
            let (_, _, title, add) = frame(Vec::new());
            assert!(title.right() <= add.left());
            for (position, expected) in [
                (add.center(), (false, true)),
                (title.center(), (true, false)),
            ] {
                frame(vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]);
                let (toggle, create, _, _) = frame(vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }]);
                assert_eq!((toggle, create), expected);
            }
        }
    }

    #[test]
    fn effect_library_header_actions_anchor_right_for_short_and_long_titles() {
        for dark in [false, true] {
            for width in [EFFECT_CARD_MIN_WIDTH, 180.0, 320.0, 520.0] {
                let context = egui::Context::default();
                context.set_visuals(if dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
                let geometry = std::cell::Cell::new((
                    egui::Rect::NOTHING,
                    egui::Rect::NOTHING,
                    egui::Rect::NOTHING,
                    0.0,
                ));
                for title in [
                    "FX",
                    "An unusually long effect title that must truncate rather than move its actions",
                ] {
                    let output = context.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(700.0, 160.0),
                            )),
                            ..Default::default()
                        },
                        |ui| {
                            effect_card(ui, width, |ui| {
                                let end = ui.max_rect().right();
                                let header = effect_library_card_header(
                                    ui,
                                    egui::Id::new("header-anchor"),
                                    crate::ui::icons::SPARKLE,
                                    title,
                                    crate::ui::icons::PLAY,
                                    true,
                                    ["Drag", "Icon", "Rename", "Run", "Place", "More", "Rename"],
                                );
                                geometry.set((
                                    header.grip.rect,
                                    header.run.rect,
                                    header.more.rect,
                                    end,
                                ));
                            });
                        },
                    );
                    discard_ui_output(output);
                    let (grip, run, more, end) = geometry.get();
                    assert!(
                        grip.is_positive(),
                        "drag indicator must occupy its own stable slot"
                    );
                    assert!(
                        grip.right() < run.left(),
                        "grip must not overlap action buttons"
                    );
                    assert!(
                        (more.right() - end).abs() < 0.1,
                        "actions must reach the card's trailing edge: width={width}, title={title:?}, actual={}, expected={end}",
                        more.right()
                    );
                }
            }
        }
    }

    #[test]
    fn production_effect_action_row_stays_within_the_card_width() {
        let spacing = 8.0;
        for available in [112.0, 180.0, 297.0] {
            let (title, actions) = effect_card_header_widths(available, spacing);
            assert!(title + spacing + actions <= available + f32::EPSILON);
            assert_eq!(actions, 120.0_f32.min(available - spacing - 12.0));
        }

        let context = egui::Context::default();
        let widths = std::cell::Cell::new((0.0_f32, 0.0_f32));
        let output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(327.0, 220.0),
                )),
                ..Default::default()
            },
            |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let outer = effects_panel_content_width(ui.available_width());
                    let header = effect_group_header(ui, outer, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(crate::ui::icons::CARET_DOWN);
                            ui.label(crate::ui::icons::FOLDER_OPEN);
                            ui.label("Cinema");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label("3");
                                },
                            );
                        });
                    });
                    let card = effect_card(ui, outer, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_sized([20.0, 24.0], egui::Label::new("effect"));
                            let spacing = ui.spacing().item_spacing.x;
                            let (title, actions) =
                                effect_card_header_widths(ui.available_width(), spacing);
                            ui.add_sized([title, 24.0], egui::Label::new("Seat rise").truncate());
                            ui.allocate_ui_with_layout(
                                egui::vec2(actions, 24.0),
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    for label in ["more", "add", "play", "rename"] {
                                        ui.add_sized([24.0, 24.0], egui::Button::new(label));
                                    }
                                },
                            );
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Timed multi-peripheral sequence");
                            ui.label("1 min 30 sec");
                        });
                    });
                    widths.set((header.response.rect.width(), card.response.rect.width()));
                    ui.allocate_space(egui::vec2(1.0, 400.0));
                });
            },
        );
        discard_ui_output(output);
        let (header, card) = widths.get();
        assert_eq!(header, card);
    }

    #[test]
    fn effect_controls_cards_are_stable_and_never_wider_than_the_panel() {
        let context = egui::Context::default();
        for available in [168.0_f32, 280.0, 420.0] {
            let painted = std::cell::RefCell::new(Vec::new());
            let output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(available, 420.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    let width = effect_controls_content_width(ui.available_width());
                    ui.set_width(width);
                    for row in 0..3 {
                        let card = effect_controls_card(
                            ui,
                            width,
                            crate::ui::icons::CLOCK,
                            &format!("Card {row}"),
                            Some("Responsive inspector section"),
                            row == 0,
                            |ui| {
                                ui.add_sized(
                                    [ui.available_width(), 24.0],
                                    egui::Label::new("Content"),
                                );
                            },
                        );
                        painted.borrow_mut().push(card.response.rect.width());
                    }
                },
            );
            discard_ui_output(output);
            let painted = painted.into_inner();
            assert_eq!(painted.len(), 3);
            assert!(painted.windows(2).all(|pair| pair[0] == pair[1]));
            assert!(painted[0] <= available + f32::EPSILON);
            assert_eq!(painted[0], effect_controls_content_width(available));
        }
    }

    #[test]
    fn effect_controls_identify_each_authoritative_effect_kind() {
        let relay = crate::four_d::models::Effect::with_target(
            "Seat".to_string(),
            String::new(),
            500,
            crate::four_d::models::HardwareTarget::Relay(5),
            vec![],
        );
        let macro_effect = crate::four_d::models::Effect::controller_macro(
            "Motion".to_string(),
            String::new(),
            1_000,
            17,
            "host".to_string(),
        );
        let strip = crate::four_d::models::Effect::controller_strip_effect(
            "Thunder".to_string(),
            2_000,
            "thunder".to_string(),
        );
        let direct = crate::four_d::models::Effect::direct_control(
            "House light".to_string(),
            String::new(),
            1_000,
            "pwm.12".to_string(),
            5_000,
            None,
        );

        assert_eq!(effect_controls_kind(&relay).1, "Relay sequence");
        assert_eq!(effect_controls_kind(&macro_effect).1, "Hardware macro");
        assert_eq!(effect_controls_kind(&strip).1, "Addressable lighting");
        assert_eq!(effect_controls_kind(&direct).1, "Direct channel cue");
    }

    #[test]
    fn effect_card_action_click_survives_the_drag_surface() {
        for action in 0..4 {
            let context = egui::Context::default();
            let payload = EffectDragPayload {
                name: "Seat rise".to_string(),
                icon: String::new(),
                duration_ms: 750,
                target: crate::four_d::models::HardwareTarget::ControllerMacro,
                actions: Vec::new(),
                controller_macro: None,
                controller_strip_effect: None,
                controller_lane: None,
            };
            let action_rect = std::cell::Cell::new(egui::Rect::NOTHING);
            let activated = std::cell::Cell::new(false);
            let render = |events: Vec<egui::Event>| {
                let output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(360.0, 180.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        let mut button_response = None;
                        effect_drag_source_with_action_gutter(
                            ui,
                            egui::Id::new("effect-card-with-action"),
                            payload.clone(),
                            EFFECT_CARD_ACTION_GUTTER,
                            |ui| {
                                effect_card(ui, 260.0, |ui| {
                                    let header = effect_library_card_header(
                                        ui,
                                        egui::Id::new("effect-card-with-action"),
                                        crate::ui::icons::SPARKLE,
                                        "Seat rise",
                                        crate::ui::icons::PLAY,
                                        true,
                                        ["Drag", "Icon", "Rename", "Run", "Place", "More", "Rename"],
                                    );
                                    let button = match action {
                                        0 => header.run,
                                        1 => header.place,
                                        2 => header.more,
                                        _ => header.rename,
                                    };
                                    action_rect.set(button.rect);
                                    button_response = Some(button);
                                })
                            },
                        );
                        activated.set(
                            activated.get()
                                || button_response
                                    .as_ref()
                                    .is_some_and(egui::Response::clicked),
                        );
                    },
                );
                discard_ui_output(output);
            };

            render(Vec::new());
            let point = action_rect.get().center();
            render(vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
            render(vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }]);

            assert!(
                activated.get(),
                "the card drag surface swallowed its action click"
            );
            assert!(egui::DragAndDrop::payload::<EffectDragPayload>(&context).is_none());
        }
    }

    #[test]
    fn effect_card_context_rename_request_is_scoped_and_consumed_once() {
        let context = egui::Context::default();
        let first = egui::Id::new("sequence-effect-card");
        let second = egui::Id::new("strip-effect-card");
        request_effect_card_rename(&context, second);
        assert!(!context.data_mut(|data| data.remove_temp::<bool>(first.with("rename-request")).unwrap_or(false)));
        assert!(context.data_mut(|data| data.remove_temp::<bool>(second.with("rename-request")).unwrap_or(false)));
        assert!(!context.data_mut(|data| data.remove_temp::<bool>(second.with("rename-request")).unwrap_or(false)));
    }

    #[test]
    fn effect_card_rename_input_reserves_confirm_cancel_without_widening() {
        for available in [64.0, 98.0, 220.0, 460.0] {
            for spacing in [2.0, 4.0] {
                let name = effect_card_rename_name_width(available, spacing);
                assert_eq!(name + 48.0 + spacing * 2.0, available);
            }
        }
    }

    #[test]
    fn effect_library_grip_starts_the_existing_drag_without_a_grab_offset_jump() {
        let context = egui::Context::default();
        let id = egui::Id::new("production-library-grip");
        let payload = EffectDragPayload {
            name: "Grip fixture".into(),
            icon: String::new(),
            duration_ms: 1000,
            target: crate::four_d::models::HardwareTarget::ControllerMacro,
            actions: Vec::new(),
            controller_macro: None,
            controller_strip_effect: None,
            controller_lane: None,
        };
        let geometry = std::cell::Cell::new((egui::Rect::NOTHING, egui::Rect::NOTHING));
        let render = |events| {
            let output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(440.0, 200.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let response = effect_drag_source_with_action_gutter(
                        ui,
                        id,
                        payload.clone(),
                        EFFECT_CARD_ACTION_GUTTER,
                        |ui| {
                            effect_card(ui, 320.0, |ui| {
                                effect_library_card_header(
                                    ui,
                                    id,
                                    crate::ui::icons::SPARKLE,
                                    "Grip fixture",
                                    crate::ui::icons::PLAY,
                                    true,
                                    ["Drag", "Icon", "Rename", "Run", "Place", "More", "Rename"],
                                )
                            })
                        },
                    );
                    geometry.set((response.response.rect, response.inner.inner.grip.rect));
                },
            );
            discard_ui_output(output);
        };
        render(vec![]);
        let (card, grip) = geometry.get();
        let press = grip.center();
        render(vec![
            egui::Event::PointerMoved(press),
            egui::Event::PointerButton {
                pos: press,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        let expected_offset = press - card.min;
        assert_eq!(
            context.data(|data| data.get_temp::<egui::Vec2>(id.with("pointer-offset"))),
            Some(expected_offset)
        );
        render(vec![egui::Event::PointerMoved(
            press + egui::vec2(48.0, 20.0),
        )]);
        render(vec![]);
        assert!(
            egui::DragAndDrop::payload::<EffectDragPayload>(&context).is_some(),
            "grip must start the real effect drag"
        );
        assert_eq!(
            context.data(|data| data.get_temp::<egui::Vec2>(id.with("pointer-offset"))),
            Some(expected_offset)
        );
    }

    #[test]
    fn effect_card_captures_grab_offset_on_press_before_drag_promotion() {
        let context = egui::Context::default();
        let payload = EffectDragPayload {
            name: "Seat rise".to_string(),
            icon: String::new(),
            duration_ms: 750,
            target: crate::four_d::models::HardwareTarget::ControllerMacro,
            actions: Vec::new(),
            controller_macro: None,
            controller_strip_effect: None,
            controller_lane: None,
        };
        let source = std::cell::Cell::new(egui::Rect::NOTHING);
        let id = egui::Id::new("press-offset-effect-card");
        let render = |events: Vec<egui::Event>| {
            let output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(360.0, 180.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    source.set(
                        effect_drag_source(ui, id, payload.clone(), |ui| {
                            ui.add_sized([180.0, 48.0], egui::Label::new("Seat rise"))
                        })
                        .response
                        .rect,
                    );
                },
            );
            discard_ui_output(output);
        };

        render(Vec::new());
        let grab_offset = egui::vec2(23.0, 9.0);
        let press = source.get().min + grab_offset;
        render(vec![
            egui::Event::PointerMoved(press),
            egui::Event::PointerButton {
                pos: press,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);

        let captured =
            context.data_mut(|data| data.get_temp::<egui::Vec2>(id.with("pointer-offset")));
        assert_eq!(captured, Some(grab_offset));
    }

    #[test]
    fn effect_card_keeps_the_original_grab_offset_while_dragging() {
        let context = egui::Context::default();
        let payload = EffectDragPayload {
            name: "Seat rise".to_string(),
            icon: String::new(),
            duration_ms: 750,
            target: crate::four_d::models::HardwareTarget::ControllerMacro,
            actions: Vec::new(),
            controller_macro: None,
            controller_strip_effect: None,
            controller_lane: None,
        };
        let source = std::cell::Cell::new(egui::Rect::NOTHING);
        let id = egui::Id::new("stable-offset-effect-card");
        let render = |events: Vec<egui::Event>| {
            let output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(640.0, 320.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    source.set(
                        effect_drag_source(ui, id, payload.clone(), |ui| {
                            effect_card(ui, 240.0, |ui| {
                                ui.add_sized(
                                    [ui.available_width(), 44.0],
                                    egui::Label::new("Seat rise"),
                                )
                            })
                        })
                        .response
                        .rect,
                    );
                },
            );
            discard_ui_output(output);
        };

        render(Vec::new());
        let expected = egui::vec2(31.0, 12.0);
        let press = source.get().min + expected;
        render(vec![
            egui::Event::PointerMoved(press),
            egui::Event::PointerButton {
                pos: press,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        render(vec![egui::Event::PointerMoved(
            press + egui::vec2(30.0, 4.0),
        )]);
        render(vec![egui::Event::PointerMoved(
            press + egui::vec2(260.0, 120.0),
        )]);

        let captured =
            context.data_mut(|data| data.get_temp::<egui::Vec2>(id.with("pointer-offset")));
        assert_eq!(captured, Some(expected));
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
            controller_lane: Some(crate::four_d::models::ControllerEffectLane::Sequence),
        };
        begin_effect_drag(&context, payload.clone());
        assert_eq!(
            egui::DragAndDrop::payload::<EffectDragPayload>(&context).as_deref(),
            Some(&payload)
        );
    }

    #[test]
    fn controller_effect_reference_is_stable_when_view_models_are_rebuilt() {
        let first = crate::app::EffectPreset {
            category: "Cinema".to_string(),
            group_icon: String::new(),
            source: crate::app::EffectPresetSource::ControllerMacro(7),
            effect: crate::four_d::models::Effect::controller_macro(
                "Seat rise".to_string(),
                String::new(),
                750,
                7,
                "mcu".to_string(),
            ),
        };
        let second = crate::app::EffectPreset {
            category: "Cinema".to_string(),
            group_icon: String::new(),
            source: crate::app::EffectPresetSource::ControllerMacro(7),
            effect: crate::four_d::models::Effect::controller_macro(
                "Seat rise".to_string(),
                String::new(),
                750,
                7,
                "mcu".to_string(),
            ),
        };

        assert_ne!(first.effect.id, second.effect.id);
        assert_eq!(
            effect_preset_reference(&first),
            effect_preset_reference(&second)
        );
        assert_eq!(
            effect_preset_reference(&first).as_deref(),
            Some("effect:7")
        );

        let strip = crate::app::EffectPreset {
            category: "Lighting".to_string(),
            group_icon: String::new(),
            source: crate::app::EffectPresetSource::ControllerStrip,
            effect: crate::four_d::models::Effect::controller_strip_effect(
                "Aurora".to_string(),
                5_000,
                "aurora".to_string(),
            ),
        };
        assert_eq!(
            effect_preset_reference(&strip).as_deref(),
            Some("effect:aurora")
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
            controller_lane: Some(crate::four_d::models::ControllerEffectLane::Sequence),
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
            discard_ui_output(output);
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
    fn effect_card_drag_survives_the_real_scroll_area_gesture_competition() {
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
            controller_lane: Some(crate::four_d::models::ControllerEffectLane::Sequence),
        };
        let source = std::cell::Cell::new(egui::Rect::NOTHING);
        let target = std::cell::Cell::new(egui::Rect::NOTHING);
        let dropped = std::cell::Cell::new(false);
        let render = |events: Vec<egui::Event>| {
            let output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(620.0, 260.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    ui.horizontal_top(|ui| {
                        ui.allocate_ui(egui::vec2(230.0, 230.0), |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("verified-effects-scroll")
                                .scroll_source(egui::scroll_area::ScrollSource::ALL)
                                .show(ui, |ui| {
                                    source.set(
                                        effect_drag_source(
                                            ui,
                                            egui::Id::new("verified-scrolled-effect-card"),
                                            payload.clone(),
                                            |ui| {
                                                egui::Frame::group(ui.style())
                                                    .inner_margin(egui::Margin::same(8))
                                                    .show(ui, |ui| {
                                                        ui.set_min_width(180.0);
                                                        ui.horizontal(|ui| {
                                                            ui.label("icon");
                                                            ui.label("Seat rise");
                                                        });
                                                    })
                                            },
                                        )
                                        .response
                                        .rect,
                                    );
                                    // Make the ScrollArea actually scrollable so it
                                    // registers its own competing drag response.
                                    ui.add_space(500.0);
                                });
                        });
                        ui.add_space(24.0);
                        let (_, timeline_response) = ui.allocate_exact_size(
                            egui::vec2(300.0, 180.0),
                            egui::Sense::click_and_drag(),
                        );
                        target.set(timeline_response.rect);
                        let released = take_effect_drop_on_rect(ui.ctx(), timeline_response.rect);
                        dropped.set(dropped.get() || released.as_deref() == Some(&payload));
                    });
                },
            );
            discard_ui_output(output);
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
            source_point + egui::vec2(20.0, 3.0),
        )]);
        assert_eq!(
            egui::DragAndDrop::payload::<EffectDragPayload>(&context).as_deref(),
            Some(&payload),
            "the scrollable card never published its drag payload"
        );
        let target_point = target.get().center();
        render(vec![egui::Event::PointerMoved(target_point)]);
        assert_eq!(
            egui::DragAndDrop::payload::<EffectDragPayload>(&context).as_deref(),
            Some(&payload),
            "the payload disappeared while crossing from the library to the timeline"
        );
        render(vec![egui::Event::PointerButton {
            pos: target_point,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);

        assert!(
            dropped.get(),
            "a scrollable Effects Library card did not reach the timeline drop zone"
        );
    }

    #[test]
    fn nested_card_context_trigger_uses_the_whole_item_rectangle() {
        let context = egui::Context::default();
        let card = egui::Rect::from_min_max(egui::pos2(20.0, 20.0), egui::pos2(220.0, 90.0));
        let point = egui::pos2(120.0, 55.0);
        let mut clicked_inside = false;
        for events in [
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Secondary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        ] {
            let output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(320.0, 160.0),
                    )),
                    events,
                    ..Default::default()
                },
                |_ui| {
                    clicked_inside |= secondary_click_inside(&context, card);
                },
            );
            discard_ui_output(output);
        }
        assert!(clicked_inside);
    }

    #[test]
    fn secondary_effect_card_gesture_only_opens_context_menu_and_never_drags() {
        let context = egui::Context::default();
        let payload = EffectDragPayload {
            name: "Seat rise".to_string(),
            icon: String::new(),
            duration_ms: 750,
            target: crate::four_d::models::HardwareTarget::ControllerMacro,
            actions: Vec::new(),
            controller_macro: None,
            controller_strip_effect: None,
            controller_lane: None,
        };
        let source = std::cell::Cell::new(egui::Rect::NOTHING);
        let menu_requested = std::cell::Cell::new(false);
        let id = egui::Id::new("secondary-only-effect-card");
        let render = |events: Vec<egui::Event>| {
            let output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(360.0, 180.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let response = effect_drag_source(ui, id, payload.clone(), |ui| {
                        ui.add_sized([180.0, 48.0], egui::Label::new("Seat rise"))
                    });
                    source.set(response.response.rect);
                    if secondary_click_inside(&context, response.response.rect) {
                        menu_requested.set(true);
                    }
                },
            );
            discard_ui_output(output);
        };

        render(Vec::new());
        let press = source.get().center();
        render(vec![
            egui::Event::PointerMoved(press),
            egui::Event::PointerButton {
                pos: press,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        let release = press + egui::vec2(18.0, 4.0);
        render(vec![egui::Event::PointerMoved(release)]);

        assert!(
            egui::DragAndDrop::payload::<EffectDragPayload>(&context).is_none(),
            "moving a held secondary button published an effect drag payload"
        );
        assert!(
            context
                .data_mut(|data| data.get_temp::<egui::Vec2>(id.with("pointer-offset")))
                .is_none(),
            "secondary press captured primary-drag state"
        );

        render(vec![egui::Event::PointerButton {
            pos: release,
            button: egui::PointerButton::Secondary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);

        assert!(
            menu_requested.get(),
            "secondary release did not request the menu"
        );
        assert!(
            egui::DragAndDrop::payload::<EffectDragPayload>(&context).is_none(),
            "secondary context-menu gesture left an effect drag payload behind"
        );
    }

    #[test]
    fn indicators_use_authoritative_output_state() {
        let mut capabilities = crate::four_d::controller::HardwareCapabilities::default();
        capabilities.active_relays.insert(2);
        capabilities.motion = Some(crate::four_d::controller::HardwareMotionState {
            left: crate::four_d::controller::HardwareMotionSide {
                requested: "up".into(),
                applied: "up".into(),
                transitioning: false,
                revision: 1,
            },
            ..Default::default()
        });
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
            control_indicator_state(&capabilities, &control("seat.a", "motion")),
            ControlIndicatorState::Active
        );
        assert_eq!(
            control_indicator_state(&capabilities, &control("pwm.1", "mosfet")),
            ControlIndicatorState::Unknown
        );
    }

    #[test]
    fn timeline_track_picker_dims_every_track_that_is_not_actually_shown() {
        let row = |linked, visible| TimelineTrackRow {
            key: "hardware:relay.5".to_string(),
            name: "User Relay 5".to_string(),
            detail: None,
            active: false,
            enabled: true,
            linked,
            visible,
            icon: crate::ui::icons::PLUG.to_string(),
            control_key: Some("relay.5".to_string()),
            relay_ids: vec![5],
            dimmed: false,
            kind: TimelineTrackKind::Relay(5),
        };

        assert!(!timeline_track_picker_item_is_dimmed(&row(true, true)));
        assert!(timeline_track_picker_item_is_dimmed(&row(true, false)));
        assert!(timeline_track_picker_item_is_dimmed(&row(false, true)));
        assert!(timeline_track_picker_item_is_dimmed(&row(false, false)));
    }

    #[test]
    fn test_dropped_effect_hover_cursor_interaction() {
        let mut app = PealayerApp::default();
        app.duration = 60.0;
        app.timeline_zoom = 100.0;
        let mut capabilities = crate::four_d::controller::HardwareCapabilities::default();
        capabilities.board_connected = true;
        capabilities.relays = vec![
            crate::four_d::controller::HardwareOutput {
                id: 5,
                key: "relay.5".into(),
                name: "Relay 5".into(),
                role: String::new(),
                control: String::new(),
            }
        ];
        capabilities.strip_effects = vec![
            crate::four_d::controller::HardwareStripEffect {
                id: "strip.strobe".into(),
                name: "Strobe".into(),
                engine: "builtin".into(),
                category: "Lighting".into(),
                icon: "sparkle".into(),
                group_icon: String::new(),
                default_duration_ms: Some(2000),
                ..Default::default()
            }
        ];
        app.update_hardware_capabilities(Some(capabilities));

        let payload = EffectDragPayload {
            name: "Strobe".into(),
            icon: "sparkle".into(),
            duration_ms: 2000,
            target: crate::four_d::models::HardwareTarget::ControllerMacro,
            actions: Vec::new(),
            controller_macro: None,
            controller_strip_effect: Some(crate::four_d::models::ControllerStripEffectCue {
                id: "strip.strobe".into(),
            }),
            controller_lane: Some(crate::four_d::models::ControllerEffectLane::Lighting),
        };
        let dropped = app.handle_effect_drop(&payload, 0, 1.0);
        assert!(dropped);

        let context = egui::Context::default();
        let mut time = 0.0;
        let mut frame = |app: &mut PealayerApp, events: Vec<egui::Event>| {
            time += 0.02;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 420.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    PealayerTabViewer { app }.ui(ui, &mut PealayerTab::Timeline);
                },
            );
            output.textures_delta.clear();
            output
        };

        frame(&mut app, vec![]);

        // Left handle area should give ResizeHorizontal
        let out_left = frame(&mut app, vec![egui::Event::PointerMoved(egui::pos2(370.0, 52.0))]);
        assert_eq!(out_left.platform_output.cursor_icon, egui::CursorIcon::ResizeHorizontal);

        // Right handle area should give ResizeHorizontal
        let out_right = frame(&mut app, vec![egui::Event::PointerMoved(egui::pos2(565.0, 52.0))]);
        assert_eq!(out_right.platform_output.cursor_icon, egui::CursorIcon::ResizeHorizontal);

        // Center should give Grab
        let out_center = frame(&mut app, vec![egui::Event::PointerMoved(egui::pos2(460.0, 52.0))]);
        assert_eq!(out_center.platform_output.cursor_icon, egui::CursorIcon::Grab);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PealayerTab {
    ProgramMonitor,
    MediaInspector,
    EffectControls,
    EffectsLibrary,
    HardwareMonitor,
    Timeline,
}

impl PealayerTab {
    pub const ALL: [PealayerTab; 6] = [
        PealayerTab::ProgramMonitor,
        PealayerTab::MediaInspector,
        PealayerTab::Timeline,
        PealayerTab::EffectControls,
        PealayerTab::EffectsLibrary,
        PealayerTab::HardwareMonitor,
    ];

    pub fn title(self, app: &crate::app::PealayerApp) -> String {
        match self {
            PealayerTab::ProgramMonitor => app.tr("Program Monitor"),
            PealayerTab::MediaInspector => app.tr("Media Inspector"),
            PealayerTab::Timeline => app.tr("Timeline"),
            PealayerTab::EffectControls => app.tr("Effect Controls"),
            PealayerTab::EffectsLibrary => app.tr("Effects Library"),
            PealayerTab::HardwareMonitor => app.tr("Hardware Monitor"),
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            PealayerTab::ProgramMonitor => crate::ui::icons::MONITOR_PLAY,
            PealayerTab::MediaInspector => crate::ui::icons::MAGNIFYING_GLASS,
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

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(*tab)
    }

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
                    crate::ui::preferences::open(self.app, ui.ctx());
                    ui.close();
                }
            }
            PealayerTab::MediaInspector => {
                if ui
                    .add_enabled(
                        self.app.current_video_path.is_some(),
                        egui::Button::new(format!(
                            "{} {}",
                            crate::ui::icons::ARROW_CLOCKWISE,
                            self.app.tr("Refresh from libmpv")
                        )),
                    )
                    .clicked()
                {
                    self.app.refresh_media_tracks();
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
            crate::ui::preferences::open(self.app, ui.ctx());
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
            PealayerTab::EffectsLibrary
            | PealayerTab::HardwareMonitor
            | PealayerTab::MediaInspector => [false, false],
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
        let estop_banner_label = self.app.tr("EMERGENCY STOP ACTIVE - ALL OUTPUTS DISABLED");
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
        let deselect_keyframe_label = self.app.tr("Deselect keyframe");
        let keyframe_label = self.app.tr("Keyframe");
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
                    PealayerTab::MediaInspector => {
                        crate::ui::media_inspector::draw(self.app, ui);
                    }
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
                                    let mute_icon = if self.app.is_muted {
                                        crate::ui::icons::SPEAKER_SLASH
                                    } else {
                                        crate::ui::icons::SPEAKER_HIGH
                                    };
                                    if ui
                                        .add_sized([30.0, 22.0], egui::Button::new(mute_icon))
                                        .on_hover_text(if self.app.is_muted {
                                            self.app.tr("Unmute")
                                        } else {
                                            self.app.tr("Mute")
                                        })
                                        .clicked()
                                    {
                                        self.app.toggle_audio_muted();
                                    }
                                    let mut volume = self.app.volume;
                                    let volume_response = ui
                                        .add_sized([76.0, 18.0], egui::Slider::new(&mut volume, 0.0..=130.0).show_value(false))
                                        .on_hover_text(format!("{}: {:.0}%", self.app.tr("Volume"), volume));
                                    if volume_response.changed() {
                                        let _ = self.app.mpv.set_property("volume", volume);
                                        self.app.volume = volume;
                                    }
                                    if (volume_response.changed() && !volume_response.dragged())
                                        || volume_response.drag_stopped()
                                    {
                                        self.app.save_config();
                                    }
                                    crate::ui::media_tracks::menu_button(
                                        self.app,
                                        ui,
                                        crate::app::MediaTrackType::Video,
                                        "nle-video-track-menu",
                                    );
                                    crate::ui::media_tracks::menu_button(
                                        self.app,
                                        ui,
                                        crate::app::MediaTrackType::Audio,
                                        "nle-audio-track-menu",
                                    );
                                    crate::ui::media_tracks::menu_button(
                                        self.app,
                                        ui,
                                        crate::app::MediaTrackType::Subtitle,
                                        "nle-subtitle-track-menu",
                                    );
                                    crate::ui::controls::draw_contextual_transport_nudge(
                                        self.app,
                                        ui,
                                        ui.make_persistent_id("nle-transport-back"),
                                        -1,
                                        crate::ui::controls::TransportNudgeDensity::Compact,
                                    );
                                    crate::ui::controls::draw_contextual_transport_nudge(
                                        self.app,
                                        ui,
                                        ui.make_persistent_id("nle-transport-forward"),
                                        1,
                                        crate::ui::controls::TransportNudgeDensity::Compact,
                                    );
                                    ui.separator();

                                    let elapsed = self.app.seek_pos.unwrap_or(self.app.playback_time);
                                    let include_hours = self.app.duration >= 3600.0;
                                    crate::ui::controls::draw_elapsed_editor(
                                        self.app,
                                        ui,
                                        "nle-elapsed-editor",
                                        can_seek,
                                    );

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

                                    // Anchor the duration and fullscreen affordance to the right
                                    // edge first, then give the seekbar the exact remaining width.
                                    // A former fixed 180 px reservation was wider than these
                                    // controls and left a visible, useless tail after fullscreen.
                                    let response = ui
                                        .with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let fullscreen_response = ui
                                                    .button(crate::ui::icons::ARROWS_OUT)
                                                    .on_hover_text(format!(
                                                        "{} (F)",
                                                        self.app.tr("Fullscreen")
                                                    ));
                                                fullscreen_response.context_menu(|ui| {
                                                    crate::ui::controls::transport_context_menu(
                                                        self.app, ui,
                                                    )
                                                });
                                                if fullscreen_response.clicked() {
                                                    self.app.set_fullscreen(ui.ctx(), true);
                                                }

                                                let total_response = ui
                                                    .add(
                                                        egui::Label::new(total_text).sense(
                                                            if finite_timeline {
                                                                egui::Sense::click()
                                                            } else {
                                                                egui::Sense::hover()
                                                            },
                                                        ),
                                                    )
                                                    .on_hover_text(total_tooltip);
                                                total_response.context_menu(|ui| {
                                                    crate::ui::controls::transport_context_menu(
                                                        self.app, ui,
                                                    )
                                                });
                                                if finite_timeline && total_response.clicked() {
                                                    self.app.show_remaining_time =
                                                        !self.app.show_remaining_time;
                                                    self.app.save_config();
                                                }

                                                let slider = egui::Slider::new(
                                                    &mut current_pos,
                                                    0.0..=max_dur,
                                                )
                                                .show_value(false)
                                                .trailing_fill(true);
                                                let response = crate::ui::controls::add_fill_width_slider(
                                                    ui,
                                                    can_seek,
                                                    slider,
                                                );
                                                response.context_menu(|ui| {
                                                    crate::ui::controls::transport_context_menu(
                                                        self.app, ui,
                                                    )
                                                });
                                                response
                                            },
                                        )
                                        .inner;

                                    if let Some(buffered_until) = self.app.buffered_until() {
                                        crate::ui::controls::paint_buffered_seekbar(
                                            ui,
                                            &response,
                                            (current_pos / self.app.duration).clamp(0.0, 1.0)
                                                as f32,
                                            (buffered_until / self.app.duration).clamp(0.0, 1.0)
                                                as f32,
                                        );
                                    }
                                    crate::ui::controls::paint_seekbar_chapters(
                                        ui, &response, self.app.duration, &self.app.media_chapters(),
                                        self.app.active_media_chapter().map(|chapter| chapter.index),
                                    );
                                    let show_seek_preview =
                                        self.app.nle_seekbar_hover_thumbnails;
                                    crate::ui::seek_preview::draw(
                                        self.app,
                                        ui,
                                        &response,
                                        show_seek_preview,
                                        "nle-seekbar-preview",
                                    );

                                    if can_seek && response.changed() {
                                        self.app.scrub_to(current_pos);
                                        ui.ctx().request_repaint();
                                    }
                                    if can_seek
                                        && (response.drag_stopped()
                                            || response.clicked()
                                            || (self.app.is_scrubbing && !ui.input(|i| i.pointer.primary_down())))
                                    {
                                        self.app.finish_scrub(current_pos);
                                    }
                                });
                            });
                        });
                    }
                    PealayerTab::EffectControls => {
                        let connected_hardware = self
                            .app
                            .advertised_hardware()
                            .filter(|capabilities| capabilities.board_connected);
                        let advertised_relays = connected_hardware
                            .as_ref()
                            .map(|capabilities| capabilities.relays.clone())
                            .unwrap_or_default();
                        let active_relays = connected_hardware
                            .as_ref()
                            .map(|capabilities| capabilities.active_relays.clone())
                            .unwrap_or_default();
                        let board_name = connected_hardware
                            .as_ref()
                            .map(|capabilities| {
                                crate::ui::i18n::visual_text(
                                    display_language,
                                    &capabilities.board_name,
                                )
                            })
                            .filter(|name| !name.trim().is_empty());
                        let selected_count = self.app.selected_instance_ids.len();
                        let panel_width = effect_controls_content_width(ui.available_width());
                        let now = ui.input(|i| i.time);
                        let ping_strength = self.app.effect_controls_ping_strength(now);
                        if ping_strength > 0.0 {
                            ui.ctx().request_repaint();
                        }
                        ui.set_width(panel_width);
                        let panel_subtitle = if selected_count == 0 {
                            self.app.tr("Select a cue to inspect and edit")
                        } else if selected_count == 1 {
                            self.app.tr("Timing, routing, and output")
                        } else {
                            self.app.tr("Batch-edit selected cues")
                        };
                        let panel_header = effect_controls_panel_header(
                            ui,
                            panel_width,
                            selected_count,
                            self.app.timeline.instances.len(),
                            &self.app.tr("Effect Controls"),
                            &panel_subtitle,
                            &if selected_count == 1 {
                                self.app.tr("1 cue")
                            } else {
                                format!("{selected_count} {}", self.app.tr("cues"))
                            },
                        );
                        ui.add_space(9.0);

                        if selected_count == 1 {
                            let id = *self.app.selected_instance_ids.iter().next().unwrap();
                            let mut timeline_dirty = false;
                            let mut delete_cue = false;
                            let mut jump_to_cue = false;
                            let mut duplicate_cue = false;
                            let mut relocate_effect_id = None;

                            let instance_idx = self
                                .app
                                .timeline
                                .instances
                                .iter()
                                .position(|instance| instance.id == id);

                            let timeline_before_edit = self.app.snapshot_timeline();
                            let mut push_undo = false;
                            let mut isolate_instance = false;
                            let mut update_start_to = None;
                            let mut update_duration_to = None;
                            let mut update_relay_to = None;
                            let mut update_direct_value_to = None;
                            let mut update_direct_behavior_to = None;

                            if let Some(idx) = instance_idx {
                                let selected_cue_label = self.app.tr("Selected cue");
                                let name_label = self.app.tr("Name");
                                let timing_label = self.app.tr("Timing");
                                let timing_subtitle = self.app.tr("Exact timeline placement and length");
                                let start_time_label = self.app.tr("Starts");
                                let duration_label = self.app.tr("Duration");
                                let fixed_duration_help = self.app.tr("Duration is defined by the recorded effect");
                                let direct_channel_value_label = self.app.tr("Direct channel value");
                                let state_label = self.app.tr("State");
                                let on_label = self.app.tr("On");
                                let off_label = self.app.tr("Off");
                                let live_output_on_label = self.app.tr("Live output on");
                                let live_output_off_label = self.app.tr("Live output off");
                                let hardware_target_label = self.app.tr("Hardware target");
                                let source_label = self.app.tr("Source");
                                let unavailable_output_label = self.app.tr("Unavailable output");
                                let unavailable_project_output_label =
                                    self.app.tr("Unavailable project output");
                                let connect_output_label = self.app.tr("No live hardware outputs");
                                let target_output_label = self.app.tr("Output");
                                let delete_cue_label = self.app.tr("Delete Cue");
                                let jump_label = self.app.tr("Go to cue");
                                let duplicate_label = self.app.tr("Duplicate cue");
                                let details_label = self.app.tr("Technical details");
                                let target_mismatch_label = self.app.tr("Target mismatch");
                                let configured_output_label = self.app.tr("Configured output");
                                let move_matching_label = self.app.tr("Move cue to matching track");
                                let macro_label = self.app.tr("Macro");
                                let mode_label = self.app.tr("Mode");
                                let effect_label = self.app.tr("Effect");
                                let untitled_effect_label = self.app.tr("Untitled effect");
                                let max_secs = if self.app.duration > 0.0 {
                                    self.app.duration
                                } else {
                                    60.0
                                };
                                let instance_effect_id = self.app.timeline.instances[idx].effect_id;
                                let instance_start_ms = self.app.timeline.instances[idx].start_time_ms;

                                let mut template_idx = None;
                                for (t_idx, tmpl) in self.app.timeline.templates.iter().enumerate() {
                                    if tmpl.id == instance_effect_id {
                                        template_idx = Some(t_idx);
                                        break;
                                    }
                                }

                                if let Some(t_idx) = template_idx {
                                    let (kind_icon, kind_label, ownership_label) =
                                        effect_controls_kind(&self.app.timeline.templates[t_idx]);
                                    let displayed_kind = self.app.tr(kind_label);
                                    let displayed_ownership = self.app.tr(ownership_label);
                                    let template = &mut self.app.timeline.templates[t_idx];
                                    let summary_title = if template.name.trim().is_empty() {
                                        untitled_effect_label
                                    } else {
                                        crate::ui::i18n::visual_text(display_language, &template.name)
                                    };

                                    effect_controls_card_with_ping(
                                        ui,
                                        panel_width,
                                        kind_icon,
                                        &summary_title,
                                        Some(&selected_cue_label),
                                        true,
                                        ping_strength,
                                        |ui| {
                                            ui.horizontal_wrapped(|ui| {
                                                effect_controls_badge(
                                                    ui,
                                                    kind_icon,
                                                    displayed_kind.clone(),
                                                );
                                                effect_controls_badge(
                                                    ui,
                                                    crate::ui::icons::CLOCK,
                                                    if template.is_state_marker() { "Set and keep".to_string() } else { crate::duration::format_effect_duration_for_language(
                                                        display_language,
                                                        template.duration_ms,
                                                    ) },
                                                );
                                            });
                                            ui.add_space(7.0);
                                            ui.label(egui::RichText::new(&name_label).small().weak());
                                            let name_align = crate::ui::i18n::input_alignment(
                                                self.app.rtl,
                                                &template.name,
                                            );
                                            let name_editor = ui.add_sized(
                                                [ui.available_width(), 26.0],
                                                crate::ui::dialog::singleline_text_edit(&mut template.name)
                                                    .horizontal_align(name_align),
                                            );
                                            if name_editor.gained_focus() {
                                                push_undo = true;
                                            }
                                            if name_editor.changed() {
                                                timeline_dirty = true;
                                            }
                                        },
                                    );

                                    ui.add_space(8.0);

                                    effect_controls_card(
                                        ui,
                                        panel_width,
                                        crate::ui::icons::CLOCK,
                                        &timing_label,
                                        Some(&timing_subtitle),
                                        false,
                                        |ui| {
                                            let max_start_ms = (max_secs * 1_000.0).round() as u64;
                                            let mut start_ms = instance_start_ms;
                                            effect_controls_timing_overview(
                                                ui,
                                                start_ms,
                                                if template.is_state_marker() { 0 } else { template.duration_ms },
                                                max_start_ms,
                                                (self.app.playback_time.max(0.0) * 1_000.0)
                                                    .round() as u64,
                                            );
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(crate::duration::format_time_value_ms(start_ms))
                                                        .small()
                                                        .weak(),
                                                );
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            egui::RichText::new(crate::duration::format_time_value_ms(
                                                                start_ms.saturating_add(if template.is_state_marker() { 0 } else { template.duration_ms }),
                                                            ))
                                                            .small()
                                                            .weak(),
                                                        );
                                                    },
                                                );
                                            });
                                            ui.add_space(6.0);
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(&start_time_label).weak());
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        let editor = ui.add(
                                                            crate::duration::time_value_drag(
                                                                &mut start_ms,
                                                                0..=max_start_ms,
                                                                50.0,
                                                                self.app.human_readable_time_units,
                                                            ),
                                                        );
                                                        if editor.drag_started()
                                                            || (editor.changed() && !editor.dragged())
                                                        {
                                                            push_undo = true;
                                                        }
                                                        if editor.changed() {
                                                            update_start_to = Some(start_ms);
                                                            timeline_dirty = true;
                                                        }
                                                    },
                                                );
                                            });
                                            ui.add_space(7.0);
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(&duration_label).weak());
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        if template.duration_resizable() {
                                                            let mut duration_ms = template.duration_ms;
                                                            let editor = ui.add(
                                                                crate::duration::time_value_drag(
                                                                    &mut duration_ms,
                                                                    50..=3_600_000,
                                                                    50.0,
                                                                    self.app.human_readable_time_units,
                                                                ),
                                                            );
                                                            if editor.drag_started()
                                                                || (editor.changed() && !editor.dragged())
                                                            {
                                                                push_undo = true;
                                                            }
                                                            if editor.changed() {
                                                                isolate_instance = true;
                                                                update_duration_to = Some(duration_ms);
                                                                timeline_dirty = true;
                                                            }
                                                        } else if template.is_state_marker() {
                                                            ui.label("Until next command");
                                                        } else {
                                                            ui.label(
                                                                crate::duration::format_effect_duration_for_language(
                                                                    display_language,
                                                                    template.duration_ms,
                                                                ),
                                                            ).on_hover_text(&fixed_duration_help);
                                                        }
                                                    },
                                                );
                                            });
                                        },
                                    );

                                    ui.add_space(8.0);

                                    let is_controller_owned = template.controller_macro.is_some()
                                        || template.controller_strip_effect.is_some();
                                    let current_relay_id = template.actions.first().map(|a| a.relay_id).unwrap_or(0);
                                    if let Some(direct) = template.direct_control.as_ref() {
                                        let is_relay = direct.control_key.starts_with("relay.");
                                        effect_controls_card(
                                            ui,
                                            panel_width,
                                            if is_relay { crate::ui::icons::PLUG } else { crate::ui::icons::LIGHTBULB },
                                            &direct_channel_value_label,
                                            Some(&direct.control_key),
                                            false,
                                            |ui| {
                                                let mut value = direct.value_basis_points;
                                                let mut behavior = direct.behavior;
                                                let mut end = direct.end_value_basis_points;
                                                direct_cue_behavior_editor(ui, &mut behavior, &mut end, !is_relay);
                                                if behavior != direct.behavior || end != direct.end_value_basis_points {
                                                    update_direct_behavior_to = Some((value, behavior, end));
                                                }
                                                if is_relay {
                                                    ui.horizontal(|ui| {
                                                        ui.label(egui::RichText::new(&state_label).weak());
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            let relay_id = direct
                                                                .control_key
                                                                .strip_prefix("relay.")
                                                                .and_then(|value| value.parse::<u8>().ok());
                                                            let live = relay_id
                                                                .is_some_and(|relay_id| active_relays.contains(&relay_id));
                                                            ui.colored_label(
                                                                if live {
                                                                    egui::Color32::from_rgb(38, 166, 91)
                                                                } else {
                                                                    ui.visuals().weak_text_color()
                                                                },
                                                                if live {
                                                                    format!("{} {live_output_on_label}", crate::ui::icons::DOT_OUTLINE)
                                                                } else {
                                                                    format!("{} {live_output_off_label}", crate::ui::icons::DOT_OUTLINE)
                                                                },
                                                            );
                                                        });
                                                    });
                                                    ui.add_space(5.0);
                                                    effect_controls_relay_state(
                                                        ui,
                                                        &mut value,
                                                        &on_label,
                                                        &off_label,
                                                    );
                                                } else {
                                                    let mut percent = f64::from(value) / 100.0;
                                                    draw_pwm_editor_row(ui, &mut percent, true);
                                                    value = (percent.clamp(0.0, 100.0) * 100.0).round() as u16;
                                                }
                                                if value != direct.value_basis_points {
                                                    if update_direct_behavior_to.is_some() {
                                                        update_direct_behavior_to = Some((value, behavior, end));
                                                    } else { update_direct_value_to = Some(value); }
                                                }
                                            },
                                        );
                                    } else if !is_controller_owned {
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
                                        effect_controls_card(
                                            ui,
                                            panel_width,
                                            crate::ui::icons::WARNING,
                                            &target_mismatch_label,
                                            Some(&format!(
                                                "{configured_output_label}: {configured_name}"
                                            )),
                                            false,
                                            |ui| {
                                            if let Some(primary) = template.target.primary_relay_id() {
                                                if advertised_relays.iter().any(|relay| relay.id == primary)
                                                    && ui.button(format!("{}  {move_matching_label}", crate::ui::icons::PUSH_PIN)).clicked()
                                                {
                                                    relocate_effect_id = Some(template.id);
                                                }
                                            }
                                            },
                                        );
                                        ui.add_space(8.0);
                                    }

                                    effect_controls_card(
                                        ui,
                                        panel_width,
                                        crate::ui::icons::PLUG,
                                        &hardware_target_label,
                                        board_name.as_deref(),
                                        false,
                                        |ui| {
                                        let mut selected_relay = current_relay_id;

                                        if advertised_relays.is_empty() {
                                            ui.horizontal(|ui| {
                                                ui.label(crate::ui::icons::WARNING);
                                                ui.label(egui::RichText::new(&connect_output_label).weak());
                                            });
                                        } else {
                                          ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&target_output_label).weak());
                                            let active = active_relays.contains(&selected_relay);
                                            ui.colored_label(
                                                if active {
                                                    egui::Color32::from_rgb(34, 197, 94)
                                                } else {
                                                    ui.visuals().weak_text_color()
                                                },
                                                crate::ui::icons::DOT_OUTLINE,
                                            );
                                            egui::ComboBox::from_id_salt("relay_combo")
                                                .width((ui.available_width() - 8.0).max(80.0))
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
                                        },
                                    );
                                    } else {
                                        effect_controls_card(
                                            ui,
                                            panel_width,
                                            kind_icon,
                                            &source_label,
                                            Some(&displayed_ownership),
                                            false,
                                            |ui| {
                                                if let Some(macro_cue) = template.controller_macro.as_ref() {
                                                    ui.horizontal(|ui| {
                                                        ui.label(egui::RichText::new(&macro_label).weak());
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            ui.label(format!("#{}", macro_cue.id));
                                                        });
                                                    });
                                                    if !macro_cue.mode.trim().is_empty() {
                                                        ui.horizontal(|ui| {
                                                            ui.label(egui::RichText::new(&mode_label).weak());
                                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                ui.label(&macro_cue.mode);
                                                            });
                                                        });
                                                    }
                                                }
                                                if let Some(strip_cue) = template.controller_strip_effect.as_ref() {
                                                    ui.horizontal(|ui| {
                                                        ui.label(egui::RichText::new(&effect_label).weak());
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            ui.label(&strip_cue.id);
                                                        });
                                                    });
                                                }
                                            },
                                        );
                                    }

                                    ui.add_space(8.0);
                                    effect_controls_card(
                                        ui,
                                        panel_width,
                                        crate::ui::icons::INFO,
                                        &details_label,
                                        Some(&displayed_ownership),
                                        false,
                                        |ui| {
                                        let identifier_width = (ui.available_width() - 112.0).max(80.0);
                                        egui::Grid::new(("effect-control-identifiers", id))
                                            .num_columns(3)
                                            .spacing([8.0, 6.0])
                                            .show(ui, |ui| {
                                                ui.label(egui::RichText::new("Cue ID").small().weak());
                                                ui.add_sized(
                                                    [identifier_width, 20.0],
                                                    egui::Label::new(egui::RichText::new(id.to_string()).monospace().small())
                                                        .selectable(true)
                                                        .truncate(),
                                                );
                                                if ui.button(crate::ui::icons::COPY).on_hover_text("Copy cue ID").clicked() {
                                                    ui.ctx().copy_text(id.to_string());
                                                }
                                                ui.end_row();
                                                ui.label(egui::RichText::new("Effect ID").small().weak());
                                                ui.add_sized(
                                                    [identifier_width, 20.0],
                                                    egui::Label::new(egui::RichText::new(template.id.to_string()).monospace().small())
                                                        .selectable(true)
                                                        .truncate(),
                                                );
                                                if ui.button(crate::ui::icons::COPY).on_hover_text("Copy effect ID").clicked() {
                                                    ui.ctx().copy_text(template.id.to_string());
                                                }
                                                ui.end_row();
                                            });
                                        },
                                    );

                                    ui.add_space(8.0);
                                    egui::Frame::new()
                                        .fill(ui.visuals().widgets.noninteractive.weak_bg_fill)
                                        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                                        .corner_radius(9.0)
                                        .inner_margin(egui::Margin::same(8))
                                        .show(ui, |ui| {
                                            ui.set_width(effect_controls_frame_content_width(panel_width));
                                            ui.horizontal_wrapped(|ui| {
                                                if ui
                                                    .button(format!("{}  {jump_label}", crate::ui::icons::SKIP_BACK))
                                                    .clicked()
                                                {
                                                    jump_to_cue = true;
                                                }
                                                if ui
                                                    .button(format!("{}  {duplicate_label}", crate::ui::icons::COPY))
                                                    .clicked()
                                                {
                                                    duplicate_cue = true;
                                                }
                                                let destructive = egui::Color32::from_rgb(220, 74, 74);
                                                if ui
                                                    .add(
                                                        egui::Button::new(
                                                            egui::RichText::new(format!(
                                                                "{}  {delete_cue_label}",
                                                                crate::ui::icons::TRASH
                                                            ))
                                                            .color(destructive),
                                                        )
                                                        .fill(destructive.gamma_multiply(0.08))
                                                        .stroke(egui::Stroke::new(1.0, destructive.gamma_multiply(0.55))),
                                                    )
                                                    .clicked()
                                                {
                                                    delete_cue = true;
                                                }
                                            });
                                        });
                                }
                            } else {
                                effect_controls_card(
                                    ui,
                                    panel_width,
                                    crate::ui::icons::WARNING,
                                    &self.app.tr("Cue unavailable"),
                                    Some(&self.app.tr("The selected cue is no longer on the timeline")),
                                    false,
                                    |_| {},
                                );
                            }

                            if push_undo {
                                self.app.undo_stack.push(timeline_before_edit);
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

                            if let Some(value) = update_direct_value_to {
                                if let Err(error) = self.app.update_direct_control_cue_value(id, value) {
                                    self.app.set_osd(error);
                                }
                                ui.ctx().request_repaint();
                            }
                            if let Some((value, behavior, end)) = update_direct_behavior_to {
                                if let Err(error) = self.app.configure_direct_control_cue(id, value, Some(behavior), Some(end)) {
                                    self.app.set_osd(error);
                                }
                            }

                            if let Some(eff_id) = relocate_effect_id {
                                self.app.relocate_effect_to_primary(eff_id);
                                ui.ctx().request_repaint();
                            }

                            if duplicate_cue {
                                if let Some(instance) = self
                                    .app
                                    .timeline
                                    .instances
                                    .iter()
                                    .find(|instance| instance.id == id)
                                    .cloned()
                                {
                                    let duration_ms = self
                                        .app
                                        .timeline
                                        .templates
                                        .iter()
                                        .find(|template| template.id == instance.effect_id)
                                        .map(|template| template.duration_ms)
                                        .unwrap_or(0);
                                    self.app.undo_stack.push(self.app.snapshot_timeline());
                                    let duplicate = crate::four_d::models::EffectInstance::new(
                                        instance.effect_id,
                                        instance.start_time_ms.saturating_add(duration_ms),
                                    );
                                    let duplicate_id = duplicate.id;
                                    self.app.timeline.instances.push(duplicate);
                                    self.app.selected_instance_ids.clear();
                                    self.app.selected_instance_ids.insert(duplicate_id);
                                    timeline_dirty = true;
                                }
                            }

                            if jump_to_cue {
                                if let Some(instance) = self
                                    .app
                                    .timeline
                                    .instances
                                    .iter()
                                    .find(|instance| instance.id == id)
                                {
                                    self.app.seek_absolute(instance.start_time_ms as f64 / 1_000.0);
                                }
                            }

                            if delete_cue {
                                self.app.undo_stack.push(self.app.snapshot_timeline());
                                self.app.timeline.instances.retain(|inst| inst.id != id);
                                self.app.selected_instance_ids.clear();
                                timeline_dirty = true;
                            }

                            if timeline_dirty {
                                self.app.commit_timeline_edit();
                                ui.ctx().request_repaint();
                            }
                        } else if selected_count > 1 {
                            let mut timeline_dirty = false;
                            let mut delete_all = false;
                            let mut bulk_relay = None;

                            effect_controls_card_with_ping(
                                ui,
                                panel_width,
                                crate::ui::icons::SELECTION_ALL,
                                &self.app.tr("Multiple cues selected"),
                                Some(&self.app.tr("Changes apply to every compatible cue")),
                                true,
                                ping_strength,
                                |ui| {
                                    let controller_owned = self
                                        .app
                                        .timeline
                                        .instances
                                        .iter()
                                        .filter(|instance| self.app.selected_instance_ids.contains(&instance.id))
                                        .filter(|instance| {
                                            self.app
                                                .timeline
                                                .templates
                                                .iter()
                                                .find(|template| template.id == instance.effect_id)
                                                .is_some_and(|template| {
                                                    template.controller_macro.is_some()
                                                        || template.controller_strip_effect.is_some()
                                                })
                                        })
                                        .count();
                                    ui.horizontal_wrapped(|ui| {
                                        effect_controls_badge(
                                            ui,
                                            crate::ui::icons::PUSH_PIN,
                                            format!("{selected_count} {}", self.app.tr("selected")),
                                        );
                                        if controller_owned > 0 {
                                            effect_controls_badge(
                                                ui,
                                                crate::ui::icons::CIRCUITRY,
                                                format!("{controller_owned} {}", self.app.tr("controller-owned")),
                                            );
                                        }
                                    });
                                },
                            );

                            ui.add_space(8.0);
                            effect_controls_card(
                                ui,
                                panel_width,
                                crate::ui::icons::PLUG,
                                &self.app.tr("Set hardware target"),
                                board_name.as_deref(),
                                false,
                                |ui| {
                                    if advertised_relays.is_empty() {
                                        ui.horizontal(|ui| {
                                            ui.label(crate::ui::icons::WARNING);
                                            ui.label(egui::RichText::new(self.app.tr("No live hardware outputs")).weak());
                                        });
                                    } else {
                                        ui.horizontal_wrapped(|ui| {
                                            for relay in &advertised_relays {
                                                let active = active_relays.contains(&relay.id);
                                                let label = format!(
                                                    "{}  {}",
                                                    if active { crate::ui::icons::DOT_OUTLINE } else { crate::ui::icons::PLUG },
                                                    crate::ui::i18n::visual_text(display_language, &relay.name),
                                                );
                                                if ui.button(label).clicked() {
                                                    bulk_relay = Some(relay.id);
                                                }
                                            }
                                        });
                                    }
                                },
                            );

                            if let Some(relay_id) = bulk_relay {
                                self.app.undo_stack.push(self.app.snapshot_timeline());
                                let selected_ids: Vec<_> = self
                                    .app
                                    .selected_instance_ids
                                    .iter()
                                    .copied()
                                    .collect();
                                for instance_id in selected_ids {
                                    let Some(effect_id) = self
                                        .app
                                        .timeline
                                        .instances
                                        .iter()
                                        .find(|instance| instance.id == instance_id)
                                        .map(|instance| instance.effect_id)
                                    else {
                                        continue;
                                    };
                                    let controller_owned = self
                                        .app
                                        .timeline
                                        .templates
                                        .iter()
                                        .find(|template| template.id == effect_id)
                                        .is_some_and(|template| {
                                            template.controller_macro.is_some()
                                                || template.controller_strip_effect.is_some()
                                        });
                                    if controller_owned {
                                        continue;
                                    }

                                    self.app.isolate_template_for_instance(instance_id);
                                    let Some(isolated_effect_id) = self
                                        .app
                                        .timeline
                                        .instances
                                        .iter()
                                        .find(|instance| instance.id == instance_id)
                                        .map(|instance| instance.effect_id)
                                    else {
                                        continue;
                                    };
                                    if let Some(template) = self
                                        .app
                                        .timeline
                                        .templates
                                        .iter_mut()
                                        .find(|template| template.id == isolated_effect_id)
                                    {
                                        template.actions =
                                            crate::four_d::patterns::generate_constant(
                                                relay_id,
                                                true,
                                                template.duration_ms,
                                            );
                                        template.target =
                                            crate::four_d::models::HardwareTarget::Relay(relay_id);
                                    }
                                }
                                timeline_dirty = true;
                            }

                            ui.add_space(10.0);
                            if ui
                                .button(
                                    egui::RichText::new(format!(
                                        "{}  {delete_all_label}",
                                        crate::ui::icons::TRASH
                                    ))
                                    .color(egui::Color32::from_rgb(220, 74, 74)),
                                )
                                .clicked()
                            {
                                delete_all = true;
                            }

                            if delete_all {
                                self.app.undo_stack.push(self.app.snapshot_timeline());
                                let selected_ids = &self.app.selected_instance_ids;
                                self.app.timeline.instances.retain(|inst| !selected_ids.contains(&inst.id));
                                self.app.selected_instance_ids.clear();
                                timeline_dirty = true;
                            }

                            if timeline_dirty {
                                self.app.commit_timeline_edit();
                            }
                        } else {
                            effect_controls_card(
                                ui,
                                panel_width,
                                crate::ui::icons::SELECTION_ALL,
                                &self.app.tr("No cue selected"),
                                Some(&self.app.tr("Select a cue on the timeline to manage it")),
                                false,
                                |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        effect_controls_badge(
                                            ui,
                                            crate::ui::icons::KEYBOARD,
                                            "A",
                                        );
                                        ui.label(
                                            egui::RichText::new(self.app.tr(
                                                "Select a hardware track and press A, or double-click its lane, to add a cue.",
                                            ))
                                            .small()
                                            .weak(),
                                        );
                                    });
                                },
                            );
                        }
                        if panel_header.previous || panel_header.next {
                            let cue_ids = sorted_cue_ids(&self.app.timeline);
                            if !cue_ids.is_empty() {
                                let selected_index = cue_ids.iter().position(|cue_id| {
                                    self.app.selected_instance_ids.contains(cue_id)
                                });
                                let next_index = if panel_header.previous {
                                    selected_index
                                        .unwrap_or(0)
                                        .checked_sub(1)
                                        .unwrap_or(cue_ids.len() - 1)
                                } else {
                                    selected_index
                                        .map(|index| (index + 1) % cue_ids.len())
                                        .unwrap_or(0)
                                };
                                self.app.selected_instance_ids.clear();
                                self.app.selected_instance_ids.insert(cue_ids[next_index]);
                                self.app.selected_keyframes.clear();
                                self.app.selected_timeline_keyframe = None;
                                ui.ctx().request_repaint();
                            }
                        }
                    }
                    PealayerTab::EffectsLibrary => {
                        // Fixed rows and scrolling cards share one trailing edge.
                        let scroll = &ui.spacing().scroll;
                        let scrollbar_width = scroll.bar_width + scroll.bar_inner_margin + scroll.bar_outer_margin;
                        let effects_width = effects_panel_content_width(ui.available_width() - scrollbar_width);
                        ui.allocate_ui_with_layout(
                            egui::vec2(effects_width, 0.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| ui.horizontal(|ui| {
                            ui.heading(self.app.tr("Effects Library"));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    // Add in reverse declaration order because the
                                    // layout anchors its first child at the trailing
                                    // edge. The visible order remains New, Manage.
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
                                        .button(format!(
                                            "{} {}",
                                            crate::ui::icons::PLUS,
                                            self.app.tr("New effect")
                                        ))
                                        .clicked()
                                    {
                                        crate::ui::effects_library::begin_new_effect(
                                            self.app, None,
                                        );
                                    }
                                },
                            );
                            }),
                        );
                        ui.add_space(4.0);

                        // 1. Instant search edit field
                        ui.allocate_ui_with_layout(
                            egui::vec2(effects_width, 0.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| ui.horizontal(|ui| {
                            ui.label(self.app.tr("Search"));
                            let search_hint = self.app.tr("Search effects...");
                            let search_align = crate::ui::i18n::input_alignment(
                                self.app.rtl,
                                &self.app.effects_search_query,
                            );
                            let res = ui.add(
                                crate::ui::dialog::singleline_text_edit(&mut self.app.effects_search_query)
                                    .horizontal_align(search_align)
                                    .hint_text(search_hint)
                                    .desired_width(ui.available_width())
                            );
                            if res.changed() {
                                // Request repaint to filter instantly
                                ui.ctx().request_repaint();
                            }
                            }),
                        );

                        ui.add_space(8.0);

                        // Filter presets based on query
                        let query = self.app.effects_search_query.trim().to_lowercase();
                        let advertised_presets = self.app.advertised_effect_presets();
                        let effect_groups = self.app.advertised_hardware()
                            .map(|hardware| hardware.effect_groups).unwrap_or_default();
                        let mut categorized: std::collections::BTreeMap<String, Vec<&crate::app::EffectPreset>> = std::collections::BTreeMap::new();

                        for group in &effect_groups {
                            if query.is_empty() || group.name.to_lowercase().contains(&query) {
                                categorized.entry(group.name.clone()).or_default();
                            }
                        }

                        for preset in &advertised_presets {
                            if query.is_empty() || preset.effect.name.to_lowercase().contains(&query) {
                                categorized.entry(preset.category.clone()).or_default().push(preset);
                            }
                        }

                        let force_open = !query.is_empty();

                        if categorized.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(24.0);
                                ui.label(
                                    egui::RichText::new(self.app.tr("No effects"))
                                    .weak()
                                    .size(12.0),
                                );
                                ui.add_space(8.0);
                                if ui.button(format!("{} {}", crate::ui::icons::FOLDER_OPEN,
                                    self.app.tr("New group"))).clicked() {
                                    crate::ui::effects_library::begin_new_group(self.app);
                                }
                            });
                        } else {
                            egui::ScrollArea::vertical()
                                .id_salt("effects_scroll")
                                .show(ui, |ui| {
                                    // Keep a deliberate gutter between cards and the scrollbar /
                                    // right panel edge. The previous full-width inner frame caused
                                    // its stroke and action row to crowd or clip against that edge.
                                    ui.set_width(effects_width);
                                    for (category, presets) in categorized {
                                        let group_id = ui.make_persistent_id(("effect-group", &category));
                                        let mut open = ui.data_mut(|data| {
                                            data.get_persisted::<bool>(group_id).unwrap_or(true)
                                        });
                                        if force_open {
                                            open = true;
                                        }
                                        if presets.is_empty() {
                                            open = false;
                                        }
                                        let displayed_category = crate::ui::i18n::visual_text(
                                            display_language,
                                            &category,
                                        );
                                        let group_icon_name = presets
                                            .iter()
                                            .map(|preset| preset.group_icon.trim())
                                            .find(|icon| !icon.is_empty())
                                            .or_else(|| effect_groups.iter().find(|group| group.name == category).map(|group| group.icon.as_str()))
                                            .unwrap_or_default()
                                            .to_string();
                                        let group_icon = crate::ui::icons::named_control_icon(
                                            &group_icon_name,
                                        )
                                        .unwrap_or(crate::ui::icons::FOLDER_OPEN);
                                        let (group_response, add_effect) = effect_group_action_header(
                                            ui,
                                            effects_width,
                                            &displayed_category,
                                            group_icon,
                                            open,
                                            presets.len(),
                                            &self.app.tr("New effect in this group"),
                                        );
                                        if add_effect.clicked() {
                                            crate::ui::effects_library::begin_new_effect(
                                                self.app, Some(category.clone()),
                                            );
                                        }
                                        let dragging_effect = egui::DragAndDrop::payload::<EffectDragPayload>(
                                            ui.ctx(),
                                        )
                                        .is_some();
                                        if dragging_effect && group_response.contains_pointer() {
                                            group_response.clone().highlight();
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                        }
                                        if let Some(payload) = take_effect_drop_on_rect(
                                            ui.ctx(),
                                            group_response.rect,
                                        ) && let Err(error) =
                                            crate::ui::effects_library::move_dragged_effect_to_group(
                                                self.app,
                                                payload.as_ref(),
                                                category.clone(),
                                            )
                                        {
                                            self.app.set_osd(error);
                                        }
                                        if !presets.is_empty() && group_response.clicked() {
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
                                                    crate::ui::icons::PENCIL_SIMPLE,
                                                    self.app.tr("Manage group")
                                                ))
                                                .clicked()
                                            {
                                                self.app.effect_group_draft = Some(
                                                    crate::app::ControllerEffectGroupDraft {
                                                        original_name: category.clone(),
                                                        name: category.clone(),
                                                        icon: group_icon_name.clone(),
                                                    },
                                                );
                                                ui.close();
                                            }
                                            if ui
                                                .button(format!(
                                                    "{} {}",
                                                    crate::ui::icons::PLUS,
                                                    self.app.tr("New effect in this group")
                                                ))
                                                .clicked()
                                            {
                                                crate::ui::effects_library::begin_new_effect(
                                                    self.app,
                                                    Some(category.clone()),
                                                );
                                                ui.close();
                                            }
                                            if ui.button(format!("{} {}", crate::ui::icons::FOLDER_OPEN,
                                                self.app.tr("New group"))).clicked() {
                                                crate::ui::effects_library::begin_new_group(self.app);
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
                                            if !presets.is_empty() {
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
                                            }
                                        });
                                        ui.data_mut(|data| data.insert_persisted(group_id, open));

                                        if open {
                                            ui.add_space(5.0);
                                            for preset in presets {
                                                let Some(reference) =
                                                    effect_preset_reference(preset)
                                                else {
                                                    continue;
                                                };
                                                // `advertised_effect_presets` materializes fresh
                                                // timeline Effect values every frame, including a
                                                // fresh UUID. Interaction IDs must instead use the
                                                // controller-owned durable reference or neither a
                                                // drag nor a popup can survive into the next frame.
                                                let item_id = ui.make_persistent_id((
                                                    "effect-card",
                                                    &reference,
                                                ));
                                                let payload = EffectDragPayload {
                                                    name: preset.effect.name.clone(),
                                                    icon: preset.effect.icon.clone(),
                                                    duration_ms: preset.effect.duration_ms,
                                                    target: preset.effect.target,
                                                    actions: preset.effect.actions.clone(),
                                                    controller_macro: preset.effect.controller_macro.clone(),
                                                    controller_strip_effect: preset.effect.controller_strip_effect.clone(),
                                                    controller_lane: preset.effect.controller_lane,
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
                                                    relay_identifier_label(id)
                                                } else {
                                                    self.app.tr("Effect")
                                                };
                                                let displayed_effect_name =
                                                    crate::ui::i18n::visual_text(
                                                        display_language,
                                                        &preset.effect.name,
                                                    );
                                                let source = preset.source;
                                                let (_, primary_action_icon, primary_action_label, primary_action_enabled) =
                                                    crate::ui::effects_library::saved_effect_preview_action_presentation(
                                                        self.app,
                                                        &reference,
                                                    );
                                                let mut run_now = false;
                                                let mut place_at_playhead = false;
                                                let mut more_response = None;
                                                let card_width = effects_width;
                                                let inline_edit_id = item_id.with("inline-identity");
                                                let mut inline_edit = ui.data_mut(|data| {
                                                    data.get_temp::<InlineEffectIdentityEdit>(
                                                        inline_edit_id,
                                                    )
                                                });
                                                let inline_editing = inline_edit.is_some();
                                                let mut begin_inline_edit = ui.data_mut(|data| {
                                                    data.remove_temp::<bool>(item_id.with("rename-request"))
                                                        .unwrap_or(false)
                                                });
                                                let mut save_inline_edit = false;
                                                let mut cancel_inline_edit = false;
                                                let response = effect_drag_source_with_action_gutter(
                                                    ui,
                                                    item_id,
                                                    payload.clone(),
                                                    if inline_editing {
                                                        card_width
                                                    } else {
                                                        EFFECT_CARD_ACTION_GUTTER
                                                    },
                                                    |ui| {
                                                        effect_card(ui, card_width, |ui| {
                                                            if let Some(edit) = inline_edit.as_mut() {
                                                                ui.horizontal(|ui| {
                                                                    if ui.available_width() < 180.0 {
                                                                        ui.spacing_mut().item_spacing.x = ui.spacing().item_spacing.x.min(2.0);
                                                                    }
                                                                    let search_hint = self.app.tr("Search icons...");
                                                                    let presets_label = self.app.tr("Presets");
                                                                    let no_matches_label = self.app.tr("No matching icons");
                                                                    crate::ui::icons::searchable_icon_picker(
                                                                        ui,
                                                                        item_id.with("inline-icon"),
                                                                        &mut edit.icon,
                                                                        crate::ui::icons::IconPickerConfig {
                                                                            language: self.app.language,
                                                                            presets: crate::ui::icons::CONTROL_ICON_PRESETS,
                                                                            fallback_glyph: crate::ui::icons::SPARKLE,
                                                                            fallback_name: "Sparkle",
                                                                            width: 38.0,
                                                                            show_selected_name: false,
                                                                            search_hint: &search_hint,
                                                                            presets_label: &presets_label,
                                                                            no_matches_label: &no_matches_label,
                                                                            clear_label: None,
                                                                        },
                                                                    );
                                                                    let action_spacing = ui.spacing().item_spacing.x;
                                                                    let name_width = effect_card_rename_name_width(ui.available_width(), action_spacing);
                                                                    let name_align = crate::ui::i18n::input_alignment(
                                                                        self.app.rtl,
                                                                        &edit.name,
                                                                    );
                                                                    let name_response = ui.add_sized(
                                                                        [name_width, 24.0],
                                                                        crate::ui::dialog::singleline_text_edit(&mut edit.name)
                                                                            .id(item_id.with("inline-name"))
                                                                            .horizontal_align(name_align)
                                                                            .char_limit(64),
                                                                    );
                                                                    if ui.data_mut(|data| data.remove_temp::<bool>(item_id.with("rename-focus")).unwrap_or(false)) {
                                                                        name_response.request_focus();
                                                                    }
                                                                    if name_response.lost_focus()
                                                                        && ui.input(|input| {
                                                                            input.key_pressed(egui::Key::Enter)
                                                                        })
                                                                    {
                                                                        save_inline_edit = true;
                                                                    }
                                                                    if ui
                                                                        .add_sized(
                                                                            [24.0, 24.0],
                                                                            egui::Button::new(
                                                                                crate::ui::icons::CHECK,
                                                                            )
                                                                            .frame(false),
                                                                        )
                                                                        .on_hover_text(self.app.tr("Save"))
                                                                        .clicked()
                                                                    {
                                                                        save_inline_edit = true;
                                                                    }
                                                                    if ui
                                                                        .add_sized(
                                                                            [24.0, 24.0],
                                                                            egui::Button::new(
                                                                                crate::ui::icons::X,
                                                                            )
                                                                            .frame(false),
                                                                        )
                                                                        .on_hover_text(self.app.tr("Cancel"))
                                                                        .clicked()
                                                                        || ui.input(|input| {
                                                                            input.key_pressed(egui::Key::Escape)
                                                                        })
                                                                    {
                                                                        cancel_inline_edit = true;
                                                                    }
                                                                });
                                                            } else {
                                                                let header = effect_library_card_header(
                                                                    ui,
                                                                    item_id,
                                                                    &preset.effect.icon,
                                                                    &displayed_effect_name,
                                                                    primary_action_icon,
                                                                    primary_action_enabled,
                                                                    [
                                                                        &self.app.tr("Drag effect to timeline"),
                                                                        &self.app.tr("Change icon"),
                                                                        &self.app.tr("Rename"),
                                                                        &primary_action_label,
                                                                        &self.app.tr("Place at playhead"),
                                                                        &self.app.tr("More actions"),
                                                                        &self.app.tr("Rename"),
                                                                    ],
                                                                );
                                                                begin_inline_edit |= header.rename.clicked() || header.icon.clicked() || header.title.clicked();
                                                                run_now = header.run.clicked();
                                                                place_at_playhead = header.place.clicked();
                                                                more_response = Some(header.more);
                                                            }
                                                                ui.add_space(5.0);
                                                                effect_library_metadata(
                                                                    ui,
                                                                    &target_label,
                                                                    &crate::duration::format_effect_duration_for_language(
                                                                        display_language,
                                                                        preset.effect.duration_ms,
                                                                    ),
                                                                );
                                                            })
                                                    },
                                                );
                                                if begin_inline_edit
                                                    && crate::ui::effects_library::select_advertised_effect(
                                                        self.app,
                                                        source,
                                                        preset
                                                            .effect
                                                            .controller_strip_effect
                                                            .as_ref()
                                                            .map(|value| value.id.as_str()),
                                                    )
                                                    .is_some()
                                                {
                                                    inline_edit = Some(InlineEffectIdentityEdit {
                                                        name: self.app.effect_library_draft.name.clone(),
                                                        icon: self.app.effect_library_draft.icon.clone(),
                                                    });
                                                    ui.data_mut(|data| data.insert_temp(item_id.with("rename-focus"), true));
                                                    ui.ctx().request_repaint();
                                                }
                                                if cancel_inline_edit {
                                                    inline_edit = None;
                                                }
                                                if save_inline_edit
                                                    && let Some(edit) = inline_edit.as_ref()
                                                {
                                                    match crate::ui::effects_library::save_advertised_effect_identity(
                                                        self.app,
                                                        source,
                                                        preset
                                                            .effect
                                                            .controller_strip_effect
                                                            .as_ref()
                                                            .map(|value| value.id.as_str()),
                                                        edit.name.clone(),
                                                        edit.icon.clone(),
                                                    ) {
                                                        Ok(()) => inline_edit = None,
                                                        Err(error) => self.app.set_osd(error),
                                                    }
                                                }
                                                ui.data_mut(|data| {
                                                    if let Some(edit) = inline_edit.clone() {
                                                        data.insert_temp(inline_edit_id, edit);
                                                    } else {
                                                        data.remove::<InlineEffectIdentityEdit>(
                                                            inline_edit_id,
                                                        );
                                                    }
                                                });
                                                if response.response.dragged() {
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                                }
                                                if run_now
                                                    && let Err(error) = self
                                                        .app
                                                        .invoke_controller_effect_preview_action(&reference)
                                                {
                                                    self.app.set_osd(error);
                                                }
                                                if place_at_playhead {
                                                    self.app
                                                        .place_controller_effect_at_playhead(&payload);
                                                }
                                                let open_effect_menu = secondary_click_inside(
                                                    ui.ctx(),
                                                    response.response.rect,
                                                ) || more_response
                                                    .as_ref()
                                                    .is_some_and(egui::Response::clicked);
                                                let menu_anchor = more_response
                                                    .as_ref()
                                                    .unwrap_or(&response.response);
                                                egui::Popup::menu(menu_anchor)
                                                    .id(item_id.with("context-menu"))
                                                    .at_pointer_fixed()
                                                    .open_memory(open_effect_menu.then_some(egui::SetOpenCommand::Bool(true)))
                                                    .show(|ui| {
                                                    ui.strong(&displayed_effect_name);
                                                    ui.label(egui::RichText::new(&reference).monospace().weak().small());
                                                    ui.separator();
                                                    if ui.button(format!("{} {}", crate::ui::icons::PENCIL_SIMPLE, self.app.tr("Manage"))).clicked() {
                                                        if crate::ui::effects_library::select_advertised_effect(
                                                            self.app,
                                                            source,
                                                            preset.effect.controller_strip_effect.as_ref().map(|value| value.id.as_str()),
                                                        ).is_some() {
                                                            self.app.show_effect_library_editor = true;
                                                        }
                                                        ui.close();
                                                    }
                                                    crate::ui::effects_library::draw_saved_effect_preview_action(
                                                        self.app,
                                                        ui,
                                                        &reference,
                                                    );
                                                    if ui.button(format!("{} {}", crate::ui::icons::PLUS, self.app.tr("Place at playhead"))).clicked() {
                                                        self.app.place_controller_effect_at_playhead(&payload);
                                                        ui.close();
                                                    }
                                                    ui.separator();
                                                    if ui.button(format!("{} {}", crate::ui::icons::COPY, self.app.tr("Duplicate"))).clicked() {
                                                        if crate::ui::effects_library::select_advertised_effect(
                                                            self.app,
                                                            source,
                                                            preset.effect.controller_strip_effect.as_ref().map(|value| value.id.as_str()),
                                                        ).is_some() {
                                                            crate::ui::effects_library::duplicate_selected(self.app);
                                                        }
                                                        ui.close();
                                                    }
                                                    if ui.button(format!("{} {}", crate::ui::icons::TRASH, self.app.tr("Delete"))).clicked() {
                                                        if crate::ui::effects_library::select_advertised_effect(
                                                            self.app,
                                                            source,
                                                            preset.effect.controller_strip_effect.as_ref().map(|value| value.id.as_str()),
                                                        ).is_some()
                                                            && let Err(error) = self.app.delete_controller_effect()
                                                        {
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
                        crate::ui::effects_library::empty_library_context_menu(self.app, ui);
                            let mut keep_group_editor_open = self.app.effect_group_draft.is_some();
                            let mut save_group = false;
                            let mut cancel_group = false;
                            let new_group = self.app.effect_group_draft.as_ref()
                                .is_some_and(|draft| draft.original_name.is_empty());
                            let group_editor_title = format!(
                                "{} {}",
                                crate::ui::icons::FOLDER_OPEN,
                                self.app.tr(if new_group { "New group" } else { "Manage effect group" })
                            );
                            let group_name_label = self.app.tr("Name");
                            let group_icon_label = self.app.tr("Icon");
                            let presets_label = self.app.tr("Presets");
                            let icon_search_hint = self.app.tr("Search icons...");
                            let icon_no_matches_label = self.app.tr("No matching icons");
                            let group_default_icon_label = self.app.tr("Folder");
                            let save_group_label = self.app.tr(if new_group { "Create group" } else { "Save to PCController" });
                            let cancel_group_label = self.app.tr("Cancel");
                            let rtl_ui = self.app.rtl;
                            let icon_language = self.app.language;
                            if let Some(draft) = self.app.effect_group_draft.as_mut() {
                                egui::Window::new(group_editor_title)
                                .id(egui::Id::new("effect_group_editor"))
                                .order(egui::Order::Foreground)
                                .open(&mut keep_group_editor_open)
                                .resizable(false)
                                .collapsible(false)
                                .frame(crate::ui::dialog::opaque_window_frame(ui))
                                .show(ui.ctx(), |ui| {
                                    egui::Grid::new("effect_group_editor_grid")
                                        .num_columns(2)
                                        .spacing([14.0, 10.0])
                                        .show(ui, |ui| {
                                            ui.label(&group_name_label);
                                            let name_align = crate::ui::i18n::input_alignment(
                                                rtl_ui,
                                                &draft.name,
                                            );
                                            let name_response = ui.add(
                                                crate::ui::dialog::singleline_text_edit(&mut draft.name)
                                                    .horizontal_align(name_align)
                                                    .desired_width(250.0),
                                            );
                                            let focus_id = egui::Id::new("new-effect-group-name-focus");
                                            if new_group && !draft.name.trim().is_empty()
                                                && name_response.lost_focus()
                                                && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                                                save_group = true;
                                            }
                                            if new_group && !ui.data_mut(|data| data.get_temp::<bool>(focus_id).unwrap_or(false)) {
                                                name_response.request_focus();
                                                ui.data_mut(|data| data.insert_temp(focus_id, true));
                                            }
                                            ui.end_row();
                                            ui.label(&group_icon_label);
                                            crate::ui::icons::searchable_icon_picker(
                                                ui,
                                                "effect_group_icon_picker",
                                                &mut draft.icon,
                                                crate::ui::icons::IconPickerConfig {
                                                    language: icon_language,
                                                    presets: crate::ui::icons::CONTROL_ICON_PRESETS,
                                                    fallback_glyph: crate::ui::icons::FOLDER_OPEN,
                                                    fallback_name: &group_default_icon_label,
                                                    width: 250.0,
                                                    show_selected_name: true,
                                                    search_hint: &icon_search_hint,
                                                    presets_label: &presets_label,
                                                    no_matches_label: &icon_no_matches_label,
                                                    clear_label: None,
                                                },
                                            );
                                            ui.end_row();
                                        });
                                    ui.add_space(10.0);
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add_enabled(
                                                !draft.name.trim().is_empty(),
                                                egui::Button::new(format!(
                                                    "{} {}",
                                                    if new_group { crate::ui::icons::PLUS } else { crate::ui::icons::FLOPPY_DISK },
                                                    save_group_label
                                                )),
                                            )
                                            .clicked()
                                        {
                                            save_group = true;
                                        }
                                        if ui
                                            .button(format!(
                                                "{} {}",
                                                crate::ui::icons::X,
                                                cancel_group_label
                                            ))
                                            .clicked()
                                        {
                                            cancel_group = true;
                                        }
                                    });
                                });
                            }
                            if cancel_group {
                                keep_group_editor_open = false;
                            }
                            if save_group {
                                if let Some(draft) = self.app.effect_group_draft.clone() {
                                    if let Err(error) = self.app.save_controller_effect_group(draft) {
                                        self.app.set_osd(error);
                                    } else {
                                        keep_group_editor_open = false;
                                    }
                                }
                            }
                            if !keep_group_editor_open {
                                self.app.effect_group_draft = None;
                                ui.data_mut(|data| data.remove::<bool>(egui::Id::new("new-effect-group-name-focus")));
                            }
                    }
                    PealayerTab::HardwareMonitor => {
                        crate::four_d::authority::draw_controls(self.app,ui);
                        let capabilities = self.app.advertised_hardware();
                        ui.horizontal(|ui| {
                            ui.heading(self.app.tr("Hardware Monitor Dashboard"));
                            if crate::ui::hardware_control::channel_manager_available(
                                self.app.is_connected,
                                capabilities.as_ref(),
                            ) {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .button(format!(
                                                "{} {}",
                                                crate::ui::icons::SLIDERS_HORIZONTAL,
                                                self.app.tr("Manage channels")
                                            ))
                                            .on_hover_text(self.app.tr(
                                                "Rename, control, and reorder every advertised board channel",
                                            ))
                                            .clicked()
                                        {
                                            self.app.show_hardware_channels_dialog = true;
                                            self.app.hardware_channel_detail_active = false;
                                        }
                                    },
                                );
                            }
                        });
                        ui.add_space(8.0);

                        hardware_monitor_scroll(ui, |ui| {
                        if self.app.advertised_hardware().is_some_and(|caps| caps.supports_rf_transmit) {
                            if ui.button(format!("{} {}",crate::ui::icons::RADIO,self.app.tr("RF controls…"))).clicked() {
                                self.app.rf.open = true;
                                if let Err(error) = self.app.request_rf("catalog",serde_json::json!({"read_board":true})) { self.app.rf.error = error; }
                            }
                        }
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

                        if !self.app.is_connected {
                            ui.label(egui::RichText::new(
                                self.app.tr("Hardware Disconnected")
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
                                    self.app.tr("Loading hardware…")
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
                            let (board_card_title, board_card_subtitle) = board_card_labels(
                                display_language,
                                &capabilities,
                                self.app.tr("Connected board"),
                            );
                            let board_card_width = ui.available_width();
                            ui.set_width(board_card_width);
                            let board_card = egui::Frame::group(ui.style())
                                .inner_margin(egui::Margin::symmetric(14, 12))
                                .stroke(egui::Stroke::new(
                                    HARDWARE_CARD_STROKE_WIDTH,
                                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                                ))
                                .corner_radius(10.0)
                                .show(ui, |ui| {
                                    configure_hardware_card_controls(ui);
                                    ui.set_width(hardware_frame_content_width(board_card_width, 14));
                                    let row_height = if board_card_subtitle.is_some() {
                                        BOARD_IDENTITY_TWO_LINE_HEIGHT
                                    } else {
                                        28.0
                                    };
                                    let status_width = 138.0_f32.min(ui.available_width() * 0.42);
                                    let icon_width = 28.0;
                                    let spacing = ui.spacing().item_spacing.x;
                                    let identity_width = (ui.available_width()
                                        - icon_width
                                        - status_width
                                        - spacing * 2.0)
                                        .max(72.0);
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(ui.available_width(), row_height),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            ui.add_sized(
                                                [icon_width, row_height],
                                                egui::Label::new(
                                                    egui::RichText::new(crate::ui::icons::PLUG)
                                                        .size(22.0),
                                                ),
                                            );
                                            let name_response = draw_board_identity_block(
                                                ui,
                                                identity_width,
                                                row_height,
                                                &board_card_title,
                                                board_card_subtitle.as_deref(),
                                            );
                                            if name_response
                                                .on_hover_text(self.app.tr("Rename board"))
                                                .clicked()
                                            {
                                                open_board_information(
                                                    self.app,
                                                    &capabilities,
                                                    0,
                                                );
                                            }
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(status_width, row_height),
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui
                                                        .button(crate::ui::icons::INFO)
                                                        .on_hover_text(
                                                            self.app.tr("Board information"),
                                                        )
                                                        .clicked()
                                                    {
                                                        open_board_information(
                                                            self.app,
                                                            &capabilities,
                                                            0,
                                                        );
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
                                                    ui.add(
                                                        egui::Label::new(
                                                            self.app.tr("Connected"),
                                                        )
                                                        .truncate(),
                                                    );
                                                },
                                            );
                                        },
                                    );
                                });
                            board_card.response.context_menu(|ui| {
                                draw_board_card_context_menu(
                                    self.app,
                                    ui,
                                    &capabilities,
                                    &board_card_title,
                                    board_card_subtitle.as_deref(),
                                );
                            });

                            let motion_controls = crate::ui::hardware_control::managed_controls(
                                &capabilities,
                            )
                                .into_iter()
                                .filter(|control| is_motion_control(control))
                                .collect::<Vec<_>>();
                            if !motion_controls.is_empty() {
                                let title = format!(
                                    "{} ({})",
                                    self.app.tr("Motion / seat controls"),
                                    motion_controls.len()
                                );
                                hardware_section(
                                    ui,
                                    "hardware_motion_section",
                                    crate::ui::icons::SEAT,
                                    &title,
                                    true,
                                    |ui| {
                                        draw_control_card_grid(
                                            self.app,
                                            ui,
                                            &capabilities,
                                            &motion_controls,
                                        );
                                    },
                                );
                            }

                            let semantic_controls = capabilities
                                .board_profile
                                .as_ref()
                                .filter(|profile| profile.attached && profile.configured)
                                .map(|_| {
                                    capabilities.controls.iter()
                                        .filter(|control| {
                                            !is_motion_control(control)
                                                &&
                                            relay_id_from_control_key(&control.key).is_none()
                                                && !is_pwm_control(control)
                                        })
                                        .cloned()
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();
                            if !semantic_controls.is_empty() {
                                let title = format!(
                                    "{} ({})",
                                    self.app.tr("Board controls"),
                                    semantic_controls.len()
                                );
                                hardware_section(
                                    ui,
                                    "hardware_semantic_controls_section",
                                    crate::ui::icons::SLIDERS_HORIZONTAL,
                                    &title,
                                    true,
                                    |ui| {
                                        draw_control_card_grid(
                                            self.app,
                                            ui,
                                            &capabilities,
                                            &semantic_controls,
                                        );
                                    },
                                );
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
                                                group: if relay.id <= 4 {
                                                    self.app.tr("Raw relays")
                                                } else {
                                                    relay.role.clone()
                                                },
                                                ..Default::default()
                                            })
                                    })
                                    .filter(|control| global_control_is_visible(self.app, &capabilities, control))
                                    .collect::<Vec<_>>();
                                if !relay_controls.is_empty() {
                                    let title = format!(
                                        "{} ({})",
                                        self.app.tr("Relay outputs"),
                                        relay_controls.len()
                                    );
                                    hardware_section(
                                        ui,
                                        "hardware_relay_section",
                                        crate::ui::icons::PLUG,
                                        &title,
                                        true,
                                        |ui| {
                                            draw_control_card_grid(
                                                self.app,
                                                ui,
                                                &capabilities,
                                                &relay_controls,
                                            );
                                        },
                                    );
                                }
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
                                    .filter(|control| global_control_is_visible(self.app, &capabilities, control))
                                    .collect::<Vec<_>>();
                                let title = format!(
                                    "{} ({})",
                                    self.app.tr("PWM outputs"),
                                    pwm_controls.len()
                                );
                                hardware_section(
                                    ui,
                                    "hardware_pwm_section",
                                    crate::ui::icons::WAVEFORM,
                                    &title,
                                    true,
                                    |ui| {
                                        draw_control_card_grid(
                                            self.app,
                                            ui,
                                            &capabilities,
                                            &pwm_controls,
                                        );
                                    },
                                );
                            }

                            hardware_section(
                                ui,
                                "hardware_buzzer_section",
                                crate::ui::icons::SPEAKER_HIGH,
                                &self.app.tr("Buzzer & melodies"),
                                true,
                                |ui| {
                                    draw_buzzer_tool(self.app, ui, &capabilities);
                                },
                            );

                            if capabilities.strip_control.is_some() {
                                hardware_section(
                                    ui,
                                    "hardware_addressable_strip_section",
                                    crate::ui::icons::SPARKLE,
                                    &self.app.tr("Addressable LED strip"),
                                    true,
                                    |ui| {
                                        draw_addressable_strip_tool(
                                            self.app,
                                            ui,
                                            &capabilities,
                                        );
                                    },
                                );
                            }

                            if capabilities.supports_segment_display
                                || capabilities.supports_lcd_display
                                || capabilities.supports_rf_transmit
                            {
                                hardware_section(
                                    ui,
                                    "hardware_board_tools_section",
                                    crate::ui::icons::SLIDERS_HORIZONTAL,
                                    &self.app.tr("Board tools"),
                                    true,
                                    |ui| {
                                        if capabilities.supports_segment_display
                                            || capabilities.supports_lcd_display
                                        {
                                            draw_display_text_tool(self.app, ui, &capabilities);
                                        }
                                        if capabilities.supports_rf_transmit {
                                            if capabilities.supports_segment_display
                                                || capabilities.supports_lcd_display
                                            {
                                                ui.add_space(8.0);
                                            }
                                            draw_rf_code_tool(self.app, ui);
                                        }
                                    },
                                );
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

                        }

                        });

                        // mpv's render callback requests frames while video is
                        // advancing. An unconditional repaint here turned the
                        // no-media state (which starts unpaused) into an
                        // unlimited GPU render loop when V-Sync was disabled.
                    }
                    PealayerTab::Timeline => {
                        let timeline_track_height =
                            timeline_track_row_height(self.app.compact_timeline_tracks);
                        let timeline_analog_height =
                            timeline_analog_row_height(self.app.compact_timeline_tracks);
                        let timeline_filter_id =
                            egui::Id::new("timeline_track_filter_text");
                        let mut timeline_track_filter = ui.ctx().data_mut(|data| {
                            data.get_temp::<String>(timeline_filter_id)
                                .unwrap_or_default()
                        });
                        let all_timeline_rows = all_timeline_track_rows(self.app);
                        let timeline_rows = all_timeline_rows
                            .iter()
                            .filter(|row| {
                                row.linked
                                    && row.visible
                                    && !hardware_row_has_analog_track(self.app, row)
                                    && timeline_track_matches_filter(
                                        row,
                                        &timeline_track_filter,
                                    )
                            })
                            .cloned()
                            .collect::<Vec<_>>();
                        let linked_analog_track_ids = self
                            .app
                            .timeline
                            .analog_tracks
                            .iter()
                            .filter(|track| {
                                let key = crate::four_d::models::hardware_timeline_track_key(
                                    &format!("pwm.{}", track.channel),
                                );
                                all_timeline_rows
                                    .iter()
                                    .find(|row| row.key == key)
                                    .map(|row| row.linked)
                                    .unwrap_or_else(|| self.app.timeline.track_state(&key).linked)
                            })
                            .map(|track| track.id)
                            .collect::<std::collections::BTreeSet<_>>();
                        let visible_analog_track_ids = self
                            .app
                            .timeline
                            .analog_tracks
                            .iter()
                            .filter(|track| {
                                let key = crate::four_d::models::hardware_timeline_track_key(
                                    &format!("pwm.{}", track.channel),
                                );
                                let state = all_timeline_rows
                                    .iter()
                                    .find(|row| row.key == key)
                                    .map(|row| (row.linked, row.visible))
                                    .unwrap_or_else(|| {
                                        let state = self.app.timeline.track_state(&key);
                                        (state.linked, state.visible)
                                    });
                                let identity = format!("P{} {}", track.channel, track.name);
                                state.0
                                    && state.1
                                    && (timeline_track_filter.trim().is_empty()
                                        || identity.to_lowercase().contains(
                                            &timeline_track_filter.trim().to_lowercase(),
                                        ))
                            })
                            .map(|track| track.id)
                            .collect::<std::collections::BTreeSet<_>>();
                        let relay_solo_active = !self.app.track_soloed.is_empty();
                        let analog_solo_active = self.app.timeline.analog_tracks.iter().any(|track| track.soloed);
                        let can_add_keyframe = can_add_timeline_keyframe(
                            &timeline_rows,
                            visible_analog_track_ids.len(),
                            self.app.playback_time,
                        );
                        if timeline_rows.is_empty() && self.app.timeline.analog_tracks.is_empty() {
                            let message = if self.app.advertised_hardware().is_none() {
                                self.app.tr("Open media or connect PCController to populate the timeline.")
                            } else {
                                self.app.tr("No media or advertised hardware tracks are available.")
                            };
                            ui.label(message);
                        }
                        // Both panes must start at the same Y coordinate. `horizontal` uses
                        // center cross-axis alignment; after the fixed header column establishes
                        // a taller row, that centered the canvas lower by roughly one ruler band.
                        // `horizontal_top` makes the header ruler and canvas ruler share one
                        // origin regardless of visible track count or panel height.
                        let timeline_vertical_sync_id =
                            egui::Id::new("timeline-shared-vertical-offset");
                        let synced_vertical_offset = ui.ctx().data_mut(|data| {
                            data.get_persisted::<f32>(timeline_vertical_sync_id)
                                .unwrap_or(0.0)
                        });
                        let mut timeline_header_scroll_id = None;
                        let mut timeline_header_offset_y = synced_vertical_offset;
                        let mut pending_timeline_wheel = None;
                        let mut pending_timeline_toolbar_action = None;
                        let mut held_timeline_pan = 0.0;
                        ui.horizontal_top(|ui| {
                            // 1. Left column: Fixed Track Headers
                            ui.vertical(|ui| {
                                ui.set_width(TIMELINE_TRACK_HEADER_WIDTH);
                                let header_rect = egui::Rect::from_min_size(ui.cursor().min,
                                    egui::vec2(TIMELINE_TRACK_HEADER_WIDTH, ui.available_height()));
                                if ui.rect_contains_pointer(header_rect) {
                                    let modifiers = ui.input(timeline_wheel_modifiers);
                                    let behavior = if modifiers.is_none() {
                                        if self.app.timeline_header_wheel_vertical_scroll {
                                            crate::config::TimelineWheelBehavior::VerticalScroll
                                        } else { crate::config::TimelineWheelBehavior::None }
                                    } else { timeline_wheel_behavior(self.app, modifiers) };
                                    pending_timeline_wheel = timeline_wheel_over_surface(ui, header_rect, behavior);
                                }
                                // The canvas paints contiguous bands. Remove egui's default
                                // inter-widget gap so the fixed header rows have the exact same
                                // top/bottom coordinates instead of drifting farther on every row.
                                ui.spacing_mut().item_spacing.y = 0.0;

                                // Header occupies the exact same band as the right-side ruler.
                                let (header_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(
                                        TIMELINE_TRACK_HEADER_WIDTH,
                                        TIMELINE_RULER_HEIGHT,
                                    ),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(header_rect, 0.0, ui.visuals().panel_fill);
                                ui.painter().line_segment(
                                    [egui::pos2(header_rect.min.x, header_rect.max.y), egui::pos2(header_rect.max.x, header_rect.max.y)],
                                    ui.visuals().widgets.noninteractive.bg_stroke,
                                );
                                let mut header_ui = ui.new_child(
                                    egui::UiBuilder::new()
                                        .max_rect(header_rect.shrink2(egui::vec2(4.0, 1.0)))
                                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                                );
                                if header_ui
                                    .add_enabled(
                                        can_add_keyframe,
                                        egui::Button::new(crate::ui::icons::DIAMOND),
                                    )
                                    .on_hover_text(self.app.tr("Add exact timeline keyframe at playhead (K)"))
                                    .clicked()
                                {
                                    let time_ms = (self.app.playback_time * 1_000.0)
                                        .round()
                                        .max(0.0) as u64;
                                    self.app.insert_timeline_keyframe(time_ms);
                                    ui.ctx().request_repaint();
                                }
                                egui::containers::menu::MenuButton::from_button(
                                    egui::Button::new(crate::ui::icons::LIST_CHECKS),
                                )
                                .ui(&mut header_ui, |ui| {
                                    ui.set_min_width(245.0);
                                    ui.strong(self.app.tr("Timeline tracks"));
                                    ui.label(
                                        egui::RichText::new(self.app.tr(
                                            "Choose which sources are linked and visible.",
                                        ))
                                        .weak()
                                        .small(),
                                    );
                                    ui.separator();
                                    for row in &all_timeline_rows {
                                        let label = format!("{}  {}", row.icon, row.name);
                                        let label = if timeline_track_picker_item_is_dimmed(row) {
                                            egui::RichText::new(label)
                                                .color(ui.visuals().weak_text_color())
                                        } else {
                                            egui::RichText::new(label)
                                        };
                                        ui.menu_button(label, |ui| {
                                                let mut linked = row.linked;
                                                if ui
                                                    .checkbox(
                                                        &mut linked,
                                                        self.app.tr("Link to timeline"),
                                                    )
                                                    .changed()
                                                {
                                                    self.app.set_timeline_track_linked(
                                                        &row.key, linked,
                                                    );
                                                }
                                                let mut visible = row.visible;
                                                if ui
                                                    .add_enabled_ui(linked, |ui| {
                                                        ui.checkbox(
                                                            &mut visible,
                                                            self.app.tr("Show timeline track"),
                                                        )
                                                    })
                                                    .inner
                                                    .changed()
                                                {
                                                    self.app.set_timeline_track_visible(
                                                        &row.key, visible,
                                                    );
                                                }
                                                ui.separator();
                                                if media_track_key(row).is_some()
                                                    && ui
                                                        .button(format!(
                                                            "{}  {}",
                                                            crate::ui::icons::INFO,
                                                            self.app.tr("Properties...")
                                                        ))
                                                        .clicked()
                                                {
                                                    open_media_track_properties(self.app, row);
                                                    ui.close();
                                                }
                                                if ui
                                                    .button(format!(
                                                        "{}  {}",
                                                        crate::ui::icons::SLIDERS_HORIZONTAL,
                                                        self.app.tr("Manage...")
                                                    ))
                                                    .clicked()
                                                {
                                                    manage_timeline_track(self.app, row);
                                                    ui.close();
                                                }
                                                if ui
                                                    .button(format!(
                                                        "{}  {}",
                                                        crate::ui::icons::FRAME_CORNERS,
                                                        self.app.tr("Bring into view")
                                                    ))
                                                    .clicked()
                                                {
                                                    request_timeline_track_into_view(
                                                        self.app,
                                                        ui.ctx(),
                                                        timeline_filter_id,
                                                        row,
                                                    );
                                                    ui.close();
                                                }
                                            });
                                    }
                                });
                                let filter_response = header_ui.add_sized(
                                    [158.0, 22.0],
                                    crate::ui::dialog::singleline_text_edit(&mut timeline_track_filter)
                                        .hint_text(self.app.tr("Tracks"))
                                        .frame(egui::Frame::NONE)
                                        .margin(egui::Margin::symmetric(4, 2)),
                                );
                                if filter_response.changed() {
                                    header_ui.ctx().data_mut(|data| {
                                        data.insert_temp(
                                            timeline_filter_id,
                                            timeline_track_filter.clone(),
                                        );
                                    });
                                    header_ui.ctx().request_repaint();
                                }

                                let header_scroll = egui::ScrollArea::vertical()
                                    .id_salt("timeline_header_scroll")
                                    .max_height(ui.available_height())
                                    .auto_shrink([false, false])
                                    .scroll_bar_visibility(
                                        egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                                    )
                                    .wheel_scroll_multiplier(egui::vec2(
                                        0.0,
                                        0.0,
                                    ))
                                    .vertical_scroll_offset(synced_vertical_offset)
                                    .show(ui, |ui| {
                                ui.set_width(TIMELINE_TRACK_HEADER_WIDTH);
                                ui.spacing_mut().item_spacing.y = 0.0;
                                let rename_key_id = egui::Id::new("timeline_track_rename_key");
                                let rename_draft_id = egui::Id::new("timeline_track_rename_draft");
                                let rename_focus_id = egui::Id::new("timeline_track_rename_focus");
                                let timeline_order_capabilities = self.app.advertised_hardware();
                                let timeline_order_controls = timeline_order_capabilities
                                    .as_ref()
                                    .map(crate::ui::hardware_control::managed_controls)
                                    .unwrap_or_default();
                                let mut pending_timeline_order_drop = None;
                                let reorder_help = self.app.tr("Drag to reorder channel");
                                for track_row in &timeline_rows {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(
                                            TIMELINE_TRACK_HEADER_WIDTH,
                                            timeline_track_height,
                                        ),
                                        egui::Sense::hover(),
                                    );
                                    // A popup remains open across frames. Auto-generated row IDs
                                    // can be rebound to a different channel when the live catalog
                                    // inserts/removes/reorders rows, which made a relay menu render
                                    // a PWM channel on the next frame. Bind interaction identity to
                                    // the stable track key instead.
                                    let response = ui.interact(
                                        rect,
                                        timeline_track_row_id(&track_row.key),
                                        egui::Sense::click(),
                                    );
                                    let reorder_control = track_row.control_key.as_ref().and_then(
                                        |control_key| {
                                            timeline_order_controls
                                                .iter()
                                                .find(|control| control.key == *control_key)
                                                .cloned()
                                        },
                                    );
                                    let brought_into_view = scroll_requested_timeline_track_into_view(
                                        ui,
                                        &track_row.key,
                                        rect,
                                    );
                                    let selected_track = self
                                        .app
                                        .selected_timeline_track
                                        .as_deref()
                                        == Some(track_row.key.as_str());
                                    let row_muted = if matches!(track_row.kind, TimelineTrackKind::Audio(_)) {
                                        self.app.is_muted
                                    } else {
                                        !track_row.relay_ids.is_empty()
                                            && track_row.relay_ids.iter().all(|relay| self.app.track_muted.contains(relay))
                                    };
                                    let row_soloed = !track_row.relay_ids.is_empty()
                                        && track_row.relay_ids.iter().all(|relay| self.app.track_soloed.contains(relay));
                                    let row_locked = !track_row.relay_ids.is_empty()
                                        && track_row.relay_ids.iter().all(|relay| self.app.track_locked.contains(relay));
                                    let row_has_solo_context = relay_solo_active && !track_row.relay_ids.is_empty();
                                    let row_visual_opacity = timeline_track_visual_opacity(
                                        row_muted,
                                        row_soloed,
                                        row_locked,
                                        row_has_solo_context,
                                    );
                                    let row_fill = if brought_into_view || selected_track {
                                        ui.visuals().selection.bg_fill.gamma_multiply(0.24)
                                    } else if track_row.active {
                                        ui.visuals().selection.bg_fill.gamma_multiply(
                                            if ui.visuals().dark_mode { 0.16 } else { 0.08 },
                                        )
                                    } else {
                                        ui.visuals().faint_bg_color
                                    };
                                    ui.painter().rect_filled(rect, 0.0, row_fill);
                                    if row_visual_opacity < 1.0 {
                                        ui.painter().rect_filled(
                                            rect,
                                            0.0,
                                            egui::Color32::from_black_alpha(((1.0 - row_visual_opacity) * 105.0).round() as u8),
                                        );
                                    }
                                    let state_color = if row_muted {
                                        Some(timeline_track_state_color(TimelineTrackStateKind::Muted))
                                    } else if row_soloed {
                                        Some(timeline_track_state_color(TimelineTrackStateKind::Soloed))
                                    } else if row_locked {
                                        Some(timeline_track_state_color(TimelineTrackStateKind::Locked))
                                    } else {
                                        None
                                    };
                                    if let Some(state_color) = state_color {
                                        ui.painter().rect_filled(
                                            egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + 3.0, rect.max.y)),
                                            0.0,
                                            state_color,
                                        );
                                    }
                                    ui.painter().rect_stroke(rect, 0.0, ui.visuals().widgets.noninteractive.bg_stroke, egui::StrokeKind::Inside);

                                    // Keep the advertised icon, caption and actions vertically
                                    // centered in the same compact row used by the canvas.
                                    let mut child_ui = ui.new_child(
                                        egui::UiBuilder::new()
                                            .max_rect(rect)
                                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                                    );
                                    child_ui.set_clip_rect(child_ui.clip_rect().intersect(rect));
                                    if track_row.dimmed {
                                        child_ui.set_opacity(0.58);
                                    }
                                    let mut media_control_clicked = false;
                                    let editing_name = track_row.control_key.as_ref().is_some_and(
                                        |control_key| {
                                            child_ui.ctx().data(|data| {
                                                data.get_temp::<String>(rename_key_id).as_deref()
                                                    == Some(control_key.as_str())
                                            })
                                        },
                                    );
                                    let mut rename_commit = None;
                                    {
                                        let ui = &mut child_ui;
                                        ui.add_space(6.0);
                                        let icon_color = if track_row.active && track_row.enabled {
                                            ui.visuals().selection.bg_fill.gamma_multiply(row_visual_opacity)
                                        } else {
                                            ui.visuals().weak_text_color().gamma_multiply(row_visual_opacity)
                                        };
                                        let (icon_rect, _) = ui.allocate_exact_size(
                                            egui::vec2(18.0, timeline_track_height),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().with_clip_rect(icon_rect).text(
                                            icon_rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            &track_row.icon,
                                            egui::FontId::proportional(14.0),
                                            icon_color,
                                        );
                                        let label_width = if !track_row.relay_ids.is_empty() {
                                            116.0
                                        } else {
                                            158.0
                                        };
                                        let (identity_rect, identity_response) =
                                            ui.allocate_exact_size(
                                                egui::vec2(
                                                    label_width,
                                                    timeline_track_height,
                                                ),
                                                egui::Sense::hover(),
                                            );
                                        let mut identity_ui = ui.new_child(
                                            egui::UiBuilder::new()
                                                .max_rect(identity_rect)
                                                .layout(egui::Layout::top_down(egui::Align::Min)),
                                        );
                                        identity_ui.set_clip_rect(
                                            identity_ui.clip_rect().intersect(identity_rect),
                                        );
                                        identity_ui.spacing_mut().item_spacing.y = 0.0;
                                        {
                                            let ui = &mut identity_ui;
                                            if editing_name {
                                                let mut draft = ui.ctx().data(|data| {
                                                    data.get_temp::<String>(rename_draft_id)
                                                        .unwrap_or_else(|| track_row.name.clone())
                                                });
                                                let edit = ui.add_sized(
                                                    [label_width, timeline_track_height],
                                                    crate::ui::dialog::singleline_text_edit(&mut draft),
                                                );
                                                let focus = ui.ctx().data_mut(|data| {
                                                    data.get_temp::<bool>(rename_focus_id)
                                                        .unwrap_or(false)
                                                });
                                                if focus {
                                                    edit.request_focus();
                                                    ui.ctx().data_mut(|data| {
                                                        data.insert_temp(rename_focus_id, false)
                                                    });
                                                }
                                                ui.ctx().data_mut(|data| {
                                                    data.insert_temp(rename_draft_id, draft.clone())
                                                });
                                                let accept = edit.lost_focus()
                                                    || ui.input(|input| {
                                                        input.key_pressed(egui::Key::Enter)
                                                    });
                                                let cancel = ui.input(|input| {
                                                    input.key_pressed(egui::Key::Escape)
                                                });
                                                if accept && !cancel {
                                                    rename_commit = Some(draft);
                                                }
                                                if accept || cancel {
                                                    ui.ctx().data_mut(|data| {
                                                        data.remove::<String>(rename_key_id);
                                                        data.remove::<String>(rename_draft_id);
                                                    });
                                                }
                                            } else {
                                                // Track identity is painted against the left edge
                                                // explicitly. Widget allocation previously left
                                                // enough free width for egui to make hardware and
                                                // effect captions look centered even though the
                                                // label requested `Align::Min`.
                                                let painter = ui.painter().with_clip_rect(identity_rect);
                                                let title_color = ui.visuals().strong_text_color().gamma_multiply(row_visual_opacity);
                                                let detail_color = ui.visuals().weak_text_color().gamma_multiply(row_visual_opacity);
                                                let title_galley = egui::WidgetText::from(
                                                    egui::RichText::new(&track_row.name)
                                                        .size(11.0)
                                                        .strong(),
                                                )
                                                .into_galley(
                                                    ui,
                                                    Some(egui::TextWrapMode::Truncate),
                                                    label_width,
                                                    egui::TextStyle::Body,
                                                );
                                                if let Some(detail) = track_row.detail.as_deref() {
                                                    let title_pos = egui::pos2(
                                                        identity_rect.left(),
                                                        identity_rect.center().y
                                                            - 7.0
                                                            - title_galley.size().y * 0.5,
                                                    );
                                                    painter.galley(
                                                        title_pos,
                                                        title_galley,
                                                        title_color,
                                                    );
                                                    let detail_galley = egui::WidgetText::from(
                                                        egui::RichText::new(detail).size(9.0).weak(),
                                                    )
                                                    .into_galley(
                                                        ui,
                                                        Some(egui::TextWrapMode::Truncate),
                                                        label_width,
                                                        egui::TextStyle::Body,
                                                    );
                                                    let detail_pos = egui::pos2(
                                                        identity_rect.left(),
                                                        identity_rect.center().y
                                                            + 7.0
                                                            - detail_galley.size().y * 0.5,
                                                    );
                                                    painter.galley(
                                                        detail_pos,
                                                        detail_galley,
                                                        detail_color,
                                                    );
                                                } else {
                                                    let title_pos = egui::pos2(
                                                        identity_rect.left(),
                                                        identity_rect.center().y
                                                            - title_galley.size().y * 0.5,
                                                    );
                                                    painter.galley(
                                                        title_pos,
                                                        title_galley,
                                                        title_color,
                                                    );
                                                }
                                            }
                                        }
                                        identity_response.on_hover_text(match track_row.detail.as_deref() {
                                            Some(detail) => format!("{}\n{}", track_row.name, detail),
                                            None => track_row.name.clone(),
                                        });
                                        ui.allocate_ui_with_layout(
                                            ui.available_size(),
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                        if let Some(control) = reorder_control.as_ref() {
                                            timeline_track_drag_handle(
                                                ui,
                                                control,
                                                rect,
                                                &reorder_help,
                                            );
                                        }
                                        match &track_row.kind {
                                            TimelineTrackKind::Video(_) => {
                                                let selector = crate::ui::media_tracks::menu_button(
                                                    self.app,
                                                    ui,
                                                    crate::app::MediaTrackType::Video,
                                                    ("timeline-video-selector", &track_row.key),
                                                );
                                                media_control_clicked |= selector.clicked();
                                            }
                                            TimelineTrackKind::Audio(_) => {
                                                let selector = crate::ui::media_tracks::menu_button(
                                                    self.app,
                                                    ui,
                                                    crate::app::MediaTrackType::Audio,
                                                    ("timeline-audio-selector", &track_row.key),
                                                );
                                                media_control_clicked |= selector.clicked();
                                                let mute_help = if self.app.is_muted { self.app.tr("Unmute") } else { self.app.tr("Mute") };
                                                let mute = timeline_track_state_button(
                                                    ui,
                                                    self.app.is_muted,
                                                    TimelineTrackStateKind::Muted,
                                                    if self.app.is_muted { crate::ui::icons::SPEAKER_SLASH } else { crate::ui::icons::SPEAKER_HIGH },
                                                    &mute_help,
                                                );
                                                if mute.clicked() {
                                                    media_control_clicked = true;
                                                    self.app.toggle_audio_muted();
                                                }
                                            }
                                            TimelineTrackKind::Subtitle(track_id) => {
                                                let selector = crate::ui::media_tracks::menu_button(
                                                    self.app,
                                                    ui,
                                                    crate::app::MediaTrackType::Subtitle,
                                                    ("timeline-subtitle-selector", &track_row.key),
                                                );
                                                media_control_clicked |= selector.clicked();
                                                let visible = track_row.active && track_row.enabled;
                                                let visibility = ui
                                                    .selectable_label(
                                                        visible,
                                                        if visible {
                                                            crate::ui::icons::EYE
                                                        } else {
                                                            crate::ui::icons::EYE_SLASH
                                                        },
                                                    )
                                                    .on_hover_text(if visible {
                                                        self.app.tr("Hide subtitles")
                                                    } else {
                                                        self.app.tr("Show subtitles")
                                                    });
                                                if visibility.clicked() {
                                                    media_control_clicked = true;
                                                    if track_row.active {
                                                        self.app.set_subtitle_visibility(!visible);
                                                    } else {
                                                        self.app.select_media_track(
                                                            crate::app::MediaTrackKey {
                                                                kind: crate::app::MediaTrackType::Subtitle,
                                                                id: *track_id,
                                                            },
                                                        );
                                                    }
                                                }
                                            }
                                            _ if !track_row.relay_ids.is_empty() => {
                                            let locked = track_row.relay_ids.iter().all(|relay| {
                                                self.app.track_locked.contains(relay)
                                            });
                                            if timeline_track_state_button(ui, locked, TimelineTrackStateKind::Locked, crate::ui::icons::LOCK, &lock_help).clicked() {
                                                for relay in &track_row.relay_ids {
                                                    if locked {
                                                        self.app.track_locked.remove(relay);
                                                    } else {
                                                        self.app.track_locked.insert(*relay);
                                                    }
                                                }
                                            }

                                            let soloed = track_row.relay_ids.iter().all(|relay| {
                                                self.app.track_soloed.contains(relay)
                                            });
                                            if timeline_track_state_button(ui, soloed, TimelineTrackStateKind::Soloed, crate::ui::icons::TARGET, &relay_solo_help).clicked() {
                                                for relay in &track_row.relay_ids {
                                                    if soloed {
                                                        self.app.track_soloed.remove(relay);
                                                    } else {
                                                        self.app.track_soloed.insert(*relay);
                                                    }
                                                }
                                                let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                                let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                            }

                                            let muted = track_row.relay_ids.iter().all(|relay| {
                                                self.app.track_muted.contains(relay)
                                            });
                                            if timeline_track_state_button(ui, muted, TimelineTrackStateKind::Muted, crate::ui::icons::PROHIBIT, &relay_mute_help).clicked() {
                                                for relay in &track_row.relay_ids {
                                                    if muted {
                                                        self.app.track_muted.remove(relay);
                                                    } else {
                                                        self.app.track_muted.insert(*relay);
                                                    }
                                                }
                                                let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                                let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                            }
                                            }
                                            _ => {}
                                        }
                                            },
                                        );
                                    }
                                    if let (Some(control_key), Some(requested_name)) =
                                        (track_row.control_key.as_deref(), rename_commit)
                                    {
                                        if let Some(capabilities) = self.app.advertised_hardware() {
                                            if let Some(control) = crate::ui::hardware_control::managed_controls(&capabilities)
                                                .into_iter()
                                                .find(|control| control.key == control_key)
                                            {
                                                update_control_name(
                                                    self.app,
                                                    &capabilities,
                                                    &control,
                                                    requested_name,
                                                );
                                            }
                                        }
                                    }
                                    if let Some(control) = reorder_control.as_ref()
                                        && let Some(drop) =
                                            timeline_track_drop_target(ui, rect, control)
                                    {
                                        pending_timeline_order_drop = Some(drop);
                                    }
                                    if response.double_clicked() {
                                        if let Some(control_key) = track_row.control_key.as_ref() {
                                            response.ctx.data_mut(|data| {
                                                data.insert_temp(rename_key_id, control_key.clone());
                                                data.insert_temp(rename_draft_id, track_row.name.clone());
                                                data.insert_temp(rename_focus_id, true);
                                            });
                                        }
                                    }
                                    if response.clicked() && !media_control_clicked {
                                        self.app.selected_timeline_track =
                                            Some(track_row.key.clone());
                                        match &track_row.kind {
                                            TimelineTrackKind::Video(Some(track_id)) => {
                                                self.app.select_media_track(crate::app::MediaTrackKey {
                                                    kind: crate::app::MediaTrackType::Video,
                                                    id: *track_id,
                                                });
                                            }
                                            TimelineTrackKind::Audio(track_id) => {
                                                self.app.select_media_track(crate::app::MediaTrackKey {
                                                    kind: crate::app::MediaTrackType::Audio,
                                                    id: *track_id,
                                                });
                                            }
                                            TimelineTrackKind::Subtitle(track_id) => {
                                                self.app.select_media_track(crate::app::MediaTrackKey {
                                                    kind: crate::app::MediaTrackType::Subtitle,
                                                    id: *track_id,
                                                });
                                            }
                                            _ => {}
                                        }
                                    }
                                    response.context_menu(|ui| {
                                        if media_track_key(track_row).is_some() {
                                            if ui
                                                .button(format!(
                                                    "{}  {}",
                                                    crate::ui::icons::INFO,
                                                    self.app.tr("Properties...")
                                                ))
                                                .clicked()
                                            {
                                                open_media_track_properties(self.app, track_row);
                                                ui.close();
                                            }
                                            ui.separator();
                                        }
                                        if let Some(control_key) = track_row.control_key.as_ref() {
                                            let capabilities = self.app.advertised_hardware();
                                            let control = capabilities.as_ref().and_then(|capabilities| {
                                                crate::ui::hardware_control::managed_controls(capabilities)
                                                    .into_iter()
                                                    .find(|control| control.key == *control_key)
                                            });
                                            if let (Some(capabilities), Some(control)) =
                                                (capabilities.as_ref(), control.as_ref())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label(crate::ui::icons::control(
                                                        &control.kind,
                                                        &control.icon,
                                                    ));
                                                    ui.strong(crate::ui::i18n::visual_text(
                                                        self.app.language,
                                                        &control.name,
                                                    ));
                                                });
                                                ui.label(
                                                    egui::RichText::new(&control.key)
                                                        .monospace()
                                                        .weak()
                                                        .small(),
                                                );
                                                ui.separator();
                                                crate::ui::hardware_control::draw_timeline_live_control(
                                                    self.app,
                                                    ui,
                                                    capabilities,
                                                    control,
                                                );
                                                ui.separator();
                                                if relay_id_from_control_key(control_key).is_some() {
                                                    ui.menu_button(
                                                        format!("{}  {}", crate::ui::icons::PLUS, self.app.tr("Add cue at playhead")),
                                                        |ui| {
                                                            for (label, value, icon) in [
                                                                (self.app.tr("On"), 10_000, crate::ui::icons::POWER),
                                                                (self.app.tr("Off"), 0, crate::ui::icons::STOP_CIRCLE),
                                                            ] {
                                                                if ui.button(format!("{icon}  {label}")).clicked() {
                                                                    let start = (self.app.playback_time * 1_000.0).round().max(0.0) as u64;
                                                                    if let Err(error) = self.app.add_direct_control_cue(control_key, value, start, 1_000) {
                                                                        self.app.set_osd(error);
                                                                    }
                                                                    ui.close();
                                                                }
                                                            }
                                                        },
                                                    );
                                                    ui.separator();
                                                } else if is_pwm_control(control) {
                                                    ui.menu_button(
                                                        format!("{}  {}", crate::ui::icons::PLUS, self.app.tr("Add value cue at playhead")),
                                                        |ui| {
                                                            for percent in [0_u16, 25, 50, 75, 100] {
                                                                if ui.button(format!("{percent}%")).clicked() {
                                                                    let start = (self.app.playback_time * 1_000.0).round().max(0.0) as u64;
                                                                    if let Err(error) = self.app.add_direct_control_cue(control_key, percent * 100, start, 1_000) {
                                                                        self.app.set_osd(error);
                                                                    }
                                                                    ui.close();
                                                                }
                                                            }
                                                        },
                                                    );
                                                    ui.separator();
                                                }
                                            }
                                            if ui
                                                .button(format!(
                                                    "{}  {}",
                                                    crate::ui::icons::PENCIL_SIMPLE,
                                                    self.app.tr("Rename")
                                                ))
                                                .clicked()
                                            {
                                                ui.ctx().data_mut(|data| {
                                                    data.insert_temp(rename_key_id, control_key.clone());
                                                    data.insert_temp(rename_draft_id, track_row.name.clone());
                                                    data.insert_temp(rename_focus_id, true);
                                                });
                                                ui.close();
                                            }
                                            if ui
                                                .button(format!(
                                                    "{}  {}",
                                                    crate::ui::icons::SLIDERS_HORIZONTAL,
                                                    self.app.tr("Manage...")
                                                ))
                                                .clicked()
                                            {
                                                if let Some(capabilities) = self.app.advertised_hardware() {
                                                    if let Some(control) = crate::ui::hardware_control::managed_controls(&capabilities)
                                                        .into_iter()
                                                        .find(|control| control.key == *control_key)
                                                    {
                                                        open_control_dialog(self.app, &capabilities, &control);
                                                    }
                                                }
                                                ui.close();
                                            }
                                            ui.separator();
                                        }
                                        if ui
                                            .button(format!(
                                                "{}  {}",
                                                crate::ui::icons::EYE_SLASH,
                                                self.app.tr("Hide timeline track")
                                            ))
                                            .clicked()
                                        {
                                            self.app
                                                .set_timeline_track_visible(&track_row.key, false);
                                            ui.close();
                                        }
                                        if ui
                                            .button(format!(
                                                "{}  {}",
                                                crate::ui::icons::LINK,
                                                self.app.tr("Unlink from timeline")
                                            ))
                                            .clicked()
                                        {
                                            self.app
                                                .set_timeline_track_linked(&track_row.key, false);
                                            ui.close();
                                        }
                                    });
                                }

                                // Analog Curve Track Headers
                                let mut analog_tracks_changed = false;
                                let mut analog_track_action: Option<(String, bool, bool)> = None;
                                let mut analog_rename_action: Option<(String, String)> = None;
                                let mut analog_manage_action: Option<String> = None;
                                let mut analog_direct_cue_action: Option<(String, u16)> = None;
                                let hide_timeline_track_label =
                                    self.app.tr("Hide timeline track");
                                let unlink_timeline_track_label =
                                    self.app.tr("Unlink from timeline");
                                let rename_track_label = self.app.tr("Rename");
                                let manage_track_label = self.app.tr("Manage...");
                                let add_value_cue_label = self.app.tr("Add value cue at playhead");
                                let analog_menu_capabilities = timeline_order_capabilities.clone();
                                let analog_menu_controls = timeline_order_controls.clone();
                                let analog_menu_sender = self.app.engine_handle.sender.clone();
                                let analog_menu_live_updates = self.app.live_pwm_updates;
                                let analog_menu_estop = self.app.estop_active;
                                for track in self
                                    .app
                                    .timeline
                                    .analog_tracks
                                    .iter_mut()
                                    .filter(|track| visible_analog_track_ids.contains(&track.id))
                                {
                                    let track_key =
                                        crate::four_d::models::hardware_timeline_track_key(
                                            &format!("pwm.{}", track.channel),
                                        );
                                    let control_key = format!("pwm.{}", track.channel);
                                    let advertised_row = all_timeline_rows
                                        .iter()
                                        .find(|row| row.key == track_key);
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(
                                            TIMELINE_TRACK_HEADER_WIDTH,
                                            timeline_analog_height,
                                        ),
                                        egui::Sense::hover(),
                                    );
                                    let response = ui.interact(
                                        rect,
                                        timeline_analog_track_row_id(&track_key),
                                        egui::Sense::click(),
                                    );
                                    let reorder_control = analog_menu_controls
                                        .iter()
                                        .find(|control| control.key == control_key)
                                        .cloned();
                                    let brought_into_view = scroll_requested_timeline_track_into_view(
                                        ui,
                                        &track_key,
                                        rect,
                                    );
                                    let selected_track = self
                                        .app
                                        .selected_timeline_track
                                        .as_deref()
                                        == Some(track_key.as_str());
                                    let track_visual_opacity = timeline_track_visual_opacity(
                                        track.muted,
                                        track.soloed,
                                        track.locked,
                                        analog_solo_active,
                                    );
                                    let row_fill = if brought_into_view || selected_track {
                                        ui.visuals().selection.bg_fill.gamma_multiply(0.24)
                                    } else {
                                        ui.visuals().extreme_bg_color
                                    };
                                    ui.painter().rect_filled(rect, 0.0, row_fill);
                                    if track_visual_opacity < 1.0 {
                                        ui.painter().rect_filled(
                                            rect,
                                            0.0,
                                            egui::Color32::from_black_alpha(((1.0 - track_visual_opacity) * 105.0).round() as u8),
                                        );
                                    }
                                    let state_color = if track.muted {
                                        Some(timeline_track_state_color(TimelineTrackStateKind::Muted))
                                    } else if track.soloed {
                                        Some(timeline_track_state_color(TimelineTrackStateKind::Soloed))
                                    } else if track.locked {
                                        Some(timeline_track_state_color(TimelineTrackStateKind::Locked))
                                    } else {
                                        None
                                    };
                                    if let Some(state_color) = state_color {
                                        ui.painter().rect_filled(
                                            egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + 3.0, rect.max.y)),
                                            0.0,
                                            state_color,
                                        );
                                    }
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

                                    let mut child_ui = ui.new_child(
                                        egui::UiBuilder::new()
                                            .max_rect(rect)
                                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                                    );
                                    if advertised_row.is_some_and(|row| row.dimmed) {
                                        child_ui.set_opacity(0.58);
                                    }
                                    child_ui.horizontal(|ui| {
                                        ui.add_space(6.0);
                                        ui.allocate_ui_with_layout(
                                            egui::vec2(18.0, timeline_analog_height),
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.label(egui::RichText::new(
                                                    advertised_row
                                                        .map(|row| row.icon.as_str())
                                                        .unwrap_or(crate::ui::icons::SLIDERS_HORIZONTAL),
                                                ).color(ui.visuals().text_color().gamma_multiply(track_visual_opacity)));
                                            },
                                        );
                                        ui.allocate_ui(egui::vec2(88.0, 24.0), |ui| {
                                            let track_name = crate::ui::i18n::visual_text(display_language, &track.name);
                                            let editing = ui.ctx().data(|data| {
                                                data.get_temp::<String>(rename_key_id).as_deref()
                                                    == Some(control_key.as_str())
                                            });
                                            if editing {
                                                let mut draft = ui.ctx().data(|data| {
                                                    data.get_temp::<String>(rename_draft_id)
                                                        .unwrap_or_else(|| track.name.clone())
                                                });
                                                let edit = ui.add_sized(
                                                    [88.0, 22.0],
                                                    crate::ui::dialog::singleline_text_edit(&mut draft),
                                                );
                                                let focus = ui.ctx().data_mut(|data| {
                                                    data.get_temp::<bool>(rename_focus_id)
                                                        .unwrap_or(false)
                                                });
                                                if focus {
                                                    edit.request_focus();
                                                    ui.ctx().data_mut(|data| data.insert_temp(rename_focus_id, false));
                                                }
                                                ui.ctx().data_mut(|data| data.insert_temp(rename_draft_id, draft.clone()));
                                                let accept = edit.lost_focus()
                                                    || ui.input(|input| input.key_pressed(egui::Key::Enter));
                                                let cancel = ui.input(|input| input.key_pressed(egui::Key::Escape));
                                                if accept && !cancel {
                                                    analog_rename_action =
                                                        Some((control_key.clone(), draft));
                                                }
                                                if accept || cancel {
                                                    ui.ctx().data_mut(|data| {
                                                        data.remove::<String>(rename_key_id);
                                                        data.remove::<String>(rename_draft_id);
                                                    });
                                                }
                                            } else {
                                                let (name_rect, name_response) = ui.allocate_exact_size(
                                                    egui::vec2(88.0, 24.0),
                                                    egui::Sense::hover(),
                                                );
                                                let name_galley = egui::WidgetText::from(
                                                    egui::RichText::new(&track_name)
                                                        .size(10.5)
                                                        .strong(),
                                                )
                                                .into_galley(
                                                    ui,
                                                    Some(egui::TextWrapMode::Truncate),
                                                    name_rect.width(),
                                                    egui::TextStyle::Body,
                                                );
                                                let name_pos = egui::pos2(
                                                    name_rect.left(),
                                                    name_rect.center().y
                                                        - name_galley.size().y * 0.5,
                                                );
                                                ui.painter().with_clip_rect(name_rect).galley(
                                                    name_pos,
                                                    name_galley,
                                                    ui.visuals().strong_text_color().gamma_multiply(track_visual_opacity),
                                                );
                                                name_response.on_hover_text(format!(
                                                    "{analog_track_label}: {}\n{port_channel_label}: P{}",
                                                    track_name, track.channel
                                                ));
                                            }
                                        });
                                        ui.allocate_ui_with_layout(
                                            ui.available_size(),
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {

                                        if let Some(control) = reorder_control.as_ref() {
                                            timeline_track_drag_handle(
                                                ui,
                                                control,
                                                rect,
                                                &reorder_help,
                                            );
                                        }

                                        let add_btn = ui
                                            .add_enabled(
                                                can_add_keyframe && !track.locked,
                                                egui::Button::new(crate::ui::icons::DIAMOND),
                                            )
                                            .on_hover_text(&add_keyframe_help);
                                        if add_btn.clicked() {
                                            let cur_ms = (self.app.playback_time * 1000.0) as u64;
                                            let cur_val = track.evaluate(cur_ms);
                                            track.add_keyframe(crate::four_d::curve::Keyframe::new(cur_ms, cur_val, crate::four_d::curve::Interpolation::Linear));
                                            analog_tracks_changed = true;
                                        }

                                        // Record Arm Button [●]
                                        let arm_color = if track.armed {
                                            egui::Color32::from_rgb(255, 60, 60)
                                        } else {
                                            egui::Color32::from_rgb(120, 120, 120)
                                        };
                                        let arm_btn = ui
                                            .add_enabled_ui(!track.locked, |ui| {
                                                ui.selectable_label(
                                                    track.armed,
                                                    egui::RichText::new(crate::ui::icons::RECORD)
                                                        .size(12.0)
                                                        .color(arm_color),
                                                )
                                            })
                                            .inner
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
                                            if ui
                                                .add_enabled_ui(!track.locked, |ui| {
                                                    ui.add_sized([65.0, 16.0], slider)
                                                })
                                                .inner
                                                .on_hover_text(&live_fader_help)
                                                .changed()
                                            {
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

                                        let lock_btn = timeline_track_state_button(ui, track.locked, TimelineTrackStateKind::Locked, crate::ui::icons::LOCK, &lock_help);
                                        if lock_btn.clicked() {
                                            track.locked = !track.locked;
                                            if track.locked {
                                                track.armed = false;
                                            }
                                            analog_tracks_changed = true;
                                        }

                                        let solo_btn = timeline_track_state_button(ui, track.soloed, TimelineTrackStateKind::Soloed, crate::ui::icons::TARGET, &relay_solo_help);
                                        if solo_btn.clicked() {
                                            track.soloed = !track.soloed;
                                            analog_tracks_changed = true;
                                        }

                                        let mute_btn = timeline_track_state_button(ui, track.muted, TimelineTrackStateKind::Muted, crate::ui::icons::PROHIBIT, &actuator_mute_help);
                                        if mute_btn.clicked() {
                                            track.muted = !track.muted;
                                            analog_tracks_changed = true;
                                        }
                                            },
                                        );
                                    });
                                    if let Some(control) = reorder_control.as_ref()
                                        && let Some(drop) =
                                            timeline_track_drop_target(ui, rect, control)
                                    {
                                        pending_timeline_order_drop = Some(drop);
                                    }
                                    if response.clicked() {
                                        self.app.selected_timeline_track = Some(track_key.clone());
                                    }
                                    if response.double_clicked() {
                                        response.ctx.data_mut(|data| {
                                            data.insert_temp(rename_key_id, control_key.clone());
                                            data.insert_temp(rename_draft_id, track.name.clone());
                                            data.insert_temp(rename_focus_id, true);
                                        });
                                    }
                                    response.context_menu(|ui| {
                                        if let Some(capabilities) = analog_menu_capabilities.as_ref()
                                            && let Some(control) = analog_menu_controls
                                                .iter()
                                                .find(|control| control.key == control_key)
                                        {
                                            ui.horizontal(|ui| {
                                                ui.label(crate::ui::icons::control(
                                                    &control.kind,
                                                    &control.icon,
                                                ));
                                                ui.strong(crate::ui::i18n::visual_text(
                                                    display_language,
                                                    &control.name,
                                                ));
                                            });
                                            ui.label(
                                                egui::RichText::new(&control.key)
                                                    .monospace()
                                                    .weak()
                                                    .small(),
                                            );
                                            ui.separator();
                                            if let Some(channel) = capabilities
                                                .pwm_channels
                                                .iter()
                                                .find(|channel| channel.key == control_key)
                                            {
                                                let value_id = ui.make_persistent_id((
                                                    "timeline_context_pwm_value",
                                                    &control_key,
                                                ));
                                                let telemetry_raw = capabilities
                                                    .telemetry
                                                    .pwm_values
                                                    .get(usize::from(channel.id))
                                                    .copied()
                                                    .flatten()
                                                    .or_else(|| {
                                                        (capabilities.telemetry.pwm_channel
                                                            == Some(channel.id))
                                                        .then_some(
                                                            capabilities
                                                                .telemetry
                                                                .pwm_value
                                                                .unwrap_or(0),
                                                        )
                                                    })
                                                    .unwrap_or(0);
                                                let mut percent = ui
                                                    .data_mut(|data| data.get_temp::<f64>(value_id))
                                                    .unwrap_or_else(|| {
                                                        capabilities.pwm_percent(
                                                            channel.id,
                                                            telemetry_raw,
                                                        )
                                                    });
                                                let pwm_response = draw_pwm_editor_row(
                                                    ui,
                                                    &mut percent,
                                                    !analog_menu_estop && !control.locked,
                                                );
                                                ui.data_mut(|data| {
                                                    data.insert_temp(value_id, percent)
                                                });
                                                transmit_pwm_editor_response_with(
                                                    &analog_menu_sender,
                                                    analog_menu_live_updates,
                                                    ui,
                                                    channel.id,
                                                    pwm_raw(percent),
                                                    pwm_response,
                                                );
                                                if ui
                                                    .button(format!(
                                                        "{}  {} ({:.2}%)",
                                                        crate::ui::icons::PLUS,
                                                        add_value_cue_label,
                                                        percent,
                                                    ))
                                                    .clicked()
                                                {
                                                    analog_direct_cue_action = Some((
                                                        control_key.clone(),
                                                        (percent.clamp(0.0, 100.0) * 100.0).round() as u16,
                                                    ));
                                                    ui.close();
                                                }
                                            }
                                            ui.separator();
                                        }
                                        if ui
                                            .button(format!(
                                                "{}  {}",
                                                crate::ui::icons::PENCIL_SIMPLE,
                                                rename_track_label
                                            ))
                                            .clicked()
                                        {
                                            ui.ctx().data_mut(|data| {
                                                data.insert_temp(rename_key_id, control_key.clone());
                                                data.insert_temp(rename_draft_id, track.name.clone());
                                                data.insert_temp(rename_focus_id, true);
                                            });
                                            ui.close();
                                        }
                                        if ui
                                            .button(format!(
                                                "{}  {}",
                                                crate::ui::icons::SLIDERS_HORIZONTAL,
                                                manage_track_label
                                            ))
                                            .clicked()
                                        {
                                            analog_manage_action = Some(control_key.clone());
                                            ui.close();
                                        }
                                        ui.separator();
                                        if ui
                                            .button(format!(
                                                "{}  {}",
                                                crate::ui::icons::EYE_SLASH,
                                                hide_timeline_track_label
                                            ))
                                            .clicked()
                                        {
                                            analog_track_action =
                                                Some((track_key.clone(), true, false));
                                            ui.close();
                                        }
                                        if ui
                                            .button(format!(
                                                "{}  {}",
                                                crate::ui::icons::LINK,
                                                unlink_timeline_track_label
                                            ))
                                            .clicked()
                                        {
                                            analog_track_action =
                                                Some((track_key.clone(), false, false));
                                            ui.close();
                                        }
                                    });
                                }
                                if let Some((key, keep_linked, visible)) = analog_track_action {
                                    if keep_linked {
                                        self.app.set_timeline_track_visible(&key, visible);
                                    } else {
                                        self.app.set_timeline_track_linked(&key, false);
                                    }
                                }
                                if let Some((control_key, requested_name)) = analog_rename_action {
                                    if let Some(capabilities) = self.app.advertised_hardware() {
                                        if let Some(control) = crate::ui::hardware_control::managed_controls(&capabilities)
                                            .into_iter()
                                            .find(|control| control.key == control_key)
                                        {
                                            update_control_name(
                                                self.app,
                                                &capabilities,
                                                &control,
                                                requested_name,
                                            );
                                        }
                                    }
                                }
                                if let Some(control_key) = analog_manage_action {
                                    if let Some(capabilities) = self.app.advertised_hardware() {
                                        if let Some(control) = crate::ui::hardware_control::managed_controls(&capabilities)
                                            .into_iter()
                                            .find(|control| control.key == control_key)
                                        {
                                            open_control_dialog(self.app, &capabilities, &control);
                                        }
                                    }
                                }
                                if let Some((control_key, value)) = analog_direct_cue_action {
                                    let start = (self.app.playback_time * 1_000.0).round().max(0.0) as u64;
                                    if let Err(error) = self.app.add_direct_control_cue(
                                        &control_key,
                                        value,
                                        start,
                                        1_000,
                                    ) {
                                        self.app.set_osd(error);
                                    }
                                }
                                if analog_tracks_changed {
                                    self.app.commit_timeline_edit();
                                }
                                if let (Some(capabilities), Some(drop)) = (
                                    timeline_order_capabilities.as_ref(),
                                    pending_timeline_order_drop,
                                ) {
                                    persist_channel_drop(self.app, capabilities, drop);
                                } else {
                                    clear_released_timeline_track_drag(ui);
                                }
                                });
                                timeline_header_scroll_id = Some(header_scroll.id);
                                timeline_header_offset_y = header_scroll.state.offset.y;
                            });

                            // 2. Right column: Scrollable Timeline Grid
                            let scroll_modifiers = ui.input(timeline_wheel_modifiers);
                            let zoom = self.app.timeline_zoom;
                            let px_per_ms = zoom / 1000.0;
                            let total_seconds = if self.app.duration > 0.0 { self.app.duration } else { 60.0 };
                            let total_width = (total_seconds * zoom as f64) as f32;
                            let num_analog = visible_analog_track_ids.len();
                            let track_area_height =
                                timeline_rows.len() as f32 * timeline_track_height;
                            let total_height = timeline_content_height(
                                timeline_rows.len(),
                                num_analog,
                                timeline_track_height,
                                timeline_analog_height,
                            );

                            // The interactive timeline canvas itself is the drop
                            // target. Wrapping it in `dnd_drop_zone` made the outer
                            // response fail `contains_pointer`: the inner
                            // click-and-drag canvas correctly owned the pointer and
                            // occluded its parent, so releases were never accepted.
                            let popup_was_open = egui::Popup::is_any_open(ui.ctx());
                            let mut keyframe_hit = false;
                            let mut keyframe_context_owned = false;
                            let timeline_scroll = egui::ScrollArea::both()
                                .id_salt("timeline_scroll")
                                .content_margin(egui::Margin::ZERO)
                                .vertical_scroll_offset(timeline_header_offset_y)
                                .show_viewport(ui, |ui, viewport| {
                                        let size = egui::vec2(total_width, total_height);
                                        let (_, rect) = ui.allocate_space(size);
                                        // Keyboard focus must target the real canvas widget. A
                                        // synthetic memory-only ID is absent from AccessKit and
                                        // crashes Windows on any mouse button when it receives focus.
                                        let response = ui.interact(
                                            rect,
                                            timeline_keyboard_focus_id(),
                                            egui::Sense::click_and_drag(),
                                        );

                                        let viewport_clip = ui.clip_rect();
                                        let mut ruler_rect = timeline_frozen_ruler_rect(rect, viewport);
                                        // Anchor the band to the viewport, not the cancellation
                                        // of fractional content/scroll offsets. Keep pan ticks
                                        // inside this fixed, physical-pixel-aligned header.
                                        let ppp = ui.ctx().pixels_per_point();
                                        let top = (viewport_clip.top() * ppp).round() / ppp;
                                        ruler_rect.min.y = top;
                                        ruler_rect.max.y = ((top + TIMELINE_RULER_HEIGHT) * ppp).round() / ppp;
                                        let track_clip = egui::Rect::from_min_max(
                                            egui::pos2(viewport_clip.left(), ruler_rect.bottom()),
                                            viewport_clip.max,
                                        ).intersect(viewport_clip);
                                        let painter = ui.painter().with_clip_rect(track_clip);
                                        let ruler_painter = ui.painter().with_clip_rect(ruler_rect.intersect(viewport_clip));

                                        let visible_start_sec = (((viewport_clip.min.x - 50.0 - rect.min.x) / zoom) as f64).max(0.0);
                                        let visible_end_sec = (((viewport_clip.max.x + 50.0 - rect.min.x) / zoom) as f64).min(total_seconds);
                                        let visible_start_i = (visible_start_sec.floor() as i32).max(0);
                                        let visible_end_i = (visible_end_sec.ceil() as i32).min(total_seconds.ceil() as i32);

                                        // Draw timeline tracks background
                                        painter.rect_filled(rect, 0.0, ui.visuals().panel_fill);

                                        let tracks_top = rect.min.y + TIMELINE_RULER_HEIGHT;

                                        // The ruler stays at the viewport top; track geometry
                                        // still scrolls with content and is clipped below it.

                                        let pointer_pos = ui.ctx().pointer_latest_pos();

                                        let mut keyframe_candidates: Vec<_> = self.app.timeline.keyframes.iter()
                                            .map(|marker| (TimelineKeyframeTarget::Marker(marker.id),
                                                timeline_keyframe_marker_center(ruler_rect, rect.min.x + marker.time_ms as f32 * px_per_ms)))
                                            .collect();
                                        for (index, track) in self.app.timeline.analog_tracks.iter()
                                            .filter(|track| visible_analog_track_ids.contains(&track.id)).enumerate()
                                        {
                                            let bottom = tracks_top + track_area_height
                                                + (index + 1) as f32 * timeline_analog_height - 4.0;
                                            keyframe_candidates.extend(track.keyframes.iter().enumerate().map(|(index, keyframe)| (
                                                TimelineKeyframeTarget::Analog(track.id, index),
                                                egui::pos2(rect.min.x + keyframe.time_ms as f32 * px_per_ms,
                                                    bottom - keyframe.value * (timeline_analog_height - 8.0)),
                                            )));
                                        }
                                        keyframe_candidates.retain(|(target, center)| match target {
                                            TimelineKeyframeTarget::Marker(_) => ruler_rect.intersect(viewport_clip).contains(*center),
                                            TimelineKeyframeTarget::Analog(_, _) => track_clip.contains(*center),
                                        });
                                        let nearest_keyframe = nearest_timeline_keyframe(
                                            pointer_pos.filter(|pos| ui.clip_rect().contains(*pos)), &keyframe_candidates,
                                        );
                                        keyframe_hit = nearest_keyframe.is_some();
                                        keyframe_context_owned = keyframe_hit || keyframe_candidates.iter()
                                            .any(|(target, _)| egui::Popup::is_id_open(ui.ctx(), target.menu_id()));

                                        let ruler_response = ui.interact(ruler_rect, egui::Id::new("timeline_ruler"), egui::Sense::click_and_drag())
                                            .on_hover_text(&timeline_ruler_help);

                                        // Capture the ruler position before opening the menu. Once
                                        // the popup is visible, the live pointer is over the menu,
                                        // not the ruler; deriving the timestamp from it made exact
                                        // keyframes jump to an unrelated (usually earlier) time.
                                        let ruler_context_requested = !keyframe_context_owned
                                            && ruler_rect.contains(pointer_pos.unwrap_or_default())
                                            && ui.input(|input| input.pointer.button_released(egui::PointerButton::Secondary));
                                        if ruler_context_requested {
                                            if let Some(position) = pointer_pos {
                                                let pointer_ms = timeline_pointer_time_ms(
                                                    position.x,
                                                    rect.min.x,
                                                    px_per_ms,
                                                    (total_seconds * 1_000.0).round() as u64,
                                                );
                                                ui.ctx().data_mut(|data| {
                                                    data.insert_temp(timeline_ruler_context_time_id(), pointer_ms);
                                                });
                                            }
                                        }

                                        if !keyframe_context_owned { ruler_response.context_menu(|ui| {
                                            ui.label(egui::RichText::new(self.app.tr("Timeline keyframe")).strong());
                                            let playhead_ms = (self.app.playback_time * 1_000.0)
                                                .round()
                                                .clamp(0.0, total_seconds * 1_000.0)
                                                as u64;
                                            if ui
                                                .add_enabled(
                                                    can_add_keyframe,
                                                    egui::Button::new(format!(
                                                    "{} {}",
                                                    crate::ui::icons::DIAMOND,
                                                    self.app.tr("Add keyframe at playhead")
                                                    )),
                                                )
                                                .clicked()
                                            {
                                                self.app.insert_timeline_keyframe(playhead_ms);
                                                ui.ctx().request_repaint();
                                                ui.close();
                                            }
                                            let pointer_ms = ui.ctx().data(|data| {
                                                data.get_temp::<u64>(timeline_ruler_context_time_id())
                                            });
                                            if let Some(pointer_ms) = pointer_ms {
                                                if ui
                                                    .add_enabled(
                                                        can_add_keyframe,
                                                        egui::Button::new(timeline_two_line_menu_label(
                                                            ui,
                                                            crate::ui::icons::PUSH_PIN,
                                                            &self.app.tr("Add exact keyframe here"),
                                                            &crate::duration::format_time_value_ms(pointer_ms),
                                                        )),
                                                    )
                                                    .clicked()
                                                {
                                                    self.app.insert_timeline_keyframe(pointer_ms);
                                                    ui.ctx().request_repaint();
                                                    ui.close();
                                                }
                                            }
                                            ui.separator();
                                            if ui
                                                .button(format!(
                                                    "{} {}",
                                                    crate::ui::icons::TARGET,
                                                    self.app.tr("Bring playhead into view")
                                                ))
                                                .clicked()
                                            {
                                                pending_timeline_toolbar_action = Some(
                                                    TimelineToolbarAction::BringPlayheadIntoView,
                                                );
                                                ui.close();
                                            }
                                        }); }

                                        let toolbar = crate::ui::timeline_toolbar::draw(
                                            ui, self.app, viewport_clip, can_add_keyframe,
                                        );
                                        let toolbar_rect = toolbar.rect;
                                        if toolbar.action.is_some() {
                                            pending_timeline_toolbar_action = toolbar.action;
                                        }
                                        held_timeline_pan = toolbar.held_pan;

                                        let visible_ruler = ruler_rect.intersect(viewport_clip);
                                        let ruler_hovered = ui.rect_contains_pointer(visible_ruler)
                                            && !ui.rect_contains_pointer(toolbar_rect);
                                        let ruler_scroll_steps = if ruler_hovered {
                                            timeline_ruler_scroll_steps(ui)
                                        } else {
                                            0
                                        };

                                        if ruler_scroll_steps != 0 {
                                            self.app.step_timeline_frame(ruler_scroll_steps);
                                            ui.ctx().request_repaint();
                                        } else if ui.rect_contains_pointer(viewport_clip) {
                                            pending_timeline_wheel = timeline_wheel_over_surface(ui, viewport_clip,
                                                timeline_wheel_behavior(self.app, scroll_modifiers));
                                        }

                                        let mut clicked_any_keyframe = keyframe_context_owned && ui.input(|input|
                                            input.pointer.any_down() || input.pointer.any_released());
                                        for marker in self.app.timeline.keyframes.clone() {
                                            let marker_x = rect.min.x + marker.time_ms as f32 * px_per_ms;
                                            if marker_x < rect.min.x || marker_x > rect.max.x || marker_x < viewport_clip.min.x - 20.0 || marker_x > viewport_clip.max.x + 20.0 {
                                                continue;
                                            }
                                            // Interaction is registered here, but the marker is
                                            // painted after the ruler background and timeline
                                            // lanes. Painting it here used to make the opaque ruler
                                            // pass erase every inserted keyframe on the same frame.
                                            let marker_center = timeline_keyframe_marker_center(
                                                ruler_rect,
                                                marker_x,
                                            );
                                            let marker_rect = egui::Rect::from_center_size(
                                                marker_center,
                                                egui::vec2(32.0, 32.0),
                                            );
                                            let marker_response = ui
                                                .interact(marker_rect, egui::Id::new(("timeline-keyframe", marker.id)), egui::Sense::click())
                                                .on_hover_text(format!(
                                                    "{} · {}",
                                                    self.app.tr("Exact timeline keyframe"),
                                                    crate::duration::format_time_value_ms(marker.time_ms)
                                                ));
                                            let target = TimelineKeyframeTarget::Marker(marker.id);
                                            let targeted = nearest_keyframe == Some(target);
                                            if marker_response.clicked() || (targeted && ui.rect_contains_pointer(marker_rect)
                                                && ui.input(|input| input.pointer.button_pressed(egui::PointerButton::Primary)
                                                    || input.pointer.button_released(egui::PointerButton::Secondary))) {
                                                clicked_any_keyframe = true;
                                                self.app.selected_timeline_keyframe = Some(marker.id);
                                                self.app.selected_instance_ids.clear();
                                                self.app.selected_keyframes.clear();
                                            }
                                            keyframe_context_menu(ui, &marker_response, target, targeted, |ui| {
                                                ui.label(egui::RichText::new(self.app.tr("Exact timeline keyframe")).strong());
                                                let mut exact_time = marker.time_ms;
                                                ui.horizontal(|ui| {
                                                    ui.label(self.app.tr("Time"));
                                                    if ui
                                                        .add(crate::duration::time_value_drag(
                                                            &mut exact_time,
                                                            0..=(total_seconds * 1_000.0).round() as u64,
                                                            1.0,
                                                            self.app.human_readable_time_units,
                                                        ))
                                                        .changed()
                                                        && exact_time != marker.time_ms
                                                    {
                                                        self.app.undo_stack.push(self.app.snapshot_timeline());
                                                        if self
                                                            .app
                                                            .timeline
                                                            .move_keyframe(marker.id, exact_time)
                                                        {
                                                            self.app.commit_timeline_edit();
                                                        }
                                                    }
                                                });
                                                if ui
                                                    .button(format!(
                                                        "{} {}",
                                                        crate::ui::icons::SKIP_FORWARD,
                                                        self.app.tr("Jump to keyframe")
                                                    ))
                                                    .clicked()
                                                {
                                                    self.app.seek_absolute(marker.time_ms as f64 / 1_000.0);
                                                    ui.close();
                                                }
                                                if ui
                                                    .button(format!(
                                                        "{} {}",
                                                        crate::ui::icons::TRASH,
                                                        self.app.tr("Delete Keyframe")
                                                    ))
                                                    .clicked()
                                                {
                                                    self.app.undo_stack.push(self.app.snapshot_timeline());
                                                    if self.app.timeline.remove_keyframe(marker.id) {
                                                        self.app.commit_timeline_edit();
                                                    }
                                                    self.app.selected_timeline_keyframe = None;
                                                    ui.close();
                                                }
                                                ui.separator();
                                                if ui.button(format!("{} {}", crate::ui::icons::X, deselect_keyframe_label)).clicked() {
                                                    self.app.selected_timeline_keyframe = None;
                                                    ui.ctx().request_repaint();
                                                    ui.close();
                                                }
                                            });
                                        }

                                        let media_chapters = self.app.media_chapters();
                                        let active_chapter_index =
                                            self.app.active_media_chapter().map(|chapter| chapter.index);
                                        for chapter in &media_chapters {
                                            let marker_x =
                                                rect.min.x + chapter.time_seconds as f32 * zoom;
                                            if marker_x < rect.min.x || marker_x > rect.max.x || marker_x < viewport_clip.min.x - 20.0 || marker_x > viewport_clip.max.x + 20.0 {
                                                continue;
                                            }
                                            let marker_rect = egui::Rect::from_center_size(
                                                egui::pos2(marker_x, ruler_rect.min.y + 7.0),
                                                egui::vec2(18.0, 14.0),
                                            );
                                            let chapter_response = ui
                                                .interact(
                                                    marker_rect,
                                                    egui::Id::new(("media-chapter", chapter.index)),
                                                    egui::Sense::click(),
                                                )
                                                .on_hover_text(format!(
                                                    "{} · {}",
                                                    chapter.title,
                                                    crate::duration::format_time_value_ms(
                                                        (chapter.time_seconds * 1_000.0).round()
                                                            as u64
                                                    )
                                                ));
                                            if chapter_response.clicked() {
                                                clicked_any_keyframe = true;
                                                self.app.jump_to_media_chapter(chapter.index);
                                            }
                                            chapter_response.context_menu(|ui| {
                                                ui.label(
                                                    egui::RichText::new(&chapter.title).strong(),
                                                );
                                                ui.label(crate::duration::format_time_value_ms(
                                                    (chapter.time_seconds * 1_000.0).round() as u64,
                                                ));
                                                if ui
                                                    .button(format!(
                                                        "{} {}",
                                                        crate::ui::icons::TARGET,
                                                        self.app.tr("Jump to chapter")
                                                    ))
                                                    .clicked()
                                                {
                                                    self.app
                                                        .jump_to_media_chapter(chapter.index);
                                                    ui.close();
                                                }
                                            });
                                        }

                                        if let Some(pos) = pointer_pos {
                                            if timeline_ruler_owns_pointer(
                                                ruler_rect, toolbar_rect, pos,
                                                ruler_response.dragged_by(egui::PointerButton::Primary),
                                            )
                                                && !popup_was_open
                                                && !egui::Popup::is_any_open(ui.ctx())
                                                && !clicked_any_keyframe
                                                && self.app.active_drag.is_none()
                                                && self.app.lasso_origin.is_none()
                                                && self.app.active_keyframe_drag.is_none()
                                            {
                                                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                                if ui.input(|i| i.pointer.primary_down())
                                                    || ruler_response.dragged_by(egui::PointerButton::Primary)
                                                {
                                                    let relative_x = (pos.x - rect.min.x).max(0.0);
                                                    let target_time = ((relative_x / zoom) as f64).clamp(0.0, total_seconds);
                                                    self.app.scrub_to(target_time);
                                                    ui.ctx().request_repaint();
                                                }
                                            }
                                        }

                                        if self.app.is_scrubbing
                                            && (!ui.input(|i| i.pointer.primary_down())
                                                || ruler_response.drag_stopped_by(egui::PointerButton::Primary)
                                                || ruler_response.clicked())
                                        {
                                            let current_target = self.app.seek_pos.unwrap_or(self.app.playback_time);
                                            self.app.finish_scrub(current_target);
                                        }

                                        // Scrolled lane widgets must neither paint over nor steal
                                        // clicks from the frozen ruler/keyframes/chapter controls.
                                        ui.set_clip_rect(track_clip);

                                        // Draw grid lines
                                        // Major grid lines every second (zoom px)
                                        for i in visible_start_i..=visible_end_i {
                                            let grid_x = rect.min.x + (i as f32 * zoom);
                                            if grid_x >= viewport_clip.min.x - 1.0 && grid_x <= viewport_clip.max.x + 1.0 && grid_x <= rect.max.x {
                                                painter.line_segment(
                                                    [egui::pos2(grid_x, rect.min.y), egui::pos2(grid_x, rect.max.y)],
                                                    ui.visuals().widgets.noninteractive.bg_stroke,
                                                );
                                            }
                                        }

                                        // Draw horizontal track separators and semantic state backgrounds.
                                        for i in 0..=timeline_rows.len() {
                                            let grid_y =
                                                tracks_top + i as f32 * timeline_track_height;

                                            if let Some(track_row) = timeline_rows.get(i)
                                                && !track_row.relay_ids.is_empty()
                                            {
                                                let muted = track_row.relay_ids.iter().all(|relay| self.app.track_muted.contains(relay));
                                                let soloed = track_row.relay_ids.iter().all(|relay| self.app.track_soloed.contains(relay));
                                                let locked = track_row.relay_ids.iter().all(|relay| self.app.track_locked.contains(relay));
                                                let opacity = timeline_track_visual_opacity(muted, soloed, locked, relay_solo_active);
                                                let track_rect = egui::Rect::from_min_max(
                                                    egui::pos2(rect.min.x, grid_y),
                                                    egui::pos2(rect.max.x, grid_y + timeline_track_height),
                                                );
                                                if opacity < 1.0 {
                                                    painter.rect_filled(
                                                        track_rect,
                                                        0.0,
                                                        egui::Color32::from_black_alpha(((1.0 - opacity) * 92.0).round() as u8),
                                                    );
                                                }
                                                let tint = if muted {
                                                    Some(TimelineTrackStateKind::Muted)
                                                } else if soloed {
                                                    Some(TimelineTrackStateKind::Soloed)
                                                } else if locked {
                                                    Some(TimelineTrackStateKind::Locked)
                                                } else {
                                                    None
                                                };
                                                if let Some(kind) = tint {
                                                    painter.rect_filled(track_rect, 0.0, timeline_track_state_color(kind).gamma_multiply(0.07));
                                                }
                                            }

                                            painter.line_segment(
                                                [egui::pos2(rect.min.x, grid_y), egui::pos2(rect.max.x, grid_y)],
                                                ui.visuals().widgets.noninteractive.bg_stroke,
                                            );
                                        }

                                        // Horizontal separators for Analog tracks
                                        for (t_idx, _) in self
                                            .app
                                            .timeline
                                            .analog_tracks
                                            .iter()
                                            .filter(|track| visible_analog_track_ids.contains(&track.id))
                                            .enumerate()
                                        {
                                            let grid_y = tracks_top
                                                + track_area_height
                                                + (t_idx + 1) as f32 * timeline_analog_height;
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
                                                    let track_index = (relative_y
                                                        / timeline_track_height)
                                                        .floor()
                                                        as i32;
                                                    if let Some(target_r) = relay_for_timeline_row(&timeline_rows, track_index) {
                                                        let target_compatible = self.app.timeline.instances.iter()
                                                            .find(|i| i.id == drag.instance_id)
                                                            .and_then(|inst| self.app.timeline.templates.iter().find(|t| t.id == inst.effect_id))
                                                            .map(|tmpl| tmpl.target.is_compatible_with_relay(target_r))
                                                            .unwrap_or(true);

                                                        if !self.app.track_locked.contains(&target_r) && target_compatible {
                                                            let row_y = tracks_top
                                                                + track_index as f32
                                                                    * timeline_track_height;
                                                            let dest_rect = egui::Rect::from_min_max(
                                                                egui::pos2(rect.min.x, row_y),
                                                                egui::pos2(
                                                                    rect.max.x,
                                                                    row_y + timeline_track_height,
                                                                ),
                                                            );
                                                            painter.rect_filled(dest_rect, 0.0, egui::Color32::from_rgba_unmultiplied(46, 204, 113, 25)); // Faint green highlight
                                                        } else if !target_compatible && !self.app.track_locked.contains(&target_r) {
                                                            let row_y = tracks_top
                                                                + track_index as f32
                                                                    * timeline_track_height;
                                                            let dest_rect = egui::Rect::from_min_max(
                                                                egui::pos2(rect.min.x, row_y),
                                                                egui::pos2(
                                                                    rect.max.x,
                                                                    row_y + timeline_track_height,
                                                                ),
                                                            );
                                                            painter.rect_filled(dest_rect, 0.0, egui::Color32::from_rgba_unmultiplied(231, 76, 60, 30)); // Faint red warning highlight for incompatible track
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if self.app.duration > 0.0 {
                                            for (row_index, track_row) in timeline_rows.iter().enumerate() {
                                                let (icon, color) = match track_row.kind {
                                                    TimelineTrackKind::Video(_) => (crate::ui::icons::FILE_VIDEO, egui::Color32::from_rgb(41, 128, 185)),
                                                    TimelineTrackKind::Audio(_) => (crate::ui::icons::SPEAKER_HIGH, egui::Color32::from_rgb(39, 174, 96)),
                                                    TimelineTrackKind::Subtitle(_) => (crate::ui::icons::SUBTITLES, egui::Color32::from_rgb(124, 92, 190)),
                                                    TimelineTrackKind::ControllerEffect(_)
                                                    | TimelineTrackKind::Relay(_)
                                                    | TimelineTrackKind::Hardware(_) => continue,
                                                };
                                                let color = color.gamma_multiply(
                                                    if track_row.active && track_row.enabled { 1.0 } else { 0.52 },
                                                );
                                                let row_y = timeline_track_row_top(
                                                    rect.min.y,
                                                    row_index,
                                                    timeline_track_height,
                                                );
                                                let clip_rect = egui::Rect::from_min_max(
                                                    egui::pos2(rect.min.x, row_y + 4.0),
                                                    egui::pos2(
                                                        rect.min.x + total_width,
                                                        row_y + timeline_track_height - 4.0,
                                                    ),
                                                );
                                                painter.rect_filled(clip_rect, 4.0, color);
                                                painter.rect_stroke(clip_rect, 4.0, egui::Stroke::new(1.0_f32, egui::Color32::WHITE), egui::StrokeKind::Inside);
                                                painter.text(
                                                    clip_rect.left_center() + egui::vec2(10.0, 0.0),
                                                    egui::Align2::LEFT_CENTER,
                                                    format!("{icon}  {}", track_row.name),
                                                    egui::FontId::proportional(11.0),
                                                    egui::Color32::WHITE.gamma_multiply(
                                                        if track_row.enabled { 1.0 } else { 0.68 },
                                                    ),
                                                );
                                            }
                                        }

                                        let mut clicked_any_clip = false;
                                        let mut started_drag = None;
                                        let mut relocate_to_primary = None;
                                        let mut manage_cue_id = None;
                                        let mut jump_to_cue_id = None;
                                        let mut delete_cue_id = None;

                                        // 1st Pass: Draw all non-dragged clips
                                        let active_drag_id = self.app.active_drag.as_ref().map(|d| d.instance_id);
                                        let mut dragged_clip_data = None;

                                        for instance in &self.app.timeline.instances {
                                            if let Some(effect) = self.app.timeline.templates.iter().find(|t| t.id == instance.effect_id) {
                                                let duration_resizable = effect.duration_resizable();
                                                let Some(placement) = timeline_cue_placement(
                                                    self.app,
                                                    effect,
                                                    &timeline_rows,
                                                    &visible_analog_track_ids,
                                                ) else {
                                                    continue;
                                                };
                                                let track_index = placement.track_index;
                                                let relay_id = placement.relay_id;
                                                let selected_track_key = placement.selected_track_key;
                                                let analog_y = placement.analog_index.map(|analog_index| {
                                                    tracks_top
                                                        + track_area_height
                                                        + analog_index as f32 * timeline_analog_height
                                                });
                                                let is_mismatched = relay_id
                                                    .is_some_and(|relay_id| !effect.target.is_compatible_with_relay(relay_id));
                                                let track_y = analog_y.unwrap_or_else(|| timeline_track_row_top(
                                                    rect.min.y,
                                                    track_index,
                                                    timeline_track_height,
                                                ));
                                                let cue_row_height = if analog_y.is_some() {
                                                    timeline_analog_height
                                                } else {
                                                    timeline_track_height
                                                };

                                                let start_x = rect.min.x + (instance.start_time_ms as f32 * px_per_ms);
                                                let end_x = if effect.is_state_marker() { start_x + 88.0 }
                                                    else { start_x + (effect.duration_ms.max(1) as f32 * px_per_ms) };
                                                let minimum_clip_width = if duration_resizable { 8.0 } else { 18.0 };

                                                let clip_rect = egui::Rect::from_min_max(
                                                    egui::pos2(start_x, track_y + 4.0),
                                                    egui::pos2(
                                                        end_x.max(start_x + minimum_clip_width),
                                                        track_y + cue_row_height - 4.0,
                                                    ),
                                                );

                                                let is_active_drag = active_drag_id == Some(instance.id);
                                                if !is_active_drag && (clip_rect.max.x < viewport_clip.min.x - 20.0 || clip_rect.min.x > viewport_clip.max.x + 20.0) {
                                                    continue;
                                                }

                                                let clip_id = egui::Id::new(instance.id);
                                                let is_track_locked = relay_id
                                                    .is_some_and(|relay_id| self.app.track_locked.contains(&relay_id));

                                                let mut clip_response = if is_track_locked {
                                                    ui.interact(clip_rect, clip_id, egui::Sense::click())
                                                } else {
                                                    ui.interact(clip_rect, clip_id, egui::Sense::click_and_drag())
                                                };

                                                let duration = crate::duration::format_effect_duration_for_language(
                                                    self.app.language,
                                                    effect.duration_ms,
                                                );
                                                let mut cue_tooltip = format!(
                                                    "{}\n{}: {}",
                                                    effect.name,
                                                    self.app.tr("Duration"),
                                                    duration,
                                                );
                                                if effect.is_state_marker() {
                                                    cue_tooltip = format!("{}\nSet and keep — until next command\nDrag to move; no automatic exit", effect.name);
                                                } else if !duration_resizable {
                                                    cue_tooltip.push_str(&format!(
                                                        "\n{}",
                                                        self.app.tr("Intrinsic duration — drag to move"),
                                                    ));
                                                }
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
                                                    cue_tooltip.push_str("\n\n");
                                                    cue_tooltip.push_str(&warn_msg);
                                                }
                                                clip_response = clip_response.on_hover_text(cue_tooltip);

                                                clip_response.context_menu(|ui| {
                                                    if ui
                                                        .button(format!(
                                                            "{} {}",
                                                            crate::ui::icons::SLIDERS_HORIZONTAL,
                                                            self.app.tr("Reveal in Effect Controls")
                                                        ))
                                                        .clicked()
                                                    {
                                                        manage_cue_id = Some(instance.id);
                                                        ui.close();
                                                    }
                                                    if ui
                                                        .button(format!(
                                                            "{} {}",
                                                            crate::ui::icons::SKIP_BACK,
                                                            self.app.tr("Jump to cue start")
                                                        ))
                                                        .clicked()
                                                    {
                                                        jump_to_cue_id = Some(instance.id);
                                                        ui.close();
                                                    }
                                                    ui.separator();
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
                                                    if ui.button(egui::RichText::new(format!("{} {timeline_delete_cue_label}", crate::ui::icons::TRASH)).color(egui::Color32::from_rgb(231, 76, 60))).clicked() {
                                                        delete_cue_id = Some(instance.id);
                                                        ui.close();
                                                    }
                                                });

                                                let is_hovered = clip_response.hovered() && !is_track_locked;
                                                let mut hovered_handle = None;

                                                if is_hovered && self.app.active_drag.is_none() {
                                                    if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                                        let mode = if duration_resizable {
                                                            crate::app::classify_clip_drag_mode(clip_rect.left(), clip_rect.right(), mouse_pos.x)
                                                        } else {
                                                            crate::app::DragMode::Move
                                                        };
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

                                                if clip_response.double_clicked() {
                                                    clicked_any_clip = true;
                                                    self.app.selected_timeline_track = selected_track_key.clone().or_else(|| timeline_rows
                                                        .get(track_index)
                                                        .map(|row| row.key.clone()));
                                                    manage_cue_id = Some(instance.id);
                                                } else if clip_response.clicked() {
                                                    let is_ctrl = ui.ctx().input(|i| i.modifiers.command || i.modifiers.ctrl);
                                                    clicked_any_clip = true;
                                                    self.app.selected_timeline_track = selected_track_key.clone().or_else(|| timeline_rows
                                                        .get(track_index)
                                                        .map(|row| row.key.clone()));
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

                                                if clip_response.drag_started_by(egui::PointerButton::Primary)
                                                    && !is_track_locked
                                                {
                                                    clicked_any_clip = true;
                                                    self.app.selected_timeline_track = selected_track_key.clone().or_else(|| timeline_rows
                                                        .get(track_index)
                                                        .map(|row| row.key.clone()));
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

                                                        let drag_mode = if duration_resizable {
                                                            crate::app::classify_clip_drag_mode(clip_rect.left(), clip_rect.right(), press_x)
                                                        } else {
                                                            crate::app::DragMode::Move
                                                        };
                                                        started_drag = Some((instance.id, drag_mode, instance.start_time_ms, effect.duration_ms, press_x, initial_positions));
                                                    }
                                                }

                                                if active_drag_id == Some(instance.id) {
                                                    // Save for 2nd pass
                                                    dragged_clip_data = Some((clip_rect, instance.id, effect.clone(), is_mismatched));
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

                                                let is_muted = relay_id.is_some_and(|relay_id| self.app.track_muted.contains(&relay_id));
                                                let is_soloed = relay_id.is_some_and(|relay_id| self.app.track_soloed.contains(&relay_id));
                                                let is_locked = relay_id.is_some_and(|relay_id| self.app.track_locked.contains(&relay_id));
                                                let alpha = timeline_track_cue_alpha(
                                                    is_muted,
                                                    is_soloed,
                                                    is_locked,
                                                    relay_id.is_some() && relay_solo_active,
                                                );

                                                // Draw clip box
                                                paint_hardware_cue(&painter, clip_rect, effect, alpha, egui::Stroke::new(stroke_width, stroke_color));

                                                // Visual handle grips
                                                let left_active = hovered_handle == Some(crate::app::DragMode::ResizeLeft);
                                                let right_active = hovered_handle == Some(crate::app::DragMode::ResizeRight);
                                                if duration_resizable {
                                                    render_clip_handles(&painter, clip_rect, left_active, right_active, alpha);
                                                } else {
                                                    render_clip_move_grip(&painter, clip_rect, alpha);
                                                }

                                                // Clip name label
                                                let displayed_effect_name = crate::ui::i18n::visual_text(display_language, &effect.name);
                                                let title = if effect.is_state_marker() {
                                                    direct_cue_marker_caption(effect)
                                                } else if is_mismatched {
                                                    format!("{} {} {}", crate::ui::icons::WARNING, crate::ui::icons::SPARKLE, displayed_effect_name)
                                                } else {
                                                    format!("{} {}", crate::ui::icons::SPARKLE, displayed_effect_name)
                                                };
                                                render_timeline_cue_label(
                                                    &painter,
                                                    clip_rect,
                                                    title,
                                                    egui::FontId::proportional(10.0),
                                                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                                                    self.app.timeline_hide_cue_text_overflow,
                                                    duration_resizable,
                                                );
                                            }
                                        }

                                        // 2nd Pass: Draw the actively dragged clip on top with a shadow and brighter color
                                        if let Some((clip_rect, instance_id, effect, is_mismatched)) = dragged_clip_data {
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
                                            paint_hardware_cue(&painter, clip_rect, &effect, 255, egui::Stroke::new(stroke_width, stroke_color));

                                            // Visual handle grips
                                            let left_active = self.app.active_drag.as_ref().map(|d| d.mode) == Some(crate::app::DragMode::ResizeLeft);
                                            let right_active = self.app.active_drag.as_ref().map(|d| d.mode) == Some(crate::app::DragMode::ResizeRight);
                                            if effect.duration_resizable() {
                                                render_clip_handles(&painter, clip_rect, left_active, right_active, 255);
                                            } else {
                                                render_clip_move_grip(&painter, clip_rect, 255);
                                            }

                                            // Clip name label
                                            let displayed_effect_name = crate::ui::i18n::visual_text(display_language, &effect.name);
                                            let title = if effect.is_state_marker() {
                                                direct_cue_marker_caption(&effect)
                                            } else if is_mismatched {
                                                format!("{} {} {}", crate::ui::icons::WARNING, crate::ui::icons::SPARKLE, displayed_effect_name)
                                            } else {
                                                format!("{} {}", crate::ui::icons::SPARKLE, displayed_effect_name)
                                            };
                                            render_timeline_cue_label(
                                                &painter,
                                                clip_rect,
                                                title,
                                                egui::FontId::proportional(10.0),
                                                egui::Color32::WHITE,
                                                self.app.timeline_hide_cue_text_overflow,
                                                effect.duration_resizable(),
                                            );
                                        }

                                        // Apply cue actions outside the immutable timeline borrow loop.
                                        if let Some(cue_id) = manage_cue_id {
                                            self.app.selected_instance_ids.clear();
                                            self.app.selected_instance_ids.insert(cue_id);
                                            self.app.selected_keyframes.clear();
                                            self.app.selected_timeline_keyframe = None;
                                            self.app.open_or_focus_tab(PealayerTab::EffectControls);
                                            self.app.trigger_effect_controls_ping(ui.input(|i| i.time));
                                            ui.ctx().request_repaint();
                                        }
                                        if let Some(cue_id) = jump_to_cue_id {
                                            self.app.selected_instance_ids.clear();
                                            self.app.selected_instance_ids.insert(cue_id);
                                            if let Some(start_seconds) = self
                                                .app
                                                .timeline
                                                .instances
                                                .iter()
                                                .find(|instance| instance.id == cue_id)
                                                .map(|instance| instance.start_time_ms as f64 / 1_000.0)
                                            {
                                                if self.app.current_video_path.is_some()
                                                    && self.app.is_seekable
                                                {
                                                    self.app.seek_absolute(start_seconds);
                                                } else {
                                                    self.app.playback_time = start_seconds;
                                                    self.app.seek_pos = Some(start_seconds);
                                                    self.app.set_osd(format!(
                                                        "{}: {}",
                                                        self.app.tr("Cue start"),
                                                        crate::ui::controls::format_player_time(
                                                            start_seconds,
                                                            self.app.duration >= 3_600.0,
                                                            true,
                                                        )
                                                    ));
                                                }
                                            }
                                            ui.ctx().request_repaint();
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

                                            if ui.ctx().input(|i| {
                                                i.pointer.button_released(egui::PointerButton::Primary)
                                            }) {
                                                drag_ended = true;
                                            } else {
                                                let delta_x = ui.ctx().pointer_latest_pos().map(|p| p.x).unwrap_or(drag_state.drag_start_x) - drag_state.drag_start_x;
                                                let delta_time_ms = (delta_x / px_per_ms) as i64;

                                                let snap_enabled = !ui.ctx().input(|i| i.modifiers.shift || i.modifiers.alt);

                                                let snap_targets = timeline_snap_targets(
                                                    &self.app.timeline,
                                                    &self.app.selected_instance_ids,
                                                    (self.app.playback_time * 1000.0).round().max(0.0) as u64,
                                                );
                                                let snap_tolerance = timeline_snap_tolerance_ms(px_per_ms);

                                                let hud_text = match drag_state.mode {
                                                    crate::app::DragMode::Move => {
                                                        // Vertical track switching
                                                        let mut target_relay = None;
                                                        let moving_direct_pwm = self.app.timeline.instances.iter()
                                                            .find(|instance| instance.id == drag_state.instance_id)
                                                            .and_then(|instance| self.app.timeline.templates.iter().find(|template| template.id == instance.effect_id))
                                                            .and_then(|template| template.direct_control.as_ref())
                                                            .is_some_and(|cue| cue.control_key.starts_with("pwm."));
                                                        if !moving_direct_pwm && let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                                    let relative_y = mouse_pos.y - tracks_top;
                                                    let track_index = (relative_y
                                                                / timeline_track_height)
                                                                .floor()
                                                                as i32;
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
                                                            if let Some(target) = nearest_snap_time(
                                                                primary_new_start,
                                                                &snap_targets,
                                                                snap_tolerance,
                                                            ) {
                                                                primary_new_start = target;
                                                                snap_line_x = Some(rect.min.x + (target as f32 * px_per_ms));
                                                            }
                                                            // Check snap to end
                                                            let moving_state_marker = self.app.timeline.instances.iter().find(|cue| cue.id == drag_state.instance_id)
                                                                .and_then(|cue| self.app.timeline.templates.iter().find(|effect| effect.id == cue.effect_id))
                                                                .is_some_and(|effect| effect.is_state_marker());
                                                            if snap_line_x.is_none() && !moving_state_marker {
                                                                let primary_new_end = primary_new_start + drag_state.initial_duration_ms;
                                                                if let Some(target) = nearest_snap_time(
                                                                    primary_new_end,
                                                                    &snap_targets,
                                                                    snap_tolerance,
                                                                ) {
                                                                    primary_new_start = target.saturating_sub(drag_state.initial_duration_ms);
                                                                    snap_line_x = Some(rect.min.x + (target as f32 * px_per_ms));
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
                                                        let track_name = target_relay
                                                            .map(relay_identifier_label)
                                                            .unwrap_or_else(|| "Track".to_string());
                                                        format!("⏱ Start: {:.3}s | {}", start_secs, track_name)
                                                    }
                                                    crate::app::DragMode::ResizeRight => {
                                                        let mut new_end = (drag_state.initial_start_time_ms + drag_state.initial_duration_ms) as i64 + delta_time_ms;
                                                        if snap_enabled {
                                                            if let Some(target) = nearest_snap_time(
                                                                new_end.max(0) as u64,
                                                                &snap_targets,
                                                                snap_tolerance,
                                                            ) {
                                                                new_end = target as i64;
                                                                snap_line_x = Some(rect.min.x + (target as f32 * px_per_ms));
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
                                                            if let Some(target) = nearest_snap_time(
                                                                new_start,
                                                                &snap_targets,
                                                                snap_tolerance,
                                                            ) {
                                                                new_start = target;
                                                                snap_line_x = Some(rect.min.x + (target as f32 * px_per_ms));
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
                                            self.app.sync_timeline_engine();
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
                                            for track in self
                                                .app
                                                .timeline
                                                .analog_tracks
                                                .iter_mut()
                                                .filter(|track| linked_analog_track_ids.contains(&track.id))
                                            {
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
                                        let mut curve_updated = false;
                                        let mut curve_persist_requested = false;
                                        let pointer_pos = ui.ctx().pointer_latest_pos();

                                        let mut started_drag_info = None;
                                        let mut kf_interp_change = None;
                                        let mut kf_to_remove = None;
                                        let mut pending_add_keyframe = None;
                                        let mut pending_cue_dialog = None;

                                        for (t_idx, track) in self
                                            .app
                                            .timeline
                                            .analog_tracks
                                            .iter_mut()
                                            .filter(|track| visible_analog_track_ids.contains(&track.id))
                                            .enumerate()
                                        {
                                            let row_y = tracks_top
                                                + track_area_height
                                                + t_idx as f32 * timeline_analog_height;
                                            let curve_bottom = row_y + timeline_analog_height - 4.0;
                                            let curve_span = timeline_analog_height - 8.0;
                                            let row_rect = egui::Rect::from_min_max(
                                                egui::pos2(rect.min.x, row_y),
                                                egui::pos2(
                                                    rect.max.x,
                                                    row_y + timeline_analog_height,
                                                ),
                                            );

                                            // Keep lane selection visible across both the heading and
                                            // editable curve area, matching fixed/media tracks.
                                            let track_key =
                                                crate::four_d::models::hardware_timeline_track_key(
                                                    &format!("pwm.{}", track.channel),
                                                );
                                            let track_visual_opacity = timeline_track_visual_opacity(
                                                track.muted,
                                                track.soloed,
                                                track.locked,
                                                analog_solo_active,
                                            );
                                            let row_fill = if self
                                                .app
                                                .selected_timeline_track
                                                .as_deref()
                                                == Some(track_key.as_str())
                                            {
                                                ui.visuals().selection.bg_fill.gamma_multiply(0.12)
                                            } else {
                                                ui.visuals().extreme_bg_color
                                            };
                                            painter.rect_filled(row_rect, 0.0, row_fill);
                                            if track_visual_opacity < 1.0 {
                                                painter.rect_filled(
                                                    row_rect,
                                                    0.0,
                                                    egui::Color32::from_black_alpha(((1.0 - track_visual_opacity) * 92.0).round() as u8),
                                                );
                                            }
                                            let state_tint = if track.muted {
                                                Some(TimelineTrackStateKind::Muted)
                                            } else if track.soloed {
                                                Some(TimelineTrackStateKind::Soloed)
                                            } else if track.locked {
                                                Some(TimelineTrackStateKind::Locked)
                                            } else {
                                                None
                                            };
                                            if let Some(kind) = state_tint {
                                                painter.rect_filled(row_rect, 0.0, timeline_track_state_color(kind).gamma_multiply(0.07));
                                            }

                                            // Centerline guide (50% intensity)
                                            painter.line_segment(
                                                [egui::pos2(rect.min.x, row_y + timeline_analog_height * 0.5), egui::pos2(rect.max.x, row_y + timeline_analog_height * 0.5)],
                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(34, 42, 48)),
                                            );

                                            // Sample continuous curve along visible timeline
                                            let step_px = 6.0_f32;
                                            let mut points = Vec::new();
                                            let visible_left = (viewport_clip.min.x - step_px).max(rect.min.x);
                                            let visible_right = (viewport_clip.max.x + step_px).min(rect.max.x);
                                            let mut curr_x = visible_left;
                                            while curr_x <= visible_right {
                                                let t_ms = (((curr_x - rect.min.x) / zoom) * 1000.0).max(0.0) as u64;
                                                let norm_val = track.evaluate(t_ms);
                                                let py = curve_bottom - (norm_val * curve_span);
                                                points.push(egui::pos2(curr_x, py));
                                                curr_x += step_px;
                                            }

                                            // Draw translucent fill under curve (Curve Gradient Underlay)
                                            let curve_color = if track.muted {
                                                egui::Color32::from_rgb(118, 118, 118)
                                            } else if track.soloed {
                                                timeline_track_state_color(TimelineTrackStateKind::Soloed)
                                            } else if track.locked {
                                                timeline_track_state_color(TimelineTrackStateKind::Locked)
                                            } else {
                                                egui::Color32::from_rgb(0, 220, 255)
                                            };
                                            let curve_color = curve_color.gamma_multiply(track_visual_opacity);
                                            let fill_col = egui::Color32::from_rgba_unmultiplied(
                                                curve_color.r(),
                                                curve_color.g(),
                                                curve_color.b(),
                                                if track.muted { 16 } else { 25 },
                                            );
                                            for window in points.windows(2) {
                                                let p1 = window[0];
                                                let p2 = window[1];
                                                let b1 = egui::pos2(p1.x, curve_bottom);
                                                let b2 = egui::pos2(p2.x, curve_bottom);
                                                painter.add(egui::Shape::convex_polygon(
                                                    vec![b1, p1, p2, b2],
                                                    fill_col,
                                                    egui::Stroke::NONE,
                                                ));
                                            }

                                            // Draw curve line
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
                                                            let gy = curve_bottom - (s.1 * curve_span);
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
                                                if kx < viewport_clip.min.x - 20.0 || kx > viewport_clip.max.x + 20.0 {
                                                    continue;
                                                }
                                                let ky = curve_bottom - (kf.value * curve_span);
                                                let center = egui::pos2(kx, ky);
                                                let is_selected = self.app.selected_keyframes.contains(&(track.id, k_idx));

                                                // One nearest target owns the shared 16px hit area.
                                                let target = TimelineKeyframeTarget::Analog(track.id, k_idx);
                                                let is_hovered = nearest_keyframe == Some(target);

                                                if is_hovered
                                                    && !track.locked
                                                    && self.app.active_keyframe_drag.is_none()
                                                {
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
                                                if is_hovered && ui.input(|input| input.pointer.button_released(egui::PointerButton::Secondary)) {
                                                    if !self.app.selected_keyframes.contains(&(track.id, k_idx)) {
                                                        self.app.selected_keyframes.clear();
                                                        self.app.selected_keyframes.insert((track.id, k_idx));
                                                    }
                                                    self.app.selected_timeline_keyframe = None;
                                                }
                                                keyframe_context_menu(ui, &kf_response, target, is_hovered, |ui| {
                                                    ui.strong(format!("{} {}", crate::ui::icons::DIAMOND, keyframe_label));
                                                    ui.label(format!("{time_label}: {} · {value_label}: {:.1}%",
                                                        format_timecode(kf.time_ms as f64 / 1000.0), kf.value * 100.0));
                                                    ui.separator();
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
                                                    if ui.add_enabled(!track.locked, egui::Button::new(format!("{} {}", crate::ui::icons::TRASH, delete_keyframe_label))).clicked() {
                                                        kf_to_remove = Some((track.id, k_idx));
                                                        ui.close();
                                                    }
                                                    ui.separator();
                                                    if ui.button(format!("{} {}", crate::ui::icons::X, deselect_keyframe_label)).clicked() {
                                                        self.app.selected_keyframes.clear();
                                                        ui.ctx().request_repaint();
                                                        ui.close();
                                                    }
                                                });

                                                if is_hovered && ui.input(|i| i.pointer.secondary_clicked()) {
                                                    clicked_any_keyframe = true;
                                                }

                                                // Primary click: Selection & Active Drag Lock initialization on mouse press/down
                                                if is_hovered
                                                    && !track.locked
                                                    && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary))
                                                    && self.app.active_keyframe_drag.is_none()
                                                {
                                                    if let Some(pos) = pointer_pos {
                                                        self.app.selected_timeline_track = Some(
                                                            crate::four_d::models::hardware_timeline_track_key(
                                                                &format!("pwm.{}", track.channel),
                                                            ),
                                                        );
                                                        started_drag_info = Some((track.id, k_idx, pos, kf.time_ms, kf.value));
                                                        clicked_any_keyframe = true;
                                                    }
                                                }
                                            }

                                            if response.double_clicked()
                                                && !track.locked
                                                && !clicked_any_keyframe
                                            {
                                                if let Some(pos) = response.interact_pointer_pos() {
                                                    if row_rect.contains(pos) {
                                                        let track_key = crate::four_d::models::hardware_timeline_track_key(
                                                            &format!("pwm.{}", track.channel),
                                                        );
                                                        self.app.selected_timeline_track = Some(track_key.clone());
                                                        let new_t = (((pos.x - rect.min.x) / zoom) * 1000.0).max(0.0) as u64;
                                                        if ui.input(|input| input.modifiers.alt) {
                                                            let new_v = ((curve_bottom - pos.y) / curve_span).clamp(0.0, 1.0);
                                                            pending_add_keyframe = Some((track.id, new_t, new_v));
                                                        } else {
                                                            pending_cue_dialog = Some((track_key, new_t));
                                                        }
                                                        clicked_any_keyframe = true;
                                                    }
                                                }
                                            }
                                        }

                                        if let Some((track_key, start_time_ms)) = pending_cue_dialog {
                                            request_timeline_cue_dialog(
                                                self.app,
                                                ui.ctx(),
                                                &track_key,
                                                start_time_ms,
                                            );
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
                                            curve_persist_requested = true;
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
                                            curve_persist_requested = true;
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
                                            curve_persist_requested = true;
                                        }

                                        if let Some((track_id, k_idx, pos, orig_t, orig_v)) = started_drag_info {
                                            if !self.app.selected_keyframes.contains(&(track_id, k_idx)) {
                                                if !ui.input(|i| {
                                                    i.modifiers.ctrl || i.modifiers.command
                                                }) {
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

                                            let drag_ended = ui.ctx().input(|i| {
                                                i.pointer.button_released(egui::PointerButton::Primary)
                                            });

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
                                                curve_persist_requested = true;
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
                                                let val_delta =
                                                    -delta_y / (timeline_analog_height - 8.0);
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

                                        if curve_persist_requested {
                                            self.app.commit_timeline_edit();
                                        } else if curve_updated {
                                            let _ = self.app.engine_handle.sender.send(
                                                crate::four_d::engine::EngineMessage::UpdateAnalogTracks(
                                                    self.app.linked_analog_tracks(),
                                                ),
                                            );
                                        }

                                        // Render Dedicated Time Ruler Bar Header
                                        ruler_painter.rect_filled(ruler_rect, 0.0, ui.visuals().panel_fill);
                                        ruler_painter.line_segment(
                                            [egui::pos2(ruler_rect.min.x, ruler_rect.max.y), egui::pos2(ruler_rect.max.x, ruler_rect.max.y)],
                                            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(50, 50, 50)),
                                        );

                                        let label_step: i32 = if zoom < 30.0 {
                                            5
                                        } else if zoom < 60.0 {
                                            2
                                        } else {
                                            1
                                        };
                                        // Clock labels grow after minute/hour boundaries. Keep
                                        // them apart rather than retaining seconds-only widths.
                                        let longest_label = ruler_painter.layout_no_wrap(
                                            crate::duration::format_timeline_time_ms(total_seconds.ceil().max(0.0) as u64 * 1_000, false),
                                            egui::FontId::monospace(9.0),
                                            egui::Color32::from_rgb(140, 140, 140),
                                        );
                                        let label_step = label_step.max(((longest_label.size().x + 10.0) / zoom).ceil().max(1.0) as i32);

                                        for i in visible_start_i..=visible_end_i {
                                            let grid_x = rect.min.x + (i as f32 * zoom);
                                            if grid_x >= viewport_clip.min.x - 100.0 && grid_x <= rect.max.x - 8.0 {
                                                // Major second tick
                                                ruler_painter.line_segment(
                                                    [egui::pos2(grid_x, ruler_rect.max.y - 8.0), egui::pos2(grid_x, ruler_rect.max.y)],
                                                    egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(100, 100, 100)),
                                                );
                                                // Sub-second frame notches
                                                if zoom >= 50.0 {
                                                    for sub in 1..10 {
                                                        let sub_x = grid_x + (sub as f32 * (zoom / 10.0));
                                                        if sub_x >= viewport_clip.min.x - 20.0 && sub_x <= rect.max.x - 8.0 {
                                                            let notch_h = if sub == 5 { 5.0 } else { 3.0 };
                                                            ruler_painter.line_segment(
                                                                [egui::pos2(sub_x, ruler_rect.max.y - notch_h), egui::pos2(sub_x, ruler_rect.max.y)],
                                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(60, 60, 60)),
                                                            );
                                                        }
                                                    }
                                                }
                                                // Time label inside ruler
                                                if i % label_step == 0 {
                                                    let label = ruler_painter.layout_no_wrap(
                                                        crate::duration::format_timeline_time_ms(i as u64 * 1_000, false),
                                                        egui::FontId::monospace(9.0),
                                                        egui::Color32::from_rgb(140, 140, 140),
                                                    );
                                                    if grid_x + 4.0 + label.size().x <= rect.max.x - 8.0 {
                                                        ruler_painter.galley(
                                                            egui::pos2(grid_x + 4.0, ruler_rect.min.y + 13.0 - label.size().y / 2.0),
                                                            label,
                                                            egui::Color32::from_rgb(140, 140, 140),
                                                        );
                                                    }
                                                }
                                            }
                                        }

                                        // Media chapters are file-owned navigation landmarks,
                                        // distinct from editable project keyframes. Render them
                                        // as subtle amber flags spanning the timeline so chapter
                                        // boundaries remain useful without looking editable.
                                        for chapter in &media_chapters {
                                            let marker_x =
                                                rect.min.x + chapter.time_seconds as f32 * zoom;
                                            if marker_x < rect.min.x || marker_x > rect.max.x || marker_x < viewport_clip.min.x - 20.0 || marker_x > viewport_clip.max.x + 20.0 {
                                                continue;
                                            }
                                            let active =
                                                active_chapter_index == Some(chapter.index);
                                            let color = if active {
                                                egui::Color32::from_rgb(255, 193, 75)
                                            } else {
                                                egui::Color32::from_rgb(201, 151, 59)
                                            };
                                            painter.line_segment(
                                                [
                                                    egui::pos2(marker_x, ruler_rect.min.y + 4.0),
                                                    egui::pos2(marker_x, rect.max.y),
                                                ],
                                                egui::Stroke::new(
                                                    if active { 1.5 } else { 1.0 },
                                                    color.gamma_multiply(if active { 0.8 } else { 0.45 }),
                                                ),
                                            );
                                            ruler_painter.add(egui::Shape::convex_polygon(
                                                vec![
                                                    egui::pos2(marker_x, ruler_rect.min.y + 3.0),
                                                    egui::pos2(marker_x + 9.0, ruler_rect.min.y + 6.5),
                                                    egui::pos2(marker_x, ruler_rect.min.y + 10.0),
                                                ],
                                                color,
                                                egui::Stroke::NONE,
                                            ));
                                        }

                                        // Exact timeline keyframes must be painted after the
                                        // opaque ruler and lane backgrounds. The previous order
                                        // erased the diamond immediately after insertion, making
                                        // a successful model mutation look completely broken.
                                        for marker in &self.app.timeline.keyframes {
                                            let marker_x =
                                                rect.min.x + marker.time_ms as f32 * px_per_ms;
                                            if marker_x < rect.min.x || marker_x > rect.max.x || marker_x < viewport_clip.min.x - 20.0 || marker_x > viewport_clip.max.x + 20.0 {
                                                continue;
                                            }
                                            let selected = self.app.selected_timeline_keyframe
                                                == Some(marker.id);
                                            let color = if selected {
                                                ui.visuals().selection.stroke.color
                                            } else {
                                                ui.visuals().hyperlink_color
                                            };
                                            painter.line_segment(
                                                [
                                                    egui::pos2(marker_x, ruler_rect.min.y + 11.0),
                                                    egui::pos2(marker_x, rect.max.y),
                                                ],
                                                egui::Stroke::new(
                                                    if selected { 1.6 } else { 1.0 },
                                                    color.gamma_multiply(0.7),
                                                ),
                                            );
                                            let center = timeline_keyframe_marker_center(
                                                ruler_rect,
                                                marker_x,
                                            );
                                            let diamond = vec![
                                                egui::pos2(center.x, center.y - 5.0),
                                                egui::pos2(center.x + 5.0, center.y),
                                                egui::pos2(center.x, center.y + 5.0),
                                                egui::pos2(center.x - 5.0, center.y),
                                            ];
                                            ruler_painter.add(egui::Shape::convex_polygon(
                                                diamond,
                                                color,
                                                egui::Stroke::new(
                                                    1.0,
                                                    ui.visuals().panel_fill,
                                                ),
                                            ));
                                        }

                                        // Draw Playhead
                                        let current_playhead_time = self.app.seek_pos.unwrap_or(self.app.playback_time);
                                        let playhead_x = rect.min.x + (current_playhead_time as f32 * zoom);
                                        if playhead_x <= rect.max.x {
                                            // Keep the playhead handle wholly inside the ruler.
                                            // The timeline line begins at the track boundary so it
                                            // cannot make the first media lane look like the ruler.
                                            painter.line_segment(
                                                [egui::pos2(playhead_x, tracks_top), egui::pos2(playhead_x, rect.max.y)],
                                                egui::Stroke::new(1.5_f32, egui::Color32::RED),
                                            );
                                            // Downward handle terminating exactly at the bottom of
                                            // the ruler, immediately above the first track.
                                            let points = vec![
                                                egui::pos2(playhead_x - 7.0, ruler_rect.bottom() - 12.0),
                                                egui::pos2(playhead_x + 7.0, ruler_rect.bottom() - 12.0),
                                                egui::pos2(playhead_x, ruler_rect.bottom()),
                                            ];
                                            ruler_painter.add(egui::Shape::convex_polygon(points, egui::Color32::RED, egui::Stroke::NONE));
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
                                                    let hovered_track_index = (relative_y
                                                        / timeline_track_height)
                                                        .floor()
                                                        as i32;

                                                    for (i, track_row) in timeline_rows.iter().enumerate() {
                                                        if payload.controller_macro.is_some()
                                                            || payload.controller_strip_effect.is_some()
                                                        {
                                                            if track_row.kind
                                                                == TimelineTrackKind::ControllerEffect(
                                                                    payload.controller_lane.unwrap_or(
                                                                        crate::four_d::models::ControllerEffectLane::Sequence,
                                                                    ),
                                                                )
                                                            {
                                                                let row_y = timeline_track_row_top(
                                                                    rect.min.y,
                                                                    i,
                                                                    timeline_track_height,
                                                                );
                                                                let track_rect = egui::Rect::from_min_max(
                                                                    egui::pos2(rect.min.x, row_y),
                                                                    egui::pos2(
                                                                        rect.max.x,
                                                                        row_y + timeline_track_height,
                                                                    ),
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
                                                        let row_index = i;
                                                        let i = i as i32;
                                                        let is_compatible = payload.target.is_compatible_with_relay(relay_id);
                                                        let is_locked = self.app.track_locked.contains(&relay_id);
                                                        let is_primary = payload.target.primary_relay_id() == Some(relay_id);
                                                        let row_y = timeline_track_row_top(
                                                            rect.min.y,
                                                            row_index,
                                                            timeline_track_height,
                                                        );
                                                        let track_rect = egui::Rect::from_min_max(
                                                            egui::pos2(rect.min.x, row_y),
                                                            egui::pos2(
                                                                rect.max.x,
                                                                row_y + timeline_track_height,
                                                            ),
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
                                                                row.kind
                                                                    != TimelineTrackKind::ControllerEffect(
                                                                        payload.controller_lane.unwrap_or(
                                                                            crate::four_d::models::ControllerEffectLane::Sequence,
                                                                        ),
                                                                    )
                                                            } else {
                                                                !matches!(row.kind, TimelineTrackKind::Relay(_))
                                                            }
                                                        })
                                                    {
                                                        ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
                                                        let row_y = tracks_top
                                                            + hovered_track_index as f32
                                                                * timeline_track_height;
                                                        let track_rect = egui::Rect::from_min_max(
                                                            egui::pos2(rect.min.x, row_y),
                                                            egui::pos2(
                                                                rect.max.x,
                                                                row_y + timeline_track_height,
                                                            ),
                                                        );
                                                        painter.rect_filled(track_rect, 0.0, egui::Color32::from_rgba_unmultiplied(255, 70, 70, 45));
                                                        painter.rect_stroke(track_rect, 0.0, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(255, 70, 70)), egui::StrokeKind::Inside);
                                                    }
                                                }
                                            }
                                        }

                                        // Lasso selection drawing
                                        if let (Some(origin), Some(lasso_rect)) =
                                            (self.app.lasso_origin, self.app.lasso_rect)
                                        {
                                            let current = ui.ctx().pointer_latest_pos().unwrap_or(lasso_rect.max);
                                            let mode = timeline_marquee_mode(origin, current);
                                            let (fill, stroke, label) = match mode {
                                                TimelineMarqueeMode::Window => (
                                                    egui::Color32::from_rgba_unmultiplied(52, 152, 219, 34),
                                                    egui::Color32::from_rgb(52, 152, 219),
                                                    self.app.tr("Window · fully enclosed"),
                                                ),
                                                TimelineMarqueeMode::Crossing => (
                                                    egui::Color32::from_rgba_unmultiplied(46, 204, 113, 34),
                                                    egui::Color32::from_rgb(46, 204, 113),
                                                    self.app.tr("Crossing · touched"),
                                                ),
                                            };
                                            painter.rect(
                                                lasso_rect,
                                                2.0,
                                                fill,
                                                egui::Stroke::new(1.5_f32, stroke),
                                                egui::StrokeKind::Inside,
                                            );
                                            painter.text(
                                                lasso_rect.left_top() + egui::vec2(4.0, -4.0),
                                                egui::Align2::LEFT_BOTTOM,
                                                label,
                                                egui::FontId::proportional(11.0),
                                                stroke,
                                            );
                                        }

                                        ((rect, response), clicked_any_clip, clicked_any_keyframe)
                                });

                            let timeline_scroll_id = timeline_scroll.id;
                            let mut timeline_scroll_state = timeline_scroll.state;
                            let timeline_content_size = timeline_scroll.content_size;
                            let timeline_viewport = timeline_scroll.inner_rect;
                            let ((rect, response), mut clicked_any_clip, clicked_any_keyframe) = timeline_scroll.inner;

                            let mut timeline_scroll_changed = false;
                            let navigation_transition_id =
                                timeline_scroll_id.with("navigation-transition");
                            let mut active_navigation_transition = None;
                            if self.app.timeline_animated_navigation {
                                if let Some(transition) = ui.data(|data| {
                                    data.get_temp::<TimelineNavigationTransition>(
                                        navigation_transition_id,
                                    )
                                    }) {
                                    let now_seconds = ui.input(|input| input.time);
                                    let (offset, transition_zoom, complete) =
                                        sample_timeline_navigation_transition(
                                            transition,
                                            now_seconds,
                                        );
                                    // The content was laid out with the previous frame's zoom.
                                    // Clamp against the interpolated target geometry so zoom and
                                    // its cursor anchor travel together without a one-frame snap.
                                    let max_offset_x =
                                        (total_seconds as f32 * transition_zoom
                                            - timeline_viewport.width())
                                            .max(0.0);
                                    let max_offset_y = (timeline_content_size.y
                                        - timeline_viewport.height())
                                        .max(0.0);
                                    timeline_scroll_state.offset = egui::vec2(
                                        offset.x.clamp(0.0, max_offset_x),
                                        offset.y.clamp(0.0, max_offset_y),
                                    );
                                    self.app.timeline_zoom = transition_zoom;
                                    timeline_scroll_changed = true;
                                    if complete {
                                        ui.data_mut(|data| {
                                            data.remove_temp::<TimelineNavigationTransition>(
                                                navigation_transition_id,
                                            );
                                        });
                                    } else {
                                        active_navigation_transition = Some(transition);
                                        ui.ctx().request_repaint();
                                    }
                                }
                            } else {
                                ui.data_mut(|data| {
                                    data.remove_temp::<TimelineNavigationTransition>(
                                        navigation_transition_id,
                                    );
                                });
                            }

                            // Timeline-local shortcuts mirror the visible toolbar. They only
                            // fire while the canvas owns keyboard focus, so text fields and the
                            // rest of the application retain their normal keys.
                            let timeline_layer = ui.layer_id();
                            let timeline_popup_open = egui::Popup::is_any_open(ui.ctx());
                            if self.app.keyboard_shortcuts_enabled && ui.ctx().memory(|memory| {
                                memory.has_focus(timeline_keyboard_focus_id())
                                    && memory.is_above_modal_layer(timeline_layer)
                                    && !timeline_popup_open
                            }) {
                                let keyboard_pan = ui.input(|input| {
                                    if input.modifiers.shift && !input.modifiers.ctrl
                                        && !input.modifiers.command && !input.modifiers.alt
                                    {
                                        u8::from(input.key_down(egui::Key::ArrowRight)) as f32
                                            - u8::from(input.key_down(egui::Key::ArrowLeft)) as f32
                                    } else { 0.0 }
                                });
                                if keyboard_pan != 0.0 { held_timeline_pan = keyboard_pan; }
                                let shortcut_action = ui.input(|input| {
                                    if input.modifiers.is_none()
                                        && (input.key_pressed(egui::Key::Plus)
                                            || input.key_pressed(egui::Key::Equals))
                                    {
                                        Some(TimelineToolbarAction::ZoomIn)
                                    } else if input.modifiers.is_none()
                                        && input.key_pressed(egui::Key::Minus)
                                    {
                                        Some(TimelineToolbarAction::ZoomOut)
                                    } else if input.modifiers.is_none()
                                        && input.key_pressed(egui::Key::C)
                                    {
                                        Some(TimelineToolbarAction::BringPlayheadIntoView)
                                    } else if (input.modifiers.ctrl || input.modifiers.command)
                                        && input.modifiers.shift
                                        && !input.modifiers.alt
                                        && input.key_pressed(egui::Key::L)
                                    {
                                        Some(TimelineToolbarAction::FollowPlayhead)
                                    } else {
                                        None
                                    }
                                });
                                if shortcut_action.is_some() {
                                    pending_timeline_toolbar_action = shortcut_action;
                                }
                            }

                            if let Some(action) = pending_timeline_toolbar_action.take() {
                                let current_offset = timeline_scroll_state.offset;
                                let current_zoom = self.app.timeline_zoom;
                                let mut navigation_target = None;
                                match action {
                                    TimelineToolbarAction::ZoomIn
                                    | TimelineToolbarAction::ZoomOut => {
                                        let factor = if action == TimelineToolbarAction::ZoomIn {
                                            1.25
                                        } else {
                                            0.8
                                        };
                                        let target_zoom = (current_zoom * factor).clamp(20.0, 500.0);
                                        let center = timeline_viewport.width() * 0.5;
                                        let target_x = timeline_offset_for_pointer_zoom(
                                            current_offset.x,
                                            center,
                                            current_zoom,
                                            target_zoom,
                                            total_seconds,
                                            timeline_viewport.width(),
                                        );
                                        navigation_target = Some((
                                            egui::vec2(target_x, current_offset.y),
                                            target_zoom,
                                        ));
                                    }
                                    TimelineToolbarAction::PanLeft
                                    | TimelineToolbarAction::PanRight => {
                                        let direction = if action == TimelineToolbarAction::PanLeft {
                                            -1.0
                                        } else {
                                            1.0
                                        };
                                        let max_x = (timeline_content_size.x
                                            - timeline_viewport.width())
                                            .max(0.0);
                                        let target_x = (current_offset.x
                                            + direction
                                                * (timeline_viewport.width() * 0.15).max(40.0))
                                            .clamp(0.0, max_x);
                                        navigation_target = Some((
                                            egui::vec2(target_x, current_offset.y),
                                            current_zoom,
                                        ));
                                    }
                                    TimelineToolbarAction::BringPlayheadIntoView => {
                                        let playhead_x =
                                            (self.app.playback_time.max(0.0) as f32 * current_zoom)
                                                .min(timeline_content_size.x);
                                        let target_x = timeline_offset_to_reveal_x(
                                            current_offset.x,
                                            playhead_x,
                                            timeline_content_size.x,
                                            timeline_viewport.width(),
                                        );
                                        navigation_target = Some((
                                            egui::vec2(target_x, current_offset.y),
                                            current_zoom,
                                        ));
                                    }
                                    TimelineToolbarAction::FollowPlayhead => {
                                        self.app.timeline_follow_playhead =
                                            !self.app.timeline_follow_playhead;
                                        self.app.request_timeline_toolbar_save(ui.ctx());
                                        if self.app.timeline_follow_playhead {
                                            let playhead_x = (self.app.playback_time.max(0.0) as f32
                                                * current_zoom)
                                                .min(timeline_content_size.x);
                                            let target_x = timeline_offset_to_reveal_x(
                                                current_offset.x,
                                                playhead_x,
                                                timeline_content_size.x,
                                                timeline_viewport.width(),
                                            );
                                            navigation_target = Some((
                                                egui::vec2(target_x, current_offset.y),
                                                current_zoom,
                                            ));
                                        }
                                    }
                                    TimelineToolbarAction::AddKeyframe => {
                                        let time_ms = (self.app.playback_time * 1_000.0)
                                            .round()
                                            .clamp(0.0, total_seconds * 1_000.0)
                                            as u64;
                                        self.app.insert_timeline_keyframe(time_ms);
                                    }
                                    TimelineToolbarAction::PreviousCue
                                    | TimelineToolbarAction::NextCue => {
                                        let cue_ids = sorted_cue_ids(&self.app.timeline);
                                        if !cue_ids.is_empty() {
                                            let selected_index = cue_ids.iter().position(|id| {
                                                self.app.selected_instance_ids.contains(id)
                                            });
                                            let next_index = if action
                                                == TimelineToolbarAction::PreviousCue
                                            {
                                                selected_index
                                                    .unwrap_or(0)
                                                    .checked_sub(1)
                                                    .unwrap_or(cue_ids.len() - 1)
                                            } else {
                                                selected_index
                                                    .map(|index| (index + 1) % cue_ids.len())
                                                    .unwrap_or(0)
                                            };
                                            let cue_id = cue_ids[next_index];
                                            self.app.selected_instance_ids.clear();
                                            self.app.selected_instance_ids.insert(cue_id);
                                            self.app.selected_keyframes.clear();
                                            self.app.selected_timeline_keyframe = None;
                                            if let Some(instance) = self
                                                .app
                                                .timeline
                                                .instances
                                                .iter()
                                                .find(|instance| instance.id == cue_id)
                                            {
                                                let target_x = (instance.start_time_ms as f32
                                                    * px_per_ms
                                                    - timeline_viewport.width() * 0.35)
                                                    .clamp(
                                                        0.0,
                                                        (timeline_content_size.x
                                                            - timeline_viewport.width())
                                                            .max(0.0),
                                                    );
                                                navigation_target = Some((
                                                    egui::vec2(target_x, current_offset.y),
                                                    current_zoom,
                                                ));
                                            }
                                        }
                                    }
                                    TimelineToolbarAction::NudgeCueLeft
                                    | TimelineToolbarAction::NudgeCueRight => {
                                        let direction = if action
                                            == TimelineToolbarAction::NudgeCueLeft
                                        {
                                            -1
                                        } else {
                                            1
                                        };
                                        let delta = timeline_frame_step_ms(
                                            self.app.media_fps,
                                            self.app.frame_step_count,
                                        ) as i64
                                            * direction;
                                        let snapshot = self.app.snapshot_timeline();
                                        if move_selected_cues(
                                            &mut self.app.timeline,
                                            &self.app.selected_instance_ids,
                                            delta,
                                        ) {
                                            self.app.undo_stack.push(snapshot);
                                            self.app.sync_timeline_engine();
                                        }
                                    }
                                    TimelineToolbarAction::SelectAll => {
                                        self.app.selected_instance_ids = self
                                            .app
                                            .timeline
                                            .instances
                                            .iter()
                                            .map(|instance| instance.id)
                                            .collect();
                                        self.app.selected_keyframes.clear();
                                        for track in self.app.timeline.analog_tracks.iter().filter(
                                            |track| visible_analog_track_ids.contains(&track.id),
                                        ) {
                                            for (index, _) in track.keyframes.iter().enumerate() {
                                                self.app.selected_keyframes.insert((track.id, index));
                                            }
                                        }
                                    }
                                    TimelineToolbarAction::ClearSelection => {
                                        self.app.selected_instance_ids.clear();
                                        self.app.selected_keyframes.clear();
                                        self.app.selected_timeline_keyframe = None;
                                    }
                                    TimelineToolbarAction::DeleteSelection => {
                                        if !self.app.selected_instance_ids.is_empty()
                                            && self.app.selected_keyframes.is_empty()
                                            && self.app.selected_timeline_keyframe.is_none()
                                        {
                                            self.app.delete_selected_timeline_cues();
                                        } else if !self.app.selected_instance_ids.is_empty()
                                            || !self.app.selected_keyframes.is_empty()
                                            || self.app.selected_timeline_keyframe.is_some()
                                        {
                                            self.app.undo_stack.push(self.app.snapshot_timeline());
                                            self.app.timeline.instances.retain(|instance| {
                                                !self.app.selected_instance_ids.contains(&instance.id)
                                            });
                                            for track in &mut self.app.timeline.analog_tracks {
                                                let mut indices = self
                                                    .app
                                                    .selected_keyframes
                                                    .iter()
                                                    .filter(|(track_id, _)| *track_id == track.id)
                                                    .map(|(_, index)| *index)
                                                    .collect::<Vec<_>>();
                                                indices.sort_unstable();
                                                for index in indices.into_iter().rev() {
                                                    if index < track.keyframes.len() {
                                                        track.keyframes.remove(index);
                                                    }
                                                }
                                            }
                                            if let Some(id) =
                                                self.app.selected_timeline_keyframe.take()
                                            {
                                                self.app.timeline.remove_keyframe(id);
                                            }
                                            self.app.selected_instance_ids.clear();
                                            self.app.selected_keyframes.clear();
                                            self.app.commit_timeline_edit();
                                        }
                                    }
                                }

                                if let Some((target_offset, target_zoom)) = navigation_target {
                                    if self.app.timeline_animated_navigation
                                        && ((target_offset - current_offset).length() > 0.5
                                            || (target_zoom - current_zoom).abs() > 0.01)
                                    {
                                        start_timeline_navigation_transition(
                                            ui, navigation_transition_id,
                                            (current_offset, current_zoom),
                                            (target_offset, target_zoom),
                                            self.app.timeline_navigation_transition_ms,
                                        );
                                    } else {
                                        self.app.timeline_zoom = target_zoom;
                                        timeline_scroll_state.offset = target_offset;
                                        timeline_scroll_changed = true;
                                    }
                                }
                            }
                            if let Some((wheel_action, pointer_screen_x)) =
                                pending_timeline_wheel
                            {
                                if self.app.timeline_animated_navigation {
                                    // Retarget an in-flight transition instead of restarting from
                                    // its old destination. Repeated wheel notches therefore remain
                                    // responsive while the rendered position eases toward the
                                    // accumulated target.
                                    let current_offset = timeline_scroll_state.offset;
                                    let current_zoom = self.app.timeline_zoom;
                                    let mut target_offset = active_navigation_transition
                                        .map_or(current_offset, |transition| {
                                            transition.target_offset
                                        });
                                    let mut target_zoom = active_navigation_transition
                                        .map_or(current_zoom, |transition| transition.target_zoom);
                                    let mut zoom_anchor = None;
                                    match wheel_action {
                                        TimelineWheelAction::Zoom(delta) => {
                                            let next_zoom =
                                                timeline_zoom_from_wheel(target_zoom, delta);
                                            let is_inside_viewport = pointer_screen_x >= timeline_viewport.left()
                                                && pointer_screen_x <= timeline_viewport.right();
                                            let pointer_x_in_viewport = if is_inside_viewport {
                                                pointer_screen_x - timeline_viewport.left()
                                            } else {
                                                let playhead_x = self.app.playback_time as f32 * current_zoom - current_offset.x;
                                                if playhead_x >= 0.0 && playhead_x <= timeline_viewport.width() {
                                                    playhead_x
                                                } else {
                                                    timeline_viewport.width() * 0.5
                                                }
                                            };

                                            let anchor_time = active_navigation_transition
                                                .and_then(|t| t.anchor)
                                                .filter(|a| (a.pointer_x_in_viewport - pointer_x_in_viewport).abs() < 1.0)
                                                .map(|a| a.media_time_seconds)
                                                .unwrap_or_else(|| {
                                                    let clamped_x = pointer_x_in_viewport.clamp(0.0, timeline_viewport.width().max(0.0));
                                                    ((current_offset.x + clamped_x) / current_zoom.max(f32::EPSILON)) as f64
                                                });

                                            let next_content_width = (total_seconds.max(0.0) as f32 * next_zoom).max(0.0);
                                            let max_offset = (next_content_width - timeline_viewport.width()).max(0.0);
                                            target_offset.x = (anchor_time as f32 * next_zoom - pointer_x_in_viewport).clamp(0.0, max_offset);
                                            target_zoom = next_zoom;
                                            zoom_anchor = Some(TimelineZoomAnchor {
                                                pointer_x_in_viewport,
                                                media_time_seconds: anchor_time,
                                                duration_seconds: total_seconds,
                                                viewport_width: timeline_viewport.width(),
                                            });
                                        }
                                        TimelineWheelAction::HorizontalScroll(delta) => {
                                            let max_offset = (total_seconds as f32 * target_zoom
                                                - timeline_viewport.width())
                                                .max(0.0);
                                            target_offset.x =
                                                (target_offset.x - delta).clamp(0.0, max_offset);
                                        }
                                        TimelineWheelAction::VerticalScroll(delta) => {
                                            let max_offset = (timeline_content_size.y
                                                - timeline_viewport.height())
                                                .max(0.0);
                                            target_offset.y =
                                                (target_offset.y - delta).clamp(0.0, max_offset);
                                        }
                                        TimelineWheelAction::MultiGesture { zoom_factor, translation } => {
                                            let next_zoom = (target_zoom * zoom_factor).clamp(20.0, 500.0);
                                            let pointer_x = pointer_screen_x - timeline_viewport.left();
                                            target_offset.x = timeline_offset_for_pointer_zoom(
                                                target_offset.x, pointer_x, target_zoom, next_zoom,
                                                total_seconds, timeline_viewport.width());
                                            target_zoom = next_zoom;
                                            let max_x = (total_seconds as f32 * target_zoom
                                                - timeline_viewport.width()).max(0.0);
                                            let max_y = (timeline_content_size.y
                                                - timeline_viewport.height()).max(0.0);
                                            target_offset.x = (target_offset.x - translation.x).clamp(0.0, max_x);
                                            target_offset.y = (target_offset.y - translation.y).clamp(0.0, max_y);
                                        }
                                    }
                                    start_anchored_timeline_navigation_transition(
                                        ui, navigation_transition_id,
                                        (current_offset, current_zoom),
                                        (target_offset, target_zoom),
                                        self.app.timeline_navigation_transition_ms,
                                        zoom_anchor,
                                    );
                                } else {
                                    ui.data_mut(|data| {
                                        data.remove_temp::<TimelineNavigationTransition>(
                                            navigation_transition_id,
                                        );
                                    });
                                    match wheel_action {
                                        TimelineWheelAction::Zoom(delta) => {
                                            let next_zoom =
                                                timeline_zoom_from_wheel(zoom, delta);
                                            let is_inside_viewport = pointer_screen_x >= timeline_viewport.left()
                                                && pointer_screen_x <= timeline_viewport.right();
                                            let pointer_x_in_viewport = if is_inside_viewport {
                                                pointer_screen_x - timeline_viewport.left()
                                            } else {
                                                let playhead_x = self.app.playback_time as f32 * zoom - timeline_scroll_state.offset.x;
                                                if playhead_x >= 0.0 && playhead_x <= timeline_viewport.width() {
                                                    playhead_x
                                                } else {
                                                    timeline_viewport.width() * 0.5
                                                }
                                            };
                                            timeline_scroll_state.offset.x =
                                                timeline_offset_for_pointer_zoom(
                                                    timeline_scroll_state.offset.x,
                                                    pointer_x_in_viewport,
                                                    zoom,
                                                    next_zoom,
                                                    total_seconds,
                                                    timeline_viewport.width(),
                                                );
                                            self.app.timeline_zoom = next_zoom;
                                            timeline_scroll_changed = true;
                                        }
                                        TimelineWheelAction::HorizontalScroll(delta) => {
                                            let max_offset = (timeline_content_size.x
                                                - timeline_viewport.width())
                                                .max(0.0);
                                            timeline_scroll_state.offset.x =
                                                (timeline_scroll_state.offset.x - delta)
                                                    .clamp(0.0, max_offset);
                                            timeline_scroll_changed = true;
                                        }
                                        TimelineWheelAction::VerticalScroll(delta) => {
                                            let max_offset = (timeline_content_size.y
                                                - timeline_viewport.height())
                                                .max(0.0);
                                            timeline_scroll_state.offset.y =
                                                (timeline_scroll_state.offset.y - delta)
                                                    .clamp(0.0, max_offset);
                                            timeline_scroll_changed = true;
                                        }
                                        TimelineWheelAction::MultiGesture { zoom_factor, translation } => {
                                            let next_zoom = (zoom * zoom_factor).clamp(20.0, 500.0);
                                            let pointer_x = pointer_screen_x - timeline_viewport.left();
                                            timeline_scroll_state.offset.x = timeline_offset_for_pointer_zoom(
                                                timeline_scroll_state.offset.x, pointer_x, zoom, next_zoom,
                                                total_seconds, timeline_viewport.width());
                                            self.app.timeline_zoom = next_zoom;
                                            let max_x = (total_seconds as f32 * next_zoom
                                                - timeline_viewport.width()).max(0.0);
                                            let max_y = (timeline_content_size.y
                                                - timeline_viewport.height()).max(0.0);
                                            timeline_scroll_state.offset.x =
                                                (timeline_scroll_state.offset.x - translation.x).clamp(0.0, max_x);
                                            timeline_scroll_state.offset.y =
                                                (timeline_scroll_state.offset.y - translation.y).clamp(0.0, max_y);
                                            timeline_scroll_changed = true;
                                        }
                                    }
                                }
                            }

                            if held_timeline_pan != 0.0 {
                                // Integrate held keys/buttons by frame time, rather than
                                // restarting an easing animation on every OS key repeat.
                                ui.data_mut(|data| {
                                    data.remove_temp::<TimelineNavigationTransition>(navigation_transition_id);
                                });
                                let dt = ui.input(|input| input.stable_dt);
                                let max_x = (timeline_content_size.x - timeline_viewport.width()).max(0.0);
                                timeline_scroll_state.offset.x = (timeline_scroll_state.offset.x
                                    + crate::ui::timeline_toolbar::held_pan_delta(
                                        held_timeline_pan, timeline_viewport.width(), dt))
                                    .clamp(0.0, max_x);
                                timeline_scroll_changed = true;
                                ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
                            }

                            // Middle-button dragging pans the existing two-axis
                            // ScrollArea viewport. Track the gesture independently
                            // of child responses so panning also works when it starts
                            // over a cue, keyframe, or ruler, and remains active if
                            // the pointer leaves the viewport before release.
                            let middle_pan_id = timeline_scroll_id.with("middle-button-pan");
                            let (
                                middle_pressed,
                                middle_down,
                                middle_released,
                                pointer_position,
                                pointer_delta,
                            ) = ui.input(|input| {
                                (
                                    input.pointer.button_pressed(egui::PointerButton::Middle),
                                    input.pointer.button_down(egui::PointerButton::Middle),
                                    input.pointer.button_released(egui::PointerButton::Middle),
                                    input.pointer.latest_pos(),
                                    input.pointer.delta(),
                                )
                            });
                            let mut middle_pan_active = ui
                                .data(|data| data.get_temp::<bool>(middle_pan_id))
                                .unwrap_or(false);
                            if self.app.timeline_middle_button_pan
                                && middle_pressed
                                && pointer_position
                                    .is_some_and(|position| timeline_viewport.contains(position))
                            {
                                ui.data_mut(|data| {
                                    data.remove_temp::<TimelineNavigationTransition>(
                                        navigation_transition_id,
                                    );
                                });
                                middle_pan_active = true;
                                self.app.lasso_origin = None;
                                self.app.lasso_rect = None;
                            }

                            let mut middle_pan_changed = false;
                            if middle_pan_active && middle_down {
                                let previous_offset = timeline_scroll_state.offset;
                                let constrained_delta = constrain_middle_pan_delta(
                                    pointer_delta,
                                    scroll_modifiers.shift,
                                    scroll_modifiers.ctrl || scroll_modifiers.command,
                                    self.app.timeline_middle_axis_lock_modifiers,
                                );
                                timeline_scroll_state.offset = pan_timeline_offset(
                                    previous_offset,
                                    constrained_delta,
                                    timeline_content_size,
                                    timeline_viewport.size(),
                                );
                                middle_pan_changed = timeline_scroll_state.offset != previous_offset;
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                ui.ctx().request_repaint();
                            }
                            if middle_released || !middle_down {
                                middle_pan_active = false;
                            }
                            ui.data_mut(|data| {
                                data.insert_temp(middle_pan_id, middle_pan_active);
                            });
                            timeline_scroll_changed |= middle_pan_changed;

                            if self.app.timeline_follow_playhead
                                && !self.app.is_paused
                                && held_timeline_pan == 0.0
                                && !middle_pan_active
                                && ui.data(|data| {
                                    data.get_temp::<TimelineNavigationTransition>(
                                        navigation_transition_id,
                                    )
                                    .is_none()
                                })
                            {
                                let content_width =
                                    (total_seconds as f32 * self.app.timeline_zoom).max(0.0);
                                let playhead_x = (self.app.playback_time.max(0.0) as f32
                                    * self.app.timeline_zoom)
                                    .min(content_width);
                                if let Some(target_x) = timeline_follow_target_offset(
                                    timeline_scroll_state.offset.x,
                                    playhead_x,
                                    content_width,
                                    timeline_viewport.width(),
                                ) {
                                    let target_offset = egui::vec2(
                                        target_x,
                                        timeline_scroll_state.offset.y,
                                    );
                                    if self.app.timeline_animated_navigation {
                                        start_timeline_navigation_transition(
                                            ui, navigation_transition_id,
                                            (timeline_scroll_state.offset, self.app.timeline_zoom),
                                            (target_offset, self.app.timeline_zoom),
                                            self.app.timeline_navigation_transition_ms,
                                        );
                                    } else {
                                        timeline_scroll_state.offset = target_offset;
                                        timeline_scroll_changed = true;
                                    }
                                }
                            }
                            if timeline_scroll_changed {
                                timeline_scroll_state.store(ui.ctx(), timeline_scroll_id);
                            }

                            let synced_offset_y = timeline_scroll_state.offset.y;
                            ui.ctx().data_mut(|data| {
                                data.insert_persisted(
                                    timeline_vertical_sync_id,
                                    synced_offset_y,
                                );
                            });
                            if let Some(header_scroll_id) = timeline_header_scroll_id
                                && let Some(mut header_state) =
                                    egui::scroll_area::State::load(ui.ctx(), header_scroll_id)
                                && header_state.offset.y != synced_offset_y
                            {
                                header_state.offset.y = synced_offset_y;
                                header_state.store(ui.ctx(), header_scroll_id);
                                ui.ctx().request_repaint();
                            }

                            if ui.input(|input| input.pointer.any_pressed()) {
                                if ui
                                    .ctx()
                                    .pointer_latest_pos()
                                    .is_some_and(|position| timeline_viewport.contains(position))
                                {
                                    ui.ctx().memory_mut(|memory| {
                                        memory.request_focus(timeline_keyboard_focus_id());
                                    });
                                } else if ui.ctx().memory(|memory| {
                                    memory.has_focus(timeline_keyboard_focus_id())
                                }) {
                                    ui.ctx().memory_mut(|memory| {
                                        memory.surrender_focus(timeline_keyboard_focus_id());
                                    });
                                }
                            }

                            let tracks_top = rect.min.y + TIMELINE_RULER_HEIGHT;

                            // Successful drop logic
                            if let Some(payload) = take_effect_drop_on_rect(ui.ctx(), rect) {
                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                    if rect.contains(mouse_pos) && mouse_pos.y >= timeline_viewport.top() + TIMELINE_RULER_HEIGHT {
                                        let relative_y = mouse_pos.y - tracks_top;
                                        let visible_row =
                                            (relative_y / timeline_track_height).floor() as i32;
                                        let relative_x = mouse_pos.x - rect.min.x;
                                        let drop_time_secs = (relative_x / zoom) as f64;

                                        self.app.handle_effect_drop(&payload, visible_row, drop_time_secs);
                                    }
                                }
                            }

                            // Background click, seek, or lasso selection logic
                            let ruler_bottom = timeline_viewport.top() + TIMELINE_RULER_HEIGHT;

                            if response.drag_started_by(egui::PointerButton::Primary)
                                && !clicked_any_clip
                                && !clicked_any_keyframe
                                && self.app.active_drag.is_none()
                                && self.app.active_keyframe_drag.is_none()
                            {
                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                    if mouse_pos.y >= ruler_bottom {
                                        self.app.lasso_origin = Some(mouse_pos);
                                        let multi_select = ui.ctx().input(|input| {
                                            input.modifiers.ctrl || input.modifiers.command
                                        });
                                        self.app.lasso_initial_instance_ids = if multi_select {
                                            self.app.selected_instance_ids.clone()
                                        } else {
                                            std::collections::HashSet::new()
                                        };
                                        self.app.lasso_initial_keyframes = if multi_select {
                                            self.app.selected_keyframes.clone()
                                        } else {
                                            std::collections::HashSet::new()
                                        };
                                    }
                                }
                            }

                            if response.dragged_by(egui::PointerButton::Primary)
                                && self.app.lasso_origin.is_some()
                            {
                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                    let origin = self.app.lasso_origin.unwrap();
                                    let lasso_rect = egui::Rect::from_two_pos(origin, mouse_pos);
                                    let marquee_mode = timeline_marquee_mode(origin, mouse_pos);
                                    self.app.lasso_rect = Some(lasso_rect);

                                    let mut new_instance_selection =
                                        self.app.lasso_initial_instance_ids.clone();
                                    let mut new_keyframe_selection =
                                        self.app.lasso_initial_keyframes.clone();

                                    // Instances
                                    for instance in &self.app.timeline.instances {
                                        if let Some(effect) = self.app.timeline.templates.iter().find(|t| t.id == instance.effect_id) {
                                            let Some(placement) = timeline_cue_placement(
                                                self.app,
                                                effect,
                                                &timeline_rows,
                                                &visible_analog_track_ids,
                                            ) else {
                                                continue;
                                            };
                                            let (track_y, cue_row_height) = if let Some(analog_index) = placement.analog_index {
                                                (
                                                    tracks_top
                                                        + track_area_height
                                                        + analog_index as f32 * timeline_analog_height,
                                                    timeline_analog_height,
                                                )
                                            } else {
                                                (
                                                    timeline_track_row_top(
                                                        rect.min.y,
                                                        placement.track_index,
                                                        timeline_track_height,
                                                    ),
                                                    timeline_track_height,
                                                )
                                            };
                                            let start_x = rect.min.x + (instance.start_time_ms as f32 * px_per_ms);
                                            let end_x = start_x + (effect.duration_ms.max(1) as f32 * px_per_ms);
                                            let clip_rect = egui::Rect::from_min_max(
                                                egui::pos2(start_x, track_y + 4.0),
                                                egui::pos2(
                                                    end_x.max(start_x + 8.0),
                                                    track_y + cue_row_height - 4.0,
                                                ),
                                            );
                                            if timeline_marquee_selects_rect(
                                                lasso_rect,
                                                clip_rect,
                                                marquee_mode,
                                            ) {
                                                new_instance_selection.insert(instance.id);
                                            }
                                        }
                                    }

                                    // Keyframes
                                    for (t_idx, track) in self
                                        .app
                                        .timeline
                                        .analog_tracks
                                        .iter()
                                        .filter(|track| visible_analog_track_ids.contains(&track.id))
                                        .enumerate()
                                    {
                                        let t_y = tracks_top
                                            + track_area_height
                                            + t_idx as f32 * timeline_analog_height;
                                        let curve_bottom =
                                            t_y + timeline_analog_height - 4.0;
                                        let curve_span = timeline_analog_height - 8.0;
                                        for (k_idx, kf) in track.keyframes.iter().enumerate() {
                                            let k_x = rect.min.x + (kf.time_ms as f32 * px_per_ms);
                                            let k_y = curve_bottom - kf.value * curve_span;
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
                            if ui.ctx().input(|i| {
                                i.pointer.button_released(egui::PointerButton::Primary)
                            }) {
                                lasso_ended = true;
                            }

                            let lasso_was_active = self.app.lasso_origin.is_some();
                            if lasso_ended {
                                self.app.lasso_origin = None;
                                self.app.lasso_rect = None;
                                self.app.lasso_initial_instance_ids.clear();
                                self.app.lasso_initial_keyframes.clear();
                            }

                            if response.double_clicked()
                                && !clicked_any_clip
                                && !clicked_any_keyframe
                                && self.app.active_drag.is_none()
                                && self.app.active_keyframe_drag.is_none()
                                && !lasso_was_active
                                && !popup_was_open
                                && !egui::Popup::is_any_open(ui.ctx())
                            {
                                if let Some(mouse_pos) = response.interact_pointer_pos()
                                    && mouse_pos.y >= tracks_top
                                    && mouse_pos.y < tracks_top + track_area_height
                                {
                                    let row_index = ((mouse_pos.y - tracks_top)
                                        / timeline_track_height)
                                        .floor()
                                        as usize;
                                    if let Some(track_key) = timeline_rows
                                        .get(row_index)
                                        .map(|row| row.key.clone())
                                    {
                                        let start_time_ms = timeline_pointer_time_ms(
                                            mouse_pos.x,
                                            rect.min.x,
                                            px_per_ms,
                                            (total_seconds * 1_000.0).round() as u64,
                                        );
                                        request_timeline_cue_dialog(
                                            self.app,
                                            ui.ctx(),
                                            &track_key,
                                            start_time_ms,
                                        );
                                        clicked_any_clip = true;
                                    }
                                }
                            }

                            if response.clicked_by(egui::PointerButton::Primary)
                                && !clicked_any_clip
                                && !clicked_any_keyframe
                                && self.app.active_drag.is_none()
                                && self.app.active_keyframe_drag.is_none()
                                && !lasso_was_active
                            {
                                if let Some(mouse_pos) = response.interact_pointer_pos() {
                                    if mouse_pos.y >= tracks_top && mouse_pos.y < tracks_top + track_area_height {
                                        let row_index = ((mouse_pos.y - tracks_top)
                                            / timeline_track_height)
                                            .floor()
                                            as usize;
                                        self.app.selected_timeline_track = timeline_rows
                                            .get(row_index)
                                            .map(|row| row.key.clone());
                                        self.app.selected_instance_ids.clear();
                                        self.app.selected_timeline_keyframe = None;
                                    } else if mouse_pos.y >= tracks_top + track_area_height {
                                        let analog_index = ((mouse_pos.y
                                            - tracks_top
                                            - track_area_height)
                                            / timeline_analog_height)
                                            .floor()
                                            as usize;
                                        if let Some(track) = self
                                            .app
                                            .timeline
                                            .analog_tracks
                                            .iter()
                                            .filter(|track| {
                                                visible_analog_track_ids.contains(&track.id)
                                            })
                                            .nth(analog_index)
                                        {
                                            self.app.selected_timeline_track = Some(
                                                crate::four_d::models::hardware_timeline_track_key(
                                                    &format!("pwm.{}", track.channel),
                                                ),
                                            );
                                        }
                                    }
                                }
                            }

                            if !keyframe_context_owned { response.context_menu(|ui| {
                                ui.label(egui::RichText::new(self.app.tr("Timeline")).strong());
                                ui.separator();
                                if ui
                                    .button(format!(
                                        "{} {}",
                                        crate::ui::icons::TARGET,
                                        self.app.tr("Bring playhead into view")
                                    ))
                                    .clicked()
                                {
                                    let playhead_x =
                                        (self.app.playback_time.max(0.0) as f32 * zoom)
                                            .min(timeline_content_size.x);
                                    let target_offset_x = timeline_offset_to_reveal_x(
                                        timeline_scroll_state.offset.x,
                                        playhead_x,
                                        timeline_content_size.x,
                                        timeline_viewport.width(),
                                    );
                                    if self.app.timeline_animated_navigation
                                        && (target_offset_x - timeline_scroll_state.offset.x).abs()
                                            > 0.5
                                    {
                                        start_timeline_navigation_transition(
                                            ui, navigation_transition_id,
                                            (timeline_scroll_state.offset, self.app.timeline_zoom),
                                            (egui::vec2(target_offset_x, timeline_scroll_state.offset.y), self.app.timeline_zoom),
                                            self.app.timeline_navigation_transition_ms,
                                        );
                                    } else {
                                        ui.data_mut(|data| {
                                            data.remove_temp::<TimelineNavigationTransition>(
                                                navigation_transition_id,
                                            );
                                        });
                                        timeline_scroll_state.offset.x = target_offset_x;
                                        timeline_scroll_state
                                            .store(ui.ctx(), timeline_scroll_id);
                                    }
                                    ui.close();
                                }
                                ui.separator();
                                if ui.button(format!("{} {}", crate::ui::icons::SELECTION_ALL, self.app.tr("Select all cues"))).clicked() {
                                    self.app.selected_instance_ids = self.app.timeline.instances.iter().map(|instance| instance.id).collect();
                                    self.app.selected_keyframes.clear();
                                    self.app.selected_timeline_keyframe = None;
                                    for track in self
                                        .app
                                        .timeline
                                        .analog_tracks
                                        .iter()
                                        .filter(|track| visible_analog_track_ids.contains(&track.id))
                                    {
                                        for (index, _) in track.keyframes.iter().enumerate() {
                                            self.app.selected_keyframes.insert((track.id, index));
                                        }
                                    }
                                    ui.close();
                                }
                                if ui.button(format!("{} {}", crate::ui::icons::ERASER, self.app.tr("Clear selection"))).clicked() {
                                    self.app.selected_instance_ids.clear();
                                    self.app.selected_keyframes.clear();
                                    self.app.selected_timeline_keyframe = None;
                                    ui.close();
                                }
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label(format!("{} {}", crate::ui::icons::MAGNIFYING_GLASS, self.app.tr("Zoom")));
                                    ui.add(egui::Slider::new(&mut self.app.timeline_zoom, TIMELINE_MIN_ZOOM..=TIMELINE_MAX_ZOOM).logarithmic(true).suffix(" px/s"));
                                });
                                if ui.button(format!("{} {}", crate::ui::icons::ARROW_COUNTER_CLOCKWISE, self.app.tr("Reset zoom"))).clicked() {
                                    self.app.timeline_zoom = 100.0;
                                    ui.close();
                                }
                                if ui.button(format!("{} {}", crate::ui::icons::ARROWS_OUT, self.app.tr("Fit to window (Shift+Z)"))).clicked() {
                                    fit_timeline_to_window(
                                        self.app,
                                        ui,
                                        timeline_viewport.width(),
                                        total_seconds,
                                        &mut timeline_scroll_state,
                                        navigation_transition_id,
                                    );
                                    timeline_scroll_changed = true;
                                    ui.close();
                                }
                                ui.separator();
                                let mut compact_rows = self.app.compact_timeline_tracks;
                                if ui
                                    .checkbox(
                                        &mut compact_rows,
                                        self.app.tr("Compact track rows"),
                                    )
                                    .changed()
                                {
                                    self.app.compact_timeline_tracks = compact_rows;
                                    self.app.save_config();
                                }
                                if ui.button(format!("{} {}", crate::ui::icons::GEAR, self.app.tr("Preferences..."))).clicked() {
                                    crate::ui::preferences::open(self.app, ui.ctx());
                                    ui.close();
                                }
                            });

                            }
                            clear_unfocused_timeline_keyframes(self.app, ui, keyframe_hit, popup_was_open);
                            delete_selected_cues_without_timeline_focus(self.app, ui);

                            if ui.ctx().memory(|memory| {
                                memory.has_focus(timeline_keyboard_focus_id())
                            }) {
                                let delete_pressed = ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace));
                                let undo_pressed = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift);
                                let redo_pressed = ui.input(|i| (i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) || (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z)));
                                let select_all_pressed = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::A));
                                let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

                                let add_timeline_keyframe_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::K)
                                        && !i.modifiers.ctrl
                                        && !i.modifiers.command
                                        && !i.modifiers.alt
                                });
                                let add_timeline_cue_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::A)
                                        && !i.modifiers.ctrl
                                        && !i.modifiers.command
                                        && !i.modifiers.alt
                                        && !i.modifiers.shift
                                }) && ui.ctx().data(|data| {
                                    data.get_temp::<TimelineCueDraft>(timeline_cue_dialog_id())
                                        .is_none()
                                });
                                let previous_cue_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::Tab) && i.modifiers.shift
                                });
                                let next_cue_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::Tab) && !i.modifiers.shift
                                });
                                let nudge_left_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::ArrowLeft)
                                        && i.modifiers.alt
                                        && !i.modifiers.ctrl
                                        && !i.modifiers.command
                                });
                                let nudge_right_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::ArrowRight)
                                        && i.modifiers.alt
                                        && !i.modifiers.ctrl
                                        && !i.modifiers.command
                                });
                                let jump_previous_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::ArrowLeft)
                                        && (i.modifiers.ctrl || i.modifiers.command)
                                        && i.modifiers.shift
                                        && !i.modifiers.alt
                                });
                                let jump_next_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::ArrowRight)
                                        && (i.modifiers.ctrl || i.modifiers.command)
                                        && i.modifiers.shift
                                        && !i.modifiers.alt
                                });
                                let playhead_left_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::ArrowLeft) && i.modifiers.is_none()
                                });
                                let playhead_right_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::ArrowRight) && i.modifiers.is_none()
                                });
                                let scroll_up_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::ArrowUp) && i.modifiers.is_none()
                                });
                                let scroll_down_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::ArrowDown) && i.modifiers.is_none()
                                });
                                let page_left_pressed = ui.input(|i| i.key_pressed(egui::Key::PageUp));
                                let page_right_pressed = ui.input(|i| i.key_pressed(egui::Key::PageDown));
                                let home_pressed = ui.input(|i| i.key_pressed(egui::Key::Home));
                                let end_pressed = ui.input(|i| i.key_pressed(egui::Key::End));
                                let fit_timeline_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::Z)
                                        && i.modifiers.shift
                                        && !i.modifiers.ctrl
                                        && !i.modifiers.command
                                        && !i.modifiers.alt
                                });

                                let num_modifier_free = ui.input(|i| !i.modifiers.ctrl && !i.modifiers.alt && !i.modifiers.command && !i.modifiers.shift);
                                let key_1_pressed = ui.input(|i| i.key_pressed(egui::Key::Num1)) && num_modifier_free;
                                let key_2_pressed = ui.input(|i| i.key_pressed(egui::Key::Num2)) && num_modifier_free;
                                let key_3_pressed = ui.input(|i| i.key_pressed(egui::Key::Num3)) && num_modifier_free;

                                if add_timeline_cue_pressed {
                                    let start_time_ms = (self.app.playback_time * 1_000.0)
                                        .round()
                                        .clamp(0.0, total_seconds * 1_000.0)
                                        as u64;
                                    if let Some(track_key) =
                                        self.app.selected_timeline_track.clone()
                                    {
                                        request_timeline_cue_dialog(
                                            self.app,
                                            ui.ctx(),
                                            &track_key,
                                            start_time_ms,
                                        );
                                    } else {
                                        let message = self.app.tr(
                                            "Select a relay or PWM timeline track first",
                                        );
                                        self.app.set_osd(message);
                                    }
                                } else if add_timeline_keyframe_pressed {
                                    let time_ms = (self.app.playback_time * 1_000.0)
                                        .round()
                                        .clamp(0.0, total_seconds * 1_000.0)
                                        as u64;
                                    self.app.insert_timeline_keyframe(time_ms);
                                    ui.ctx().request_repaint();
                                } else if nudge_left_pressed || nudge_right_pressed {
                                    if !self.app.selected_instance_ids.is_empty() {
                                        let direction = if nudge_left_pressed { -1 } else { 1 };
                                        let delta = timeline_frame_step_ms(
                                            self.app.media_fps,
                                            self.app.frame_step_count,
                                        ) as i64
                                            * direction;
                                        let snapshot = self.app.snapshot_timeline();
                                        if move_selected_cues(
                                            &mut self.app.timeline,
                                            &self.app.selected_instance_ids,
                                            delta,
                                        ) {
                                            self.app.undo_stack.push(snapshot);
                                            self.app.sync_timeline_engine();
                                        }
                                    }
                                } else if jump_previous_pressed || jump_next_pressed {
                                    let selected_none = std::collections::HashSet::new();
                                    let targets = timeline_snap_targets(
                                        &self.app.timeline,
                                        &selected_none,
                                        0,
                                    );
                                    let current_ms = (self.app.playback_time * 1_000.0)
                                        .round()
                                        .max(0.0) as u64;
                                    if let Some(target_ms) = adjacent_timeline_time(
                                        current_ms,
                                        jump_next_pressed,
                                        &targets,
                                    ) {
                                        self.app.seek_absolute(target_ms as f64 / 1_000.0);
                                        timeline_scroll_state.offset.x =
                                            (target_ms as f32 * px_per_ms
                                                - timeline_viewport.width() * 0.35)
                                                .clamp(
                                                    0.0,
                                                    (timeline_content_size.x - timeline_viewport.width())
                                                        .max(0.0),
                                                );
                                    }
                                } else if previous_cue_pressed || next_cue_pressed {
                                    ui.ctx().input_mut(|input| {
                                        input.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
                                        input.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab);
                                    });
                                    let cue_ids = sorted_cue_ids(&self.app.timeline);
                                    if !cue_ids.is_empty() {
                                        let selected_index = cue_ids.iter().position(|id| {
                                            self.app.selected_instance_ids.contains(id)
                                        });
                                        let next_index = if previous_cue_pressed {
                                            selected_index
                                                .unwrap_or(0)
                                                .checked_sub(1)
                                                .unwrap_or(cue_ids.len() - 1)
                                        } else {
                                            selected_index
                                                .map(|index| (index + 1) % cue_ids.len())
                                                .unwrap_or(0)
                                        };
                                        let cue_id = cue_ids[next_index];
                                        self.app.selected_instance_ids.clear();
                                        self.app.selected_instance_ids.insert(cue_id);
                                        self.app.selected_keyframes.clear();
                                        self.app.selected_timeline_keyframe = None;
                                        if let Some(instance) = self
                                            .app
                                            .timeline
                                            .instances
                                            .iter()
                                            .find(|instance| instance.id == cue_id)
                                        {
                                            timeline_scroll_state.offset.x =
                                                (instance.start_time_ms as f32 * px_per_ms
                                                    - timeline_viewport.width() * 0.35)
                                                    .clamp(
                                                        0.0,
                                                        (timeline_content_size.x
                                                            - timeline_viewport.width())
                                                            .max(0.0),
                                                    );
                                        }
                                    }
                                } else if scroll_up_pressed || scroll_down_pressed {
                                    let direction = if scroll_up_pressed { -1.0 } else { 1.0 };
                                    timeline_scroll_state.offset.y =
                                        (timeline_scroll_state.offset.y
                                            + direction * timeline_track_height)
                                            .clamp(
                                            0.0,
                                            (timeline_content_size.y - timeline_viewport.height())
                                                .max(0.0),
                                        );
                                } else if page_left_pressed || page_right_pressed {
                                    let direction = if page_left_pressed { -1.0 } else { 1.0 };
                                    timeline_scroll_state.offset.x =
                                        (timeline_scroll_state.offset.x
                                            + direction * timeline_viewport.width() * 0.85)
                                            .clamp(
                                                0.0,
                                                (timeline_content_size.x - timeline_viewport.width())
                                                    .max(0.0),
                                            );
                                } else if home_pressed || end_pressed {
                                    let target_seconds = if home_pressed { 0.0 } else { total_seconds };
                                    self.app.seek_absolute(target_seconds);
                                    timeline_scroll_state.offset.x = if home_pressed {
                                        0.0
                                    } else {
                                        (timeline_content_size.x - timeline_viewport.width()).max(0.0)
                                    };
                                } else if fit_timeline_pressed {
                                    fit_timeline_to_window(
                                        self.app,
                                        ui,
                                        timeline_viewport.width(),
                                        total_seconds,
                                        &mut timeline_scroll_state,
                                        navigation_transition_id,
                                    );
                                    timeline_scroll_changed = true;
                                } else if playhead_left_pressed || playhead_right_pressed {
                                    let direction = if playhead_left_pressed { -1.0 } else { 1.0 };
                                    let step_seconds = timeline_frame_step_ms(
                                        self.app.media_fps,
                                        self.app.frame_step_count,
                                    ) as f64
                                        / 1_000.0;
                                    self.app.seek_absolute(
                                        (self.app.playback_time + direction * step_seconds)
                                            .clamp(0.0, total_seconds),
                                    );
                                } else if undo_pressed {
                                    let current = self.app.snapshot_timeline();
                                    if let Some(prev) = self.app.undo_stack.undo(current) {
                                        self.app.restore_timeline_snapshot(prev);
                                        self.app.selected_instance_ids.clear();
                                        self.app.selected_keyframes.clear();
                                        self.app.selected_timeline_keyframe = None;
                                        self.app.sync_timeline_engine();
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.linked_analog_tracks()));
                                    }
                                } else if redo_pressed {
                                    let current = self.app.snapshot_timeline();
                                    if let Some(next) = self.app.undo_stack.redo(current) {
                                        self.app.restore_timeline_snapshot(next);
                                        self.app.selected_instance_ids.clear();
                                        self.app.selected_keyframes.clear();
                                        self.app.selected_timeline_keyframe = None;
                                        let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.linked_analog_tracks()));
                                    }
                                } else if select_all_pressed {
                                    self.app.selected_instance_ids = self.app.timeline.instances.iter().map(|i| i.id).collect();
                                    self.app.selected_keyframes.clear();
                                    for track in self
                                        .app
                                        .timeline
                                        .analog_tracks
                                        .iter()
                                        .filter(|track| visible_analog_track_ids.contains(&track.id))
                                    {
                                        for (k_idx, _) in track.keyframes.iter().enumerate() {
                                            self.app.selected_keyframes.insert((track.id, k_idx));
                                        }
                                    }
                                } else if escape_pressed {
                                    self.app.selected_instance_ids.clear();
                                    self.app.selected_keyframes.clear();
                                    self.app.selected_timeline_keyframe = None;
                                } else if delete_pressed {
                                    if !self.app.selected_instance_ids.is_empty()
                                        && self.app.selected_keyframes.is_empty()
                                        && self.app.selected_timeline_keyframe.is_none()
                                    {
                                        self.app.delete_selected_timeline_cues();
                                    } else if !self.app.selected_instance_ids.is_empty()
                                        || !self.app.selected_keyframes.is_empty()
                                        || self.app.selected_timeline_keyframe.is_some()
                                    {
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

                                        if let Some(id) = self.app.selected_timeline_keyframe.take() {
                                            self.app.timeline.remove_keyframe(id);
                                        }

                                        self.app.selected_instance_ids.clear();
                                        self.app.selected_keyframes.clear();
                                        self.app.commit_timeline_edit();
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
                                        self.app.commit_timeline_edit();
                                    }
                                }
                                timeline_scroll_state.store(ui.ctx(), timeline_scroll_id);
                            }
                        });
                    }
                }
              });
            });
        if matches!(tab, PealayerTab::Timeline) {
            draw_timeline_cue_dialog(self.app, ui.ctx());
        }
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
                .or_else(|| dock_state.iter_all_tabs().map(|(path, _)| path).next())
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
        PealayerTab::MediaInspector => {
            if let Some(sibling_path) = dock_state.find_tab(&PealayerTab::EffectsLibrary) {
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
                    .split_right(node_index, 0.72, vec![tab]);
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

/// Makes a workspace tab visibly active and keyboard-focused, restoring it
/// first when the user previously closed it. Activating a tab alone is not
/// enough: its leaf (or one of its split ancestors) may still be collapsed,
/// which made cue "Manage..." actions appear to do nothing.
pub fn reveal_and_focus_tab(
    dock_state: &mut egui_dock::DockState<PealayerTab>,
    tab: PealayerTab,
) -> bool {
    let was_open = dock_state.find_tab(&tab).is_some();
    if !was_open {
        restore_tab_to_canonical_slot(dock_state, tab);
    }

    let Some(path) = dock_state.find_tab(&tab) else {
        return false;
    };

    let node_path = path.node_path();
    let mut layout_changed = !was_open;
    let mut node = Some(node_path.node);
    while let Some(node_index) = node {
        let ancestor = egui_dock::NodePath::new(node_path.surface, node_index);
        if let Ok(dock_node) = dock_state.node_mut(ancestor) {
            if dock_node.is_collapsed() {
                dock_node.set_collapsed(false);
                layout_changed = true;
            }
        }
        node = node_index.parent();
    }

    if dock_state
        .leaf(node_path)
        .is_ok_and(|leaf| leaf.active != path.tab)
    {
        layout_changed = true;
    }
    let _ = dock_state.set_active_tab(path);
    dock_state.set_focused_node_and_surface(node_path);
    layout_changed
}

fn format_timecode(t: f64) -> String {
    crate::duration::format_timeline_time_ms((t.max(0.0) * 1_000.0).round() as u64, true)
}

impl PealayerApp {
    pub(crate) fn place_controller_effect_at_playhead(
        &mut self,
        payload: &EffectDragPayload,
    ) -> bool {
        let rows = timeline_track_rows(self);
        let lane = payload
            .controller_lane
            .unwrap_or(crate::four_d::models::ControllerEffectLane::Sequence);
        let Some(index) = rows
            .iter()
            .position(|row| row.kind == TimelineTrackKind::ControllerEffect(lane))
        else {
            self.set_osd(format!(
                "{} {}",
                controller_effect_lane_label(self, lane),
                self.tr("track is not available")
            ));
            return false;
        };
        self.handle_effect_drop(payload, index as i32, self.playback_time)
    }

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
            let lane = payload
                .controller_lane
                .unwrap_or(crate::four_d::models::ControllerEffectLane::Sequence);
            let valid_track = usize::try_from(track_index)
                .ok()
                .and_then(|index| timeline_rows.get(index))
                .is_some_and(|row| row.kind == TimelineTrackKind::ControllerEffect(lane));
            if !valid_track {
                self.set_osd(format!(
                    "Place '{}' on the {} lane",
                    payload.name,
                    controller_effect_lane_label(self, lane)
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
                self.timeline.templates.iter_mut().find(|effect| {
                    effect.controller_macro == payload.controller_macro
                        && effect.controller_strip_effect == payload.controller_strip_effect
                }) {
                existing.name.clone_from(&payload.name);
                existing.icon.clone_from(&payload.icon);
                existing.duration_ms = payload.duration_ms.max(1);
                existing.controller_lane = Some(lane);
                existing.duration_policy = crate::four_d::models::CueDurationPolicy::Resizable;
                existing.id
            } else {
                let mut effect = if let Some(controller_macro) = payload.controller_macro.as_ref() {
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
                effect.controller_lane = Some(lane);
                effect.duration_policy = crate::four_d::models::CueDurationPolicy::Resizable;
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
        let template_id = if let Some(existing) = self.timeline.templates.iter_mut().find(|t| {
            t.name == payload.name
                && t.duration_ms == payload.duration_ms
                && t.target == payload.target
                && t.actions.first().map(|a| a.relay_id) == Some(target_relay)
        }) {
            existing.duration_policy = crate::four_d::models::CueDurationPolicy::Resizable;
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
            let mut new_effect = crate::four_d::models::Effect::with_target(
                payload.name.clone(),
                payload.icon.clone(),
                payload.duration_ms,
                payload.target,
                actions,
            );
            new_effect.duration_policy = crate::four_d::models::CueDurationPolicy::Resizable;
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

        self.commit_timeline_edit();

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
    if clip_rect.width() < 12.0 {
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
            egui::Color32::from_rgba_unmultiplied(0, 220, 255, (60.0 * alpha_scale) as u8),
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
            egui::Color32::from_rgba_unmultiplied(0, 220, 255, (60.0 * alpha_scale) as u8),
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
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, (120.0 * alpha_scale) as u8)
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
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, (120.0 * alpha_scale) as u8)
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

/// Paints a centered move affordance on intrinsic-duration cues. Unlike the
/// edge handles on resizable cues, this grip deliberately sits inside the
/// clip so it cannot be mistaken for a duration control.
fn render_clip_move_grip(painter: &egui::Painter, clip_rect: egui::Rect, alpha: u8) {
    if clip_rect.width() < 12.0 || clip_rect.height() < 12.0 {
        return;
    }

    let compact = clip_rect.width() < 30.0;
    let center_x = if compact {
        clip_rect.center().x
    } else {
        clip_rect.left() + 11.0
    };
    let grip_height = (clip_rect.height() - 10.0).clamp(6.0, 12.0);
    let grip_rect = egui::Rect::from_center_size(
        egui::pos2(center_x, clip_rect.center().y),
        egui::vec2(12.0, grip_height + 6.0),
    );
    let alpha_scale = alpha as f32 / 255.0;
    painter.rect_filled(
        grip_rect,
        3.0,
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, (42.0 * alpha_scale) as u8),
    );

    let bar_color =
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, (175.0 * alpha_scale) as u8);
    for offset in [-3.0_f32, 0.0, 3.0] {
        painter.line_segment(
            [
                egui::pos2(center_x + offset, clip_rect.center().y - grip_height / 2.0),
                egui::pos2(center_x + offset, clip_rect.center().y + grip_height / 2.0),
            ],
            egui::Stroke::new(1.25, bar_color),
        );
    }
}

/// Renders a cue label with native egui elision while preserving the opt-in
/// legacy overflow mode. The painter is constrained to the cue interior so
/// glyphs never leak through rounded borders in ellipsis mode.
fn render_timeline_cue_label(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    title: String,
    font_id: egui::FontId,
    color: egui::Color32,
    hide_overflow: bool,
    duration_resizable: bool,
) {
    let left_padding = if duration_resizable { 12.0 } else { 22.0 };
    if !hide_overflow {
        painter.text(
            clip_rect.left_center() + egui::vec2(left_padding, 0.0),
            egui::Align2::LEFT_CENTER,
            title,
            font_id,
            color,
        );
        return;
    }

    let right_padding = if duration_resizable { 12.0 } else { 6.0 };
    let content_rect = egui::Rect::from_min_max(
        egui::pos2(clip_rect.left() + left_padding, clip_rect.top() + 2.0),
        egui::pos2(clip_rect.right() - right_padding, clip_rect.bottom() - 2.0),
    );
    if content_rect.width() < 2.0 || content_rect.height() < 2.0 {
        return;
    }

    let mut job = egui::text::LayoutJob {
        wrap: egui::text::TextWrapping::truncate_at_width(content_rect.width()),
        ..Default::default()
    };
    job.append(&title, 0.0, egui::TextFormat::simple(font_id, color));
    let galley = painter.layout_job(job);
    let position = egui::pos2(
        content_rect.left(),
        content_rect.center().y - galley.size().y / 2.0,
    );
    painter
        .with_clip_rect(content_rect)
        .galley(position, galley, color);
}
