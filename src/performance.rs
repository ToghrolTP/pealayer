//! Fixed-size, allocation-free native frame timing. No history, files or probes.
use std::sync::{atomic::{AtomicU64, Ordering}, OnceLock};

pub enum Phase { Logic, Ui, Video, Menu, Status, Program, Inspector, Cue, Library, Hardware, Timeline, Designer }
struct Timing { count: AtomicU64, total: AtomicU64, last: AtomicU64, max: AtomicU64 }
impl Timing {
    const fn new() -> Self { Self { count: AtomicU64::new(0), total: AtomicU64::new(0),
        last: AtomicU64::new(0), max: AtomicU64::new(0) } }
}
static TIMINGS: [Timing; 12] = [const { Timing::new() }; 12];
static RENDERER: OnceLock<serde_json::Value> = OnceLock::new();
pub struct Span { phase: usize, start: std::time::Instant }
pub fn span(phase: Phase) -> Span { Span { phase: phase as usize, start: std::time::Instant::now() } }
impl Drop for Span {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed().as_micros().min(u64::MAX as u128) as u64;
        let timing = &TIMINGS[self.phase];
        timing.total.fetch_add(elapsed, Ordering::Relaxed);
        timing.last.store(elapsed, Ordering::Relaxed);
        timing.max.fetch_max(elapsed, Ordering::Relaxed);
        timing.count.fetch_add(1, Ordering::Relaxed);
    }
}
/// Only called on the paint thread with its current OpenGL context, once.
pub unsafe fn observe_renderer(gl: &eframe::glow::Context) {
    if RENDERER.get().is_some() { return; }
    use eframe::glow::HasContext;
    let bounded = |value: String| value.chars().take(256).collect::<String>();
    let value = unsafe { serde_json::json!({
        "vendor":bounded(gl.get_parameter_string(eframe::glow::VENDOR)),
        "renderer":bounded(gl.get_parameter_string(eframe::glow::RENDERER)),
        "version":bounded(gl.get_parameter_string(eframe::glow::VERSION)),
    }) };
    let _ = RENDERER.set(value);
}
pub fn snapshot() -> serde_json::Value {
    let phases: Vec<_> = ["logic", "ui", "video_callback", "menu", "status_bar", "program_monitor",
        "media_inspector", "cue_controls", "effects_library", "hardware_monitor", "timeline", "effects_designer"]
        .iter().zip(TIMINGS.iter()).map(|(name, timing)| {
        serde_json::json!({"phase":name, "count":timing.count.load(Ordering::Relaxed),
            "total_us":timing.total.load(Ordering::Relaxed), "last_us":timing.last.load(Ordering::Relaxed),
            "max_us":timing.max.load(Ordering::Relaxed)})
    }).collect();
    serde_json::json!({"process_id":std::process::id(),"phases":phases,"renderer":RENDERER.get(),
        "mpv_render_count":crate::ui::video::MPV_RENDER_COUNT.load(Ordering::Relaxed),
        "mpv_reused_count":crate::ui::video::MPV_RENDER_SKIPPED_COUNT.load(Ordering::Relaxed),
        "mpv_last_render_us":crate::ui::video::LAST_MPV_RENDER_MICROS.load(Ordering::Relaxed),
        "mpv_max_render_us":crate::ui::video::MAX_MPV_RENDER_MICROS.load(Ordering::Relaxed)})
}
