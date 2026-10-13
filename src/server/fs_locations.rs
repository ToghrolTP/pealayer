//! Read-only host locations. Never mount shares, resolve shortcuts interactively,
//! enumerate the LAN, or probe every network/removable target for availability.
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct FileLocation {
    pub name: String,
    pub path: String,
    pub kind: &'static str,
}

pub(super) fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
    }
}

fn add(out: &mut Vec<FileLocation>, name: impl Into<String>, path: PathBuf, kind: &'static str) {
    if !path.is_absolute() {
        return;
    }
    let path = display_path(&path);
    let key = |value: &str| {
        if cfg!(windows) {
            value
                .replace('/', "\\")
                .trim_end_matches('\\')
                .to_lowercase()
        } else {
            value.trim_end_matches('/').to_string()
        }
    };
    if out.iter().any(|item| key(&item.path) == key(&path)) {
        return;
    }
    out.push(FileLocation {
        name: name.into(),
        path,
        kind,
    });
}

pub fn locations(recent_media: &[PathBuf]) -> Vec<FileLocation> {
    let mut result = Vec::new();
    #[cfg(windows)]
    windows_locations(&mut result);
    #[cfg(not(windows))]
    {
        add(&mut result, "Root", PathBuf::from("/"), "drive");
        if let Some(home) = std::env::var_os("HOME") {
            add(&mut result, "Home", PathBuf::from(home), "folder");
        }
        // Only inspect immediate mount directories, not mounted contents.
        for root in ["/Volumes", "/media", "/mnt"] {
            if let Ok(entries) = std::fs::read_dir(root) {
                for entry in entries.flatten().take(128) {
                    if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                        add(
                            &mut result,
                            entry.file_name().to_string_lossy(),
                            entry.path(),
                            "drive",
                        );
                    }
                }
            }
        }
    }
    if let Ok(current) = std::env::current_dir() {
        add(&mut result, "Current folder", current, "folder");
    }
    for media in recent_media.iter().rev().take(100) {
        if let Some(parent) = media.parent().filter(|path| path.is_absolute()) {
            let name = parent
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| display_path(parent));
            add(&mut result, name, parent.to_path_buf(), "recent");
        }
    }
    result
}

