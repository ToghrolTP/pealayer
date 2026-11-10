//! Native top-level host for a detached DirectComposition video surface.
//!
//! The window intentionally contains no egui controls. mpv keeps ownership of
//! decoding and of the D3D11 composition swapchain; the composition module
//! merely retargets that same swapchain between this HWND and the main HWND.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::UpdateWindow;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW,
    DestroyWindow, GetClientRect, IsWindow, RegisterClassW, SW_HIDE, SW_RESTORE, ShowWindow,
    WINDOW_EX_STYLE, WM_CLOSE, WM_ERASEBKGND, WM_SIZE, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};
use windows::core::PCWSTR;

static VIDEO_HWND: AtomicIsize = AtomicIsize::new(0);
static USER_CLOSED: AtomicBool = AtomicBool::new(false);
static CLASS_REGISTERED: OnceLock<Result<(), String>> = OnceLock::new();

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn video_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CLOSE => {
            USER_CLOSED.store(true, Ordering::Release);
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            crate::platform::taskbar_preview::request_repaint();
            LRESULT(0)
        }
        WM_SIZE => {
            crate::platform::taskbar_preview::request_repaint();
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        // DirectComposition supplies the complete client surface. Suppressing
        // the class erase prevents a white flash while resizing or retargeting.
        WM_ERASEBKGND => LRESULT(1),
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn register_class() -> Result<(), String> {
    CLASS_REGISTERED
        .get_or_init(|| {
            let module = unsafe { GetModuleHandleW(None) }
                .map_err(|error| format!("resolve detached video module: {error}"))?;
            let class_name = wide_null("PealayerDetachedVideoHost");
            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
                lpfnWndProc: Some(video_window_proc),
                hInstance: HINSTANCE(module.0),
                lpszClassName: PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };
            let atom = unsafe { RegisterClassW(&class) };
            if atom == 0 {
                return Err(format!(
                    "register detached video window: {}",
                    windows::core::Error::from_thread()
                ));
            }
            Ok(())
        })
        .clone()
}

pub fn ensure(title: &str) -> Result<isize, String> {
    let current = VIDEO_HWND.load(Ordering::Acquire);
    if current != 0 && unsafe { IsWindow(Some(HWND(current as *mut _))).as_bool() } {
        USER_CLOSED.store(false, Ordering::Release);
        unsafe {
            let _ = ShowWindow(HWND(current as *mut _), SW_RESTORE);
        }
        return Ok(current);
    }
    register_class()?;
    let module = unsafe { GetModuleHandleW(None) }
        .map_err(|error| format!("resolve detached video module: {error}"))?;
    let class_name = wide_null("PealayerDetachedVideoHost");
    let title = wide_null(title);
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            960,
            540,
            None,
            None,
            Some(HINSTANCE(module.0)),
            None,
        )
    }
    .map_err(|error| format!("create detached video window: {error}"))?;
    let raw = hwnd.0 as isize;
    VIDEO_HWND.store(raw, Ordering::Release);
    USER_CLOSED.store(false, Ordering::Release);
    unsafe {
        let _ = UpdateWindow(hwnd);
    }
    Ok(raw)
}

pub fn hwnd() -> Option<isize> {
    let hwnd = VIDEO_HWND.load(Ordering::Acquire);
    (hwnd != 0 && unsafe { IsWindow(Some(HWND(hwnd as *mut _))).as_bool() }).then_some(hwnd)
}

pub fn client_rect() -> Option<RECT> {
    let hwnd = hwnd()?;
    let mut rect = RECT::default();
    unsafe { GetClientRect(HWND(hwnd as *mut _), &mut rect) }
        .ok()
        .map(|_| rect)
}

pub fn take_user_closed() -> bool {
    USER_CLOSED.swap(false, Ordering::AcqRel)
}

pub fn hide() {
    if let Some(hwnd) = hwnd() {
        unsafe {
            let _ = ShowWindow(HWND(hwnd as *mut _), SW_HIDE);
        }
    }
}

pub fn destroy() {
    let hwnd = VIDEO_HWND.swap(0, Ordering::AcqRel);
    if hwnd != 0 {
        unsafe {
            let _ = DestroyWindow(HWND(hwnd as *mut _));
        }
    }
}
