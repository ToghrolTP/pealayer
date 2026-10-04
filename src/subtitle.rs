use serde::{Deserialize, Serialize};

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
pub fn overlay_ass_event(text: &str, direction: SubtitleDirection, font_size: f64) -> String {
    let escaped = escape_ass_text(text);
    let (alignment, x, directed) = match direction {
        SubtitleDirection::Auto => (2, 640, escaped),
        SubtitleDirection::Ltr => (1, 32, format!("\u{202A}{escaped}\u{202C}")),
        SubtitleDirection::Rtl => (3, 1248, format!("\u{202B}{escaped}\u{202C}")),
    };
    let font_size = font_size.clamp(10.0, 100.0);
    format!(
        "{{\\an{alignment}\\pos({x},680)\\q2\\fnVazirmatn\\fs{font_size:.1}\\1c&HFFFFFF&\\3c&H000000&\\bord2\\shad0}}{directed}"
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
    fn overlay_uses_requested_direction_and_preserves_multiline_text() {
        let rtl = overlay_ass_event("خط یک\nخط دو", SubtitleDirection::Rtl, 55.0);
        assert!(rtl.contains("\\an3\\pos(1248,680)"));
        assert!(rtl.contains('\u{202B}'));
        assert!(rtl.contains("\\N"));
        assert!(rtl.ends_with('\u{202C}'));
    }
}
