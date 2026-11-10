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
    let digit_width = ui.fonts_mut(|fonts| {
        ('0'..='9')
            .map(|ch| fonts.glyph_width(&font, ch))
            .fold(0.0_f32, f32::max)
    });
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
            let advance = ui.fonts_mut(|fonts| fonts.glyph_width(&font, ch));
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
