//! Media-target classification and safe presentation helpers.
//!
//! Pealayer deliberately keeps the original target for playback and local
//! recents, while every user-visible/status representation passes through the
//! redactor so URL credentials never leave the player UI through its APIs.

const REMOTE_SCHEMES: &[&str] = &[
    "http", "https", "ftp", "ftps", "rtsp", "rtsps", "rtmp", "rtmps", "rtp", "srt", "rist", "udp",
    "tcp", "tls",
];

const LIVE_SCHEMES: &[&str] = &[
    "rtsp", "rtsps", "rtmp", "rtmps", "rtp", "srt", "rist", "udp",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MediaTimelineState {
    NoMedia,
    Determining,
    Live,
    Finite { duration: f64, seekable: bool },
}

pub fn timeline_state(
    target: Option<&str>,
    metadata_loaded: bool,
    duration: f64,
    seekable: bool,
) -> MediaTimelineState {
    let Some(target) = target else {
        return MediaTimelineState::NoMedia;
    };
    if duration.is_finite() && duration > 0.0 {
        return MediaTimelineState::Finite { duration, seekable };
    }
    if is_live_media_target(target) || metadata_loaded {
        MediaTimelineState::Live
    } else {
        MediaTimelineState::Determining
    }
}

pub fn media_scheme(target: &str) -> Option<&str> {
    let (scheme, _) = target.trim().split_once("://")?;
    if scheme.is_empty()
        || !scheme.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphabetic()
                || (index > 0 && (byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.')))
        })
    {
        return None;
    }
    Some(scheme)
}

pub fn is_remote_media_target(target: &str) -> bool {
    media_scheme(target).is_some_and(|scheme| {
        REMOTE_SCHEMES
            .iter()
            .any(|candidate| scheme.eq_ignore_ascii_case(candidate))
    })
}

/// Explicit clipboard action: keep the original URL bytes, not a proxy/cache
/// path or redacted diagnostics. Local paths are made absolute without IO.
pub fn source_location(path:&std::path::Path)->Result<String,String> {
    let target=path.to_str().ok_or("Media location is not valid text")?;
    if target.is_empty(){return Err("No media is loaded".into());}
    if media_scheme(target).is_some() {Ok(target.to_owned())}
    else {std::path::absolute(path).map(|path|path.to_string_lossy().into_owned()).map_err(|error|error.to_string())}
}

pub fn is_live_media_target(target: &str) -> bool {
    media_scheme(target).is_some_and(|scheme| {
        LIVE_SCHEMES
            .iter()
            .any(|candidate| scheme.eq_ignore_ascii_case(candidate))
    })
}

pub fn prefers_rtsp_tcp(target: &str) -> bool {
    media_scheme(target).is_some_and(|scheme| {
        scheme.eq_ignore_ascii_case("rtsp") || scheme.eq_ignore_ascii_case("rtsps")
    })
}

pub fn redact_media_target(target: &str) -> String {
    let trimmed = target.trim();
    let Some((scheme, remainder)) = trimmed.split_once("://") else {
        return trimmed.to_string();
    };
    let authority_end = remainder.find(['/', '?', '#']).unwrap_or(remainder.len());
    let (authority, suffix) = remainder.split_at(authority_end);
    let Some((_, host)) = authority.rsplit_once('@') else {
        return trimmed.to_string();
    };
    format!("{scheme}://***@{host}{suffix}")
}

pub fn media_target_label(target: &str) -> String {
    if media_scheme(target).is_some() && let Ok(url) = url::Url::parse(target) {
        return url.path_segments()
            .and_then(|mut parts| parts.rfind(|part| !part.is_empty()))
            .map(decode_media_name)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| url.host_str().unwrap_or("Media").to_owned());
    }
    if media_scheme(target).is_some() { return "Media".into(); }
    // Windows locations must also be presentable by remote/non-Windows clients.
    let name = if target.as_bytes().get(1) == Some(&b':') || target.starts_with("\\\\") {
        target.rsplit(['/', '\\']).next().unwrap_or(target)
    } else {
        std::path::Path::new(target).file_name().and_then(|name| name.to_str()).unwrap_or(target)
    };
    clean_display_text(name)
}

