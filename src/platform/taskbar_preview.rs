//! Small taskbar cards consume the MPV-only bitmap. Full-size Task View/Peek
//! consumes a separate, on-demand composite of the actual application UI.
//! Readback follows new video frames at up to 30 Hz, not a guessed hover lease.
//! No decoding, disk I/O or GL runs
//! in the window procedure, and the real Pealayer window remains untouched.

use std::sync::{Mutex, OnceLock, mpsc::SyncSender};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Preview {
    enabled: bool,
    media: String,
    frame: Option<image::RgbaImage>,
    captured: Option<Instant>,
    frame_dirty: bool,
    captured_frames: u64,
    requests: u64,
    delivered: u64,
    last_error: Option<String>,
    configuration_error: Option<String>,
}

static PREVIEW: Mutex<Preview> = Mutex::new(Preview {
    enabled: false,
    media: String::new(),
    frame: None,
    captured: None,
    frame_dirty: false,
    captured_frames: 0,
    requests: 0,
    delivered: 0,
    last_error: None,
    configuration_error: None,
});
static REPAINT: OnceLock<eframe::egui::Context> = OnceLock::new();
static REPAINT_WAKE: OnceLock<SyncSender<()>> = OnceLock::new();
#[cfg(target_os = "windows")]
static FULL_REQUESTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(target_os = "windows")]
static MINIMIZE_REQUESTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static MINIMIZE_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static MINIMIZE_WAITING: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(target_os = "windows")]
#[derive(Default)]
struct FullPreview {
    frame: Option<std::sync::Arc<image::RgbaImage>>,
    captured: Option<Instant>,
    pending: Option<Instant>,
    requests: u64,
    captures: u64,
    delivered: u64,
    non_uniform: bool,
    error: Option<String>,
}
#[cfg(target_os = "windows")]
static FULL: std::sync::LazyLock<Mutex<FullPreview>> = std::sync::LazyLock::new(|| Mutex::new(FullPreview::default()));

#[cfg(target_os = "windows")]
fn full_rgba(image: &eframe::egui::ColorImage) -> Option<image::RgbaImage> {
    if image.size[0]==0 || image.size[1]==0 {return None;}
    let bytes: Vec<_> = image.pixels.iter().flat_map(|pixel| pixel.to_array()).collect();
    image::RgbaImage::from_raw(image.size[0] as u32,image.size[1] as u32,bytes)
}

#[cfg(target_os = "windows")]
fn remember_full(image: &eframe::egui::ColorImage) -> Option<std::sync::Arc<image::RgbaImage>> {
    let frame=full_rgba(image).map(std::sync::Arc::new)?;
    if let Ok(mut state)=FULL.lock() {
        state.frame=Some(frame.clone());state.captured=Some(Instant::now());state.pending=None;state.captures+=1;
        state.non_uniform=frame.pixels().skip(1).step_by(64).any(|pixel|pixel!=frame.get_pixel(0,0));
    }
    Some(frame)
}

pub fn cancel_minimize() {
    MINIMIZE_WAITING.store(0,std::sync::atomic::Ordering::Release);
    #[cfg(target_os="windows")]
    MINIMIZE_REQUESTED.store(false,std::sync::atomic::Ordering::Release);
}

/// Preserve the last complete UI before minimizing, without polling/capturing
/// it continuously or ever restoring the window merely to produce a preview.
pub fn minimize(ctx: &eframe::egui::Context) {
    #[cfg(target_os="windows")]
    if PREVIEW.lock().is_ok_and(|state|state.enabled) {
        use std::sync::atomic::Ordering;
        let token=MINIMIZE_SEQUENCE.fetch_add(1,Ordering::AcqRel)+1;
        MINIMIZE_WAITING.store(token,Ordering::Release);
        let completed=ctx.clone();
        ctx.request_screenshot(move |image| {
            let _=remember_full(&image);
            if MINIMIZE_WAITING.compare_exchange(token,0,Ordering::AcqRel,Ordering::Acquire).is_ok() {
                completed.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(true));
            }
        });
        // A lost/failed paint must never leave the user's minimize request
        // waiting forever. This short-lived fallback exists only for this action.
        let fallback=ctx.clone();
        let fallback_started=std::thread::Builder::new().name("pealayer-minimize".into()).spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            if MINIMIZE_WAITING.compare_exchange(token,0,Ordering::AcqRel,Ordering::Acquire).is_ok() {
                fallback.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(true));
            }
        });
        if fallback_started.is_err() && MINIMIZE_WAITING.compare_exchange(token,0,Ordering::AcqRel,Ordering::Acquire).is_ok() {
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(true));
        }
        return;
    }
    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(true));
}

