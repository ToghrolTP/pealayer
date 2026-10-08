use pealayer::mpv::frame_cache::{CachedFrame, FrameCache};
use pealayer::mpv::seek::{ScrubResult, SeekBackend, SeekController, SeekMode};
use pealayer::ui::controls::resolve_display_time;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
struct MockSeekBackend {
    seeks: Arc<Mutex<Vec<(f64, SeekMode)>>>,
}

impl SeekBackend for MockSeekBackend {
    fn execute_seek(&self, target_time: f64, mode: SeekMode) {
        self.seeks.lock().unwrap().push((target_time, mode));
    }
}

#[test]
fn test_scrub_fallback_hierarchy() {
    let cache = Arc::new(RwLock::new(FrameCache::new(10 * 1024 * 1024)));

    // Tier 1: Exact frame available
    let exact = CachedFrame::new(4.0, 160, 90, vec![255; 160 * 90 * 4]);
    cache.write().unwrap().insert(exact.clone(), 4.0);

    let query_hit = cache.read().unwrap().query_exact(4.005, 0.02);
    assert!(query_hit.is_some());
    assert_eq!(query_hit.unwrap().pts, 4.0);

    // Tier 2: Nearest frame available
    let query_near = cache.read().unwrap().query_nearest(4.5);
    assert!(query_near.is_some());
    assert_eq!(query_near.unwrap().pts, 4.0);

    // Tier 3: Empty cache returns None, signaling disk thumbnail fallback
    cache.write().unwrap().clear();
    assert!(cache.read().unwrap().query_nearest(4.5).is_none());
}

#[test]
fn test_rapid_scrub_stress_across_cache_boundaries() {
    let backend = MockSeekBackend::default();
    let cache = Arc::new(RwLock::new(FrameCache::new(5 * 1024 * 1024)));

    // Prepopulate cache with frames at selected intervals (e.g., even seconds from 0.0 to 40.0)
    for s in (0..=40).step_by(2) {
        let pts = s as f64;
        let frame = CachedFrame::new(pts, 160, 90, vec![128u8; 160 * 90 * 4]);
        cache.write().unwrap().insert(frame, pts);
    }

    let controller = SeekController::with_cache(backend.clone(), cache.clone());

    let mut cached_count = 0;
    let mut dispatched_count = 0;

    // Issue 100 sequential scrub requests crossing cached frames and gaps
    for i in 0..100 {
        let target = (i as f64) * 0.4;
        match controller.request_scrub(target) {
            ScrubResult::Cached(frame) => {
                cached_count += 1;
                // Cached result must be within the 20ms tolerance window
                assert!(
                    (frame.pts - target).abs() <= 0.02,
                    "Cached frame PTS {} deviated from target {}",
                    frame.pts,
                    target
                );
            }
            ScrubResult::Dispatched(req_id) => {
                dispatched_count += 1;
                assert!(req_id > 0);
            }
        }

        // Periodically inject new decoded frames during scrubbing to simulate background playback
        if i % 10 == 0 {
            let dyn_pts = target + 0.01;
            let dyn_frame = CachedFrame::new(dyn_pts, 160, 90, vec![200u8; 160 * 90 * 4]);
            cache.write().unwrap().insert(dyn_frame, target);
        }
    }

    assert_eq!(
        cached_count + dispatched_count,
        100,
        "Total requests must equal 100"
    );
    assert!(cached_count > 0, "Expected at least some cache hits");
    assert!(
        dispatched_count > 0,
        "Expected at least some dispatched seeks"
    );

    // Commit seek to conclude the scrub sequence
    let commit_id = controller.request_commit(40.0);
    assert!(commit_id > 0);

    // Allow background worker to process dispatched seeks deterministically
    let start = Instant::now();
    let timeout = Duration::from_secs(2);
    let mut completed = Vec::new();
    while start.elapsed() < timeout {
        completed.extend(controller.take_completed());
        if !completed.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !completed.is_empty(),
        "Backend worker should have processed seeks within timeout"
    );
}

#[test]
fn test_memory_limit_enforcement_continuous_insertions() {
    // Tight 64 KB cache budget
    let max_bytes = 64 * 1024;
    let mut cache = FrameCache::new(max_bytes);

    let frame_w = 32;
    let frame_h = 32;
    let frame_len = (frame_w * frame_h * 4) as usize;

    // 1,000 continuous insertions simulating high-volume frame decoding
    for i in 0..1000 {
        let pts = (i as f64) * (1.0 / 30.0);
        let frame = CachedFrame::new(pts, frame_w, frame_h, vec![(i % 255) as u8; frame_len]);
        let playhead = pts;
        cache.insert(frame, playhead);

        // Invariant: current_bytes must never exceed max_bytes
        assert!(
            cache.current_bytes() <= cache.max_bytes(),
            "Memory limit breached on frame {}: {} bytes > max {} bytes",
            i,
            cache.current_bytes(),
            cache.max_bytes()
        );
        assert!(!cache.is_empty());
    }

    // Cache should stay bounded under max_bytes
    assert!(cache.len() <= 16);
    assert!(cache.len() >= 12);
    assert!(cache.current_bytes() <= max_bytes);

    // Verify playhead-aware retention: frames near final playhead (frame 999) must be retained
    let final_pts = 999.0 / 30.0;
    let nearest = cache
        .query_nearest(final_pts)
        .expect("nearest frame present");
    assert!(
        (nearest.pts - final_pts).abs() < 1.0,
        "Retained frames should be close to latest playhead"
    );

    // Distant frames from earlier in the stream must have been evicted
    assert!(
        cache.query_exact(1.0, 0.01).is_none(),
        "Old frames should be evicted under memory pressure"
    );

    // Stationary playhead in center: insert 500 frames and ensure memory stays bounded
    for i in 0..500 {
        let pts = (i as f64) * 0.1;
        let frame = CachedFrame::new(pts, frame_w, frame_h, vec![0u8; frame_len]);
        cache.insert(frame, 10.0);

        assert!(
            cache.current_bytes() <= cache.max_bytes(),
            "Memory limit breached with stationary playhead on frame {}",
            i
        );
    }
    let center_nearest = cache
        .query_nearest(10.0)
        .expect("frame nearest to 10.0 present");
    assert!((center_nearest.pts - 10.0).abs() <= 1.0);

    // Clear operation
    cache.clear();
    assert_eq!(cache.current_bytes(), 0);
    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());
}

