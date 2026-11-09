//! Explicit user clipboard operations. Explorer cut/paste never moves or deletes
//! files: Pealayer imports references/bytes through the normal asset owner.
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

pub fn write(text: &str) -> Result<(), String> {
    if text.len() > MAX_BYTES || text.contains('\0') {
        return Err("Clipboard selection is too large or contains invalid text".into());
    }
    arboard::Clipboard::new()
        .and_then(|mut c| c.set_text(text))
        .map_err(|e| format!("Could not copy selection: {e}"))?;
    #[cfg(windows)]
    if serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| v.get("format").and_then(|v| v.as_str()).map(str::to_owned))
        .is_some_and(|v| v.starts_with("pealayer."))
    {
        // Unicode text remains the interoperable fallback. Add a registered
        // structured format without clearing that successful text transaction.
        unsafe {
            use windows::{
                Win32::{
                    Foundation::{GlobalFree, HANDLE, HGLOBAL},
                    System::{
                        DataExchange::{
                            CloseClipboard, GetClipboardData, OpenClipboard,
                            RegisterClipboardFormatW, SetClipboardData,
                        },
                        Memory::{
                            GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
                        },
                    },
                },
                core::w,
            };
            if OpenClipboard(None).is_ok() {
                // Another process may have replaced the clipboard after the
                // text write. Never attach our structured format to its data.
                let same = GetClipboardData(13).ok().is_some_and(|handle| {
                    let memory = HGLOBAL(handle.0);
                    let bytes = GlobalSize(memory);
                    let ptr = GlobalLock(memory);
                    if ptr.is_null() {
                        return false;
                    }
                    let same = if bytes <= MAX_BYTES * 2 + 2 {
                        let chars = std::slice::from_raw_parts(ptr.cast::<u16>(), bytes / 2);
                        let end = chars.iter().position(|v| *v == 0).unwrap_or(chars.len());
                        String::from_utf16(&chars[..end]).is_ok_and(|value| value == text)
                    } else {
                        false
                    };
                    let _ = GlobalUnlock(memory);
                    same
                });
                if !same {
                    let _ = CloseClipboard();
                    return Err(
                        "Clipboard changed before copying completed; selection was not cut".into(),
                    );
                }
                if let Ok(memory) = GlobalAlloc(GMEM_MOVEABLE, text.len() + 1) {
                    let ptr = GlobalLock(memory);
                    if !ptr.is_null() {
                        std::ptr::copy_nonoverlapping(text.as_ptr(), ptr.cast(), text.len());
                        *ptr.cast::<u8>().add(text.len()) = 0;
                        let _ = GlobalUnlock(memory);
                        if SetClipboardData(
                            RegisterClipboardFormatW(w!("Pealayer.Authoring")),
                            Some(HANDLE(memory.0)),
                        )
                        .is_err()
                        {
                            let _ = GlobalFree(Some(memory));
                        }
                    } else {
                        let _ = GlobalFree(Some(memory));
                    }
                }
                let _ = CloseClipboard();
            }
        }
    }
    Ok(())
}
pub fn text() -> Result<String, String> {
    let text = arboard::Clipboard::new()
        .and_then(|mut c| c.get_text())
        .map_err(|_| "Clipboard does not contain text".to_string())?;
    if text.len() > MAX_BYTES {
        return Err("Clipboard content exceeds 8 MiB".into());
    }
    Ok(text)
}

#[cfg(windows)]
pub fn files() -> Result<Vec<String>, String> {
    use windows::Win32::{
        System::DataExchange::{
            CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
        },
        UI::Shell::{DragQueryFileW, HDROP},
    };
    // CF_HDROP is predefined, not a registered private clipboard format.
    unsafe {
        if IsClipboardFormatAvailable(15).is_err() {
            return Ok(Vec::new());
        }
        OpenClipboard(None).map_err(|_| "Clipboard is busy; try Paste again".to_string())?;
        struct Close;
        impl Drop for Close {
            fn drop(&mut self) {
                unsafe {
                    let _ = CloseClipboard();
                }
            }
        }
        let _close = Close;
        let handle = GetClipboardData(15).map_err(|e| e.to_string())?;
        let drop = HDROP(handle.0);
        let count = DragQueryFileW(drop, u32::MAX, None);
        if count == 0 || count as usize > crate::assets::MAX_FILES {
            return Err("Paste 1–64 asset files".into());
        }
        let mut paths = Vec::new();
        for index in 0..count {
            let length = DragQueryFileW(drop, index, None) as usize;
            if length == 0 || length > 32767 {
                return Err("Invalid clipboard file path".into());
            }
            let mut path = vec![0u16; length + 1];
            if DragQueryFileW(drop, index, Some(&mut path)) as usize != length {
                return Err("Clipboard file list changed".into());
            }
            paths.push(
                String::from_utf16(&path[..length])
                    .map_err(|_| "Invalid Unicode clipboard path")?,
            );
        }
        // The clipboard owns HDROP. Never DragFinish/GlobalFree it.
        Ok(paths)
    }
}
#[cfg(not(windows))]
pub fn files() -> Result<Vec<String>, String> {
    Ok(Vec::new())
}

/// File URI lists are accepted only on explicit Paste, never as shell commands.
pub fn text_paths(text: &str) -> Option<Vec<String>> {
    let paths = text
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let line = line.trim().trim_matches('"');
            if let Ok(url) = url::Url::parse(line) {
                if url.scheme() == "file" {
                    return url
                        .to_file_path()
                        .ok()
                        .map(|p| p.to_string_lossy().into_owned());
                }
            }
            let path = std::path::Path::new(line);
            path.is_absolute().then(|| line.to_string())
        })
        .collect::<Option<Vec<_>>>()?;
    (!paths.is_empty() && paths.len() <= crate::assets::MAX_FILES).then_some(paths)
}

#[cfg(test)]
mod tests {
    #[test]
    fn accepts_only_explicit_file_lists() {
        assert_eq!(
            super::text_paths("# comment\nfile:///C:/media/movie.mkv")
                .unwrap()
                .len(),
            1
        );
        assert!(super::text_paths("https://example.invalid/movie.mkv").is_none());
        assert!(super::text_paths("run something").is_none());
        assert!(super::text_paths("").is_none());
    }
}