#[cfg(target_os="windows")]
pub fn defer_system_minimize(hwnd: isize) -> bool {
    if hwnd!=crate::platform::windows::get_registered_hwnd() || !PREVIEW.lock().is_ok_and(|state|state.enabled) {return false;}
    MINIMIZE_REQUESTED.store(true,std::sync::atomic::Ordering::Release);
    request_repaint();true
}

#[cfg(target_os = "windows")]
fn capture_full_if_requested(ctx: &eframe::egui::Context) {
    use std::sync::atomic::Ordering;
    if !FULL_REQUESTED.swap(false,Ordering::AcqRel) {return;}
    let hwnd = crate::platform::windows::get_registered_hwnd();
    // The backend reads the finished egui+video framebuffer after painting.
    // No PrintWindow recursion, video-only replacement or GL in the WndProc.
    ctx.request_screenshot(move |image| {
        let Some(frame) = remember_full(&image) else {
            if let Ok(mut state)=FULL.lock() {state.pending=None;state.error=Some("The preview surface has no drawable extent".into());}
            return;
        };
        publish_full(hwnd,&frame);
    });
}

#[cfg(target_os = "windows")]
fn publish_full(hwnd: isize, frame: &image::RgbaImage) {
    use windows::Win32::{Foundation::{HWND,RECT},Graphics::{Dwm::DwmSetIconicLivePreviewBitmap,Gdi::DeleteObject},UI::WindowsAndMessaging::GetClientRect};
    let mut client=RECT::default();
    if unsafe {GetClientRect(HWND(hwnd as *mut _),&mut client)}.is_err() {return;}
    // A minimized HWND reports a zero client. DWM still accepts its cached
    // normal-size iconic representation; do not discard that valid bitmap.
    if client.right<=0 || client.bottom<=0 {client.right=frame.width() as i32;client.bottom=frame.height() as i32;}
    // DWM requires a 32-bit bitmap no larger than the current client area.
    let (w,h)=fit_size(frame.width(),frame.height(),client.right as u32,client.bottom as u32);
    let resized;
    let image=if w!=frame.width() || h!=frame.height() {
        resized=image::imageops::resize(frame,w,h,image::imageops::FilterType::Triangle);&resized
    }else{frame};
    let result=bitmap(image).and_then(|bitmap| unsafe {
        let result=DwmSetIconicLivePreviewBitmap(HWND(hwnd as *mut _),bitmap,None,1); // DWM_SIT_DISPLAYFRAME
        let _=DeleteObject(bitmap.into());result
    });
    if let Ok(mut state)=FULL.lock() {match result {Ok(())=>{state.delivered+=1;state.error=None;},Err(error)=>state.error=Some(error.to_string())}}
}

#[cfg(target_os = "windows")]
fn handle_full_request(hwnd: isize) -> bool {
    if !PREVIEW.lock().is_ok_and(|state|state.enabled) {return false;}
    let (frame,capture)=if let Ok(mut state)=FULL.lock() {
        state.requests+=1;
        // A snapshot is enough for Task View; do not read the entire 4K UI on
        // every video-frame invalidation. Paused previews cannot create loops.
        let drawable=!unsafe {windows::Win32::UI::WindowsAndMessaging::IsIconic(windows::Win32::Foundation::HWND(hwnd as *mut _))}.as_bool();
        let capture=drawable && state.captured.is_none_or(|at|at.elapsed()>=Duration::from_secs(1))
            && state.pending.is_none_or(|at|at.elapsed()>=Duration::from_secs(2));
        if capture {state.pending=Some(Instant::now());}
        (state.frame.clone(),capture)
    }else{(None,false)};
    if let Some(frame)=frame {publish_full(hwnd,&frame);}
    if capture {
        FULL_REQUESTED.store(true,std::sync::atomic::Ordering::Release);
        request_repaint();
    }
    true
}

