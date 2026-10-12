//! Owned, DPI-sized 32-bit-alpha icons, independent of winit's legacy XOR mask.
use std::{collections::HashMap, sync::{Arc, LazyLock, Mutex}};
use windows::Win32::{Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::{CreateBitmap, DeleteObject},
    UI::WindowsAndMessaging::{CreateIconIndirect, DestroyIcon, HICON, ICONINFO,
        ICON_BIG, ICON_SMALL, WM_NCDESTROY, WM_SETICON, SendMessageW}};

const SUBCLASS_ID: usize = 0x5049_434f;
type Callback = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM, usize, usize) -> LRESULT;
#[link(name = "comctl32")]
unsafe extern "system" {
    fn SetWindowSubclass(hwnd: HWND, callback: Option<Callback>, id: usize, data: usize) -> windows::core::BOOL;
    fn DefSubclassProc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
}
#[link(name = "user32")]
unsafe extern "system" {
    fn GetDpiForWindow(hwnd: isize) -> u32;
    fn GetSystemMetricsForDpi(index: i32, dpi: u32) -> i32;
}

#[derive(Clone)]
struct Icons {
    image: Arc<image::RgbaImage>,
    dpi: u32,
    small: isize,
    big: isize,
}
static ICONS: LazyLock<Mutex<HashMap<isize, Icons>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn tray_icon(hwnd: isize) -> Option<isize> {
    ICONS.lock().ok()?.get(&hwnd).map(|icons| icons.small)
}

fn create(image: &image::RgbaImage, side: u32) -> Result<isize, String> {
    let image = image::imageops::resize(image, side, side, image::imageops::FilterType::Lanczos3);
    let color = super::taskbar_preview::bitmap(&image).map_err(|error| error.to_string())?;
    let mask_bytes = vec![0u8; (((side + 15) / 16 * 2) * side) as usize];
    unsafe {
        let mask = CreateBitmap(side as i32, side as i32, 1, 1, Some(mask_bytes.as_ptr().cast()));
        if mask.0.is_null() {
            let _ = DeleteObject(color.into());
            return Err(windows::core::Error::from_thread().to_string());
        }
        let result = CreateIconIndirect(&ICONINFO { fIcon: true.into(), hbmColor: color, hbmMask: mask, ..Default::default() });
        let _ = DeleteObject(color.into());
        let _ = DeleteObject(mask.into());
        result.map(|icon| icon.0 as isize).map_err(|error| error.to_string())
    }
}

fn destroy(icons: Icons) {
    unsafe {
        let _ = DestroyIcon(HICON(icons.small as *mut _));
        let _ = DestroyIcon(HICON(icons.big as *mut _));
    }
}

fn apply_image(hwnd: isize, image: Arc<image::RgbaImage>, dpi: u32) -> Result<(), String> {
    let small_side = unsafe { GetSystemMetricsForDpi(49, dpi) }.clamp(16, 128) as u32;
    let big_side = unsafe { GetSystemMetricsForDpi(11, dpi) }.clamp(32, 256) as u32;
    let small = create(&image, small_side)?;
    let big = match create(&image, big_side) {
        Ok(icon) => icon,
        Err(error) => { unsafe { let _ = DestroyIcon(HICON(small as *mut _)); } return Err(error); }
    };
    let next = Icons { image, dpi, small, big };
    if !unsafe { SetWindowSubclass(HWND(hwnd as *mut _), Some(icon_window_proc), SUBCLASS_ID, 0) }.as_bool() {
        destroy(next);
        return Err("Install native icon lifecycle hook failed".into());
    }
    let previous = match ICONS.lock() {
        Ok(mut cache) => cache.insert(hwnd, next),
        Err(_) => { destroy(next); return Err("Native icon cache is unavailable".into()); }
    };
    // Release the cache lock before synchronous window messages reenter hooks.
    // Only our previous handles are destroyed; returned winit/resource handles
    // are borrowed and remain their owner's responsibility.
    unsafe {
        SendMessageW(HWND(hwnd as *mut _), WM_SETICON, Some(WPARAM(ICON_SMALL as usize)), Some(LPARAM(small)));
        SendMessageW(HWND(hwnd as *mut _), WM_SETICON, Some(WPARAM(ICON_BIG as usize)), Some(LPARAM(big)));
    }
    if let Some(previous) = previous { destroy(previous); }
    Ok(())
}

pub fn apply(hwnd: isize, data: &eframe::egui::IconData) -> Result<(), String> {
    if hwnd == 0 || !unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindow(Some(HWND(hwnd as *mut _))) }.as_bool() {
        return Err("Native icon target is unavailable".into());
    }
    let image = image::RgbaImage::from_raw(data.width, data.height, data.rgba.clone())
        .filter(|image| image.width() != 0 && image.height() != 0).ok_or("Invalid RGBA icon")?;
    apply_image(hwnd, Arc::new(image), unsafe { GetDpiForWindow(hwnd) }.max(96))
}

