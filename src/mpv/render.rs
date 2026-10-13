use std::sync::Arc;

pub type GetProcAddress = Arc<dyn Fn(&std::ffi::CStr) -> *const std::ffi::c_void + Send + Sync>;

pub fn mpv_get_proc_address(ctx: &GetProcAddress, name: &str) -> *mut std::ffi::c_void {
    if let Ok(cstr) = std::ffi::CString::new(name) {
        (ctx)(&cstr) as *mut _
    } else {
        std::ptr::null_mut()
    }
}

pub struct RenderContextWrapper(
    pub libmpv2::render::RenderContext<'static>,
    pub VideoUpdateSignal,
);
unsafe impl Send for RenderContextWrapper {}
unsafe impl Sync for RenderContextWrapper {}

#[derive(Clone)]
pub struct VideoUpdateSignal(Arc<std::sync::atomic::AtomicBool>);
impl Default for VideoUpdateSignal {
    fn default() -> Self { Self(Arc::new(std::sync::atomic::AtomicBool::new(true))) }
}
impl VideoUpdateSignal {
    pub fn mark(&self) { self.0.store(true, std::sync::atomic::Ordering::Release); }
    pub fn take(&self) -> bool { self.0.swap(false, std::sync::atomic::Ordering::AcqRel) }
}

/// Isolate libmpv from egui's premultiplied blending and window-space clip.
/// libmpv requires standard GL defaults, including disabled BLEND/SCISSOR_TEST.
/// egui_glow rebinds its shader, VAO, buffers and blend function after a callback;
/// framebuffer bindings, clipping coordinates and clear color are our responsibility.
pub(crate) struct MpvGlStateGuard<'a> {
    gl: &'a eframe::glow::Context,
    read_fbo: Option<eframe::glow::Framebuffer>,
    draw_fbo: Option<eframe::glow::Framebuffer>,
    viewport: [i32; 4],
    scissor: [i32; 4],
    clear_color: [f32; 4],
    blend_enabled: bool,
    scissor_enabled: bool,
}

impl<'a> MpvGlStateGuard<'a> {
    /// The caller must own the current GL context on the paint thread.
    pub(crate) unsafe fn enter(gl: &'a eframe::glow::Context) -> Self {
        use eframe::glow::{self, HasContext};
        unsafe {
            let framebuffer = |binding| {
                std::num::NonZeroU32::new(gl.get_parameter_i32(binding) as u32)
                    .map(glow::NativeFramebuffer)
            };
            let mut state = Self {
                gl,
                read_fbo: framebuffer(glow::READ_FRAMEBUFFER_BINDING),
                draw_fbo: framebuffer(glow::DRAW_FRAMEBUFFER_BINDING),
                viewport: [0; 4],
                scissor: [0; 4],
                clear_color: [0.0; 4],
                blend_enabled: gl.is_enabled(glow::BLEND),
                scissor_enabled: gl.is_enabled(glow::SCISSOR_TEST),
            };
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut state.viewport);
            gl.get_parameter_i32_slice(glow::SCISSOR_BOX, &mut state.scissor);
            gl.get_parameter_f32_slice(glow::COLOR_CLEAR_VALUE, &mut state.clear_color);
            gl.disable(glow::BLEND);
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::STENCIL_TEST);
            gl.disable(glow::CULL_FACE);
            gl.color_mask(true, true, true, true);
            gl.use_program(None);
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            state
        }
    }

    /// Clear the entire offscreen target, never through the window's clip box.
    pub(crate) unsafe fn begin_frame(
        &self,
        fbo: eframe::glow::Framebuffer,
        width: i32,
        height: i32,
    ) {
        use eframe::glow::{self, HasContext};
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            self.gl.viewport(0, 0, width, height);
            self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }
}

impl Drop for MpvGlStateGuard<'_> {
    fn drop(&mut self) {
        use eframe::glow::{self, HasContext};
        unsafe {
            self.gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, self.read_fbo);
            self.gl
                .bind_framebuffer(glow::DRAW_FRAMEBUFFER, self.draw_fbo);
            self.gl.viewport(
                self.viewport[0],
                self.viewport[1],
                self.viewport[2],
                self.viewport[3],
            );
            self.gl.scissor(
                self.scissor[0],
                self.scissor[1],
                self.scissor[2],
                self.scissor[3],
            );
            self.gl.clear_color(
                self.clear_color[0],
                self.clear_color[1],
                self.clear_color[2],
                self.clear_color[3],
            );
            for (capability, enabled) in [
                (glow::BLEND, self.blend_enabled),
                (glow::SCISSOR_TEST, self.scissor_enabled),
            ] {
                if enabled {
                    self.gl.enable(capability);
                } else {
                    self.gl.disable(capability);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn decoder_update_signal_coalesces_work_without_losing_the_next_edge() {
        let signal = super::VideoUpdateSignal::default();
        assert!(signal.take()); // First frame always requires initialization.
        assert!(!signal.take());
        let callback = signal.clone();
        std::thread::spawn(move || { callback.mark(); callback.mark(); }).join().unwrap();
        assert!(signal.take());
        assert!(!signal.take());
        signal.mark(); // Callback arriving after the previous consumption.
        assert!(signal.take());
    }

    #[test]
    fn mpv_boundary_disables_ui_compositing_and_restores_independent_targets() {
        // Inspect the production boundary, not a test that requires GPU/driver
        // startup or launches the application on the author's physical board.
        let source = include_str!("render.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        let (enter, restore) = source.split_once("impl Drop for MpvGlStateGuard").unwrap();
        assert!(enter.contains("gl.disable(glow::BLEND)"));
        assert!(enter.contains("gl.disable(glow::SCISSOR_TEST)"));
        assert!(
            enter.find("gl.disable(glow::SCISSOR_TEST)").unwrap()
                < enter.find("self.gl.clear(glow::COLOR_BUFFER_BIT)").unwrap()
        );
        assert!(enter.contains("self.gl.clear_color(0.0, 0.0, 0.0, 1.0)"));
        assert!(restore.contains("glow::READ_FRAMEBUFFER, self.read_fbo"));
        assert!(restore.contains("glow::DRAW_FRAMEBUFFER, self.draw_fbo"));
        assert!(restore.contains("self.blend_enabled"));
        assert!(restore.contains("self.scissor_enabled"));
    }
}
