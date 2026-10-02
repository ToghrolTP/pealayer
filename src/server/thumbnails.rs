use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const THUMBNAIL_PIPELINE_REVISION: &str = "fit-letterbox-v2";
const THUMBNAIL_FILTER: &str = "thumbnail=60,scale=640:360:force_original_aspect_ratio=decrease,pad=640:360:(ow-iw)/2:(oh-ih)/2:color=0x0b0f14";
const REMOTE_THUMBNAIL_PIPELINE_REVISION: &str = "remote-20-percent-v1";

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
    cache_identity: &str,
    use_proxy: bool,
    proxy_url: Option<&str>,
) -> Result<RemoteThumbnailFile, String> {
    let cache_dir = get_thumbnail_cache_dir().join("remote");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|error| format!("Could not create the thumbnail cache: {error}"))?;

    let position_seconds = probe_remote_duration(media_url, use_proxy, proxy_url)
        .filter(|duration| duration.is_finite() && *duration > 0.0)
        .map(|duration| duration * 0.20);
    let cache_key = remote_thumbnail_cache_key(media_url, cache_identity);
    let thumb_path = cache_dir.join(format!("{cache_key}.jpg"));
    if thumb_path.exists() {
        return Ok(RemoteThumbnailFile {
            path: thumb_path,
            position_seconds,
        });
    }

    let staged_path = cache_dir.join(format!("{cache_key}.tmp.jpg"));
    let _ = std::fs::remove_file(&staged_path);
    let output = staged_path.to_string_lossy().into_owned();
    let seek = position_seconds.unwrap_or(0.0).to_string();

    let mut ffmpeg = silent_command("ffmpeg");
    configure_remote_proxy(&mut ffmpeg, use_proxy, proxy_url);
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

fn probe_remote_duration(media_url: &str, use_proxy: bool, proxy_url: Option<&str>) -> Option<f64> {
    let mut command = silent_command("ffprobe");
    command.stdout(Stdio::piped());
    configure_remote_proxy(&mut command, use_proxy, proxy_url);
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

fn remote_thumbnail_cache_key(media_url: &str, cache_identity: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    REMOTE_THUMBNAIL_PIPELINE_REVISION.hash(&mut hasher);
    media_url.hash(&mut hasher);
    cache_identity.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
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
    fn remote_cache_identity_is_stable_and_sensitive_to_metadata() {
        let first = remote_thumbnail_cache_key("https://example.test/movie.mp4", "etag-one");
        assert_eq!(
            first,
            remote_thumbnail_cache_key("https://example.test/movie.mp4", "etag-one")
        );
        assert_ne!(
            first,
            remote_thumbnail_cache_key("https://example.test/movie.mp4", "etag-two")
        );
    }
}
