use pealayer::mpv::frame_cache::{CachedFrame, FrameCache};
use std::sync::Arc;

#[test]
fn test_frame_cache_exact_and_nearest_queries() {
    let mut cache = FrameCache::new(10 * 1024 * 1024); // 10 MB

    let frame1 = CachedFrame::new(1.0, 320, 180, vec![0u8; 320 * 180 * 4]);
    let frame2 = CachedFrame::new(1.05, 320, 180, vec![1u8; 320 * 180 * 4]);
    let frame3 = CachedFrame::new(2.0, 320, 180, vec![2u8; 320 * 180 * 4]);

    cache.insert(frame1, 1.0);
    cache.insert(frame2, 1.0);
    cache.insert(frame3, 1.0);

    assert_eq!(cache.len(), 3);

    // Exact query with 20ms tolerance
    let exact_hit = cache.query_exact(1.04, 0.02);
    assert!(exact_hit.is_some());
    assert_eq!(exact_hit.unwrap().pts, 1.05);

    let exact_miss = cache.query_exact(1.5, 0.02);
    assert!(exact_miss.is_none());

    // Nearest query
    let nearest = cache.query_nearest(1.4).expect("nearest should find frame");
    assert_eq!(nearest.pts, 1.05);

    let nearest_late = cache.query_nearest(1.8).expect("nearest should find frame");
    assert_eq!(nearest_late.pts, 2.0);
}

#[test]
fn test_frame_cache_eviction_under_memory_ceiling() {
    // Each 100x100 RGBA frame is ~40 KB
    let frame_bytes = 100 * 100 * 4;
    let max_bytes = frame_bytes * 3 + 2048;
    let mut cache = FrameCache::new(max_bytes);

    let f1 = CachedFrame::new(10.0, 100, 100, vec![0u8; frame_bytes]);
    let f2 = CachedFrame::new(10.1, 100, 100, vec![0u8; frame_bytes]);
    let f3 = CachedFrame::new(10.2, 100, 100, vec![0u8; frame_bytes]);
    let f_distant = CachedFrame::new(30.0, 100, 100, vec![0u8; frame_bytes]);

    let playhead = 10.1;
    cache.insert(f1, playhead);
    cache.insert(f2, playhead);
    cache.insert(f_distant, playhead);
    // Inserting f3 should exceed budget and evict f_distant (furthest from playhead 10.1)
    cache.insert(f3, playhead);

    assert!(cache.current_bytes() <= max_bytes);
    assert!(
        cache.query_exact(30.0, 0.01).is_none(),
        "Distant frame should be evicted"
    );
    assert!(cache.query_exact(10.0, 0.01).is_some());
    assert!(cache.query_exact(10.1, 0.01).is_some());
    assert!(cache.query_exact(10.2, 0.01).is_some());
}

#[test]
fn test_frame_cache_replacement_and_clear() {
    let mut cache = FrameCache::new(1024 * 1024);
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.current_bytes(), 0);

    let f1 = CachedFrame::new(5.0, 10, 10, vec![1u8; 400]);
    cache.insert(f1, 5.0);
    assert_eq!(cache.len(), 1);
    let bytes_before = cache.current_bytes();

    // Replace frame with same PTS (~5.0001 within 0.001 tolerance)
    let f1_updated = CachedFrame::new(5.0002, 10, 10, vec![2u8; 800]);
    cache.insert(f1_updated, 5.0);
    assert_eq!(cache.len(), 1);
    assert!(cache.current_bytes() > bytes_before);

    let queried = cache.query_exact(5.0, 0.001).expect("frame found");
    assert_eq!(queried.rgba[0], 2u8);

    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.current_bytes(), 0);
    assert!(cache.query_nearest(5.0).is_none());
}

#[test]
fn test_app_initializes_and_clears_frame_cache() {
    let cache = Arc::new(std::sync::RwLock::new(FrameCache::new(128 * 1024 * 1024)));
    assert_eq!(cache.read().unwrap().len(), 0);
    assert_eq!(cache.read().unwrap().max_bytes(), 128 * 1024 * 1024);

    let dummy = CachedFrame::new(0.0, 10, 10, vec![0; 400]);
    cache.write().unwrap().insert(dummy, 0.0);
    assert_eq!(cache.read().unwrap().len(), 1);

    cache.write().unwrap().clear();
    assert_eq!(cache.read().unwrap().len(), 0);
}

#[test]
fn test_frame_cache_replacement_enforces_eviction() {
    // Budget: 100 KB
    let max_bytes = 100 * 1024;
    let mut cache = FrameCache::new(max_bytes);

    // Two frames of ~40 KB each (total ~80 KB < 100 KB)
    let f1 = CachedFrame::new(10.0, 100, 100, vec![1u8; 40 * 1024]);
    let f2 = CachedFrame::new(20.0, 100, 100, vec![2u8; 40 * 1024]);

    let playhead = 10.0;
    cache.insert(f1, playhead);
    cache.insert(f2, playhead);
    assert_eq!(cache.len(), 2);
    assert!(cache.current_bytes() <= max_bytes);

    // Replace f1 with a larger frame (~70 KB)
    // New total before eviction: 70 KB + 40 KB = 110 KB > 100 KB budget
    let f1_larger = CachedFrame::new(10.0001, 100, 100, vec![3u8; 70 * 1024]);
    cache.insert(f1_larger, playhead);

    // Eviction must trigger: f2 (at 20.0, furthest from playhead 10.0) is evicted
    assert!(
        cache.current_bytes() <= cache.max_bytes(),
        "Cache byte size {} must not exceed max {}",
        cache.current_bytes(),
        cache.max_bytes()
    );
    assert_eq!(cache.len(), 1);
    assert!(
        cache.query_exact(10.0, 0.01).is_some(),
        "Replaced frame near playhead must be retained"
    );
    assert!(
        cache.query_exact(20.0, 0.01).is_none(),
        "Furthest frame must have been evicted"
    );

    // Also verify replacement where the replaced frame itself is furthest from playhead
    let mut cache2 = FrameCache::new(max_bytes);
    let f_near = CachedFrame::new(20.0, 100, 100, vec![1u8; 40 * 1024]);
    let f_far = CachedFrame::new(5.0, 100, 100, vec![2u8; 40 * 1024]);
    let playhead2 = 20.0;
    cache2.insert(f_near, playhead2);
    cache2.insert(f_far, playhead2);
    assert_eq!(cache2.len(), 2);

    // Replace f_far with larger frame when playhead is at 20.0
    let f_far_larger = CachedFrame::new(5.0001, 100, 100, vec![3u8; 70 * 1024]);
    cache2.insert(f_far_larger, playhead2);
    assert!(cache2.current_bytes() <= cache2.max_bytes());
    assert_eq!(cache2.len(), 1);
    assert!(cache2.query_exact(20.0, 0.01).is_some());
    assert!(cache2.query_exact(5.0, 0.01).is_none());
}