#[cfg(target_os = "windows")]
fn wake_native_window() {
    use windows::Win32::{
        Foundation::HWND,
        Graphics::Gdi::{RDW_INVALIDATE, RDW_UPDATENOW, RedrawWindow},
    };

    let hwnd = crate::platform::windows::get_registered_hwnd();
    if hwnd != 0 {
        // request_repaint is normally sufficient, but Windows can suppress
        // the winit redraw while an inactive/minimized taskbar owner is being
        // controlled. RedrawWindow from this worker gives the native queue a
        // real paint edge without activating or focusing the application.
        unsafe {
            let _ = RedrawWindow(
                Some(HWND(hwnd as *mut _)),
                None,
                None,
                RDW_INVALIDATE | RDW_UPDATENOW,
            );
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn wake_native_window() {}

// Win32 message values are stable ABI constants. Keeping this decision pure
// lets every CI host verify that video-only rendering is limited to the small
// taskbar thumbnail and never leaks into the full-size Peek preview.
fn is_static_thumbnail_request(message: u32) -> bool {
    message == 0x0323 // WM_DWMSENDICONICTHUMBNAIL
}

pub fn register_repaint(ctx: &eframe::egui::Context) {
    if REPAINT.set(ctx.clone()).is_ok() {
        // Shell callbacks run inside the native window procedure. Scheduling
        // the repaint from a worker avoids a wake being coalesced into the
        // WM_COMMAND currently being dispatched while the app is inactive.
        let (sender, receiver) = std::sync::mpsc::sync_channel(4);
        let repaint = ctx.clone();
        let _ = std::thread::Builder::new()
            .name("pealayer-shell-wake".to_owned())
            .spawn(move || {
                while receiver.recv().is_ok() {
                    #[cfg(target_os = "windows")]
                    if MINIMIZE_REQUESTED.swap(false,std::sync::atomic::Ordering::AcqRel) {minimize(&repaint);}
                    #[cfg(target_os = "windows")]
                    capture_full_if_requested(&repaint);
                    repaint.request_repaint();
                    wake_native_window();
                    // A second edge publishes the state that results from the
                    // command (play/pause, mute, fullscreen) back to Explorer.
                    std::thread::sleep(Duration::from_millis(12));
                    repaint.request_repaint();
                    wake_native_window();
                }
            });
        let _ = REPAINT_WAKE.set(sender);
    }
}

/// Wake the GUI after a shell callback queues an action without activating it.
pub fn request_repaint() {
    if REPAINT_WAKE.get().is_some_and(|sender| sender.try_send(()).is_ok()) {
        return;
    }
    if let Some(ctx) = REPAINT.get() {
        ctx.request_repaint();
    }
}

pub fn fit_size(width: u32, height: u32, max_width: u32, max_height: u32) -> (u32, u32) {
    let scale = (max_width.max(1) as f64 / width.max(1) as f64)
        .min(max_height.max(1) as f64 / height.max(1) as f64);
    (
        (width as f64 * scale).round().max(1.0) as u32,
        (height as f64 * scale).round().max(1.0) as u32,
    )
}

pub fn configure(hwnd: isize, enabled: bool) -> Result<(), String> {
    let changed = PREVIEW
        .lock()
        .map_err(|_| "thumbnail state unavailable")?
        .enabled
        != enabled;
    if !changed {
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::{
            Foundation::HWND,
            Graphics::Dwm::{
                DWMWA_FORCE_ICONIC_REPRESENTATION, DWMWA_HAS_ICONIC_BITMAP,
                DwmInvalidateIconicBitmaps, DwmSetWindowAttribute,
            },
        };
        use windows::core::BOOL;
        let value = BOOL::from(enabled);
        // Both attributes are required; SetThumbnailClip alone is not an
        // explicit iconic representation and can leave the whole UI visible.
        for attribute in [DWMWA_HAS_ICONIC_BITMAP, DWMWA_FORCE_ICONIC_REPRESENTATION] {
            let result = DwmSetWindowAttribute(
                HWND(hwnd as *mut _),
                attribute,
                (&value as *const BOOL).cast(),
                std::mem::size_of::<BOOL>() as u32,
            )
            .map_err(|error| format!("configure video-only DWM thumbnail: {error}"));
            if let Err(error) = result {
                if let Ok(mut state) = PREVIEW.lock() {
                    state.configuration_error = Some(error.clone());
                }
                return Err(error);
            }
        }
        let _ = DwmInvalidateIconicBitmaps(HWND(hwnd as *mut _));
    }
    #[cfg(not(target_os = "windows"))]
    let _ = hwnd;
    let mut state = PREVIEW.lock().map_err(|_| "thumbnail state unavailable")?;
    state.enabled = enabled;
    state.configuration_error = None;
    state.frame = None;
    state.captured = None;
    state.frame_dirty = false;
    #[cfg(target_os = "windows")]
    {
        FULL_REQUESTED.store(false,std::sync::atomic::Ordering::Release);
        if let Ok(mut full)=FULL.lock() {*full=FullPreview::default();}
    }
    drop(state);
    #[cfg(target_os="windows")]
    if enabled {
        // One seed also covers shell/desktop minimize routes that bypass
        // WM_SYSCOMMAND. Later explicit minimization refreshes the last UI.
        FULL_REQUESTED.store(true,std::sync::atomic::Ordering::Release);
        request_repaint();
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CapturePlan {
    Idle,
    Capture,
    RetryAfter(Duration),
}

fn capture_plan(
    enabled: bool,
    has_frame: bool,
    frame_dirty: bool,
    captured_age: Option<Duration>,
) -> CapturePlan {
    const FRAME_INTERVAL: Duration = Duration::from_millis(33);
    if !enabled {
        return CapturePlan::Idle;
    }
    if !has_frame {
        return CapturePlan::Capture;
    }
    if !frame_dirty {
        return CapturePlan::Idle;
    }
    match captured_age {
        Some(age) if age < FRAME_INTERVAL => CapturePlan::RetryAfter(FRAME_INTERVAL - age),
        _ => CapturePlan::Capture,
    }
}

pub fn reset_shell() {
    if let Ok(mut state) = PREVIEW.lock() {
        state.enabled = false;
    }
}

/// Called only with the current render context and the MPV FBO bound.
#[cfg(target_os = "windows")]
pub unsafe fn capture(
    gl: &eframe::glow::Context,
    hwnd: isize,
    source: eframe::glow::Framebuffer,
    width: i32,
    height: i32,
    media: &str,
    frame_updated: bool,
) {
    use eframe::glow::{self, HasContext};
    let plan = if let Ok(mut state) = PREVIEW.lock() {
        if !state.enabled || width <= 0 || height <= 0 {
            return;
        }
        if state.media != media {
            state.media = media.to_owned();
            state.frame = None;
            state.captured = None;
        }
        state.frame_dirty |= frame_updated;
        capture_plan(
            state.enabled,
            state.frame.is_some(),
            state.frame_dirty,
            state.captured.map(|at| at.elapsed()),
        )
    } else {
        CapturePlan::Idle
    };
    match plan {
        CapturePlan::Idle => return,
        CapturePlan::RetryAfter(delay) => {
            // DWM can ask for the invalidated thumbnail immediately. If the
            // 30 Hz limiter simply returns here, no later paint is guaranteed
            // and the taskbar freezes on that frame. Schedule the exact next
            // eligible capture to keep the preview live while it is visible.
            if let Some(ctx) = REPAINT.get() {
                ctx.request_repaint_after(delay);
            }
            return;
        }
        CapturePlan::Capture => {}
    }
    let (w, h) = fit_size(width as u32, height as u32, 640, 360);
    unsafe {
        // Preserve split FBO bindings and pixel packing. Never read the egui
        // framebuffer, which would include controls/OSD and leak through panels.
        let scissor = gl.is_enabled(glow::SCISSOR_TEST);
        gl.disable(glow::SCISSOR_TEST);
        let read = gl.get_parameter_i32(glow::READ_FRAMEBUFFER_BINDING) as u32;
        let draw = gl.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING) as u32;
        let pack = gl.get_parameter_i32(glow::PIXEL_PACK_BUFFER_BINDING) as u32;
        let alignment = gl.get_parameter_i32(glow::PACK_ALIGNMENT);
        let row_length = gl.get_parameter_i32(glow::PACK_ROW_LENGTH);
        let skip_rows = gl.get_parameter_i32(glow::PACK_SKIP_ROWS);
        let skip_pixels = gl.get_parameter_i32(glow::PACK_SKIP_PIXELS);
        let Ok(fbo) = gl.create_framebuffer() else {
            if scissor { gl.enable(glow::SCISSOR_TEST); }
            return;
        };
        let Ok(texture) = gl.create_renderbuffer() else {
            gl.delete_framebuffer(fbo);
            if scissor { gl.enable(glow::SCISSOR_TEST); }
            return;
        };
        let rb = gl.get_parameter_i32(glow::RENDERBUFFER_BINDING) as u32;
        gl.bind_renderbuffer(glow::RENDERBUFFER, Some(texture));
        gl.renderbuffer_storage(glow::RENDERBUFFER, glow::RGBA8, w as i32, h as i32);
        gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(fbo));
        gl.framebuffer_renderbuffer(
            glow::DRAW_FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::RENDERBUFFER,
            Some(texture),
        );
        let complete =
            gl.check_framebuffer_status(glow::DRAW_FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        if complete {
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(source));
            gl.blit_framebuffer(
                0,
                0,
                width,
                height,
                0,
                0,
                w as i32,
                h as i32,
                glow::COLOR_BUFFER_BIT,
                glow::LINEAR,
            );
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(fbo));
            gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
            gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            gl.pixel_store_i32(glow::PACK_ROW_LENGTH, 0);
            gl.pixel_store_i32(glow::PACK_SKIP_ROWS, 0);
            gl.pixel_store_i32(glow::PACK_SKIP_PIXELS, 0);
            gl.read_pixels(
                0,
                0,
                w as i32,
                h as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut rgba)),
            );
        }
        gl.bind_framebuffer(
            glow::READ_FRAMEBUFFER,
            std::num::NonZeroU32::new(read).map(glow::NativeFramebuffer),
        );
        gl.bind_framebuffer(
            glow::DRAW_FRAMEBUFFER,
            std::num::NonZeroU32::new(draw).map(glow::NativeFramebuffer),
        );
        gl.bind_buffer(
            glow::PIXEL_PACK_BUFFER,
            std::num::NonZeroU32::new(pack).map(glow::NativeBuffer),
        );
        for (key, value) in [
            (glow::PACK_ALIGNMENT, alignment),
            (glow::PACK_ROW_LENGTH, row_length),
            (glow::PACK_SKIP_ROWS, skip_rows),
            (glow::PACK_SKIP_PIXELS, skip_pixels),
        ] {
            gl.pixel_store_i32(key, value);
        }
        gl.bind_renderbuffer(
            glow::RENDERBUFFER,
            std::num::NonZeroU32::new(rb).map(glow::NativeRenderbuffer),
        );
        gl.delete_renderbuffer(texture);
        gl.delete_framebuffer(fbo);
        if scissor { gl.enable(glow::SCISSOR_TEST); }
        if !complete {
            return;
        }
        // MPV render(false) matches the top-down egui texture used on screen.
        for pixel in rgba.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        if let Ok(mut state) = PREVIEW.lock() {
            state.frame = image::RgbaImage::from_raw(w, h, rgba);
            state.captured = Some(Instant::now());
            state.frame_dirty = false;
            state.captured_frames += 1;
        }
        let _ = windows::Win32::Graphics::Dwm::DwmInvalidateIconicBitmaps(
            windows::Win32::Foundation::HWND(hwnd as *mut _),
        );
    }
}

