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
#[cfg(test)]
pub(crate) fn owner(surface: crate::config::ControlBarSurface) -> Id {
    Id::new(("control-bar", surface))
}

/// Disabled actions remain customizable; invocation is separately capability-gated.
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
    let response = ui.interact(rect, id, egui::Sense::click_and_drag());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    let visuals = ui.style().interact_selectable(&response, selected);
    let hover = super::motion::hover_alpha(
        ui,
        id.with("action-hover"),
        response.hovered() || response.has_focus(),
    );
    let fill = if selected {
        ui.visuals().selection.bg_fill
    } else if frame {
        ui.visuals()
            .widgets
            .inactive
            .weak_bg_fill
            .lerp_to_gamma(visuals.weak_bg_fill, hover)
    } else {
        visuals.weak_bg_fill.gamma_multiply(hover)
    };
    let stroke = if selected || !frame {
        outline(
            ui.visuals(),
            selected,
            enabled && response.is_pointer_button_down_on(),
        )
    } else {
        egui::Stroke::new(
            1.0,
            ui.visuals()
                .widgets
                .inactive
                .bg_stroke
                .color
                .lerp_to_gamma(visuals.bg_stroke.color, hover),
        )
    };
    let color = if enabled {
        visuals.text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    ui.painter().rect(
        rect.shrink(if frame { 0.5 } else { 2.0 }),
        4.0,
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );
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
