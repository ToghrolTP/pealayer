//! Experimental zero-copy Windows video presentation.
//!
//! mpv owns a D3D11 composition swapchain and decodes/presents directly into
//! it. DirectComposition then places that swapchain over the egui video
//! viewport. No OpenGL texture, CPU readback, or per-frame application copy is
//! involved in this on-screen path.

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use windows::Win32::Foundation::{HANDLE, HWND};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_STAGING, ID3D11Device, ID3D11Multithread, ID3D11Texture2D,
    D3D11_CREATE_DEVICE_SINGLETHREADED,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice2, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB, DXGI_FORMAT_R10G10B10A2_UNORM,
};
use windows::Win32::Graphics::Dxgi::{
    DXGI_SWAP_EFFECT_DISCARD, DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
    DXGI_SWAP_EFFECT_SEQUENTIAL,
};
use windows::Win32::Graphics::Dxgi::{IDXGISwapChain, IDXGISwapChain3};
use windows::core::{IUnknown, Interface};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static INITIALIZED: AtomicBool = AtomicBool::new(false);
static HAS_SWAPCHAIN: AtomicBool = AtomicBool::new(false);
static SWAP_EFFECT: Mutex<Option<&'static str>> = Mutex::new(None);

thread_local! {
    static STATE: RefCell<Option<CompositionState>> = const { RefCell::new(None) };
}

struct CompositionState {
    hwnd: isize,
    touched: bool,
    icon_font: HANDLE,
    device: IDCompositionDevice,
    _target: IDCompositionTarget,
    visual: IDCompositionVisual,
    swapchain: Option<IDXGISwapChain>,
    swapchain_ptr: isize,
    size: (u32, u32),
    offset: (i32, i32),
    visible: bool,
    thumbnail_staging: Option<ID3D11Texture2D>,
    thumbnail_staging_desc: Option<(u32, u32, DXGI_FORMAT)>,
    logged_thumbnail_capture: bool,
    last_back_buffer_index: Option<u32>,
    flip_chain_advanced: bool,
    media: String,
    last_present_count: Option<u32>,
    overlay_present_count: Option<u32>,
    overlay_revision: u64,
    last_swapchain_query: Option<Instant>,
    last_overlay_capture: Option<Instant>,
    overlay_frame: Option<image::RgbaImage>,
}

fn create_state(hwnd: isize) -> windows::core::Result<CompositionState> {
    // A null rendering device asks DirectComposition to create its own device.
    // The swapchain itself remains owned and presented by mpv.
    let device: IDCompositionDevice =
        unsafe { DCompositionCreateDevice2::<_, IDCompositionDevice>(None::<&IUnknown>)? };
    // Topmost makes the video visual sit above egui's opaque OpenGL surface.
    // The visual has no input surface, so pointer gestures still reach egui.
    let target = unsafe { device.CreateTargetForHwnd(HWND(hwnd as *mut _), true)? };
    let visual = unsafe { device.CreateVisual()? };
    unsafe {
        target.SetRoot(&visual)?;
        device.Commit()?;
    }
    Ok(CompositionState {
        hwnd,
        touched: false,
        icon_font: HANDLE::default(),
        device,
        _target: target,
        visual,
        swapchain: None,
        swapchain_ptr: 0,
        size: (0, 0),
        offset: (0, 0),
        visible: false,
        thumbnail_staging: None,
        thumbnail_staging_desc: None,
        logged_thumbnail_capture: false,
        last_back_buffer_index: None,
        flip_chain_advanced: false,
        media: String::new(),
        last_present_count: None,
        overlay_present_count: None,
        overlay_revision: 0,
        last_swapchain_query: None,
        last_overlay_capture: None,
        overlay_frame: None,
    })
}

