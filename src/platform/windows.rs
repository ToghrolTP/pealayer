use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};

static WINDOW_HWND: AtomicIsize = AtomicIsize::new(0);
static WINDOW_DARK_THEME: AtomicBool = AtomicBool::new(true);
static WINDOW_DWM_THEMING: AtomicBool = AtomicBool::new(true);
static WINDOW_MICA_BACKDROP: AtomicBool = AtomicBool::new(false);
static ORIGINAL_WINDOW_PROC: AtomicIsize = AtomicIsize::new(0);
static SHELL_COMMAND: AtomicU32 = AtomicU32::new(0);
static SHELL_PAUSED: AtomicBool = AtomicBool::new(true);
static SHELL_MUTED: AtomicBool = AtomicBool::new(false);
static SHELL_HAS_MEDIA: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
pub struct GuiOwnershipGuard(windows::Win32::Foundation::HANDLE);

#[cfg(target_os = "windows")]
impl Drop for GuiOwnershipGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(target_os = "windows")]
pub enum GuiOwnership {
    Primary(GuiOwnershipGuard),
    Existing,
}

fn gui_identity_hash(app_identity: &str) -> u64 {
    // FNV-1a is deterministic across processes and Rust releases. This is a
    // namespace discriminator, not a security boundary.
    app_identity
        .trim()
        .to_lowercase()
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
}

pub fn gui_mutex_name(app_identity: &str) -> String {
    format!(
        "Local\\Pealayer.GuiOwner.{:016x}",
        gui_identity_hash(app_identity)
    )
}

#[cfg(target_os = "windows")]
pub fn current_session_id() -> Result<u32, String> {
    use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
    use windows::Win32::System::Threading::GetCurrentProcessId;

    let mut session_id = 0;
    unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session_id) }
        .map_err(|error| format!("resolve current Windows session: {error}"))?;
    Ok(session_id)
}

#[cfg(target_os = "windows")]
pub fn acquire_gui_ownership(app_identity: &str) -> Result<GuiOwnership, String> {
    use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::PCWSTR;

    let wide_name: Vec<u16> = gui_mutex_name(app_identity)
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let handle = unsafe { CreateMutexW(None, false, PCWSTR(wide_name.as_ptr())) }
        .map_err(|error| format!("create GUI ownership mutex: {error}"))?;
    let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if already_exists {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(handle);
        }
        Ok(GuiOwnership::Existing)
    } else {
        Ok(GuiOwnership::Primary(GuiOwnershipGuard(handle)))
    }
}

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
    pub has_error: bool,
}

impl Default for TaskbarState {
    fn default() -> Self {
        Self {
            progress_percent: 0,
            is_paused: false,
            is_active: false,
            has_error: false,
        }
    }
}

impl TaskbarState {
    pub fn to_progress_flag(&self) -> TaskbarProgressFlag {
        if self.has_error {
            TaskbarProgressFlag::Error
        } else if !self.is_active {
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
            has_error: false,
        };
    }

    let clamped_time = playback_time.max(0.0);
    let ratio = (clamped_time / duration).clamp(0.0, 1.0);
    let progress_percent = (ratio * 100.0).round() as u32;

    TaskbarState {
        progress_percent,
        is_paused,
        is_active: true,
        has_error: false,
    }
}

pub fn compute_taskbar_state_with_error(
    playback_time: f64,
    duration: f64,
    is_paused: bool,
    has_error: bool,
) -> TaskbarState {
    if has_error {
        return TaskbarState {
            progress_percent: 100,
            is_paused,
            is_active: true,
            has_error: true,
        };
    }
    let mut state = compute_taskbar_state(playback_time, duration, is_paused);
    state.has_error = false;
    state
}

pub fn register_window_hwnd(hwnd: isize) {
    WINDOW_HWND.store(hwnd, Ordering::SeqCst);
    apply_windows_window_decorations(hwnd);
}

pub fn get_registered_hwnd() -> isize {
    WINDOW_HWND.load(Ordering::SeqCst)
}

pub fn set_window_theme(dark: bool) {
    let previous = WINDOW_DARK_THEME.swap(dark, Ordering::SeqCst);
    let hwnd = get_registered_hwnd();
    if hwnd != 0 && previous != dark {
        apply_windows_window_decorations(hwnd);
    }
}

pub fn configure_window_composition(dwm_theming: bool, mica_backdrop: bool) {
    let dwm_changed = WINDOW_DWM_THEMING.swap(dwm_theming, Ordering::SeqCst) != dwm_theming;
    let mica_changed = WINDOW_MICA_BACKDROP.swap(mica_backdrop, Ordering::SeqCst) != mica_backdrop;
    let hwnd = get_registered_hwnd();
    if hwnd != 0 && (dwm_changed || mica_changed) {
        apply_windows_window_decorations(hwnd);
    }
}

#[cfg(any(target_os = "windows", test))]
fn decoration_colors(dark: bool) -> (u32, u32) {
    if dark {
        (0x00212121, 0x00FFFFFF)
    } else {
        (0x00F4F4F4, 0x00111111)
    }
}

