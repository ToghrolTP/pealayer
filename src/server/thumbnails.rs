use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

const THUMBNAIL_PIPELINE_REVISION: &str = "fit-letterbox-v2";
const THUMBNAIL_FILTER: &str = "thumbnail=60,scale=640:360:force_original_aspect_ratio=decrease,pad=640:360:(ow-iw)/2:(oh-ih)/2:color=0x0b0f14";
const REMOTE_THUMBNAIL_PIPELINE_REVISION: &str = "remote-20-percent-v1";
const SEEK_THUMBNAIL_PIPELINE_REVISION: &str = "seek-preview-320x180-v1";
const SEEK_THUMBNAIL_FILTER: &str = "scale=320:180:force_original_aspect_ratio=decrease,pad=320:180:(ow-iw)/2:(oh-ih)/2:color=0x0b0f14";
const MAX_SEEK_THUMBNAILS: usize = 512;
static SEEK_THUMBNAIL_STAGE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq)]
pub struct RemoteThumbnailFile {
    pub path: PathBuf,
    pub position_seconds: Option<f64>,
}

pub fn get_thumbnail_cache_dir() -> PathBuf {
    if let Ok(cache) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(cache).join("pealayer").join("thumbnails")
    } else if let Ok(cache) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(cache)
            .join("Pealayer")
            .join("cache")
            .join("thumbnails")
    } else if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join("Library")
            .join("Caches")
            .join("Pealayer")
            .join("thumbnails")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".cache")
            .join("pealayer")
            .join("thumbnails")
    }
}

pub fn get_or_generate_thumbnail(video_path: &Path) -> Option<PathBuf> {
    if !video_path.exists() || !video_path.is_file() {
        return None;
    }

    let cache_dir = get_thumbnail_cache_dir();
    std::fs::create_dir_all(&cache_dir).ok()?;

    let cache_key = thumbnail_cache_key(video_path)?;
    let thumb_path = cache_dir.join(format!("{cache_key}.jpg"));
    if thumb_path.exists() {
        return Some(thumb_path);
    }

    let staged_path = cache_dir.join(format!("{cache_key}.tmp.jpg"));
    let _ = std::fs::remove_file(&staged_path);
    let input = video_path.to_string_lossy();
    let output = staged_path.to_string_lossy();

    let ffmpeg_ok = silent_command("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-ss",
            "00:00:05",
            "-i",
            input.as_ref(),
            "-an",
            "-sn",
            "-dn",
            "-frames:v",
            "1",
            "-vf",
            THUMBNAIL_FILTER,
            "-q:v",
            "4",
            "-y",
            output.as_ref(),
        ])
        .status()
        .is_ok_and(|status| status.success());

    let generated = ffmpeg_ok && staged_path.exists() || {
        let mpv_filter = format!("--vf=lavfi=[{THUMBNAIL_FILTER}]");
        let output_arg = format!("--o={}", output.as_ref());
        silent_command("mpv")
            .args([
                input.as_ref(),
                "--no-config",
                "--no-audio",
                "--start=5",
                "--frames=1",
                "--ovc=mjpeg",
                "--of=image2",
                mpv_filter.as_str(),
                output_arg.as_str(),
            ])
            .status()
            .is_ok_and(|status| status.success())
            && staged_path.exists()
    };

    if !generated {
        let _ = std::fs::remove_file(staged_path);
        return None;
    }

    // Publish only complete images. Concurrent requests may race, in which
    // case the already-published thumbnail is equally valid.
    if std::fs::rename(&staged_path, &thumb_path).is_err() && !thumb_path.exists() {
        let _ = std::fs::remove_file(staged_path);
        return None;
    }
    let _ = std::fs::remove_file(staged_path);
    Some(thumb_path)
}

