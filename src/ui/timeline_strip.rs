//! Palette-owned media strips and horizontally pinned, clip-bounded captions.
use crate::config::ColorPalette;
use eframe::egui::{self, Color32, Rect};

#[derive(Clone, Copy)]
pub(super) enum MediaKind {
    Video,
    Audio,
    Subtitle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MediaStyle {
    fill: Color32,
    fill_end: Option<Color32>,
    edge: Color32,
    ink: Color32,
    accent: Color32,
}
impl MediaStyle {
    fn resolve(palette: ColorPalette, dark: bool, kind: MediaKind) -> Self {
        let prefix = match kind {
            MediaKind::Video => "video",
            MediaKind::Audio => "audio",
            MediaKind::Subtitle => "subtitle",
        };
        let color =
            |part| super::palette::color(palette, dark, &format!("timeline-{prefix}-{part}"));
        Self {
            fill: color("fill"),
            fill_end: (palette == ColorPalette::Pastel).then(|| color("fill-end")),
            edge: color("edge"),
            ink: color("ink"),
            accent: color("accent"),
        }
    }
}

/// This changes paint only, never the source strip's extent or hit targets.
pub(super) fn paint_media(
    painter: &egui::Painter,
    rect: Rect,
    kind: MediaKind,
    title: String,
    palette: ColorPalette,
    dark: bool,
    active: bool,
    enabled: bool,
    rtl: bool,
) {
    let style = MediaStyle::resolve(palette, dark, kind);
    let opacity = if active && enabled { 1.0 } else { 0.52 };
    if let Some(end) = style.fill_end {
        super::palette::paint_gradient(painter, rect, 4, style.fill, end, opacity);
    } else {
        painter.rect_filled(rect, 4.0, style.fill.gamma_multiply(opacity));
    }
    painter.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(
            1.0,
            style
                .edge
                .gamma_multiply(if palette != ColorPalette::Native {
                    opacity
                } else {
                    1.0
                }),
        ),
        egui::StrokeKind::Inside,
    );
    if palette != ColorPalette::Native && rect.width() > 10.0 {
        let cap = Rect::from_min_max(
            rect.min + egui::vec2(4.0, 1.0),
            egui::pos2(rect.right() - 4.0, rect.top() + 2.0),
        );
        painter.rect_filled(cap, 1.0, style.accent.gamma_multiply(0.7 * opacity));
    }
    paint_label(
        painter,
        rect,
        title,
        egui::FontId::proportional(11.0),
        style.ink.gamma_multiply(if enabled { 1.0 } else { 0.68 }),
        true,
        10.0,
        10.0,
        rtl,
    );
}

fn label_bounds(rect: Rect, viewport: Rect, left: f32, right: f32) -> Option<Rect> {
    if !rect.is_finite() || !viewport.is_finite() {
        return None;
    }
    let visible = rect.intersect(viewport);
    let content = Rect::from_min_max(
        egui::pos2(visible.left() + left, rect.top() + 2.0),
        egui::pos2(visible.right() - right, rect.bottom() - 2.0),
    );
    (content.width() >= 2.0 && content.height() >= 2.0 && visible.is_positive()).then_some(content)
}

