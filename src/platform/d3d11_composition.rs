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

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_STAGING, ID3D11Device, ID3D11Texture2D,
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
static SWAP_EFFECT: Mutex<Option<&'static str>> = Mutex::new(None);

thread_local! {
    static STATE: RefCell<Option<CompositionState>> = const { RefCell::new(None) };
}

struct CompositionState {
    hwnd: isize,
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
    })
}

fn unpack_swapchain_frame(
    mapped: D3D11_MAPPED_SUBRESOURCE,
    desc: D3D11_TEXTURE2D_DESC,
) -> Option<image::RgbaImage> {
    let row_pitch = mapped.RowPitch as usize;
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

fn capture_taskbar_thumbnail(
    state: &mut CompositionState,
    hwnd: isize,
    media: &str,
) -> windows::core::Result<()> {
    let Some(swapchain) = state.swapchain.as_ref().cloned() else {
        return Ok(());
    };
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
    let Some((target_width, target_height)) =
        crate::platform::taskbar_preview::begin_capture(media, desc.Width, desc.Height)
    else {
        return Ok(());
    };
    if desc.SampleDesc.Count != 1 {
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
    let context = unsafe { device.GetImmediateContext()? };
    unsafe { context.CopyResource(staging, &source) };
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    unsafe { context.Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))? };
    let frame = unpack_swapchain_frame(mapped, desc);
    unsafe { context.Unmap(staging, 0) };
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
            image::imageops::FilterType::Triangle,
        )
    };
    crate::platform::taskbar_preview::submit_capture(hwnd, frame);
    if !state.logged_thumbnail_capture {
        log::info!(
            "D3D11 video-only taskbar thumbnail capture is active ({}x{})",
            target_width,
            target_height
        );
        state.logged_thumbnail_capture = true;
    }
    Ok(())
}

/// Capture the latest presented composition buffer before the visual is
/// temporarily hidden for an egui popup. This does not alter the swapchain or
/// create another decoder/player.
pub fn capture_for_overlay(hwnd: isize, media: &str) {
    STATE.with(|slot| {
        if let Some(state) = slot.borrow_mut().as_mut()
            && let Err(error) = capture_taskbar_thumbnail(state, hwnd, media)
        {
            log::debug!("D3D11 popup fallback capture failed: {error}");
        }
    });
}

/// Synchronize mpv's composition swapchain with the physical egui video rect.
/// Returns true only when the swapchain is attached and visible.
pub fn update(
    mpv: &libmpv2::Mpv,
    hwnd: isize,
    rect: eframe::egui::Rect,
    pixels_per_point: f32,
    media_loaded: bool,
    taskbar_media: Option<&str>,
) -> bool {
    if hwnd == 0 {
        ACTIVE.store(false, Ordering::Release);
        return false;
    }

    let width = (rect.width() * pixels_per_point).round().max(1.0) as u32;
    let height = (rect.height() * pixels_per_point).round().max(1.0) as u32;
    let x = (rect.left() * pixels_per_point).round() as i32;
    let y = (rect.top() * pixels_per_point).round() as i32;

    STATE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.as_ref().is_none_or(|state| state.hwnd != hwnd) {
            match create_state(hwnd) {
                Ok(state) => {
                    log::info!("D3D11 composition experiment created a DirectComposition target");
                    *slot = Some(state);
                }
                Err(error) => {
                    log::error!("D3D11 composition target creation failed: {error}");
                    ACTIVE.store(false, Ordering::Release);
                    return false;
                }
            }
        }
        let state = slot.as_mut().expect("composition state initialized");

        if state.size != (width, height) {
            if let Err(error) = mpv.set_property(
                "d3d11-composition-size",
                format!("{width}x{height}"),
            ) {
                log::warn!("could not resize mpv composition swapchain: {error}");
            }
            state.size = (width, height);
        }

        let raw = mpv.get_property::<i64>("display-swapchain").unwrap_or(0) as isize;
        let mut swapchain_changed = false;
        if raw != 0 && raw != state.swapchain_ptr {
            let raw_ptr = raw as *mut c_void;
            let Some(borrowed) = (unsafe { IDXGISwapChain::from_raw_borrowed(&raw_ptr) }) else {
                log::error!("mpv returned an invalid D3D11 display swapchain pointer");
                ACTIVE.store(false, Ordering::Release);
                return false;
            };
            let swapchain = borrowed.clone();
            state.swapchain = Some(swapchain);
            state.swapchain_ptr = raw;
            state.thumbnail_staging = None;
            state.thumbnail_staging_desc = None;
            state.last_back_buffer_index = None;
            state.flip_chain_advanced = false;
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

        let visible = media_loaded && state.swapchain.is_some();
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
        if visible
            && let Some(media) = taskbar_media
            && let Err(error) = capture_taskbar_thumbnail(state, hwnd, media)
        {
            log::debug!("D3D11 taskbar thumbnail capture failed: {error}");
        }
        visible
    })
}

pub fn active() -> bool {
    ACTIVE.load(Ordering::Acquire)
}

pub fn diagnostics() -> serde_json::Value {
    serde_json::json!({
        "active": active(),
        "swap_effect": SWAP_EFFECT.lock().ok().and_then(|effect| *effect),
        "flip_discard": SWAP_EFFECT.lock().ok().and_then(|effect| *effect) == Some("flip-discard"),
        "zero_copy": true,
    })
}