fn clean_display_text(value: &str) -> String {
    value.chars().filter(|c| !c.is_control() && !matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')).collect()
}

/// Decode URL path presentation once, not form data (`+` is a literal plus).
/// Never pass this result back to a loader, history key or directory-scope check.
pub fn decode_media_name(value: &str) -> String {
    let mut bytes = Vec::with_capacity(value.len());
    let mut source = value.as_bytes().iter().copied().peekable();
    while let Some(byte) = source.next() {
        if byte == b'%' {
            let mut probe = source.clone();
            if let (Some(a), Some(b)) = (probe.next(), probe.next())
                && let (Some(a), Some(b)) = ((a as char).to_digit(16), (b as char).to_digit(16)) {
                bytes.push((a * 16 + b) as u8);
                source = probe;
                continue;
            }
        }
        bytes.push(byte);
    }
    let decoded = clean_display_text(&String::from_utf8_lossy(&bytes));
    if !decoded.contains('&') { return decoded; }
    // Reuse the existing HTML entity parser, but escape literal angle brackets
    // first so filenames cannot become markup. Only one entity-decoding pass:
    // &amp;amp; stays &amp;, and URL %2520 stays %20.
    let text = decoded.replace('<', "&lt;").replace('>', "&gt;");
    let fragment = scraper::Html::parse_fragment(&text);
    clean_display_text(&fragment.root_element().text().collect::<String>())
}

/// Full-location display is opt-in; credentials remain redacted. The original
/// encoded source is retained separately for copying and playback.
pub fn media_location_display(target: &str) -> String {
    if media_scheme(target).is_some() { decode_media_name(&redact_media_target(target)) }
    else { clean_display_text(target) }
}

pub fn media_title(target: &str, format: crate::config::MediaTitleFormat) -> String {
    use crate::config::MediaTitleFormat;
    match format {
        MediaTitleFormat::FileName => media_target_label(target),
        MediaTitleFormat::FileStem => {
            let name = media_target_label(target);
            if media_scheme(target).is_some() && url::Url::parse(target).ok()
                .is_none_or(|url| url.path().trim_matches('/').is_empty()) { return name; }
            // Only remove an actual final extension, not a leading dot.
            name.rsplit_once('.').filter(|(stem, _)| !stem.is_empty())
                .map(|(stem, _)| stem.to_owned()).unwrap_or(name)
        }
        MediaTitleFormat::Location => media_location_display(target),
    }
}

/// Selects the media target to load for a fresh process.
///
/// An explicit CLI target is authoritative. Automatic restoration is
/// deliberately conservative for local media (the file must still exist) but
/// keeps remote targets eligible because reachability is determined by mpv
/// asynchronously. `last_media_target` is the durable source; recent media and
/// playback history provide migration recovery for configurations written by
/// builds that predate that field.
pub fn startup_media_target(
    explicit: Option<&str>,
    restore_enabled: bool,
    last_media_target: Option<&std::path::Path>,
    recent_media: &[std::path::PathBuf],
    playback_positions: &[crate::config::PlaybackPositionEntry],
) -> Option<String> {
    if let Some(explicit) = explicit.map(str::trim).filter(|value| !value.is_empty()) {
        return Some(explicit.to_string());
    }
    if !restore_enabled {
        return None;
    }

    last_media_target
        .into_iter()
        .chain(recent_media.iter().map(std::path::PathBuf::as_path))
        .map(|target| target.to_string_lossy().into_owned())
        .chain(
            playback_positions
                .iter()
                .map(|entry| entry.target.trim().to_string()),
        )
        .find(|target| restorable_media_target(target))
}

fn restorable_media_target(target: &str) -> bool {
    let target = target.trim();
    !target.is_empty() && (is_remote_media_target(target) || std::path::Path::new(target).is_file())
}

/// Stable identity for bounded playback-position history. URL fragments do
/// not change the underlying media. Existing local files are canonicalized so
/// aliases share a resume point; non-existing paths remain usable as entered.
pub fn playback_history_key(target: &str) -> String {
    let trimmed = target.trim();
    if is_remote_media_target(trimmed) {
        let Ok(mut parsed) = url::Url::parse(trimmed) else {
            return trimmed.to_string();
        };
        parsed.set_fragment(None);
        return parsed.to_string();
    }

    let path = std::path::Path::new(trimmed);
    let normalized = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let key = normalized.to_string_lossy().into_owned();
    if cfg!(target_os = "windows") {
        key.to_lowercase()
    } else {
        key
    }
}

pub fn buffered_until(
    duration: f64,
    playback_time: f64,
    cache_duration: Option<f64>,
) -> Option<f64> {
    if duration <= 0.0 {
        return None;
    }
    cache_duration
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
        .map(|seconds| (playback_time + seconds).clamp(0.0, duration))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_decode_once_without_changing_source_identity_or_local_percent_names() {
        for (target, expected) in [
            ("https://user:secret@example.invalid/dir/My%20movie%20%26amp%3B%20you.mkv?token=private#chapter", "My movie & you.mkv"),
            ("https://example.invalid/%D9%81%DB%8C%D9%84%D9%85.mp4", "فیلم.mp4"),
            ("https://example.invalid/a+b%2520%26amp%3Bamp%3B.mp4", "a+b%20&amp;.mp4"),
            ("https://example.invalid/a%ZZ%00%0A%E2%80%AE.mp4", "a%ZZ.mp4"),
            ("https://user:secret@example.invalid/?token=private", "example.invalid"),
            ("https://user:secret@[invalid", "Media"),
            ("file:///C:/media/My%20movie.mp4", "My movie.mp4"),
            (r"C:\media\Literal%20&amp;.mp4", "Literal%20&amp;.mp4"),
            ("/media/Literal%20&amp;.mp4", "Literal%20&amp;.mp4"),
            ("https://example.invalid/a%26%23x41%3B%26%2366%3B%26lt%3B.mp4", "aAB<.mp4"),
            ("https://example.invalid/A%26ndash%3BB%26copy%3B%3Cclip%3E.mp4", "A–B©<clip>.mp4"),
            ("https://example.invalid/A%00%26amp%3BB.mp4", "A&B.mp4"),
        ] {
            let original = target.to_owned();
            assert_eq!(media_target_label(target), expected, "{target}");
            assert_eq!(target, original);
        }
    }

    #[test]
    fn media_title_formats_are_explicit_persistent_and_safe() {
        use crate::config::MediaTitleFormat::*;
        let target = "https://user:secret@example.invalid/dir/My%20movie.mkv";
        assert_eq!(media_title(target, FileName), "My movie.mkv");
        assert_eq!(media_title(target, FileStem), "My movie");
        assert_eq!(media_title(target, Location), "https://***@example.invalid/dir/My movie.mkv");
        assert_eq!(media_title(".hidden", FileStem), ".hidden");
        assert_eq!(media_title("https://example.invalid/", FileStem), "example.invalid");
        let mut config = crate::config::AppConfig::default();
        assert_eq!(config.local_media_title_format, FileName);
        assert_eq!(config.remote_media_title_format, FileName);
        config.local_media_title_format = Location;
        config.remote_media_title_format = FileStem;
        let restored: crate::config::AppConfig = serde_json::from_value(serde_json::to_value(&config).unwrap()).unwrap();
        assert_eq!(restored.local_media_title_format, Location);
        assert_eq!(restored.remote_media_title_format, FileStem);
        let updated = restored.apply_patch(&serde_json::json!({"remote_media_title_format":"file_name"})).unwrap();
        assert_eq!(updated.local_media_title_format, Location);
        assert_eq!(updated.remote_media_title_format, FileName);
        assert!(updated.apply_patch(&serde_json::json!({"remote_media_title_format":"bogus"})).is_err());
    }

    #[test]
    fn recognizes_network_media_protocols_without_mistaking_windows_paths() {
        assert!(is_remote_media_target("rtsp://camera.invalid/live"));
        assert!(is_remote_media_target("HTTPS://cdn.invalid/video.mp4"));
        assert!(is_remote_media_target("srt://media.invalid:9000"));
        assert!(is_live_media_target("rtsp://camera.invalid/live"));
        assert!(!is_live_media_target("https://cdn.invalid/video.mp4"));
        assert!(prefers_rtsp_tcp("rtsp://camera.invalid/live"));
        assert!(!prefers_rtsp_tcp("https://cdn.invalid/video.mp4"));
        assert!(!is_remote_media_target(r"C:\media\clip.mp4"));
        assert!(!is_remote_media_target("notes:clip.mp4"));
    }

    #[test]
    fn redacts_url_userinfo_and_keeps_the_playable_location_legible() {
        assert_eq!(
            redact_media_target("rtsp://operator:secret@camera.invalid/live?channel=2"),
            "rtsp://***@camera.invalid/live?channel=2"
        );
        assert_eq!(
            redact_media_target("https://cdn.invalid/movie.mp4"),
            "https://cdn.invalid/movie.mp4"
        );
    }

    #[test]
    fn buffered_position_is_bounded_by_duration() {
        assert_eq!(buffered_until(100.0, 20.0, Some(15.0)), Some(35.0));
        assert_eq!(buffered_until(100.0, 95.0, Some(15.0)), Some(100.0));
        assert_eq!(buffered_until(0.0, 20.0, Some(15.0)), None);
    }

    #[test]
    fn timeline_distinguishes_pending_metadata_from_durationless_media() {
        assert_eq!(
            timeline_state(Some("https://cdn.invalid/movie.mp4"), false, 0.0, false),
            MediaTimelineState::Determining
        );
        assert_eq!(
            timeline_state(Some("https://cdn.invalid/endless"), true, 0.0, false),
            MediaTimelineState::Live
        );
        assert_eq!(
            timeline_state(Some("rtsp://camera.invalid/live"), false, 0.0, false),
            MediaTimelineState::Live
        );
        assert_eq!(
            timeline_state(Some("movie.mp4"), true, 125.0, true),
            MediaTimelineState::Finite {
                duration: 125.0,
                seekable: true,
            }
        );
    }

    #[test]
    fn playback_history_identity_ignores_remote_fragments() {
        assert_eq!(
            playback_history_key("https://example.invalid/movie.mp4#chapter"),
            "https://example.invalid/movie.mp4"
        );
    }

    #[test]
    fn startup_restore_prefers_cli_then_durable_last_target_and_skips_missing_files() {
        let existing = std::env::temp_dir().join(format!(
            "pealayer-startup-media-{}.mp4",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&existing, b"test").unwrap();
        let missing = existing.with_extension("missing.mp4");
        let history = vec![crate::config::PlaybackPositionEntry {
            target: existing.to_string_lossy().into_owned(),
            position_seconds: 12.0,
            updated_at_unix_ms: 1,
        }];

        assert_eq!(
            startup_media_target(
                Some("https://example.invalid/explicit.mp4"),
                true,
                Some(&existing),
                &[],
                &history,
            )
            .as_deref(),
            Some("https://example.invalid/explicit.mp4")
        );
        assert_eq!(
            startup_media_target(None, true, Some(&missing), &[], &history),
            Some(existing.to_string_lossy().into_owned())
        );
        assert_eq!(
            startup_media_target(None, false, Some(&existing), &[], &history),
            None
        );
        std::fs::remove_file(existing).unwrap();
    }

    #[test]
    fn remote_media_remains_restorable_without_blocking_on_a_network_probe() {
        assert_eq!(
            startup_media_target(
                None,
                true,
                Some(std::path::Path::new("https://example.invalid/movie.mp4")),
                &[],
                &[],
            )
            .as_deref(),
            Some("https://example.invalid/movie.mp4")
        );
    }
    #[test]
    fn clipboard_location_preserves_url_bytes_and_resolves_local_paths() {
        let url="https://example.invalid/a%2Fb%20c.mkv?token=example&x=a+b";
        assert_eq!(source_location(std::path::Path::new(url)).unwrap(),url);
        let local=source_location(std::path::Path::new("movie with spaces.mkv")).unwrap();
        assert!(std::path::Path::new(&local).is_absolute());
        assert!(local.ends_with("movie with spaces.mkv"));
    }
}
