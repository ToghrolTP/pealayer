use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;

use crate::mpv::frame_cache::{CachedFrame, FrameCache};
use crate::mpv::seek_completion::NativeSeekClient;

// Clients can outlive a UI fixture/window. Do not reuse an async reply token
// when a new controller is constructed against the same decoder core.
static NEXT_SEEK_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

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
    pub native_zero: bool,
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
    pub accepted: bool,
}

/// Abstract backend interface for seeking, enabling decoupled testing and MPV integration.
pub trait SeekBackend: Send + Sync + 'static {
    fn execute_seek(&self, request: PendingSeek) -> bool;
}

/// MPV seek backend that issues commands via libmpv2.
pub struct MpvSeekBackend {
    mpv: &'static libmpv2::Mpv,
    completion: Arc<NativeSeekClient>,
}

impl MpvSeekBackend {
    pub(crate) fn new(mpv: &'static libmpv2::Mpv, completion: Arc<NativeSeekClient>) -> Self {
        Self { mpv, completion }
    }
}

impl SeekBackend for MpvSeekBackend {
    fn execute_seek(&self, request: PendingSeek) -> bool {
        let PendingSeek {
            request_id,
            target_time,
            mode,
            native_zero,
        } = request;
        if native_zero {
            // Revalidate the authority route at dispatch. A mode switch must
            // not turn a locally fenced seek into an untracked peer command.
            return mode == SeekMode::Commit
                && target_time == 0.0
                && !crate::peer::active()
                && !crate::mpv::external::active()
                && self.completion.queue_zero(request_id);
        }
        if let Some(client) = crate::peer::client() {
            let command = match mode {
                SeekMode::Scrub => "scrub_to",
                SeekMode::Commit => "finish_scrub",
            };
            return client
                .queue(
                    "/api/player/command",
                    serde_json::json!({"command":command,"seconds":target_time.max(0.0)}),
                )
                .is_ok();
        }
        let t = target_time.max(0.0);
        let t_str = t.to_string();
        match mode {
            SeekMode::Scrub => {
                // High-precision scrubbing via absolute+exact seek with decoder frame-drop
                crate::mpv::player::Player(self.mpv)
                    .command("seek", &[&t_str, "absolute+exact"])
                    .is_ok()
            }
            SeekMode::Commit => {
                // "absolute+exact" performs precision seek to exact frame on release
                crate::mpv::player::Player(self.mpv)
                    .command("seek", &[&t_str, "absolute+exact"])
                    .is_ok()
            }
        }
    }
}

/// A thread-safe, decoupled seek controller that coalesces high-frequency scrub events
/// and executes them asynchronously on a background worker without blocking the UI thread.
pub struct SeekController {
    state: Arc<(Mutex<Option<PendingSeek>>, Condvar)>,
    completed: Arc<Mutex<VecDeque<CompletedSeek>>>,
    latest_request: Arc<AtomicU64>,
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
        let latest_request = Arc::new(AtomicU64::new(0));
        let completed = Arc::new(Mutex::new(VecDeque::new()));

        let state_clone = Arc::clone(&state);
        let is_running_clone = Arc::clone(&is_running);
        let completed_clone = Arc::clone(&completed);
        let latest_request_clone = Arc::clone(&latest_request);

        let worker_handle = thread::spawn(move || {
            let (lock, cvar) = &*state_clone;
            while is_running_clone.load(Ordering::SeqCst) {
                let seek_req = {
                    let mut guard = lock.lock().unwrap();
                    while guard.is_none() && is_running_clone.load(Ordering::SeqCst) {
                        guard = cvar.wait(guard).unwrap();
                    }
                    guard.take()
                };

                if let Some(req) = seek_req {
                    let accepted = is_running_clone.load(Ordering::Acquire)
                        && latest_request_clone.load(Ordering::Acquire) == req.request_id
                        && backend.execute_seek(req);
                    completed_clone.lock().unwrap().push_back(CompletedSeek {
                        request_id: req.request_id,
                        target_time: req.target_time,
                        mode: req.mode,
                        accepted,
                    });
                }
            }
        });

        Self {
            state,
            completed,
            latest_request,
            is_running,
            worker_handle: Some(worker_handle),
            frame_cache,
        }
    }

    /// Cached previews must be exact and must still move the decoder. A nearby
    /// cached frame otherwise changes to a different frame when the gesture ends.
    pub fn request_scrub(&self, target_time: f64) -> ScrubResult {
        let cached = self.frame_cache.as_ref().and_then(|cache| {
            cache
                .read()
                .ok()
                .and_then(|cache| cache.query_exact(target_time, 0.000_001))
        });

        let request_id = NEXT_SEEK_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
        let (lock, cvar) = &*self.state;
        let mut guard = lock.lock().unwrap();
        self.latest_request.store(request_id, Ordering::Release);
        *guard = Some(PendingSeek {
            request_id,
            target_time,
            mode: SeekMode::Scrub,
            native_zero: false,
        });
        cvar.notify_one();
        cached.map_or(ScrubResult::Dispatched(request_id), ScrubResult::Cached)
    }

    /// Submits a final commit seek request. Overwrites any pending scrub requests.
    pub fn request_commit(&self, target_time: f64) -> PendingSeek {
        let request_id = NEXT_SEEK_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
        let (lock, cvar) = &*self.state;
        let mut guard = lock.lock().unwrap();
        self.latest_request.store(request_id, Ordering::Release);
        let request = PendingSeek {
            request_id,
            target_time,
            mode: SeekMode::Commit,
            native_zero: target_time == 0.0
                && !crate::peer::active()
                && !crate::mpv::external::active(),
        };
        *guard = Some(request);
        cvar.notify_one();
        request
    }

    /// Revoke queued work. Already submitted decoder commands may still finish;
    /// their tokens cannot settle another request or media epoch.
    pub fn cancel_pending(&self) {
        if let Ok(mut pending) = self.state.0.lock() {
            self.latest_request.store(0, Ordering::Release);
            *pending = None;
        }
    }

    /// Drains backend acknowledgements without blocking the UI thread.
    pub fn take_completed(&self) -> Vec<CompletedSeek> {
        self.completed.lock().unwrap().drain(..).collect()
    }
}

impl Drop for SeekController {
    fn drop(&mut self) {
        let (lock, cvar) = &*self.state;
        {
            let mut guard = lock.lock().unwrap();
            self.is_running.store(false, Ordering::SeqCst);
            self.latest_request.store(0, Ordering::Release);
            *guard = None;
            cvar.notify_all();
        }
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Rejected;
    impl SeekBackend for Rejected {
        fn execute_seek(&self, _: PendingSeek) -> bool {
            false
        }
    }

    #[test]
    fn controllers_never_reuse_async_reply_tokens() {
        let first = SeekController::new(Rejected);
        let second = SeekController::new(Rejected);
        let a = first.request_commit(1.0);
        let b = second.request_commit(2.0);
        assert_ne!(a.request_id, b.request_id);
        assert_ne!(a.request_id, 0);
        assert_ne!(b.request_id, 0);
    }

    #[test]
    fn rejected_backend_reports_failure_for_the_matching_request() {
        let controller = SeekController::new(Rejected);
        let request = controller.request_commit(1.0);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if let Some(completion) = controller
                .take_completed()
                .into_iter()
                .find(|completion| completion.request_id == request.request_id)
            {
                assert!(!completion.accepted);
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