#[cfg(target_os = "windows")]
pub fn apply_windows_window_decorations(hwnd_raw: isize) {
    use windows::Win32::Foundation::{BOOL, HWND};
    use windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE;
    use windows::Win32::Graphics::Dwm::{
        DWMSBT_MAINWINDOW, DWMSBT_NONE, DWMWA_CAPTION_COLOR, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_TEXT_COLOR,
        DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute,
    };

    if hwnd_raw == 0 {
        return;
    }
    let hwnd = HWND(hwnd_raw as *mut _);
    let dark = WINDOW_DARK_THEME.load(Ordering::SeqCst);
    let dwm_theming = WINDOW_DWM_THEMING.load(Ordering::SeqCst);
    let mica_backdrop = WINDOW_MICA_BACKDROP.load(Ordering::SeqCst);
    let (caption_color, text_color) = decoration_colors(dark);

    unsafe {
        // 1. Enable immersive dark mode (attribute 20, fallback 19 for older Win10 builds)
        let dark_mode = BOOL::from(dwm_theming && dark);
        if DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark_mode as *const _ as *const _,
            std::mem::size_of::<BOOL>() as u32,
        )
        .is_err()
        {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWINDOWATTRIBUTE(19),
                &dark_mode as *const _ as *const _,
                std::mem::size_of::<BOOL>() as u32,
            );
        }

        // Pealayer presents an opaque OpenGL swapchain. Mica is optional because
        // some drivers briefly expose the backdrop while swapping buffers.
        let backdrop = if mica_backdrop {
            DWMSBT_MAINWINDOW.0
        } else {
            DWMSBT_NONE.0
        } as u32;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );

        // Keep native caption and text colors aligned with the app theme, or
        // return them to the system-selected default when DWM theming is off.
        let caption_color = if dwm_theming { caption_color } else { 0xFFFF_FFFF };
        let text_color = if dwm_theming { text_color } else { 0xFFFF_FFFF };
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR,
            &caption_color as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );

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
pub fn update_windows_taskbar_state_ext(
    progress: f64,
    duration: f64,
    is_paused: bool,
    has_error: bool,
) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::Win32::UI::Shell::{ITaskbarList3, TBPFLAG, TaskbarList};

    let hwnd_raw = get_registered_hwnd();
    if hwnd_raw == 0 {
        return;
    }
    let hwnd = HWND(hwnd_raw as *mut _);

    let state = compute_taskbar_state_with_error(progress, duration, is_paused, has_error);

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if let Ok(taskbar) =
            CoCreateInstance::<_, ITaskbarList3>(&TaskbarList, None, CLSCTX_INPROC_SERVER)
        {
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
                TaskbarProgressFlag::Error => {
                    let _ = taskbar.SetProgressState(hwnd, TBPFLAG(4));
                    let _ = taskbar.SetProgressValue(hwnd, 100, 100);
                }
                _ => {}
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn update_windows_taskbar_state(progress: f64, duration: f64, is_paused: bool) {
    update_windows_taskbar_state_ext(progress, duration, is_paused, false);
}

#[cfg(not(target_os = "windows"))]
pub fn update_windows_taskbar_state_ext(
    _progress: f64,
    _duration: f64,
    _is_paused: bool,
    _has_error: bool,
) {
    // No-op on non-Windows platforms
}

#[cfg(not(target_os = "windows"))]
pub fn update_windows_taskbar_state(_progress: f64, _duration: f64, _is_paused: bool) {
    // No-op on non-Windows platforms
}

#[cfg(target_os = "windows")]
pub fn sync_windows_jump_list(recent_media: &[std::path::PathBuf]) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::{SHARD_PATHW, SHAddToRecentDocs};

    // Windows' shell history is file-oriented and may be visible outside the
    // app. Keep network locations (especially credential-bearing URLs) in
    // Pealayer's own recent list only.
    for path in recent_media
        .iter()
        .filter(|path| !crate::media::is_remote_media_target(&path.to_string_lossy()))
        .take(10)
    {
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

pub const THUMB_BUTTON_PREV: u32 = 1001;
pub const THUMB_BUTTON_PLAYPAUSE: u32 = 1002;
pub const THUMB_BUTTON_NEXT: u32 = 1003;

pub fn thumbnail_button_tooltip(button_id: u32, is_paused: bool) -> &'static str {
    match button_id {
        THUMB_BUTTON_PREV => "Previous",
        THUMB_BUTTON_PLAYPAUSE => {
            if is_paused {
                "Play"
            } else {
                "Pause"
            }
        }
        THUMB_BUTTON_NEXT => "Next",
        _ => "",
    }
}

#[cfg(target_os = "windows")]
fn str_to_u16_buf_260(text: &str) -> [u16; 260] {
    let mut buf = [0u16; 260];
    for (i, code_unit) in text.encode_utf16().take(259).enumerate() {
        buf[i] = code_unit;
    }
    buf
}

#[cfg(target_os = "windows")]
pub fn init_taskbar_thumbnail_toolbar(hwnd_raw: isize) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::Win32::UI::Shell::{
        ITaskbarList3, THB_FLAGS, THB_TOOLTIP, THBF_ENABLED, THUMBBUTTON, TaskbarList,
    };
    use windows::Win32::UI::WindowsAndMessaging::HICON;

    if hwnd_raw == 0 {
        return Err("invalid window handle (HWND is 0)".to_string());
    }
    let hwnd = HWND(hwnd_raw as *mut _);

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let taskbar: ITaskbarList3 = CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("failed to instantiate ITaskbarList3: {e}"))?;

        let buttons = [
            THUMBBUTTON {
                dwMask: THB_FLAGS | THB_TOOLTIP,
                iId: THUMB_BUTTON_PREV,
                iBitmap: 0,
                hIcon: HICON(std::ptr::null_mut()),
                szTip: str_to_u16_buf_260(thumbnail_button_tooltip(THUMB_BUTTON_PREV, false)),
                dwFlags: THBF_ENABLED,
            },
            THUMBBUTTON {
                dwMask: THB_FLAGS | THB_TOOLTIP,
                iId: THUMB_BUTTON_PLAYPAUSE,
                iBitmap: 0,
                hIcon: HICON(std::ptr::null_mut()),
                szTip: str_to_u16_buf_260(thumbnail_button_tooltip(THUMB_BUTTON_PLAYPAUSE, true)),
                dwFlags: THBF_ENABLED,
            },
            THUMBBUTTON {
                dwMask: THB_FLAGS | THB_TOOLTIP,
                iId: THUMB_BUTTON_NEXT,
                iBitmap: 0,
                hIcon: HICON(std::ptr::null_mut()),
                szTip: str_to_u16_buf_260(thumbnail_button_tooltip(THUMB_BUTTON_NEXT, false)),
                dwFlags: THBF_ENABLED,
            },
        ];

        taskbar
            .ThumbBarAddButtons(hwnd, &buttons)
            .map_err(|e| format!("ThumbBarAddButtons failed: {e}"))?;

        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn init_taskbar_thumbnail_toolbar(_hwnd_raw: isize) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn update_taskbar_thumbnail_buttons(
    hwnd_raw: isize,
    is_paused: bool,
    has_media: bool,
) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::Win32::UI::Shell::{
        ITaskbarList3, THB_FLAGS, THB_TOOLTIP, THBF_DISABLED, THBF_ENABLED, THUMBBUTTON,
        TaskbarList,
    };
    use windows::Win32::UI::WindowsAndMessaging::HICON;

    if hwnd_raw == 0 {
        return Err("invalid window handle (HWND is 0)".to_string());
    }
    let hwnd = HWND(hwnd_raw as *mut _);

    let flags = if has_media {
        THBF_ENABLED
    } else {
        THBF_DISABLED
    };

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let taskbar: ITaskbarList3 = CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("failed to instantiate ITaskbarList3: {e}"))?;

        let buttons = [
            THUMBBUTTON {
                dwMask: THB_FLAGS | THB_TOOLTIP,
                iId: THUMB_BUTTON_PREV,
                iBitmap: 0,
                hIcon: HICON(std::ptr::null_mut()),
                szTip: str_to_u16_buf_260(thumbnail_button_tooltip(THUMB_BUTTON_PREV, is_paused)),
                dwFlags: flags,
            },
            THUMBBUTTON {
                dwMask: THB_FLAGS | THB_TOOLTIP,
                iId: THUMB_BUTTON_PLAYPAUSE,
                iBitmap: 0,
                hIcon: HICON(std::ptr::null_mut()),
                szTip: str_to_u16_buf_260(thumbnail_button_tooltip(
                    THUMB_BUTTON_PLAYPAUSE,
                    is_paused,
                )),
                dwFlags: flags,
            },
            THUMBBUTTON {
                dwMask: THB_FLAGS | THB_TOOLTIP,
                iId: THUMB_BUTTON_NEXT,
                iBitmap: 0,
                hIcon: HICON(std::ptr::null_mut()),
                szTip: str_to_u16_buf_260(thumbnail_button_tooltip(THUMB_BUTTON_NEXT, is_paused)),
                dwFlags: flags,
            },
        ];

        taskbar
            .ThumbBarUpdateButtons(hwnd, &buttons)
            .map_err(|e| format!("ThumbBarUpdateButtons failed: {e}"))?;

        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn update_taskbar_thumbnail_buttons(
    _hwnd_raw: isize,
    _is_paused: bool,
    _has_media: bool,
) -> Result<(), String> {
    Ok(())
}

