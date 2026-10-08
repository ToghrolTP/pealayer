use pealayer::mpv::frame_cache::{CachedFrame, FrameCache};
use std::sync::{Arc, RwLock};

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
