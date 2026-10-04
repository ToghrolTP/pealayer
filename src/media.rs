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
    if is_remote_media_target(target) {
        redact_media_target(target)
    } else {
        std::path::Path::new(target)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(target)
            .to_string()
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
}
