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
    Center,
    Right,
    /// Preserve native subtitle styling when no text processing is requested.
    #[default]
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
    // Track-style mode is the lossless path. In particular, `sub-text` and
    // even `sub-text/ass` cannot reproduce an ASS header/style table, while
    // plain-text formats such as SRT can still carry inline color and
    // positioning tags. Leave all of that with libass unless the user asks
    // Pealayer to force paragraph direction or placement.
    if direction == SubtitleDirection::Auto && alignment == SubtitleAlignment::SubtitleStyle {
        return false;
    }
    alignment != SubtitleAlignment::SubtitleStyle
        || direction != SubtitleDirection::Auto
        || replacements
            .iter()
            .any(|replacement| !replacement.from.is_empty())
}

pub fn preserves_track_style(
    direction: SubtitleDirection,
    alignment: SubtitleAlignment,
) -> bool {
    direction == SubtitleDirection::Auto && alignment == SubtitleAlignment::SubtitleStyle
}

pub fn ass_override_mode(
    direction: SubtitleDirection,
    alignment: SubtitleAlignment,
) -> &'static str {
    if preserves_track_style(direction, alignment) {
        "no"
    } else {
        "force"
    }
}

pub fn effective_position_percent(
    direction: SubtitleDirection,
    alignment: SubtitleAlignment,
    configured: f64,
) -> f64 {
    if preserves_track_style(direction, alignment) {
        // mpv documents 100 as the subtitle script's original position.
        100.0
    } else {
        configured.clamp(0.0, 100.0)
    }
}

fn skip_parenthesized(bytes: &[u8], mut index: usize) -> usize {
    if bytes.get(index) != Some(&b'(') {
        return index;
    }
    let mut depth = 0_u32;
    while index < bytes.len() {
        match bytes[index] {
            b'(' => depth += 1,
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return index + 1;
                }
            }
            _ => {}
        }
        index += 1;
    }
    index
}

fn strip_ass_placement_block(block: &str) -> String {
    let bytes = block.as_bytes();
    let mut output = String::with_capacity(block.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\\' {
            let ch = block[index..].chars().next().expect("valid UTF-8");
            output.push(ch);
            index += ch.len_utf8();
            continue;
        }

        let command_start = index;
        let name_start = index + 1;
        let rest = &bytes[name_start..];
        let alignment_len = if rest.starts_with(b"an")
            && rest.get(2).is_some_and(u8::is_ascii_digit)
        {
            Some(2)
        } else if rest.starts_with(b"a") && rest.get(1).is_some_and(u8::is_ascii_digit) {
            Some(1)
        } else {
            None
        };
        if let Some(name_len) = alignment_len {
            index = name_start + name_len;
            while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                index += 1;
            }
            continue;
        }

        let placement_name_len = [b"pos".as_slice(), b"move".as_slice(), b"org".as_slice()]
            .into_iter()
            .find(|name| rest.starts_with(name))
            .map(|name| name.len());
        if let Some(name_len) = placement_name_len {
            let mut value_start = name_start + name_len;
            while bytes.get(value_start).is_some_and(u8::is_ascii_whitespace) {
                value_start += 1;
            }
            if bytes.get(value_start) == Some(&b'(') {
                index = skip_parenthesized(bytes, value_start);
                continue;
            }
        }

        output.push('\\');
        index = command_start + 1;
    }
    output
}

fn normalize_ass_text(text: &str) -> String {
    let normalized = text
        .replace("\r\n", "\\N")
        .replace('\r', "\\N")
        .replace('\n', "\\N");
    let mut output = String::with_capacity(normalized.len());
    let mut remainder = normalized.as_str();
    while let Some(open) = remainder.find('{') {
        output.push_str(&remainder[..open]);
        let after_open = &remainder[open + 1..];
        let Some(close) = after_open.find('}') else {
            output.push_str(&remainder[open..]);
            return output;
        };
        let block = strip_ass_placement_block(&after_open[..close]);
        if !block.is_empty() {
            output.push('{');
            output.push_str(&block);
            output.push('}');
        }
        remainder = &after_open[close + 1..];
    }
    output.push_str(remainder);
    output
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
    // `sub-text/ass` preserves inline color/emphasis tags from SRT and other
    // text formats. Remove only authored placement commands when Pealayer is
    // explicitly supplying its own alignment/location; retain the rest.
    let styled = normalize_ass_text(text);
    let (alignment, x) = alignment.ass_anchor();
    let directed = match direction {
        SubtitleDirection::Auto => styled,
        SubtitleDirection::Ltr => format!("\u{202A}{styled}\u{202C}"),
        SubtitleDirection::Rtl => format!("\u{202B}{styled}\u{202C}"),
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
            &replacements
        ));
        assert!(requires_processed_overlay(
            SubtitleDirection::Rtl,
            SubtitleAlignment::SubtitleStyle,
            &replacements
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
    fn processed_overlay_preserves_colors_but_replaces_authored_placement() {
        let event = overlay_ass_event(
            r"{\an4\c&H00FFFF&}Colored{\pos(10,20)\b1} text",
            SubtitleDirection::Auto,
            SubtitleAlignment::Right,
            42.0,
            25.0,
        );
        assert!(event.contains(r"\an3\pos(1248,200)"));
        assert!(event.contains(r"{\c&H00FFFF&}Colored{\b1} text"));
        assert!(!event.contains(r"\an4"));
        assert!(!event.contains(r"\pos(10,20)"));
    }

    #[test]
    fn track_style_mode_is_lossless_and_uses_original_position() {
        let replacements = default_text_replacements();
        assert!(!requires_processed_overlay(
            SubtitleDirection::Auto,
            SubtitleAlignment::SubtitleStyle,
            &replacements
        ));
        assert_eq!(
            ass_override_mode(SubtitleDirection::Auto, SubtitleAlignment::SubtitleStyle),
            "no"
        );
        assert_eq!(
            effective_position_percent(
                SubtitleDirection::Auto,
                SubtitleAlignment::SubtitleStyle,
                24.0
            ),
            100.0
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
    fn subtitle_alignment_defaults_to_track_style_and_roundtrips_without_changing_direction() {
        let old: crate::config::AppConfig =
            serde_json::from_str(r#"{"subtitle_direction":"rtl"}"#).unwrap();
        assert_eq!(old.subtitle_alignment, SubtitleAlignment::SubtitleStyle);
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
            app.sub_position_percent = 25.0;
            app.sync_subtitle_rendering();
            assert_eq!(
                app.mpv.get_property::<String>("sub-align-x").unwrap(),
                alignment.mpv_value()
            );
            assert_eq!(
                app.mpv.get_property::<String>("sub-justify").unwrap(),
                alignment.mpv_value()
            );
            assert_eq!(
                app.mpv.get_property::<String>("sub-ass-override").unwrap(),
                ass_override_mode(SubtitleDirection::Auto, alignment)
            );
            assert_eq!(
                app.mpv.get_property::<f64>("sub-pos").unwrap(),
                effective_position_percent(SubtitleDirection::Auto, alignment, 25.0)
            );
            assert_eq!(app.runtime_config_snapshot().subtitle_alignment, alignment);
        }
    }
}
