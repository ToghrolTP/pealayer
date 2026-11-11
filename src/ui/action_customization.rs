//! Shared action customization: primary movement, frozen grab offset, Escape,
//! typed payload isolation and semantic ordering. Never executes an action.
use eframe::egui::{self, Id, Response};

pub(crate) fn moved_for_drag(ctx: &egui::Context) -> bool {
    let distance = ctx.options(|options| options.input_options.max_click_dist);
    ctx.input(|input| {
        input
            .pointer
            .press_origin()
            .zip(input.pointer.interact_pos())
            .is_some_and(|(start, current)| start.distance(current) > distance)
    })
}
pub(crate) fn register_drag<T: Send + Sync + 'static>(
    response: &Response,
    moved: bool,
    payload: impl FnOnce(egui::Vec2) -> T,
) {
    let offset_id = response.id.with("pointer-offset");
    let started_id = response.id.with("payload-started");
    let fresh = response.ctx.input(|i| {
        i.pointer.button_pressed(egui::PointerButton::Primary)
            && i.pointer
                .press_origin()
                .is_some_and(|p| response.rect.contains(p))
    });
    if fresh {
        response.ctx.data_mut(|d| d.remove_temp::<bool>(started_id));
    }
    let started = response
        .ctx
        .data(|d| d.get_temp::<bool>(started_id))
        .unwrap_or(false);
    super::layout::remember_drag_offset_on_press(&response.ctx, response.rect, offset_id);
    if response.dragged_by(egui::PointerButton::Primary)
        && moved
        && !started
        && !egui::DragAndDrop::has_any_payload(&response.ctx)
    {
        let offset = response
            .ctx
            .data(|d| d.get_temp(offset_id))
            .unwrap_or(response.rect.size() * 0.5);
        egui::DragAndDrop::set_payload(&response.ctx, payload(offset));
        response.ctx.data_mut(|d| d.insert_temp(started_id, true));
    }
    if response
        .ctx
        .input(|i| i.pointer.button_released(egui::PointerButton::Primary))
    {
        response.ctx.data_mut(|d| d.remove_temp::<bool>(started_id));
    }
}
pub(crate) fn drop_payload<T: Send + Sync + 'static>(
    response: &Response,
) -> Option<std::sync::Arc<T>> {
    response
        .ctx
        .input(|i| i.pointer.button_released(egui::PointerButton::Primary))
        .then(|| response.dnd_release_payload::<T>())
        .flatten()
}
pub(crate) use crate::config::move_ordered_action as move_action;
pub(crate) fn hide_action<T: PartialEq>(hidden: &mut Vec<T>, action: T) -> bool {
    if hidden.contains(&action) {
        false
    } else {
        hidden.push(action);
        true
    }
}

/// Shared overflow-to-trash presentation. The owner validates its typed payload
/// before calling; hover is reversible and only a primary release consumes it.
pub(crate) fn menu_hide_target<T: Send + Sync + 'static>(
    ui: &mut egui::Ui,
    response: &Response,
    dragging: bool,
    font: egui::FontId,
) -> Option<std::sync::Arc<T>> {
    let fade = super::motion::hover_alpha(ui, response.id.with("hide-fade"), dragging);
    if fade > 0.0 {
        ui.painter().rect_filled(
            response.rect.shrink(2.0),
            4.0,
            ui.visuals().error_fg_color.gamma_multiply(fade),
        );
    }
    for (icon, alpha, ink) in [
        (
            super::icons::DOTS_THREE,
            1.0 - fade,
            ui.visuals().text_color(),
        ),
        (super::icons::TRASH, fade, egui::Color32::WHITE),
    ] {
        let ink = ink.gamma_multiply(alpha);
        let galley = ui.painter().layout_no_wrap(icon.into(), font.clone(), ink);
        let bounds = if galley.mesh_bounds.is_positive() {
            galley.mesh_bounds
        } else {
            egui::Rect::from_min_size(egui::Pos2::ZERO, galley.size())
        };
        ui.painter().galley(
            response.rect.center() - bounds.center().to_vec2(),
            galley,
            ink,
        );
    }
    if dragging && ui.is_enabled() {
        response.clone().on_hover_cursor(egui::CursorIcon::Grabbing);
        drop_payload::<T>(response)
    } else {
        None
    }
}
#[cfg(test)]
pub(crate) fn owner(surface: crate::config::ControlBarSurface) -> Id {
    Id::new(("control-bar", surface))
}