pub fn compute_thumbnail_clip_ratio(
    window_width: f32,
    window_height: f32,
    video_rect: [f32; 4],
) -> [f32; 4] {
    if window_width <= 0.0
        || window_height <= 0.0
        || window_width.is_nan()
        || window_height.is_nan()
    {
        return [0.0, 0.0, 1.0, 1.0];
    }

    let clamp_ratio = |val: f32| -> f32 {
        if val.is_nan() {
            0.0
        } else {
            val.clamp(0.0, 1.0)
        }
    };

    [
        clamp_ratio(video_rect[0] / window_width),
        clamp_ratio(video_rect[1] / window_height),
        clamp_ratio(video_rect[2] / window_width),
        clamp_ratio(video_rect[3] / window_height),
    ]
}

#[cfg(target_os = "windows")]
pub fn configure_video_taskbar_thumbnail(hwnd_raw: isize) -> Result<(), String> {
    use windows::Win32::Foundation::{BOOL, HWND};
    use windows::Win32::Graphics::Dwm::{DWMWA_FORCE_ICONIC_REPRESENTATION, DwmSetWindowAttribute};

    if hwnd_raw == 0 {
        return Err("invalid window handle (HWND is 0)".to_string());
    }
    let hwnd = HWND(hwnd_raw as *mut _);
    let enable = BOOL::from(true);
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_FORCE_ICONIC_REPRESENTATION,
            &enable as *const _ as *const _,
            std::mem::size_of::<BOOL>() as u32,
        )
        .map_err(|e| {
            format!("DwmSetWindowAttribute DWMWA_FORCE_ICONIC_REPRESENTATION failed: {e}")
        })?;
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn configure_video_taskbar_thumbnail(_hwnd_raw: isize) -> Result<(), String> {
    Ok(())
}