/// Fetch a representative frame for a remote media target without showing a
/// console window. Finite media is sampled at roughly 20% of its duration;
/// duration-less streams fall back to their initial decodable frame.
pub fn get_or_generate_remote_thumbnail(
    media_url: &str,
    use_proxy: bool,
    proxy_url: Option<&str>,
) -> Result<RemoteThumbnailFile, String> {
    let cache_dir = get_thumbnail_cache_dir().join("remote");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|error| format!("Could not create the thumbnail cache: {error}"))?;

    let cache_key = remote_thumbnail_cache_key(media_url);
    let thumb_path = cache_dir.join(format!("{cache_key}.jpg"));
    if thumb_path.exists() {
        return Ok(RemoteThumbnailFile {
            path: thumb_path,
            position_seconds: None,
        });
    }

    let position_seconds = probe_remote_duration(media_url, use_proxy, proxy_url)
        .filter(|duration| duration.is_finite() && *duration > 0.0)
        .map(|duration| duration * 0.20);

    let staged_path = cache_dir.join(format!("{cache_key}.tmp.jpg"));
    let _ = std::fs::remove_file(&staged_path);
    let output = staged_path.to_string_lossy().into_owned();
    let seek = position_seconds.unwrap_or(0.0).to_string();

    let mut ffmpeg = silent_command("ffmpeg");
    configure_remote_proxy(&mut ffmpeg, use_proxy, proxy_url);
    ffmpeg.args(["-user_agent", crate::remote_location::USER_AGENT]);
    let ffmpeg_ok = ffmpeg
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-rw_timeout",
            "8000000",
            "-ss",
            seek.as_str(),
            "-i",
            media_url,
            "-an",
            "-sn",
            "-dn",
            "-frames:v",
            "1",
            "-vf",
            THUMBNAIL_FILTER,
            "-q:v",
            "4",
            "-y",
            output.as_str(),
        ])
        .status()
        .is_ok_and(|status| status.success());

    let generated = ffmpeg_ok && staged_path.exists() || {
        let mpv_filter = format!("--vf=lavfi=[{THUMBNAIL_FILTER}]");
        let output_arg = format!("--o={output}");
        let start_arg = format!("--start={seek}");
        let mut mpv = silent_command("mpv");
        configure_remote_proxy(&mut mpv, use_proxy, proxy_url);
        mpv.arg(format!("--user-agent={}", crate::remote_location::USER_AGENT));
        mpv.args([
            media_url,
            "--no-config",
            "--no-audio",
            start_arg.as_str(),
            "--frames=1",
            "--ovc=mjpeg",
            "--of=image2",
            mpv_filter.as_str(),
            output_arg.as_str(),
        ])
        .status()
        .is_ok_and(|status| status.success())
            && staged_path.exists()
    };

    if !generated {
        let _ = std::fs::remove_file(&staged_path);
        return Err(
            "No representative frame could be decoded with the available FFmpeg/MPV tools."
                .to_string(),
        );
    }

    if std::fs::rename(&staged_path, &thumb_path).is_err() && !thumb_path.exists() {
        let _ = std::fs::remove_file(&staged_path);
        return Err("The completed remote thumbnail could not be published.".to_string());
    }
    let _ = std::fs::remove_file(&staged_path);
    Ok(RemoteThumbnailFile {
        path: thumb_path,
        position_seconds,
    })
}