#[cfg(windows)]
fn windows_locations(out: &mut Vec<FileLocation>) {
    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, GetDriveTypeW, GetLogicalDriveStringsW, WIN32_FIND_DATAW,
    };
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoTaskMemFree, CoUninitialize, IPersistFile, STGM_READ,
    };
    use windows::Win32::UI::Shell::*;
    use windows::core::{Interface, PCWSTR};

    unsafe fn known(id: &windows::core::GUID) -> Option<PathBuf> {
        let value = unsafe { SHGetKnownFolderPath(id, KF_FLAG_DONT_VERIFY, None) }.ok()?;
        let text = unsafe { value.to_string() }.ok();
        unsafe { CoTaskMemFree(Some(value.0.cast())) };
        text.map(PathBuf::from)
    }
    unsafe fn remote(path: &Path) -> bool {
        let text = path.to_string_lossy();
        if text.starts_with(r"\\") {
            return true;
        }
        let chars: Vec<u16> = text.encode_utf16().take(2).collect();
        if chars.len() == 2 && chars[1] == b':' as u16 {
            let root = [chars[0], chars[1], b'\\' as u16, 0];
            return unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) } == 4;
        }
        false
    }
    unsafe fn shortcut(file: &Path, network_folder: bool) -> Option<PathBuf> {
        let link: IShellLinkW =
            unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }.ok()?;
        let persistent: IPersistFile = link.cast().ok()?;
        use std::os::windows::ffi::OsStrExt;
        let file: Vec<u16> = file.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe { persistent.Load(PCWSTR(file.as_ptr()), STGM_READ) }.ok()?;
        let mut text = [0u16; 32768];
        let mut attributes = WIN32_FIND_DATAW::default();
        // GetPath reads the stored target. Do not Resolve: that can search for
        // missing targets, contact servers, update links or show an auth dialog.
        unsafe { link.GetPath(&mut text, &mut attributes, SLGP_RAWPATH.0 as u32) }.ok()?;
        let end = text.iter().position(|value| *value == 0)?;
        let path = PathBuf::from(String::from_utf16_lossy(&text[..end]));
        if network_folder
            || attributes.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0
            || (!unsafe { remote(&path) } && path.is_dir())
        {
            Some(path)
        } else {
            None
        }
    }
    unsafe {
        let mut buffer = [0u16; 512]; // 26 drive roots need only 105 UTF-16 units.
        let count = GetLogicalDriveStringsW(Some(&mut buffer)) as usize;
        if count > 0 && count < buffer.len() {
            for root in buffer[..count]
                .split(|value| *value == 0)
                .filter(|root| !root.is_empty())
            {
                let path = String::from_utf16_lossy(root);
                let wide: Vec<u16> = root.iter().copied().chain(Some(0)).collect();
                let kind = match GetDriveTypeW(PCWSTR(wide.as_ptr())) {
                    4 => "network",
                    2 | 5 => "removable",
                    _ => "drive",
                };
                add(out, path.clone(), PathBuf::from(path), kind);
            }
        }
        // Remembered mappings may not be connected in this logon session. Offer
        // their UNC target without claiming it is mounted or presently online.
        if let Ok(network) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey_with_flags("Network", winreg::enums::KEY_READ)
        {
            for letter in network.enum_keys().flatten().take(26) {
                if letter.len() != 1 || !letter.as_bytes()[0].is_ascii_alphabetic() {
                    continue;
                }
                if out
                    .iter()
                    .any(|item| item.path.eq_ignore_ascii_case(&format!("{letter}:\\")))
                {
                    continue;
                }
                if let Ok(key) = network.open_subkey_with_flags(&letter, winreg::enums::KEY_READ) {
                    if let Ok(remote) = key.get_value::<String, _>("RemotePath") {
                        if remote.starts_with(r"\\") && remote.len() < 32768 {
                            add(
                                out,
                                format!("{letter}: — {remote}"),
                                PathBuf::from(remote),
                                "network",
                            );
                        }
                    }
                }
            }
        }
        for (name, id) in [
            ("Home", FOLDERID_Profile),
            ("Desktop", FOLDERID_Desktop),
            ("Documents", FOLDERID_Documents),
            ("Downloads", FOLDERID_Downloads),
            ("Videos", FOLDERID_Videos),
            ("Music", FOLDERID_Music),
            ("Pictures", FOLDERID_Pictures),
        ] {
            if let Some(path) = known(&id) {
                add(out, name, path, "folder");
            }
        }
        let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        if initialized {
            for id in [FOLDERID_NetHood, FOLDERID_Links, FOLDERID_Desktop] {
                if let Some(folder) = known(&id) {
                    if remote(&folder) {
                        continue;
                    }
                    if let Ok(entries) = std::fs::read_dir(folder) {
                        for entry in entries.flatten().take(128) {
                            let file = entry.path();
                            let (link, name) = if file
                                .extension()
                                .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
                            {
                                (
                                    file.clone(),
                                    file.file_stem()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                )
                            } else if id == FOLDERID_NetHood
                                && entry.file_type().is_ok_and(|kind| kind.is_dir())
                            {
                                (
                                    file.join("target.lnk"),
                                    entry.file_name().to_string_lossy().to_string(),
                                )
                            } else {
                                continue;
                            };
                            if let Some(target) = shortcut(&link, id == FOLDERID_NetHood) {
                                add(out, name, target, "shortcut");
                            }
                        }
                    }
                }
            }
            CoUninitialize();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn namespace_paths_keep_unc_roots() {
        assert_eq!(
            display_path(Path::new(r"\\?\UNC\server\share\films")),
            r"\\server\share\films"
        );
        assert_eq!(display_path(Path::new(r"\\?\C:\Films")), r"C:\Films");
    }
    #[test]
    fn locations_reject_relative_paths_and_deduplicate() {
        let mut values = Vec::new();
        add(&mut values, "relative", PathBuf::from("relative"), "recent");
        assert!(values.is_empty());
        let root = std::env::temp_dir();
        add(&mut values, "one", root.clone(), "folder");
        add(&mut values, "two", root, "recent");
        assert_eq!(values.len(), 1);
    }
}