/// Anchor to the visible portion, not the scrolled-away source start. Elide
/// only when that portion cannot fit the full caption. Keep explicit cue
/// overflow opt-in and normal vertical row clipping intact.
pub(super) fn paint_label(
    painter: &egui::Painter,
    rect: Rect,
    title: String,
    font: egui::FontId,
    color: Color32,
    hide_overflow: bool,
    left: f32,
    right: f32,
    rtl: bool,
) {
    let Some(content) = label_bounds(rect, painter.clip_rect(), left, right) else {
        return;
    };
    let galley = if hide_overflow {
        let mut job = egui::text::LayoutJob {
            wrap: egui::text::TextWrapping::truncate_at_width(content.width()),
            ..Default::default()
        };
        job.append(&title, 0.0, egui::TextFormat::simple(font, color));
        painter.layout_job(job)
    } else {
        painter.layout_no_wrap(title, font, color)
    };
    let position = egui::pos2(
        if rtl {
            content.right() - galley.size().x
        } else {
            content.left()
        },
        rect.center().y - galley.size().y / 2.0,
    );
    if hide_overflow {
        painter
            .with_clip_rect(content.intersect(painter.clip_rect()))
            .galley(position, galley, color);
    } else {
        painter.galley(position, galley, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_and_disabled_strip_paint_preserves_native_borders_and_semantic_dimming() {
        for palette in [ColorPalette::Native, ColorPalette::Studio, ColorPalette::Pastel] {
            for active in [false, true] {
                for enabled in [false, true] {
                    let ctx = egui::Context::default();
                    let rect = Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(400.0, 24.0));
                    let style = MediaStyle::resolve(palette, true, MediaKind::Audio);
                    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                        paint_media(
                            ui.painter(),
                            rect,
                            MediaKind::Audio,
                            "Audio".into(),
                            palette,
                            true,
                            active,
                            enabled,
                            false,
                        );
                    });
                    output.textures_delta.clear();
                    let opacity = if active && enabled { 1.0 } else { 0.52 };
                    let gradient = palette == ColorPalette::Pastel;
                    let fill = if gradient { Color32::WHITE } else { style.fill };
                    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
                        egui::Shape::Rect(r) if r.rect == rect && r.brush.is_some() == gradient
                            && r.fill == fill.gamma_multiply(opacity))));
                    let edge = if palette == ColorPalette::Native {
                        Color32::WHITE
                    } else {
                        style.edge.gamma_multiply(opacity)
                    };
                    assert!(output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Rect(r) if r.rect==rect && r.stroke.color==edge && r.stroke.width==1.0)));
                    let has_cap=output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Rect(r) if r.rect.height()==1.0 && r.fill==style.accent.gamma_multiply(0.7*opacity)));
                    assert_eq!(has_cap, palette != ColorPalette::Native);
                }
            }
        }
    }

    #[test]
    fn studio_and_pastel_media_roles_are_readable_and_preserve_neutral() {
        let luminance = |color: Color32| {
            let linear = |value: u8| {
                let s = value as f64 / 255.0;
                if s <= 0.04045 {
                    s / 12.92
                } else {
                    ((s + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
        };
        for dark in [false, true] {
            for (kind, original) in [
                (MediaKind::Video, Color32::from_rgb(41, 128, 185)),
                (MediaKind::Audio, Color32::from_rgb(39, 174, 96)),
                (MediaKind::Subtitle, Color32::from_rgb(124, 92, 190)),
            ] {
                let native = MediaStyle::resolve(ColorPalette::Native, dark, kind);
                assert_eq!(native.fill, original);
                assert_eq!(native.edge, Color32::WHITE);
                let studio = MediaStyle::resolve(ColorPalette::Studio, dark, kind);
                assert_ne!(studio.fill, native.fill);
                assert_ne!(studio.edge, Color32::WHITE);
                let a = luminance(studio.fill);
                let b = luminance(studio.ink);
                assert!(
                    (a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5,
                    "Unreadable Studio caption: {studio:?}"
                );
                assert_ne!(
                    studio,
                    MediaStyle::resolve(ColorPalette::Studio, !dark, kind)
                );
                let pastel = MediaStyle::resolve(ColorPalette::Pastel, dark, kind);
                let end = pastel.fill_end.expect("Pastel gradient endpoint");
                assert_ne!(end, pastel.fill);
                for stop in [pastel.fill, end] {
                    let a = luminance(stop);
                    let b = luminance(pastel.ink);
                    assert!((a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5,
                        "Unreadable Pastel caption: {pastel:?}");
                }
            }
        }
    }

    #[test]
    fn media_names_stay_inside_the_visible_strip_across_pan_scale_direction_and_theme() {
        for rtl in [false, true] {
            for scale in [1.0, 1.25, 1.5, 2.0] {
                for dark in [false, true] {
                    for width in [180.0, 640.0, 1200.0] {
                        let ctx = egui::Context::default();
                        ctx.set_pixels_per_point(scale);
                        crate::ui::configure_native_appearance(
                            &ctx,
                            &crate::config::AppConfig {
                                color_palette: ColorPalette::Studio,
                                theme: if dark {
                                    crate::config::AppTheme::Dark
                                } else {
                                    crate::config::AppTheme::Light
                                },
                                ..Default::default()
                            },
                        );
                        for (start, end, should_elide) in [
                            (-2000.0, 3000.0, false),
                            (30.0, 3000.0, false),
                            (-2000.0, 60.0, true),
                            (25.0, 62.0, true),
                        ] {
                            let viewport = Rect::from_min_size(
                                egui::pos2(20.0, 20.0),
                                egui::vec2(width, 100.0),
                            );
                            let mut output = ctx.run_ui(
                                egui::RawInput {
                                    screen_rect: Some(Rect::from_min_size(
                                        egui::Pos2::ZERO,
                                        egui::vec2(width + 40.0, 200.0),
                                    )),
                                    ..Default::default()
                                },
                                |ui| {
                                    let painter = ui.painter().with_clip_rect(viewport);
                                    for (index, kind) in
                                        [MediaKind::Video, MediaKind::Audio, MediaKind::Subtitle]
                                            .into_iter()
                                            .enumerate()
                                    {
                                        let y = 24.0 + index as f32 * 28.0;
                                        let rect = Rect::from_min_max(
                                            egui::pos2(start, y),
                                            egui::pos2(end, y + 24.0),
                                        );
                                        paint_media(
                                            &painter,
                                            rect,
                                            kind,
                                            "Media strip caption".into(),
                                            ColorPalette::Studio,
                                            dark,
                                            true,
                                            true,
                                            rtl,
                                        );
                                    }
                                },
                            );
                            output.textures_delta.clear();
                            let labels: Vec<_> = output
                                .shapes
                                .iter()
                                .filter_map(|shape| match &shape.shape {
                                    egui::Shape::Text(text) => Some((text, shape.clip_rect)),
                                    _ => None,
                                })
                                .collect();
                            assert_eq!(labels.len(), 3);
                            for (text, clip) in labels {
                                let text_rect = Rect::from_min_size(text.pos, text.galley.size());
                                assert_eq!(
                                    text.galley.elided, should_elide,
                                    "start={start}, end={end}, width={width}, rtl={rtl}"
                                );
                                assert!(
                                    clip.left() >= viewport.left()
                                        && clip.right() <= viewport.right()
                                );
                                assert!(
                                    text_rect.left() >= clip.left() - 0.1
                                        && text_rect.right() <= clip.right() + 0.1,
                                    "Pinned text escaped: {text_rect:?} {clip:?}"
                                );
                                if start < viewport.left() && !should_elide && !rtl {
                                    assert!((text.pos.x - viewport.left() - 10.0).abs() < 0.1);
                                }
                                if end > viewport.right() && !should_elide && rtl {
                                    assert!(
                                        (text_rect.right() - viewport.right() + 10.0).abs() < 0.1
                                    );
                                }
                            }
                            output.textures_delta.clear();
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn short_or_offscreen_strips_never_leak_labels_and_explicit_overflow_is_preserved() {
        let ctx = egui::Context::default();
        let viewport = Rect::from_min_max(egui::pos2(100.0, 20.0), egui::pos2(400.0, 100.0));
        assert!(label_bounds(
            Rect::from_min_max(egui::pos2(40.0, 30.0), egui::pos2(90.0, 60.0)),
            viewport,
            10.0,
            10.0
        )
        .is_none());
        assert!(label_bounds(
            Rect::from_min_max(egui::pos2(390.0, 30.0), egui::pos2(600.0, 60.0)),
            viewport,
            10.0,
            10.0
        )
        .is_none());
        for hide in [false, true] {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                paint_label(
                    &ui.painter().with_clip_rect(viewport),
                    Rect::from_min_max(egui::pos2(-800.0, 30.0), egui::pos2(180.0, 60.0)),
                    "A deliberately long effect caption".into(),
                    egui::FontId::proportional(11.0),
                    Color32::WHITE,
                    hide,
                    12.0,
                    12.0,
                    false,
                );
            });
            output.textures_delta.clear();
            let text = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text),
                    _ => None,
                })
                .unwrap();
            assert_eq!(text.galley.elided, hide);
            assert!(
                (text.pos.x - 112.0).abs() < 0.1,
                "Even explicit overflow must pin its source label"
            );
            output.textures_delta.clear();
        }
    }
}
