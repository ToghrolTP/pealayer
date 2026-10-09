use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;

use crate::mpv::frame_cache::{CachedFrame, FrameCache};

/// Result of a scrub request: either served immediately from the frame cache
/// or dispatched to the background worker.
#[derive(Debug, Clone)]
pub enum ScrubResult {
    Cached(CachedFrame),
    Dispatched(u64),
}


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
    pub request_id: u64,
    pub target_time: f64,
    pub mode: SeekMode,
}

/// A seek command that has been accepted by the backend worker.
///
/// This is deliberately distinct from mpv's `PlaybackRestart` event: command
/// acceptance tells the UI which coalesced request was actually dispatched,
/// while `PlaybackRestart` confirms that decoding reached a displayable frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompletedSeek {
    pub request_id: u64,
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
        if let Some(client)=crate::peer::client() {
            let command = match mode { SeekMode::Scrub => "scrub_to", SeekMode::Commit => "finish_scrub" };
            let _=client.queue("/api/player/command",serde_json::json!({"command":command,"seconds":target_time.max(0.0)}));
            return;
        }
        let t = target_time.max(0.0);
        let t_str = t.to_string();
        match mode {
            SeekMode::Scrub => {
                // High-precision scrubbing via absolute+exact seek with decoder frame-drop
                let _ = self.mpv.command("seek", &[&t_str, "absolute+exact"]);
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
    completed: Arc<Mutex<VecDeque<CompletedSeek>>>,
    next_request_id: AtomicU64,
    is_running: Arc<AtomicBool>,
    worker_handle: Option<thread::JoinHandle<()>>,
    frame_cache: Option<Arc<RwLock<FrameCache>>>,
}

impl SeekController {
    pub fn new<B: SeekBackend>(backend: B) -> Self {
        Self::with_cache_internal(backend, None)
    }

    pub fn with_cache<B: SeekBackend>(backend: B, cache: Arc<RwLock<FrameCache>>) -> Self {
        Self::with_cache_internal(backend, Some(cache))
    }

    fn with_cache_internal<B: SeekBackend>(
        backend: B,
        frame_cache: Option<Arc<RwLock<FrameCache>>>,
    ) -> Self {
        let state: Arc<(Mutex<Option<PendingSeek>>, Condvar)> =
            Arc::new((Mutex::new(None), Condvar::new()));
        let is_running = Arc::new(AtomicBool::new(true));
        let completed = Arc::new(Mutex::new(VecDeque::new()));

        let state_clone = Arc::clone(&state);
        let is_running_clone = Arc::clone(&is_running);
        let completed_clone = Arc::clone(&completed);

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
                    completed_clone.lock().unwrap().push_back(CompletedSeek {
                        request_id: req.request_id,
                        target_time: req.target_time,
                        mode: req.mode,
                    });
                }
            }
        });

        Self {
            state,
            completed,
            next_request_id: AtomicU64::new(1),
            is_running,
            worker_handle: Some(worker_handle),
            frame_cache,
        }
    }

    /// Cached previews must be exact and must still move the decoder. A nearby
    /// cached frame otherwise changes to a different frame when the gesture ends.
    pub fn request_scrub(&self, target_time: f64) -> ScrubResult {
        let cached = self.frame_cache.as_ref().and_then(|cache| {
            cache.read().ok().and_then(|cache| cache.query_exact(target_time, 0.000_001))
        });

        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let (lock, cvar) = &*self.state;
        let mut guard = lock.lock().unwrap();
        *guard = Some(PendingSeek {
            request_id,
            target_time,
            mode: SeekMode::Scrub,
        });
        cvar.notify_one();
        cached.map_or(ScrubResult::Dispatched(request_id), ScrubResult::Cached)
    }

    /// Submits a final commit seek request. Overwrites any pending scrub requests.
    pub fn request_commit(&self, target_time: f64) -> u64 {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let (lock, cvar) = &*self.state;
        let mut guard = lock.lock().unwrap();
        *guard = Some(PendingSeek {
            request_id,
            target_time,
            mode: SeekMode::Commit,
        });
        cvar.notify_one();
        request_id
    }

    /// Drains backend acknowledgements without blocking the UI thread.
    pub fn take_completed(&self) -> Vec<CompletedSeek> {
        self.completed.lock().unwrap().drain(..).collect()
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
