use std::sync::Arc;

pub type GetProcAddress = Arc<dyn Fn(&std::ffi::CStr) -> *const std::ffi::c_void + Send + Sync>;

pub fn mpv_get_proc_address(ctx: &GetProcAddress, name: &str) -> *mut std::ffi::c_void {
    if let Ok(cstr) = std::ffi::CString::new(name) {
        (ctx)(&cstr) as *mut _
    } else {
        std::ptr::null_mut()
    }
}

pub struct RenderContextWrapper(pub libmpv2::render::RenderContext<'static>);
unsafe impl Send for RenderContextWrapper {}
unsafe impl Sync for RenderContextWrapper {}