pub const WM_TRAYICON: u32 = 0x8000 + 101; // WM_APP + 101
pub const TRAY_CMD_PLAYPAUSE: u32 = 2001;
pub const TRAY_CMD_MUTE: u32 = 2002;
pub const TRAY_CMD_OPEN: u32 = 2003;
pub const TRAY_CMD_EXIT: u32 = 2004;
pub const TRAY_CMD_SHOW: u32 = 2005;

pub fn tray_menu_label(cmd: u32, active: bool) -> &'static str {
    match cmd {
        TRAY_CMD_PLAYPAUSE => {
            if active {
                "Play"
            } else {
                "Pause"
            }
        }
        TRAY_CMD_MUTE => {
            if active {
                "Unmute"
            } else {
                "Mute"
            }
        }
        TRAY_CMD_OPEN => "Open Media...",
        TRAY_CMD_EXIT => "Exit",
        TRAY_CMD_SHOW => "Show Pealayer",
        _ => "",
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn shell_window_proc(
    hwnd: windows::Win32::Foundation::HWND,
    message: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, WM_COMMAND, WM_CONTEXTMENU, WM_LBUTTONDBLCLK, WM_RBUTTONUP, WNDPROC,
    };

    if message == WM_TRAYICON {
        let mouse_message = lparam.0 as u32;
        if mouse_message == WM_RBUTTONUP || mouse_message == WM_CONTEXTMENU {
            if let Some(command) = show_tray_popup_menu(
                hwnd.0 as isize,
                SHELL_PAUSED.load(Ordering::Relaxed),
                SHELL_MUTED.load(Ordering::Relaxed),
                SHELL_HAS_MEDIA.load(Ordering::Relaxed),
            ) {
                SHELL_COMMAND.store(command, Ordering::Release);
            }
            return LRESULT(0);
        }
        if mouse_message == WM_LBUTTONDBLCLK {
            SHELL_COMMAND.store(TRAY_CMD_SHOW, Ordering::Release);
            return LRESULT(0);
        }
    } else if message == WM_COMMAND {
        let packed = wparam.0 as u32;
        let notification = (packed >> 16) & 0xffff;
        let command = packed & 0xffff;
        const THBN_CLICKED: u32 = 0x1800;
        if notification == THBN_CLICKED
            && matches!(
                command,
                THUMB_BUTTON_PREV | THUMB_BUTTON_PLAYPAUSE | THUMB_BUTTON_NEXT
            )
        {
            SHELL_COMMAND.store(command, Ordering::Release);
            return LRESULT(0);
        }
    }

    let original = ORIGINAL_WINDOW_PROC.load(Ordering::Acquire);
    if original == 0 {
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, message, wparam, lparam)
        }
    } else {
        let original: WNDPROC = unsafe { std::mem::transmute(original) };
        unsafe { CallWindowProcW(original, hwnd, message, wparam, lparam) }
    }
}

#[cfg(target_os = "windows")]
pub fn install_shell_message_hook(hwnd_raw: isize) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GWLP_WNDPROC, SetWindowLongPtrW};

    if hwnd_raw == 0 {
        return Err("invalid window handle (HWND is 0)".to_string());
    }
    if ORIGINAL_WINDOW_PROC.load(Ordering::Acquire) != 0 {
        return Ok(());
    }
    let previous = unsafe {
        SetWindowLongPtrW(
            HWND(hwnd_raw as *mut _),
            GWLP_WNDPROC,
            shell_window_proc as *const () as isize,
        )
    };
    if previous == 0 {
        return Err("SetWindowLongPtrW GWLP_WNDPROC failed".to_string());
    }
    ORIGINAL_WINDOW_PROC.store(previous, Ordering::Release);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn install_shell_message_hook(_hwnd_raw: isize) -> Result<(), String> {
    Ok(())
}

