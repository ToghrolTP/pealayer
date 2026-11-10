//! Shortcut presentation without replacing TextEdit's editing/accessibility.
use eframe::egui;

fn keys(value: &str) -> Vec<&str> {
    // A terminal literal '+' is a key, not two empty keycaps. Punctuation such
    // as the comma in Ctrl+, must remain intact; this never rewrites bindings.
    let literal_plus = value.trim_end().ends_with("++");
    let mut keys = value.split('+').map(str::trim).filter(|part| !part.is_empty()).collect::<Vec<_>>();
    if literal_plus { keys.push("+"); }
    keys
}

pub fn paint(ui: &egui::Ui, rect: egui::Rect, value: &str, rtl: bool) {
    let clip = ui.clip_rect().intersect(rect.shrink2(egui::vec2(5.0, 1.0)));
    let painter = ui.painter().with_clip_rect(clip);
    let height = (rect.height() - 6.0).max(10.0);
    let mut font = egui::TextStyle::Button.resolve(ui.style());
    font.size = (font.size * 0.9).min(height - 2.0);
    let color = ui.visuals().text_color();
    let muted = ui.visuals().weak_text_color();
    let galleys = keys(value).into_iter().map(|key| painter.layout_no_wrap(key.into(), font.clone(), color)).collect::<Vec<_>>();
    let widths = galleys.iter().map(|galley| (galley.size().x + 12.0).max(20.0)).collect::<Vec<_>>();
    let separator = painter.layout_no_wrap("+".into(), font.clone(), muted);
    let gap = separator.size().x + 8.0;
    let width = widths.iter().sum::<f32>() + gap * widths.len().saturating_sub(1) as f32;
    let overflow = width > clip.width();
    // Chords remain LTR; only their position in an RTL field is mirrored.
    let mut x = if rtl && !overflow { clip.right() - width } else { clip.left() };
    let end = clip.right() - if overflow { 18.0 } else { 0.0 };
    for (index, (galley, width)) in galleys.into_iter().zip(widths).enumerate() {
        if x + width > end { break; }
        if index > 0 {
            let ink = separator.mesh_bounds;
            painter.galley(egui::pos2(x - gap * 0.5 - ink.center().x, rect.center().y - ink.center().y), separator.clone(), muted);
        }
        let cap = egui::Rect::from_min_size(egui::pos2(x, rect.center().y - height * 0.5), egui::vec2(width, height));
        painter.rect(cap, 3.0, ui.visuals().widgets.inactive.bg_fill,
            ui.visuals().widgets.noninteractive.bg_stroke, egui::StrokeKind::Inside);
        // A restrained lower lip suggests a physical key without a heavy font.
        painter.line_segment([cap.left_bottom() + egui::vec2(3.0, -1.5), cap.right_bottom() + egui::vec2(-3.0, -1.5)],
            egui::Stroke::new(1.0, ui.visuals().widgets.inactive.bg_stroke.color));
        let ink = if galley.mesh_bounds.is_finite() { galley.mesh_bounds } else { galley.rect };
        painter.galley(cap.center() - ink.center().to_vec2(), galley, color);
        x += width + gap;
    }
    if overflow {
        painter.text(egui::pos2(clip.right(), rect.center().y), egui::Align2::RIGHT_CENTER, "…", font, muted);
    }
}

