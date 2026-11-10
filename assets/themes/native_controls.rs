use super::egui::{
    self, Atom, Atoms, Button, Color32, Direction, Id, IdSalt, IntoAtoms, NumExt, Rect, Response,
    Role, Sense, Shadow, Shape, Stroke, StrokeKind, Vec2, Widget, WidgetAtom, WidgetInfo,
    class::HasClasses as _,
    theme::{DefaultStyle, StyleProvider},
    widget_style::{ButtonStyle, CheckboxStyle, StyleArgs, WidgetState},
};

const PASTEL_CONTROLS: &str = "pealayer-pastel-controls";

pub fn configure(ctx: &egui::Context, pastel: bool) {
    ctx.data_mut(|data| data.insert_temp(Id::unique(PASTEL_CONTROLS), pastel));
    if pastel {
        ctx.add_widget_theme::<ButtonStyle>(PastelButtons);
    } else {
        ctx.add_widget_theme::<ButtonStyle>(DefaultStyle);
    }
}

fn depth(dark: bool, pressed: bool) -> Shadow {
    if pressed {
        return Shadow::NONE;
    }
    Shadow {
        offset: [0, 2],
        blur: 3,
        spread: 0,
        color: if dark {
            Color32::from_black_alpha(128)
        } else {
            Color32::from_rgba_unmultiplied(54, 44, 82, 56)
        },
    }
}

struct PastelButtons;

impl StyleProvider<ButtonStyle> for PastelButtons {
    fn style(&mut self, args: &StyleArgs<'_>) -> ButtonStyle {
        let mut style: ButtonStyle = DefaultStyle.style(args);
        let frame = &mut style.atom_layout.frame;
        if frame.fill != Color32::TRANSPARENT && args.state != WidgetState::Noninteractive {
            let pressed = args.state == WidgetState::Active
                || args.classes.has_class(&Button::CLASS_SELECTED)
                || args.classes.has_class(&egui::ComboBox::CLASS_OPEN);
            frame.shadow = depth(args.style.visuals.dark_mode, pressed);
            if args.state == WidgetState::Inactive && !pressed {
                frame.stroke.color = frame
                    .stroke
                    .color
                    .lerp_to_gamma(style.atom_layout.text_style.color, 0.18);
            }
        }
        style
    }
}

/// A checkbox with Pealayer's Pastel face. The label, interaction and sizing
/// follow egui's checkbox, including the logical leading edge in RTL.
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    atoms: Atoms<'a>,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, label: impl IntoAtoms<'a>) -> Self {
        Self {
            checked,
            atoms: label.into_atoms(),
        }
    }
}