fn unpack_swapchain_frame(
    mapped: D3D11_MAPPED_SUBRESOURCE,
    desc: D3D11_TEXTURE2D_DESC,
) -> Option<image::RgbaImage> {
    let row_pitch = mapped.RowPitch as usize;
    if desc.Width == 0 || desc.Height == 0 || desc.Width > 8192 || desc.Height > 8192 {
        return None;
    }
    let base = mapped.pData.cast::<u8>();
    if base.is_null() || row_pitch < desc.Width as usize * 4 {
        return None;
    }
    let mut rgba = vec![0_u8; desc.Width as usize * desc.Height as usize * 4];
    for y in 0..desc.Height as usize {
        let source =
            unsafe { std::slice::from_raw_parts(base.add(y * row_pitch), desc.Width as usize * 4) };
        let target = &mut rgba[y * desc.Width as usize * 4..(y + 1) * desc.Width as usize * 4];
        match desc.Format {
            DXGI_FORMAT_B8G8R8A8_UNORM | DXGI_FORMAT_B8G8R8A8_UNORM_SRGB => {
                for (src, dst) in source.chunks_exact(4).zip(target.chunks_exact_mut(4)) {
                    dst.copy_from_slice(&[src[2], src[1], src[0], 255]);
                }
            }
            DXGI_FORMAT_R8G8B8A8_UNORM | DXGI_FORMAT_R8G8B8A8_UNORM_SRGB => {
                target.copy_from_slice(source);
                for pixel in target.chunks_exact_mut(4) {
                    pixel[3] = 255;
                }
            }
            DXGI_FORMAT_R10G10B10A2_UNORM => {
                for (src, dst) in source.chunks_exact(4).zip(target.chunks_exact_mut(4)) {
                    let packed = u32::from_le_bytes(src.try_into().expect("four-byte pixel"));
                    let scale = |value: u32| ((value * 255 + 511) / 1023) as u8;
                    dst.copy_from_slice(&[
                        scale(packed & 0x3ff),
                        scale((packed >> 10) & 0x3ff),
                        scale((packed >> 20) & 0x3ff),
                        255,
                    ]);
                }
            }
            _ => return None,
        }
    }
    image::RgbaImage::from_raw(desc.Width, desc.Height, rgba)
}

#[derive(Clone, Copy)]
enum CapturePurpose {
    Taskbar,
    PopupOverlay { maximum: (u32, u32) },
}

