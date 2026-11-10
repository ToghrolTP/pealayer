//! Finish built-in window title paint against the final, measured window size.
//! Body content can grow a window after egui has allocated its title strip.
use eframe::egui;

pub(crate) fn install(ctx: &egui::Context) {
    // Plugins are deduplicated by type, independently of restored egui Memory.
    ctx.add_plugin(WindowTitleFrames);
}

struct WindowTitleFrames;
impl egui::Plugin for WindowTitleFrames {
    fn debug_name(&self) -> &'static str {
        "Pealayer window title frames"
    }

    fn on_end_pass(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx();
        let layers = ctx.memory(|memory| memory.areas().visible_layer_ids());
        let windows: Vec<_> = layers
            .into_iter()
            .filter_map(|layer| {
                // This is the pinned egui Window's title-drag response, not a pane,
                // popup or arbitrary rectangle that merely resembles a title bar.
                let title = ctx.read_response(layer.id.with("__title_click"))?;
                if title.layer_id != layer {
                    return None;
                }
                let bounds = egui::AreaState::load(ctx, layer.id)?.rect();
                (bounds.is_positive() && title.rect.is_positive())
                    .then_some((layer, bounds, title.rect))
            })
            .collect();
        // Snapshot all Context-owned facts before taking the graphics lock.
        // No input, memory, style or repaint call is made inside this closure.
        ctx.graphics_mut(|graphics| {
            for (layer, bounds, title) in windows {
                let Some(list) = graphics.get_mut(layer) else {
                    continue;
                };
                let count = list.all_entries().len();
                for index in 0..count {
                    list.mutate_shape(egui::layers::ShapeIdx(index), |clipped| {
                        extend_title_background(&mut clipped.shape, bounds, title);
                    });
                }
            }
        });
    }
}

fn extend_title_background(shape: &mut egui::Shape, bounds: egui::Rect, title: egui::Rect) {
    match shape {
        egui::Shape::Vec(shapes) => {
            for shape in shapes {
                extend_title_background(shape, bounds, title);
            }
        }
        egui::Shape::Rect(rect)
            if rect.fill != egui::Color32::TRANSPARENT
                && (rect.rect.top() - title.top()).abs() <= 2.0
                && (rect.rect.bottom() - title.bottom()).abs() <= 2.0
                && rect.rect.width() >= title.width() =>
        {
            // Preserve any deliberately supplied outer margin and every other
            // property: palette, opacity, corner radii, stroke and paint order.
            let inset = (rect.rect.left() - bounds.left()).max(0.0);
            let right = bounds.right() - inset;
            if right > rect.rect.right() && right.is_finite() {
                rect.rect.max.x = right;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correction_preserves_palette_border_radius_and_unrelated_shapes() {
        let bounds = egui::Rect::from_min_max(egui::pos2(40.0, 40.0), egui::pos2(534.0, 300.0));
        let title = egui::Rect::from_min_max(bounds.min, egui::pos2(480.0, 72.0));
        let mut shape = egui::Shape::rect_filled(
            egui::Rect::from_min_max(bounds.min, egui::pos2(520.0, 72.0)),
            egui::CornerRadius {
                nw: 8,
                ne: 8,
                sw: 0,
                se: 0,
            },
            egui::Color32::from_rgba_premultiplied(20, 30, 40, 220),
        );
        let original = shape.clone();
        extend_title_background(&mut shape, bounds, title);
        if let (egui::Shape::Rect(after), egui::Shape::Rect(before)) = (&shape, &original) {
            assert_eq!(after.rect.max.x, bounds.max.x);
            assert_eq!(after.rect.min, before.rect.min);
            assert_eq!(after.fill, before.fill);
            assert_eq!(after.stroke, before.stroke);
            assert_eq!(after.corner_radius, before.corner_radius);
        } else {
            panic!("title rectangle");
        }
        let corrected = shape.clone();
        extend_title_background(&mut shape, bounds, title);
        assert_eq!(shape, corrected, "correction must be idempotent");
        for mut unrelated in [
            egui::Shape::rect_filled(bounds, 4, egui::Color32::GRAY),
            egui::Shape::rect_filled(
                title.translate(egui::vec2(0.0, 60.0)),
                4,
                egui::Color32::GRAY,
            ),
            egui::Shape::rect_filled(title, 4, egui::Color32::TRANSPARENT),
            egui::Shape::rect_filled(
                egui::Rect::from_min_size(title.min, egui::vec2(24.0, 32.0)),
                4,
                egui::Color32::GRAY,
            ),
        ] {
            let before = unrelated.clone();
            extend_title_background(&mut unrelated, bounds, title);
            assert_eq!(
                unrelated, before,
                "body, action hover fills and transparent strokes are not titles"
            );
        }
    }
}