pub fn update_shell_command_state(is_paused: bool, is_muted: bool, has_media: bool) {
    SHELL_PAUSED.store(is_paused, Ordering::Relaxed);
    SHELL_MUTED.store(is_muted, Ordering::Relaxed);
    SHELL_HAS_MEDIA.store(has_media, Ordering::Relaxed);
}

pub fn take_shell_command() -> Option<u32> {
    let command = SHELL_COMMAND.swap(0, Ordering::AcqRel);
    (command != 0).then_some(command)
}

#[cfg(target_os = "windows")]
fn str_to_u16_buf_128(text: &str) -> [u16; 128] {
    let mut buf = [0u16; 128];
    for (i, code_unit) in text.encode_utf16().take(127).enumerate() {
        buf[i] = code_unit;
    }
    buf
}

#[cfg(target_os = "windows")]
pub fn register_system_tray_icon(hwnd_raw: isize, tip: &str) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Shell::{
        NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NOTIFYICONDATAW, Shell_NotifyIconW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GCLP_HICON, GetClassLongPtrW, HICON, LoadIconW};
    use windows::core::PCWSTR;

    if hwnd_raw == 0 {
        return Err("invalid window handle (HWND is 0)".to_string());
    }
    let hwnd = HWND(hwnd_raw as *mut _);

    unsafe {
        let module =
            GetModuleHandleW(None).map_err(|error| format!("GetModuleHandleW failed: {error}"))?;
        let instance: windows::Win32::Foundation::HINSTANCE = module.into();
        let hicon = LoadIconW(Some(&instance), PCWSTR(1usize as *const u16))
            .unwrap_or_else(|_| HICON(GetClassLongPtrW(hwnd, GCLP_HICON) as *mut _));
        if hicon.0.is_null() {
            return Err("packaged application icon is unavailable".to_string());
        }

        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: hicon,
            szTip: str_to_u16_buf_128(tip),
            ..Default::default()
        };

        let res = Shell_NotifyIconW(NIM_ADD, &mut nid);
        if res.as_bool() {
            Ok(())
        } else {
            Err("Shell_NotifyIconW NIM_ADD failed".to_string())
        }
    }
}

#[cfg(target_os = "windows")]
pub fn show_system_notification(hwnd_raw: isize, title: &str, message: &str) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::{
        NIF_INFO, NIIF_INFO, NIM_MODIFY, NOTIFYICONDATAW, Shell_NotifyIconW,
    };
    if hwnd_raw == 0 {
        return Err("invalid window handle (HWND is 0)".to_string());
    }
    let mut info = [0u16; 256];
    for (index, unit) in message.encode_utf16().take(255).enumerate() {
        info[index] = unit;
    }
    let mut info_title = [0u16; 64];
    for (index, unit) in title.encode_utf16().take(63).enumerate() {
        info_title[index] = unit;
    }
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: HWND(hwnd_raw as *mut _),
        uID: 1,
        uFlags: NIF_INFO,
        szInfo: info,
        szInfoTitle: info_title,
        dwInfoFlags: NIIF_INFO,
        ..Default::default()
    };
    let result = unsafe { Shell_NotifyIconW(NIM_MODIFY, &mut nid) };
    result
        .as_bool()
        .then_some(())
        .ok_or_else(|| "Shell_NotifyIconW notification failed".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn show_system_notification(
    _hwnd_raw: isize,
    _title: &str,
    _message: &str,
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn update_system_tray_icon(hwnd_raw: isize, tip: &str) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::{NIF_TIP, NIM_MODIFY, NOTIFYICONDATAW, Shell_NotifyIconW};

    if hwnd_raw == 0 {
        return Err("invalid window handle (HWND is 0)".to_string());
    }
    let hwnd = HWND(hwnd_raw as *mut _);

    unsafe {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_TIP,
            szTip: str_to_u16_buf_128(tip),
            ..Default::default()
        };

        let res = Shell_NotifyIconW(NIM_MODIFY, &mut nid);
        if res.as_bool() {
            Ok(())
        } else {
            Err("Shell_NotifyIconW NIM_MODIFY failed".to_string())
        }
    }
}

#[cfg(target_os = "windows")]
pub fn remove_system_tray_icon(hwnd_raw: isize) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::{NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW};

    if hwnd_raw == 0 {
        return Err("invalid window handle (HWND is 0)".to_string());
    }
    let hwnd = HWND(hwnd_raw as *mut _);

    unsafe {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            ..Default::default()
        };

        let res = Shell_NotifyIconW(NIM_DELETE, &mut nid);
        if res.as_bool() {
            Ok(())
        } else {
            Err("Shell_NotifyIconW NIM_DELETE failed".to_string())
        }
    }
}

