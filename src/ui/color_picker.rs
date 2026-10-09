//! Compact RGB editor shared by channel presentation and custom accents.
//! The swatch is a click target inside the text field's reserved leading margin.
use eframe::egui;

pub fn color_field(
    ui: &mut egui::Ui,
    hex: &mut String,
    fallback: [u8; 3],
    width: f32,
) -> egui::Response {
    let id = ui.next_auto_id().with("rgb-color-field");
    let mut response = ui.add_sized(
        [width, 28.0],
        crate::ui::dialog::singleline_text_edit(hex)
            .id(id)
            .desired_width(width)
            .margin(egui::Margin {
                left: 30,
                right: 6,
                top: 5,
                bottom: 5,
            })
            .char_limit(7)
            .hint_text("#RRGGBB"),
    );
    let rgb = crate::config::parse_rgb_hex(hex).unwrap_or(fallback);
    let swatch_rect = egui::Rect::from_center_size(
        egui::pos2(response.rect.left() + 15.0, response.rect.center().y),
        egui::vec2(20.0, 20.0),
    );
    let swatch = ui
        .interact(swatch_rect, id.with("swatch"), egui::Sense::click())
        .on_hover_text("Choose color");
    ui.painter().circle_filled(
        swatch_rect.center(),
        6.0,
        egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
    );
    ui.painter().circle_stroke(
        swatch_rect.center(),
        6.0,
        ui.visuals().widgets.noninteractive.bg_stroke,
    );
    egui::Popup::menu(&swatch)
        .id(id.with("palette"))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            let mut color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            if egui::color_picker::color_picker_color32(
                ui,
                &mut color,
                egui::color_picker::Alpha::Opaque,
            ) {
                *hex = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
                response.mark_changed();
            }
        });
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrated_color_field_keeps_round_swatch_inside_text_field_in_both_themes() {
        for dark in [false, true] {
            let ctx = egui::Context::default();
            ctx.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            let mut hex = "#38D27A".to_owned();
            let mut rect = egui::Rect::NOTHING;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                rect = color_field(ui, &mut hex, [0, 0, 0], 124.0).rect;
            });
            output.textures_delta.clear();
            assert!((rect.width() - 124.0).abs() < 0.1);
            let circles: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| {
                    if let egui::epaint::Shape::Circle(circle) = &shape.shape {
                        Some(circle)
                    } else {
                        None
                    }
                })
                .collect();
            assert!(circles.iter().any(|circle| circle.fill
                == egui::Color32::from_rgb(56, 210, 122)
                && rect.contains(circle.center)
                && circle.radius == 6.0));
            assert_eq!(hex, "#38D27A");
        }
    }

    #[test]
    fn color_field_hex_edit_and_swatch_popup_are_interactive() {
        let ctx = egui::Context::default();
        let mut hex = "#38D27A".to_owned();
        let frame = |events: Vec<egui::Event>, hex: &mut String| {
            let mut response = None;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    response = Some(color_field(ui, hex, [0, 0, 0], 124.0));
                },
            );
            output.textures_delta.clear();
            response.unwrap()
        };
        let response = frame(vec![], &mut hex);
        let text_pos = egui::pos2(response.rect.left() + 60.0, response.rect.center().y);
        let click = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            vec![egui::Event::PointerMoved(text_pos), click(text_pos, true)],
            &mut hex,
        );
        frame(vec![click(text_pos, false)], &mut hex);
        let command = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        let changed = frame(
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: command,
                },
                egui::Event::Text("#123ABC".to_owned()),
            ],
            &mut hex,
        );
        assert!(changed.changed());
        assert_eq!(hex, "#123ABC");
        let swatch_pos = egui::pos2(response.rect.left() + 15.0, response.rect.center().y);
        frame(
            vec![
                egui::Event::PointerMoved(swatch_pos),
                click(swatch_pos, true),
            ],
            &mut hex,
        );
        frame(vec![click(swatch_pos, false)], &mut hex);
        assert!(
            egui::Popup::is_id_open(&ctx, response.id.with("palette")),
            "leading swatch opens the actual color picker"
        );
    }
}
