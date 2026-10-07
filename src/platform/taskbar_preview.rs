//! Cached MPV-only bitmap fallback for the Windows taskbar preview strategy.
//! A visible embedded panel uses `SetThumbnailClip`, and a detached native
//! video HWND uses its own DWM surface. Readback is retained for the remaining
//! hidden-panel case: one seed frame, then up to 30 Hz only while Windows is
//! actively requesting previews. No decoding, disk I/O or GL runs in the
//! window procedure.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock, mpsc::SyncSender};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Preview {
    enabled: bool,
    mode: PreviewMode,
    media: String,
    frame: Option<image::RgbaImage>,
    history: VecDeque<TimedFrame>,
    freeze_at: Option<f64>,
    frozen_frame: Option<image::RgbaImage>,
    captured: Option<Instant>,
    seed_until: Option<Instant>,
    requested: Option<Instant>,
    requests: u64,
    delivered: u64,
    last_error: Option<String>,
    configuration_error: Option<String>,
}

#[derive(Clone)]
struct TimedFrame {
    media_time: f64,
    frame: image::RgbaImage,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PreviewMode {
    #[default]
    Disabled,
    WindowClip,
    IconicBitmap,
    ExternalWindow,
}

static PREVIEW: Mutex<Preview> = Mutex::new(Preview {
    enabled: false,
    mode: PreviewMode::Disabled,
    media: String::new(),
    frame: None,
    history: VecDeque::new(),
    freeze_at: None,
    frozen_frame: None,
    captured: None,
    seed_until: None,
    requested: None,
    requests: 0,
    delivered: 0,
    last_error: None,
    configuration_error: None,
});
static REPAINT: OnceLock<eframe::egui::Context> = OnceLock::new();
static REPAINT_WAKE: OnceLock<SyncSender<()>> = OnceLock::new();

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
fn is_iconic_bitmap_request(message: u32) -> bool {
    matches!(
        message,
        0x0323 // WM_DWMSENDICONICTHUMBNAIL
            | 0x0326 // WM_DWMSENDICONICLIVEPREVIEWBITMAP
    )
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
    if REPAINT_WAKE
        .get()
        .is_some_and(|sender| sender.try_send(()).is_ok())
    {
        return;
    }
    if let Some(ctx) = REPAINT.get() {
        ctx.request_repaint();
    }
}

pub fn request_fullscreen(fullscreen: bool) {
    if let Some(ctx) = REPAINT.get() {
        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Fullscreen(fullscreen));
    }
    request_repaint();
}

pub(crate) fn frame_rgba() -> Option<image::RgbaImage> {
    PREVIEW.lock().ok()?.frame.clone()
}

/// Freeze the frame Explorer is already displaying when playback pauses.
///
/// Selecting another history entry here made the thumbnail visibly jump at
/// the exact moment the Play/Pause button changed state. The cached frame is
/// already the most recent frame published to DWM, so retain those exact
/// pixels until playback resumes. History is only a fallback when Pause lands
/// before the first thumbnail frame has been published.
pub fn set_playback_state(paused: bool, media_time: Option<f64>) {
    let Ok(mut state) = PREVIEW.lock() else {
        return;
    };
    if paused {
        let target = media_time.filter(|time| time.is_finite());
        if state.freeze_at.is_none() {
            state.frozen_frame = state
                .frame
                .clone()
                .or_else(|| target.and_then(|target| frozen_frame(&state.history, target)));
        }
        state.freeze_at = target;
        // Do not run the media-load seed loop after Pause. Repeatedly
        // invalidating DWM with the same frozen frame caused a second visual
        // jump and needless readback work.
        state.seed_until = None;
    } else {
        state.freeze_at = None;
        state.frozen_frame = None;
    }
    drop(state);
    request_repaint();
}

fn frozen_frame(history: &VecDeque<TimedFrame>, target: f64) -> Option<image::RgbaImage> {
    const EPSILON: f64 = 0.000_5;
    history
        .iter()
        .filter(|entry| entry.media_time <= target + EPSILON)
        .max_by(|a, b| a.media_time.total_cmp(&b.media_time))
        .or_else(|| {
            history.iter().min_by(|a, b| {
                (a.media_time - target)
                    .abs()
                    .total_cmp(&(b.media_time - target).abs())
            })
        })
        .map(|entry| entry.frame.clone())
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
    state.mode = if enabled {
        PreviewMode::IconicBitmap
    } else if state.mode == PreviewMode::IconicBitmap {
        PreviewMode::Disabled
    } else {
        state.mode
    };
    state.configuration_error = None;
    state.requested = None;
    Ok(())
}

