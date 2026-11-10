//! Simple player chrome, bounded placement and GPU-only video backdrop.
use crate::{app::PealayerApp, config::ControlBackdrop};
use eframe::egui::{self, Color32, Pos2, Rect, Shape, Vec2};

pub fn toolbar_visible(app: &PealayerApp) -> bool {
    if app.show_four_d_editor {
        app.toolbar_state.preferences.enabled
    } else {
        app.simple_workspace.show_toolbar
    }
}
pub fn set_toolbar_visible(app: &mut PealayerApp, visible: bool) {
    if app.show_four_d_editor {
        app.toolbar_state.preferences.enabled = visible;
    } else {
        app.simple_workspace.show_toolbar = visible;
    }
}
pub fn alpha(ctx: &egui::Context, elapsed: f32, pinned: bool, editing: bool) -> f32 {
    super::motion::sync(ctx);
    let visible = pinned || editing || elapsed < 2.5 || egui::Popup::is_any_open(ctx);
    let duration = if ctx.style_of(ctx.theme()).animation_time > 0.0 {
        0.25
    } else {
        0.0
    };
    if visible && !pinned && !editing && elapsed < 2.5 {
        ctx.request_repaint_after(std::time::Duration::from_secs_f32(
            (2.5 - elapsed).max(0.001),
        ));
    }
    ctx.animate_bool_with_time_and_easing(
        egui::Id::unique("simple-controls-visibility"),
        visible,
        duration,
        egui::emath::easing::cubic_in_out,
    )
}
pub fn position(bounds: Rect, size: Vec2, normalized: [f32; 2]) -> Pos2 {
    let safe = bounds.shrink(16.0);
    let travel = (safe.size() - size).max(Vec2::ZERO);
    safe.min
        + egui::vec2(
            travel.x * normalized[0].clamp(0.0, 1.0),
            travel.y * normalized[1].clamp(0.0, 1.0),
        )
}
pub fn normalized(bounds: Rect, size: Vec2, pos: Pos2) -> [f32; 2] {
    let safe = bounds.shrink(16.0);
    let travel = (safe.size() - size).max(Vec2::ZERO);
    [
        if travel.x > 0.0 {
            ((pos.x - safe.left()) / travel.x).clamp(0.0, 1.0)
        } else {
            0.5
        },
        if travel.y > 0.0 {
            ((pos.y - safe.top()) / travel.y).clamp(0.0, 1.0)
        } else {
            1.0
        },
    ]
}
pub fn menu(app: &mut PealayerApp, ui: &mut egui::Ui) {
    let before = app.simple_workspace;
    ui.label(egui::RichText::new(app.tr("Simple workspace")).weak());
    ui.checkbox(&mut app.simple_workspace.show_menu_bar, "Show menu bar");
    ui.checkbox(
        &mut app.simple_workspace.show_toolbar,
        "Show command toolbar",
    );
    ui.checkbox(&mut app.simple_workspace.show_status_bar, "Show status bar");
    ui.separator();
    if ui
        .button(format!(
            "{}  Reset control bar position",
            super::icons::ARROW_COUNTER_CLOCKWISE
        ))
        .clicked()
    {
        app.simple_workspace.controls_x = 0.5;
        app.simple_workspace.controls_y = 1.0;
        ui.close();
    }
    if app.simple_workspace != before {
        app.request_simple_workspace_save(ui.ctx());
    }
}

