use eframe::egui::{self, Pos2, Vec2};

use crate::app::PealayerApp;
use crate::four_d::seat_kinematics::CinemaSeatKinematics;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum View {
    #[default]
    Side,
    Orbit,
}

#[derive(Clone, Copy)]
struct Camera {
    yaw: f32,
    pitch: f32,
    pan: Vec2,
    zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 0.65,
            pitch: 0.38,
            pan: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct SeatView {
    view: View,
    side: Option<usize>,
    camera: Camera,
}

type Point = [f32; 3];

fn rail(z: f32, stroke: f32) -> [Point; 5] {
    let pivot = [-0.68, 0.48, z];
    let angle = (stroke - 0.5) * 0.52;
    let (sin, cos) = angle.sin_cos();
    [
        [-0.23, 0.84],
        [-0.15, 0.72],
        [0.0, 0.04],
        [1.30, 0.04],
        [1.34, -0.05],
    ]
    .map(|[x, y]| {
        [
            pivot[0] + x * cos - y * sin,
            pivot[1] + x * sin + y * cos,
            z,
        ]
    })
}

fn floor(z: f32) -> [Point; 2] {
    [[-0.93, 0.0, z], [0.85, 0.0, z]]
}

fn draw_side(
    painter: &egui::Painter,
    project: impl Fn(Point) -> Pos2,
    z: f32,
    stroke: f32,
    color: egui::Color32,
    line_width: f32,
) {
    let profile = rail(z, stroke);
    let line = |a, b, width, ink| {
        painter.line_segment([project(a), project(b)], egui::Stroke::new(width, ink))
    };
    let [base_back, base_front] = floor(z);
    line(
        base_back,
        base_front,
        line_width * 0.65,
        color.gamma_multiply(0.65),
    );
    line(
        [-0.68, 0.0, z],
        [-0.68, 0.48, z],
        line_width * 0.65,
        color.gamma_multiply(0.65),
    );
    line(
        [0.50, 0.02, z],
        [0.50, 0.37, z],
        line_width * 1.7,
        color.gamma_multiply(0.6),
    );
    line([0.50, 0.37, z], profile[3], line_width * 0.8, color);
    for segment in profile.windows(2) {
        line(segment[0], segment[1], line_width, color);
    }
}

fn orbit_projection(rect: egui::Rect, camera: Camera, point: Point) -> (Pos2, f32) {
    let [x, y, z] = point;
    let (ys, yc) = camera.yaw.sin_cos();
    let (ps, pc) = camera.pitch.sin_cos();
    let horizontal = yc * x - ys * z;
    let forward = ys * x + yc * z;
    let height = y - 0.58;
    let vertical = pc * height - ps * forward;
    let depth = ps * height + pc * forward;
    let scale = (rect.width() / 2.9).min(rect.height() / 2.25) * camera.zoom;
    (
        rect.center() + camera.pan + egui::vec2(horizontal, -vertical) * scale,
        depth,
    )
}

fn side_projection(rect: egui::Rect, point: Point) -> Pos2 {
    let [x, y, z] = point;
    let scale = (rect.width() / 2.3).min(rect.height() / 1.8);
    rect.center() + egui::vec2(x - z * 0.18, 0.64 - y + z * 0.11) * scale
}

fn near_profile(pointer: Pos2, profile: [Pos2; 5]) -> f32 {
    profile
        .windows(2)
        .map(|segment| {
            let line = segment[1] - segment[0];
            let fraction =
                ((pointer - segment[0]).dot(line) / line.length_sq().max(0.01)).clamp(0.0, 1.0);
            pointer.distance(segment[0] + line * fraction)
        })
        .fold(f32::INFINITY, f32::min)
}

fn draw_orbit(
    painter: &egui::Painter,
    rect: egui::Rect,
    state: SeatView,
    strokes: [f32; 2],
    colors: [egui::Color32; 2],
) {
    let project = |point| orbit_projection(rect, state.camera, point).0;
    let (left, right) = (rail(0.63, strokes[0]), rail(-0.63, strokes[1]));
    let base = [
        floor(0.63)[0],
        floor(0.63)[1],
        floor(-0.63)[1],
        floor(-0.63)[0],
    ];
    let floor_ink = colors[0].gamma_multiply(0.30);
    painter.add(egui::Shape::closed_line(
        base.map(project).to_vec(),
        egui::Stroke::new(1.0, floor_ink),
    ));
    // Sort the four seat surfaces from far to near so the rails remain
    // legible as the camera rotates through the rear of the seat.
    let mut surfaces =
        std::array::from_fn::<_, 4, _>(|i| [left[i], left[i + 1], right[i + 1], right[i]]);
    let depth = |surface: &[Point; 4]| {
        surface
            .iter()
            .map(|&point| orbit_projection(rect, state.camera, point).1)
            .sum::<f32>()
    };
    surfaces.sort_by(|a, b| depth(a).total_cmp(&depth(b)));
    for surface in surfaces {
        painter.add(egui::Shape::convex_polygon(
            surface.map(project).to_vec(),
            colors[0].gamma_multiply(0.25),
            egui::Stroke::new(1.0, floor_ink),
        ));
    }
    for (i, z) in [0.63, -0.63].into_iter().enumerate() {
        draw_side(painter, project, z, strokes[i], colors[i], 2.0);
    }
}

pub(super) fn draw(
    app: &PealayerApp,
    ui: &mut egui::Ui,
    kinematics: CinemaSeatKinematics,
    frozen: bool,
) {
    let id = egui::Id::unique("virtual-board-seat-view");
    let mut state = ui
        .data(|data| data.get_temp::<SeatView>(id))
        .unwrap_or_default();
    let strokes = [kinematics.left_stroke, kinematics.right_stroke];
    ui.horizontal_wrapped(|ui| {
        ui.strong(app.tr("Seat motion"));
        ui.selectable_value(&mut state.view, View::Side, app.tr("2D side"));
        ui.selectable_value(&mut state.view, View::Orbit, app.tr("3D orbit"));
        if state.view == View::Orbit && ui.small_button(app.tr("Reset view")).clicked() {
            state.camera = Camera::default();
        }
    });

    let width = ui.available_width().max(1.0);
    let height = (width * 0.57).clamp(110.0, 290.0);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click_and_drag());
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    painter.rect(
        rect,
        7.0,
        ui.visuals().faint_bg_color,
        ui.visuals().widgets.noninteractive.bg_stroke,
        egui::StrokeKind::Inside,
    );
    let accent = ui.visuals().selection.stroke.color;
    let rail_color = ui.visuals().text_color();
    match state.view {
        View::Side => {
            // Left is the near side of this side elevation. The right side
            // shares the same baseline and is offset slightly for inspection.
            let depths = [0.63, -0.63];
            let project = |point| side_projection(rect, point);
            let hovered = response.hover_pos().and_then(|pointer| {
                let [left, right] = std::array::from_fn(|i| {
                    near_profile(pointer, rail(depths[i], strokes[i]).map(project))
                });
                if left.min(right) > 10.0 {
                    None
                } else {
                    Some(usize::from(right < left))
                }
            });
            if response.clicked() {
                state.side = hovered;
            }
            for i in [1, 0] {
                let color = if state.side == Some(i) {
                    accent
                } else if i == 0 || hovered == Some(i) {
                    rail_color
                } else {
                    rail_color.gamma_multiply(0.35)
                };
                draw_side(&painter, project, depths[i], strokes[i], color, 3.0);
            }
        }
        View::Orbit => {
            if response.dragged() {
                let motion = response.drag_delta();
                if response.dragged_by(egui::PointerButton::Secondary)
                    || ui.input(|input| input.modifiers.shift)
                {
                    state.camera.pan += motion;
                    state.camera.pan = state.camera.pan.clamp(-rect.size(), rect.size());
                } else {
                    state.camera.yaw =
                        (state.camera.yaw + motion.x * 0.012).rem_euclid(std::f32::consts::TAU);
                    state.camera.pitch = (state.camera.pitch + motion.y * 0.012).clamp(-1.35, 1.35);
                }
            }
            if response.hovered() {
                let zoom = ui.input(|input| input.zoom_delta());
                state.camera.zoom = (state.camera.zoom * zoom).clamp(0.55, 3.0);
            }
            response
                .on_hover_cursor(egui::CursorIcon::Grab)
                .on_hover_text(app.tr("Drag to rotate. Shift-drag or right-drag to pan."));
            let colors = std::array::from_fn(|i| {
                if state.side == Some(i) {
                    accent
                } else {
                    rail_color
                }
            });
            draw_orbit(&painter, rect, state, strokes, colors);
        }
    }

