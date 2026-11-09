//! Explorer's group/pinned artwork is separate from WM_SETICON.
use std::{path::{Path, PathBuf}, sync::Mutex};
use windows::{core::{GUID, PCWSTR}, Win32::{
    Foundation::{HWND, PROPERTYKEY},
    System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, StructuredStorage::PROPVARIANT},
    UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow},
}};
const MODEL: GUID = GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3);
const COMMAND: PROPERTYKEY = PROPERTYKEY { fmtid: MODEL, pid: 2 };
const ICON: PROPERTYKEY = PROPERTYKEY { fmtid: MODEL, pid: 3 };
const NAME: PROPERTYKEY = PROPERTYKEY { fmtid: MODEL, pid: 4 };
const ID: PROPERTYKEY = PROPERTYKEY { fmtid: MODEL, pid: 5 };
#[derive(Default)]
struct Applied { stamp: Option<(isize, u64, String)>, icon: String, shortcuts_updated: u64, refreshes: u64, refresh_pending: bool, detached: bool, error: Option<String> }
static APPLIED: Mutex<Applied> = Mutex::new(Applied { stamp: None, icon: String::new(), shortcuts_updated: 0, refreshes: 0, refresh_pending: false, detached: false, error: None });
const REFRESH_TIMER: usize = 0x5042_5249;

// WM_SETICON/property notifications are asynchronous in Explorer. Re-registering
// in the same UI turn lets its taskbar group reuse the preceding artwork. A
// single HWND-owned timer allows those notifications to settle, coalesces rapid
// changes, and works even when paused/minimized without egui painting or polling.
fn schedule_refresh(hwnd: HWND) -> Result<(), String> {
    let timer = unsafe { windows::Win32::UI::WindowsAndMessaging::SetTimer(Some(hwnd), REFRESH_TIMER, 200, Some(refresh_timer)) };
    if timer == 0 { return Err(windows::core::Error::from_thread().to_string()); }
    Ok(())
}
unsafe extern "system" fn refresh_timer(hwnd: HWND, _message: u32, id: usize, _time: u32) {
    use windows::Win32::UI::WindowsAndMessaging::KillTimer;
    if id != REFRESH_TIMER { return; }
    let _ = unsafe { KillTimer(Some(hwnd), REFRESH_TIMER) };
    let detached = APPLIED.lock().ok().and_then(|value| (value.refresh_pending && value.stamp.as_ref().is_some_and(|stamp| stamp.0 == hwnd.0 as isize)).then_some(value.detached));
    let Some(detached) = detached else { return; };
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
    let mut deferred_error = None;
    if !detached {
        // Schedule restoration BEFORE removing anything. Keep the shell's
        // remove/add operations in separate turns so it cannot coalesce them.
        let timer = unsafe { windows::Win32::UI::WindowsAndMessaging::SetTimer(Some(hwnd), REFRESH_TIMER, 100, Some(refresh_timer)) };
        if timer != 0 {
            match crate::platform::windows::detach_taskbar_application_icon(hwnd.0 as isize) {
                Ok(()) => {
                    if let Ok(mut applied) = APPLIED.lock() { applied.detached = true; }
                    if initialized { unsafe { CoUninitialize(); } }
                    return;
                }
                Err(error) => deferred_error = Some(error),
            }
            let _ = unsafe { KillTimer(Some(hwnd), REFRESH_TIMER) };
        } else {
            deferred_error = Some(windows::core::Error::from_thread().to_string());
        }
        // Scheduling/removal failure must not leave the application's button
        // missing: run the existing Add/retry/toolbar recovery immediately.
    }
    let result = crate::platform::windows::refresh_taskbar_application_icon(hwnd.0 as isize)
        .and_then(|()| deferred_error.map_or(Ok(()), Err));
    if initialized { unsafe { CoUninitialize(); } }
    if let Ok(mut applied) = APPLIED.lock() {
        applied.refresh_pending = false;
        applied.detached = false;
        match result {
            Ok(()) => { applied.refreshes += 1; applied.error = None; }
            Err(error) => { log::warn!("Refresh displayed taskbar artwork: {error}"); applied.error = Some(error); }
        }
    }
}