fn capture_composition_frame(
    state: &mut CompositionState,
    hwnd: isize,
    media: &str,
    purpose: CapturePurpose,
) -> windows::core::Result<()> {
    let Some(swapchain) = state.swapchain.as_ref().cloned() else {
        return Ok(());
    };
    let present_count = unsafe { swapchain.GetLastPresentCount().ok() };
    let swapchain_desc = unsafe { swapchain.GetDesc()? };
    // On a flip-model chain buffer 0 is not synonymous with the frame DWM is
    // currently presenting. After Present, DXGI advances the current render
    // buffer; the immediately preceding buffer is the displayed frame. Reading
    // a fixed buffer explains the visible jump several frames backwards when
    // playback is paused on a three-buffer chain.
    let presented_index = if matches!(
        swapchain_desc.SwapEffect,
        DXGI_SWAP_EFFECT_FLIP_DISCARD | DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL
    ) && swapchain_desc.BufferCount > 1
    {
        swapchain
            .cast::<IDXGISwapChain3>()
            .map(|chain| {
                let current = unsafe { chain.GetCurrentBackBufferIndex() };
                if state
                    .last_back_buffer_index
                    .is_some_and(|previous| previous != current)
                {
                    state.flip_chain_advanced = true;
                }
                state.last_back_buffer_index = Some(current);
                if state.flip_chain_advanced {
                    (current + swapchain_desc.BufferCount - 1) % swapchain_desc.BufferCount
                } else {
                    // A paused media restore can render into the initial back
                    // buffer without advancing the chain. In that state the
                    // preceding slot has never been presented and is black.
                    current
                }
            })
            .unwrap_or(0)
    } else {
        0
    };
    let source: ID3D11Texture2D = unsafe { swapchain.GetBuffer(presented_index)? };
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { source.GetDesc(&mut desc) };
    let (target_width, target_height) = match purpose {
        CapturePurpose::Taskbar => {
            let dirty = present_count.is_none() || present_count != state.last_present_count;
            state.last_present_count = present_count;
            let Some(size) = super::taskbar_preview::capture_size(
                media, desc.Width as i32, desc.Height as i32, dirty,
            ) else { return Ok(()); };
            size
        }
        CapturePurpose::PopupOverlay { maximum } => {
            if state.overlay_frame.is_some() && present_count.is_some()
                && present_count == state.overlay_present_count
            {
                return Ok(());
            }
            if state
                .last_overlay_capture
                .is_some_and(|at| at.elapsed() < Duration::from_millis(33))
            {
                return Ok(());
            }
            state.last_overlay_capture = Some(Instant::now());
            crate::platform::taskbar_preview::fit_size(
                desc.Width,
                desc.Height,
                maximum.0.clamp(1, 1280),
                maximum.1.clamp(1, 720),
            )
        }
    };
    if desc.SampleDesc.Count != 1 || desc.Width > 8192 || desc.Height > 8192 {
        log::debug!("D3D11 taskbar capture skipped a multisampled swapchain");
        return Ok(());
    }
    let signature = (desc.Width, desc.Height, desc.Format);
    if state.thumbnail_staging_desc != Some(signature) {
        let device: ID3D11Device = unsafe { swapchain.GetDevice()? };
        let staging_desc = D3D11_TEXTURE2D_DESC {
            MipLevels: 1,
            ArraySize: 1,
            SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
            ..desc
        };
        let mut staging = None;
        unsafe { device.CreateTexture2D(&staging_desc, None, Some(&mut staging))? };
        state.thumbnail_staging = staging;
        state.thumbnail_staging_desc = Some(signature);
    }
    let Some(staging) = state.thumbnail_staging.as_ref() else {
        return Ok(());
    };
    let device: ID3D11Device = unsafe { swapchain.GetDevice()? };
    if unsafe { device.GetCreationFlags() } & D3D11_CREATE_DEVICE_SINGLETHREADED.0 != 0 {
        return Ok(());
    }
    let context = unsafe { device.GetImmediateContext()? };
    let lock = context.cast::<ID3D11Multithread>()?;
    // mpv presents on its own thread. Serialize these immediate-context
    // operations with its draws. The explicit lock is released before resize.
    unsafe {
        if !lock.GetMultithreadProtected().as_bool() {
            lock.SetMultithreadProtected(true);
        }
        lock.Enter();
    }
    let frame = (|| -> windows::core::Result<_> {
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            context.CopyResource(staging, &source);
            context.Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;
        }
        let frame = unpack_swapchain_frame(mapped, desc);
        unsafe { context.Unmap(staging, 0) };
        Ok(frame)
    })();
    unsafe { lock.Leave() };
    let frame = frame?;
    let Some(frame) = frame else {
        log::debug!(
            "D3D11 taskbar capture does not support swapchain format {:?}",
            desc.Format
        );
        return Ok(());
    };
    let frame = if frame.dimensions() == (target_width, target_height) {
        frame
    } else {
        image::imageops::resize(
            &frame,
            target_width,
            target_height,
            match purpose {
                CapturePurpose::Taskbar => image::imageops::FilterType::Triangle,
                CapturePurpose::PopupOverlay { .. } => image::imageops::FilterType::CatmullRom,
            },
        )
    };
    match purpose {
        CapturePurpose::Taskbar => {
            super::taskbar_preview::submit_capture(hwnd, media, frame);
            if !state.logged_thumbnail_capture {
                log::info!(
                    "D3D11 video-only taskbar thumbnail capture is active ({}x{})",
                    target_width,
                    target_height
                );
                state.logged_thumbnail_capture = true;
            }
        }
        CapturePurpose::PopupOverlay { .. } => {
            // Popup composition is a separate consumer from Explorer's
            // taskbar preview. Reusing the taskbar cache here meant a paused
            // popup displayed whichever older frame Explorer last requested,
            // then closing it revealed the actual DirectComposition frame.
            // Keep the current swapchain readback private to the popup so
            // opening or closing a menu cannot mutate the paused thumbnail.
            state.overlay_frame = Some(frame);
            state.overlay_present_count = present_count;
            state.overlay_revision = state.overlay_revision.wrapping_add(1);
        }
    }
    Ok(())
}

pub fn initialize(window: Option<&winit::window::Window>) -> bool {
    let Some(window) = window else { return false; };
    let Ok(handle) = window.window_handle() else { return false; };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else { return false; };
    match create_state(handle.hwnd.get()) {
        Ok(mut state) => {
            let font = egui_phosphor::Variant::Regular.font_bytes();
            let mut font_count = 0;
            state.icon_font = unsafe {
                windows::Win32::Graphics::Gdi::AddFontMemResourceEx(
                    font.as_ptr().cast(), font.len() as u32, None, &mut font_count,
                )
            };
            STATE.with(|slot| *slot.borrow_mut() = Some(state));
            INITIALIZED.store(true, Ordering::Release);
            true
        }
        Err(error) => {
            log::warn!("Could not create DirectComposition target; using OpenGL: {error}");
            false
        }
    }
}

pub fn shutdown() {
    STATE.with(|slot| {
        if let Some(state) = slot.borrow_mut().take() && !state.icon_font.is_invalid() {
            let _ = unsafe { windows::Win32::Graphics::Gdi::RemoveFontMemResourceEx(state.icon_font) };
        }
    });
    INITIALIZED.store(false, Ordering::Release);
    HAS_SWAPCHAIN.store(false, Ordering::Release);
    ACTIVE.store(false, Ordering::Release);
    if let Ok(mut effect) = SWAP_EFFECT.lock() { *effect = None; }
}