pub fn set_mode(mode: PreviewMode) {
    if let Ok(mut state) = PREVIEW.lock() {
        state.mode = mode;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CapturePlan {
    Idle,
    Capture,
    RetryAfter(Duration),
}

/// Reserve the next video-only thumbnail capture. Both OpenGL and D3D11 use
/// this gate so DWM gets identical media-change invalidation, demand tracking,
/// and 30 Hz throttling regardless of the selected Windows video renderer.
pub(crate) fn begin_capture(media: &str, width: u32, height: u32) -> Option<(u32, u32)> {
    let plan = if let Ok(mut state) = PREVIEW.lock() {
        if width == 0 || height == 0 {
            return None;
        }
        if state.media != media {
            state.media = media.to_owned();
            state.frame = None;
            state.history.clear();
            state.freeze_at = None;
            state.frozen_frame = None;
            state.captured = None;
            // The first render after opening media can precede the decoded
            // video frame and therefore be black. Briefly refresh the cache
            // after a media change so Explorer receives decoded pixels even
            // before its first hover request.
            state.seed_until = Some(Instant::now() + Duration::from_secs(2));
        }
        let seeding = state.seed_until.is_some_and(|until| until > Instant::now());
        let requested = state
            .requested
            .is_some_and(|at| at.elapsed() < Duration::from_secs(1));
        capture_plan(
            state.enabled || state.frame.is_none() || seeding || requested,
            state.frame.is_some(),
            seeding,
            state.requested.map(|at| at.elapsed()),
            state.captured.map(|at| at.elapsed()),
        )
    } else {
        CapturePlan::Idle
    };
    match plan {
        CapturePlan::Idle => None,
        CapturePlan::RetryAfter(delay) => {
            // DWM can ask for the invalidated thumbnail immediately. Without
            // a future paint, a rate-limited renderer would freeze forever.
            if let Some(ctx) = REPAINT.get() {
                ctx.request_repaint_after(delay);
            }
            None
        }
        CapturePlan::Capture => Some(fit_size(width, height, 640, 360)),
    }
}

/// Publish a top-down RGBA video frame to the common DWM thumbnail state.
pub(crate) fn submit_capture(hwnd: isize, frame: image::RgbaImage) {
    submit_capture_at(hwnd, frame, None);
}

pub(crate) fn submit_capture_at(hwnd: isize, frame: image::RgbaImage, media_time: Option<f64>) {
    let mut seed_delay = None;
    let mut published_frame_changed = false;
    if let Ok(mut state) = PREVIEW.lock() {
        if let Some(media_time) = media_time.filter(|time| time.is_finite()) {
            state.history.push_back(TimedFrame {
                media_time,
                frame: frame.clone(),
            });
            while state.history.len() > 12 {
                state.history.pop_front();
            }
        }
        if state.freeze_at.is_some() {
            if state.frozen_frame.is_none() {
                state.frozen_frame = state
                    .freeze_at
                    .and_then(|target| frozen_frame(&state.history, target))
                    .or_else(|| Some(frame.clone()));
                published_frame_changed = true;
            }
            state.frame = state.frozen_frame.clone();
        } else {
            state.frame = Some(frame);
            published_frame_changed = true;
        }
        if published_frame_changed {
            state.captured = Some(Instant::now());
        }
        if let Some(until) = state.seed_until {
            let remaining = until.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                state.seed_until = None;
            } else {
                seed_delay = Some(remaining.min(Duration::from_millis(120)));
            }
        }
    }
    if let (Some(ctx), Some(delay)) = (REPAINT.get(), seed_delay) {
        ctx.request_repaint_after(delay);
    }
    #[cfg(target_os = "windows")]
    if published_frame_changed {
        unsafe {
            let _ = windows::Win32::Graphics::Dwm::DwmInvalidateIconicBitmaps(
                windows::Win32::Foundation::HWND(hwnd as *mut _),
            );
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = hwnd;
}

fn capture_plan(
    enabled: bool,
    has_frame: bool,
    seeding: bool,
    requested_age: Option<Duration>,
    captured_age: Option<Duration>,
) -> CapturePlan {
    const ACTIVE_WINDOW: Duration = Duration::from_secs(1);
    const FRAME_INTERVAL: Duration = Duration::from_millis(33);
    if !enabled {
        return CapturePlan::Idle;
    }
    if !has_frame {
        return CapturePlan::Capture;
    }
    if seeding {
        const SEED_INTERVAL: Duration = Duration::from_millis(120);
        return match captured_age {
            Some(age) if age < SEED_INTERVAL => CapturePlan::RetryAfter(SEED_INTERVAL - age),
            _ => CapturePlan::Capture,
        };
    }
    if !requested_age.is_some_and(|age| age < ACTIVE_WINDOW) {
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
        state.mode = PreviewMode::Disabled;
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
) {
    use eframe::glow::{self, HasContext};
    let Some((w, h)) = begin_capture(media, width.max(0) as u32, height.max(0) as u32) else {
        return;
    };
    unsafe {
        // Preserve split FBO bindings and pixel packing. Never read the egui
        // framebuffer, which would include controls/OSD and leak through panels.
        let read = gl.get_parameter_i32(glow::READ_FRAMEBUFFER_BINDING) as u32;
        let draw = gl.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING) as u32;
        let pack = gl.get_parameter_i32(glow::PIXEL_PACK_BUFFER_BINDING) as u32;
        let alignment = gl.get_parameter_i32(glow::PACK_ALIGNMENT);
        let row_length = gl.get_parameter_i32(glow::PACK_ROW_LENGTH);
        let skip_rows = gl.get_parameter_i32(glow::PACK_SKIP_ROWS);
        let skip_pixels = gl.get_parameter_i32(glow::PACK_SKIP_PIXELS);
        let Ok(fbo) = gl.create_framebuffer() else {
            return;
        };
        let Ok(texture) = gl.create_renderbuffer() else {
            gl.delete_framebuffer(fbo);
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
        if !complete {
            return;
        }
        // MPV render(false) matches the top-down egui texture used on screen.
        for pixel in rgba.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        if let Some(frame) = image::RgbaImage::from_raw(w, h, rgba) {
            submit_capture(hwnd, frame);
        }
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
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{IsIconic, WM_DWMSENDICONICLIVEPREVIEWBITMAP},
    };

    if !is_iconic_bitmap_request(message) {
        return false;
    }
    let enabled = {
        let Ok(mut state) = PREVIEW.lock() else {
            return false;
        };
        if !state.enabled {
            return false;
        }
        state.requests += 1;
        state.requested = Some(Instant::now());
        true
    };
    if !enabled {
        return false;
    }
    // A custom live-preview bitmap is the full-size Aero Peek surface, not
    // the small taskbar thumbnail. Publishing the video-only cache while the
    // owner is visible makes Windows cover the real Pealayer interface with a
    // low-resolution bitmap. Only a minimized owner needs this fallback.
    if message == WM_DWMSENDICONICLIVEPREVIEWBITMAP
        && !unsafe { IsIconic(HWND(hwnd as *mut _)).as_bool() }
    {
        return false;
    }
    #[cfg(all(target_os = "windows", feature = "d3d11-composition-experiment"))]
    crate::platform::d3d11_composition::capture_for_taskbar_request(hwnd);
    let frame = PREVIEW.lock().ok().and_then(|state| state.frame.clone());
    if let Some(ctx) = REPAINT.get() {
        ctx.request_repaint();
    }
    let Some(frame) = frame else {
        return true;
    };
    if message == WM_DWMSENDICONICLIVEPREVIEWBITMAP {
        publish_live_preview(hwnd, &frame);
    } else {
        let (max_w, max_h) = (
            ((lparam as u32 >> 16) & 0xffff).max(1),
            (lparam as u32 & 0xffff).max(1),
        );
        publish_thumbnail(hwnd, &frame, (max_w, max_h));
    }
    true
}

#[cfg(target_os = "windows")]
fn publish_live_preview(hwnd: isize, frame: &image::RgbaImage) {
    use windows::Win32::{
        Foundation::{HWND, POINT, RECT},
        Graphics::{Dwm::DwmSetIconicLivePreviewBitmap, Gdi::DeleteObject},
        UI::WindowsAndMessaging::GetClientRect,
    };
    let mut client = RECT::default();
    let result = unsafe { GetClientRect(HWND(hwnd as *mut _), &mut client) };
    if result.is_err() {
        return;
    }
    let maximum = (
        (client.right - client.left).max(1) as u32,
        (client.bottom - client.top).max(1) as u32,
    );
    let (width, height) = fit_size(frame.width(), frame.height(), maximum.0, maximum.1);
    let resized =
        image::imageops::resize(frame, width, height, image::imageops::FilterType::Triangle);
    let origin = POINT {
        x: ((maximum.0 - width) / 2) as i32,
        y: ((maximum.1 - height) / 2) as i32,
    };
    let result = bitmap(&resized).and_then(|bitmap| unsafe {
        let result = DwmSetIconicLivePreviewBitmap(HWND(hwnd as *mut _), bitmap, Some(&origin), 0);
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
    #[cfg(all(target_os = "windows", feature = "d3d11-composition-experiment"))]
    let renderer = crate::platform::d3d11_composition::diagnostics();
    #[cfg(not(all(target_os = "windows", feature = "d3d11-composition-experiment")))]
    let renderer = serde_json::Value::Null;
    serde_json::json!({"video_only":state.mode != PreviewMode::Disabled,
        "strategy":format!("{:?}", state.mode),
        "d3d11_composition":renderer,
        "dwm_configuration":{"supported":cfg!(target_os="windows"),
            "force_iconic":state.enabled,"has_iconic_bitmap":state.enabled,
            "source":"successful DwmSetWindowAttribute calls","error":state.configuration_error},
        "toolbar_icon_size":crate::platform::windows::thumbnail_toolbar_metrics(crate::platform::windows::get_registered_hwnd()).0,
        "frame_size":state.frame.as_ref().map(|f| [f.width(),f.height()]),
        "frame_age_ms":state.captured.map(|at| at.elapsed().as_millis() as u64),
        "timed_frame_history":state.history.len(),"paused_frame_time":state.freeze_at,
        "paused_frame_frozen":state.frozen_frame.is_some(),
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
    #[test]
    fn iconic_fallback_handles_thumbnail_and_live_preview_requests() {
        assert!(super::is_iconic_bitmap_request(0x0323));
        assert!(super::is_iconic_bitmap_request(0x0326));
        assert!(!super::is_iconic_bitmap_request(0x000f));
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
                false,
                Some(Duration::from_millis(5)),
                Some(Duration::from_millis(20)),
            ),
            super::CapturePlan::RetryAfter(Duration::from_millis(13))
        );
        assert_eq!(
            super::capture_plan(
                true,
                true,
                false,
                Some(Duration::from_millis(40)),
                Some(Duration::from_millis(33)),
            ),
            super::CapturePlan::Capture
        );
        assert_eq!(
            super::capture_plan(
                true,
                true,
                false,
                Some(Duration::from_secs(2)),
                Some(Duration::from_secs(1)),
            ),
            super::CapturePlan::Idle
        );
    }

    #[test]
    fn media_seed_refreshes_before_the_first_shell_hover() {
        use std::time::Duration;
        assert_eq!(
            super::capture_plan(true, true, true, None, Some(Duration::from_millis(40)),),
            super::CapturePlan::RetryAfter(Duration::from_millis(80))
        );
        assert_eq!(
            super::capture_plan(true, true, true, None, Some(Duration::from_millis(120)),),
            super::CapturePlan::Capture
        );
    }

    #[test]
    fn paused_preview_prefers_the_last_frame_not_after_the_playhead() {
        let frame = |value| image::RgbaImage::from_pixel(1, 1, image::Rgba([value, 0, 0, 255]));
        let history = std::collections::VecDeque::from([
            super::TimedFrame {
                media_time: 10.000,
                frame: frame(1),
            },
            super::TimedFrame {
                media_time: 10.041,
                frame: frame(2),
            },
            super::TimedFrame {
                media_time: 10.083,
                frame: frame(3),
            },
        ]);
        let selected = super::frozen_frame(&history, 10.060).expect("frame before pause");
        assert_eq!(selected.get_pixel(0, 0).0[0], 2);
    }

    #[test]
    fn pausing_keeps_the_pixels_already_published_to_explorer() {
        let frame = |value| image::RgbaImage::from_pixel(1, 1, image::Rgba([value, 0, 0, 255]));
        {
            let mut state = super::PREVIEW.lock().expect("preview state");
            state.frame = Some(frame(9));
            state.history = std::collections::VecDeque::from([super::TimedFrame {
                media_time: 10.0,
                frame: frame(1),
            }]);
            state.freeze_at = None;
            state.frozen_frame = None;
        }

        super::set_playback_state(true, Some(10.0));
        let state = super::PREVIEW.lock().expect("preview state");
        assert_eq!(state.frame.as_ref().unwrap().get_pixel(0, 0).0[0], 9);
        assert_eq!(
            state
                .frozen_frame
                .as_ref()
                .unwrap()
                .get_pixel(0, 0)
                .0[0],
            9
        );
        drop(state);
        super::set_playback_state(false, None);
    }
}