#[cfg(target_os = "windows")]
fn install_tray_menu_icons(menu: isize) -> Vec<isize> {
    #[repr(C)]
    struct BitmapInfoHeader {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bit_count: u16,
        compression: u32,
        size_image: u32,
        x_pixels_per_meter: i32,
        y_pixels_per_meter: i32,
        colors_used: u32,
        colors_important: u32,
    }
    #[repr(C)]
    struct RgbQuad {
        blue: u8,
        green: u8,
        red: u8,
        reserved: u8,
    }
    #[repr(C)]
    struct BitmapInfo {
        header: BitmapInfoHeader,
        colors: [RgbQuad; 1],
    }
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateDIBSection(
            dc: isize,
            info: *const BitmapInfo,
            usage: u32,
            bits: *mut *mut std::ffi::c_void,
            section: isize,
            offset: u32,
        ) -> isize;
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetMenuItemBitmaps(
            menu: isize,
            item: u32,
            flags: u32,
            unchecked: isize,
            checked: isize,
        ) -> i32;
        fn GetSysColor(index: i32) -> u32;
    }

    const WINDOW: [u16; 16] = [
        0x0000, 0x7FFE, 0x4002, 0x5FFA, 0x4002, 0x4002, 0x4002, 0x4002, 0x4002, 0x4002, 0x4002,
        0x4002, 0x4002, 0x7FFE, 0x0000, 0x0000,
    ];
    const PLAY: [u16; 16] = [
        0x0000, 0x1000, 0x1800, 0x1C00, 0x1E00, 0x1F00, 0x1F80, 0x1FC0, 0x1FC0, 0x1F80, 0x1F00,
        0x1E00, 0x1C00, 0x1800, 0x1000, 0x0000,
    ];
    const SPEAKER: [u16; 16] = [
        0x0000, 0x0300, 0x0700, 0x0F30, 0x1F18, 0x3F0C, 0x7F66, 0x7F62, 0x7F62, 0x7F66, 0x3F0C,
        0x1F18, 0x0F30, 0x0700, 0x0300, 0x0000,
    ];
    const FOLDER: [u16; 16] = [
        0x0000, 0x0000, 0x3C00, 0x4200, 0x7FF0, 0x4010, 0x4010, 0x7FFE, 0x4002, 0x4002, 0x4002,
        0x4002, 0x4002, 0x7FFE, 0x0000, 0x0000,
    ];
    const CLOSE: [u16; 16] = [
        0x0000, 0x4002, 0x6006, 0x300C, 0x1818, 0x0C30, 0x0660, 0x03C0, 0x03C0, 0x0660, 0x0C30,
        0x1818, 0x300C, 0x6006, 0x4002, 0x0000,
    ];

    fn create_argb_bitmap(mask: [u16; 16], color: u32) -> isize {
        const BI_RGB: u32 = 0;
        const DIB_RGB_COLORS: u32 = 0;
        let info = BitmapInfo {
            header: BitmapInfoHeader {
                size: std::mem::size_of::<BitmapInfoHeader>() as u32,
                width: 16,
                // Negative height creates a top-down bitmap, matching the icon rows.
                height: -16,
                planes: 1,
                bit_count: 32,
                compression: BI_RGB,
                size_image: 16 * 16 * 4,
                x_pixels_per_meter: 0,
                y_pixels_per_meter: 0,
                colors_used: 0,
                colors_important: 0,
            },
            colors: [RgbQuad {
                blue: 0,
                green: 0,
                red: 0,
                reserved: 0,
            }],
        };
        let mut pixels = std::ptr::null_mut();
        let bitmap = unsafe { CreateDIBSection(0, &info, DIB_RGB_COLORS, &mut pixels, 0, 0) };
        if bitmap == 0 || pixels.is_null() {
            return 0;
        }
        let red = color & 0xFF;
        let green = (color >> 8) & 0xFF;
        let blue = (color >> 16) & 0xFF;
        let argb = 0xFF00_0000 | (red << 16) | (green << 8) | blue;
        let pixels = unsafe { std::slice::from_raw_parts_mut(pixels.cast::<u32>(), 16 * 16) };
        pixels.fill(0);
        for (y, row) in mask.into_iter().enumerate() {
            for x in 0..16 {
                if row & (1 << (15 - x)) != 0 {
                    pixels[y * 16 + x] = argb;
                }
            }
        }
        bitmap
    }

    let menu_text = unsafe { GetSysColor(7) }; // COLOR_MENUTEXT
    [
        (TRAY_CMD_SHOW, WINDOW),
        (TRAY_CMD_PLAYPAUSE, PLAY),
        (TRAY_CMD_MUTE, SPEAKER),
        (TRAY_CMD_OPEN, FOLDER),
        (TRAY_CMD_EXIT, CLOSE),
    ]
    .into_iter()
    .filter_map(|(command, bits)| {
        let bitmap = create_argb_bitmap(bits, menu_text);
        if bitmap == 0 {
            return None;
        }
        unsafe {
            let _ = SetMenuItemBitmaps(menu, command, 0, bitmap, bitmap);
        }
        Some(bitmap)
    })
    .collect()
}

