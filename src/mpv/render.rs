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

/// Private GPU scratch space. Decoder work must not clear/resize the texture
/// egui is already displaying: publish it only after a successful render.
#[derive(Debug)]
pub(crate) struct VideoRenderTarget {
    texture: eframe::glow::Texture,
    pub(crate) fbo: eframe::glow::Framebuffer,
    size: (i32, i32),
}

impl VideoRenderTarget {
    pub(crate) unsafe fn create(gl: &eframe::glow::Context) -> Result<Self, String> {
        use eframe::glow::{self, HasContext};
        unsafe {
            let texture = gl.create_texture()?;
            let fbo = match gl.create_framebuffer() {
                Ok(fbo) => fbo,
                Err(error) => { gl.delete_texture(texture); return Err(error); }
            };
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D, Some(texture), 0);
            Ok(Self { texture, fbo, size: (0, 0) })
        }
    }

    pub(crate) unsafe fn resize(&mut self, gl: &eframe::glow::Context, size: (i32, i32)) -> Result<(), String> {
        use eframe::glow::{self, HasContext};
        if self.size == size { return Ok(()); }
        unsafe {
            let maximum = gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE);
            if size.0 <= 0 || size.1 <= 0 || size.0 > maximum || size.1 > maximum {
                return Err("Video target dimensions exceed this GPU's supported range".into());
            }
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, size.0, size.1, 0,
                glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
            if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                self.size = (0, 0); // Allow a later allocation retry.
                return Err("Video render framebuffer is incomplete".into());
            }
            self.size = size;
            Ok(())
        }
    }

    /// Render privately, then commit. A failed or partially drawn decoder pass
    /// leaves the displayed pixels and dimensions untouched, including resize.
    pub(crate) unsafe fn render_and_publish(&mut self, state: &MpvGlStateGuard<'_>,
        displayed_fbo: eframe::glow::Framebuffer, displayed_texture: eframe::glow::Texture,
        displayed_size: (u32, u32), size: (i32, i32),
        render: impl FnOnce(i32) -> Result<(), String>) -> Result<(), String> {
        use eframe::glow::{self, HasContext};
        let gl = state.gl;
        unsafe {
            self.resize(gl, size)?;
            state.begin_frame(self.fbo, size.0, size.1);
            render(self.fbo.0.get() as i32)?;
            if displayed_size != (size.0 as u32, size.1 as u32) {
                gl.bind_texture(glow::TEXTURE_2D, Some(displayed_texture));
                gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, size.0, size.1, 0,
                    glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
                gl.bind_texture(glow::TEXTURE_2D, None);
                gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(displayed_fbo));
                if gl.check_framebuffer_status(glow::DRAW_FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                    return Err("Video presentation framebuffer is incomplete".into());
                }
            }
            self.publish(gl, displayed_fbo);
            Ok(())
        }
    }

    /// GPU-only publication; no CPU readback and no decoder/texture ID change.
    unsafe fn publish(&self, gl: &eframe::glow::Context,
        displayed_fbo: eframe::glow::Framebuffer) {
        use eframe::glow::{self, HasContext};
        unsafe {
            gl.disable(glow::SCISSOR_TEST);
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.fbo));
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(displayed_fbo));
            gl.blit_framebuffer(0, 0, self.size.0, self.size.1,
                0, 0, self.size.0, self.size.1, glow::COLOR_BUFFER_BIT, glow::NEAREST);
        }
    }
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
    #[cfg(windows)]
    #[test]
    #[ignore = "Requires a local Windows OpenGL context; run explicitly with isolated config"]
    fn gpu_failed_render_and_resize_preserve_displayed_pixels() {
        use eframe::{egui, glow::{self, HasContext}};
        use winit::platform::windows::EventLoopBuilderExtWindows;
        let outcome = std::sync::Arc::new(std::sync::Mutex::new(None));
        let result = outcome.clone();
        let config_path = std::path::PathBuf::from(std::env::var_os("PEALAYER_CONFIG_FILE")
            .expect("GPU probe requires a parent-provided isolated config path"));
        let _finished = eframe::run_native("Pealayer GPU frame publication probe", eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_visible(false).with_inner_size([64.0, 64.0]),
            renderer: eframe::Renderer::Glow,
            persistence_path: Some(config_path.with_extension("gpu.ron")),
            event_loop_builder: Some(Box::new(|builder| { builder.with_any_thread(true); })),
            ..Default::default()
        }, Box::new(move |creation| {
            let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
                let gl = creation.gl.as_ref().expect("OpenGL context");
                eprintln!("GPU publication probe: {} / {}", gl.get_parameter_string(glow::RENDERER),
                    gl.get_parameter_string(glow::VERSION));
                let state = super::MpvGlStateGuard::enter(gl);
                let texture = gl.create_texture().unwrap();
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, 4, 4, 0,
                    glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
                let fbo = gl.create_framebuffer().unwrap();
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
                gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D, Some(texture), 0);
                let mut target = super::VideoRenderTarget::create(gl).unwrap();
                let pixel = || {
                    let mut rgba = [0u8; 4];
                    gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(fbo));
                    gl.read_pixels(0, 0, 1, 1, glow::RGBA, glow::UNSIGNED_BYTE,
                        glow::PixelPackData::Slice(Some(&mut rgba)));
                    rgba
                };
                target.render_and_publish(&state, fbo, texture, (4, 4), (4, 4), |_| {
                    gl.clear_color(0.0, 1.0, 0.0, 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT); Ok(())
                }).unwrap();
                assert_eq!(pixel(), [0, 255, 0, 255]);
                for size in [(4, 4), (8, 6)] {
                    let failed = target.render_and_publish(&state, fbo, texture, (4, 4), size, |_| {
                        gl.clear_color(0.0, 0.0, 0.0, 1.0);
                        gl.clear(glow::COLOR_BUFFER_BIT);
                        Err("Injected decoder render failure".into())
                    });
                    assert!(failed.is_err());
                    assert_eq!(pixel(), [0, 255, 0, 255], "failed pass must not publish black at {size:?}");
                }
                target.render_and_publish(&state, fbo, texture, (4, 4), (8, 6), |_| {
                    gl.clear_color(1.0, 0.0, 0.0, 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT); Ok(())
                }).unwrap();
                assert_eq!(pixel(), [255, 0, 0, 255], "next successful frame publishes after resize");
                assert_eq!(gl.get_error(), glow::NO_ERROR);
                gl.delete_framebuffer(target.fbo);
                gl.delete_texture(target.texture);
                gl.delete_framebuffer(fbo);
                gl.delete_texture(texture);
            }));
            *result.lock().unwrap() = Some(checked.map_err(|_| "GPU publication assertions failed"));
            // Finish before starting a hidden-window event loop. This is an
            // intentional AppCreation exit, not a request to close other apps.
            Err("GPU publication probe finished".into())
        }));
        outcome.lock().unwrap().take().expect("GPU probe ran").unwrap();
    }

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
