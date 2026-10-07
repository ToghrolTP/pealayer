//! DWM consumes an MPV-only bitmap, not a screenshot/crop of egui.
//! Readback is demand-driven: one seed frame, then up to 30 Hz only while
//! Windows is actively requesting previews. No decoding, disk I/O or GL runs
//! in the window procedure, and the real Pealayer window remains untouched.

use std::sync::{Mutex, OnceLock, mpsc::SyncSender};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Preview {
    enabled: bool,
    media: String,
    frame: Option<image::RgbaImage>,
    captured: Option<Instant>,
    requested: Option<Instant>,
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
    state.requested = None;
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
    let plan = if let Ok(mut state) = PREVIEW.lock() {
        if !state.enabled || width <= 0 || height <= 0 {
            return;
        }
        if state.media != media {
            state.media = media.to_owned();
            state.frame = None;
            state.captured = None;
        }
        capture_plan(
            state.enabled,
            state.frame.is_some(),
            state.requested.map(|at| at.elapsed()),
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
        if let Ok(mut state) = PREVIEW.lock() {
            state.frame = image::RgbaImage::from_raw(w, h, rgba);
            state.captured = Some(Instant::now());
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

    // Video-only is correct for the small taskbar thumbnail, but a Peek/live
    // preview is projected over the real desktop window. Leave that full-size
    // preview to DWM so Pealayer keeps its complete UI while being hovered.
    if message == WM_DWMSENDICONICLIVEPREVIEWBITMAP {
        return false;
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
        state.requested = Some(Instant::now());
        state.frame.clone()
    };
    if let Some(ctx) = REPAINT.get() {
        ctx.request_repaint();
    }
    let Some(frame) = frame else {
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
    serde_json::json!({"video_only":state.enabled,
        "dwm_configuration":{"supported":cfg!(target_os="windows"),
            "force_iconic":state.enabled,"has_iconic_bitmap":state.enabled,
            "source":"successful DwmSetWindowAttribute calls","error":state.configuration_error},
        "toolbar_icon_size":crate::platform::windows::thumbnail_toolbar_metrics(crate::platform::windows::get_registered_hwnd()).0,
        "frame_size":state.frame.as_ref().map(|f| [f.width(),f.height()]),
        "frame_age_ms":state.captured.map(|at| at.elapsed().as_millis() as u64),
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
                Some(Duration::from_millis(5)),
                Some(Duration::from_millis(20)),
            ),
            super::CapturePlan::RetryAfter(Duration::from_millis(13))
        );
        assert_eq!(
            super::capture_plan(
                true,
                true,
                Some(Duration::from_millis(40)),
                Some(Duration::from_millis(33)),
            ),
            super::CapturePlan::Capture
        );
        assert_eq!(
            super::capture_plan(
                true,
                true,
                Some(Duration::from_secs(2)),
                Some(Duration::from_secs(1)),
            ),
            super::CapturePlan::Idle
        );
    }
}
