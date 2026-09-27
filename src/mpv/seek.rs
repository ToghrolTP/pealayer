use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// The seek mode distinguishing high-frequency scrubbing previews from final exact commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekMode {
    /// Scrubbing preview seek (dispatched rapidly while dragging)
    Scrub,
    /// Accurate commit seek (dispatched on mouse release)
    Commit,
}

/// A target position and mode requested by the UI.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingSeek {
    pub target_time: f64,
    pub mode: SeekMode,
}

/// Abstract backend interface for seeking, enabling decoupled testing and MPV integration.
pub trait SeekBackend: Send + Sync + 'static {
    fn execute_seek(&self, target_time: f64, mode: SeekMode);
}

/// MPV seek backend that issues commands via libmpv2.
pub struct MpvSeekBackend {
    mpv: &'static libmpv2::Mpv,
}

impl MpvSeekBackend {
    pub fn new(mpv: &'static libmpv2::Mpv) -> Self {
        Self { mpv }
    }
}

impl SeekBackend for MpvSeekBackend {
    fn execute_seek(&self, target_time: f64, mode: SeekMode) {
        let t = target_time.max(0.0);
        let t_str = t.to_string();
        match mode {
            SeekMode::Scrub => {
                // "absolute" flag in mpv seeks to timestamp and decodes the target preview frame
                let _ = self.mpv.command("seek", &[&t_str, "absolute"]);
            }
            SeekMode::Commit => {
                // "absolute+exact" performs precision seek to exact frame on release
                let _ = self.mpv.command("seek", &[&t_str, "absolute+exact"]);
            }
        }
    }
}

/// A thread-safe, decoupled seek controller that coalesces high-frequency scrub events
/// and executes them asynchronously on a background worker without blocking the UI thread.
pub struct SeekController {
    state: Arc<(Mutex<Option<PendingSeek>>, Condvar)>,
    is_running: Arc<AtomicBool>,
    worker_handle: Option<thread::JoinHandle<()>>,
}

impl SeekController {
    pub fn new<B: SeekBackend>(backend: B) -> Self {
        let state: Arc<(Mutex<Option<PendingSeek>>, Condvar)> =
            Arc::new((Mutex::new(None), Condvar::new()));
        let is_running = Arc::new(AtomicBool::new(true));

        let state_clone = Arc::clone(&state);
        let is_running_clone = Arc::clone(&is_running);

        let worker_handle = thread::spawn(move || {
            let (lock, cvar) = &*state_clone;
            while is_running_clone.load(Ordering::Relaxed) {
                let seek_req = {
                    let mut guard = lock.lock().unwrap();
                    while guard.is_none() && is_running_clone.load(Ordering::Relaxed) {
                        guard = cvar.wait(guard).unwrap();
                    }
                    guard.take()
                };

                if let Some(req) = seek_req {
                    backend.execute_seek(req.target_time, req.mode);
                }
            }
        });

        Self {
            state,
            is_running,
            worker_handle: Some(worker_handle),
        }
    }

    /// Submits a scrub preview request. Rapid successive calls are coalesced so
    /// intermediate targets are skipped if the backend is currently busy decoding.
    pub fn request_scrub(&self, target_time: f64) {
        let (lock, cvar) = &*self.state;
        let mut guard = lock.lock().unwrap();
        *guard = Some(PendingSeek {
            target_time,
            mode: SeekMode::Scrub,
        });
        cvar.notify_one();
    }

    /// Submits a final commit seek request. Overwrites any pending scrub requests.
    pub fn request_commit(&self, target_time: f64) {
        let (lock, cvar) = &*self.state;
        let mut guard = lock.lock().unwrap();
        *guard = Some(PendingSeek {
            target_time,
            mode: SeekMode::Commit,
        });
        cvar.notify_one();
    }
}

impl Drop for SeekController {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
        let (_, cvar) = &*self.state;
        cvar.notify_all();
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}