impl Widget for Checkbox<'_> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let Self { checked, mut atoms } = self;
        if !ui
            .ctx()
            .data(|data| data.get_temp(Id::unique(PASTEL_CONTROLS)).unwrap_or(false))
        {
            return egui::Checkbox::new(checked, atoms).ui(ui);
        }

        let CheckboxStyle {
            atom_layout,
            checkbox_size,
            check_size,
            checkbox_frame,
            ..
        } = ui.widget_style(ui.next_auto_id(), &Default::default());
        let min_size = atom_layout.min_size.at_least(Vec2::new(0.0, checkbox_size));
        let id = IdSalt::new("checkbox_icon");
        atoms.push_left(Atom::custom(id, egui::vec2(checkbox_size, min_size.y)));
        let label = atoms.text().map(String::from);
        let direction = if ui.style().right_to_left {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        let layout = WidgetAtom::new(atoms)
            .sense(Sense::click())
            .direction(direction);
        let mut allocated = atom_layout.apply(layout).min_size(min_size).allocate(ui);
        if allocated.response.clicked() {
            *checked = !*checked;
            allocated.response.mark_changed();
        }
        allocated.response.widget_info(|| {
            WidgetInfo::selected(
                Role::CheckBox,
                ui.is_enabled(),
                *checked,
                label.as_deref().unwrap_or(""),
            )
        });
        if !ui.is_rect_visible(allocated.response.rect) {
            return allocated.response;
        }

        let painted = allocated.paint(ui);
        if let Some(icon) = painted.rect(id) {
            let face = Rect::from_center_size(icon.center(), Vec2::splat(checkbox_size));
            let response = &painted.response;
            let visuals = ui.visuals();
            let state = ui.style().interact(response);
            let pressed = response.is_pointer_button_down_on();
            let radius = checkbox_frame.corner_radius;
            let accent = visuals.selection;
            let fill = if *checked {
                accent.bg_fill
            } else {
                state.bg_fill
            };
            let edge = if response.has_focus() {
                accent.bg_fill
            } else if *checked {
                fill.lerp_to_gamma(accent.stroke.color, 0.22)
            } else {
                state
                    .bg_stroke
                    .color
                    .lerp_to_gamma(state.fg_stroke.color, 0.18)
            };
            if response.enabled() && !pressed {
                ui.painter()
                    .add(depth(visuals.dark_mode, false).as_shape(face, radius));
            }
            ui.painter().rect(
                face,
                radius,
                fill,
                Stroke::new(1.0, edge),
                StrokeKind::Inside,
            );
            if response.has_focus() {
                ui.painter().rect_stroke(
                    face.expand(1.0),
                    radius + 1,
                    Stroke::new(1.0, accent.bg_fill),
                    StrokeKind::Inside,
                );
            }
            if *checked {
                let mark = Rect::from_center_size(face.center(), Vec2::splat(check_size));
                ui.painter().add(Shape::line(
                    vec![
                        mark.left_center(),
                        egui::pos2(mark.center().x, mark.bottom()),
                        mark.right_top(),
                    ],
                    Stroke::new(1.5, accent.stroke.color),
                ));
            }
        }
        painted.response
    }
}

pub trait CheckboxUiExt {
    fn themed_checkbox<'a>(&mut self, checked: &'a mut bool, label: impl IntoAtoms<'a>)
    -> Response;
}

impl CheckboxUiExt for egui::Ui {
    fn themed_checkbox<'a>(
        &mut self,
        checked: &'a mut bool,
        label: impl IntoAtoms<'a>,
    ) -> Response {
        self.add(Checkbox::new(checked, label))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pastel_controls_add_depth_without_moving_the_hit_targets() {
        fn frame(
            ctx: &egui::Context,
            checked: &mut bool,
            events: Vec<egui::Event>,
        ) -> ([Rect; 2], egui::FullOutput) {
            let mut rects = [Rect::NOTHING; 2];
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    rects[0] = ui.button("Apply").rect;
                    rects[1] = ui.themed_checkbox(checked, "Loop media").rect;
                },
            );
            output.textures_delta.clear();
            (rects, output)
        }

        fn has_fill(shape: &Shape, fill: Color32) -> bool {
            match shape {
                Shape::Rect(rect) => rect.fill == fill,
                Shape::Vec(shapes) => shapes.iter().any(|shape| has_fill(shape, fill)),
                _ => false,
            }
        }

        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            for rtl in [false, true] {
                let ctx = egui::Context::default();
                ctx.set_theme(theme);
                ctx.all_styles_mut(|style| style.right_to_left = rtl);
                let mut checked = false;
                let (neutral, _) = frame(&ctx, &mut checked, Vec::new());
                configure(&ctx, true);
                let (pastel, output) = frame(&ctx, &mut checked, Vec::new());
                assert_eq!(pastel, neutral);
                let shadow = depth(theme == egui::Theme::Dark, false).color;
                let shadows = output
                    .shapes
                    .iter()
                    .filter(|shape| has_fill(&shape.shape, shadow))
                    .count();
                assert_eq!(shadows, 2, "raised button and checkbox");
                let pos = pastel[1].center();
                for pressed in [true, false] {
                    frame(
                        &ctx,
                        &mut checked,
                        vec![
                            egui::Event::PointerMoved(pos),
                            egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ],
                    );
                }
                assert!(checked);
                configure(&ctx, false);
                let (restored, output) = frame(&ctx, &mut checked, Vec::new());
                assert_eq!(restored, neutral);
                assert!(
                    !output
                        .shapes
                        .iter()
                        .any(|shape| has_fill(&shape.shape, shadow))
                );
            }
        }
    }
}