/// Resting values use keycaps inside the real input frame. Focus restores the
/// unmodified text editor, retaining keyboard, paste, IME and screen-reader
/// semantics. Caps are decorative, never separate focusable fake buttons.
pub fn text_edit(ui: &mut egui::Ui, id: egui::Id, text: &mut String,
    width: f32, hint: &str, rtl: bool) -> egui::Response {
    let editing = ui.memory(|memory| memory.has_focus(id));
    let show_caps = !editing && !text.trim().is_empty();
    let mut editor = crate::ui::dialog::singleline_text_edit(ui, text)
        .id(id).desired_width(width).hint_text(hint)
        .align(if rtl { egui::Align2::RIGHT_CENTER } else { egui::Align2::LEFT_CENTER });
    if show_caps { editor = editor.text_color(egui::Color32::TRANSPARENT); }
    let response = ui.add(editor);
    if show_caps { paint(ui, response.rect, text, rtl); }
    response.on_hover_text(text.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keycaps_preserve_modifier_and_punctuation_labels() {
        assert_eq!(keys("F11"), ["F11"]);
        assert_eq!(keys(" Ctrl + Shift + F10 "), ["Ctrl", "Shift", "F10"]);
        assert_eq!(keys("Ctrl+,"), ["Ctrl", ","]);
        assert_eq!(keys("Ctrl++"), ["Ctrl", "+"]);
        assert!(keys("").is_empty());
    }

    fn frame(ctx: &egui::Context, text: &mut String, events: Vec<egui::Event>) -> (egui::FullOutput, egui::Response) {
        let mut response = None;
        let mut output = ctx.run_ui(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO, egui::vec2(500.0, 200.0))), events, ..Default::default() }, |ui| {
            response = Some(text_edit(ui, egui::Id::unique("shortcut-test"), text, 240.0, "Shortcut", false));
        });
        output.textures_delta.clear();
        (output, response.unwrap())
    }

    #[test]
    fn keycap_ink_is_centered_and_clipped_across_density_palette_and_direction() {
        use crate::config::{ColorPalette, UiControlDensity};
        for density in [UiControlDensity::ExtraCompact, UiControlDensity::Compact, UiControlDensity::Regular,
            UiControlDensity::Comfortable, UiControlDensity::Spacious] {
            for palette in [ColorPalette::Native, ColorPalette::Studio] { for dark in [false, true] {
                for rtl in [false, true] { for size in [14.0, 22.0] { for width in [120.0, 300.0] {
                    let ctx = egui::Context::default();
                    ctx.set_pixels_per_point(1.25);
                    let theme = if dark { egui::Theme::Dark } else { egui::Theme::Light };
                    ctx.set_theme(theme);
                    ctx.style_mut_of(theme, |style| {
                        crate::ui::palette::apply(&mut style.visuals, palette);
                        style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(size));
                        crate::ui::control_density::configure(style, density);
                    });
                    let mut value = "Ctrl+Shift+F10".to_owned();
                    for _ in 0..3 {
                        let mut bounds = egui::Rect::NOTHING;
                        let mut output = ctx.run_ui(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO, egui::vec2(600.0, 300.0))), ..Default::default() }, |ui| {
                            let response = text_edit(ui, egui::Id::unique("geometry"), &mut value, width, "Shortcut", rtl);
                            bounds = response.rect;
                            assert!(!response.changed());
                        });
                        output.textures_delta.clear();
                        let caps = output.shapes.iter().filter_map(|s| match &s.shape {
                            egui::Shape::Rect(r) if r.rect.height() < bounds.height() - 2.0 => Some(r.rect), _ => None,
                        }).collect::<Vec<_>>();
                        assert!(!caps.is_empty());
                        for cap in caps { assert!(bounds.contains_rect(cap)); }
                        for shape in &output.shapes {
                            if let egui::Shape::Text(text) = &shape.shape {
                                if matches!(text.galley.text(), "Ctrl" | "Shift" | "F10" | "+") {
                                    let ink = text.galley.mesh_bounds.translate(text.pos.to_vec2());
                                    assert!((ink.center().y - bounds.center().y).abs() < 0.1,
                                        "keycap ink must use a shared centerline: {ink:?}, {bounds:?}");
                                    assert!(bounds.contains_rect(shape.clip_rect));
                                }
                            }
                        }
                        assert_eq!(value, "Ctrl+Shift+F10");
                    }
                }}}
            }}
        }
    }

    #[test]
    fn keycap_editor_preserves_real_focus_typing_paste_and_changed_flags() {
        let ctx = egui::Context::default();
        let mut value = "Ctrl+Shift+F10".to_owned();
        let (_, response) = frame(&ctx, &mut value, vec![]);
        let slot = response.rect;
        assert!(!response.changed());
        let pos = slot.center();
        frame(&ctx, &mut value, vec![egui::Event::PointerMoved(pos), egui::Event::PointerButton {
            pos, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE }]);
        frame(&ctx, &mut value, vec![egui::Event::PointerButton {
            pos, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE }]);
        let (_, focused) = frame(&ctx, &mut value, vec![]);
        assert!(focused.has_focus());
        assert_eq!(focused.rect, slot);
        assert!(!focused.changed());
        let select = egui::Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false,
            modifiers: egui::Modifiers::COMMAND };
        frame(&ctx, &mut value, vec![select]);
        let (copied, _) = frame(&ctx, &mut value, vec![egui::Event::Copy]);
        assert!(copied.platform_output.commands.iter().any(|command|
            matches!(command, egui::OutputCommand::CopyText(text) if text == "Ctrl+Shift+F10")));
        let (_, pasted) = frame(&ctx, &mut value, vec![egui::Event::Paste("Alt+F11".into())]);
        assert_eq!(value, "Alt+F11");
        assert!(pasted.changed());
        ctx.memory_mut(|m| m.surrender_focus(response.id));
        let (output, blurred) = frame(&ctx, &mut value, vec![]);
        assert_eq!(blurred.rect, slot);
        assert!(!blurred.changed());
        assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Alt")));
    }
}
