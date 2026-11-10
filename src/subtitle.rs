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

/// Horizontal placement is independent from Unicode paragraph direction.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubtitleAlignment {
    Left,
    #[default]
    Center,
    Right,
    /// Preserve native subtitle styling when no text processing is requested.
    SubtitleStyle,
}

impl SubtitleAlignment {
    pub fn mpv_value(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center | Self::SubtitleStyle => "center",
            Self::Right => "right",
        }
    }

    fn ass_anchor(self) -> (u8, u16) {
        match self {
            Self::Left => (1, 32),
            Self::Center | Self::SubtitleStyle => (2, 640),
            Self::Right => (3, 1248),
        }
    }
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
    alignment: SubtitleAlignment,
    replacements: &[SubtitleReplacement],
) -> bool {
    alignment != SubtitleAlignment::SubtitleStyle
        || direction != SubtitleDirection::Auto
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
/// libass still performs glyph shaping. Unicode embeddings control paragraph
/// direction; placement independently controls horizontal alignment.
pub fn overlay_ass_event(
    text: &str,
    direction: SubtitleDirection,
    alignment: SubtitleAlignment,
    font_size: f64,
    position_percent: f64,
) -> String {
    let escaped = escape_ass_text(text);
    let (alignment, x) = alignment.ass_anchor();
    let directed = match direction {
        SubtitleDirection::Auto => escaped,
        SubtitleDirection::Ltr => format!("\u{202A}{escaped}\u{202C}"),
        SubtitleDirection::Rtl => format!("\u{202B}{escaped}\u{202C}"),
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
            SubtitleAlignment::Center,
            &replacements
        ));
        assert!(requires_processed_overlay(
            SubtitleDirection::Rtl,
            SubtitleAlignment::Center,
            &[]
        ));
        assert!(!requires_processed_overlay(
            SubtitleDirection::Auto,
            SubtitleAlignment::SubtitleStyle,
            &[]
        ));

        // The previous content-dependent test disagreed for these two lines,
        // causing a visible font swap. Renderer selection now ignores text.
        assert_eq!(apply_text_replacements("Hello", &replacements), "Hello");
        assert_ne!(apply_text_replacements("كي", &replacements), "كي");
        assert!(requires_processed_overlay(
            SubtitleDirection::Auto,
            SubtitleAlignment::Center,
            &replacements
        ));
    }

    #[test]
    fn overlay_uses_requested_direction_and_preserves_multiline_text() {
        let rtl = overlay_ass_event(
            "خط یک\nخط دو",
            SubtitleDirection::Rtl,
            SubtitleAlignment::Center,
            55.0,
            100.0,
        );
        assert!(rtl.contains("\\an2\\pos(640,680)"));
        assert!(rtl.contains(&format!("\\fn{SUBTITLE_FONT_FAMILY}")));
        assert!(rtl.contains('\u{202B}'));
        assert!(rtl.contains("\\N"));
        assert!(rtl.ends_with('\u{202C}'));
    }

    #[test]
    fn overlay_position_uses_safe_area_and_clamps() {
        assert!(
            overlay_ass_event(
                "Top",
                SubtitleDirection::Auto,
                SubtitleAlignment::Center,
                55.0,
                -10.0
            )
            .contains("\\pos(640,40)")
        );
        assert!(
            overlay_ass_event(
                "Bottom",
                SubtitleDirection::Auto,
                SubtitleAlignment::Center,
                55.0,
                150.0
            )
            .contains("\\pos(640,680)")
        );
    }

    #[test]
    fn subtitle_alignment_is_enforced_independently_of_direction() {
        for direction in [
            SubtitleDirection::Auto,
            SubtitleDirection::Ltr,
            SubtitleDirection::Rtl,
        ] {
            for (alignment, anchor, x) in [
                (SubtitleAlignment::Left, 1, 32),
                (SubtitleAlignment::Center, 2, 640),
                (SubtitleAlignment::Right, 3, 1248),
            ] {
                let event = overlay_ass_event("Hello\nسلام", direction, alignment, 55.0, 100.0);
                assert!(
                    event.contains(&format!("\\an{anchor}\\pos({x},680)")),
                    "{direction:?}/{alignment:?}: {event}"
                );
                assert!(
                    requires_processed_overlay(direction, alignment, &[]),
                    "explicit alignment must override text-track positioning"
                );
                assert_eq!(
                    event.contains('\u{202A}'),
                    direction == SubtitleDirection::Ltr
                );
                assert_eq!(
                    event.contains('\u{202B}'),
                    direction == SubtitleDirection::Rtl
                );
            }
        }
    }

    #[test]
    fn subtitle_alignment_defaults_center_and_roundtrips_without_changing_direction() {
        let old: crate::config::AppConfig =
            serde_json::from_str(r#"{"subtitle_direction":"rtl"}"#).unwrap();
        assert_eq!(old.subtitle_alignment, SubtitleAlignment::Center);
        assert_eq!(old.subtitle_direction, SubtitleDirection::Rtl);
        for alignment in [
            SubtitleAlignment::Left,
            SubtitleAlignment::Center,
            SubtitleAlignment::Right,
            SubtitleAlignment::SubtitleStyle,
        ] {
            let config = crate::config::AppConfig {
                subtitle_alignment: alignment,
                ..old.clone()
            };
            let reloaded: crate::config::AppConfig =
                serde_json::from_value(serde_json::to_value(config).unwrap()).unwrap();
            assert_eq!(reloaded.subtitle_alignment, alignment);
            assert_eq!(reloaded.subtitle_direction, SubtitleDirection::Rtl);
        }
        assert!(
            serde_json::from_str::<crate::config::AppConfig>(r#"{"subtitle_alignment":"rtl"}"#)
                .is_err()
        );
    }

    #[test]
    fn subtitle_alignment_syncs_real_libmpv_and_runtime_config_snapshot() {
        let mut app = crate::app::PealayerApp::default();
        // No media/board output: verify the native fallback property contract
        // through the headless test player's actual libmpv instance.
        for alignment in [
            SubtitleAlignment::Left,
            SubtitleAlignment::Center,
            SubtitleAlignment::Right,
            SubtitleAlignment::SubtitleStyle,
        ] {
            app.subtitle_alignment = alignment;
            app.sync_subtitle_rendering();
            assert_eq!(
                app.mpv.get_property::<String>("sub-align-x").unwrap(),
                alignment.mpv_value()
            );
            assert_eq!(
                app.mpv.get_property::<String>("sub-justify").unwrap(),
                alignment.mpv_value()
            );
            assert_eq!(app.runtime_config_snapshot().subtitle_alignment, alignment);
        }
    }
}