#[cfg(target_os = "windows")]
pub fn bitmap(
    image: &image::RgbaImage,
) -> windows::core::Result<windows::Win32::Graphics::Gdi::HBITMAP> {
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDIBSection, DIB_RGB_COLORS,
    };
    let mut bits = std::ptr::null_mut();
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: image.width() as i32,
            biHeight: -(image.height() as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let bitmap = unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0)? };
    let bgra = unsafe { std::slice::from_raw_parts_mut(bits.cast::<u8>(), image.as_raw().len()) };
    for (src, dst) in image.as_raw().chunks_exact(4).zip(bgra.chunks_exact_mut(4)) {
        // Windows icons require premultiplied BGRA, not 1-bit XOR masks.
        let a = src[3] as u16;
        dst.copy_from_slice(&[
            (src[2] as u16 * a / 255) as u8,
            (src[1] as u16 * a / 255) as u8,
            (src[0] as u16 * a / 255) as u8,
            src[3],
        ]);
    }
    Ok(bitmap)
}

#[cfg(target_os = "windows")]
pub fn handle_request(hwnd: isize, message: u32, lparam: isize) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::WM_DWMSENDICONICLIVEPREVIEWBITMAP;

    // With forced iconic representation Windows also asks us for this surface.
    // Ignoring it leaves Task View black. Never use the small video bitmap here.
    if message == WM_DWMSENDICONICLIVEPREVIEWBITMAP {
        return handle_full_request(hwnd);
    }
    if !is_static_thumbnail_request(message) {
        return false;
    }
    let frame = {
        let Ok(mut state) = PREVIEW.lock() else {
            return false;
        };
        if !state.enabled {
            return false;
        }
        state.requests += 1;
        state.frame.clone()
    };
    let Some(frame) = frame else {
        request_repaint();
        return true;
    };
    let (max_w, max_h) = (
        ((lparam as u32 >> 16) & 0xffff).max(1),
        (lparam as u32 & 0xffff).max(1),
    );
    publish_thumbnail(hwnd, &frame, (max_w, max_h));
    true
}