pub fn app_id() -> String {
    let mut id = "Pealayer.Desktop".to_owned();
    if let Ok(instance) = std::env::var("PEALAYER_INSTANCE_ID") {
        let instance: String = instance.chars().filter(char::is_ascii_alphanumeric).take(80).collect();
        if !instance.is_empty() { id.push('.'); id.push_str(&instance); }
    }
    id
}
fn wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}
fn cache_icon(directory: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    use sha2::{Digest, Sha256};
    let digest: String = Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect();
    let path = directory.join(format!("pealayer-{digest}.ico"));
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    if std::fs::read(&path).ok().as_deref() != Some(bytes) { std::fs::write(&path, bytes).map_err(|e| e.to_string())?; }
    Ok(path)
}
fn prune_cache(directory: &Path, keep: &[&Path]) {
    let Ok(entries) = std::fs::read_dir(directory) else { return; };
    let mut owned: Vec<_> = entries.flatten().filter(|entry| {
        let name = entry.file_name().to_string_lossy().into_owned();
        entry.file_type().is_ok_and(|kind| kind.is_file())
            && name.strip_prefix("pealayer-").and_then(|name| name.strip_suffix(".ico"))
                .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
    }).collect();
    owned.sort_by_key(|entry| std::cmp::Reverse(entry.metadata().ok().and_then(|meta| meta.modified().ok())));
    for entry in owned.into_iter().skip(16) {
        if !keep.contains(&entry.path().as_path()) { let _ = std::fs::remove_file(entry.path()); }
    }
}
unsafe fn set_window_properties(hwnd: HWND, icon: &Path, name: &str, executable: &Path) -> windows::core::Result<bool> {
    let store: IPropertyStore = unsafe { SHGetPropertyStoreForWindow(hwnd)? };
    let resource = format!("{},0", icon.display());
    // Reassigning the same ID is a no-op to Explorer's group cache. Remove only
    // this window's association before changing its relaunch artwork, then
    // restore the exact same stable ID. No global icon-cache reset/Explorer
    // restart, new identity, or pin/unpin operation is involved.
    // A fresh HWND may inherit Explorer's cached artwork for our stable app ID
    // from the preceding process. First assignment needs the same deferred
    // refresh as a live change; empty properties do not mean fresh shell pixels.
    let changed = unsafe { store.GetValue(&ICON)? }.to_string() != resource;
    if changed {
        unsafe { store.SetValue(&ID, &PROPVARIANT::default())?; store.Commit()?; }
    }
    // ID last: Windows refreshes the taskbar group at this point.
    for (key, value) in [(COMMAND, format!("\"{}\" --activate", executable.display())),
        (NAME, name.to_owned()), (ICON, resource), (ID, app_id())] {
        unsafe { store.SetValue(&key, &PROPVARIANT::from(value.as_str()))?; }
    }
    unsafe { store.Commit()?; }
    Ok(changed)
}
pub fn clear_window(hwnd: isize) {
    // Window properties are not shortcut properties: release only this HWND's
    // values when it is destroyed, never its saved launcher's branding.
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::KillTimer(Some(HWND(hwnd as *mut _)), REFRESH_TIMER);
        if let Ok(store) = SHGetPropertyStoreForWindow::<IPropertyStore>(HWND(hwnd as *mut _)) {
            for key in [COMMAND, ICON, NAME, ID] { let _ = store.SetValue(&key, &PROPVARIANT::default()); }
            let _ = store.Commit();
        }
    }
    if let Ok(mut applied) = APPLIED.lock()
        && applied.stamp.as_ref().is_some_and(|stamp| stamp.0 == hwnd) { applied.stamp = None; applied.refresh_pending = false; applied.detached = false; }
}
fn same_executable(left: &Path, right: &Path) -> bool {
    let normalize = |path: &Path| path.to_string_lossy().trim_start_matches(r"\\?\").replace('/', "\\").to_ascii_lowercase();
    normalize(left) == normalize(right)
}
/// Only ordinary launchers for this exact executable qualify. Preserve everything else.
unsafe fn update_shortcut(path: &Path, executable: &Path, icon: &Path) -> windows::core::Result<bool> {
    use windows::{core::Interface, Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER, IPersistFile, STGM_READ, STGM_READWRITE},
        Win32::UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH}};
    let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)? };
    let file: IPersistFile = link.cast()?;
    let filename = wide(path);
    unsafe { file.Load(PCWSTR(filename.as_ptr()), STGM_READ)?; }
    let mut target = vec![0u16; 32768];
    let mut args = vec![0u16; 32768];
    unsafe { link.GetPath(&mut target, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32)?; link.GetArguments(&mut args)?; }
    let string = |value: &[u16]| String::from_utf16_lossy(&value[..value.iter().position(|ch| *ch == 0).unwrap_or(value.len())]);
    if !same_executable(Path::new(&string(&target)), executable)
        || !matches!(string(&args).trim(), "" | "--activate") { return Ok(false); }
    let mut current = vec![0u16; 32768];
    let mut index = 0;
    unsafe { link.GetIconLocation(&mut current, &mut index)?; }
    let store: IPropertyStore = link.cast()?;
    let id = unsafe { store.GetValue(&ID)? };
    if same_executable(Path::new(&string(&current)), icon) && index == 0 && id.to_string() == app_id() { return Ok(false); }
    let location = wide(icon);
    unsafe {
        // Don't request write access for unrelated or already-current pins.
        file.Load(PCWSTR(filename.as_ptr()), STGM_READWRITE)?;
        link.SetIconLocation(PCWSTR(location.as_ptr()), 0)?;
        store.SetValue(&ID, &PROPVARIANT::from(app_id().as_str()))?;
        store.Commit()?;
        file.Save(PCWSTR(filename.as_ptr()), true)?;
        use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_UPDATEITEM, SHCNF_PATHW};
        SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATHW, Some(filename.as_ptr().cast()), None);
    }
    Ok(true)
}
pub fn apply(hwnd: isize, source: u64, config: &crate::config::AppConfig, state: crate::branding::PlaybackIconState) {
    let name = crate::config::resolved_app_name(config);
    let stamp = (hwnd, source, name.clone());
    if APPLIED.lock().is_ok_and(|value| value.stamp.as_ref() == Some(&stamp)) { return; }
    let result = (|| -> Result<(PathBuf, u64, bool), String> {
        let directory = crate::config::AppConfig::get_config_path().with_file_name("shell-icons");
        let icon = cache_icon(&directory, &crate::branding::native_icon_bytes(config, state)?)?;
        let default = cache_icon(&directory, &crate::branding::native_icon_bytes(config, crate::branding::PlaybackIconState::Stopped)?)?;
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        let result = (|| -> windows::core::Result<(u64,bool)> {
            let mut updated = 0;
            if std::env::var_os("PEALAYER_INSTANCE_ID").is_none()
                && let Some(roaming) = std::env::var_os("APPDATA") {
                let pins = PathBuf::from(roaming).join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar");
                if let Ok(entries) = std::fs::read_dir(pins) {
                    for entry in entries.flatten().filter(|entry| entry.path().extension().is_some_and(|value| value.eq_ignore_ascii_case("lnk"))) {
                        match unsafe { update_shortcut(&entry.path(), &executable, &default) } {
                            Ok(true) => updated += 1,
                            Ok(false) => {}
                            Err(error) => log::warn!("Refresh Pealayer pinned icon: {error}"),
                        }
                    }
                }
            }
            let changed=unsafe { set_window_properties(HWND(hwnd as *mut _), &icon, &name, &executable)? };
            if changed {
                schedule_refresh(HWND(hwnd as *mut _))
                    .map_err(|error| windows::core::Error::new(windows::core::HRESULT(0x80004005u32 as i32),error))?;
            }
            Ok((updated,changed))
        })();
        if initialized { unsafe { CoUninitialize(); } }
        prune_cache(&directory, &[&icon, &default]);
        result.map(|(updated,changed)| (icon, updated, changed)).map_err(|e| e.to_string())
    })();
    if let Ok(mut applied) = APPLIED.lock() {
        match result {
            Ok((icon, updated, changed)) => {
                applied.stamp = Some(stamp);
                applied.icon = icon.file_name().unwrap_or_default().to_string_lossy().into_owned();
                applied.shortcuts_updated += updated;
                applied.refresh_pending |= changed;
                applied.error = None;
            }
            Err(error) => { log::warn!("Refresh taskbar application branding: {error}"); applied.error = Some(error); }
        }
    }
}
pub fn diagnostics() -> serde_json::Value {
    APPLIED.lock().map(|value| serde_json::json!({"applied":value.stamp.is_some(),
        "icon":value.icon,"app_id":app_id(),"refreshes":value.refreshes,"refresh_pending":value.refresh_pending,"pinned_shortcuts_updated":value.shortcuts_updated,"error":value.error})).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_window_receives_explicit_group_icon_and_relaunch_metadata() {
        use windows::{core::w, Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WINDOW_STYLE}};
        let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        unsafe {
            let window = CreateWindowExW(WINDOW_EX_STYLE::default(), w!("STATIC"), w!("Branding fixture"),
                WINDOW_STYLE::default(), 0, 0, 32, 32, None, None, None, None).unwrap();
            assert!(set_window_properties(window, Path::new(r"C:\fixture\classic.ico"), "Fixture", Path::new(r"C:\fixture\pealayer.exe")).unwrap(), "first HWND assignment must invalidate inherited Explorer artwork too");
            assert!(!set_window_properties(window, Path::new(r"C:\fixture\classic.ico"), "Fixture", Path::new(r"C:\fixture\pealayer.exe")).unwrap(), "unchanged artwork must not churn taskbar membership");
            let store: IPropertyStore = SHGetPropertyStoreForWindow(window).unwrap();
            assert_eq!(store.GetValue(&ICON).unwrap().to_string(), r"C:\fixture\classic.ico,0");
            assert_eq!(store.GetValue(&ID).unwrap().to_string(), app_id());
            assert_eq!(store.GetValue(&NAME).unwrap().to_string(), "Fixture");
            assert_eq!(store.GetValue(&COMMAND).unwrap().to_string(), r#""C:\fixture\pealayer.exe" --activate"#);
            for icon in [r"C:\fixture\modern.ico", r"C:\fixture\classic.ico", r"C:\fixture\modern.ico"] {
                set_window_properties(window,Path::new(icon),"Fixture",Path::new(r"C:\fixture\pealayer.exe")).unwrap();
                assert_eq!(store.GetValue(&ICON).unwrap().to_string(),format!("{icon},0"));
                assert_eq!(store.GetValue(&ID).unwrap().to_string(),app_id());
            }
            clear_window(window.0 as isize);
            for key in [COMMAND, ICON, NAME, ID] { assert!(store.GetValue(&key).unwrap().to_string().is_empty()); }
            DestroyWindow(window).unwrap();
            if initialized { CoUninitialize(); }
        }
    }
    #[test]
    fn shortcut_icon_update_is_target_scoped_idempotent_and_preserves_other_fields() {
        use windows::{core::{Interface, w}, Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER, IPersistFile},
            Win32::UI::Shell::{IShellLinkW, ShellLink}};
        let directory = std::env::temp_dir().join(format!("pealayer-branding-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("launcher.lnk");
        let executable = std::env::current_exe().unwrap();
        let icon = directory.join("classic.ico");
        let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        unsafe {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
            let target = wide(&executable); let filename = wide(&path);
            link.SetPath(PCWSTR(target.as_ptr())).unwrap();
            link.SetDescription(w!("User's launcher caption")).unwrap();
            link.SetArguments(w!("--activate")).unwrap();
            let file: IPersistFile = link.cast().unwrap();
            file.Save(PCWSTR(filename.as_ptr()), true).unwrap();
            assert!(!update_shortcut(&path, Path::new(r"C:\different.exe"), &icon).unwrap());
            assert!(update_shortcut(&path, &executable, &icon).unwrap());
            assert!(!update_shortcut(&path, &executable, &icon).unwrap());
            file.Load(PCWSTR(filename.as_ptr()), windows::Win32::System::Com::STGM_READWRITE).unwrap();
            let mut caption = vec![0; 256]; let mut args = vec![0; 256];
            link.GetDescription(&mut caption).unwrap(); link.GetArguments(&mut args).unwrap();
            assert!(String::from_utf16_lossy(&caption).starts_with("User's launcher caption"));
            assert!(String::from_utf16_lossy(&args).starts_with("--activate"));
            link.SetArguments(w!("--preferences")).unwrap();
            file.Save(PCWSTR(filename.as_ptr()), true).unwrap();
            assert!(!update_shortcut(&path, &executable, &directory.join("other.ico")).unwrap());
            if initialized { CoUninitialize(); }
        }
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn icon_cache_reuses_identical_content_and_keeps_unowned_files() {
        let directory = std::env::temp_dir().join(format!("pealayer-icon-cache-{}", uuid::Uuid::new_v4()));
        let first = cache_icon(&directory, b"one").unwrap();
        let modified = std::fs::metadata(&first).unwrap().modified().unwrap();
        assert_eq!(cache_icon(&directory, b"one").unwrap(), first);
        assert_eq!(std::fs::metadata(&first).unwrap().modified().unwrap(), modified);
        std::fs::write(directory.join("user.ico"), b"preserve").unwrap();
        for index in 0..20 { cache_icon(&directory, &[index]).unwrap(); }
        prune_cache(&directory, &[&first]);
        assert!(first.is_file() && directory.join("user.ico").is_file());
        assert!(std::fs::read_dir(&directory).unwrap().count() <= 18);
        // Exactly this newly allocated fixture directory; no shared/user cache.
        for entry in std::fs::read_dir(&directory).unwrap() { std::fs::remove_file(entry.unwrap().path()).unwrap(); }
        std::fs::remove_dir(directory).unwrap();
    }
}
