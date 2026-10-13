//! One density policy for native inputs, selectors and their choice rows.
use eframe::egui;
use crate::config::UiControlDensity;

pub fn configure(style: &mut egui::Style, density: UiControlDensity) {
    style.spacing.interact_size.y = density.base_height();
    style.spacing.button_padding.y = ((style.spacing.interact_size.y - style.spacing.icon_width) * 0.5).max(0.0);
    // Stroke changes must never expand a field, chevron or neighboring widget.
    for visuals in [&mut style.visuals.widgets.inactive, &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active, &mut style.visuals.widgets.open] {
        visuals.expansion = 0.0;
    }
}

pub fn height(ui: &egui::Ui) -> f32 {
    // Allow large/accessibility fonts to grow every control together.
    let font = egui::TextStyle::Button.resolve(ui.style());
    let line = ui.painter().layout_no_wrap("Ag".into(), font, egui::Color32::WHITE).size().y;
    ui.spacing().interact_size.y.max(line.max(ui.spacing().icon_width) + ui.spacing().button_padding.y * 2.0)
}

pub fn paint_chevron(ui: &egui::Ui, rect: egui::Rect, color: egui::Color32) {
    // An open chevron, rather than mixing a filled triangle and a font glyph.
    let center = rect.center();
    ui.painter().add(egui::Shape::line(vec![
        center + egui::vec2(-4.0, -2.0), center + egui::vec2(0.0, 2.0), center + egui::vec2(4.0, -2.0),
    ], egui::Stroke::new(1.5, color)));
}

pub fn paint_choice_ticks(ui: &egui::Ui, rect: egui::Rect, count: usize, selected: usize) {
    if count < 2 { return; }
    let radius = rect.height() / 2.5;
    let left = rect.left() + radius;
    let right = rect.right().min(rect.left() + ui.spacing().slider_width) - radius;
    for index in 0..count {
        let x = egui::lerp(left..=right, index as f32 / (count - 1) as f32);
        let color = if index == selected { ui.visuals().selection.stroke.color } else { ui.visuals().weak_text_color() };
        // Visible notches flank the rail without covering its pointer target.
        ui.painter().line_segment([egui::pos2(x, rect.bottom() - 3.0), egui::pos2(x, rect.bottom())], egui::Stroke::new(1.0, color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs_combos_color_triggers_and_menu_rows_share_density_in_both_themes() {
        for density in UiControlDensity::ALL {
            for dark in [false, true] {
                for scale in [1.0, 1.25, 1.5, 2.0] {
                    for font_size in [12.0, 14.0, 20.0] {
                        let context = egui::Context::default();
                        context.set_pixels_per_point(scale);
                        context.set_visuals(if dark { egui::Visuals::dark() } else { egui::Visuals::light() });
                        context.all_styles_mut(|style| {
                            configure(style, density);
                            style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(font_size));
                        });
                        let mut value = "System".to_owned();
                        let mut color = "system".to_owned();
                        let mut hex = "#0078D4".to_owned();
                        let options = [crate::ui::color_picker::ColorChoice {
                            value: "system".into(), label: "System".into(), rgb: [0, 120, 212],
                        }];
                        let mut expected = 0.0;
                        let mut rects = Vec::new();
                        for pass in 0..3 {
                            rects.clear();
                            let mut output = context.run_ui(egui::RawInput {
                                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 500.0))),
                                events: if pass == 0 { vec![] } else { vec![egui::Event::PointerMoved(egui::pos2(40.0, 45.0))] },
                                ..Default::default()
                            }, |ui| {
                                ui.set_width(300.0);
                                expected = height(ui);
                                rects.push(crate::ui::dialog::singleline_text_edit(ui, &mut value).show(ui).response.rect);
                                rects.push(crate::ui::dropdown::ComboBox::from_id_salt("density-combo").width(300.0)
                                    .selected_text("System").show_ui(ui, |_| {}).response.rect);
                                rects.push(crate::ui::color_picker::color_dropdown(ui, "density-color", &mut color,
                                    &mut hex, &options, 300.0, "Custom").rect);
                                rects.push(crate::ui::dropdown::choice(ui, false, "System", None).rect);
                                rects.push(crate::ui::color_picker::custom_menu_row(ui, false, "Custom", &hex, [0,120,212]).rect);
                                rects.push(crate::ui::color_picker::color_field(ui, &mut hex, [0,120,212], 300.0).rect);
                            });
                            output.textures_delta.clear();
                            for (index, rect) in rects.iter().enumerate() {
                                assert!((rect.height() - expected).abs() < 0.01,
                                    "mixed density: {density:?}, dark={dark}, scale={scale}, font={font_size}, control={index}, expected={expected}, got={rect:?}");
                            }
                        }
                    }
                }
            }
        }
    }
}