#[derive(Clone, Copy)]
struct Backdrop {
    texture: egui::TextureId,
    video: Rect,
}
fn backdrop_id() -> egui::Id {
    egui::Id::unique("simple-video-backdrop")
}
pub fn publish_video(ctx: &egui::Context, source: Option<(egui::TextureId, Rect)>) {
    ctx.data_mut(|data| {
        if let Some((texture, video)) = source {
            data.insert_temp(backdrop_id(), Backdrop { texture, video });
        } else {
            data.remove::<Backdrop>(backdrop_id());
        }
    });
}
fn rounded_points(rect: Rect, radius: f32) -> Vec<Pos2> {
    let r = radius
        .min(rect.width() * 0.5)
        .min(rect.height() * 0.5)
        .max(0.0);
    let mut points = Vec::with_capacity(28);
    for (center, start) in [
        (rect.left_top() + egui::vec2(r, r), std::f32::consts::PI),
        (
            rect.right_top() + egui::vec2(-r, r),
            1.5 * std::f32::consts::PI,
        ),
        (rect.right_bottom() + egui::vec2(-r, -r), 0.0),
        (
            rect.left_bottom() + egui::vec2(r, -r),
            0.5 * std::f32::consts::PI,
        ),
    ] {
        for i in 0..=6 {
            let angle = start + i as f32 * std::f32::consts::FRAC_PI_2 / 6.0;
            points.push(center + egui::vec2(angle.cos() * r, angle.sin() * r));
        }
    }
    points
}
fn sample_mesh(source: Backdrop, rect: Rect, offset: Vec2, opacity: f32) -> egui::Mesh {
    let mut mesh = egui::Mesh::with_texture(source.texture);
    let points = rounded_points(rect, 8.0);
    for point in std::iter::once(rect.center()).chain(points.iter().copied()) {
        let uv = (point - source.video.min + offset) / source.video.size();
        mesh.vertices.push(egui::epaint::Vertex {
            pos: point,
            uv: egui::pos2(uv.x.clamp(0.0, 1.0), uv.y.clamp(0.0, 1.0)),
            color: Color32::WHITE.linear_multiply(opacity),
        });
    }
    for i in 0..points.len() {
        mesh.indices
            .extend([0, i as u32 + 1, (i as u32 + 1) % points.len() as u32 + 1]);
    }
    mesh
}
/// Nine weighted Gaussian taps use the existing GPU texture and egui shader.
/// No framebuffer reads, CPU images, extra decode, GL resources or idle timer.
pub fn backdrop_shapes(
    ctx: &egui::Context,
    rect: Rect,
    fill: Color32,
    alpha: f32,
    mode: ControlBackdrop,
) -> Vec<Shape> {
    let mut shapes = vec![Shape::rect_filled(rect, 8.0, fill.linear_multiply(alpha))];
    let source = ctx.data(|data| data.get_temp::<Backdrop>(backdrop_id()));
    let enabled = mode != ControlBackdrop::Solid
        && crate::performance::hardware_backdrop_available()
        && (mode == ControlBackdrop::Blur || auto_blur_allowed(ctx));
    if enabled && let Some(source) = source {
        let clipped = rect.intersect(source.video);
        if clipped.is_positive() {
            let mut total = 0.0;
            for (x, y, weight) in [
                (0., 0., 4.),
                (-1., 0., 2.),
                (1., 0., 2.),
                (0., -1., 2.),
                (0., 1., 2.),
                (-1., -1., 1.),
                (1., -1., 1.),
                (-1., 1., 1.),
                (1., 1., 1.),
            ] {
                total += weight;
                let opacity = alpha * weight / (16.0 - alpha * (16.0 - total));
                shapes.push(Shape::mesh(sample_mesh(
                    source,
                    clipped,
                    egui::vec2(x * 4.0, y * 4.0),
                    opacity,
                )));
            }
            shapes.push(Shape::rect_filled(
                rect,
                8.0,
                fill.linear_multiply(alpha * 0.78),
            ));
        }
    }
    shapes
}

