use std::collections::VecDeque;
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicU32, AtomicU64, Ordering},
};

#[cfg(target_os = "windows")]
fn wide_null_path(path: &std::path::Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Register a bundled font for the lifetime of this process.
///
/// mpv's regular subtitle renderer can load faces from `sub-fonts-dir`, but
/// custom `osd-overlay` ASS events use the OSD renderer's font provider. On
/// Windows that provider only saw installed/process fonts and silently fell
/// back to Arial even though the bundled Vazirmatn file existed. `FR_PRIVATE`
/// exposes the face to this process without installing it for the user or
/// leaking it to other applications. Windows removes the registration when
/// the process exits.
#[cfg(target_os = "windows")]
pub fn register_private_font(path: &std::path::Path) -> Result<u32, String> {
    use windows::Win32::Graphics::Gdi::{AddFontResourceExW, FR_PRIVATE};
    use windows::core::PCWSTR;

    if !path.is_file() {
        return Err(format!("font file does not exist: {}", path.display()));
    }
    let wide_path = wide_null_path(path);
    let faces = unsafe {
        AddFontResourceExW(
            PCWSTR(wide_path.as_ptr()),
            FR_PRIVATE,
            Some(std::ptr::null()),
        )
    };
    if faces == 0 {
        Err(format!(
            "Windows did not register the private font {}",
            path.display()
        ))
    } else {
        Ok(faces as u32)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn register_private_font(_path: &std::path::Path) -> Result<u32, String> {
    Ok(0)
}

/// Give the roaming configuration folder a recognizable native Explorer
/// identity. The JSON remains the authoritative cross-platform store; this is
/// presentation metadata only and is deliberately absent in portable mode.
#[cfg(target_os = "windows")]
pub fn configure_config_directory(
    config_path: &std::path::Path,
    config: &crate::config::AppConfig,
) -> Result<(), String> {
    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_SYSTEM, SetFileAttributesW,
    };
    use windows::core::PCWSTR;

    let directory = config_path.parent().ok_or_else(|| {
        format!(
            "configuration path has no parent: {}",
            config_path.display()
        )
    })?;
    std::fs::create_dir_all(directory).map_err(|error| {
        format!(
            "create configuration directory {}: {error}",
            directory.display()
        )
    })?;
    // A single native ICO derivative, written only when branding actually
    // changes. Do not rewrite/sign the running executable or cache one copy
    // per playback state. Explorer needs a persistent icon location.
    let icon_path = directory.join("application.ico");
    let icon_bytes = crate::branding::native_icon_bytes(config, crate::branding::PlaybackIconState::Stopped)?;
    let icon_changed = write_if_changed(&icon_path, &icon_bytes)?;
    let app_name = crate::config::resolved_app_name(config);
    let safe_name = app_name.trim().replace(['\r', '\n'], " ");
    let desktop_ini = directory.join("desktop.ini");
    let contents = format!(
        "[.ShellClassInfo]\r\nIconResource=\"{}\",0\r\nInfoTip={} configuration and workspace settings\r\nConfirmFileOp=0\r\n",
        icon_path.display(),
        if safe_name.is_empty() {
            "Pealayer"
        } else {
            &safe_name
        },
    );
    let mut encoded = vec![0xff, 0xfe];
    encoded.extend(contents.encode_utf16().flat_map(u16::to_le_bytes));
    let metadata_changed = write_if_changed(&desktop_ini, &encoded)?;

    let desktop_ini_wide = wide_null_path(&desktop_ini);
    unsafe {
        SetFileAttributesW(
            PCWSTR(desktop_ini_wide.as_ptr()),
            FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM,
        )
    }
    .map_err(|error| {
        format!(
            "mark {} as native folder metadata: {error}",
            desktop_ini.display()
        )
    })?;
    let directory_wide = wide_null_path(directory);
    unsafe {
        SetFileAttributesW(
            PCWSTR(directory_wide.as_ptr()),
            FILE_ATTRIBUTE_READONLY | FILE_ATTRIBUTE_SYSTEM,
        )
    }
    .map_err(|error| {
        format!(
            "apply native folder identity to {}: {error}",
            directory.display()
        )
    })?;
    if icon_changed || metadata_changed {
        use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_UPDATEITEM, SHCNF_PATHW};
        unsafe { SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATHW, Some(directory_wide.as_ptr().cast()), None); }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn write_if_changed(path: &std::path::Path, bytes: &[u8]) -> Result<bool, String> {
    use std::io::Write;
    if std::fs::read(path).ok().as_deref() == Some(bytes) { return Ok(false); }
    // OPEN_EXISTING, not CREATE_ALWAYS, preserves hidden/system file attributes.
    let result = if path.exists() {
        std::fs::OpenOptions::new().write(true).truncate(true).open(path)
            .and_then(|mut file| file.write_all(bytes))
    } else { std::fs::write(path, bytes) };
    result.map_err(|error| format!("write {}: {error}", path.display()))?;
    Ok(true)
}

#[cfg(not(target_os = "windows"))]
pub fn configure_config_directory(
    _config_path: &std::path::Path,
    _config: &crate::config::AppConfig,
) -> Result<(), String> {
    Ok(())
}

static WINDOW_HWND: AtomicIsize = AtomicIsize::new(0);
static WINDOW_DARK_THEME: AtomicBool = AtomicBool::new(true);
static WINDOW_STUDIO_PALETTE: AtomicBool = AtomicBool::new(false);
static WINDOW_DWM_THEMING: AtomicBool = AtomicBool::new(true);
static WINDOW_MICA_BACKDROP: AtomicBool = AtomicBool::new(false);
static WINDOW_MOVE_RESIZE_ACTIVE: AtomicBool = AtomicBool::new(false);
static WINDOW_MOVE_RESIZE_ENDED: AtomicBool = AtomicBool::new(false);
static WINDOW_LIVE_VIDEO_DURING_MOVE: AtomicBool = AtomicBool::new(true);
static WINDOW_COMPOSITOR_PACED_MOVE: AtomicBool = AtomicBool::new(true);
static WINDOW_MOVE_FRAME_PUMP_STARTED: AtomicBool = AtomicBool::new(false);
static WINDOW_MOVE_FRAME_PUMP_THREAD: OnceLock<std::thread::Thread> = OnceLock::new();
static SIMPLE_VIDEO_ASPECT_ENABLED: AtomicBool = AtomicBool::new(false);
static SIMPLE_VIDEO_ASPECT_BITS: AtomicU64 = AtomicU64::new(0);
static SIMPLE_VIDEO_CHROME_WIDTH: AtomicI32 = AtomicI32::new(0);
static SIMPLE_VIDEO_CHROME_HEIGHT: AtomicI32 = AtomicI32::new(0);
static WINDOW_MAGNETIC_SNAP_ENABLED: AtomicBool = AtomicBool::new(false);
static WINDOW_MAGNETIC_SNAP_DISTANCE: AtomicI32 = AtomicI32::new(16);
static WINDOW_MAGNETIC_DRAG: Mutex<MagneticDragSession> = Mutex::new(MagneticDragSession::new());
static SHELL_SUBCLASS_HWND: AtomicIsize = AtomicIsize::new(0);
static SHELL_COMMAND_MESSAGES: AtomicU64 = AtomicU64::new(0);
static SHELL_COMMANDS_QUEUED: AtomicU64 = AtomicU64::new(0);
static SHELL_COMMANDS: Mutex<VecDeque<u32>> = Mutex::new(VecDeque::new());
static SHELL_PAUSED: AtomicBool = AtomicBool::new(true);
static SHELL_MUTED: AtomicBool = AtomicBool::new(false);
static SHELL_FULLSCREEN: AtomicBool = AtomicBool::new(false);
static SHELL_HAS_MEDIA: AtomicBool = AtomicBool::new(false);
static SHELL_MEDIA_KEYS_ENABLED: AtomicBool = AtomicBool::new(true);
static SHELL_REINITIALIZE: AtomicBool = AtomicBool::new(false);
static TASKBAR_BUTTON_CREATED_MESSAGE: AtomicU32 = AtomicU32::new(0);
static TASKBAR_THUMBNAIL_CLIP: Mutex<Option<(isize, Option<[i32; 4]>)>> = Mutex::new(None);
static THUMBNAIL_METRICS_DIRTY: AtomicBool = AtomicBool::new(true);
static THUMBNAIL_TOOLBAR_ADDED_HWND: AtomicIsize = AtomicIsize::new(0);
static THUMBNAIL_TOOLBAR_ADD_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static THUMBNAIL_TOOLBAR_ADD_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static THUMBNAIL_TOOLBAR_UPDATE_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static THUMBNAIL_TOOLBAR_UPDATE_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static TASKBAR_BUTTON_CREATED_EVENTS: AtomicU64 = AtomicU64::new(0);
static THUMBNAIL_TOOLBAR_ENABLED: AtomicBool = AtomicBool::new(false);
static THUMBNAIL_TOOLBAR_HAS_MEDIA: AtomicBool = AtomicBool::new(false);
static THUMBNAIL_TOOLBAR_LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);
#[cfg(target_os = "windows")]
static THUMBNAIL_TOOLBAR_ICONS: Mutex<Vec<isize>> = Mutex::new(Vec::new());

fn queue_shell_command(command: u32) {
    if let Ok(mut commands) = SHELL_COMMANDS.lock() {
        commands.push_back(command);
        SHELL_COMMANDS_QUEUED.fetch_add(1, Ordering::Relaxed);
    }
    crate::platform::taskbar_preview::request_repaint();
}

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

// WM_ENTERSIZEMOVE and WM_EXITSIZEMOVE are emitted by Windows around the
// modal loop used for a native title-bar drag or border resize. Egui does not
// receive a pointer-down event for the non-client title bar, so its drag state
// alone cannot distinguish this operation from ordinary playback.
const WM_ENTERSIZEMOVE_VALUE: u32 = 0x0231;
const WM_EXITSIZEMOVE_VALUE: u32 = 0x0232;
const WM_SIZING_VALUE: u32 = 0x0214;
const WM_MOVING_VALUE: u32 = 0x0216;

const WMSZ_LEFT_VALUE: usize = 1;
const WMSZ_RIGHT_VALUE: usize = 2;
const WMSZ_TOP_VALUE: usize = 3;
const WMSZ_TOPLEFT_VALUE: usize = 4;
const WMSZ_TOPRIGHT_VALUE: usize = 5;
const WMSZ_BOTTOM_VALUE: usize = 6;
const WMSZ_BOTTOMLEFT_VALUE: usize = 7;
const WMSZ_BOTTOMRIGHT_VALUE: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
struct SimpleVideoAspectConstraint {
    aspect_ratio: f64,
    chrome_width: i32,
    chrome_height: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SizingRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SnapAxisAnchor {
    active: bool,
    bypass: bool,
    cursor: i32,
    raw_leading: i32,
    raw_trailing: i32,
    snapped_leading: i32,
    snapped_trailing: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct MagneticDragSession {
    x: SnapAxisAnchor,
    y: SnapAxisAnchor,
    control_bypass: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SnapResult {
    snapped_x: bool,
    snapped_y: bool,
}

impl MagneticDragSession {
    const fn new() -> Self {
        Self {
            x: SnapAxisAnchor {
                active: false,
                bypass: false,
                cursor: 0,
                raw_leading: 0,
                raw_trailing: 0,
                snapped_leading: 0,
                snapped_trailing: 0,
            },
            y: SnapAxisAnchor {
                active: false,
                bypass: false,
                cursor: 0,
                raw_leading: 0,
                raw_trailing: 0,
                snapped_leading: 0,
                snapped_trailing: 0,
            },
            control_bypass: false,
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn apply(
        &mut self,
        target: SizingRect,
        cursor: (i32, i32),
        work_area: SizingRect,
        dpi: u32,
        threshold_dip: i32,
        control_down: bool,
    ) -> (SizingRect, SnapResult, bool) {
        if target.width() <= 0 || target.height() <= 0 || threshold_dip <= 0 {
            self.reset();
            return (target, SnapResult::default(), false);
        }
        if control_down {
            return self.detach_for_control(target, cursor);
        }
        if self.control_bypass {
            // Ctrl is a live override. Releasing it during the same native
            // move loop re-arms both axes immediately at the current position.
            self.control_bypass = false;
            self.x = SnapAxisAnchor::default();
            self.y = SnapAxisAnchor::default();
        }

        let release_distance = i64::from(scaled_snap_threshold(threshold_dip, dpi) + 4).max(8);
        let mut raw = target;
        let mut handled = false;
        if self.x.active {
            let delta = i64::from(cursor.0) - i64::from(self.x.cursor);
            (raw.left, raw.right) =
                translate_snap_axis(self.x.raw_leading, self.x.raw_trailing, delta);
            if delta.abs() >= release_distance {
                self.x = SnapAxisAnchor {
                    bypass: true,
                    ..SnapAxisAnchor::default()
                };
                handled = true;
            }
        }
        if self.y.active {
            let delta = i64::from(cursor.1) - i64::from(self.y.cursor);
            (raw.top, raw.bottom) =
                translate_snap_axis(self.y.raw_leading, self.y.raw_trailing, delta);
            if delta.abs() >= release_distance {
                self.y = SnapAxisAnchor {
                    bypass: true,
                    ..SnapAxisAnchor::default()
                };
                handled = true;
            }
        }

        let (candidate, candidate_result) =
            snap_to_work_area(raw, work_area, scaled_snap_threshold(threshold_dip, dpi));
        let mut output = raw;
        let mut result = SnapResult::default();
        let x_can_acquire = !self.x.bypass;
        let y_can_acquire = !self.y.bypass;
        if self.x.bypass && !candidate_result.snapped_x {
            self.x.bypass = false;
        }
        if self.y.bypass && !candidate_result.snapped_y {
            self.y.bypass = false;
        }

        if self.x.active {
            output.left = self.x.snapped_leading;
            output.right = self.x.snapped_trailing;
            result.snapped_x = true;
            handled = true;
        } else if x_can_acquire && candidate_result.snapped_x {
            self.x.acquire(
                cursor.0,
                raw.left,
                raw.right,
                candidate.left,
                candidate.right,
            );
            output.left = candidate.left;
            output.right = candidate.right;
            result.snapped_x = true;
            handled = true;
        }
        if self.y.active {
            output.top = self.y.snapped_leading;
            output.bottom = self.y.snapped_trailing;
            result.snapped_y = true;
            handled = true;
        } else if y_can_acquire && candidate_result.snapped_y {
            self.y.acquire(
                cursor.1,
                raw.top,
                raw.bottom,
                candidate.top,
                candidate.bottom,
            );
            output.top = candidate.top;
            output.bottom = candidate.bottom;
            result.snapped_y = true;
            handled = true;
        }
        (output, result, handled)
    }

    fn detach_for_control(
        &mut self,
        mut target: SizingRect,
        cursor: (i32, i32),
    ) -> (SizingRect, SnapResult, bool) {
        let mut handled = false;
        if self.x.active {
            (target.left, target.right) = translate_snap_axis(
                self.x.raw_leading,
                self.x.raw_trailing,
                i64::from(cursor.0) - i64::from(self.x.cursor),
            );
            handled = true;
        }
        if self.y.active {
            (target.top, target.bottom) = translate_snap_axis(
                self.y.raw_leading,
                self.y.raw_trailing,
                i64::from(cursor.1) - i64::from(self.y.cursor),
            );
            handled = true;
        }
        self.x = SnapAxisAnchor::default();
        self.y = SnapAxisAnchor::default();
        self.control_bypass = true;
        (target, SnapResult::default(), handled)
    }
}

impl SnapAxisAnchor {
    fn acquire(
        &mut self,
        cursor: i32,
        raw_leading: i32,
        raw_trailing: i32,
        snapped_leading: i32,
        snapped_trailing: i32,
    ) {
        *self = Self {
            active: true,
            bypass: false,
            cursor,
            raw_leading,
            raw_trailing,
            snapped_leading,
            snapped_trailing,
        };
    }
}

fn scaled_snap_threshold(threshold_dip: i32, dpi: u32) -> i32 {
    if threshold_dip <= 0 {
        return 0;
    }
    let dpi = i64::from(if dpi == 0 { 96 } else { dpi });
    ((i64::from(threshold_dip) * dpi + 48) / 96).clamp(1, i64::from(i32::MAX)) as i32
}

fn nearest_snap_delta(first: i64, second: i64, threshold: i64) -> Option<i32> {
    [first, second]
        .into_iter()
        .filter(|delta| delta.abs() <= threshold)
        .min_by_key(|delta| delta.abs())
        .map(|delta| delta.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32)
}

fn snap_to_work_area(
    moving: SizingRect,
    work_area: SizingRect,
    threshold: i32,
) -> (SizingRect, SnapResult) {
    if moving.width() <= 0
        || moving.height() <= 0
        || work_area.width() <= 0
        || work_area.height() <= 0
        || threshold <= 0
    {
        return (moving, SnapResult::default());
    }
    let x_delta = nearest_snap_delta(
        i64::from(work_area.left) - i64::from(moving.left),
        i64::from(work_area.right) - i64::from(moving.right),
        i64::from(threshold),
    );
    let y_delta = nearest_snap_delta(
        i64::from(work_area.top) - i64::from(moving.top),
        i64::from(work_area.bottom) - i64::from(moving.bottom),
        i64::from(threshold),
    );
    let mut snapped = moving;
    if let Some(delta) = x_delta {
        (snapped.left, snapped.right) =
            translate_snap_axis(moving.left, moving.right, i64::from(delta));
    }
    if let Some(delta) = y_delta {
        (snapped.top, snapped.bottom) =
            translate_snap_axis(moving.top, moving.bottom, i64::from(delta));
    }
    (
        snapped,
        SnapResult {
            snapped_x: x_delta.is_some(),
            snapped_y: y_delta.is_some(),
        },
    )
}

fn translate_snap_axis(leading: i32, trailing: i32, delta: i64) -> (i32, i32) {
    let width = i64::from(trailing) - i64::from(leading);
    if width <= 0 {
        return (leading, trailing);
    }
    let delta = delta.clamp(
        i64::from(i32::MIN) - i64::from(leading),
        i64::from(i32::MAX) - i64::from(trailing),
    );
    let translated = i64::from(leading) + delta;
    (translated as i32, (translated + width) as i32)
}

pub fn configure_window_magnetic_snap(enabled: bool, distance_dip: i32) {
    WINDOW_MAGNETIC_SNAP_DISTANCE.store(distance_dip.clamp(1, 128), Ordering::Release);
    WINDOW_MAGNETIC_SNAP_ENABLED.store(enabled, Ordering::Release);
    if !enabled && let Ok(mut drag) = WINDOW_MAGNETIC_DRAG.lock() {
        drag.reset();
    }
}

fn reset_window_magnetic_drag() {
    if let Ok(mut drag) = WINDOW_MAGNETIC_DRAG.lock() {
        drag.reset();
    }
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct NativePoint {
    x: i32,
    y: i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct NativeMonitorInfo {
    size: u32,
    monitor: SizingRect,
    work: SizingRect,
    flags: u32,
}

#[cfg(target_os = "windows")]
unsafe fn constrain_native_moving_rect(hwnd: isize, lparam: isize) -> bool {
    const MONITOR_DEFAULTTONEAREST: u32 = 2;
    const VK_CONTROL: i32 = 0x11;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetAsyncKeyState(key: i32) -> i16;
        fn GetCursorPos(point: *mut NativePoint) -> i32;
        fn GetDpiForWindow(hwnd: isize) -> u32;
        fn GetMonitorInfoW(monitor: isize, info: *mut NativeMonitorInfo) -> i32;
        fn MonitorFromRect(rect: *const SizingRect, flags: u32) -> isize;
    }
    #[link(name = "shcore")]
    unsafe extern "system" {
        fn GetDpiForMonitor(monitor: isize, dpi_type: i32, dpi_x: *mut u32, dpi_y: *mut u32)
        -> i32;
    }

    if !WINDOW_MAGNETIC_SNAP_ENABLED.load(Ordering::Acquire) || lparam == 0 {
        reset_window_magnetic_drag();
        return false;
    }
    // SAFETY: WM_MOVING supplies a writable RECT for this window-procedure
    // call. The null case is rejected above.
    let target = unsafe { &mut *(lparam as *mut SizingRect) };
    let mut cursor = NativePoint { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut cursor) } == 0 {
        reset_window_magnetic_drag();
        return false;
    }
    let control_down = (unsafe { GetAsyncKeyState(VK_CONTROL) } as u16 & 0x8000) != 0;
    let monitor = unsafe { MonitorFromRect(target, MONITOR_DEFAULTTONEAREST) };
    if monitor == 0 {
        reset_window_magnetic_drag();
        return false;
    }
    let mut dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 {
        dpi = 96;
    }
    // Snapping is evaluated against the destination monitor selected from
    // the proposed WM_MOVING rectangle, so the threshold must use that
    // monitor's effective DPI rather than the DPI of the monitor being left.
    let mut monitor_dpi_x = 0;
    let mut monitor_dpi_y = 0;
    if unsafe { GetDpiForMonitor(monitor, 0, &mut monitor_dpi_x, &mut monitor_dpi_y) } >= 0
        && monitor_dpi_x > 0
    {
        dpi = monitor_dpi_x;
    }
    let work_area = if control_down {
        SizingRect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }
    } else {
        let mut info = NativeMonitorInfo {
            size: std::mem::size_of::<NativeMonitorInfo>() as u32,
            monitor: *target,
            work: *target,
            flags: 0,
        };
        if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
            reset_window_magnetic_drag();
            return false;
        }
        info.work
    };
    let distance = WINDOW_MAGNETIC_SNAP_DISTANCE.load(Ordering::Acquire);
    let Ok(mut drag) = WINDOW_MAGNETIC_DRAG.lock() else {
        return false;
    };
    let (snapped, _, handled) = drag.apply(
        *target,
        (cursor.x, cursor.y),
        work_area,
        dpi,
        distance,
        control_down,
    );
    if handled {
        *target = snapped;
    }
    handled
}

impl SizingRect {
    fn width(self) -> i32 {
        self.right.saturating_sub(self.left)
    }

    fn height(self) -> i32 {
        self.bottom.saturating_sub(self.top)
    }

    fn set_width(&mut self, edge: usize, width: i32) {
        match edge {
            WMSZ_LEFT_VALUE | WMSZ_TOPLEFT_VALUE | WMSZ_BOTTOMLEFT_VALUE => {
                self.left = self.right.saturating_sub(width);
            }
            WMSZ_RIGHT_VALUE | WMSZ_TOPRIGHT_VALUE | WMSZ_BOTTOMRIGHT_VALUE => {
                self.right = self.left.saturating_add(width);
            }
            _ => {
                let center = i64::from(self.left) + i64::from(self.width()) / 2;
                self.left = (center - i64::from(width) / 2) as i32;
                self.right = self.left.saturating_add(width);
            }
        }
    }

    fn set_height(&mut self, edge: usize, height: i32) {
        match edge {
            WMSZ_TOP_VALUE | WMSZ_TOPLEFT_VALUE | WMSZ_TOPRIGHT_VALUE => {
                self.top = self.bottom.saturating_sub(height);
            }
            WMSZ_BOTTOM_VALUE | WMSZ_BOTTOMLEFT_VALUE | WMSZ_BOTTOMRIGHT_VALUE => {
                self.bottom = self.top.saturating_add(height);
            }
            _ => {
                let center = i64::from(self.top) + i64::from(self.height()) / 2;
                self.top = (center - i64::from(height) / 2) as i32;
                self.bottom = self.top.saturating_add(height);
            }
        }
    }
}

/// Publish the most recently painted Simple-workspace video geometry to the
/// native HWND hook. `WM_SIZING` runs inside Windows' modal resize loop, where
/// waiting for another egui frame would visibly lag behind the pointer.
pub fn set_simple_video_aspect_constraint(
    enabled: bool,
    aspect_ratio: f64,
    chrome_width: i32,
    chrome_height: i32,
) {
    if !enabled || !aspect_ratio.is_finite() || !(0.05..=20.0).contains(&aspect_ratio) {
        SIMPLE_VIDEO_ASPECT_ENABLED.store(false, Ordering::Release);
        return;
    }

    // Publish the tuple as one logical update. The HWND and egui callbacks run
    // on the UI thread today, but this also prevents a future cross-thread
    // reader from combining a new aspect with an old chrome measurement.
    SIMPLE_VIDEO_ASPECT_ENABLED.store(false, Ordering::Release);
    SIMPLE_VIDEO_ASPECT_BITS.store(aspect_ratio.to_bits(), Ordering::Relaxed);
    SIMPLE_VIDEO_CHROME_WIDTH.store(chrome_width.max(0), Ordering::Relaxed);
    SIMPLE_VIDEO_CHROME_HEIGHT.store(chrome_height.max(0), Ordering::Relaxed);
    SIMPLE_VIDEO_ASPECT_ENABLED.store(true, Ordering::Release);
}

fn simple_video_aspect_constraint() -> Option<SimpleVideoAspectConstraint> {
    if !SIMPLE_VIDEO_ASPECT_ENABLED.load(Ordering::Acquire) {
        return None;
    }
    let aspect_ratio = f64::from_bits(SIMPLE_VIDEO_ASPECT_BITS.load(Ordering::Relaxed));
    if !aspect_ratio.is_finite() || !(0.05..=20.0).contains(&aspect_ratio) {
        return None;
    }
    Some(SimpleVideoAspectConstraint {
        aspect_ratio,
        chrome_width: SIMPLE_VIDEO_CHROME_WIDTH.load(Ordering::Relaxed).max(0),
        chrome_height: SIMPLE_VIDEO_CHROME_HEIGHT.load(Ordering::Relaxed).max(0),
    })
}

fn width_driven_size(width: i32, constraint: SimpleVideoAspectConstraint) -> (i32, i32) {
    const MIN_VIDEO_WIDTH: i32 = 96;
    let video_width = width
        .saturating_sub(constraint.chrome_width)
        .max(MIN_VIDEO_WIDTH);
    let video_height = (f64::from(video_width) / constraint.aspect_ratio)
        .round()
        .max(1.0) as i32;
    (
        video_width.saturating_add(constraint.chrome_width),
        video_height.saturating_add(constraint.chrome_height),
    )
}

fn height_driven_size(height: i32, constraint: SimpleVideoAspectConstraint) -> (i32, i32) {
    const MIN_VIDEO_HEIGHT: i32 = 54;
    let video_height = height
        .saturating_sub(constraint.chrome_height)
        .max(MIN_VIDEO_HEIGHT);
    let video_width = (f64::from(video_height) * constraint.aspect_ratio)
        .round()
        .max(1.0) as i32;
    (
        video_width.saturating_add(constraint.chrome_width),
        video_height.saturating_add(constraint.chrome_height),
    )
}

fn constrain_sizing_rect(
    rect: SizingRect,
    edge: usize,
    constraint: SimpleVideoAspectConstraint,
) -> Option<SizingRect> {
    if rect.width() <= 0 || rect.height() <= 0 {
        return None;
    }

    let (width, height) = match edge {
        WMSZ_LEFT_VALUE | WMSZ_RIGHT_VALUE => width_driven_size(rect.width(), constraint),
        WMSZ_TOP_VALUE | WMSZ_BOTTOM_VALUE => height_driven_size(rect.height(), constraint),
        WMSZ_TOPLEFT_VALUE
        | WMSZ_TOPRIGHT_VALUE
        | WMSZ_BOTTOMLEFT_VALUE
        | WMSZ_BOTTOMRIGHT_VALUE => {
            let from_width = width_driven_size(rect.width(), constraint);
            let from_height = height_driven_size(rect.height(), constraint);
            let width_correction = (from_width.1 - rect.height()).abs();
            let height_correction = (from_height.0 - rect.width()).abs();
            if width_correction <= height_correction {
                from_width
            } else {
                from_height
            }
        }
        _ => return None,
    };

    let mut constrained = rect;
    constrained.set_width(edge, width);
    constrained.set_height(edge, height);
    Some(constrained)
}

#[cfg(target_os = "windows")]
unsafe fn constrain_native_sizing_rect(edge: usize, lparam: isize) -> bool {
    use windows::Win32::Foundation::RECT;

    let Some(constraint) = simple_video_aspect_constraint() else {
        return false;
    };
    // SAFETY: WM_SIZING guarantees that lParam points to a writable RECT for
    // the duration of the window-procedure call. We reject a null pointer.
    let Some(native_rect) = (unsafe { (lparam as *mut RECT).as_mut() }) else {
        return false;
    };
    let proposed = SizingRect {
        left: native_rect.left,
        top: native_rect.top,
        right: native_rect.right,
        bottom: native_rect.bottom,
    };
    let Some(constrained) = constrain_sizing_rect(proposed, edge, constraint) else {
        return false;
    };
    native_rect.left = constrained.left;
    native_rect.top = constrained.top;
    native_rect.right = constrained.right;
    native_rect.bottom = constrained.bottom;
    true
}

fn native_window_operation_transition(message: u32) -> Option<bool> {
    match message {
        WM_ENTERSIZEMOVE_VALUE => Some(true),
        WM_EXITSIZEMOVE_VALUE => Some(false),
        _ => None,
    }
}

fn observe_native_window_message(message: u32) {
    let Some(active) = native_window_operation_transition(message) else {
        return;
    };
    // A magnetic latch belongs to exactly one native move loop. Resetting at
    // both boundaries also prevents a Preferences/config change from replaying
    // an old snapped coordinate during the next drag.
    reset_window_magnetic_drag();
    WINDOW_MOVE_RESIZE_ACTIVE.store(active, Ordering::Release);
    wake_window_move_frame_pump();
    if !active {
        // Playback callbacks are intentionally suppressed during the modal
        // move loop. Arrange one final paint so a paused frame and any queued
        // libmpv property events become visible immediately after release.
        WINDOW_MOVE_RESIZE_ENDED.store(true, Ordering::Release);
    }
}

pub fn native_window_operation_active() -> bool {
    WINDOW_MOVE_RESIZE_ACTIVE.load(Ordering::Acquire)
}

pub fn configure_live_video_during_window_move(enabled: bool) {
    WINDOW_LIVE_VIDEO_DURING_MOVE.store(enabled, Ordering::Release);
    wake_window_move_frame_pump();
}

pub fn configure_compositor_paced_window_move(enabled: bool) {
    WINDOW_COMPOSITOR_PACED_MOVE.store(enabled, Ordering::Release);
    wake_window_move_frame_pump();
}

fn video_rendering_allowed_during_window_operation(active: bool, enabled: bool) -> bool {
    !active || enabled
}

/// Whether libmpv's decoder-frame callback and GL paint callback should keep
/// presenting while Windows owns the thread in its native move/resize loop.
/// Property wakeups are deliberately still coalesced during that loop: the
/// frame callback already repaints at media cadence and drains queued state,
/// whereas repainting for both streams made the title bar trail the pointer.
pub fn native_window_video_rendering_allowed() -> bool {
    video_rendering_allowed_during_window_operation(
        native_window_operation_active(),
        WINDOW_LIVE_VIDEO_DURING_MOVE.load(Ordering::Acquire),
    )
}

pub fn native_window_compositor_pacing_active() -> bool {
    compositor_move_frame_pump_active(
        WINDOW_MOVE_RESIZE_ACTIVE.load(Ordering::Acquire),
        WINDOW_LIVE_VIDEO_DURING_MOVE.load(Ordering::Acquire),
        WINDOW_COMPOSITOR_PACED_MOVE.load(Ordering::Acquire),
    )
}

fn compositor_move_frame_pump_active(active: bool, live_video: bool, paced: bool) -> bool {
    active && live_video && paced
}

fn wake_window_move_frame_pump() {
    if let Some(thread) = WINDOW_MOVE_FRAME_PUMP_THREAD.get() {
        thread.unpark();
    }
}

/// Request native window redraws at the DWM composition cadence while Windows
/// owns the modal title-bar move/resize loop. Media-frame callbacks are often
/// 24/25/30 Hz and arrive at uneven points relative to a 60+ Hz desktop; using
/// them as the only clock makes the complete window alternately catch and lag
/// behind the pointer. DwmFlush supplies a monitor/compositor-paced wakeup on a
/// dedicated thread, leaving the GUI thread available to process WM_MOVING.
#[cfg(target_os = "windows")]
pub fn start_window_move_frame_pump(egui_ctx: eframe::egui::Context) {
    if WINDOW_MOVE_FRAME_PUMP_STARTED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    let spawn_result = std::thread::Builder::new()
        .name("pealayer-dwm-frame-pump".to_string())
        .spawn(move || {
            let current = std::thread::current();
            let _ = WINDOW_MOVE_FRAME_PUMP_THREAD.set(current.clone());
            loop {
                while compositor_move_frame_pump_active(
                    WINDOW_MOVE_RESIZE_ACTIVE.load(Ordering::Acquire),
                    WINDOW_LIVE_VIDEO_DURING_MOVE.load(Ordering::Acquire),
                    WINDOW_COMPOSITOR_PACED_MOVE.load(Ordering::Acquire),
                ) {
                    let wait_started = std::time::Instant::now();
                    let synchronized = unsafe { windows::Win32::Graphics::Dwm::DwmFlush() }.is_ok();
                    egui_ctx.request_repaint();
                    if !synchronized || wait_started.elapsed() < std::time::Duration::from_millis(2)
                    {
                        // DWM composition is normally mandatory on supported
                        // Windows releases. Preserve a bounded 120 Hz fallback
                        // for remote/compatibility environments, and prevent a
                        // driver that returns immediately from causing a CPU
                        // spin while the user holds the title bar.
                        std::thread::park_timeout(std::time::Duration::from_millis(8));
                    }
                }
                std::thread::park();
            }
        });
    if let Err(error) = spawn_result {
        log::warn!("failed to start DWM window-move frame pump: {error}");
        WINDOW_MOVE_FRAME_PUMP_STARTED.store(false, Ordering::Release);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn start_window_move_frame_pump(_egui_ctx: eframe::egui::Context) {}

pub fn take_native_window_operation_ended() -> bool {
    WINDOW_MOVE_RESIZE_ENDED.swap(false, Ordering::AcqRel)
}

/// Hide the implementation viewport used by Web-only mode without stopping
/// its event loop. The Rust media, hardware, IPC, and HTTP/WebSocket engines
/// remain fully active while Windows has no native Pealayer surface to show.
#[cfg(target_os = "windows")]
pub fn hide_native_window(hwnd_raw: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};
    if hwnd_raw != 0 {
        unsafe {
            let _ = ShowWindow(HWND(hwnd_raw as *mut _), SW_HIDE);
        }
    }
}

/// Hide every top-level window owned by this process. Winit may create or
/// publish its HWND after eframe's first callback, before it is registered in
/// the application state, so Web-only mode also needs this process-scoped
/// fallback.
#[cfg(target_os = "windows")]
pub fn hide_current_process_windows() {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumWindows(
            callback: Option<unsafe extern "system" fn(isize, isize) -> i32>,
            parameter: isize,
        ) -> i32;
        fn GetWindowThreadProcessId(hwnd: isize, process_id: *mut u32) -> u32;
        fn ShowWindow(hwnd: isize, command: i32) -> i32;
    }

    unsafe extern "system" fn hide_owned_window(hwnd: isize, process_id: isize) -> i32 {
        let mut owner = 0_u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut owner);
            if owner == process_id as u32 {
                ShowWindow(hwnd, 0); // SW_HIDE
            }
        }
        1
    }

    unsafe {
        EnumWindows(Some(hide_owned_window), std::process::id() as isize);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn hide_current_process_windows() {}

#[cfg(not(target_os = "windows"))]
pub fn hide_native_window(_hwnd_raw: isize) {}

#[cfg(target_os = "windows")]
pub fn set_window_owner(hwnd_raw: isize, owner_raw: isize) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GWLP_HWNDPARENT, SetWindowLongPtrW};

    if hwnd_raw == 0 || owner_raw == 0 {
        return Ok(());
    }
    unsafe {
        SetWindowLongPtrW(HWND(hwnd_raw as *mut _), GWLP_HWNDPARENT, owner_raw);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn set_window_owner(_hwnd_raw: isize, _owner_raw: isize) -> Result<(), String> {
    Ok(())
}

pub fn set_window_appearance(dark: bool, palette: crate::config::ColorPalette) {
    let theme_changed = WINDOW_DARK_THEME.swap(dark, Ordering::SeqCst) != dark;
    let studio = palette == crate::config::ColorPalette::Studio;
    let palette_changed = WINDOW_STUDIO_PALETTE.swap(studio, Ordering::SeqCst) != studio;
    let hwnd = get_registered_hwnd();
    if hwnd != 0 && (theme_changed || palette_changed) {
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

/// Return the user's current desktop accent color when the platform exposes
/// one. Windows supplies this through DWM; other platforms fall back to their
/// standard native accent in the UI layer.
#[cfg(target_os = "windows")]
pub fn system_accent_color() -> Option<[u8; 3]> {
    use windows::Win32::Graphics::Dwm::DwmGetColorizationColor;
    use windows::core::BOOL;

    let mut argb = 0_u32;
    let mut opaque = BOOL::default();
    unsafe { DwmGetColorizationColor(&mut argb, &mut opaque) }
        .ok()
        .map(|_| {
            [
                ((argb >> 16) & 0xff) as u8,
                ((argb >> 8) & 0xff) as u8,
                (argb & 0xff) as u8,
            ]
        })
}

#[cfg(not(target_os = "windows"))]
pub fn system_accent_color() -> Option<[u8; 3]> {
    None
}

#[cfg(any(target_os = "windows", test))]
fn decoration_colors(dark: bool, palette: crate::config::ColorPalette) -> (u32, u32) {
    let colorref = |role| {
        let color = crate::ui::palette::color(palette, dark, role);
        u32::from(color.r()) | (u32::from(color.g()) << 8) | (u32::from(color.b()) << 16)
    };
    // Match the header's panel surface, including Studio's blue-gray tint.
    (colorref("surface-0"), colorref("text"))
}

#[cfg(target_os = "windows")]
pub fn apply_windows_window_decorations(hwnd_raw: isize) {
    apply_windows_window_decorations_with(
        hwnd_raw,
        WINDOW_DARK_THEME.load(Ordering::SeqCst),
        WINDOW_DWM_THEMING.load(Ordering::SeqCst),
        WINDOW_MICA_BACKDROP.load(Ordering::SeqCst),
        if WINDOW_STUDIO_PALETTE.load(Ordering::SeqCst) {
            crate::config::ColorPalette::Studio
        } else {
            crate::config::ColorPalette::Native
        },
    );
}

#[cfg(target_os = "windows")]
fn apply_windows_window_decorations_with(
    hwnd_raw: isize,
    dark: bool,
    dwm_theming: bool,
    mica_backdrop: bool,
    palette: crate::config::ColorPalette,
) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE;
    use windows::Win32::Graphics::Dwm::{
        DWMSBT_MAINWINDOW, DWMSBT_NONE, DWMWA_CAPTION_COLOR, DWMWA_SYSTEMBACKDROP_TYPE,
        DWMWA_TEXT_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute,
    };
    use windows::core::BOOL;

    if hwnd_raw == 0 {
        return;
    }
    let hwnd = HWND(hwnd_raw as *mut _);
    let (caption_color, text_color) = decoration_colors(dark, palette);

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
        let caption_color = if dwm_theming {
            caption_color
        } else {
            0xFFFF_FFFF
        };
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

pub use super::windows_quick_actions::{WindowsQuickAction, WINDOWS_QUICK_ACTIONS};

#[cfg(target_os = "windows")]
fn build_windows_jump_list(include_quick_actions: bool) -> Result<(), String> {
    use windows::Win32::Foundation::PROPERTYKEY;
    use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::Win32::UI::Shell::Common::{IObjectArray, IObjectCollection};
    use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
    use windows::Win32::UI::Shell::{
        DestinationList, EnumerableObjectCollection, ICustomDestinationList, IShellLinkW,
        KDC_RECENT, ShellLink,
    };
    use windows::core::{GUID, Interface, PCWSTR};

    let executable = std::env::current_exe()
        .map_err(|error| format!("resolve executable for Windows quick actions: {error}"))?;
    let executable_wide = wide_null_path(&executable);
    let working_directory_wide = wide_null_path(executable.parent().ok_or_else(||
        "Windows quick-action executable has no containing directory".to_owned())?);
    const PKEY_TITLE: PROPERTYKEY = PROPERTYKEY {
        fmtid: GUID::from_u128(0xf29f85e0_4ff9_1068_ab91_08002b27b3d9),
        pid: 2,
    };

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let destination_list: ICustomDestinationList =
            CoCreateInstance(&DestinationList, None, CLSCTX_INPROC_SERVER)
                .map_err(|error| format!("create Windows Jump List: {error}"))?;
        let mut minimum_slots = 0;
        let _: IObjectArray = destination_list
            .BeginList(&mut minimum_slots)
            .map_err(|error| format!("begin Windows Jump List: {error}"))?;

        if include_quick_actions {
            let collection: IObjectCollection =
                CoCreateInstance(&EnumerableObjectCollection, None, CLSCTX_INPROC_SERVER)
                    .map_err(|error| format!("create Windows quick-action collection: {error}"))?;

            for action in WINDOWS_QUICK_ACTIONS {
                let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                    .map_err(|error| format!("create Windows quick action: {error}"))?;
                let arguments = action
                    .arguments
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect::<Vec<_>>();
                let description = action
                    .title
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect::<Vec<_>>();
                link.SetPath(PCWSTR(executable_wide.as_ptr()))
                    .map_err(|error| format!("set Windows quick-action executable: {error}"))?;
                link.SetArguments(PCWSTR(arguments.as_ptr()))
                    .map_err(|error| format!("set Windows quick-action arguments: {error}"))?;
                link.SetWorkingDirectory(PCWSTR(working_directory_wide.as_ptr()))
                    .map_err(|error| format!("set Windows quick-action working directory: {error}"))?;
                link.SetDescription(PCWSTR(description.as_ptr()))
                    .map_err(|error| format!("set Windows quick-action description: {error}"))?;
                link.SetIconLocation(PCWSTR(executable_wide.as_ptr()), action.icon_location_index())
                    .map_err(|error| format!("set Windows quick-action icon: {error}"))?;

                let properties: IPropertyStore = link
                    .cast()
                    .map_err(|error| format!("open Windows quick-action properties: {error}"))?;
                let title = PROPVARIANT::from(action.title);
                properties
                    .SetValue(&PKEY_TITLE, &title)
                    .map_err(|error| format!("set Windows quick-action title: {error}"))?;
                properties
                    .Commit()
                    .map_err(|error| format!("commit Windows quick-action title: {error}"))?;
                collection
                    .AddObject(&link)
                    .map_err(|error| format!("append Windows quick action: {error}"))?;
            }
            let tasks: IObjectArray = collection
                .cast()
                .map_err(|error| format!("finalize Windows quick-action collection: {error}"))?;
            destination_list
                .AddUserTasks(&tasks)
                .map_err(|error| format!("publish Windows quick actions: {error}"))?;
        }

        destination_list
            .AppendKnownCategory(KDC_RECENT)
            .map_err(|error| format!("publish Windows recent-media category: {error}"))?;
        destination_list
            .CommitList()
            .map_err(|error| format!("commit Windows Jump List: {error}"))?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn sync_windows_jump_list_with_options(
    recent_media: &[std::path::PathBuf],
    include_quick_actions: bool,
) {
    if crate::peer::active() { return; }
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

    if let Err(error) = build_windows_jump_list(include_quick_actions) {
        log::warn!("Could not synchronize Windows quick actions: {error}");
    }
}

#[cfg(target_os = "windows")]
pub fn sync_windows_jump_list(recent_media: &[std::path::PathBuf]) {
    sync_windows_jump_list_with_options(recent_media, true);
}

pub const THUMB_BUTTON_PREV: u32 = 1001;
pub const THUMB_BUTTON_PLAYPAUSE: u32 = 1002;
pub const THUMB_BUTTON_NEXT: u32 = 1003;
pub const THUMB_BUTTON_MUTE: u32 = 1004;
pub const THUMB_BUTTON_FULLSCREEN: u32 = 1005;

pub fn thumbnail_button_tooltip(
    button_id: u32,
    is_paused: bool,
    is_muted: bool,
    is_fullscreen: bool,
) -> &'static str {
    match button_id {
        THUMB_BUTTON_PREV => "Back 10 seconds",
        THUMB_BUTTON_PLAYPAUSE => {
            if is_paused {
                "Play"
            } else {
                "Pause"
            }
        }
        THUMB_BUTTON_NEXT => "Forward 10 seconds",
        THUMB_BUTTON_MUTE => {
            if is_muted {
                "Unmute"
            } else {
                "Mute"
            }
        }
        THUMB_BUTTON_FULLSCREEN => {
            if is_fullscreen {
                "Exit fullscreen"
            } else {
                "Fullscreen"
            }
        }
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
#[derive(Clone, Copy)]
enum ThumbnailGlyph {
    Back,
    Play,
    Pause,
    Forward,
    Mute,
    Unmute,
    Fullscreen,
    Restore,
}

#[cfg(target_os = "windows")]
fn create_thumbnail_button_icon(
    glyph: ThumbnailGlyph,
    size: u32,
) -> windows::core::Result<windows::Win32::UI::WindowsAndMessaging::HICON> {
    use windows::Win32::{Graphics::Gdi::{CreateBitmap, DeleteObject},
        UI::WindowsAndMessaging::{CreateIconIndirect, ICONINFO}};
    let image = thumbnail_icon_pixels(glyph, size);
    let color = crate::platform::taskbar_preview::bitmap(&image)?;
    let mask_bytes = vec![0u8; ((size as usize + 15) / 16 * 2) * size as usize];
    unsafe {
        let mask = CreateBitmap(size as i32, size as i32, 1, 1, Some(mask_bytes.as_ptr().cast()));
        if mask.0.is_null() { let _ = DeleteObject(color.into()); return Err(windows::core::Error::from_thread()); }
        let result = CreateIconIndirect(&ICONINFO { fIcon: true.into(), hbmColor:color, hbmMask:mask, ..Default::default() });
        let _ = DeleteObject(color.into()); let _ = DeleteObject(mask.into()); result
    }
}

#[cfg(target_os = "windows")]
fn retain_thumbnail_toolbar_icons(
    icons: Vec<windows::Win32::UI::WindowsAndMessaging::HICON>,
) {
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON};

    let replacement = icons
        .into_iter()
        .map(|icon| icon.0 as isize)
        .collect::<Vec<_>>();
    let previous = THUMBNAIL_TOOLBAR_ICONS
        .lock()
        .map(|mut cached| std::mem::replace(&mut *cached, replacement))
        .unwrap_or_default();
    for raw in previous {
        unsafe {
            let _ = DestroyIcon(HICON(raw as *mut _));
        }
    }
}

#[cfg(target_os = "windows")]
fn destroy_thumbnail_toolbar_icons(
    icons: Vec<windows::Win32::UI::WindowsAndMessaging::HICON>,
) {
    use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;
    for icon in icons {
        unsafe {
            let _ = DestroyIcon(icon);
        }
    }
}

#[cfg(target_os = "windows")]
fn thumbnail_icon_pixels(glyph: ThumbnailGlyph, size: u32) -> image::RgbaImage {
    use ab_glyph::{Font, FontRef, PxScale, point};
    use crate::ui::icons;
    let symbol = match glyph {
        ThumbnailGlyph::Back => icons::REWIND, ThumbnailGlyph::Forward => icons::FAST_FORWARD,
        ThumbnailGlyph::Play => icons::PLAY, ThumbnailGlyph::Pause => icons::PAUSE,
        ThumbnailGlyph::Mute => icons::SPEAKER_SLASH, ThumbnailGlyph::Unmute => icons::SPEAKER_HIGH,
        ThumbnailGlyph::Fullscreen => icons::ARROWS_OUT, ThumbnailGlyph::Restore => icons::ARROWS_IN,
    };
    let mut image = image::RgbaImage::new(size,size);
    let font = FontRef::try_from_slice(egui_phosphor::Variant::Regular.font_bytes()).expect("bundled Phosphor font");
    let glyph = font.glyph_id(symbol.chars().next().expect("Phosphor glyph"));
    let scale = PxScale::from(size as f32 * 0.9);
    if let Some(outline) = font.outline_glyph(glyph.with_scale(scale)) {
        let bounds = outline.px_bounds();
        let position = point((size as f32-bounds.width())*0.5-bounds.min.x,
            (size as f32-bounds.height())*0.5-bounds.min.y);
        if let Some(outline) = font.outline_glyph(glyph.with_scale_and_position(scale,position)) {
            let bounds = outline.px_bounds();
            let light = native_taskbar_light_theme();
            let color = if light { [32,32,32] } else { [245,245,245] };
            outline.draw(|x,y,coverage| {
                let x=bounds.min.x.floor() as i32+x as i32;
                let y=bounds.min.y.floor() as i32+y as i32;
                if x>=0 && y>=0 && x<size as i32 && y<size as i32 {
                    image.put_pixel(x as u32,y as u32,image::Rgba([color[0],color[1],color[2],(coverage*255.0).round() as u8]));
                }
            });
        }
    }
    image
}

#[cfg(target_os = "windows")]
fn native_taskbar_light_theme() -> bool {
    winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
        .ok().and_then(|key| key.get_value::<u32,_>("SystemUsesLightTheme").ok()).is_some_and(|v| v!=0)
}

pub fn thumbnail_toolbar_metrics(hwnd: isize) -> (u32,bool) {
    #[cfg(target_os = "windows")]
    {
        static METRICS: Mutex<Option<(isize, std::time::Instant, (u32,bool))>> = Mutex::new(None);
        let mut cached = METRICS.lock().unwrap_or_else(|error| error.into_inner());
        let dirty = THUMBNAIL_METRICS_DIRTY.swap(false, Ordering::AcqRel);
        if !dirty && let Some((owner, at, metrics)) = *cached {
            if owner == hwnd && at.elapsed() < std::time::Duration::from_secs(1) { return metrics; }
        }
        #[link(name = "user32")]
        unsafe extern "system" { fn GetDpiForWindow(hwnd: isize) -> u32; fn GetSystemMetricsForDpi(index:i32,dpi:u32)->i32; }
        let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
        let metrics = (unsafe { GetSystemMetricsForDpi(49,dpi) }.clamp(16,64) as u32,native_taskbar_light_theme());
        *cached = Some((hwnd,std::time::Instant::now(),metrics)); metrics
    }
    #[cfg(not(target_os = "windows"))]
    { let _=hwnd; (16,false) }
}

/// Diagnostic atlas uses the exact same rasterizer as the native HICONs.
pub fn thumbnail_toolbar_png() -> Option<Vec<u8>> {
    #[cfg(target_os = "windows")]
    {
        let size = thumbnail_toolbar_metrics(get_registered_hwnd()).0;
        let mut atlas = image::RgbaImage::new(size*8,size);
        for (index,glyph) in [ThumbnailGlyph::Back,ThumbnailGlyph::Play,ThumbnailGlyph::Pause,
            ThumbnailGlyph::Forward,ThumbnailGlyph::Mute,ThumbnailGlyph::Unmute,
            ThumbnailGlyph::Fullscreen,ThumbnailGlyph::Restore].into_iter().enumerate() {
            image::imageops::overlay(&mut atlas,&thumbnail_icon_pixels(glyph,size),index as i64*size as i64,0);
        }
        let mut out=std::io::Cursor::new(Vec::new());
        atlas.write_to(&mut out,image::ImageFormat::Png).ok()?;
        Some(out.into_inner())
    }
    #[cfg(not(target_os = "windows"))]
    { None }
}

#[cfg(target_os = "windows")]
fn taskbar_thumbnail_buttons(
    hwnd: isize,
    is_paused: bool,
    is_muted: bool,
    is_fullscreen: bool,
    has_media: bool,
    enabled: bool,
) -> Result<
    (
        Vec<windows::Win32::UI::Shell::THUMBBUTTON>,
        Vec<windows::Win32::UI::WindowsAndMessaging::HICON>,
    ),
    String,
> {
    use windows::Win32::UI::Shell::{
        THB_FLAGS, THB_ICON, THB_TOOLTIP, THBF_DISABLED, THBF_ENABLED, THBF_HIDDEN, THUMBBUTTON,
    };

    let flags = if !enabled {
        THBF_HIDDEN
    } else if has_media {
        THBF_ENABLED
    } else {
        THBF_DISABLED
    };
    let specs = [
        (THUMB_BUTTON_PREV, ThumbnailGlyph::Back),
        (
            THUMB_BUTTON_PLAYPAUSE,
            if is_paused {
                ThumbnailGlyph::Play
            } else {
                ThumbnailGlyph::Pause
            },
        ),
        (THUMB_BUTTON_NEXT, ThumbnailGlyph::Forward),
        (
            THUMB_BUTTON_MUTE,
            if is_muted {
                ThumbnailGlyph::Unmute
            } else {
                ThumbnailGlyph::Mute
            },
        ),
        (
            THUMB_BUTTON_FULLSCREEN,
            if is_fullscreen {
                ThumbnailGlyph::Restore
            } else {
                ThumbnailGlyph::Fullscreen
            },
        ),
    ];
    let icons = specs
        .iter()
        .map(|(_, glyph)| {
            create_thumbnail_button_icon(*glyph, thumbnail_toolbar_metrics(hwnd).0)
                .map_err(|error| format!("create thumbnail-toolbar icon: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let buttons = specs
        .iter()
        .zip(icons.iter())
        .map(|((id, _), icon)| THUMBBUTTON {
            dwMask: THB_FLAGS | THB_TOOLTIP | THB_ICON,
            iId: *id,
            iBitmap: 0,
            hIcon: *icon,
            szTip: str_to_u16_buf_260(thumbnail_button_tooltip(
                *id,
                is_paused,
                is_muted,
                is_fullscreen,
            )),
            dwFlags: flags,
        })
        .collect();
    Ok((buttons, icons))
}

#[cfg(target_os = "windows")]
pub fn init_taskbar_thumbnail_toolbar(
    hwnd_raw: isize,
    is_paused: bool,
    is_muted: bool,
    is_fullscreen: bool,
    has_media: bool,
    enabled: bool,
) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::Win32::UI::Shell::{ITaskbarList3, TaskbarList};

    if hwnd_raw == 0 {
        return Err("invalid window handle (HWND is 0)".to_string());
    }
    let hwnd = HWND(hwnd_raw as *mut _);

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let taskbar: ITaskbarList3 = CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("failed to instantiate ITaskbarList3: {e}"))?;
        taskbar
            .HrInit()
            .map_err(|e| format!("ITaskbarList3::HrInit failed: {e}"))?;
        THUMBNAIL_TOOLBAR_ENABLED.store(enabled, Ordering::Relaxed);
        THUMBNAIL_TOOLBAR_HAS_MEDIA.store(has_media, Ordering::Relaxed);
        let (buttons, icons) = taskbar_thumbnail_buttons(
            hwnd_raw,
            is_paused,
            is_muted,
            is_fullscreen,
            has_media,
            enabled,
        )?;
        THUMBNAIL_TOOLBAR_ADD_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        let add_result = taskbar.ThumbBarAddButtons(hwnd, &buttons);
        let result = match add_result {
            Ok(()) => {
                THUMBNAIL_TOOLBAR_ADD_SUCCESSES.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(add_error) => {
                // A toolbar already accepted by Explorer rejects a duplicate
                // Add with E_INVALIDARG. Updating that toolbar is a successful
                // recovery, not an initialization failure that should hide it.
                THUMBNAIL_TOOLBAR_UPDATE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
                taskbar.ThumbBarUpdateButtons(hwnd, &buttons).map(|()| {
                    THUMBNAIL_TOOLBAR_UPDATE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
                }).map_err(|update_error| format!(
                    "ThumbBarAddButtons failed: {add_error}; ThumbBarUpdateButtons recovery failed: {update_error}"
                ))
            }
        };
        if result.is_ok() {
            // Explorer may consume HICONs after this COM call returns. Keep
            // the current state atlas alive until the next update.
            retain_thumbnail_toolbar_icons(icons);
            THUMBNAIL_TOOLBAR_ADDED_HWND.store(hwnd_raw, Ordering::Release);
            if let Ok(mut error) = THUMBNAIL_TOOLBAR_LAST_ERROR.lock() {
                *error = None;
            }
        } else {
            destroy_thumbnail_toolbar_icons(icons);
            if let Ok(mut error) = THUMBNAIL_TOOLBAR_LAST_ERROR.lock() {
                *error = result.as_ref().err().cloned();
            }
        }
        result
    }
}

#[cfg(not(target_os = "windows"))]
pub fn init_taskbar_thumbnail_toolbar(
    _hwnd_raw: isize,
    _is_paused: bool,
    _is_muted: bool,
    _is_fullscreen: bool,
    _has_media: bool,
    _enabled: bool,
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn update_taskbar_thumbnail_buttons(
    hwnd_raw: isize,
    is_paused: bool,
    is_muted: bool,
    is_fullscreen: bool,
    has_media: bool,
    enabled: bool,
) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::Win32::UI::Shell::{ITaskbarList3, TaskbarList};

    if hwnd_raw == 0 {
        return Err("invalid window handle (HWND is 0)".to_string());
    }
    let hwnd = HWND(hwnd_raw as *mut _);

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let taskbar: ITaskbarList3 = CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("failed to instantiate ITaskbarList3: {e}"))?;
        taskbar
            .HrInit()
            .map_err(|e| format!("ITaskbarList3::HrInit failed: {e}"))?;
        THUMBNAIL_TOOLBAR_ENABLED.store(enabled, Ordering::Relaxed);
        THUMBNAIL_TOOLBAR_HAS_MEDIA.store(has_media, Ordering::Relaxed);
        let (buttons, icons) =
            taskbar_thumbnail_buttons(hwnd_raw, is_paused, is_muted, is_fullscreen, has_media, enabled)?;
        THUMBNAIL_TOOLBAR_UPDATE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        let update_result = taskbar.ThumbBarUpdateButtons(hwnd, &buttons);
        let result = match update_result {
            Ok(()) => {
                THUMBNAIL_TOOLBAR_UPDATE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(update_error) => {
                // Explorer may recreate the taskbar button without Pealayer
                // observing the broadcast during a focus/minimize transition.
                // Re-add the toolbar immediately instead of waiting for a
                // future state change that may never occur.
                THUMBNAIL_TOOLBAR_ADD_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
                taskbar.ThumbBarAddButtons(hwnd, &buttons).map(|()| {
                    THUMBNAIL_TOOLBAR_ADD_SUCCESSES.fetch_add(1, Ordering::Relaxed);
                }).map_err(|add_error| format!(
                    "ThumbBarUpdateButtons failed: {update_error}; ThumbBarAddButtons recovery failed: {add_error}"
                ))
            }
        };
        if result.is_ok() {
            retain_thumbnail_toolbar_icons(icons);
            THUMBNAIL_TOOLBAR_ADDED_HWND.store(hwnd_raw, Ordering::Release);
            if let Ok(mut error) = THUMBNAIL_TOOLBAR_LAST_ERROR.lock() {
                *error = None;
            }
        } else {
            destroy_thumbnail_toolbar_icons(icons);
            if let Ok(mut error) = THUMBNAIL_TOOLBAR_LAST_ERROR.lock() {
                *error = result.as_ref().err().cloned();
            }
        }
        result
    }
}

#[cfg(not(target_os = "windows"))]
pub fn update_taskbar_thumbnail_buttons(
    _hwnd_raw: isize,
    _is_paused: bool,
    _is_muted: bool,
    _is_fullscreen: bool,
    _has_media: bool,
    _enabled: bool,
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
    update_video_taskbar_thumbnail(hwnd_raw, None)
}

/// Enables Pealayer's MPV-only iconic thumbnail while media is visible.
///
/// A compositor clip still samples the complete egui swapchain on Windows 11
/// and regresses to an application-UI thumbnail. The explicit iconic bitmap is
/// sourced from MPV's offscreen framebuffer instead, while the actual window
/// and DWM Peek remain ordinary full-application surfaces.
#[cfg(target_os = "windows")]
pub fn update_video_taskbar_thumbnail(
    hwnd_raw: isize,
    video_rect: Option<[i32; 4]>,
) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::Win32::UI::Shell::{ITaskbarList3, TaskbarList};

    if hwnd_raw == 0 {
        return Err("invalid window handle (HWND is 0)".to_string());
    }
    let normalized = video_rect.filter(|rect| rect[2] > rect[0] && rect[3] > rect[1]);
    crate::platform::taskbar_preview::configure(hwnd_raw, normalized.is_some())?;
    if TASKBAR_THUMBNAIL_CLIP
        .lock()
        .is_ok_and(|cached| *cached == Some((hwnd_raw, normalized)))
    {
        return Ok(());
    }
    let hwnd = HWND(hwnd_raw as *mut _);
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let taskbar: ITaskbarList3 = CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("create taskbar thumbnail service: {error}"))?;
        taskbar
            .HrInit()
            .map_err(|error| format!("initialize taskbar thumbnail service: {error}"))?;
        // Clear any clip left by an older build. The iconic bitmap already
        // contains video only and must not be cropped a second time.
        taskbar
            .SetThumbnailClip(hwnd, std::ptr::null())
            .map_err(|error| format!("clear legacy taskbar thumbnail clip: {error}"))?;
    }
    if let Ok(mut cached) = TASKBAR_THUMBNAIL_CLIP.lock() {
        *cached = Some((hwnd_raw, normalized));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn configure_video_taskbar_thumbnail(_hwnd_raw: isize) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn update_video_taskbar_thumbnail(
    _hwnd_raw: isize,
    _video_rect: Option<[i32; 4]>,
) -> Result<(), String> {
    Ok(())
}

pub const WM_TRAYICON: u32 = 0x8000 + 101; // WM_APP + 101
pub const TRAY_CMD_PLAYPAUSE: u32 = 2001;
pub const TRAY_CMD_MUTE: u32 = 2002;
pub const TRAY_CMD_OPEN: u32 = 2003;
pub const TRAY_CMD_EXIT: u32 = 2004;
pub const TRAY_CMD_SHOW: u32 = 2005;
pub const MEDIA_KEY_CMD_PLAY: u32 = 2101;
pub const MEDIA_KEY_CMD_PAUSE: u32 = 2102;
pub const MEDIA_KEY_CMD_STOP: u32 = 2103;
pub const MEDIA_KEY_CMD_NEXT: u32 = 2104;
pub const MEDIA_KEY_CMD_PREVIOUS: u32 = 2105;

fn shell_command_for_appcommand(appcommand: u32, enabled: bool, has_media: bool) -> Option<u32> {
    if !enabled || !has_media {
        return None;
    }
    // Foreground-window fallback for keyboards that emit WM_APPCOMMAND.
    // Souvlaki remains the single global SMTC/MPRIS/Now Playing registration.
    Some(match appcommand {
        11 => MEDIA_KEY_CMD_NEXT,
        12 => MEDIA_KEY_CMD_PREVIOUS,
        13 => MEDIA_KEY_CMD_STOP,
        14 => THUMB_BUTTON_PLAYPAUSE,
        46 => MEDIA_KEY_CMD_PLAY,
        47 => MEDIA_KEY_CMD_PAUSE,
        _ => return None,
    })
}

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
    _subclass_id: usize,
    _reference_data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::WindowsAndMessaging::{
        WM_APPCOMMAND, WM_COMMAND, WM_CONTEXTMENU, WM_LBUTTONDBLCLK, WM_RBUTTONUP, WM_SETICON,
    };

    observe_native_window_message(message);

    if message == WM_SETICON {
        // eframe owns the HICON lifetime. Shell copies it; do not destroy it.
        let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
        if lparam.0 != 0 {
            let _ = refresh_system_tray_window_icon(hwnd.0 as isize, lparam.0);
        }
        return result;
    }

    if matches!(message, 0x02e0 | 0x031a | 0x001a) { // DPI/theme/system settings
        THUMBNAIL_METRICS_DIRTY.store(true, Ordering::Release);
    }

    if crate::platform::taskbar_preview::handle_request(hwnd.0 as isize, message, lparam.0) {
        return LRESULT(0);
    }

    let taskbar_created = TASKBAR_BUTTON_CREATED_MESSAGE.load(Ordering::Acquire);
    if taskbar_created != 0 && message == taskbar_created {
        TASKBAR_BUTTON_CREATED_EVENTS.fetch_add(1, Ordering::Relaxed);
        THUMBNAIL_TOOLBAR_ADDED_HWND.store(0, Ordering::Release);
        // Microsoft requires ThumbBarAddButtons after TaskbarButtonCreated.
        // Rebuild immediately: paused, inactive and minimized eframe windows
        // are not guaranteed to paint soon enough to restore missing actions.
        // DWM's iconic-bitmap attributes belong to this HWND and survive an
        // Explorer taskbar recreation, so preserve the video-only preview.
        let restored = init_taskbar_thumbnail_toolbar(
            hwnd.0 as isize,
            SHELL_PAUSED.load(Ordering::Relaxed),
            SHELL_MUTED.load(Ordering::Relaxed),
            SHELL_FULLSCREEN.load(Ordering::Relaxed),
            SHELL_HAS_MEDIA.load(Ordering::Relaxed),
            THUMBNAIL_TOOLBAR_ENABLED.load(Ordering::Relaxed),
        )
        .is_ok();
        SHELL_REINITIALIZE.store(!restored, Ordering::Release);
        crate::platform::taskbar_preview::request_repaint();
    }

    if message == WM_MOVING_VALUE
        && unsafe { constrain_native_moving_rect(hwnd.0 as isize, lparam.0) }
    {
        // WM_MOVING uses the same in/out RECT convention as WM_SIZING.
        return LRESULT(1);
    }

    if message == WM_SIZING_VALUE && unsafe { constrain_native_sizing_rect(wparam.0, lparam.0) } {
        // WM_SIZING expects TRUE after the application updates the proposed
        // screen-coordinate RECT in place.
        return LRESULT(1);
    }

    if message == WM_TRAYICON {
        let mouse_message = lparam.0 as u32;
        if mouse_message == WM_RBUTTONUP || mouse_message == WM_CONTEXTMENU {
            if let Some(command) = show_tray_popup_menu(
                hwnd.0 as isize,
                SHELL_PAUSED.load(Ordering::Relaxed),
                SHELL_MUTED.load(Ordering::Relaxed),
                SHELL_HAS_MEDIA.load(Ordering::Relaxed),
            ) {
                queue_shell_command(command);
            }
            return LRESULT(0);
        }
        if mouse_message == WM_LBUTTONDBLCLK {
            queue_shell_command(TRAY_CMD_SHOW);
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
                THUMB_BUTTON_PREV
                    | THUMB_BUTTON_PLAYPAUSE
                    | THUMB_BUTTON_NEXT
                    | THUMB_BUTTON_MUTE
                    | THUMB_BUTTON_FULLSCREEN
            )
        {
            SHELL_COMMAND_MESSAGES.fetch_add(1, Ordering::Relaxed);
            queue_shell_command(command);
            return LRESULT(0);
        }
    } else if message == WM_APPCOMMAND {
        // GET_APPCOMMAND_LPARAM: high word with device bits masked out.
        let appcommand = ((lparam.0 as usize >> 16) & 0x07ff) as u32;
        if let Some(command) = shell_command_for_appcommand(
            appcommand,
            SHELL_MEDIA_KEYS_ENABLED.load(Ordering::Relaxed),
            SHELL_HAS_MEDIA.load(Ordering::Relaxed),
        ) {
            queue_shell_command(command);
            return LRESULT(1);
        }
    }

    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

#[cfg(target_os = "windows")]
#[link(name = "comctl32")]
unsafe extern "system" {
    fn SetWindowSubclass(
        hwnd: windows::Win32::Foundation::HWND,
        callback: Option<
            unsafe extern "system" fn(
                windows::Win32::Foundation::HWND,
                u32,
                windows::Win32::Foundation::WPARAM,
                windows::Win32::Foundation::LPARAM,
                usize,
                usize,
            ) -> windows::Win32::Foundation::LRESULT,
        >,
        subclass_id: usize,
        reference_data: usize,
    ) -> windows::core::BOOL;
    fn DefSubclassProc(
        hwnd: windows::Win32::Foundation::HWND,
        message: u32,
        wparam: windows::Win32::Foundation::WPARAM,
        lparam: windows::Win32::Foundation::LPARAM,
    ) -> windows::Win32::Foundation::LRESULT;
}

#[cfg(target_os = "windows")]
pub fn install_shell_message_hook(hwnd_raw: isize) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW;
    use windows::core::w;

    if hwnd_raw == 0 {
        return Err("invalid window handle (HWND is 0)".to_string());
    }
    if SHELL_SUBCLASS_HWND.load(Ordering::Acquire) == hwnd_raw {
        return Ok(());
    }
    let taskbar_created = unsafe { RegisterWindowMessageW(w!("TaskbarButtonCreated")) };
    if taskbar_created == 0 {
        return Err("RegisterWindowMessageW TaskbarButtonCreated failed".to_string());
    }
    TASKBAR_BUTTON_CREATED_MESSAGE.store(taskbar_created, Ordering::Release);
    const PEALAYER_SHELL_SUBCLASS_ID: usize = 0x5045_414c;
    let installed = unsafe {
        SetWindowSubclass(
            HWND(hwnd_raw as *mut _),
            Some(shell_window_proc),
            PEALAYER_SHELL_SUBCLASS_ID,
            0,
        )
    };
    if !installed.as_bool() {
        return Err("SetWindowSubclass failed".to_string());
    }
    SHELL_SUBCLASS_HWND.store(hwnd_raw, Ordering::Release);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn install_shell_message_hook(_hwnd_raw: isize) -> Result<(), String> {
    Ok(())
}

pub fn update_shell_command_state(
    is_paused: bool,
    is_muted: bool,
    is_fullscreen: bool,
    has_media: bool,
    media_keys_enabled: bool,
) {
    SHELL_PAUSED.store(is_paused, Ordering::Relaxed);
    SHELL_MUTED.store(is_muted, Ordering::Relaxed);
    SHELL_FULLSCREEN.store(is_fullscreen, Ordering::Relaxed);
    SHELL_HAS_MEDIA.store(has_media, Ordering::Relaxed);
    SHELL_MEDIA_KEYS_ENABLED.store(media_keys_enabled, Ordering::Relaxed);
}

pub fn take_shell_command() -> Option<u32> {
    SHELL_COMMANDS
        .lock()
        .ok()
        .and_then(|mut commands| commands.pop_front())
}

pub fn shell_command_diagnostics() -> serde_json::Value {
    serde_json::json!({
        "hook_hwnd": SHELL_SUBCLASS_HWND.load(Ordering::Acquire),
        "registered_hwnd": get_registered_hwnd(),
        "thumbnail_click_messages": SHELL_COMMAND_MESSAGES.load(Ordering::Relaxed),
        "commands_queued": SHELL_COMMANDS_QUEUED.load(Ordering::Relaxed),
        "queue_depth": SHELL_COMMANDS.lock().map(|queue| queue.len()).unwrap_or_default(),
        "has_media": SHELL_HAS_MEDIA.load(Ordering::Relaxed),
        "paused": SHELL_PAUSED.load(Ordering::Relaxed),
        "muted": SHELL_MUTED.load(Ordering::Relaxed),
        "thumbnail_toolbar": {
            "added_hwnd": THUMBNAIL_TOOLBAR_ADDED_HWND.load(Ordering::Acquire),
            "enabled": THUMBNAIL_TOOLBAR_ENABLED.load(Ordering::Relaxed),
            "has_media": THUMBNAIL_TOOLBAR_HAS_MEDIA.load(Ordering::Relaxed),
            "add_attempts": THUMBNAIL_TOOLBAR_ADD_ATTEMPTS.load(Ordering::Relaxed),
            "add_successes": THUMBNAIL_TOOLBAR_ADD_SUCCESSES.load(Ordering::Relaxed),
            "update_attempts": THUMBNAIL_TOOLBAR_UPDATE_ATTEMPTS.load(Ordering::Relaxed),
            "update_successes": THUMBNAIL_TOOLBAR_UPDATE_SUCCESSES.load(Ordering::Relaxed),
            "taskbar_button_created_events": TASKBAR_BUTTON_CREATED_EVENTS.load(Ordering::Relaxed),
            "last_error": THUMBNAIL_TOOLBAR_LAST_ERROR.lock().ok().and_then(|error| error.clone()),
        },
    })
}

pub fn take_shell_reinitialize_request() -> bool {
    let requested = SHELL_REINITIALIZE.swap(false, Ordering::AcqRel);
    if requested && let Ok(mut cached) = TASKBAR_THUMBNAIL_CLIP.lock() {
        *cached = None;
    }
    requested
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
        NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_MODIFY, NOTIFYICONDATAW,
        Shell_NotifyIconW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GCLP_HICON, GetClassLongPtrW, HICON, LoadIconW, SendMessageW, WM_GETICON, ICON_SMALL2};
    use windows::core::PCWSTR;

    if hwnd_raw == 0 {
        return Err("invalid window handle (HWND is 0)".to_string());
    }
    let hwnd = HWND(hwnd_raw as *mut _);

    unsafe {
        let module =
            GetModuleHandleW(None).map_err(|error| format!("GetModuleHandleW failed: {error}"))?;
        let instance: windows::Win32::Foundation::HINSTANCE = module.into();
        let window_icon = SendMessageW(hwnd, WM_GETICON,
            Some(windows::Win32::Foundation::WPARAM(ICON_SMALL2 as usize)), None).0;
        let hicon = if window_icon != 0 {
            HICON(window_icon as *mut _)
        } else {
            LoadIconW(Some(instance), PCWSTR(1usize as *const u16))
                .unwrap_or_else(|_| HICON(GetClassLongPtrW(hwnd, GCLP_HICON) as *mut _))
        };
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

        let added = Shell_NotifyIconW(NIM_ADD, &mut nid);
        // Explorer broadcasts TaskbarButtonCreated after recreating its taskbar
        // surfaces.  The thumbnail toolbar must be added again, but the tray
        // icon commonly survives that event.  Treat an existing icon as an
        // idempotent registration and refresh it in place; otherwise a failed
        // duplicate NIM_ADD keeps the whole shell initializer retrying every
        // frame and repeatedly tears down/re-adds the thumbnail toolbar.
        if added.as_bool() || Shell_NotifyIconW(NIM_MODIFY, &mut nid).as_bool() {
            Ok(())
        } else {
            Err("Shell_NotifyIconW NIM_ADD and NIM_MODIFY failed".to_string())
        }
    }
}

#[cfg(target_os = "windows")]
fn refresh_system_tray_window_icon(hwnd_raw: isize, icon_raw: isize) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::{NIF_ICON, NIM_MODIFY, NOTIFYICONDATAW, Shell_NotifyIconW};
    use windows::Win32::UI::WindowsAndMessaging::HICON;
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: HWND(hwnd_raw as *mut _),
        uID: 1,
        uFlags: NIF_ICON,
        hIcon: HICON(icon_raw as *mut _),
        ..Default::default()
    };
    unsafe { Shell_NotifyIconW(NIM_MODIFY, &mut nid) }.as_bool()
        .then_some(())
        .ok_or_else(|| "Shell_NotifyIconW icon refresh failed".to_string())
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
    use windows::Win32::UI::Shell::{NIF_ICON, NIF_TIP, NIM_MODIFY, NOTIFYICONDATAW, Shell_NotifyIconW};
    use windows::Win32::UI::WindowsAndMessaging::{HICON, ICON_SMALL2, SendMessageW, WM_GETICON};
    use windows::Win32::Foundation::{LPARAM, WPARAM};

    if hwnd_raw == 0 {
        return Err("invalid window handle (HWND is 0)".to_string());
    }
    let hwnd = HWND(hwnd_raw as *mut _);

    unsafe {
        let icon = SendMessageW(hwnd, WM_GETICON, Some(WPARAM(ICON_SMALL2 as usize)), Some(LPARAM(0))).0;
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: if icon == 0 { NIF_TIP } else { NIF_TIP | NIF_ICON },
            hIcon: HICON(icon as *mut _),
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
            Some(0),
            hwnd,
            None,
        );
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
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

#[cfg(not(target_os = "windows"))]
pub fn sync_windows_jump_list_with_options(
    _recent_media: &[std::path::PathBuf],
    _include_quick_actions: bool,
) {
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
        use crate::config::ColorPalette;
        assert_eq!(
            decoration_colors(true, ColorPalette::Studio),
            (0x0016100d, 0x00f7f2ee)
        );
        assert_eq!(
            decoration_colors(false, ColorPalette::Studio),
            (0x00fcfaf8, 0x002b2017)
        );
        assert_eq!(
            decoration_colors(true, ColorPalette::Native),
            (0x00202020, 0x00eeeeee)
        );
        assert_eq!(
            decoration_colors(false, ColorPalette::Native),
            (0x00f3f3f3, 0x00342a23)
        );
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
            thumbnail_button_tooltip(THUMB_BUTTON_PREV, false, false, false),
            "Back 10 seconds"
        );
        assert_eq!(
            thumbnail_button_tooltip(THUMB_BUTTON_PLAYPAUSE, true, false, false),
            "Play"
        );
        assert_eq!(
            thumbnail_button_tooltip(THUMB_BUTTON_PLAYPAUSE, false, false, false),
            "Pause"
        );
        assert_eq!(
            thumbnail_button_tooltip(THUMB_BUTTON_NEXT, false, false, false),
            "Forward 10 seconds"
        );
        assert_eq!(
            thumbnail_button_tooltip(THUMB_BUTTON_MUTE, false, false, false),
            "Mute"
        );
        assert_eq!(
            thumbnail_button_tooltip(THUMB_BUTTON_MUTE, false, true, false),
            "Unmute"
        );
        assert_eq!(
            thumbnail_button_tooltip(THUMB_BUTTON_FULLSCREEN, false, false, false),
            "Fullscreen"
        );
        assert_eq!(
            thumbnail_button_tooltip(THUMB_BUTTON_FULLSCREEN, false, false, true),
            "Exit fullscreen"
        );
    }

    #[test]
    fn appcommand_media_keys_respect_configuration_and_media_state() {
        assert_eq!(shell_command_for_appcommand(14, true, true), Some(THUMB_BUTTON_PLAYPAUSE));
        assert_eq!(shell_command_for_appcommand(46, true, true), Some(MEDIA_KEY_CMD_PLAY));
        assert_eq!(shell_command_for_appcommand(47, true, true), Some(MEDIA_KEY_CMD_PAUSE));
        assert_eq!(shell_command_for_appcommand(13, true, true), Some(MEDIA_KEY_CMD_STOP));
        assert_eq!(shell_command_for_appcommand(11, true, true), Some(MEDIA_KEY_CMD_NEXT));
        assert_eq!(shell_command_for_appcommand(12, true, true), Some(MEDIA_KEY_CMD_PREVIOUS));
        assert_eq!(shell_command_for_appcommand(14, false, true), None);
        assert_eq!(shell_command_for_appcommand(14, true, false), None);
        assert_eq!(shell_command_for_appcommand(999, true, true), None);
    }

    #[test]
    fn windows_quick_actions_use_supported_unified_cli_switches() {
        assert!(WINDOWS_QUICK_ACTIONS.len() >= 5);
        for action in WINDOWS_QUICK_ACTIONS {
            let args = std::iter::once("pealayer".to_string())
                .chain(action.arguments.split_whitespace().map(str::to_string))
                .collect::<Vec<_>>();
            assert!(
                crate::cli::parse_cli_args(args).is_ok(),
                "{} uses unsupported arguments {}",
                action.title,
                action.arguments
            );
        }
    }

    #[test]
    fn windows_quick_action_icons_are_distinct_embedded_resources() {
        let mut ids = std::collections::HashSet::new();
        let mut images = std::collections::HashSet::new();
        for action in WINDOWS_QUICK_ACTIONS {
            assert!(action.icon_resource_id > 1, "task must not use the app logo");
            assert!(ids.insert(action.icon_resource_id), "duplicate task icon resource");
            assert_eq!(action.icon_location_index(), -(action.icon_resource_id as i32));
            let ico = super::super::windows_shell_icons::shell_icon_ico(action.icon_glyph);
            assert_eq!(&ico[..4], &[0, 0, 1, 0]);
            assert_eq!(u16::from_le_bytes([ico[4], ico[5]]) as usize,
                super::super::windows_shell_icons::SHELL_ICON_SIZES.len());
            let decoded = image::load_from_memory_with_format(&ico, image::ImageFormat::Ico)
                .expect("generated shell ICO must decode").to_rgba8();
            assert_eq!(decoded.dimensions(), (64, 64));
            assert!(images.insert(decoded.into_raw()), "two actions have identical icons");
            for size in super::super::windows_shell_icons::SHELL_ICON_SIZES {
                let pixels = super::super::windows_shell_icons::shell_icon_rgba(action.icon_glyph, size);
                assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
                for x in 0..size {
                    assert_eq!(pixels[(x * 4 + 3) as usize], 0, "top edge clipped");
                    assert_eq!(pixels[((size - 1) * size * 4 + x * 4 + 3) as usize], 0, "bottom edge clipped");
                }
            }
        }
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
    fn native_move_resize_messages_track_the_operation_until_it_ends() {
        WINDOW_MOVE_RESIZE_ACTIVE.store(false, Ordering::Release);
        WINDOW_MOVE_RESIZE_ENDED.store(false, Ordering::Release);

        assert_eq!(
            native_window_operation_transition(WM_ENTERSIZEMOVE_VALUE),
            Some(true)
        );
        observe_native_window_message(WM_ENTERSIZEMOVE_VALUE);
        assert!(native_window_operation_active());
        assert!(!take_native_window_operation_ended());

        observe_native_window_message(WM_EXITSIZEMOVE_VALUE);
        assert!(!native_window_operation_active());
        assert!(take_native_window_operation_ended());
        assert!(!take_native_window_operation_ended());
        assert_eq!(native_window_operation_transition(0x000F), None);
    }

    #[test]
    fn live_video_is_the_default_move_loop_policy_with_freeze_as_fallback() {
        assert!(video_rendering_allowed_during_window_operation(
            false, false
        ));
        assert!(video_rendering_allowed_during_window_operation(false, true));
        assert!(video_rendering_allowed_during_window_operation(true, true));
        assert!(!video_rendering_allowed_during_window_operation(
            true, false
        ));
        assert!(compositor_move_frame_pump_active(true, true, true));
        assert!(!compositor_move_frame_pump_active(false, true, true));
        assert!(!compositor_move_frame_pump_active(true, false, true));
        assert!(!compositor_move_frame_pump_active(true, true, false));
    }

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> SizingRect {
        SizingRect {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn magnetic_window_snap_threshold_scales_with_destination_dpi() {
        assert_eq!(scaled_snap_threshold(16, 0), 16);
        assert_eq!(scaled_snap_threshold(16, 96), 16);
        assert_eq!(scaled_snap_threshold(16, 144), 24);
        assert_eq!(scaled_snap_threshold(16, 192), 32);
    }

    #[test]
    fn magnetic_window_snap_acquires_all_work_area_edges() {
        let work = rect(0, 0, 1000, 800);
        for (moving, expected, expected_result) in [
            (
                rect(8, 200, 108, 300),
                rect(0, 200, 100, 300),
                SnapResult {
                    snapped_x: true,
                    snapped_y: false,
                },
            ),
            (
                rect(892, 200, 992, 300),
                rect(900, 200, 1000, 300),
                SnapResult {
                    snapped_x: true,
                    snapped_y: false,
                },
            ),
            (
                rect(200, 8, 300, 108),
                rect(200, 0, 300, 100),
                SnapResult {
                    snapped_x: false,
                    snapped_y: true,
                },
            ),
            (
                rect(200, 692, 300, 792),
                rect(200, 700, 300, 800),
                SnapResult {
                    snapped_x: false,
                    snapped_y: true,
                },
            ),
        ] {
            let (snapped, result) = snap_to_work_area(moving, work, 10);
            assert_eq!(snapped, expected);
            assert_eq!(result, expected_result);
        }
    }

    #[test]
    fn magnetic_window_snap_breakaway_uses_raw_acquisition_rect() {
        let work = rect(0, 0, 1000, 800);
        let raw = rect(8, 100, 108, 200);
        let mut drag = MagneticDragSession::new();
        let (snapped, result, handled) = drag.apply(raw, (100, 150), work, 96, 10, false);
        assert!(handled && result.snapped_x);
        assert_eq!(snapped, rect(0, 100, 100, 200));

        let (released, result, handled) = drag.apply(snapped, (120, 150), work, 96, 10, false);
        assert!(handled && !result.snapped_x);
        assert_eq!(released, rect(28, 100, 128, 200));
    }

    #[test]
    fn magnetic_window_snap_control_detaches_then_rearms_in_same_drag() {
        let work = rect(0, 0, 1000, 800);
        let mut drag = MagneticDragSession::new();
        let (snapped, _, _) = drag.apply(rect(8, 100, 108, 200), (100, 150), work, 96, 10, false);

        let (detached, result, handled) =
            drag.apply(snapped, (110, 150), SizingRect::default(), 96, 10, true);
        assert!(handled);
        assert_eq!(result, SnapResult::default());
        assert_eq!(detached, rect(18, 100, 118, 200));

        let (resnapped, result, handled) =
            drag.apply(rect(5, 120, 105, 220), (115, 170), work, 96, 10, false);
        assert!(handled && result.snapped_x && !result.snapped_y);
        assert_eq!(resnapped, rect(0, 120, 100, 220));
    }

    #[test]
    fn magnetic_window_snap_corner_axes_release_independently() {
        let work = rect(0, 0, 1000, 800);
        let mut drag = MagneticDragSession::new();
        let (corner, result, _) = drag.apply(rect(8, 8, 108, 108), (100, 100), work, 96, 10, false);
        assert!(result.snapped_x && result.snapped_y);

        let (released, result, handled) = drag.apply(corner, (120, 103), work, 96, 10, false);
        assert!(handled && !result.snapped_x && result.snapped_y);
        assert_eq!(released, rect(28, 0, 128, 100));
    }

    #[test]
    fn magnetic_window_snap_reset_clears_latches_and_control_bypass() {
        let work = rect(0, 0, 1000, 800);
        let mut drag = MagneticDragSession::new();
        let (snapped, _, _) = drag.apply(rect(8, 100, 108, 200), (100, 150), work, 96, 10, false);
        let _ = drag.apply(snapped, (110, 150), SizingRect::default(), 96, 10, true);
        assert!(drag.control_bypass);
        drag.reset();
        assert_eq!(drag, MagneticDragSession::new());

        let (right, result, handled) =
            drag.apply(rect(892, 240, 992, 340), (942, 290), work, 96, 10, false);
        assert!(handled && result.snapped_x && !result.snapped_y);
        assert_eq!(right, rect(900, 240, 1000, 340));
    }

    fn assert_video_aspect(
        rect: SizingRect,
        constraint: SimpleVideoAspectConstraint,
        expected: f64,
    ) {
        let video_width = rect.width() - constraint.chrome_width;
        let video_height = rect.height() - constraint.chrome_height;
        assert!(video_width > 0 && video_height > 0);
        assert!((f64::from(video_width) / f64::from(video_height) - expected).abs() < 0.01);
    }

    #[test]
    fn native_right_edge_resize_keeps_the_live_video_aspect() {
        let constraint = SimpleVideoAspectConstraint {
            aspect_ratio: 16.0 / 9.0,
            chrome_width: 16,
            chrome_height: 92,
        };
        let proposed = SizingRect {
            left: 100,
            top: 80,
            right: 1116,
            bottom: 772,
        };
        let constrained = constrain_sizing_rect(proposed, WMSZ_RIGHT_VALUE, constraint).unwrap();
        assert_eq!(constrained.left, proposed.left);
        assert_eq!(
            constrained.top + constrained.height() / 2,
            proposed.top + proposed.height() / 2
        );
        assert_video_aspect(constrained, constraint, 16.0 / 9.0);
    }

    #[test]
    fn native_bottom_edge_resize_keeps_the_live_video_aspect() {
        let constraint = SimpleVideoAspectConstraint {
            aspect_ratio: 4.0 / 3.0,
            chrome_width: 18,
            chrome_height: 88,
        };
        let proposed = SizingRect {
            left: 200,
            top: 100,
            right: 1018,
            bottom: 788,
        };
        let constrained = constrain_sizing_rect(proposed, WMSZ_BOTTOM_VALUE, constraint).unwrap();
        assert_eq!(constrained.top, proposed.top);
        assert_video_aspect(constrained, constraint, 4.0 / 3.0);
    }

    #[test]
    fn native_top_left_corner_keeps_the_opposite_corner_fixed() {
        let constraint = SimpleVideoAspectConstraint {
            aspect_ratio: 2.35,
            chrome_width: 20,
            chrome_height: 90,
        };
        let proposed = SizingRect {
            left: 120,
            top: 110,
            right: 1320,
            bottom: 810,
        };
        let constrained = constrain_sizing_rect(proposed, WMSZ_TOPLEFT_VALUE, constraint).unwrap();
        assert_eq!(constrained.right, proposed.right);
        assert_eq!(constrained.bottom, proposed.bottom);
        assert_video_aspect(constrained, constraint, 2.35);
        assert!(constrain_sizing_rect(proposed, 0, constraint).is_none());
    }

    #[test]
    fn every_native_resize_edge_and_corner_preserves_video_geometry() {
        let constraint = SimpleVideoAspectConstraint {
            aspect_ratio: 16.0 / 10.0,
            chrome_width: 18,
            chrome_height: 86,
        };
        let proposed = SizingRect {
            left: 100,
            top: 120,
            right: 1118,
            bottom: 806,
        };
        for edge in [
            WMSZ_LEFT_VALUE,
            WMSZ_RIGHT_VALUE,
            WMSZ_TOP_VALUE,
            WMSZ_TOPLEFT_VALUE,
            WMSZ_TOPRIGHT_VALUE,
            WMSZ_BOTTOM_VALUE,
            WMSZ_BOTTOMLEFT_VALUE,
            WMSZ_BOTTOMRIGHT_VALUE,
        ] {
            let constrained = constrain_sizing_rect(proposed, edge, constraint)
                .unwrap_or_else(|| panic!("resize edge {edge} was not handled"));
            assert_video_aspect(constrained, constraint, constraint.aspect_ratio);
            if matches!(
                edge,
                WMSZ_LEFT_VALUE | WMSZ_TOPLEFT_VALUE | WMSZ_BOTTOMLEFT_VALUE
            ) {
                assert_eq!(constrained.right, proposed.right);
            }
            if matches!(
                edge,
                WMSZ_RIGHT_VALUE | WMSZ_TOPRIGHT_VALUE | WMSZ_BOTTOMRIGHT_VALUE
            ) {
                assert_eq!(constrained.left, proposed.left);
            }
            if matches!(
                edge,
                WMSZ_TOP_VALUE | WMSZ_TOPLEFT_VALUE | WMSZ_TOPRIGHT_VALUE
            ) {
                assert_eq!(constrained.bottom, proposed.bottom);
            }
            if matches!(
                edge,
                WMSZ_BOTTOM_VALUE | WMSZ_BOTTOMLEFT_VALUE | WMSZ_BOTTOMRIGHT_VALUE
            ) {
                assert_eq!(constrained.top, proposed.top);
            }
        }
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
