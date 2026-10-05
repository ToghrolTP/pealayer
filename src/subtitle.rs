use serde::{Deserialize, Serialize};

pub const SUBTITLE_FONT_FAMILY: &str = "Vazirmatn";

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubtitleDirection {
    #[default]
    Auto,
    Ltr,
    Rtl,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct SubtitleReplacement {
    pub from: String,
    pub to: String,
}

impl SubtitleReplacement {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

impl Default for SubtitleReplacement {
    fn default() -> Self {
        Self::new(String::new(), String::new())
    }
}

pub fn default_text_replacements() -> Vec<SubtitleReplacement> {
    vec![
        SubtitleReplacement::new("ي", "ی"),
        SubtitleReplacement::new("ك", "ک"),
    ]
}

pub fn apply_text_replacements(text: &str, replacements: &[SubtitleReplacement]) -> String {
    replacements
        .iter()
        .filter(|replacement| !replacement.from.is_empty())
        .fold(text.to_owned(), |text, replacement| {
            text.replace(&replacement.from, &replacement.to)
        })
}

/// Decide once from the selected rendering policy, never from the contents of
/// an individual subtitle event. Basing this decision on the current line made
/// the renderer alternate between Pealayer's Vazirmatn overlay and mpv's native
/// ASS style whenever only some lines contained replaceable Persian glyphs.
pub fn requires_processed_overlay(
    direction: SubtitleDirection,
    replacements: &[SubtitleReplacement],
) -> bool {
    direction != SubtitleDirection::Auto
        || replacements
            .iter()
            .any(|replacement| !replacement.from.is_empty())
}

fn escape_ass_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace("\r\n", "\\N")
        .replace('\r', "\\N")
        .replace('\n', "\\N")
}

/// Build the single ASS event used by Pealayer's processed subtitle overlay.
/// libass still performs glyph shaping; the explicit Unicode embeddings and
/// edge alignment only control the requested paragraph direction/layout.
pub fn overlay_ass_event(
    text: &str,
    direction: SubtitleDirection,
    font_size: f64,
    position_percent: f64,
) -> String {
    let escaped = escape_ass_text(text);
    let (alignment, x, directed) = match direction {
        SubtitleDirection::Auto => (2, 640, escaped),
        SubtitleDirection::Ltr => (1, 32, format!("\u{202A}{escaped}\u{202C}")),
        SubtitleDirection::Rtl => (3, 1248, format!("\u{202B}{escaped}\u{202C}")),
    };
    let font_size = font_size.clamp(10.0, 100.0);
    // Keep a small safe-area at both edges while mapping mpv's familiar
    // 0% (top) .. 100% (bottom) subtitle-position contract into ASS space.
    let y = 40.0 + position_percent.clamp(0.0, 100.0) * 6.4;
    format!(
        "{{\\an{alignment}\\pos({x},{y:.0})\\q2\\fn{SUBTITLE_FONT_FAMILY}\\fs{font_size:.1}\\1c&HFFFFFF&\\3c&H000000&\\bord2\\shad0}}{directed}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persian_defaults_replace_arabic_code_points_in_order() {
        assert_eq!(
            apply_text_replacements("جيك", &default_text_replacements()),
            "جیک"
        );
    }

    #[test]
    fn empty_sources_are_ignored_instead_of_expanding_every_boundary() {
        let replacements = [SubtitleReplacement::new("", "x")];
        assert_eq!(apply_text_replacements("caption", &replacements), "caption");
    }

    #[test]
    fn processed_overlay_policy_is_stable_across_subtitle_lines() {
        let replacements = default_text_replacements();
        assert!(requires_processed_overlay(
            SubtitleDirection::Auto,
            &replacements
        ));
        assert!(requires_processed_overlay(SubtitleDirection::Rtl, &[]));
        assert!(!requires_processed_overlay(SubtitleDirection::Auto, &[]));

        // The previous content-dependent test disagreed for these two lines,
        // causing a visible font swap. Renderer selection now ignores text.
        assert_eq!(apply_text_replacements("Hello", &replacements), "Hello");
        assert_ne!(apply_text_replacements("كي", &replacements), "كي");
        assert!(requires_processed_overlay(
            SubtitleDirection::Auto,
            &replacements
        ));
    }

    #[test]
    fn overlay_uses_requested_direction_and_preserves_multiline_text() {
        let rtl = overlay_ass_event("خط یک\nخط دو", SubtitleDirection::Rtl, 55.0, 100.0);
        assert!(rtl.contains("\\an3\\pos(1248,680)"));
        assert!(rtl.contains(&format!("\\fn{SUBTITLE_FONT_FAMILY}")));
        assert!(rtl.contains('\u{202B}'));
        assert!(rtl.contains("\\N"));
        assert!(rtl.ends_with('\u{202C}'));
    }

    #[test]
    fn overlay_position_uses_safe_area_and_clamps() {
        assert!(
            overlay_ass_event("Top", SubtitleDirection::Auto, 55.0, -10.0)
                .contains("\\pos(640,40)")
        );
        assert!(
            overlay_ass_event("Bottom", SubtitleDirection::Auto, 55.0, 150.0)
                .contains("\\pos(640,680)")
        );
    }
}