#[derive(Clone, Default)]
struct BlurBudget {
    frame: u64,
    slow: u8,
    disabled: bool,
}
fn auto_blur_allowed(ctx: &egui::Context) -> bool {
    use std::sync::atomic::Ordering;
    let frame = super::video::MPV_RENDER_COUNT.load(Ordering::Relaxed);
    let micros = super::video::LAST_MPV_RENDER_MICROS.load(Ordering::Relaxed);
    ctx.data_mut(|data| {
        let budget =
            data.get_temp_mut_or_default::<BlurBudget>(egui::Id::unique("simple-blur-budget"));
        if budget.frame != frame {
            budget.frame = frame;
            budget.slow = if micros > 20_000 {
                budget.slow.saturating_add(1)
            } else {
                0
            };
            budget.disabled |= budget.slow >= 3;
        }
        // Latch degradation for this process, avoiding per-frame quality flicker.
        !budget.disabled
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn position_respects_video_bounds_and_status_bar_reservation() {
        let full = Rect::from_min_size(Pos2::ZERO, egui::vec2(1200., 800.));
        let size = egui::vec2(900., 44.);
        let shorter = Rect::from_min_max(full.min, egui::pos2(1200., 770.));
        let a = position(full, size, [0.5, 1.]);
        let b = position(shorter, size, [0.5, 1.]);
        assert_eq!(a.y - b.y, 30.);
        assert!(b.y + size.y < shorter.bottom());
        for p in [[0., 0.], [0.3, 0.4], [1., 1.]] {
            let round = normalized(full, size, position(full, size, p));
            assert!((round[0] - p[0]).abs() < 0.001 && (round[1] - p[1]).abs() < 0.001);
        }
    }
    #[test]
    fn defaults_are_clean_and_configuration_is_validated() {
        let config = crate::config::AppConfig::default();
        assert!(
            !config.simple_workspace.show_toolbar
                && !config.simple_workspace.show_status_bar
                && !config.simple_workspace.show_menu_bar
        );
        assert!(
            config
                .apply_patch(&serde_json::json!({"simple_workspace":{"controls_x":1.1}}))
                .is_err()
        );
        assert!(
            config
                .apply_patch(&serde_json::json!({"simple_workspace":{"show_status_bar":true}}))
                .unwrap()
                .simple_workspace
                .show_status_bar
        );
    }
    #[test]
    fn toolbar_visibility_does_not_change_the_other_workspace() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = PealayerApp::default();
        app.toolbar_state.preferences.enabled = true;
        assert!(toolbar_visible(&app));
        app.show_four_d_editor = false;
        assert!(!toolbar_visible(&app));
        set_toolbar_visible(&mut app, true);
        assert!(toolbar_visible(&app));
        set_toolbar_visible(&mut app, false);
        assert!(app.toolbar_state.preferences.enabled);
        app.show_four_d_editor = true;
        assert!(toolbar_visible(&app));
    }
    #[test]
    fn mesh_has_bounded_single_gpu_source_and_valid_uvs() {
        let source = Backdrop {
            texture: egui::TextureId::User(3),
            video: Rect::from_min_size(Pos2::ZERO, egui::vec2(800., 450.)),
        };
        let mesh = sample_mesh(
            source,
            Rect::from_min_size(egui::pos2(20., 400.), egui::vec2(700., 40.)),
            egui::vec2(4., 4.),
            0.5,
        );
        assert_eq!(mesh.texture_id, source.texture);
        assert!(mesh.is_valid());
        assert!(
            mesh.vertices
                .iter()
                .all(|v| (0.0..=1.0).contains(&v.uv.x) && (0.0..=1.0).contains(&v.uv.y))
        );
    }
    #[test]
    fn smooth_fade_is_intermediate_and_reduced_motion_is_instant() {
        let ctx = egui::Context::default();
        super::super::motion::configure(&ctx, crate::config::UiAnimationMode::Enabled);
        let mut value = 0.0;
        for (time, elapsed) in [(0.0, 0.0), (0.01, 3.0), (0.06, 3.0), (0.12, 3.0)] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    ..Default::default()
                },
                |_| value = alpha(&ctx, elapsed, false, false),
            );
            output.textures_delta.clear();
        }
        assert!(value > 0.0 && value < 1.0, "intermediate fade: {value}");
        super::super::motion::configure(&ctx, crate::config::UiAnimationMode::Disabled);
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(0.2),
                ..Default::default()
            },
            |_| value = alpha(&ctx, 3.0, false, false),
        );
        output.textures_delta.clear();
        assert_eq!(value, 0.0);
    }
    fn render(
        ctx: &egui::Context,
        app: &mut PealayerApp,
        time: f64,
        events: Vec<egui::Event>,
    ) -> Rect {
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1600., 900.))),
                events,
                ..Default::default()
            },
            |ui| {
                super::super::status_bar::draw(app, ui);
                egui::CentralPanel::default()
                    .show_inside(ui, |ui| super::super::controls::draw(app, ui));
            },
        );
        output.textures_delta.clear();
        ctx.memory(|memory| memory.area_rect(egui::Id::unique("simple-transport-overlay")))
            .unwrap()
    }
    #[test]
    fn actual_bar_drags_without_seeking_and_clears_the_status_panel() {
        let _guard = crate::app::tests::lock_app_tests();
        let mut app = PealayerApp::default();
        app.show_four_d_editor = false;
        app.pin_controls = true;
        app.simple_workspace.show_status_bar = true;
        let ctx = egui::Context::default();
        let mut rect = Rect::NOTHING;
        for n in 0..4 {
            rect = render(&ctx, &mut app, n as f64 * 0.05, vec![]);
        }
        assert!(rect.bottom() < 870., "bar must clear status: {rect:?}");
        let start = rect.left_center() + egui::vec2(7., 0.);
        let pointer = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        render(
            &ctx,
            &mut app,
            0.3,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
        );
        let end = start + egui::vec2(30., -150.);
        render(&ctx, &mut app, 0.4, vec![egui::Event::PointerMoved(end)]);
        render(&ctx, &mut app, 0.5, vec![pointer(end, false)]);
        assert!(
            app.simple_workspace.controls_y < 0.9,
            "drag must update placement"
        );
        assert!(
            app.pending_simple_workspace_save.is_some(),
            "placement must use coalesced owner save"
        );
        assert_eq!(app.volume, 100.);
        assert!(app.seek_pos.is_none());
    }
}