/// Decode the frame belonging to one whole playback second. Hover previews
/// deliberately share a persistent, bounded cache so egui and Web UI requests
/// never run separate extraction pipelines for the same media position.
pub fn get_or_generate_seek_thumbnail(
    media_target: &str,
    position_seconds: f64,
    use_proxy: bool,
    proxy_url: Option<&str>,
) -> Result<PathBuf, String> {
    let second = seek_preview_second(position_seconds)?;
    let use_proxy = crate::remote_location::playback_proxy_for(media_target).unwrap_or(use_proxy);
    let target = media_target.trim();
    if target.is_empty() {
        return Err("No media target is available for seek preview".to_string());
    }
    let remote = crate::media::is_remote_media_target(target);
    if !remote && !Path::new(target).is_file() {
        return Err("The media file is no longer available".to_string());
    }

    let cache_dir = get_thumbnail_cache_dir().join("seek");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|error| format!("Could not create the seek-preview cache: {error}"))?;
    let cache_key = seek_thumbnail_cache_key(target, second)?;
    let thumbnail_path = cache_dir.join(format!("{cache_key}.jpg"));
    if thumbnail_path.is_file() {
        return Ok(thumbnail_path);
    }

    let stage_id = SEEK_THUMBNAIL_STAGE_ID.fetch_add(1, Ordering::Relaxed);
    let staged_path = cache_dir.join(format!(
        "{cache_key}.{}.{}.tmp.jpg",
        std::process::id(),
        stage_id
    ));
    let _ = std::fs::remove_file(&staged_path);
    let output = staged_path.to_string_lossy().into_owned();
    let seek = second.to_string();

    let mut ffmpeg = silent_command("ffmpeg");
    if remote {
        configure_remote_proxy(&mut ffmpeg, use_proxy, proxy_url);
        ffmpeg.args(["-user_agent", crate::remote_location::USER_AGENT]);
    }
    let ffmpeg_ok = ffmpeg
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-ss",
            seek.as_str(),
            "-i",
            target,
            "-an",
            "-sn",
            "-dn",
            "-frames:v",
            "1",
            "-vf",
            SEEK_THUMBNAIL_FILTER,
            "-q:v",
            "4",
            "-y",
            output.as_str(),
        ])
        .status()
        .is_ok_and(|status| status.success());

    let generated = ffmpeg_ok && staged_path.is_file() || {
        let filter = format!("--vf=lavfi=[{SEEK_THUMBNAIL_FILTER}]");
        let output_arg = format!("--o={output}");
        let start_arg = format!("--start={seek}");
        let mut mpv = silent_command("mpv");
        if remote {
            configure_remote_proxy(&mut mpv, use_proxy, proxy_url);
        }
        mpv.args([
            target,
            "--no-config",
            "--no-audio",
            start_arg.as_str(),
            "--frames=1",
            "--ovc=mjpeg",
            "--of=image2",
            filter.as_str(),
            output_arg.as_str(),
        ])
        .status()
        .is_ok_and(|status| status.success())
            && staged_path.is_file()
    };

    if !generated {
        let _ = std::fs::remove_file(&staged_path);
        return Err(format!(
            "Could not decode a preview frame at {second} seconds"
        ));
    }
    if std::fs::rename(&staged_path, &thumbnail_path).is_err() && !thumbnail_path.is_file() {
        let _ = std::fs::remove_file(&staged_path);
        return Err("The completed seek preview could not be published".to_string());
    }
    let _ = std::fs::remove_file(&staged_path);
    prune_seek_thumbnail_cache(&cache_dir, MAX_SEEK_THUMBNAILS);
    Ok(thumbnail_path)
}

fn seek_preview_second(position_seconds: f64) -> Result<u64, String> {
    if !position_seconds.is_finite() || position_seconds < 0.0 {
        return Err("Seek-preview time must be a finite non-negative value".to_string());
    }
    Ok(position_seconds.floor().min(u64::MAX as f64) as u64)
}

fn seek_thumbnail_cache_key(media_target: &str, second: u64) -> Result<String, String> {
    let remote = crate::media::is_remote_media_target(media_target);
    let identity = if remote {
        normalize_remote_thumbnail_url(media_target)
    } else {
        std::fs::canonicalize(media_target)
            .unwrap_or_else(|_| PathBuf::from(media_target))
            .to_string_lossy()
            .into_owned()
    };
    let mut hash = 0xcbf29ce484222325_u64;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    };
    feed(SEEK_THUMBNAIL_PIPELINE_REVISION.as_bytes());
    feed(&[0]);
    feed(identity.as_bytes());
    if !remote {
        let metadata = Path::new(media_target)
            .metadata()
            .map_err(|error| format!("Could not inspect media for seek preview: {error}"))?;
        feed(&metadata.len().to_le_bytes());
        let modified = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|value| value.as_nanos())
            .unwrap_or_default();
        feed(&modified.to_le_bytes());
    }
    feed(&second.to_le_bytes());
    Ok(format!("{hash:016x}-{second}"))
}

