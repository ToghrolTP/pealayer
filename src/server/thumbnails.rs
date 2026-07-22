use std::path::{Path, PathBuf};
use std::process::Command;

pub fn get_thumbnail_cache_dir() -> PathBuf {
    if let Ok(cache) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(cache).join("pealayer").join("thumbnails")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".cache").join("pealayer").join("thumbnails")
    }
}

pub fn get_or_generate_thumbnail(video_path: &Path) -> Option<PathBuf> {
    if !video_path.exists() || !video_path.is_file() {
        return None;
    }

    let cache_dir = get_thumbnail_cache_dir();
    let _ = std::fs::create_dir_all(&cache_dir);

    // Hash file path to produce deterministic thumbnail filename
    let path_str = video_path.to_string_lossy();
    let hash = format!("{:x}", md5_hash(path_str.as_bytes()));
    let thumb_path = cache_dir.join(format!("{}.jpg", hash));

    if thumb_path.exists() {
        return Some(thumb_path);
    }

    // Try generating thumbnail using ffmpeg first
    let status = Command::new("ffmpeg")
        .args(&[
            "-ss", "00:00:05",
            "-i", path_str.as_ref(),
            "-vframes", "1",
            "-s", "320x180",
            "-q:v", "5",
            "-y",
            thumb_path.to_str().unwrap_or(""),
        ])
        .status();

    if status.map(|s| s.success()).unwrap_or(false) && thumb_path.exists() {
        return Some(thumb_path);
    }

    // Fallback to mpv if ffmpeg is not available
    let mpv_status = Command::new("mpv")
        .args(&[
            path_str.as_ref(),
            "--no-audio",
            "--start=5",
            "--frames=1",
            &format!("--o={}", thumb_path.to_str().unwrap_or("")),
        ])
        .status();

    if mpv_status.map(|s| s.success()).unwrap_or(false) && thumb_path.exists() {
        return Some(thumb_path);
    }

    None
}

fn md5_hash(data: &[u8]) -> u128 {
    // Simple fast hashing helper for thumbnail filename generation
    let mut hash: u128 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u128;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thumbnail_cache_path() {
        let dir = get_thumbnail_cache_dir();
        assert!(dir.to_str().unwrap().contains("pealayer"));
    }
}