/// Disabled actions remain customizable; invocation is separately capability-gated.
#[allow(dead_code)]
pub(crate) fn outline(visuals: &egui::Visuals, selected: bool, pressed: bool) -> egui::Stroke {
    egui::Stroke::new(
        1.0,
        if selected || pressed {
            visuals.selection.bg_fill
        } else {
            egui::Color32::TRANSPARENT
        },
    )
}

pub(crate) fn control(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: Id,
    caption: &str,
    label: &str,
    enabled: bool,
    selected: bool,
    frame: bool,
) -> Response {
    paint_control(ui, rect, id, caption, label, enabled, selected, frame, None)
}

#[derive(Clone, Copy)]
pub(crate) struct TransportPaint {
    pub intent: Option<egui::Color32>,
    pub active: bool,
    pub idle_backgrounds: bool,
}

pub(crate) fn transport_control(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: Id,
    caption: &str,
    label: &str,
    enabled: bool,
    paint: TransportPaint,
) -> Response {
    paint_control(ui, rect, id, caption, label, enabled, false, true, Some(paint))
}

fn paint_control(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: Id,
    caption: &str,
    label: &str,
    enabled: bool,
    selected: bool,
    frame: bool,
    transport: Option<TransportPaint>,
) -> Response {
    let response = ui.interact(rect, id, egui::Sense::click_and_drag());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
    let visuals = ui.style().interact_selectable(&response, selected);
    let tokens = super::craft_theme::current(ui.ctx());
    let btn_style = super::craft_theme::icon_button_style(&tokens, frame);

    let pressed = enabled && response.is_pointer_button_down_on();
    let hovered = enabled && response.hovered();
    let (fill, stroke) = if selected {
        (btn_style.selected.fill, btn_style.selected.stroke)
    } else if pressed {
        let p = btn_style.pressed.unwrap_or(btn_style.hovered);
        (p.fill, p.stroke)
    } else if hovered {
        (btn_style.hovered.fill, btn_style.hovered.stroke)
    } else {
        (btn_style.normal.fill, btn_style.normal.stroke)
    };

    // Availability gates invocation immediately; only its presentation fades.
    // The shared timing policy also honors disabled/reduced OS animation.
    let availability = super::motion::hover_alpha(ui, id.with("availability"), enabled);
    // Disabled actions retain drag customization, so their Response still
    // receives pointer hover. That must not brighten an unavailable icon.
    let available_ink = if enabled {
        visuals.text_color()
    } else {
        ui.visuals().widgets.inactive.text_color()
    };
    let color = ui.visuals().weak_text_color().lerp_to_gamma(available_ink, availability);
    let (fill, stroke, color) = if let Some(paint) = transport {
        let idle_fill = if paint.idle_backgrounds {
            tokens.panel_bg.lerp_to_gamma(tokens.hover, 0.28)
        } else {
            egui::Color32::TRANSPARENT
        };
        if !enabled {
            (idle_fill.gamma_multiply(0.55), egui::Stroke::new(1.0, tokens.field_border.gamma_multiply(0.60)),
                ui.visuals().weak_text_color().gamma_multiply(0.72))
        } else if let Some(intent) = paint.intent.filter(|_| paint.active || pressed || hovered || response.has_focus()) {
            let tint = if pressed { 0.24 } else if hovered { 0.17 } else { 0.12 };
            (tokens.panel_bg.lerp_to_gamma(intent, tint),
                egui::Stroke::new(1.0, tokens.panel_bg.lerp_to_gamma(intent, 0.68)), intent)
        } else if pressed || hovered {
            (fill, stroke, color)
        } else {
            (idle_fill, btn_style.normal.stroke, color)
        }
    } else {
        (fill, stroke, color)
    };
    let button_rect = rect.shrink(if frame { 0.5 } else { 2.0 });
    ui.painter().rect(
        button_rect,
        btn_style.corner_radius,
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );
    if response.has_focus() && !btn_style.focus_stroke.is_empty()
        && (transport.is_none() || enabled) {
        let (focus_rect, radius) = if transport.is_some() {
            (button_rect.shrink(2.0), egui::CornerRadius::same(2))
        } else {
            (button_rect, btn_style.corner_radius)
        };
        ui.painter().rect_stroke(
            focus_rect,
            radius,
            btn_style.focus_stroke,
            egui::StrokeKind::Inside,
        );
    }
    let mut font = egui::TextStyle::Button.resolve(ui.style());
    if !frame {
        font.size = 14.0;
    } // fixed ruler geometry; framed bars use shared density
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        caption,
        font,
        color,
    );
    response.on_hover_cursor(if enabled {
        egui::CursorIcon::PointingHand
    } else {
        egui::CursorIcon::Default
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_control_renders_with_craft_icon_button_visuals() {
        let ctx = egui::Context::default();
        let config = crate::config::AppConfig::default();
        crate::ui::configure_native_visuals(&ctx, &config);
        let tokens = crate::ui::craft_theme::current(&ctx);

        let style = crate::ui::craft_theme::icon_button_style(&tokens, false);
        assert_eq!(style.selected.fill, tokens.pill_active_bg);
        assert_eq!(style.selected.stroke.color, tokens.accent);

        // Verify normal unselected state
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(28.0, 28.0));
        let mut normal_output = ctx.run_ui(egui::RawInput::default(), |ui| {
            control(
                ui,
                rect,
                egui::Id::new("normal-btn"),
                "A",
                "Action A",
                true,
                false,
                false,
            );
        });
        normal_output.textures_delta.clear();
        let normal_rect = normal_output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(r) if r.rect == rect.shrink(2.0) => Some(r),
                _ => None,
            })
            .expect("normal button rect shape");
        assert_eq!(normal_rect.fill, style.normal.fill);
        assert_eq!(normal_rect.stroke, style.normal.stroke);

        // Verify selected state
        let mut selected_output = ctx.run_ui(egui::RawInput::default(), |ui| {
            control(
                ui,
                rect,
                egui::Id::new("selected-btn"),
                "B",
                "Action B",
                true,
                true,
                false,
            );
        });
        selected_output.textures_delta.clear();
        let selected_rect = selected_output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(r) if r.rect == rect.shrink(2.0) => Some(r),
                _ => None,
            })
            .expect("selected button rect shape");
        assert_eq!(selected_rect.fill, style.selected.fill);
        assert_eq!(selected_rect.stroke, style.selected.stroke);

        // Verify framed button visuals
        let framed_style = crate::ui::craft_theme::icon_button_style(&tokens, true);
        let mut framed_output = ctx.run_ui(egui::RawInput::default(), |ui| {
            control(
                ui,
                rect,
                egui::Id::new("framed-btn"),
                "C",
                "Action C",
                true,
                false,
                true,
            );
        });
        framed_output.textures_delta.clear();
        let framed_rect = framed_output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Rect(r) if r.rect == rect.shrink(0.5) => Some(r),
                _ => None,
            })
            .expect("framed button rect shape");
        assert_eq!(framed_rect.fill, framed_style.normal.fill);
        assert_eq!(framed_rect.stroke, framed_style.normal.stroke);

        // Verify keyboard focus outline
        let focus_id = egui::Id::new("focused-btn");
        let mut focused_output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.memory_mut(|m| m.request_focus(focus_id));
            control(ui, rect, focus_id, "D", "Action D", true, false, false);
        });
        focused_output.textures_delta.clear();
        let has_focus_stroke = focused_output.shapes.iter().any(|s| match &s.shape {
            egui::epaint::Shape::Rect(r) => r.stroke == style.focus_stroke,
            _ => false,
        });
        assert!(
            has_focus_stroke,
            "focused control must paint focus outline stroke"
        );
    }
}
