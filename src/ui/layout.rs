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
const EFFECT_CARD_MIN_WIDTH: f32 = 140.0;
const EFFECT_CARD_HORIZONTAL_MARGIN: i8 = 9;
const EFFECT_CARD_STROKE_WIDTH: f32 = 1.0;
const EFFECT_CARD_ACTION_GUTTER: f32 = 100.0;
const EFFECT_CARD_ACTION_BUTTONS_WIDTH: f32 = 72.0;
const HARDWARE_CARD_STROKE_WIDTH: f32 = 1.0;
const BOARD_IDENTITY_TWO_LINE_HEIGHT: f32 = 42.0;
const BOARD_IDENTITY_LINE_GAP: f32 = 0.0;
const EFFECT_CONTROLS_RIGHT_GUTTER: f32 = 8.0;
const EFFECT_CONTROLS_CARD_MARGIN: i8 = 10;

#[derive(Clone, Debug, PartialEq, Eq)]
struct InlineEffectIdentityEdit {
    name: String,
    icon: String,
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

fn hardware_channel_drag_id() -> egui::Id {
    egui::Id::new("hardware-channel-drag")
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

pub(crate) fn hardware_channel_is_dragging(ui: &mut egui::Ui, key: &str) -> bool {
    ui.data_mut(|data| data.get_temp::<HardwareChannelDrag>(hardware_channel_drag_id()))
        .is_some_and(|drag| drag.key == key)
}

pub(crate) fn hardware_channel_drag_handle(
    app: &PealayerApp,
    ui: &mut egui::Ui,
    control: &crate::four_d::controller::HardwareControl,
) -> egui::Response {
    let source_rect_id = egui::Id::new(("hardware-channel-source-rect", control.key.as_str()));
    let active = ui
        .data_mut(|data| data.get_temp::<HardwareChannelDrag>(hardware_channel_drag_id()))
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
        egui::Id::new(("hardware-channel-handle-visible", control.key.as_str())),
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
                hardware_channel_drag_id(),
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
) {
    let source_rect_id = egui::Id::new(("hardware-channel-source-rect", control.key.as_str()));
    ui.data_mut(|data| data.insert_temp(source_rect_id, card_rect));
    let drag = ui.data_mut(|data| data.get_temp::<HardwareChannelDrag>(hardware_channel_drag_id()));
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
) -> Option<HardwareChannelDrop> {
    let drag =
        ui.data_mut(|data| data.get_temp::<HardwareChannelDrag>(hardware_channel_drag_id()))?;
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
    if ui.input(|input| input.pointer.any_released()) {
        ui.data_mut(|data| {
            data.remove_temp::<HardwareChannelDrag>(hardware_channel_drag_id());
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
            targets.push(instance.start_time_ms.saturating_add(effect.duration_ms));
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
    // Three 24 px action buttons plus the two gaps between them. The title and
    // action container are separate horizontal-layout items, so their own gap
    // must also be removed from the title budget. Omitting that final gap made
    // every child card one spacing unit wider than its group header.
    let actions_width = EFFECT_CARD_ACTION_BUTTONS_WIDTH + item_spacing * 2.0;
    let title_width = (available_after_icon - actions_width - item_spacing).max(1.0);
    (title_width, actions_width)
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

fn effect_controls_content_width(available_width: f32) -> f32 {
    (available_width - EFFECT_CONTROLS_RIGHT_GUTTER).max(1.0)
}

fn effect_controls_frame_content_width(outer_width: f32) -> f32 {
    (outer_width - f32::from(EFFECT_CONTROLS_CARD_MARGIN) * 2.0 - 2.0).max(1.0)
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
    ui.set_width(outer_width);
    let visuals = ui.visuals();
    let fill = if emphasized {
        visuals
            .selection
            .bg_fill
            .gamma_multiply(if visuals.dark_mode { 0.18 } else { 0.10 })
    } else {
        visuals.widgets.noninteractive.bg_fill
    };
    let stroke = if emphasized {
        egui::Stroke::new(1.0_f32, visuals.selection.bg_fill.gamma_multiply(0.72))
    } else {
        visuals.widgets.noninteractive.bg_stroke
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(9.0)
        .inner_margin(egui::Margin::symmetric(EFFECT_CONTROLS_CARD_MARGIN, 9))
        .show(ui, |ui| {
            ui.set_width(effect_controls_frame_content_width(outer_width));
            ui.horizontal(|ui| {
                let icon_color = if emphasized {
                    ui.visuals().selection.bg_fill
                } else {
                    ui.visuals().strong_text_color()
                };
                ui.label(egui::RichText::new(icon).size(17.0).color(icon_color));
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(title).strong());
                    if let Some(subtitle) = subtitle.filter(|value| !value.trim().is_empty()) {
                        ui.label(egui::RichText::new(subtitle).small().weak());
                    }
                });
            });
            ui.add_space(7.0);
            add_contents(ui)
        })
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
    if effect.controller_strip_effect.is_some() {
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

fn pwm_raw(percent: f64) -> u16 {
    (percent.clamp(0.0, 100.0) * 4095.0 / 100.0).round() as u16
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PwmEditorResponse {
    changed: bool,
    committed: bool,
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

fn send_pwm_raw(app: &PealayerApp, channel: u8, raw: u16) {
    let _ = app
        .engine_handle
        .sender
        .send(crate::four_d::engine::EngineMessage::ControllerCall {
            method: "controller.pwm.set".to_string(),
            params: serde_json::json!({"channel": channel, "value": raw}),
        });
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
    let telemetry_raw = if capabilities.telemetry.pwm_channel == Some(channel.id) {
        capabilities.telemetry.pwm_value.unwrap_or(0)
    } else {
        0
    };
    let displayed_raw = ui
        .data_mut(|data| data.get_temp::<u16>(value_id))
        .unwrap_or(telemetry_raw);
    let last_sent = ui
        .data_mut(|data| data.get_temp::<u16>(sent_id))
        .unwrap_or(telemetry_raw);
    let mut percent = pwm_percent(displayed_raw);
    let response = draw_pwm_editor_row_sized(ui, &mut percent, !control.locked, row_width);
    let raw = pwm_raw(percent);
    ui.data_mut(|data| data.insert_temp(value_id, raw));
    if response.changed {
        ui.ctx().request_repaint();
    }
    if response.should_transmit(app.live_pwm_updates) && raw != last_sent {
        send_pwm_raw(app, channel.id, raw);
        ui.data_mut(|data| data.insert_temp(sent_id, raw));
    }
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
        crate::app::EffectPresetSource::ControllerMacro(id) => Some(format!("sequence:{id}")),
        crate::app::EffectPresetSource::ControllerStrip => preset
            .effect
            .controller_strip_effect
            .as_ref()
            .map(|effect| effect.id.trim())
            .filter(|id| !id.is_empty())
            .map(|id| format!("strip:{id}")),
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
    Video,
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
    kind: TimelineTrackKind,
}

fn media_timeline_track_label(
    app: &PealayerApp,
    kind: &'static str,
    ordinal: usize,
    title: Option<&str>,
    language: Option<&str>,
) -> (String, Option<String>) {
    let title = title.map(str::trim).filter(|value| !value.is_empty());
    let language = language.map(str::trim).filter(|value| !value.is_empty());
    let name = title
        .map(|value| app.display_text(value))
        .unwrap_or_else(|| format!("{} {}", app.tr(kind), ordinal + 1));
    let detail = if title.is_some() {
        Some(match language {
            Some(value) => format!("{} · {}", app.tr(kind), app.display_text(value)),
            None => app.tr(kind),
        })
    } else {
        language.map(|value| app.display_text(value))
    };
    (name, detail)
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
        let key = "media:video".to_string();
        let state = app.timeline.track_state(&key);
        rows.push(TimelineTrackRow {
            key,
            name: app.tr("Video"),
            detail: None,
            active: true,
            enabled: true,
            linked: state.linked,
            visible: state.visible,
            icon: crate::ui::icons::FILE_VIDEO.to_string(),
            kind: TimelineTrackKind::Video,
        });
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
            kind: TimelineTrackKind::ControllerEffect(lane),
        }
    }));
    if let Some(capabilities) = capabilities.as_ref() {
        rows.extend(
            crate::ui::hardware_control::managed_controls(capabilities)
                .into_iter()
                .map(|control| {
                    let key = crate::four_d::models::hardware_timeline_track_key(&control.key);
                    let state = app.timeline.track_state(&key);
                    let relay_id = relay_id_from_control_key(&control.key);
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

fn relay_id_from_control_key(key: &str) -> Option<u8> {
    key.strip_prefix("relay.")?.parse().ok()
}

fn relay_identifier_label(relay_id: u8) -> String {
    format!("R{relay_id}")
}

fn is_pwm_control(control: &crate::four_d::controller::HardwareControl) -> bool {
    matches!(control.kind.as_str(), "mosfet" | "pwm") || control.key.starts_with("pwm.")
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
            _ => ControlIndicatorState::Unknown,
        };
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

fn is_non_user_control(
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
    let channel = capabilities
        .pwm_channels
        .iter()
        .find(|channel| channel.key == control.key)?;
    let value_id = ui.make_persistent_id(("pwm_value", channel.id));
    let local = ui.data_mut(|data| data.get_temp::<u16>(value_id));
    let raw = local.or_else(|| {
        (capabilities.telemetry.pwm_channel == Some(channel.id))
            .then_some(capabilities.telemetry.pwm_value.unwrap_or(0))
    })?;
    Some(f32::from(raw.min(4095)) / 4095.0)
}

fn control_indicator_color(control: &crate::four_d::controller::HardwareControl) -> egui::Color32 {
    crate::config::parse_rgb_hex(&control.color)
        .map(|[red, green, blue]| egui::Color32::from_rgb(red, green, blue))
        .unwrap_or_else(|| egui::Color32::from_rgb(34, 197, 94))
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

fn invoke_advertised_action(app: &PealayerApp, action_id: &str) {
    let _ = app.engine_handle.sender.send(
        crate::four_d::engine::EngineMessage::InvokeControllerAction {
            action_id: action_id.to_string(),
        },
    );
}

fn invoke_held_motion_action(app: &PealayerApp, action_id: &str) {
    let mut parts = action_id.split('.');
    if parts.next() == Some("raw-motion")
        && let (Some(side), Some(verb), None) = (parts.next(), parts.next(), parts.next())
        && matches!(side, "left" | "right")
        && matches!(verb, "up" | "down" | "stop")
    {
        let _ =
            app.engine_handle
                .sender
                .send(crate::four_d::engine::EngineMessage::ControllerCall {
                    method: "controller.command.execute".to_string(),
                    params: serde_json::json!({"command": format!("relay side {side} {verb}")}),
                });
        return;
    }
    invoke_advertised_action(app, action_id);
}

fn update_held_motion_action(
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
            if let Some((_, previous_stop)) = app.held_motion_action.take() {
                invoke_held_motion_action(app, &previous_stop);
            }
            crate::ui::hardware_control::invoke_action(app, control, action);
            app.held_motion_action = Some((action_id.to_string(), stop.id.clone()));
        }
        HoldMotionTransition::Stop => {
            if let Some((_, stop)) = app.held_motion_action.take() {
                invoke_held_motion_action(app, &stop);
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
    let key = control.key.to_ascii_lowercase();
    if control.control.eq_ignore_ascii_case("raw-motion") {
        let enable_relay = if key.contains("left") || key.ends_with(".a") {
            Some(2)
        } else if key.contains("right") || key.ends_with(".b") {
            Some(4)
        } else {
            None
        };
        return enable_relay.is_some_and(|relay| capabilities.active_relays.contains(&relay));
    }
    let side = if key.contains("left") || key.ends_with(".a") {
        Some(["left", "motion-a", "seat-a"])
    } else if key.contains("right") || key.ends_with(".b") {
        Some(["right", "motion-b", "seat-b"])
    } else {
        None
    };
    capabilities.relays.iter().any(|relay| {
        let role = relay.role.to_ascii_lowercase();
        capabilities.active_relays.contains(&relay.id)
            && side.is_none_or(|aliases| aliases.iter().any(|alias| role.contains(alias)))
    })
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
    app.hardware_control_icon_search.clear();
    app.hardware_control_color_draft = if control.color.trim().is_empty() {
        "#38D27A".to_string()
    } else {
        control.color.clone()
    };
    app.hardware_control_pwm_percent = capabilities
        .pwm_channels
        .iter()
        .find(|channel| channel.key == control.key)
        .and_then(|channel| {
            (capabilities.telemetry.pwm_channel == Some(channel.id))
                .then_some(capabilities.telemetry.pwm_value.unwrap_or(0))
        })
        .map(pwm_percent)
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
                let _ = app.engine_handle.sender.send(
                    crate::four_d::engine::EngineMessage::ControllerCall {
                        method: "controller.command.execute".to_string(),
                        params: serde_json::json!({
                            "command": format!("relay {relay_id} {}", if state { "on" } else { "off" })
                        }),
                    },
                );
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

fn should_show_stop_preview(preview_active: bool, pending_operation: Option<&str>) -> bool {
    preview_active || pending_operation == Some("effect-stop")
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

fn draw_rf_code_tool(app: &PealayerApp, ui: &mut egui::Ui) {
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
            egui::TextEdit::singleline(&mut code)
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
    let layer_id = egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new(("hardware-channel-card", control.key.as_str())),
    );
    let dragging_source = ui
        .data_mut(|data| data.get_temp::<HardwareChannelDrag>(hardware_channel_drag_id()))
        .is_some_and(|drag| drag.key == control.key);
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
                ui.label(
                    egui::RichText::new(crate::ui::icons::control(&control.kind, &control.icon))
                        .size(17.0),
                );
                let indicator = draw_control_indicator(
                    app,
                    ui,
                    indicator_state,
                    relay_id.is_some() && !control.locked,
                    control_indicator_color(control),
                    indicator_intensity,
                );
                if hardware_control_activated(app, ui, &indicator)
                    && !control.locked
                    && let Some(id) = relay_id
                {
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
                if app.prefix_relay_identifiers && let Some(id) = relay_id {
                    ui.label(
                        egui::RichText::new(relay_identifier_label(id))
                            .monospace()
                            .weak(),
                    );
                }
                if control.locked {
                    ui.label(egui::RichText::new(crate::ui::icons::LOCK).weak())
                        .on_hover_text(app.tr("Channel is locked in PCController"));
                }
				hardware_channel_drag_handle(app, ui, control);

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
                        egui::TextEdit::singleline(&mut draft)
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
                    let edit_align = crate::ui::i18n::input_alignment(app.rtl, &draft);
                    let edit_width = (ui.available_width() * 0.42).clamp(64.0, 190.0);
                    let edit = ui.add_sized(
                        [edit_width, 24.0],
                        egui::TextEdit::singleline(&mut draft)
                            .horizontal_align(edit_align)
                            .hint_text(app.tr("No group")),
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
                    let response = left_aligned_click_label(
                        ui,
                        &title,
                        (ui.available_width() - reserved).max(48.0),
                        24.0,
                        13.0,
                    );
                    if response.clicked() {
                        ui.data_mut(|data| {
                            data.insert_temp(draft_id, control.name.clone());
                            data.insert_temp(edit_id, true);
                            data.insert_temp(focus_pending_id, true);
                        });
                    }
                    response.on_hover_text(format!("{} — {}", title, app.tr("Rename")));
                    if is_motion {
                        if let Some(stop) = contextual_stop_action(capabilities, control)
                            && ui
                                .add_enabled(
                                    !app.estop_active && !control.locked,
                                    egui::Button::new(crate::ui::icons::action(&stop.verb))
                                        .min_size(egui::vec2(28.0, 26.0)),
                                )
                                .on_hover_text(crate::ui::i18n::visual_text(
                                    app.language,
                                    &stop.name,
                                ))
                                .clicked()
                        {
                            crate::ui::hardware_control::invoke_action(app, control, stop);
                        }
                        if ui
                            .button(crate::ui::icons::PENCIL_SIMPLE)
                            .on_hover_text(app.tr("Rename"))
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
                            } else if (matches!(action.verb.to_ascii_lowercase().as_str(), "on" | "off")
                                && hardware_control_activated(app, ui, &response))
                                || (!matches!(action.verb.to_ascii_lowercase().as_str(), "on" | "off")
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
        })
    }).inner;

    finish_hardware_channel_card(ui, control, card.response.rect, layer_id);
    let drop = hardware_channel_drop_target(ui, card.response.rect, control);
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
    let layer_id = egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new(("hardware-channel-card", control.key.as_str())),
    );
    let dragging_source = ui
        .data_mut(|data| data.get_temp::<HardwareChannelDrag>(hardware_channel_drag_id()))
        .is_some_and(|drag| drag.key == control.key);
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
                        ui.label(
                            egui::RichText::new(crate::ui::icons::control(
                                &control.kind,
                                &control.icon,
                            ))
                            .size(18.0),
                        );
                        let indicator = draw_control_indicator(
                            app,
                            ui,
                            indicator_state,
                            relay_id.is_some() && !control.locked,
                            control_indicator_color(control),
                            indicator_intensity,
                        );
                        if hardware_control_activated(app, ui, &indicator)
                            && !control.locked
                            && let Some(id) = relay_id
                        {
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
                        if app.prefix_relay_identifiers
                            && let Some(id) = relay_id
                        {
                            ui.label(
                                egui::RichText::new(relay_identifier_label(id))
                                    .monospace()
                                    .weak(),
                            );
                        }
                        if control.locked {
                            ui.label(egui::RichText::new(crate::ui::icons::LOCK).weak())
                                .on_hover_text(app.tr("Channel is locked in PCController"));
                        }
                        hardware_channel_drag_handle(app, ui, control);

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
                                            egui::TextEdit::singleline(&mut draft)
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
                                    if ui
                                        .button(crate::ui::icons::PENCIL_SIMPLE)
                                        .on_hover_text(app.tr("Rename"))
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
                                        && ui
                                            .add_enabled(
                                                !app.estop_active && !control.locked,
                                                egui::Button::new(crate::ui::icons::action(
                                                    &stop.verb,
                                                )),
                                            )
                                            .on_hover_text(crate::ui::i18n::visual_text(
                                                app.language,
                                                &stop.name,
                                            ))
                                            .clicked()
                                    {
                                        crate::ui::hardware_control::invoke_action(
                                            app, control, stop,
                                        );
                                    }
                                    let title = left_aligned_click_label(
                                        ui,
                                        &title_text,
                                        ui.available_width().max(52.0),
                                        24.0,
                                        13.0,
                                    );
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
                        let edit_align = crate::ui::i18n::input_alignment(app.rtl, &draft);
                        ui.horizontal(|ui| {
                            ui.label(app.tr("Group"));
                            let edit = ui.add_sized(
                                [ui.available_width().max(80.0) - 52.0, 24.0],
                                egui::TextEdit::singleline(&mut draft)
                                    .horizontal_align(edit_align)
                                    .hint_text(app.tr("No group")),
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
                                        } else if (matches!(verb.as_str(), "on" | "off")
                                            && hardware_control_activated(app, ui, &response))
                                            || (!matches!(verb.as_str(), "on" | "off")
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
                })
        })
        .inner;

    finish_hardware_channel_card(ui, control, card.response.rect, layer_id);
    let drop = hardware_channel_drop_target(ui, card.response.rect, control);
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
                        let handle = hardware_channel_drag_handle(app, ui, control);
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
                        ) {
                            pending_drop = Some(drop);
                        }
                        if response.clicked() {
                            let _ = app.engine_handle.sender.send(
                                crate::four_d::engine::EngineMessage::ControllerCall {
                                    method: "controller.command.execute".to_string(),
                                    params: serde_json::json!({
                                        "command": format!(
                                            "relay {relay_id} {}",
                                            if active { "off" } else { "on" }
                                        )
                                    }),
                                },
                            );
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
                                let _ = app.engine_handle.sender.send(
                                    crate::four_d::engine::EngineMessage::ControllerCall {
                                        method: "controller.command.execute".to_string(),
                                        params: serde_json::json!({
                                            "command": format!(
                                                "relay {relay_id} {}",
                                                if active { "off" } else { "on" }
                                            )
                                        }),
                                    },
                                );
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
    } else if ui.input(|input| input.pointer.any_released()) {
        ui.data_mut(|data| {
            data.remove_temp::<HardwareChannelDrag>(hardware_channel_drag_id());
        });
    }
}

#[cfg(test)]
mod timeline_row_tests {
    use super::*;

    fn discard_ui_output(mut output: egui::FullOutput) {
        output.textures_delta.clear();
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
                kind: TimelineTrackKind::Video,
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
        app.current_aid = "7".to_string();
        app.current_sid = "12".to_string();
        app.sub_visibility = true;
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
        assert_eq!(rows[0].kind, TimelineTrackKind::Video);
        assert_eq!(rows[1].kind, TimelineTrackKind::Audio(7));
        assert_eq!(rows[2].kind, TimelineTrackKind::Audio(8));
        assert_eq!(rows[3].kind, TimelineTrackKind::Subtitle(11));
        assert_eq!(rows[4].kind, TimelineTrackKind::Subtitle(12));
        assert!(rows[1].active);
        assert!(!rows[2].active);
        assert!(!rows[3].active);
        assert!(rows[4].active && rows[4].enabled);
        assert_eq!(rows[1].detail.as_deref(), Some("Audio · en"));
        assert_eq!(rows[4].detail.as_deref(), Some("Subtitles · fa"));
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
        }));
        assert!(rows.iter().any(|row| {
            row.key == "hardware:relay.5" && row.kind == TimelineTrackKind::Relay(5)
        }));
        assert!(rows.iter().any(|row| {
            row.key == "hardware:pwm.12"
                && row.kind == TimelineTrackKind::Hardware("pwm.12".to_string())
        }));

        app.timeline.set_track_visible("hardware:pwm.12", false);
        app.timeline.set_track_linked("hardware:relay.5", false);
        let visible_rows = timeline_track_rows(&app);
        assert!(!visible_rows.iter().any(|row| row.key == "hardware:pwm.12"));
        assert!(!visible_rows.iter().any(|row| row.key == "hardware:relay.5"));
        assert!(visible_rows.iter().any(|row| row.key == "hardware:seat.a"));
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
    fn stop_preview_is_only_visible_for_an_active_or_stopping_preview() {
        assert!(!should_show_stop_preview(false, None));
        assert!(!should_show_stop_preview(false, Some("effect-preview")));
        assert!(should_show_stop_preview(true, None));
        assert!(should_show_stop_preview(false, Some("effect-stop")));
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
        };
        assert!(changing.should_transmit(true));
        assert!(!changing.should_transmit(false));

        let committed = PwmEditorResponse {
            changed: false,
            committed: true,
        };
        assert!(!committed.should_transmit(true));
        assert!(committed.should_transmit(false));
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
    fn seat_stop_visibility_ignores_other_sides_and_unrelated_relays() {
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
        assert!(motion_control_is_active(&capabilities, &left));
    }

    #[test]
    fn seat_stop_is_inline_and_only_present_while_that_seat_is_active() {
        let mut capabilities = crate::four_d::controller::HardwareCapabilities::default();
        let control = crate::four_d::controller::HardwareControl {
            key: "seat.a".into(),
            kind: "motion".into(),
            control: "raw-motion".into(),
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

        capabilities.active_relays.insert(2);
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
    fn production_effect_action_row_stays_within_the_card_width() {
        let spacing = 8.0;
        for available in [112.0, 180.0, 297.0] {
            let (title, actions) = effect_card_header_widths(available, spacing);
            assert!(title + spacing + actions <= available + f32::EPSILON);
            assert_eq!(actions, 88.0);
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
                                    for label in ["more", "add", "play"] {
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

        assert_eq!(effect_controls_kind(&relay).1, "Relay sequence");
        assert_eq!(effect_controls_kind(&macro_effect).1, "Hardware macro");
        assert_eq!(effect_controls_kind(&strip).1, "Addressable lighting");
    }

    #[test]
    fn effect_card_action_click_survives_the_drag_surface() {
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
                        110.0,
                        |ui| {
                            effect_card(ui, 260.0, |ui| {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let button = ui.button("More");
                                        action_rect.set(button.rect);
                                        button_response = Some(button);
                                    },
                                );
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
            Some("sequence:7")
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
            ControlIndicatorState::Inactive
        );
        assert_eq!(
            control_indicator_state(&capabilities, &control("pwm.1", "mosfet")),
            ControlIndicatorState::Unknown
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
                                    let elapsed_response = crate::ui::controls::draw_elapsed_editor(
                                        self.app,
                                        ui,
                                        "nle-elapsed-editor",
                                        can_seek,
                                    );
                                    elapsed_response.context_menu(|ui| {
                                        crate::ui::controls::transport_context_menu(self.app, ui)
                                    });

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
                                                let seekbar_width =
                                                    ui.available_width().max(1.0);
                                                let old_width = ui.spacing().slider_width;
                                                ui.spacing_mut().slider_width = seekbar_width;
                                                let response = ui.add_enabled(can_seek, slider);
                                                ui.spacing_mut().slider_width = old_width;
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
                        ui.set_width(panel_width);

                        if selected_count == 1 {
                            let id = *self.app.selected_instance_ids.iter().next().unwrap();
                            let mut timeline_dirty = false;
                            let mut delete_cue = false;
                            let mut jump_to_cue = false;
                            let mut relocate_effect_id = None;

                            ui.horizontal(|ui| {
                                ui.heading(self.app.tr("Effect Controls"));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    effect_controls_badge(
                                        ui,
                                        crate::ui::icons::SELECTION_ALL,
                                        self.app.tr("1 cue"),
                                    );
                                });
                            });
                            ui.add_space(6.0);

                            let instance_idx = self
                                .app
                                .timeline
                                .instances
                                .iter()
                                .position(|instance| instance.id == id);

                            let mut push_undo = false;
                            let mut isolate_instance = false;
                            let mut update_start_to = None;
                            let mut update_duration_to = None;
                            let mut update_relay_to = None;

                            if let Some(idx) = instance_idx {
                                let selected_cue_label = self.app.tr("Selected cue");
                                let name_label = self.app.tr("Name");
                                let timing_label = self.app.tr("Timing");
                                let timing_subtitle = self.app.tr("Exact timeline placement and length");
                                let start_time_label = self.app.tr("Starts");
                                let duration_label = self.app.tr("Duration");
                                let hardware_target_label = self.app.tr("Hardware target");
                                let source_label = self.app.tr("Source");
                                let unavailable_output_label = self.app.tr("Unavailable output");
                                let unavailable_project_output_label =
                                    self.app.tr("Unavailable project output");
                                let connect_output_label = self.app.tr("No live hardware outputs");
                                let target_output_label = self.app.tr("Output");
                                let delete_cue_label = self.app.tr("Delete Cue");
                                let jump_label = self.app.tr("Go to cue");
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

                                    effect_controls_card(
                                        ui,
                                        panel_width,
                                        kind_icon,
                                        &summary_title,
                                        Some(&selected_cue_label),
                                        true,
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
                                                    crate::duration::format_effect_duration_for_language(
                                                        display_language,
                                                        template.duration_ms,
                                                    ),
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
                                                egui::TextEdit::singleline(&mut template.name)
                                                    .horizontal_align(name_align),
                                            );
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
                                            let mut start_secs = start_ms as f64 / 1_000.0;
                                            let slider = ui.add_sized(
                                                [ui.available_width(), 18.0],
                                                egui::Slider::new(&mut start_secs, 0.0..=max_secs)
                                                    .show_value(false),
                                            );
                                            if slider.drag_started()
                                                || (slider.changed() && !slider.dragged())
                                            {
                                                push_undo = true;
                                            }
                                            if slider.changed() {
                                                update_start_to = Some((start_secs * 1000.0) as u64);
                                                timeline_dirty = true;
                                            }

                                            ui.add_space(5.0);
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(&duration_label).weak());
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        let mut duration_ms = template.duration_ms;
                                                        let editor = ui.add(
                                                            crate::duration::time_value_drag(
                                                                &mut duration_ms,
                                                                50..=3_600_000,
                                                                50.0,
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
                                                    },
                                                );
                                            });
                                        },
                                    );

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
                                    let details_open = crate::ui::icons::disclosure_header(
                                        ui,
                                        ("effect-control-details", id),
                                        &format!("{}  {details_label}", crate::ui::icons::INFO),
                                        false,
                                    );
                                    if details_open {
                                        egui::Grid::new(("effect-control-identifiers", id))
                                            .num_columns(2)
                                            .spacing([8.0, 4.0])
                                            .show(ui, |ui| {
                                                ui.label(egui::RichText::new("Cue ID").small().weak());
                                                ui.add(egui::Label::new(egui::RichText::new(id.to_string()).monospace().small()).selectable(true));
                                                ui.end_row();
                                                ui.label(egui::RichText::new("Effect ID").small().weak());
                                                ui.add(egui::Label::new(egui::RichText::new(template.id.to_string()).monospace().small()).selectable(true));
                                                ui.end_row();
                                            });
                                    }

                                    ui.add_space(8.0);
                                    ui.horizontal(|ui| {
                                        if ui
                                            .button(format!("{}  {jump_label}", crate::ui::icons::SKIP_BACK))
                                            .clicked()
                                        {
                                            jump_to_cue = true;
                                        }
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button(
                                                        egui::RichText::new(format!(
                                                            "{}  {delete_cue_label}",
                                                            crate::ui::icons::TRASH
                                                        ))
                                                        .color(egui::Color32::from_rgb(220, 74, 74)),
                                                    )
                                                    .clicked()
                                                {
                                                    delete_cue = true;
                                                }
                                            },
                                        );
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
                                self.app.sync_timeline_engine();
                                ui.ctx().request_repaint();
                            }
                        } else if selected_count > 1 {
                            ui.horizontal(|ui| {
                                ui.heading(self.app.tr("Effect Controls"));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    effect_controls_badge(
                                        ui,
                                        crate::ui::icons::SELECTION_ALL,
                                        format!("{selected_count} {}", self.app.tr("cues")),
                                    );
                                });
                            });
                            ui.add_space(6.0);

                            let mut timeline_dirty = false;
                            let mut delete_all = false;
                            let mut bulk_relay = None;

                            effect_controls_card(
                                ui,
                                panel_width,
                                crate::ui::icons::SELECTION_ALL,
                                &self.app.tr("Multiple cues selected"),
                                Some(&self.app.tr("Changes apply to every compatible cue")),
                                true,
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
                                self.app.sync_timeline_engine();
                            }
                        } else {
                            ui.heading(self.app.tr("Effect Controls"));
                            ui.add_space(6.0);
                            effect_controls_card(
                                ui,
                                panel_width,
                                crate::ui::icons::SELECTION_ALL,
                                &self.app.tr("No cue selected"),
                                Some(&self.app.tr("Select a cue on the timeline to manage it")),
                                false,
                                |_| {},
                            );
                        }
                    }
                    PealayerTab::EffectsLibrary => {
                        ui.horizontal(|ui| {
                            ui.heading(self.app.tr("Effects Library"));
                            if ui
                                .button(format!(
                                    "{} {}",
                                    crate::ui::icons::PLUS,
                                    self.app.tr("New effect")
                                ))
                                .clicked()
                            {
                                crate::ui::effects_library::begin_new_effect(self.app, None);
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
                            }
                        });
                        ui.add_space(4.0);

                        // 1. Instant search edit field
                        ui.horizontal(|ui| {
                            ui.label(self.app.tr("Search"));
                            let search_hint = self.app.tr("Search effects...");
                            let search_align = crate::ui::i18n::input_alignment(
                                self.app.rtl,
                                &self.app.effects_search_query,
                            );
                            let res = ui.add(
                                egui::TextEdit::singleline(&mut self.app.effects_search_query)
                                    .horizontal_align(search_align)
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
                                ui.label(
                                    egui::RichText::new(self.app.tr("No effects"))
                                    .weak()
                                    .size(12.0),
                                );
                            });
                        } else {
                            egui::ScrollArea::vertical()
                                .id_salt("effects_scroll")
                                .show(ui, |ui| {
                                    // Keep a deliberate gutter between cards and the scrollbar /
                                    // right panel edge. The previous full-width inner frame caused
                                    // its stroke and action row to crowd or clip against that edge.
                                    let effects_width =
                                        effects_panel_content_width(ui.available_width());
                                    ui.set_width(effects_width);
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
                                        let group_icon_name = presets
                                            .iter()
                                            .map(|preset| preset.group_icon.trim())
                                            .find(|icon| !icon.is_empty())
                                            .unwrap_or_default()
                                            .to_string();
                                        let group_icon = crate::ui::icons::named_control_icon(
                                            &group_icon_name,
                                        )
                                        .unwrap_or(crate::ui::icons::FOLDER_OPEN);
                                        let group_header = effect_group_header(
                                            ui,
                                            effects_width,
                                            |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(if open {
                                                        crate::ui::icons::CARET_DOWN
                                                    } else {
                                                        crate::ui::icons::CARET_RIGHT
                                                    });
                                                    ui.label(group_icon);
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
                                            },
                                        );
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
                                                    crate::ui::icons::SPARKLE,
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
                                                let mut begin_inline_edit = false;
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
                                                                    let selected_icon = crate::ui::icons::named_control_icon(
                                                                        &edit.icon,
                                                                    )
                                                                    .unwrap_or(crate::ui::icons::SPARKLE);
                                                                    egui::ComboBox::from_id_salt(
                                                                        item_id.with("inline-icon"),
                                                                    )
                                                                    .width(38.0)
                                                                    .selected_text(selected_icon)
                                                                    .show_ui(ui, |ui| {
                                                                        for (key, label, glyph) in
                                                                            crate::ui::icons::CONTROL_ICON_PRESETS
                                                                        {
                                                                            if ui
                                                                                .selectable_label(
                                                                                    edit.icon.eq_ignore_ascii_case(key),
                                                                                    format!("{glyph}  {label}"),
                                                                                )
                                                                                .clicked()
                                                                            {
                                                                                edit.icon = (*key).to_string();
                                                                            }
                                                                        }
                                                                    });
                                                                    let action_spacing = ui.spacing().item_spacing.x;
                                                                    let name_width = (ui.available_width()
                                                                        - 48.0
                                                                        - action_spacing * 2.0)
                                                                        .max(54.0);
                                                                    let name_align = crate::ui::i18n::input_alignment(
                                                                        self.app.rtl,
                                                                        &edit.name,
                                                                    );
                                                                    let name_response = ui.add_sized(
                                                                        [name_width, 24.0],
                                                                        egui::TextEdit::singleline(&mut edit.name)
                                                                            .horizontal_align(name_align)
                                                                            .char_limit(64),
                                                                    );
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
                                                                ui.horizontal(|ui| {
                                                                    let icon_response = ui.add_sized(
                                                                        [20.0, 24.0],
                                                                        egui::Button::new(
                                                                        egui::RichText::new(&preset.effect.icon)
                                                                        .size(16.0),
                                                                        )
                                                                        .frame(false),
                                                                    )
                                                                    .on_hover_text(self.app.tr("Change icon"));
                                                                    begin_inline_edit |= icon_response.clicked();
                                                                    let action_spacing = ui.spacing().item_spacing.x;
                                                                    let (title_width, reserved_actions) =
                                                                        effect_card_header_widths(
                                                                            ui.available_width(),
                                                                            action_spacing,
                                                                        );
                                                                    let title_response = ui
                                                                        .allocate_ui_with_layout(
                                                                            egui::vec2(title_width, 24.0),
                                                                            egui::Layout::left_to_right(
                                                                                egui::Align::Center,
                                                                            ),
                                                                            |ui| {
                                                                                ui.add(
                                                                                    egui::Label::new(
                                                                                        egui::RichText::new(
                                                                                            &displayed_effect_name,
                                                                                        )
                                                                                        .strong(),
                                                                                    )
                                                                                    .truncate()
                                                                                    .sense(egui::Sense::click()),
                                                                                )
                                                                            },
                                                                        )
                                                                        .inner
                                                                        .on_hover_text(self.app.tr("Rename"));
                                                                    begin_inline_edit |= title_response.clicked();
                                                                    ui.allocate_ui_with_layout(
                                                                        egui::vec2(reserved_actions, 24.0),
                                                                        egui::Layout::right_to_left(egui::Align::Center),
                                                                        |ui| {
                                                                            let more = ui
                                                                                .add_sized(
                                                                                    [24.0, 24.0],
                                                                                    egui::Button::new(
                                                                                        crate::ui::icons::DOTS_THREE,
                                                                                    )
                                                                                    .frame(false),
                                                                                )
                                                                                .on_hover_text(
                                                                                    self.app.tr("More actions"),
                                                                                );
                                                                            more_response = Some(more);
                                                                            let place = ui
                                                                                .add_sized(
                                                                                    [24.0, 24.0],
                                                                                    egui::Button::new(
                                                                                        crate::ui::icons::PLUS,
                                                                                    )
                                                                                    .frame(false),
                                                                                )
                                                                                .on_hover_text(
                                                                                    self.app.tr("Place at playhead"),
                                                                                );
                                                                            place_at_playhead = place.clicked();
                                                                            let run = ui
                                                                                .add_sized(
                                                                                    [24.0, 24.0],
                                                                                    egui::Button::new(
                                                                                        crate::ui::icons::PLAY,
                                                                                    )
                                                                                    .frame(false),
                                                                                )
                                                                                .on_hover_text(self.app.tr("Run now"));
                                                                            run_now = run.clicked();
                                                                        },
                                                                    );
                                                                });
                                                            }
                                                                ui.add_space(5.0);
                                                                ui.horizontal_wrapped(|ui| {
                                                                    for text in [
                                                                        target_label.clone(),
                                                                        crate::duration::format_effect_duration_for_language(
                                                                            display_language,
                                                                            preset.effect.duration_ms,
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
                                                        .play_controller_effect(&reference)
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
                                                    if ui.button(format!("{} {}", crate::ui::icons::PLAY, self.app.tr("Run now"))).clicked() {
                                                        if let Err(error) = self.app.play_controller_effect(&reference) {
                                                            self.app.set_osd(error);
                                                        }
                                                        ui.close();
                                                    }
                                                    if ui.button(format!("{} {}", crate::ui::icons::STOP_CIRCLE, self.app.tr("Stop"))).clicked() {
                                                        if let Err(error) = self.app.stop_controller_effect(&reference) {
                                                            self.app.set_osd(error);
                                                        }
                                                        ui.close();
                                                    }
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

                            let mut keep_group_editor_open = self.app.effect_group_draft.is_some();
                            let mut save_group = false;
                            let mut cancel_group = false;
                            let group_editor_title = format!(
                                "{} {}",
                                crate::ui::icons::FOLDER_OPEN,
                                self.app.tr("Manage effect group")
                            );
                            let group_name_label = self.app.tr("Name");
                            let group_icon_label = self.app.tr("Icon");
                            let presets_label = self.app.tr("Presets");
                            let save_group_label = self.app.tr("Save to PCController");
                            let cancel_group_label = self.app.tr("Cancel");
                            let rtl_ui = self.app.rtl;
                            if let Some(draft) = self.app.effect_group_draft.as_mut() {
                                egui::Window::new(group_editor_title)
                                .id(egui::Id::new("effect_group_editor"))
                                .open(&mut keep_group_editor_open)
                                .resizable(false)
                                .collapsible(false)
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
                                            ui.add(
                                                egui::TextEdit::singleline(&mut draft.name)
                                                    .horizontal_align(name_align)
                                                    .desired_width(250.0),
                                            );
                                            ui.end_row();
                                            ui.label(&group_icon_label);
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    crate::ui::icons::named_control_icon(&draft.icon)
                                                        .unwrap_or(
                                                            crate::ui::icons::FOLDER_OPEN,
                                                        ),
                                                );
                                                ui.add(
                                                    egui::TextEdit::singleline(&mut draft.icon)
                                                        .desired_width(130.0)
                                                        .hint_text("folder"),
                                                );
                                                egui::ComboBox::from_id_salt(
                                                    "effect_group_icon_preset",
                                                )
                                                .selected_text(&presets_label)
                                                .show_ui(ui, |ui| {
                                                    for (key, label, glyph) in
                                                        crate::ui::icons::CONTROL_ICON_PRESETS
                                                    {
                                                        if ui
                                                            .selectable_label(
                                                                draft
                                                                    .icon
                                                                    .eq_ignore_ascii_case(key),
                                                                format!("{glyph}  {label}"),
                                                            )
                                                            .clicked()
                                                        {
                                                            draft.icon = (*key).to_string();
                                                        }
                                                    }
                                                });
                                            });
                                            ui.end_row();
                                        });
                                    ui.add_space(10.0);
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add_enabled(
                                                !draft.name.trim().is_empty(),
                                                egui::Button::new(format!(
                                                    "{} {}",
                                                    crate::ui::icons::FLOPPY_DISK,
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
                            }
                        }
                    }
                    PealayerTab::HardwareMonitor => {
                        ui.horizontal(|ui| {
                            ui.heading(self.app.tr("Hardware Monitor Dashboard"));
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
                        });
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

                            let can_record = capabilities.board_profile.as_ref().is_some_and(|profile| {
                                profile.attached && profile.configured
                            }) && !capabilities.controls.is_empty();
                            if can_record {
                                let record_title = self.app.tr("Record effect");
                                let name_label = self.app.tr("Name");
                                let name_hint = self.app.tr("Seat motion take");
                                let start_label = self.app.tr("Start recording");
                                let start_help = self.app.tr("Record from the current media time");
                                let refresh_label = self.app.tr("Refresh status");
                                let save_label = self.app.tr("Save and place");
                                let discard_label = self.app.tr("Discard");
                                let anchor_label = self.app.tr("Timeline anchor:");
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
                                                let name_align = crate::ui::i18n::input_alignment(
                                                    self.app.rtl,
                                                    &self.app.hardware_effect_authoring.name,
                                                );
                                                ui.add_enabled(
                                                    !self.app.hardware_effect_authoring.active
                                                        && self.app.hardware_effect_authoring.pending_operation.is_none(),
                                                    egui::TextEdit::singleline(&mut self.app.hardware_effect_authoring.name)
                                                        .horizontal_align(name_align)
                                                        .desired_width(ui.available_width().min(280.0))
                                                        .hint_text(&name_hint),
                                                );
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

                            let motion_controls = capabilities
                                .controls
                                .iter()
                                .filter(|control| is_motion_control(control))
                                .cloned()
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

                        // mpv's render callback requests frames while video is
                        // advancing. An unconditional repaint here turned the
                        // no-media state (which starts unpaused) into an
                        // unlimited GPU render loop when V-Sync was disabled.
                    }
                    PealayerTab::Timeline => {
                        let all_timeline_rows = all_timeline_track_rows(self.app);
                        let timeline_rows = all_timeline_rows
                            .iter()
                            .filter(|row| {
                                row.linked
                                    && row.visible
                                    && !hardware_row_has_analog_track(self.app, row)
                            })
                            .cloned()
                            .collect::<Vec<_>>();
                        let linked_analog_track_ids = self
                            .app
                            .timeline
                            .analog_tracks
                            .iter()
                            .filter(|track| {
                                self.app
                                    .timeline
                                    .track_state(&crate::four_d::models::hardware_timeline_track_key(
                                        &format!("pwm.{}", track.channel),
                                    ))
                                    .linked
                            })
                            .map(|track| track.id)
                            .collect::<std::collections::BTreeSet<_>>();
                        let visible_analog_track_ids = self
                            .app
                            .timeline
                            .analog_tracks
                            .iter()
                            .filter(|track| {
                                let state = self.app.timeline.track_state(
                                    &crate::four_d::models::hardware_timeline_track_key(&format!(
                                        "pwm.{}",
                                        track.channel
                                    )),
                                );
                                state.linked && state.visible
                            })
                            .map(|track| track.id)
                            .collect::<std::collections::BTreeSet<_>>();
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
                                let mut header_ui = ui.new_child(
                                    egui::UiBuilder::new()
                                        .max_rect(header_rect.shrink2(egui::vec2(4.0, 1.0)))
                                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                                );
                                if header_ui
                                    .button(crate::ui::icons::DIAMOND)
                                    .on_hover_text(self.app.tr("Add exact timeline keyframe at playhead (K)"))
                                    .clicked()
                                {
                                    let time_ms = (self.app.playback_time * 1_000.0)
                                        .round()
                                        .max(0.0) as u64;
                                    if !self.app.timeline.keyframes.iter().any(|keyframe| keyframe.time_ms == time_ms) {
                                        self.app.undo_stack.push(self.app.snapshot_timeline());
                                    }
                                    self.app.selected_timeline_keyframe =
                                        Some(self.app.timeline.add_keyframe(time_ms));
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
                                        ui.menu_button(
                                            format!("{}  {}", row.icon, row.name),
                                            |ui| {
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
                                            },
                                        );
                                    }
                                });

                                for track_row in &timeline_rows {
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(250.0, 32.0),
                                        egui::Sense::click(),
                                    );
                                    let row_fill = if track_row.active {
                                        ui.visuals().selection.bg_fill.gamma_multiply(
                                            if ui.visuals().dark_mode { 0.16 } else { 0.08 },
                                        )
                                    } else {
                                        ui.visuals().faint_bg_color
                                    };
                                    ui.painter().rect_filled(rect, 0.0, row_fill);
                                    ui.painter().rect_stroke(rect, 0.0, ui.visuals().widgets.noninteractive.bg_stroke, egui::StrokeKind::Inside);

                                    // Create a nested UI at this rect to place buttons
                                    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(*ui.layout()));
                                    let mut media_control_clicked = false;
                                    child_ui.horizontal(|ui| {
                                        ui.add_space(6.0);
                                        let icon_color = if track_row.active && track_row.enabled {
                                            ui.visuals().selection.bg_fill
                                        } else {
                                            ui.visuals().weak_text_color()
                                        };
                                        ui.label(egui::RichText::new(&track_row.icon).color(icon_color));
                                        let label_width = if matches!(
                                            track_row.kind,
                                            TimelineTrackKind::Relay(_)
                                                | TimelineTrackKind::Hardware(_)
                                        ) {
                                            78.0
                                        } else {
                                            158.0
                                        };
                                        ui.allocate_ui(egui::vec2(label_width, 28.0), |ui| {
                                            let title = egui::RichText::new(&track_row.name)
                                                .size(11.0)
                                                .strong();
                                            if let Some(detail) = track_row.detail.as_deref() {
                                                ui.spacing_mut().item_spacing.y = 0.0;
                                                ui.vertical(|ui| {
                                                    ui.add(egui::Label::new(title).truncate());
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new(detail).size(9.0).weak(),
                                                        )
                                                        .truncate(),
                                                    );
                                                });
                                            } else {
                                                ui.with_layout(
                                                    egui::Layout::left_to_right(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        ui.add(
                                                            egui::Label::new(title).truncate(),
                                                        );
                                                    },
                                                );
                                            }
                                        });

                                        if let TimelineTrackKind::Subtitle(_) = track_row.kind {
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
                                                    self.app.sub_visibility = !visible;
                                                    let _ = self
                                                        .app
                                                        .mpv
                                                        .set_property("sub-visibility", !visible);
                                                } else if let TimelineTrackKind::Subtitle(track_id) =
                                                    track_row.kind
                                                {
                                                    let track_id = track_id.to_string();
                                                    self.app.current_sid = track_id.clone();
                                                    self.app.sub_visibility = true;
                                                    let _ = self
                                                        .app
                                                        .mpv
                                                        .set_property("sid", track_id);
                                                    let _ = self
                                                        .app
                                                        .mpv
                                                        .set_property("sub-visibility", true);
                                                }
                                            }
                                        } else if track_row.active
                                            && matches!(track_row.kind, TimelineTrackKind::Audio(_))
                                        {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(34, 197, 94),
                                                crate::ui::icons::DOT_OUTLINE,
                                            )
                                            .on_hover_text(self.app.tr("Active track"));
                                        } else if let TimelineTrackKind::Relay(relay_id) = track_row.kind {
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
                                    if response.clicked() && !media_control_clicked {
                                        match track_row.kind {
                                            TimelineTrackKind::Audio(track_id) => {
                                                let track_id = track_id.to_string();
                                                self.app.current_aid = track_id.clone();
                                                let _ = self.app.mpv.set_property("aid", track_id);
                                            }
                                            TimelineTrackKind::Subtitle(track_id) => {
                                                let track_id = track_id.to_string();
                                                self.app.current_sid = track_id.clone();
                                                self.app.sub_visibility = true;
                                                let _ = self.app.mpv.set_property("sid", track_id);
                                                let _ = self
                                                    .app
                                                    .mpv
                                                    .set_property("sub-visibility", true);
                                            }
                                            _ => {}
                                        }
                                    }
                                    response.context_menu(|ui| {
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
                                let hide_timeline_track_label =
                                    self.app.tr("Hide timeline track");
                                let unlink_timeline_track_label =
                                    self.app.tr("Unlink from timeline");
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
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(250.0, 40.0),
                                        egui::Sense::click(),
                                    );
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
                                    response.context_menu(|ui| {
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
                                if analog_tracks_changed {
                                    let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.linked_analog_tracks()));
                                }
                            });

                            // 2. Right column: Scrollable Timeline Grid
                            let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
                            let zoom = self.app.timeline_zoom;
                            let px_per_ms = zoom / 1000.0;
                            let total_seconds = if self.app.duration > 0.0 { self.app.duration } else { 60.0 };
                            let total_width = (total_seconds * zoom as f64) as f32;
                            let num_analog = visible_analog_track_ids.len();
                            let track_area_height = timeline_rows.len() as f32 * 32.0;
                            let total_height = 26.0 + track_area_height + (num_analog as f32 * 40.0);

                            // The interactive timeline canvas itself is the drop
                            // target. Wrapping it in `dnd_drop_zone` made the outer
                            // response fail `contains_pointer`: the inner
                            // click-and-drag canvas correctly owned the pointer and
                            // occluded its parent, so releases were never accepted.
                            let timeline_scroll = egui::ScrollArea::both()
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

                                        ruler_response.context_menu(|ui| {
                                            ui.label(egui::RichText::new(self.app.tr("Timeline keyframe")).strong());
                                            let playhead_ms = (self.app.playback_time * 1_000.0)
                                                .round()
                                                .clamp(0.0, total_seconds * 1_000.0)
                                                as u64;
                                            if ui
                                                .button(format!(
                                                    "{} {}",
                                                    crate::ui::icons::DIAMOND,
                                                    self.app.tr("Add keyframe at playhead")
                                                ))
                                                .clicked()
                                            {
                                                if !self.app.timeline.keyframes.iter().any(|keyframe| keyframe.time_ms == playhead_ms) {
                                                    self.app.undo_stack.push(self.app.snapshot_timeline());
                                                }
                                                self.app.selected_timeline_keyframe =
                                                    Some(self.app.timeline.add_keyframe(playhead_ms));
                                                ui.close();
                                            }
                                            if let Some(position) = pointer_pos {
                                                let pointer_ms = (((position.x - rect.min.x).max(0.0) / px_per_ms)
                                                    .round() as u64)
                                                    .min((total_seconds * 1_000.0).round() as u64);
                                                if ui
                                                    .button(format!(
                                                        "{} {} ({})",
                                                        crate::ui::icons::PUSH_PIN,
                                                        self.app.tr("Add exact keyframe here"),
                                                        crate::duration::format_time_value_ms(pointer_ms)
                                                    ))
                                                    .clicked()
                                                {
                                                    if !self.app.timeline.keyframes.iter().any(|keyframe| keyframe.time_ms == pointer_ms) {
                                                        self.app.undo_stack.push(self.app.snapshot_timeline());
                                                    }
                                                    self.app.selected_timeline_keyframe =
                                                        Some(self.app.timeline.add_keyframe(pointer_ms));
                                                    ui.close();
                                                }
                                            }
                                        });

                                        let mut clicked_any_keyframe = false;
                                        for marker in self.app.timeline.keyframes.clone() {
                                            let marker_x = rect.min.x + marker.time_ms as f32 * px_per_ms;
                                            if marker_x < rect.min.x || marker_x > rect.max.x {
                                                continue;
                                            }
                                            let selected = self.app.selected_timeline_keyframe == Some(marker.id);
                                            let color = if selected {
                                                ui.visuals().selection.stroke.color
                                            } else {
                                                ui.visuals().hyperlink_color
                                            };
                                            painter.line_segment(
                                                [
                                                    egui::pos2(marker_x, ruler_rect.center().y),
                                                    egui::pos2(marker_x, rect.max.y),
                                                ],
                                                egui::Stroke::new(if selected { 1.6_f32 } else { 1.0_f32 }, color.gamma_multiply(0.7)),
                                            );
                                            let marker_rect = egui::Rect::from_center_size(
                                                egui::pos2(marker_x, ruler_rect.center().y),
                                                egui::vec2(20.0, 22.0),
                                            );
                                            let marker_response = ui
                                                .interact(marker_rect, egui::Id::new(("timeline-keyframe", marker.id)), egui::Sense::click())
                                                .on_hover_text(format!(
                                                    "{} · {}",
                                                    self.app.tr("Exact timeline keyframe"),
                                                    crate::duration::format_time_value_ms(marker.time_ms)
                                                ));
                                            painter.text(
                                                marker_rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                crate::ui::icons::DIAMOND,
                                                egui::FontId::proportional(13.0),
                                                color,
                                            );
                                            if marker_response.clicked() {
                                                clicked_any_keyframe = true;
                                                self.app.selected_timeline_keyframe = Some(marker.id);
                                                self.app.selected_instance_ids.clear();
                                                self.app.selected_keyframes.clear();
                                            }
                                            if marker_response.double_clicked() {
                                                self.app.seek_absolute(marker.time_ms as f64 / 1_000.0);
                                            }
                                            marker_response.context_menu(|ui| {
                                                ui.label(egui::RichText::new(self.app.tr("Exact timeline keyframe")).strong());
                                                let mut exact_time = marker.time_ms;
                                                ui.horizontal(|ui| {
                                                    ui.label(self.app.tr("Time"));
                                                    if ui
                                                        .add(crate::duration::time_value_drag(
                                                            &mut exact_time,
                                                            0..=(total_seconds * 1_000.0).round() as u64,
                                                            1.0,
                                                        ))
                                                        .changed()
                                                        && exact_time != marker.time_ms
                                                    {
                                                        self.app.undo_stack.push(self.app.snapshot_timeline());
                                                        let _ = self.app.timeline.move_keyframe(marker.id, exact_time);
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
                                                    self.app.timeline.remove_keyframe(marker.id);
                                                    self.app.selected_timeline_keyframe = None;
                                                    ui.close();
                                                }
                                            });
                                        }

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
                                        for (t_idx, _) in self
                                            .app
                                            .timeline
                                            .analog_tracks
                                            .iter()
                                            .filter(|track| visible_analog_track_ids.contains(&track.id))
                                            .enumerate()
                                        {
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
                                                let (icon, color) = match track_row.kind {
                                                    TimelineTrackKind::Video => (crate::ui::icons::FILE_VIDEO, egui::Color32::from_rgb(41, 128, 185)),
                                                    TimelineTrackKind::Audio(_) => (crate::ui::icons::SPEAKER_HIGH, egui::Color32::from_rgb(39, 174, 96)),
                                                    TimelineTrackKind::Subtitle(_) => (crate::ui::icons::SUBTITLES, egui::Color32::from_rgb(124, 92, 190)),
                                                    TimelineTrackKind::ControllerEffect(_)
                                                    | TimelineTrackKind::Relay(_)
                                                    | TimelineTrackKind::Hardware(_) => continue,
                                                };
                                                let color = color.gamma_multiply(
                                                    if track_row.active && track_row.enabled { 1.0 } else { 0.52 },
                                                );
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
                                                let is_controller_owned = effect.controller_macro.is_some()
                                                    || effect.controller_strip_effect.is_some();
                                                let (track_index, relay_id) = if is_controller_owned {
                                                    let lane = effect
                                                        .controller_lane
                                                        .unwrap_or(crate::four_d::models::ControllerEffectLane::Sequence);
                                                    let Some(index) = timeline_rows
                                                        .iter()
                                                        .position(|row| row.kind == TimelineTrackKind::ControllerEffect(lane))
                                                    else {
                                                        continue;
                                                    };
                                                    (index, None)
                                                } else {
                                                    let Some(relay_id) = effect.actions.first().map(|a| a.relay_id) else {
                                                        continue;
                                                    };
                                                    let Some(index) = timeline_row_for_relay(&timeline_rows, relay_id) else {
                                                        continue;
                                                    };
                                                    (index, Some(relay_id))
                                                };
                                                let is_mismatched = relay_id
                                                    .is_some_and(|relay_id| !effect.target.is_compatible_with_relay(relay_id));
                                                let track_y = tracks_top + track_index as f32 * 32.0;

                                                let start_x = rect.min.x + (instance.start_time_ms as f32 * px_per_ms);
                                                let end_x = start_x + (effect.duration_ms.max(1) as f32 * px_per_ms);

                                                let clip_rect = egui::Rect::from_min_max(
                                                    egui::pos2(start_x, track_y + 4.0),
                                                    egui::pos2(end_x.max(start_x + 8.0), track_y + 28.0),
                                                );

                                                let clip_id = egui::Id::new(instance.id);
                                                let is_track_locked = relay_id
                                                    .is_some_and(|relay_id| self.app.track_locked.contains(&relay_id));

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
                                                    if ui
                                                        .button(format!(
                                                            "{} {}",
                                                            crate::ui::icons::SLIDERS_HORIZONTAL,
                                                            self.app.tr("Manage...")
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

                                                if clip_response.double_clicked() {
                                                    clicked_any_clip = true;
                                                    manage_cue_id = Some(instance.id);
                                                } else if clip_response.clicked() {
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

                                                let is_muted = relay_id
                                                    .is_some_and(|relay_id| self.app.track_muted.contains(&relay_id));
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

                                        // Apply cue actions outside the immutable timeline borrow loop.
                                        if let Some(cue_id) = manage_cue_id {
                                            self.app.selected_instance_ids.clear();
                                            self.app.selected_instance_ids.insert(cue_id);
                                            self.app.selected_keyframes.clear();
                                            self.app.selected_timeline_keyframe = None;
                                            self.app.open_or_focus_tab(PealayerTab::EffectControls);
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

                                            if ui.ctx().input(|i| i.pointer.any_released()) {
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
                                                            if let Some(target) = nearest_snap_time(
                                                                primary_new_start,
                                                                &snap_targets,
                                                                snap_tolerance,
                                                            ) {
                                                                primary_new_start = target;
                                                                snap_line_x = Some(rect.min.x + (target as f32 * px_per_ms));
                                                            }
                                                            // Check snap to end
                                                            if snap_line_x.is_none() {
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
                                        let pointer_pos = ui.ctx().pointer_latest_pos();

                                        let mut started_drag_info = None;
                                        let mut kf_interp_change = None;
                                        let mut kf_to_remove = None;
                                        let mut pending_add_keyframe = None;

                                        for (t_idx, track) in self
                                            .app
                                            .timeline
                                            .analog_tracks
                                            .iter_mut()
                                            .filter(|track| visible_analog_track_ids.contains(&track.id))
                                            .enumerate()
                                        {
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
                                                    self.app.linked_analog_tracks(),
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
                                                            if track_row.kind
                                                                == TimelineTrackKind::ControllerEffect(
                                                                    payload.controller_lane.unwrap_or(
                                                                        crate::four_d::models::ControllerEffectLane::Sequence,
                                                                    ),
                                                                )
                                                            {
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
                                });

                            let timeline_scroll_id = timeline_scroll.id;
                            let mut timeline_scroll_state = timeline_scroll.state;
                            let timeline_content_size = timeline_scroll.content_size;
                            let timeline_viewport = timeline_scroll.inner_rect;
                            let ((rect, response), clicked_any_clip, clicked_any_keyframe) = timeline_scroll.inner;

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

                            let tracks_top = rect.min.y + 26.0;

                            // Successful drop logic
                            if let Some(payload) = take_effect_drop_on_rect(ui.ctx(), rect) {
                                if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                                    if rect.contains(mouse_pos) {
                                        let relative_y = mouse_pos.y - tracks_top;
                                        let visible_row = (relative_y / 32.0).floor() as i32;
                                        let relative_x = mouse_pos.x - rect.min.x;
                                        let drop_time_secs = (relative_x / zoom) as f64;

                                        self.app.handle_effect_drop(&payload, visible_row, drop_time_secs);
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
                                    for (t_idx, track) in self
                                        .app
                                        .timeline
                                        .analog_tracks
                                        .iter()
                                        .filter(|track| visible_analog_track_ids.contains(&track.id))
                                        .enumerate()
                                    {
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
                                        self.app.selected_timeline_keyframe = None;
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
                                        && !i.modifiers.alt
                                });
                                let jump_next_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::ArrowRight)
                                        && (i.modifiers.ctrl || i.modifiers.command)
                                        && !i.modifiers.alt
                                });
                                let scroll_left_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::ArrowLeft)
                                        && i.modifiers.shift
                                        && !i.modifiers.ctrl
                                        && !i.modifiers.command
                                        && !i.modifiers.alt
                                });
                                let scroll_right_pressed = ui.input(|i| {
                                    i.key_pressed(egui::Key::ArrowRight)
                                        && i.modifiers.shift
                                        && !i.modifiers.ctrl
                                        && !i.modifiers.command
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

                                let num_modifier_free = ui.input(|i| !i.modifiers.ctrl && !i.modifiers.alt && !i.modifiers.command && !i.modifiers.shift);
                                let key_1_pressed = ui.input(|i| i.key_pressed(egui::Key::Num1)) && num_modifier_free;
                                let key_2_pressed = ui.input(|i| i.key_pressed(egui::Key::Num2)) && num_modifier_free;
                                let key_3_pressed = ui.input(|i| i.key_pressed(egui::Key::Num3)) && num_modifier_free;

                                if add_timeline_keyframe_pressed {
                                    let time_ms = (self.app.playback_time * 1_000.0)
                                        .round()
                                        .clamp(0.0, total_seconds * 1_000.0)
                                        as u64;
                                    if !self.app.timeline.keyframes.iter().any(|keyframe| keyframe.time_ms == time_ms) {
                                        self.app.undo_stack.push(self.app.snapshot_timeline());
                                    }
                                    self.app.selected_timeline_keyframe =
                                        Some(self.app.timeline.add_keyframe(time_ms));
                                    self.app.selected_instance_ids.clear();
                                    self.app.selected_keyframes.clear();
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
                                } else if scroll_left_pressed || scroll_right_pressed {
                                    let direction = if scroll_left_pressed { -1.0 } else { 1.0 };
                                    timeline_scroll_state.offset.x =
                                        (timeline_scroll_state.offset.x
                                            + direction * (timeline_viewport.width() * 0.12).max(40.0))
                                            .clamp(
                                                0.0,
                                                (timeline_content_size.x - timeline_viewport.width())
                                                    .max(0.0),
                                            );
                                } else if scroll_up_pressed || scroll_down_pressed {
                                    let direction = if scroll_up_pressed { -1.0 } else { 1.0 };
                                    timeline_scroll_state.offset.y =
                                        (timeline_scroll_state.offset.y + direction * 32.0).clamp(
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

                                        let compiled = crate::four_d::engine::compile_timeline(&self.app.timeline, &self.app.track_muted, &self.app.track_soloed);
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateQueue(compiled));
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.linked_analog_tracks()));
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
                                        let _ = self.app.engine_handle.sender.send(crate::four_d::engine::EngineMessage::UpdateAnalogTracks(self.app.linked_analog_tracks()));
                                    }
                                }
                                timeline_scroll_state.store(ui.ctx(), timeline_scroll_id);
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
    let secs = t.floor() as i64;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    let f = ((t - t.floor()) * 24.0).round() as i64;
    format!("{:02}:{:02}:{:02}:{:02}", h, m, s, f)
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
