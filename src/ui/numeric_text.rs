//! Proportional UI lettering with tabular digit advances, independent of a
//! fixed-pitch face. egui's current TextFormat exposes variation axes but not
//! OpenType feature selection; do not pretend a `tnum` flag was applied.
use eframe::egui;
use std::sync::Arc;

pub(crate) fn layout(
    ui: &egui::Ui,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
) -> Arc<egui::Galley> {
    // Measure the shaped section, not the font's raw glyph advance. CoreText
    // system faces can shape a different advance (including variation metrics),
    // which otherwise left a fractional drift despite equal raw-width padding.
    let advances: [f32;10] = std::array::from_fn(|digit| ui.fonts_mut(|fonts| {
        let shaped=fonts.layout_no_wrap(char::from(b'0'+digit as u8).to_string(),font.clone(),color);
        shaped.rows.first().and_then(|row|row.glyphs.first()).map_or(0.0,|glyph|glyph.advance_width)
    }));
    let scale=ui.ctx().pixels_per_point();
    // A cell ends on the physical grid. Measuring a rounded Galley width, or
    // repeatedly accumulating fractional cell ends, can move the following word
    // across a rounding boundary on CoreText and at fractional Windows scales.
    let digit_width = (advances.iter().copied().fold(0.0_f32,f32::max)*scale).ceil()/scale;
    let mut job = egui::text::LayoutJob::default();
    let format = egui::TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    };
    let mut pending = 0.0;
    let mut run = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            if !run.is_empty() {
                job.append(&run, pending, format.clone());
                run.clear();
                pending = 0.0;
            }
            let advance = advances[(ch as u8-b'0') as usize];
            let pad = (digit_width - advance).max(0.0) * 0.5;
            // Explicit section boundaries avoid kerning across numeric cells;
            // ordinary words remain whole shaped runs, including RTL scripts.
            let start = job.text.len();
            job.text.push(ch);
            job.sections.push(egui::text::LayoutSection {
                leading_space: pending + pad,
                byte_range: egui::text::ByteIndex(start)..egui::text::ByteIndex(job.text.len()),
                format: format.clone(),
            });
            pending = pad;
        } else {
            run.push(ch);
        }
    }
    job.append(&run, pending, format);
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

/// Numeric baselines use one stable cap/digit reference, not the current
/// caption's descenders or changing digit overshoot. Icons share this center.
pub(crate) fn center_y(ui: &egui::Ui, font: egui::FontId) -> f32 {
    let reference = ui
        .painter()
        .layout_no_wrap("0123456789".into(), font, egui::Color32::WHITE);
    reference.mesh_bounds.center().y
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn digits_are_tabular_but_letters_remain_proportional_and_text_is_preserved() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(scale);
            crate::ui::fonts::install(&ctx, &Default::default(), false);
            let mut output = ctx.run_ui(Default::default(), |ui| {
                let font = egui::FontId::new(12.0, crate::ui::fonts::status_family(ui));
                let samples: Vec<_> = ["111.11 fps", "999.99 fps", "000.00 fps", "888.88 fps"]
                    .iter()
                    .map(|text| layout(ui, text, font.clone(), egui::Color32::WHITE))
                    .collect();
                for (text, galley) in ["111.11 fps", "999.99 fps", "000.00 fps", "888.88 fps"]
                    .iter()
                    .zip(&samples)
                {
                    assert_eq!(galley.text(), *text);
                    assert!(
                        (galley.size().x - samples[0].size().x).abs() * scale < 0.1,
                        "numeric advance changed: {text} {:?} vs {:?}",
                        galley.size(),
                        samples[0].size()
                    );
                }
                assert!(
                    layout(ui, "WW", font.clone(), egui::Color32::WHITE)
                        .size()
                        .x
                        > layout(ui, "ii", font, egui::Color32::WHITE).size().x * 1.5
                );
            });
            output.textures_delta.clear();
        }
    }
}