#[cfg(target_os = "windows")]
fn publish_thumbnail(hwnd: isize, frame: &image::RgbaImage, maximum: (u32, u32)) {
    use windows::Win32::{
        Foundation::HWND,
        Graphics::{Dwm::DwmSetIconicThumbnail, Gdi::DeleteObject},
    };
    let (w, h) = fit_size(frame.width(), frame.height(), maximum.0, maximum.1);
    let resized = image::imageops::resize(frame, w, h, image::imageops::FilterType::Triangle);
    let result = bitmap(&resized).and_then(|bitmap| unsafe {
        let result = DwmSetIconicThumbnail(HWND(hwnd as *mut _), bitmap, 0);
        let _ = DeleteObject(bitmap.into());
        result
    });
    if let Ok(mut state) = PREVIEW.lock() {
        match result {
            Ok(()) => {
                state.delivered += 1;
                state.last_error = None;
            }
            Err(error) => state.last_error = Some(error.to_string()),
        }
    }
}

pub fn diagnostics() -> serde_json::Value {
    let Ok(state) = PREVIEW.lock() else {
        return serde_json::json!({"error":"thumbnail state unavailable"});
    };
    // These two attributes are documented for Set, not Get. Report the
    // successfully accepted configuration, not fabricated read-back values.
    #[cfg(target_os = "windows")]
    let branding = crate::platform::taskbar_branding::diagnostics();
    #[cfg(not(target_os = "windows"))]
    let branding = serde_json::Value::Null;
    #[cfg(target_os = "windows")]
    let full = FULL.lock().map(|state|serde_json::json!({"requests":state.requests,"captures":state.captures,
        "delivered":state.delivered,"non_uniform":state.non_uniform,"frame_size":state.frame.as_ref().map(|frame|[frame.width(),frame.height()]),
        "frame_age_ms":state.captured.map(|at|at.elapsed().as_millis() as u64),"error":state.error})).unwrap_or_default();
    #[cfg(not(target_os = "windows"))]
    let full = serde_json::Value::Null;
    serde_json::json!({"video_only":state.enabled,"branding":branding,
        "full_preview":full,
        "dwm_configuration":{"supported":cfg!(target_os="windows"),
            "force_iconic":state.enabled,"has_iconic_bitmap":state.enabled,
            "source":"successful DwmSetWindowAttribute calls","error":state.configuration_error},
        "toolbar_icon_size":crate::platform::windows::thumbnail_toolbar_metrics(crate::platform::windows::get_registered_hwnd()).0,
        "frame_size":state.frame.as_ref().map(|f| [f.width(),f.height()]),
        "frame_age_ms":state.captured.map(|at| at.elapsed().as_millis() as u64),
        "captured_frames":state.captured_frames,"frame_dirty":state.frame_dirty,
        "dwm_requests":state.requests,"dwm_delivered":state.delivered,"last_error":state.last_error,
        "shell_commands":crate::platform::windows::shell_command_diagnostics()})
}

