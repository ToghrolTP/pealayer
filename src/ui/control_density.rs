//! One density policy for native inputs, selectors and their choice rows.
use eframe::egui;
use crate::config::UiControlDensity;

pub fn configure(style: &mut egui::Style, density: UiControlDensity) {
    // Before 88741e3, egui 0.36.2 supplied an 18-point minimum and 1-point
    // vertical padding. Deriving padding from icon width would inflate that
    // restored size again when the text is taller than the icon.
    super::palette::native::configure_density(style, density.base_height());
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
    ui.painter().with_clip_rect(ui.clip_rect().intersect(rect)).add(egui::Shape::line(vec![
        center + egui::vec2(-4.0, -2.0), center + egui::vec2(0.0, 2.0), center + egui::vec2(4.0, -2.0),
    ], egui::Stroke::new(1.5, color)));
}

/// Discrete choices retain egui's pointer, keyboard and accessibility behavior.
/// The value capsule reserves the widest translated label, never the active one.
pub fn choice_slider(
    ui: &mut egui::Ui,
    index: &mut usize,
    labels: &[String],
    caption: &str,
    width: f32,
) -> egui::Response {
    assert!(!labels.is_empty());
    *index = (*index).min(labels.len() - 1);
    let width = width.min(ui.available_width()).max(1.0);
    let font = egui::TextStyle::Button.resolve(ui.style());
    let label_width = labels.iter().map(|label| ui.painter()
        .layout_no_wrap(label.clone(), font.clone(), ui.visuals().text_color()).size().x)
        .fold(0.0_f32, f32::max);
    let row_height = height(ui).max(ui.text_style_height(&egui::TextStyle::Body));
    let badge_width = (label_width + 24.0).min(width);
    let stacked = width < badge_width + 84.0;
    let total_height = if stacked { row_height * 2.0 + 6.0 } else { row_height };
    let (bounds, _) = ui.allocate_exact_size(egui::vec2(width, total_height), egui::Sense::hover());
    let badge = egui::Rect::from_min_size(
        egui::pos2(bounds.right() - badge_width, bounds.bottom() - row_height),
        egui::vec2(badge_width, row_height));
    let rail_slot = egui::Rect::from_min_size(bounds.min,
        egui::vec2(if stacked {width} else {width - badge_width - 12.0}, row_height))
        .shrink2(egui::vec2(2.0_f32.min(width / 4.0), 0.0));
    let mut response = {
        // The parent has allocated bounds already. A non-allocating child avoids
        // advancing the surrounding Preferences row a second time.
        let mut slider_ui = ui.new_child(egui::UiBuilder::new().max_rect(rail_slot)
            .layout(egui::Layout::left_to_right(egui::Align::Center)));
        slider_ui.spacing_mut().slider_width = rail_slot.width();
        // Inner rail/handle geometry and reserved slot have the same height.
        slider_ui.spacing_mut().interact_size.y = row_height;
        // Fixed geometry through hover, dragging and keyboard focus.
        let widgets = &mut slider_ui.visuals_mut().widgets;
        for visuals in [&mut widgets.inactive, &mut widgets.hovered, &mut widgets.active] {
            visuals.expansion = 0.0;
            visuals.fg_stroke.width = 1.25;
        }
        slider_ui.add(egui::Slider::new(index, 0..=labels.len() - 1)
            .step_by(1.0).show_value(false).trailing_fill(true)
            .handle_shape(egui::style::HandleShape::Circle))
    };
    let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(bounds));
    // egui's explicit Circle handle travels within height/2.5 of the endpoints.
    // Test this against actual painted handle centers, not just this calculation.
    let travel = response.rect.x_range().shrink(response.rect.height() / 2.5);
    if labels.len() > 1 {
        for position in 0..labels.len() {
            if position == *index { continue; } // Never paint over the handle.
            let x = egui::lerp(travel, position as f32 / (labels.len() - 1) as f32);
            let color = if position < *index { ui.visuals().selection.stroke.color }
                else { ui.visuals().weak_text_color() };
            painter.circle_filled(egui::pos2(x, response.rect.center().y), 1.25, color);
        }
    }
    // The capsule is a pointer extension of the same slider, not a second
    // directional-focus target beside it. Keep arrow keys on the rail.
    let badge_response = ui.interact(badge, response.id.with("value"), egui::Sense::CLICK)
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if badge_response.clicked() {
        *index = (*index + 1) % labels.len();
        response.mark_changed();
        response.request_focus();
    }
    badge_response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button,
        ui.is_enabled(), format!("{caption}: {} → {}", labels[*index], labels[(*index + 1) % labels.len()])));
    let focused = response.has_focus() || badge_response.has_focus();
    let border = if focused { ui.visuals().selection.bg_fill }
        else { ui.visuals().widgets.noninteractive.bg_stroke.color };
    painter.rect(badge.shrink(0.5), egui::CornerRadius::same(6),
        ui.visuals().widgets.inactive.weak_bg_fill, egui::Stroke::new(1.0, border), egui::StrokeKind::Inside);
    let mut job = egui::text::LayoutJob::simple(labels[*index].clone(), font,
        ui.visuals().text_color(), (badge.width() - 16.0).max(1.0));
    job.wrap.max_rows = 1;
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let ink_center_y = if galley.mesh_bounds.is_finite() {galley.mesh_bounds.center().y}
        else {galley.rect.center().y};
    painter.with_clip_rect(badge.shrink2(egui::vec2(6.0, 0.0)))
        .galley(egui::pos2(badge.center().x - galley.size().x / 2.0,
            badge.center().y - ink_center_y), galley, ui.visuals().text_color());
    badge_response.on_hover_text(format!("{} → {}", labels[*index], labels[(*index + 1) % labels.len()]));
    response.widget_info(|| egui::WidgetInfo::slider(ui.is_enabled(), *index as f64,
        format!("{caption}: {}", labels[*index])));
    #[cfg(test)]
    ui.data_mut(|data| data.insert_temp(response.id.with("choice-geometry"), (bounds, response.rect, badge)));
    response.on_hover_text(format!("{caption}: {} ({}/{})", labels[*index], *index + 1, labels.len()))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choices() -> Vec<String> {
        ["Extra compact", "Compact", "Regular", "Comfortable", "Spacious"].map(str::to_string).to_vec()
    }

    #[test]
    fn choice_slider_allocates_its_row_only_once() {
        for horizontal in [false,true] {
            let ctx = egui::Context::default();
            ctx.run_ui(egui::RawInput::default(),|ui| {
                let layout = if horizontal {egui::Layout::left_to_right(egui::Align::Center)}
                    else {egui::Layout::top_down(egui::Align::Min)};
                ui.with_layout(layout,|ui| {
                    let response = choice_slider(ui,&mut 1,&choices(),"Control density",300.0);
                    let (bounds,_,_) = ui.data(|data| data.get_temp::<(egui::Rect,egui::Rect,egui::Rect)>(response.id.with("choice-geometry"))).unwrap();
                    let (next,_) = ui.allocate_exact_size(egui::vec2(10.0,10.0),egui::Sense::hover());
                    if horizontal {assert!((next.left()-bounds.right()-ui.spacing().item_spacing.x).abs()<0.1);}
                    else {assert!((next.top()-bounds.bottom()-ui.spacing().item_spacing.y).abs()<0.1);}
                });
            }).drop_without_applying_deltas();
        }
    }

    #[test]
    fn choice_slider_detents_match_painted_thumb_and_all_content_stays_inside() {
        for density in UiControlDensity::ALL {
            for dark in [false, true] {
                for scale in [1.0, 1.25, 2.0] {
                    for font_size in [14.0, 24.0] {
                        for width in [180.0, 260.0, 420.0] {
                            let ctx = egui::Context::default();
                            ctx.set_pixels_per_point(scale);
                            ctx.set_visuals(if dark {egui::Visuals::dark()} else {egui::Visuals::light()});
                            ctx.all_styles_mut(|style| {
                                configure(style, density);
                                for text in [egui::TextStyle::Body, egui::TextStyle::Button] {
                                    style.text_styles.insert(text, egui::FontId::proportional(font_size));
                                }
                            });
                            let mut initial = None;
                            for selected in 0..5 {
                                for state in 0..4 {
                                    let mut index = selected;
                                    let mut geometry = None;
                                    let mut output = ctx.run_ui(egui::RawInput {
                                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width+32.0, 180.0))),
                                        events: if state == 1 {vec![egui::Event::PointerMoved(egui::pos2(30.0,15.0))]}
                                            else {vec![egui::Event::PointerGone]}, ..Default::default()
                                    }, |ui| {
                                        ui.set_width(width);
                                        ui.add_enabled_ui(state != 3, |ui| {
                                            let response = choice_slider(ui, &mut index, &choices(), "Control density", width);
                                            if state == 2 { response.request_focus(); }
                                            geometry = ui.data(|data| data.get_temp::<(egui::Rect,egui::Rect,egui::Rect)>(response.id.with("choice-geometry")));
                                        });
                                    });
                                    let (bounds, rail, badge) = geometry.unwrap();
                                    assert_eq!(index, selected);
                                    assert!(bounds.contains_rect(rail) && bounds.contains_rect(badge), "{density:?}/{width}: {geometry:?}");
                                    assert!(!rail.intersects(badge));
                                    if let Some(initial) = initial { assert_eq!((bounds,rail,badge), initial); }
                                    else { initial = Some((bounds,rail,badge)); }
                                    let circles: Vec<_> = output.shapes.iter().filter_map(|shape| match &shape.shape {
                                        egui::Shape::Circle(circle) => Some(circle), _ => None,
                                    }).collect();
                                    let thumb = circles.iter().find(|circle| circle.radius > 2.0).unwrap();
                                    assert!(bounds.contains_rect(egui::Rect::from_center_size(thumb.center,
                                        egui::Vec2::splat(2.0*thumb.radius+thumb.stroke.width))));
                                    let travel = rail.x_range().shrink(rail.height()/2.5);
                                    let expected_x = egui::lerp(travel, selected as f32/4.0);
                                    assert!((thumb.center.x - expected_x).abs() < 0.01, "thumb and ticks disagree: {rail:?}");
                                    let ticks: Vec<_> = circles.iter().filter(|circle| circle.radius <= 2.0).collect();
                                    assert_eq!(ticks.len(), 4);
                                    for (tick, position) in ticks.iter().zip((0..5).filter(|position| *position != selected)) {
                                        assert!((tick.center.x - egui::lerp(travel,position as f32/4.0)).abs() < 0.01);
                                        assert_eq!(tick.center.y, thumb.center.y);
                                        assert!(rail.contains_rect(egui::Rect::from_center_size(tick.center,egui::Vec2::splat(tick.radius*2.0))));
                                    }
                                    for shape in &output.shapes {
                                        if let egui::Shape::Text(text) = &shape.shape {
                                            if text.galley.text() == choices()[selected] && text.galley.mesh_bounds.is_finite() {
                                                assert!((text.pos.y + text.galley.mesh_bounds.center().y - badge.center().y).abs() < 0.1);
                                            }
                                        }
                                    }
                                    output.textures_delta.clear();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn choice_slider_keyboard_pointer_and_disabled_behavior_is_retained() {
        let ctx = egui::Context::default();
        let mut index = 1;
        let mut rect = egui::Rect::NOTHING;
        let render = |index: &mut usize, rect: &mut egui::Rect, events, enabled| {
            ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    let response = choice_slider(ui,index,&choices(),"Control density",420.0);
                    *rect = response.rect;
                    response.request_focus();
                });
            }).drop_without_applying_deltas();
        };
        render(&mut index,&mut rect,vec![],true);
        let arrow = |key| egui::Event::Key {key,physical_key:None,pressed:true,repeat:false,modifiers:egui::Modifiers::NONE};
        render(&mut index,&mut rect,vec![arrow(egui::Key::ArrowRight)],true);
        assert_eq!(index,2);
        render(&mut index,&mut rect,vec![arrow(egui::Key::ArrowLeft)],true);
        assert_eq!(index,1);
        let point = rect.right_center() - egui::vec2(rect.height()/2.5,0.0);
        render(&mut index,&mut rect,vec![egui::Event::PointerMoved(point),egui::Event::PointerButton {
            pos:point,button:egui::PointerButton::Primary,pressed:true,modifiers:egui::Modifiers::NONE}],true);
        assert_eq!(index,4);
        render(&mut index,&mut rect,vec![egui::Event::PointerButton {
            pos:point,button:egui::PointerButton::Primary,pressed:false,modifiers:egui::Modifiers::NONE}],true);
        render(&mut index,&mut rect,vec![arrow(egui::Key::ArrowLeft)],false);
        assert_eq!(index,4);
    }

    #[test]
    fn value_capsule_cycles_immediately_wraps_and_respects_disabled_state() {
        for width in [150.0, 420.0] {
            let ctx = egui::Context::default();
            let mut index = 1;
            let mut badge = egui::Rect::NOTHING;
            let mut changed = false;
            let mut render = |index: &mut usize, events, enabled| {
                ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| {
                    ui.add_enabled_ui(enabled, |ui| {
                        let response = choice_slider(ui, index, &choices(), "Control density", width);
                        changed = response.changed();
                        badge = ui.data(|data| data.get_temp::<(egui::Rect, egui::Rect, egui::Rect)>(response.id.with("choice-geometry"))).unwrap().2;
                    });
                }).drop_without_applying_deltas();
                (badge, changed)
            };
            let (badge, _) = render(&mut index, vec![], true);
            for expected in [2, 3, 4, 0, 1] {
                for pressed in [true, false] {
                    let (_, changed) = render(&mut index, vec![egui::Event::PointerMoved(badge.center()),
                        egui::Event::PointerButton {pos: badge.center(), button: egui::PointerButton::Primary,
                            pressed, modifiers: egui::Modifiers::NONE}], true);
                    if !pressed { assert!(changed); }
                }
                assert_eq!(index, expected);
            }
            for pressed in [true, false] {
                let (_, changed) = render(&mut index, vec![egui::Event::PointerButton {pos: badge.center(),
                    button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE}], false);
                assert!(!changed);
            }
            assert_eq!(index, 1);
        }
    }

    #[test]
    fn extra_compact_restores_the_historical_eighteen_point_controls() {
        for dark in [false, true] {
            for scale in [1.0, 1.25, 1.5, 2.0] {
                let ctx = egui::Context::default();
                ctx.set_pixels_per_point(scale);
                ctx.set_visuals(if dark { egui::Visuals::dark() } else { egui::Visuals::light() });
                ctx.all_styles_mut(|style| configure(style, UiControlDensity::ExtraCompact));
                let mut value = "Compact".to_owned();
                let mut previous = None;
                for hovered in [false, true, false] {
                    ctx.run_ui(egui::RawInput {
                        events: if hovered { vec![egui::Event::PointerMoved(egui::pos2(40.0, 10.0))] }
                            else { vec![egui::Event::PointerGone] },
                        ..Default::default()
                    }, |ui| {
                        assert_eq!(ui.spacing().interact_size.y, 18.0);
                        assert_eq!(ui.spacing().button_padding.y, 1.0);
                        assert_eq!(height(ui), 18.0);
                        let output = crate::ui::dialog::singleline_text_edit(ui, &mut value).show(ui);
                        assert_eq!(output.response.rect.height(), 18.0);
                        assert!((output.galley_pos.y + output.galley.size().y / 2.0
                            - output.response.rect.center().y).abs() <= 1.0);
                        let geometry = (output.response.rect, output.galley_pos);
                        if let Some(previous) = previous { assert_eq!(geometry, previous); }
                        previous = Some(geometry);
                    }).drop_without_applying_deltas();
                }
            }
        }
    }

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