fn prune_seek_thumbnail_cache(cache_dir: &Path, maximum: usize) {
    let Ok(entries) = std::fs::read_dir(cache_dir) else {
        return;
    };
    let mut thumbnails = entries
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|value| value == "jpg"))
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, entry.path()))
        })
        .collect::<Vec<_>>();
    if thumbnails.len() <= maximum {
        return;
    }
    thumbnails.sort_by_key(|(modified, _)| *modified);
    let remove_count = thumbnails.len() - maximum;
    for (_, path) in thumbnails.into_iter().take(remove_count) {
        let _ = std::fs::remove_file(path);
    }
}

fn probe_remote_duration(media_url: &str, use_proxy: bool, proxy_url: Option<&str>) -> Option<f64> {
    let mut command = silent_command("ffprobe");
    command.stdout(Stdio::piped());
    configure_remote_proxy(&mut command, use_proxy, proxy_url);
    command.args(["-user_agent", crate::remote_location::USER_AGENT]);
    let output = command
        .args([
            "-v",
            "error",
            "-rw_timeout",
            "8000000",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            media_url,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()?
        .trim()
        .parse::<f64>()
        .ok()
}

fn configure_remote_proxy(command: &mut Command, use_proxy: bool, proxy_url: Option<&str>) {
    const PROXY_VARIABLES: [&str; 6] = [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ];
    if !use_proxy {
        for variable in PROXY_VARIABLES {
            command.env_remove(variable);
        }
    } else if let Some(proxy) = proxy_url.filter(|proxy| !proxy.trim().is_empty()) {
        for variable in PROXY_VARIABLES {
            command.env(variable, proxy);
        }
    }
}

/// Canonicalize a remote target before it is used as a persistent cache key.
/// Fragments do not affect the fetched media and are deliberately discarded.
pub fn normalize_remote_thumbnail_url(media_url: &str) -> String {
    let trimmed = media_url.trim();
    let Ok(mut parsed) = url::Url::parse(trimmed) else {
        return trimmed.to_string();
    };
    parsed.set_fragment(None);
    parsed.to_string()
}

fn remote_thumbnail_cache_key(media_url: &str) -> String {
    // FNV-1a is intentionally used instead of DefaultHasher: its output is
    // stable across processes and Rust versions, which is required for an
    // OS-native persistent cache.
    let normalized = normalize_remote_thumbnail_url(media_url);
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in REMOTE_THUMBNAIL_PIPELINE_REVISION
        .bytes()
        .chain(std::iter::once(0))
        .chain(normalized.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Remove every remote thumbnail that no longer belongs to recent history.
/// The retained set is keyed by normalized URL, so aliases that differ only
/// by a fragment share one cache entry.
pub fn prune_remote_thumbnail_cache<'a>(recent_urls: impl IntoIterator<Item = &'a str>) {
    let cache_dir = get_thumbnail_cache_dir().join("remote");
    prune_remote_thumbnail_cache_dir(&cache_dir, recent_urls);
}

fn prune_remote_thumbnail_cache_dir<'a>(
    cache_dir: &Path,
    recent_urls: impl IntoIterator<Item = &'a str>,
) {
    let retained = recent_urls
        .into_iter()
        .flat_map(|url| {
            let key = remote_thumbnail_cache_key(url);
            [format!("{key}.jpg"), format!("{key}.tmp.jpg")]
        })
        .collect::<HashSet<_>>();
    let Ok(entries) = std::fs::read_dir(cache_dir) else {
        return;
    };
    for entry in entries.flatten() {
        if !retained.contains(&entry.file_name().to_string_lossy().to_string()) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn silent_command(program: &str) -> Command {
    let mut command = Command::new(program);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn thumbnail_cache_key(video_path: &Path) -> Option<String> {
    let metadata = video_path.metadata().ok()?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    THUMBNAIL_PIPELINE_REVISION.hash(&mut hasher);
    video_path.to_string_lossy().hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    modified.hash(&mut hasher);
    Some(format!("{:016x}", hasher.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thumbnail_cache_path() {
        let dir = get_thumbnail_cache_dir();
        assert!(
            dir.to_string_lossy()
                .to_ascii_lowercase()
                .contains("pealayer")
        );
        assert!(dir.to_string_lossy().contains("thumbnails"));
    }

    #[test]
    fn thumbnail_filter_preserves_source_aspect_ratio() {
        assert!(THUMBNAIL_FILTER.contains("force_original_aspect_ratio=decrease"));
        assert!(THUMBNAIL_FILTER.contains("pad=640:360"));
    }

    #[test]
    fn remote_cache_identity_is_stable_and_uses_normalized_url() {
        let first = remote_thumbnail_cache_key("https://EXAMPLE.test/movie.mp4#chapter");
        assert_eq!(
            first,
            remote_thumbnail_cache_key("https://example.test/movie.mp4")
        );
        assert_ne!(
            first,
            remote_thumbnail_cache_key("https://example.test/movie.mp4?edition=two")
        );
    }

    #[test]
    fn seek_preview_uses_whole_seconds_and_rejects_invalid_time() {
        assert_eq!(seek_preview_second(0.0).unwrap(), 0);
        assert_eq!(seek_preview_second(12.999).unwrap(), 12);
        assert!(seek_preview_second(-0.001).is_err());
        assert!(seek_preview_second(f64::NAN).is_err());
        assert!(seek_preview_second(f64::INFINITY).is_err());
    }

    #[test]
    fn seek_preview_remote_cache_key_tracks_media_and_second() {
        let first = seek_thumbnail_cache_key("https://example.test/movie.mp4#chapter", 12)
            .expect("first cache key");
        assert_eq!(
            first,
            seek_thumbnail_cache_key("https://example.test/movie.mp4", 12)
                .expect("normalized cache key")
        );
        assert_ne!(
            first,
            seek_thumbnail_cache_key("https://example.test/movie.mp4", 13)
                .expect("next-second cache key")
        );
        assert_ne!(
            first,
            seek_thumbnail_cache_key("https://example.test/other.mp4", 12)
                .expect("different-media cache key")
        );
    }

    #[test]
    fn seek_preview_filter_preserves_source_aspect_ratio() {
        assert!(SEEK_THUMBNAIL_FILTER.contains("force_original_aspect_ratio=decrease"));
        assert!(SEEK_THUMBNAIL_FILTER.contains("pad=320:180"));
        assert!(!SEEK_THUMBNAIL_FILTER.contains("thumbnail="));
    }

    #[test]
    fn pruning_keeps_only_current_recent_urls() {
        let cache_dir = std::env::temp_dir().join(format!(
            "pealayer-thumbnail-prune-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&cache_dir).unwrap();
        let keep_url = "https://example.test/keep.mp4";
        let keep = cache_dir.join(format!("{}.jpg", remote_thumbnail_cache_key(keep_url)));
        let remove = cache_dir.join(format!(
            "{}.jpg",
            remote_thumbnail_cache_key("https://example.test/remove.mp4")
        ));
        std::fs::write(&keep, b"keep").unwrap();
        std::fs::write(&remove, b"remove").unwrap();

        prune_remote_thumbnail_cache_dir(&cache_dir, [keep_url]);

        assert!(keep.exists());
        assert!(!remove.exists());
        let _ = std::fs::remove_dir_all(cache_dir);
    }
}
