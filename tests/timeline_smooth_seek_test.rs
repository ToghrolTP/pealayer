use pealayer::mpv::frame_cache::{CachedFrame, FrameCache};
use pealayer::mpv::seek::{ScrubResult, SeekBackend, SeekController, SeekMode};
use pealayer::ui::controls::resolve_display_time;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
struct MockSeekBackend {
    seeks: Arc<Mutex<Vec<(f64, SeekMode)>>>,
    delay: Option<Duration>,
}

impl SeekBackend for MockSeekBackend {
    fn execute_seek(&self, target_time: f64, mode: SeekMode) {
        if let Some(delay) = self.delay {
            thread::sleep(delay);
        }
        self.seeks.lock().unwrap().push((target_time, mode));
    }
}

fn wait_for_seek_target(seeks: &Mutex<Vec<(f64, SeekMode)>>, target: f64) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if seeks
            .lock()
            .unwrap()
            .last()
            .is_some_and(|seek| seek.0 == target)
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for seek target {target}; observed {:?}",
            seeks.lock().unwrap().as_slice()
        );
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn test_seek_controller_scrub_and_commit() {
    let backend = MockSeekBackend::default();
    let seeks = backend.seeks.clone();
    let controller = SeekController::new(backend);

    // Request scrub
    controller.request_scrub(10.5);
    thread::sleep(Duration::from_millis(50));

    // Request commit
    let commit_id = controller.request_commit(15.0);
    thread::sleep(Duration::from_millis(50));

    let executed = seeks.lock().unwrap().clone();
    assert!(!executed.is_empty(), "Expected at least one seek executed");

    // First should be scrub, last should be commit to 15.0
    let last = executed.last().unwrap();
    assert_eq!(last.0, 15.0);
    assert_eq!(last.1, SeekMode::Commit);
    let completed = controller.take_completed();
    assert!(completed.iter().any(|seek| {
        seek.request_id == commit_id && seek.target_time == 15.0 && seek.mode == SeekMode::Commit
    }));
}

#[test]
fn test_seek_controller_coalesces_rapid_scrub_requests() {
    let backend = MockSeekBackend {
        seeks: Arc::new(Mutex::new(Vec::new())),
        delay: Some(Duration::from_millis(25)),
    };
    let seeks = backend.seeks.clone();
    let controller = SeekController::new(backend);

    // Send 10 rapid scrub requests within 10ms
    for i in 1..=10 {
        controller.request_scrub(i as f64);
        thread::sleep(Duration::from_millis(1));
    }

    // Wait for the observable result instead of assuming a fixed scheduling
    // budget. Hosted macOS runners can occasionally starve the worker beyond
    // 150ms even though the coalescer is behaving correctly.
    wait_for_seek_target(&seeks, 10.0);

    let executed = seeks.lock().unwrap().clone();
    // Because the backend takes 25ms, 10 requests should coalesce to fewer than 10 seeks
    assert!(
        executed.len() < 10,
        "Expected coalescing to reduce seek count from 10, got {}",
        executed.len()
    );
    // The final executed seek must be the latest target (10.0)
    assert_eq!(executed.last().unwrap().0, 10.0);

    let completed = controller.take_completed();
    assert_eq!(completed.last().unwrap().target_time, 10.0);
    assert!(completed.len() < 10);
}

#[test]
fn test_display_time_isolation_during_scrub() {
    let mut playback_time = 5.0;
    let mut seek_pos = None;

    // Normal playback
    assert_eq!(resolve_display_time(seek_pos, playback_time), 5.0);

    // User starts scrubbing to 25.0
    seek_pos = Some(25.0);
    assert_eq!(resolve_display_time(seek_pos, playback_time), 25.0);

    // Delayed playback_time arrives from old frames (e.g. 5.1, 5.2)
    playback_time = 5.2;
    // Display time MUST remain at scrub position 25.0, immune to delayed playback_time
    assert_eq!(resolve_display_time(seek_pos, playback_time), 25.0);

    // Scrub moves to 30.0
    seek_pos = Some(30.0);
    assert_eq!(resolve_display_time(seek_pos, playback_time), 30.0);

    // Scrub completes and settles
    seek_pos = None;
    playback_time = 30.0;
    assert_eq!(resolve_display_time(seek_pos, playback_time), 30.0);
}

#[test]
fn test_seek_controller_serves_cache_hits_without_backend_dispatch() {
    let backend = MockSeekBackend::default();
    let seeks = backend.seeks.clone();
    let cache = Arc::new(RwLock::new(FrameCache::new(10 * 1024 * 1024)));

    // Prepopulate cache with frame at 5.0s
    let frame = CachedFrame::new(5.0, 320, 180, vec![42u8; 320 * 180 * 4]);
    cache.write().unwrap().insert(frame, 5.0);

    let controller = SeekController::with_cache(backend, cache);

    // Scrub to cached frame (within 20ms tolerance)
    let result = controller.request_scrub(5.01);
    match result {
        ScrubResult::Cached(hit) => {
            assert_eq!(hit.pts, 5.0);
        }
        ScrubResult::Dispatched(_) => panic!("Expected cache hit, got dispatched seek"),
    }

    thread::sleep(Duration::from_millis(30));
    assert!(
        seeks.lock().unwrap().is_empty(),
        "Backend should not receive seek command on cache hit"
    );

    // Scrub to uncached frame
    let miss_result = controller.request_scrub(12.0);
    match miss_result {
        ScrubResult::Dispatched(_) => {}
        ScrubResult::Cached(_) => panic!("Expected cache miss, got cache hit"),
    }

    wait_for_seek_target(&seeks, 12.0);
}

