use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const THUMBNAIL_PIPELINE_REVISION: &str = "fit-letterbox-v2";
const THUMBNAIL_FILTER: &str = "thumbnail=60,scale=640:360:force_original_aspect_ratio=decrease,pad=640:360:(ow-iw)/2:(oh-ih)/2:color=0x0b0f14";

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
}