unsafe extern "system" fn icon_window_proc(hwnd: HWND, message: u32, wparam: WPARAM,
    lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
    let raw = hwnd.0 as isize;
    if message == WM_SETICON && matches!(wparam.0, value if value == ICON_SMALL as usize || value == ICON_BIG as usize) {
        let icon = ICONS.lock().ok().and_then(|cache| cache.get(&raw).map(|icons|
            if wparam.0 == ICON_BIG as usize { icons.big } else { icons.small }));
        // eframe may reapply its legacy icon during DPI/viewport processing.
        // Preserve the owned alpha icon without taking ownership of that handle.
        return unsafe { DefSubclassProc(hwnd, message, wparam, icon.map(LPARAM).unwrap_or(lparam)) };
    }
    if message == 0x02e0 { // WM_DPICHANGED
        let cached = ICONS.lock().ok().and_then(|cache| cache.get(&raw).cloned());
        let dpi = (wparam.0 & 0xffff) as u32;
        if let Some(cached) = cached && dpi != 0 && dpi != cached.dpi {
            if let Err(error) = apply_image(raw, cached.image, dpi) { log::warn!("Refresh native window icon: {error}"); }
        }
    }
    let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
    if message == WM_NCDESTROY {
        let previous = ICONS.lock().ok().and_then(|mut cache| cache.remove(&raw));
        if let Some(previous) = previous { destroy(previous); }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_window_keeps_both_icon_slots_and_releases_only_owned_handles() {
        use windows::{core::w, Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow,
            GetIconInfo, WINDOW_EX_STYLE, WINDOW_STYLE, WM_GETICON}};
        struct Window(HWND);
        impl Drop for Window { fn drop(&mut self) { unsafe { let _ = DestroyWindow(self.0); } } }
        struct Icon(isize);
        impl Drop for Icon { fn drop(&mut self) { unsafe { let _ = DestroyIcon(HICON(self.0 as *mut _)); } } }
        fn exists(icon: isize) -> bool {
            let mut info = ICONINFO::default();
            unsafe {
                if GetIconInfo(HICON(icon as *mut _), &mut info).is_err() { return false; }
                let _ = DeleteObject(info.hbmColor.into()); let _ = DeleteObject(info.hbmMask.into());
            }
            true
        }
        // A private, never-visible fixture: no live window, focus, tray or hardware.
        let window = Window(unsafe { CreateWindowExW(WINDOW_EX_STYLE::default(), w!("STATIC"), w!("Icon fixture"),
            WINDOW_STYLE::default(), 0, 0, 32, 32, None, None, None, None).unwrap() });
        let raw = window.0.0 as isize;
        let image = Arc::new(image::RgbaImage::from_pixel(64, 64, image::Rgba([20, 130, 240, 200])));
        apply_image(raw, image.clone(), 96).unwrap();
        let first = ICONS.lock().unwrap().get(&raw).unwrap().clone();
        let get = |slot| unsafe { SendMessageW(window.0, WM_GETICON, Some(WPARAM(slot as usize)), Some(LPARAM(0))).0 };
        assert_eq!(get(ICON_SMALL), first.small);
        assert_eq!(get(ICON_BIG), first.big);
        assert_eq!(tray_icon(raw), Some(first.small));
        let borrowed = Icon(create(&image, 16).unwrap());
        unsafe { SendMessageW(window.0, WM_SETICON, Some(WPARAM(ICON_SMALL as usize)), Some(LPARAM(borrowed.0))); }
        assert_eq!(get(ICON_SMALL), first.small);
        assert!(exists(borrowed.0), "borrowed legacy icon must remain owned by its creator");
        apply_image(raw, image, 192).unwrap();
        let next = ICONS.lock().unwrap().get(&raw).unwrap().clone();
        assert_eq!(get(ICON_SMALL), next.small);
        assert_eq!(get(ICON_BIG), next.big);
        assert!(!exists(first.small)); assert!(!exists(first.big));
        drop(window);
        assert_eq!(tray_icon(raw), None);
        assert!(!exists(next.small)); assert!(!exists(next.big));
        assert!(exists(borrowed.0));
    }

    #[test]
    fn application_icons_use_true_color_alpha_bitmaps_at_each_native_size() {
        use windows::Win32::{Graphics::Gdi::{BITMAP, GetObjectW}, UI::WindowsAndMessaging::GetIconInfo};
        let image = image::RgbaImage::from_fn(64, 64, |x, y| image::Rgba([x as u8 * 3, y as u8 * 3, 123, (x+y) as u8 * 2]));
        for size in [16, 24, 32, 48, 64] {
            let icon = create(&image, size).unwrap();
            let mut info = ICONINFO::default();
            unsafe {
                GetIconInfo(HICON(icon as *mut _), &mut info).unwrap();
                let mut bitmap = BITMAP::default();
                assert_ne!(GetObjectW(info.hbmColor.into(), std::mem::size_of::<BITMAP>() as i32,
                    Some((&mut bitmap as *mut BITMAP).cast())), 0);
                let _ = DeleteObject(info.hbmColor.into()); let _ = DeleteObject(info.hbmMask.into());
                let _ = DestroyIcon(HICON(icon as *mut _));
                assert_eq!(bitmap.bmBitsPixel, 32, "palette/XOR icon at {size}");
                assert_eq!((bitmap.bmWidth, bitmap.bmHeight), (size as i32, size as i32));
            }
        }
    }
}
