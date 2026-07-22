use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntryInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_media: bool,
    pub size_bytes: u64,
    pub has_thumbnail: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryBrowseResponse {
    pub current_path: String,
    pub parent_path: Option<String>,
    pub entries: Vec<FileEntryInfo>,
}

#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub old_path: String,
    pub new_name: String,
}

#[derive(Debug, Deserialize)]
pub struct TrashRequest {
    pub target_path: String,
}

pub const MEDIA_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "webm", "mov", "flv", "mp3", "flac", "wav", "m4v", "ts",
];

pub fn browse_directory(dir_path: Option<&str>) -> Result<DirectoryBrowseResponse, String> {
    let target = match dir_path {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    let canonical = target.canonicalize().unwrap_or(target.clone());
    let parent_path = canonical.parent().map(|p| p.to_string_lossy().to_string());

    let mut entries = Vec::new();

    if let Ok(read_dir) = fs::read_dir(&canonical) {
        for entry_res in read_dir {
            if let Ok(entry) = entry_res {
                let path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();

                if file_name.starts_with('.') {
                    continue; // Skip hidden files
                }

                let is_dir = path.is_dir();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                let is_media = !is_dir && MEDIA_EXTENSIONS.contains(&ext.as_str());

                let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let has_thumbnail = is_media;

                entries.push(FileEntryInfo {
                    name: file_name,
                    path: path.to_string_lossy().to_string(),
                    is_dir,
                    is_media,
                    size_bytes,
                    has_thumbnail,
                });
            }
        }
    }

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(DirectoryBrowseResponse {
        current_path: canonical.to_string_lossy().to_string(),
        parent_path,
        entries,
    })
}

pub fn rename_file(old_path_str: &str, new_name: &str) -> Result<String, String> {
    let old_path = Path::new(old_path_str);
    if !old_path.exists() {
        return Err("File does not exist".to_string());
    }

    let parent = old_path.parent().ok_or("Invalid parent directory")?;
    let new_path = parent.join(new_name);

    if new_path.exists() {
        return Err("Target filename already exists".to_string());
    }

    fs::rename(old_path, &new_path)
        .map_err(|e| format!("Rename failed: {}", e))?;

    Ok(new_path.to_string_lossy().to_string())
}

pub fn trash_file(target_path_str: &str) -> Result<(), String> {
    let target = Path::new(target_path_str);
    if !target.exists() {
        return Err("File does not exist".to_string());
    }

    // Move to trash or remove file
    fs::remove_file(target).map_err(|e| format!("Failed deleting file: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_extensions_list() {
        assert!(MEDIA_EXTENSIONS.contains(&"mp4"));
        assert!(MEDIA_EXTENSIONS.contains(&"mkv"));
        assert!(MEDIA_EXTENSIONS.contains(&"webm"));
    }

    #[test]
    fn test_browse_current_directory() {
        let resp = browse_directory(None).unwrap();
        assert!(!resp.current_path.is_empty());
    }
}