#[test]
fn test_edge_cases_non_finite_and_negative_seek_times() {
    let mut cache = FrameCache::new(1024 * 1024);

    // 1. FrameCache queries with edge values on empty cache
    assert!(cache.query_exact(f64::NAN, 0.02).is_none());
    assert!(cache.query_exact(f64::INFINITY, 0.02).is_none());
    assert!(cache.query_exact(f64::NEG_INFINITY, 0.02).is_none());
    assert!(cache.query_exact(-5.0, 0.02).is_none());
    assert!(cache.query_exact(0.0, 0.0).is_none());
    assert!(cache.query_nearest(f64::NAN).is_none());
    assert!(cache.query_nearest(f64::INFINITY).is_none());
    assert!(cache.query_nearest(f64::NEG_INFINITY).is_none());
    assert!(cache.query_nearest(-5.0).is_none());
    assert!(cache.query_nearest(0.0).is_none());

    // 2. Insert valid frames
    let f0 = CachedFrame::new(0.0, 16, 16, vec![0u8; 1024]);
    let f10 = CachedFrame::new(10.0, 16, 16, vec![1u8; 1024]);
    cache.insert(f0, 0.0);
    cache.insert(f10, 0.0);

    // Queries with edge values on populated cache
    assert!(cache.query_exact(f64::NAN, 0.02).is_none());
    assert!(cache.query_exact(f64::INFINITY, 0.02).is_none());
    assert!(cache.query_exact(f64::NEG_INFINITY, 0.02).is_none());
    assert!(cache.query_exact(-5.0, 0.02).is_none());
    assert_eq!(cache.query_exact(0.0, 0.0).unwrap().pts, 0.0);

    // Nearest queries with edge values must not panic
    let _ = cache.query_nearest(f64::NAN);
    assert!(cache.query_nearest(f64::INFINITY).is_some());
    assert!(cache.query_nearest(f64::NEG_INFINITY).is_some());
    assert_eq!(cache.query_nearest(-10.0).unwrap().pts, 0.0);
    assert_eq!(cache.query_nearest(0.0).unwrap().pts, 0.0);

    // 3. Inserting frames with non-finite / negative PTS and playhead
    let nan_frame = CachedFrame::new(f64::NAN, 16, 16, vec![0u8; 1024]);
    cache.insert(nan_frame, f64::NAN);
    assert!(cache.current_bytes() <= cache.max_bytes());

    let inf_frame = CachedFrame::new(f64::INFINITY, 16, 16, vec![0u8; 1024]);
    cache.insert(inf_frame, f64::INFINITY);
    assert!(cache.current_bytes() <= cache.max_bytes());

    let neg_inf_frame = CachedFrame::new(f64::NEG_INFINITY, 16, 16, vec![0u8; 1024]);
    cache.insert(neg_inf_frame, f64::NEG_INFINITY);
    assert!(cache.current_bytes() <= cache.max_bytes());

    let neg_frame = CachedFrame::new(-10.0, 16, 16, vec![0u8; 1024]);
    cache.insert(neg_frame, -10.0);
    assert!(cache.current_bytes() <= cache.max_bytes());

    // 4. SeekController with non-finite and negative seek times
    let backend = MockSeekBackend::default();
    let cache_lock = Arc::new(RwLock::new(cache));
    let controller = SeekController::with_cache(backend.clone(), cache_lock);

    let edge_times = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -100.0,
        -1.0,
        -0.0001,
        0.0,
    ];

    for &t in &edge_times {
        let scrub_res = controller.request_scrub(t);
        match scrub_res {
            ScrubResult::Cached(_) | ScrubResult::Dispatched(_) => {}
        }
        let commit_id = controller.request_commit(t);
        assert!(commit_id > 0);
    }

    // Poll deterministically for backend worker completion
    let start = Instant::now();
    let timeout = Duration::from_secs(2);
    let mut completed = Vec::new();
    while start.elapsed() < timeout {
        completed.extend(controller.take_completed());
        if !completed.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !completed.is_empty(),
        "Backend worker should have processed edge case seeks within timeout"
    );

    // 5. Display time resolution with edge values
    assert_eq!(resolve_display_time(Some(0.0), 10.0), 0.0);
    assert_eq!(resolve_display_time(Some(-5.0), 10.0), -5.0);
    assert_eq!(resolve_display_time(None, 0.0), 0.0);
    assert_eq!(resolve_display_time(None, -5.0), -5.0);
    let nan_res = resolve_display_time(Some(f64::NAN), 10.0);
    assert!(nan_res.is_nan());
    let inf_res = resolve_display_time(Some(f64::INFINITY), 10.0);
    assert!(inf_res.is_infinite());

    // 6. Zero-duration clamping safety check
    let duration = 0.0;
    for &t in &edge_times {
        let clamped = if duration > 0.0 {
            t.clamp(0.0, duration)
        } else {
            t.max(0.0)
        };
        // Ensure clamped values for negative / zero durations are zero (or NaN stays NaN)
        if !t.is_nan() && t <= 0.0 {
            assert_eq!(clamped, 0.0);
        }
    }
}
