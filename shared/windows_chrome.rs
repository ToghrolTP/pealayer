pub fn colorref([r, g, b]: [u8; 3]) -> u32 {
    u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16)
}

#[cfg(windows)]
pub fn apply(
    hwnd_raw: isize,
    dark: bool,
    dwm_theming: bool,
    mica_backdrop: bool,
    caption_color: u32,
    text_color: u32,
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