#[cfg(target_os = "windows")]
fn delete_tray_menu_icons(icons: &[isize]) {
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn DeleteObject(object: isize) -> i32;
    }
    for icon in icons {
        unsafe {
            let _ = DeleteObject(*icon);
        }
    }
}

#[cfg(target_os = "windows")]
fn set_menu_default_item(menu: isize, command: u32) {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetMenuDefaultItem(menu: isize, item: u32, by_position: i32) -> i32;
    }
    unsafe {
        let _ = SetMenuDefaultItem(menu, command, 0);
    }
}

#[cfg(target_os = "windows")]
pub fn show_tray_popup_menu(
    hwnd_raw: isize,
    is_paused: bool,
    is_muted: bool,
    has_media: bool,
) -> Option<u32> {
    use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, MF_DISABLED, MF_GRAYED,
        MF_SEPARATOR, MF_STRING, PostMessageW, SetForegroundWindow, TPM_NONOTIFY, TPM_RETURNCMD,
        TPM_RIGHTBUTTON, TrackPopupMenu, WM_NULL,
    };
    use windows::core::PCWSTR;

    if hwnd_raw == 0 {
        return None;
    }
    let hwnd = HWND(hwnd_raw as *mut _);

    unsafe {
        let mut cursor = POINT { x: 0, y: 0 };
        let _ = GetCursorPos(&mut cursor);

        let hmenu = CreatePopupMenu().ok()?;

        let show_text: Vec<u16> = format!("{}\tWin+Shift+P", tray_menu_label(TRAY_CMD_SHOW, false))
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let playpause_text: Vec<u16> =
            format!("{}\tSpace", tray_menu_label(TRAY_CMD_PLAYPAUSE, is_paused))
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
        let mute_text: Vec<u16> = format!("{}\tM", tray_menu_label(TRAY_CMD_MUTE, is_muted))
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let open_text: Vec<u16> = format!("{}\tCtrl+O", tray_menu_label(TRAY_CMD_OPEN, false))
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let exit_text: Vec<u16> = format!("{}\tAlt+F4", tray_menu_label(TRAY_CMD_EXIT, false))
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            TRAY_CMD_SHOW as usize,
            PCWSTR(show_text.as_ptr()),
        );
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        let media_flags = if has_media {
            MF_STRING
        } else {
            MF_STRING | MF_DISABLED | MF_GRAYED
        };
        let _ = AppendMenuW(
            hmenu,
            media_flags,
            TRAY_CMD_PLAYPAUSE as usize,
            PCWSTR(playpause_text.as_ptr()),
        );
        let _ = AppendMenuW(
            hmenu,
            media_flags,
            TRAY_CMD_MUTE as usize,
            PCWSTR(mute_text.as_ptr()),
        );
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            TRAY_CMD_OPEN as usize,
            PCWSTR(open_text.as_ptr()),
        );
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            TRAY_CMD_EXIT as usize,
            PCWSTR(exit_text.as_ptr()),
        );

        let icons = install_tray_menu_icons(hmenu.0 as isize);
        set_menu_default_item(hmenu.0 as isize, TRAY_CMD_SHOW);

        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            hmenu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY,
            cursor.x,
            cursor.y,
            0,
            hwnd,
            None,
        );
        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(hmenu);
        delete_tray_menu_icons(&icons);

        if cmd.0 != 0 { Some(cmd.0 as u32) } else { None }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn register_system_tray_icon(_hwnd_raw: isize, _tip: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn update_system_tray_icon(_hwnd_raw: isize, _tip: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn remove_system_tray_icon(_hwnd_raw: isize) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn show_tray_popup_menu(
    _hwnd_raw: isize,
    _is_paused: bool,
    _is_muted: bool,
    _has_media: bool,
) -> Option<u32> {
    None
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
    fn test_compute_taskbar_state_with_error() {
        let err_state = compute_taskbar_state_with_error(10.0, 60.0, false, true);
        assert!(err_state.is_active);
        assert_eq!(err_state.to_progress_flag(), TaskbarProgressFlag::Error);

        let normal_state = compute_taskbar_state_with_error(10.0, 60.0, false, false);
        assert_eq!(normal_state.to_progress_flag(), TaskbarProgressFlag::Normal);

        let paused_state = compute_taskbar_state_with_error(10.0, 60.0, true, false);
        assert_eq!(paused_state.to_progress_flag(), TaskbarProgressFlag::Paused);
    }

    #[test]
    fn test_decoration_colorref_conversion() {
        // RGB(33, 33, 33) => COLORREF 0x00212121
        let r: u32 = 33;
        let g: u32 = 33;
        let b: u32 = 33;
        let colorref = r | (g << 8) | (b << 16);
        assert_eq!(colorref, 0x00212121);
        assert_eq!(decoration_colors(true), (0x00212121, 0x00FFFFFF));
        assert_eq!(decoration_colors(false), (0x00F4F4F4, 0x00111111));
    }

    #[test]
    fn gui_mutex_namespace_is_stable_and_brand_specific() {
        assert_eq!(gui_mutex_name(" Pealayer "), gui_mutex_name("pealayer"));
        assert_ne!(
            gui_mutex_name("Pealayer"),
            gui_mutex_name("Workshop Player")
        );
        assert!(gui_mutex_name("Pealayer").starts_with("Local\\Pealayer.GuiOwner."));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn gui_mutex_has_one_owner_and_recovers_after_drop() {
        let identity = format!("Pealayer ownership test {}", std::process::id());
        let primary = match acquire_gui_ownership(&identity).expect("first mutex acquisition") {
            GuiOwnership::Primary(owner) => owner,
            GuiOwnership::Existing => panic!("unique test identity unexpectedly had an owner"),
        };
        assert!(matches!(
            acquire_gui_ownership(&identity).expect("second mutex acquisition"),
            GuiOwnership::Existing
        ));
        drop(primary);
        assert!(matches!(
            acquire_gui_ownership(&identity).expect("post-drop mutex acquisition"),
            GuiOwnership::Primary(_)
        ));
    }

    #[test]
    fn test_thumbnail_button_tooltips() {
        assert_eq!(
            thumbnail_button_tooltip(THUMB_BUTTON_PREV, false),
            "Previous"
        );
        assert_eq!(
            thumbnail_button_tooltip(THUMB_BUTTON_PLAYPAUSE, true),
            "Play"
        );
        assert_eq!(
            thumbnail_button_tooltip(THUMB_BUTTON_PLAYPAUSE, false),
            "Pause"
        );
        assert_eq!(thumbnail_button_tooltip(THUMB_BUTTON_NEXT, false), "Next");
    }

    #[test]
    fn test_tray_command_ids_and_menu_labels() {
        assert_eq!(tray_menu_label(TRAY_CMD_PLAYPAUSE, true), "Play");
        assert_eq!(tray_menu_label(TRAY_CMD_PLAYPAUSE, false), "Pause");
        assert_eq!(tray_menu_label(TRAY_CMD_MUTE, true), "Unmute");
        assert_eq!(tray_menu_label(TRAY_CMD_MUTE, false), "Mute");
        assert_eq!(tray_menu_label(TRAY_CMD_OPEN, false), "Open Media...");
        assert_eq!(tray_menu_label(TRAY_CMD_EXIT, false), "Exit");
    }

    #[test]
    fn test_tray_stubs_or_validation() {
        #[cfg(target_os = "windows")]
        {
            assert!(register_system_tray_icon(0, "test").is_err());
            assert!(update_system_tray_icon(0, "test").is_err());
            assert!(remove_system_tray_icon(0).is_err());
            assert_eq!(show_tray_popup_menu(0, false, false, false), None);
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert!(register_system_tray_icon(0, "test").is_ok());
            assert!(update_system_tray_icon(0, "test").is_ok());
            assert!(remove_system_tray_icon(0).is_ok());
            assert_eq!(show_tray_popup_menu(0, false, false, false), None);
        }
    }

    #[test]
    fn test_compute_thumbnail_clip_ratio() {
        let win_w = 1920.0;
        let win_h = 1080.0;
        let video_rect = [0.0, 100.0, 1920.0, 900.0]; // letterboxed: [min_x, min_y, max_x, max_y]
        let ratio = compute_thumbnail_clip_ratio(win_w, win_h, video_rect);
        assert_eq!(ratio[0], 0.0);
        assert!((ratio[1] - (100.0 / 1080.0)).abs() < 1e-4);
        assert_eq!(ratio[2], 1.0);

        // Non-positive dimensions return default full frame
        assert_eq!(
            compute_thumbnail_clip_ratio(0.0, 1080.0, video_rect),
            [0.0, 0.0, 1.0, 1.0]
        );
        assert_eq!(
            compute_thumbnail_clip_ratio(1920.0, -10.0, video_rect),
            [0.0, 0.0, 1.0, 1.0]
        );

        // Clamping bounds
        let out_of_bounds = [-100.0, -50.0, 2500.0, 2000.0];
        assert_eq!(
            compute_thumbnail_clip_ratio(win_w, win_h, out_of_bounds),
            [0.0, 0.0, 1.0, 1.0]
        );
    }

    #[test]
    fn test_configure_video_taskbar_thumbnail_stubs() {
        #[cfg(target_os = "windows")]
        {
            assert!(configure_video_taskbar_thumbnail(0).is_err());
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert!(configure_video_taskbar_thumbnail(0).is_ok());
            assert!(configure_video_taskbar_thumbnail(12345).is_ok());
        }
    }
}
