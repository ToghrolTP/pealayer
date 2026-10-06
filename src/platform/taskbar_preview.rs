//! DWM consumes an MPV-only bitmap, not a screenshot/crop of egui.
//! Readback is bounded and demand-driven: one seed frame, then <= 5 Hz while
//! Windows is requesting previews. No decoding, disk I/O or GL runs in WndProc.

use std::sync::{Mutex, OnceLock};
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
});
static REPAINT: OnceLock<eframe::egui::Context> = OnceLock::new();

pub fn register_repaint(ctx: &eframe::egui::Context) {
    let _ = REPAINT.set(ctx.clone());
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
            DwmSetWindowAttribute(
                HWND(hwnd as *mut _),
                attribute,
                (&value as *const BOOL).cast(),
                std::mem::size_of::<BOOL>() as u32,
            )
            .map_err(|error| format!("configure video-only DWM thumbnail: {error}"))?;
        }
        let _ = DwmInvalidateIconicBitmaps(HWND(hwnd as *mut _));
    }
    #[cfg(not(target_os = "windows"))]
    let _ = hwnd;
    let mut state = PREVIEW.lock().map_err(|_| "thumbnail state unavailable")?;
    state.enabled = enabled;
    state.frame = None;
    state.captured = None;
    state.requested = None;
    Ok(())
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
    let needed = if let Ok(mut state) = PREVIEW.lock() {
        if !state.enabled || width <= 0 || height <= 0 {
            return;
        }
        if state.media != media {
            state.media = media.to_owned();
            state.frame = None;
            state.captured = None;
        }
        state.frame.is_none()
            || (state
                .requested
                .is_some_and(|at| at.elapsed() < Duration::from_secs(1))
                && state
                    .captured
                    .is_none_or(|at| at.elapsed() >= Duration::from_millis(200)))
    } else {
        false
    };
    if !needed {
        return;
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
    use windows::Win32::{
        Foundation::{HWND, RECT},
        Graphics::{
            Dwm::{DwmSetIconicLivePreviewBitmap, DwmSetIconicThumbnail},
            Gdi::DeleteObject,
        },
        UI::WindowsAndMessaging::{
            GetClientRect, WM_DWMSENDICONICLIVEPREVIEWBITMAP, WM_DWMSENDICONICTHUMBNAIL,
        },
    };
    if !matches!(
        message,
        WM_DWMSENDICONICTHUMBNAIL | WM_DWMSENDICONICLIVEPREVIEWBITMAP
    ) {
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
    let hwnd = HWND(hwnd as *mut _);
    let (max_w, max_h) = if message == WM_DWMSENDICONICTHUMBNAIL {
        (
            ((lparam as u32 >> 16) & 0xffff).max(1),
            (lparam as u32 & 0xffff).max(1),
        )
    } else {
        let mut rect = RECT::default();
        if unsafe { GetClientRect(hwnd, &mut rect) }.is_err() {
            return true;
        }
        (rect.right.max(1) as u32, rect.bottom.max(1) as u32)
    };
    let (w, h) = fit_size(frame.width(), frame.height(), max_w, max_h);
    let resized = image::imageops::resize(&frame, w, h, image::imageops::FilterType::Triangle);
    let result = bitmap(&resized).and_then(|bitmap| unsafe {
        let result = if message == WM_DWMSENDICONICTHUMBNAIL {
            DwmSetIconicThumbnail(hwnd, bitmap, 0)
        } else {
            DwmSetIconicLivePreviewBitmap(hwnd, bitmap, None, 0)
        };
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
    true
}

pub fn diagnostics() -> serde_json::Value {
    let Ok(state) = PREVIEW.lock() else {
        return serde_json::json!({"error":"thumbnail state unavailable"});
    };
    #[cfg(target_os = "windows")]
    let attributes = unsafe {
        use windows::Win32::{
            Foundation::HWND,
            Graphics::Dwm::{
                DWMWA_FORCE_ICONIC_REPRESENTATION, DWMWA_HAS_ICONIC_BITMAP, DwmGetWindowAttribute,
            },
        };
        use windows::core::BOOL;
        let hwnd = HWND(crate::platform::windows::get_registered_hwnd() as *mut _);
        let mut force = BOOL(0);
        let mut has = BOOL(0);
        let sample = DwmGetWindowAttribute(
            hwnd,
            DWMWA_FORCE_ICONIC_REPRESENTATION,
            (&mut force as *mut BOOL).cast(),
            std::mem::size_of::<BOOL>() as u32,
        )
        .and_then(|_| {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_HAS_ICONIC_BITMAP,
                (&mut has as *mut BOOL).cast(),
                std::mem::size_of::<BOOL>() as u32,
            )
        });
        serde_json::json!({"force_iconic":force.as_bool(),"has_iconic_bitmap":has.as_bool(),"error":sample.err().map(|e| e.to_string())})
    };
    #[cfg(not(target_os = "windows"))]
    let attributes = serde_json::Value::Null;
    serde_json::json!({"video_only":state.enabled,"dwm_attributes":attributes,
        "toolbar_icon_size":crate::platform::windows::thumbnail_toolbar_metrics(crate::platform::windows::get_registered_hwnd()).0,
        "frame_size":state.frame.as_ref().map(|f| [f.width(),f.height()]),
        "frame_age_ms":state.captured.map(|at| at.elapsed().as_millis() as u64),
        "dwm_requests":state.requests,"dwm_delivered":state.delivered,"last_error":state.last_error})
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
    fn thumbnail_fits_without_aspect_distortion() {
        assert_eq!(super::fit_size(1920, 1080, 320, 240), (320, 180));
        assert_eq!(super::fit_size(1080, 1920, 320, 180), (101, 180));
        assert_eq!(super::fit_size(1920, 800, 320, 180), (320, 133));
    }
}