pub fn begin_frame() {
    STATE.with(|slot| {
        if let Some(state) = slot.borrow_mut().as_mut() { state.touched = false; }
    });
}

pub fn finish_frame(covered: bool) {
    STATE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(state) = slot.as_mut() else { return; };
        if state.visible && (!state.touched || covered) {
            let result = unsafe {
                state.visual.SetContent(None::<&IUnknown>).and_then(|_| state.device.Commit())
            };
            if result.is_ok() {
                state.visible = false;
                ACTIVE.store(false, Ordering::Release);
            }
        }
    });
}

/// Use the same GPU-owned player while the native visual is hidden to allow
/// egui overlays. The small Explorer thumbnail has its own capture state.
pub fn fallback_image(
    maximum: (u32, u32), previous_revision: Option<u64>,
) -> Option<(u64, eframe::egui::ColorImage)> {
    STATE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let state = slot.as_mut()?;
        let media = state.media.clone();
        if media.is_empty() { return None; }
        let hwnd = state.hwnd;
        if let Err(error) = capture_composition_frame(
            state, hwnd, &media, CapturePurpose::PopupOverlay { maximum },
        ) {
            log::debug!("D3D11 UI fallback capture failed: {error}");
        }
        if previous_revision == Some(state.overlay_revision) { return None; }
        let frame = state.overlay_frame.as_ref()?;
        let image = eframe::egui::ColorImage::from_rgba_unmultiplied(
            [frame.width() as usize, frame.height() as usize], frame.as_raw(),
        );
        Some((state.overlay_revision, image))
    })
}

pub fn capture_taskbar() {
    STATE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(state) = slot.as_mut() else { return; };
        if state.media.is_empty() { return; }
        let media = state.media.clone();
        let hwnd = state.hwnd;
        if let Err(error) = capture_composition_frame(state, hwnd, &media, CapturePurpose::Taskbar) {
            log::debug!("D3D11 taskbar capture failed: {error}");
        }
    });
}

/// Synchronize mpv's composition swapchain with the physical egui video rect.
/// Returns true only when the swapchain is attached and visible.
pub fn update(
    mpv: &libmpv2::Mpv,
    rect: eframe::egui::Rect,
    pixels_per_point: f32,
    media: &str,
    show_native: bool,
) -> bool {
    let width = (rect.width() * pixels_per_point).round().max(1.0) as u32;
    let height = (rect.height() * pixels_per_point).round().max(1.0) as u32;
    let x = (rect.left() * pixels_per_point).round() as i32;
    let y = (rect.top() * pixels_per_point).round() as i32;

    STATE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(state) = slot.as_mut() else { return false; };
        state.touched = true;
        if state.media != media {
            state.media = media.to_owned();
            state.overlay_frame = None;
            state.overlay_present_count = None;
            state.last_overlay_capture = None;
            state.last_present_count = None;
            state.last_swapchain_query = None;
        }

        if state.size != (width, height) {
            if let Err(error) = mpv.set_property(
                "d3d11-composition-size",
                format!("{width}x{height}"),
            ) {
                log::warn!("could not resize mpv composition swapchain: {error}");
            }
            state.size = (width, height);
        }

        let refresh = state.last_swapchain_query.is_none_or(|at| at.elapsed() >= Duration::from_millis(200));
        let raw = if refresh {
            state.last_swapchain_query = Some(Instant::now());
            mpv.get_property::<i64>("display-swapchain").unwrap_or(0) as isize
        } else {
            state.swapchain_ptr
        };
        let mut swapchain_changed = false;
        if raw == 0 && state.swapchain_ptr != 0 {
            state.swapchain = None;
            HAS_SWAPCHAIN.store(false, Ordering::Release);
            state.swapchain_ptr = 0;
            state.overlay_frame = None;
            swapchain_changed = true;
        }
        if raw != 0 && raw != state.swapchain_ptr {
            let raw_ptr = raw as *mut c_void;
            let Some(borrowed) = (unsafe { IDXGISwapChain::from_raw_borrowed(&raw_ptr) }) else {
                log::error!("mpv returned an invalid D3D11 display swapchain pointer");
                ACTIVE.store(false, Ordering::Release);
                return false;
            };
            let swapchain = borrowed.clone();
            state.swapchain = Some(swapchain);
            HAS_SWAPCHAIN.store(true, Ordering::Release);
            state.swapchain_ptr = raw;
            state.thumbnail_staging = None;
            state.thumbnail_staging_desc = None;
            state.last_back_buffer_index = None;
            state.flip_chain_advanced = false;
            state.overlay_frame = None;
            state.overlay_present_count = None;
            state.last_present_count = None;
            swapchain_changed = true;
            if let Ok(desc) = unsafe { state.swapchain.as_ref().expect("new swapchain").GetDesc() } {
                let effect = match desc.SwapEffect {
                    DXGI_SWAP_EFFECT_FLIP_DISCARD => "flip-discard",
                    DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL => "flip-sequential",
                    DXGI_SWAP_EFFECT_DISCARD => "discard-bitblt",
                    DXGI_SWAP_EFFECT_SEQUENTIAL => "sequential-bitblt",
                    _ => "unknown",
                };
                if let Ok(mut observed) = SWAP_EFFECT.lock() {
                    *observed = Some(effect);
                }
                if effect == "flip-discard" {
                    log::info!(
                        "mpv exposed a D3D11 zero-copy composition swapchain using DXGI_SWAP_EFFECT_FLIP_DISCARD"
                    );
                } else {
                    log::warn!(
                        "mpv composition swapchain uses {effect}; flip-discard was requested but the active mpv/driver selected a fallback"
                    );
                }
            } else {
                log::info!("mpv exposed a D3D11 zero-copy composition swapchain");
            }
        }

        let visible = show_native && !state.media.is_empty() && state.swapchain.is_some();
        let changed = swapchain_changed || state.offset != (x, y) || state.visible != visible;
        if changed {
            let result: windows::core::Result<()> = (|| unsafe {
                state.visual.SetOffsetX2(x as f32)?;
                state.visual.SetOffsetY2(y as f32)?;
                if visible {
                    state.visual.SetContent(
                        state
                            .swapchain
                            .as_ref()
                            .expect("visible composition requires swapchain"),
                    )?;
                } else {
                    state.visual.SetContent(None::<&IUnknown>)?;
                }
                state.device.Commit()?;
                Ok(())
            })();
            if let Err(error) = result {
                log::error!("could not commit D3D11 composition viewport: {error}");
                ACTIVE.store(false, Ordering::Release);
                return false;
            }
            state.offset = (x, y);
            state.visible = visible;
            if visible {
                log::info!(
                    "D3D11 zero-copy swapchain attached to the Pealayer video viewport"
                );
            }
        }

        ACTIVE.store(visible, Ordering::Release);
        visible
    })
}