pub fn frame_png() -> Option<Vec<u8>> {
    let frame = PREVIEW.lock().ok()?.frame.clone()?;
    let mut out = std::io::Cursor::new(Vec::new());
    frame.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    #[test]
    fn full_preview_preserves_composited_pixels_and_orientation() {
        assert!(super::full_rgba(&eframe::egui::ColorImage::new([0,0],Vec::new())).is_none());
        let pixels=[eframe::egui::Color32::RED,eframe::egui::Color32::GREEN,eframe::egui::Color32::BLUE,eframe::egui::Color32::WHITE];
        let image=eframe::egui::ColorImage::new([2,2],pixels.to_vec());
        let full=super::full_rgba(&image).unwrap();
        assert_eq!(full.dimensions(),(2,2));
        for (pixel,expected) in full.pixels().zip(pixels) {assert_eq!(pixel.0,expected.to_array());}
        let source=include_str!("taskbar_preview.rs");
        assert!(source.contains("ctx.request_screenshot(move |image|"));
        assert!(source.contains("return handle_full_request(hwnd)"));
    }
    #[test]
    fn video_only_bitmap_is_never_used_for_full_size_peek() {
        assert!(super::is_static_thumbnail_request(0x0323));
        assert!(!super::is_static_thumbnail_request(0x0326));
    }

    #[test]
    fn thumbnail_fits_without_aspect_distortion() {
        assert_eq!(super::fit_size(1920, 1080, 320, 240), (320, 180));
        assert_eq!(super::fit_size(1080, 1920, 320, 180), (101, 180));
        assert_eq!(super::fit_size(1920, 800, 320, 180), (320, 133));
    }

    #[test]
    fn active_preview_schedules_the_next_eligible_frame_instead_of_stalling() {
        use std::time::Duration;
        assert_eq!(
            super::capture_plan(
                true,
                true,
                true,
                Some(Duration::from_millis(20)),
            ),
            super::CapturePlan::RetryAfter(Duration::from_millis(13))
        );
        assert_eq!(
            super::capture_plan(
                true,
                true,
                true,
                Some(Duration::from_millis(33)),
            ),
            super::CapturePlan::Capture
        );
        assert_eq!(
            super::capture_plan(
                true,
                true,
                false,
                Some(Duration::from_secs(1)),
            ),
            super::CapturePlan::Idle
        );
    }

    #[test]
    fn new_video_frames_never_expire_but_paused_unchanged_frames_do_not_loop() {
        use std::time::Duration;
        for age in [Duration::from_secs(2), Duration::from_secs(60)] {
            assert_eq!(super::capture_plan(true, true, true, Some(age)), super::CapturePlan::Capture);
            assert_eq!(super::capture_plan(true, true, false, Some(age)), super::CapturePlan::Idle);
        }
        assert_eq!(super::capture_plan(false, false, true, None), super::CapturePlan::Idle);
    }
}
