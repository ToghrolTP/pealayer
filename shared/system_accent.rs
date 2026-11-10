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
