//! Media-target classification and safe presentation helpers.
//!
//! Pealayer deliberately keeps the original target for playback and local
//! recents, while every user-visible/status representation passes through the
//! redactor so URL credentials never leave the player UI through its APIs.

const REMOTE_SCHEMES: &[&str] = &[
    "http", "https", "ftp", "ftps", "rtsp", "rtsps", "rtmp", "rtmps", "rtp", "srt", "rist", "udp",
    "tcp", "tls",
];

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
}
