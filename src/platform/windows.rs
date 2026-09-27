use std::sync::atomic::{AtomicIsize, Ordering};

static WINDOW_HWND: AtomicIsize = AtomicIsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskbarProgressFlag {
    NoProgress = 0,
    Indeterminate = 1,
    Normal = 2,
    Error = 4,
    Paused = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskbarState {
    pub progress_percent: u32,
    pub is_paused: bool,
    pub is_active: bool,
}

impl Default for TaskbarState {
    fn default() -> Self {
        Self {
            progress_percent: 0,
            is_paused: false,
            is_active: false,
        }
    }
}

impl TaskbarState {
    pub fn to_progress_flag(&self) -> TaskbarProgressFlag {
        if !self.is_active {
            TaskbarProgressFlag::NoProgress
        } else if self.is_paused {
            TaskbarProgressFlag::Paused
        } else {
            TaskbarProgressFlag::Normal
        }
    }
}

pub fn compute_taskbar_state(playback_time: f64, duration: f64, is_paused: bool) -> TaskbarState {
    if duration <= 0.0 || duration.is_nan() || playback_time.is_nan() {
        return TaskbarState {
            progress_percent: 0,
            is_paused,
            is_active: false,
        };
    }

    let clamped_time = playback_time.max(0.0);
    let ratio = (clamped_time / duration).clamp(0.0, 1.0);
    let progress_percent = (ratio * 100.0).round() as u32;

    TaskbarState {
        progress_percent,
        is_paused,
        is_active: true,
    }
}

pub fn register_window_hwnd(hwnd: isize) {
    WINDOW_HWND.store(hwnd, Ordering::SeqCst);
    apply_windows_window_decorations(hwnd);
}

pub fn get_registered_hwnd() -> isize {
    WINDOW_HWND.load(Ordering::SeqCst)
}

#[cfg(target_os = "windows")]
pub fn apply_windows_window_decorations(hwnd_raw: isize) {
    use windows::Win32::Foundation::{BOOL, HWND};
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute,
        DWMWA_CAPTION_COLOR,
        DWMWA_SYSTEMBACKDROP_TYPE,
        DWMWA_TEXT_COLOR,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
        DWMSBT_MAINWINDOW,
    };
    use windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE;

    if hwnd_raw == 0 {
        return;
    }
    let hwnd = HWND(hwnd_raw as *mut _);

    unsafe {
        // 1. Enable immersive dark mode (attribute 20, fallback 19 for older Win10 builds)
        let dark_mode = BOOL::from(true);
        if DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark_mode as *const _ as *const _,
            std::mem::size_of::<BOOL>() as u32,
        ).is_err() {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWINDOWATTRIBUTE(19),
                &dark_mode as *const _ as *const _,
                std::mem::size_of::<BOOL>() as u32,
            );
        }

        // 2. Set Mica backdrop on Windows 11 (build 22621+ attribute 38 = DWMSBT_MAINWINDOW)
        let backdrop = DWMSBT_MAINWINDOW.0 as u32;
        if DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        ).is_err() {
            // Fallback for Windows 11 22000: DWMWA_MICA_EFFECT = 1029
            let mica_legacy = BOOL::from(true);
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWINDOWATTRIBUTE(1029),
                &mica_legacy as *const _ as *const _,
                std::mem::size_of::<BOOL>() as u32,
            );
        }

        // 3. Caption Color: #212121 (RGB 33, 33, 33 -> COLORREF 0x00212121)
        let caption_color: u32 = 0x00212121;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR,
            &caption_color as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );

        // 4. Text Color: White (0x00FFFFFF)
        let text_color: u32 = 0x00FFFFFF;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TEXT_COLOR,
            &text_color as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn apply_windows_window_decorations(_hwnd_raw: isize) {
    // No-op on non-Windows platforms
}


#[cfg(target_os = "windows")]
pub fn update_windows_taskbar_state(progress: f64, duration: f64, is_paused: bool) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{ITaskbarList3, TaskbarList, TBPFLAG};

    let hwnd_raw = get_registered_hwnd();
    if hwnd_raw == 0 {
        return;
    }
    let hwnd = HWND(hwnd_raw as *mut _);

    let state = compute_taskbar_state(progress, duration, is_paused);

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if let Ok(taskbar) = CoCreateInstance::<_, ITaskbarList3>(&TaskbarList, None, CLSCTX_INPROC_SERVER) {
            match state.to_progress_flag() {
                TaskbarProgressFlag::NoProgress => {
                    let _ = taskbar.SetProgressState(hwnd, TBPFLAG(0));
                }
                TaskbarProgressFlag::Normal => {
                    let _ = taskbar.SetProgressState(hwnd, TBPFLAG(2));
                    let _ = taskbar.SetProgressValue(hwnd, state.progress_percent as u64, 100);
                }
                TaskbarProgressFlag::Paused => {
                    let _ = taskbar.SetProgressState(hwnd, TBPFLAG(8));
                    let _ = taskbar.SetProgressValue(hwnd, state.progress_percent as u64, 100);
                }
                _ => {}
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn update_windows_taskbar_state(_progress: f64, _duration: f64, _is_paused: bool) {
    // No-op on non-Windows platforms
}

#[cfg(target_os = "windows")]
pub fn sync_windows_jump_list(recent_media: &[std::path::PathBuf]) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::{SHAddToRecentDocs, SHARD_PATHW};

    for path in recent_media.iter().take(10) {
        if let Some(path_str) = path.to_str() {
            let wide_path: Vec<u16> = OsStr::new(path_str)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            unsafe {
                SHAddToRecentDocs(SHARD_PATHW.0 as u32, Some(wide_path.as_ptr() as *const _));
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn sync_windows_jump_list(_recent_media: &[std::path::PathBuf]) {
    // No-op on non-Windows platforms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taskbar_state_default() {
        let state = TaskbarState::default();
        assert_eq!(state.progress_percent, 0);
        assert!(!state.is_paused);
    }

    #[test]
    fn test_compute_taskbar_state_and_flags() {
        // Inactive / zero duration
        let inactive = compute_taskbar_state(0.0, 0.0, false);
        assert!(!inactive.is_active);
        assert_eq!(inactive.progress_percent, 0);
        assert_eq!(inactive.to_progress_flag(), TaskbarProgressFlag::NoProgress);

        // Playing mid-way
        let playing = compute_taskbar_state(30.0, 60.0, false);
        assert!(playing.is_active);
        assert_eq!(playing.progress_percent, 50);
        assert_eq!(playing.to_progress_flag(), TaskbarProgressFlag::Normal);

        // Paused
        let paused = compute_taskbar_state(30.0, 60.0, true);
        assert!(paused.is_active);
        assert_eq!(paused.progress_percent, 50);
        assert_eq!(paused.to_progress_flag(), TaskbarProgressFlag::Paused);

        // Clamping overflow
        let clamped = compute_taskbar_state(120.0, 60.0, false);
        assert_eq!(clamped.progress_percent, 100);
    }

    #[test]
    fn test_decoration_colorref_conversion() {
        // RGB(33, 33, 33) => COLORREF 0x00212121
        let r: u32 = 33;
        let g: u32 = 33;
        let b: u32 = 33;
        let colorref = r | (g << 8) | (b << 16);
        assert_eq!(colorref, 0x00212121);
    }
}