pub fn has_swapchain() -> bool {
    STATE.with(|slot| slot.borrow().as_ref().is_some_and(|state| state.swapchain.is_some()))
}

pub fn active() -> bool {
    ACTIVE.load(Ordering::Acquire)
}

pub fn diagnostics() -> serde_json::Value {
    serde_json::json!({
        "initialized": INITIALIZED.load(Ordering::Acquire),
        "swapchain_available": HAS_SWAPCHAIN.load(Ordering::Acquire),
        "active": active(),
        "swap_effect": SWAP_EFFECT.lock().ok().and_then(|effect| *effect),
        "zero_copy_presentation": active(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_capture_preserves_color_and_ignores_row_padding() {
        let mut data = [3u8, 2, 1, 0, 99, 99, 99, 99, 6, 5, 4, 0, 98, 98, 98, 98];
        let mapped = D3D11_MAPPED_SUBRESOURCE {
            pData: data.as_mut_ptr().cast(),
            RowPitch: 8,
            DepthPitch: 16,
        };
        let mut description = D3D11_TEXTURE2D_DESC {
            Width: 1,
            Height: 2,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            ..Default::default()
        };
        let bgra = unpack_swapchain_frame(mapped, description).unwrap();
        assert_eq!(bgra.as_raw(), &[1, 2, 3, 255, 4, 5, 6, 255]);
        description.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
        let rgba = unpack_swapchain_frame(mapped, description).unwrap();
        assert_eq!(rgba.as_raw(), &[3, 2, 1, 255, 6, 5, 4, 255]);
    }

    #[test]
    fn staging_capture_expands_ten_bit_channels_and_rejects_bad_stride() {
        let mut data = (1023u32 | (512 << 10)).to_le_bytes();
        let mut mapped = D3D11_MAPPED_SUBRESOURCE {
            pData: data.as_mut_ptr().cast(),
            RowPitch: 4,
            DepthPitch: 4,
        };
        let description = D3D11_TEXTURE2D_DESC {
            Width: 1,
            Height: 1,
            Format: DXGI_FORMAT_R10G10B10A2_UNORM,
            ..Default::default()
        };
        assert_eq!(unpack_swapchain_frame(mapped, description).unwrap().as_raw(), &[255, 128, 0, 255]);
        mapped.RowPitch = 3;
        assert!(unpack_swapchain_frame(mapped, description).is_none());
    }
}