    ui.horizontal_wrapped(|ui| {
        for (i, label) in ["Left side", "Right side"].into_iter().enumerate() {
            let label = format!("{} · {:.0}%", app.tr(label), strokes[i] * 100.0);
            if ui.selectable_label(state.side == Some(i), label).clicked() {
                state.side = (state.side != Some(i)).then_some(i);
            }
        }
        ui.weak(format!(
            "{} {:+.1}° · {} {:+.1}°",
            app.tr("Pitch"),
            kinematics.pitch_degrees(),
            app.tr("Roll"),
            kinematics.roll_degrees()
        ));
        if frozen {
            ui.weak(app.tr("Motion interlocked"));
        }
    });
    ui.data_mut(|data| data.insert_temp(id, state));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_and_orbit_project_independent_actuators() {
        let viewport = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(170.0, 110.0));
        let left = rail(0.63, 1.0);
        let right = rail(-0.63, 0.0);
        assert!(side_projection(viewport, left[3]).y < side_projection(viewport, right[3]).y);
        for point in left.into_iter().chain(right) {
            assert!(viewport.contains(side_projection(viewport, point)));
            assert!(viewport.contains(orbit_projection(viewport, Camera::default(), point).0));
        }
        let point = rail(0.63, 0.5)[3];
        let original = orbit_projection(viewport, Camera::default(), point).0;
        let pan = egui::vec2(12.0, -7.0);
        assert_eq!(
            orbit_projection(
                viewport,
                Camera {
                    pan,
                    ..Camera::default()
                },
                point
            )
            .0,
            original + pan
        );
        assert_ne!(
            orbit_projection(
                viewport,
                Camera {
                    yaw: 2.1,
                    ..Camera::default()
                },
                point
            )
            .0,
            original
        );
    }
}
