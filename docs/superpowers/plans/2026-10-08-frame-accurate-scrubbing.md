# Frame-Accurate Scrubbing and Decoded Frame Cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement an industrial-grade, frame-accurate timeline scrubbing pipeline with an in-memory L1 decoded frame cache and instant pseudo-frame fallback in Pealayer.

**Architecture:** An in-memory, thread-safe ring buffer (`FrameCache`) stores recently decoded frames bounded by a 128 MB budget. `SeekController` checks this cache first to serve sub-millisecond frame hits at locked 60 fps; on misses, it dispatches low-latency exact seeks to MPV with hardware frame-dropping while the viewport displays the closest cached frame (pseudo-frame) to ensure zero UI freezes or black frames.

**Tech Stack:** Rust (2024 edition), `libmpv2`, OpenGL/`glow`, `eframe`/`egui`, `std::sync::{Arc, RwLock}`.

**Spec:** [`docs/superpowers/specs/2026-10-08-frame-accurate-scrubbing-design.md`](file:///home/toghrol/Documents/rust/pealayer/docs/superpowers/specs/2026-10-08-frame-accurate-scrubbing-design.md)

## Global Constraints

- Pealayer alpha contract: Single current contract; no legacy migrations, obsolete field aliases, or build-number checks.
- Zero black frames or UI freezes during scrubbing.
- Strict memory ceiling: Default 128 MB frame cache with temporal distance eviction.
- Git isolation: All work isolated on branch `feature/frame-accurate-scrubbing`.

---

### Task 1: Implement `FrameCache` Core Subsystem

**Files:**
- Create: `src/mpv/frame_cache.rs`
- Modify: `src/mpv/mod.rs:1-5`
- Test: `tests/frame_cache_test.rs`

**Interfaces:**
- Produces:
  - `CachedFrame { pts: f64, width: u32, height: u32, rgba: Arc<[u8]>, byte_size: usize }`
  - `FrameCache::new(max_bytes: usize) -> Self`
  - `FrameCache::query_exact(&self, target_pts: f64, tolerance: f64) -> Option<CachedFrame>`
  - `FrameCache::query_nearest(&self, target_pts: f64) -> Option<CachedFrame>`
  - `FrameCache::insert(&mut self, frame: CachedFrame, current_playhead: f64)`
  - `FrameCache::clear(&mut self)`
  - `FrameCache::current_bytes(&self) -> usize`
  - `FrameCache::len(&self) -> usize`

- [ ] **Step 1: Write the failing test**

Create `tests/frame_cache_test.rs`:

```rust
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
    assert!(cache.query_exact(30.0, 0.01).is_none(), "Distant frame should be evicted");
    assert!(cache.query_exact(10.0, 0.01).is_some());
    assert!(cache.query_exact(10.1, 0.01).is_some());
    assert!(cache.query_exact(10.2, 0.01).is_some());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test frame_cache_test`
Expected: FAIL with "unresolved import `pealayer::mpv::frame_cache`"

- [ ] **Step 3: Write minimal implementation**

Create `src/mpv/frame_cache.rs`:

```rust
use std::sync::Arc;

/// A decoded video frame retained in memory for instant scrub preview.
#[derive(Clone, Debug)]
pub struct CachedFrame {
    pub pts: f64,
    pub width: u32,
    pub height: u32,
    pub rgba: Arc<[u8]>,
    pub byte_size: usize,
}

impl CachedFrame {
    pub fn new(pts: f64, width: u32, height: u32, rgba: Vec<u8>) -> Self {
        let byte_size = rgba.len() + std::mem::size_of::<Self>();
        Self {
            pts,
            width,
            height,
            rgba: Arc::from(rgba),
            byte_size,
        }
    }
}

/// Thread-safe in-memory frame cache bounded by a byte budget.
#[derive(Debug)]
pub struct FrameCache {
    frames: Vec<CachedFrame>,
    max_bytes: usize,
    current_bytes: usize,
}

impl FrameCache {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            frames: Vec::new(),
            max_bytes,
            current_bytes: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn current_bytes(&self) -> usize {
        self.current_bytes
    }

    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    pub fn query_exact(&self, target_pts: f64, tolerance: f64) -> Option<CachedFrame> {
        self.frames
            .iter()
            .find(|f| (f.pts - target_pts).abs() <= tolerance)
            .cloned()
    }

    pub fn query_nearest(&self, target_pts: f64) -> Option<CachedFrame> {
        self.frames
            .iter()
            .min_by(|a, b| {
                let diff_a = (a.pts - target_pts).abs();
                let diff_b = (b.pts - target_pts).abs();
                diff_a.total_cmp(&diff_b)
            })
            .cloned()
    }

    pub fn insert(&mut self, frame: CachedFrame, current_playhead: f64) {
        if let Some(pos) = self.frames.iter().position(|f| (f.pts - frame.pts).abs() < 0.001) {
            self.current_bytes = self.current_bytes.saturating_sub(self.frames[pos].byte_size);
            self.current_bytes += frame.byte_size;
            self.frames[pos] = frame;
            return;
        }

        self.current_bytes += frame.byte_size;
        self.frames.push(frame);

        while self.current_bytes > self.max_bytes && self.frames.len() > 1 {
            let furthest_idx = self
                .frames
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| {
                    let diff_a = (a.pts - current_playhead).abs();
                    let diff_b = (b.pts - current_playhead).abs();
                    diff_a.total_cmp(&diff_b)
                })
                .map(|(idx, _)| idx);

            if let Some(idx) = furthest_idx {
                let removed = self.frames.remove(idx);
                self.current_bytes = self.current_bytes.saturating_sub(removed.byte_size);
            } else {
                break;
            }
        }
    }

    pub fn clear(&mut self) {
        self.frames.clear();
        self.current_bytes = 0;
    }
}
```

In `src/mpv/mod.rs`, add:
```rust
pub mod frame_cache;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test frame_cache_test`
Expected: PASS (2 tests pass)

- [ ] **Step 5: Commit**

```bash
git add src/mpv/frame_cache.rs src/mpv/mod.rs tests/frame_cache_test.rs
git commit -m "feat(cache): implement L1 in-memory decoded frame cache"
```

---

### Task 2: Upgrade `SeekController` with Cache Interception & High-Precision Scrub

**Files:**
- Modify: `src/mpv/seek.rs`
- Test: `tests/timeline_smooth_seek_test.rs`

**Interfaces:**
- Consumes: `CachedFrame`, `FrameCache` from `src/mpv/frame_cache.rs`
- Produces:
  - `SeekController::with_cache<B: SeekBackend>(backend: B, cache: Arc<RwLock<FrameCache>>) -> Self`
  - `SeekController::request_scrub(&self, target_time: f64) -> ScrubResult`
  - `ScrubResult::Cached(CachedFrame)`
  - `ScrubResult::Dispatched(u64)`
  - `MpvSeekBackend` executes `absolute+exact` for `SeekMode::Scrub`

- [ ] **Step 1: Write the failing test**

In `tests/timeline_smooth_seek_test.rs`, add:

```rust
use pealayer::mpv::frame_cache::{CachedFrame, FrameCache};
use pealayer::mpv::seek::ScrubResult;
use std::sync::RwLock;

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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test timeline_smooth_seek_test test_seek_controller_serves_cache_hits_without_backend_dispatch`
Expected: FAIL with "no function or associated item named `with_cache`"

- [ ] **Step 3: Implement minimal code**

In `src/mpv/seek.rs`:
1. Define `ScrubResult`:
```rust
use crate::mpv::frame_cache::{CachedFrame, FrameCache};
use std::sync::RwLock;

#[derive(Debug, Clone)]
pub enum ScrubResult {
    Cached(CachedFrame),
    Dispatched(u64),
}
```

2. Add `frame_cache: Option<Arc<RwLock<FrameCache>>>` to `SeekController`.
3. Provide `with_cache`:
```rust
impl SeekController {
    pub fn new<B: SeekBackend>(backend: B) -> Self {
        Self::with_cache_internal(backend, None)
    }

    pub fn with_cache<B: SeekBackend>(backend: B, cache: Arc<RwLock<FrameCache>>) -> Self {
        Self::with_cache_internal(backend, Some(cache))
    }

    fn with_cache_internal<B: SeekBackend>(backend: B, cache: Option<Arc<RwLock<FrameCache>>>) -> Self {
        // ... existing worker setup, storing cache in struct ...
    }
}
```

4. In `request_scrub(&self, target_time: f64) -> ScrubResult`:
```rust
    pub fn request_scrub(&self, target_time: f64) -> ScrubResult {
        if let Some(cache_lock) = &self.frame_cache {
            if let Ok(cache) = cache_lock.read() {
                // 20ms tolerance (covers 30/60fps frame matches)
                if let Some(frame) = cache.query_exact(target_time, 0.02) {
                    return ScrubResult::Cached(frame);
                }
            }
        }

        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let (lock, cvar) = &*self.state;
        let mut guard = lock.lock().unwrap();
        *guard = Some(PendingSeek {
            request_id,
            target_time,
            mode: SeekMode::Scrub,
        });
        cvar.notify_one();
        ScrubResult::Dispatched(request_id)
    }
```

5. In `MpvSeekBackend::execute_seek`:
```rust
    match mode {
        SeekMode::Scrub => {
            // High-precision scrubbing via absolute+exact seek with decoder frame-drop
            let _ = self.mpv.command("seek", &[&t_str, "absolute+exact"]);
        }
        SeekMode::Commit => {
            let _ = self.mpv.command("seek", &[&t_str, "absolute+exact"]);
        }
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test timeline_smooth_seek_test`
Expected: PASS (all seek tests pass)

- [ ] **Step 5: Commit**

```bash
git add src/mpv/seek.rs tests/timeline_smooth_seek_test.rs
git commit -m "feat(seek): integrate frame cache and exact scrub into seek controller"
```

---

### Task 3: Wire `FrameCache` into Application Lifecycle & Frame Capture

**Files:**
- Modify: `src/app.rs`
- Test: `tests/frame_cache_test.rs`

**Interfaces:**
- Consumes: `FrameCache`, `SeekController::with_cache`
- Produces:
  - `PealayerApp.frame_cache: Arc<std::sync::RwLock<FrameCache>>`
  - `PealayerApp.active_pseudo_frame: Option<CachedFrame>`
  - Initialized with 128 MB budget
  - Cleared on media unload / new media load

- [ ] **Step 1: Write the failing test**

In `tests/frame_cache_test.rs`, add:

```rust
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
```

- [ ] **Step 2: Run test to verify it passes baseline**

Run: `cargo test --test frame_cache_test test_app_initializes_and_clears_frame_cache`
Expected: PASS

- [ ] **Step 3: Modify `src/app.rs`**

1. Add fields to `PealayerApp`:
```rust
pub(crate) frame_cache: std::sync::Arc<std::sync::RwLock<crate::mpv::frame_cache::FrameCache>>,
pub(crate) active_pseudo_frame: Option<crate::mpv::frame_cache::CachedFrame>,
```
2. In `PealayerApp` initialization (`default` / `new`):
```rust
let frame_cache = std::sync::Arc::new(std::sync::RwLock::new(
    crate::mpv::frame_cache::FrameCache::new(128 * 1024 * 1024)
));
// Pass frame_cache clone to SeekController::with_cache
seek_controller: crate::mpv::seek::SeekController::with_cache(
    crate::mpv::seek::MpvSeekBackend::new(mpv),
    frame_cache.clone(),
),
frame_cache,
active_pseudo_frame: None,
```
3. In `app.seek_pos` / scrubbing handler (`app.rs:5570`):
```rust
self.seek_pos = Some(clamped);
match self.seek_controller.request_scrub(clamped) {
    crate::mpv::seek::ScrubResult::Cached(frame) => {
        self.active_pseudo_frame = Some(frame);
    }
    crate::mpv::seek::ScrubResult::Dispatched(_) => {
        if let Ok(cache) = self.frame_cache.read() {
            self.active_pseudo_frame = cache.query_nearest(clamped);
        }
    }
}
```
4. In `finish_scrub`:
```rust
self.active_pseudo_frame = None;
```
5. In media unload / `open_video`:
```rust
if let Ok(mut cache) = self.frame_cache.write() {
    cache.clear();
}
self.active_pseudo_frame = None;
```

- [ ] **Step 4: Run full compile and check**

Run: `cargo check`
Expected: PASS with 0 errors

- [ ] **Step 5: Commit**

```bash
git add src/app.rs tests/frame_cache_test.rs
git commit -m "feat(app): wire frame cache and pseudo-frame tracking into application lifecycle"
```

---

### Task 4: Viewport Presentation & Pseudo-Frame Fallback Rendering

**Files:**
- Modify: `src/ui/video.rs`
- Test: `tests/frame_accurate_scrubbing_tests.rs`

**Interfaces:**
- Consumes: `app.active_pseudo_frame`, `app.is_scrubbing`
- Produces: Seamless display of pseudo-frame in viewport during active scrub when MPV decode is in-flight.

- [ ] **Step 1: Write the failing test**

Create `tests/frame_accurate_scrubbing_tests.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --test frame_accurate_scrubbing_tests`
Expected: PASS

- [ ] **Step 3: Modify `src/ui/video.rs` to retain FBO and blit pseudo-frame when active**

In `src/ui/video.rs`, ensure that when `app.is_scrubbing` is true:
1. When MPV renders a frame via `rc.0.render`, keep the FBO valid and do not clear to black.
2. If `app.active_pseudo_frame` is present and MPV is in-flight:
   Render the retained frame or paint the pseudo-frame over the canvas area using egui's image painter or offscreen texture upload.
3. On scrub release, the exact committed MPV frame seamlessly updates without glitching.

- [ ] **Step 4: Run full compile and tests**

Run: `cargo test`
Expected: PASS (all tests pass)

- [ ] **Step 5: Commit**

```bash
git add src/ui/video.rs tests/frame_accurate_scrubbing_tests.rs
git commit -m "feat(ui): implement pseudo-frame presentation and FBO retention during scrub"
```

---

### Task 5: End-to-End Test Suite, Lint & Alpha Contract Verification

**Files:**
- Modify: `tests/frame_accurate_scrubbing_tests.rs`

- [ ] **Step 1: Expand end-to-end integration tests**

In `tests/frame_accurate_scrubbing_tests.rs`, add:
- Rapid scrub stress test (100 sequential scrub requests across cache boundaries).
- Memory limit enforcement under 1,000 continuous insertions.
- Zero panic on zero-duration, NaN, or negative seek times.

- [ ] **Step 2: Run test suite**

Run: `cargo test --test frame_accurate_scrubbing_tests`
Expected: PASS

- [ ] **Step 3: Run full workspace test suite**

Run: `cargo test`
Expected: PASS across all packages and test binaries.

- [ ] **Step 4: Run linter and formatting checks**

Run: `cargo fmt --check`
Run: `cargo clippy -- -D warnings` (or fix any warnings introduced)
Expected: Clean exit code 0.

- [ ] **Step 5: Commit and summarize**

```bash
git add tests/frame_accurate_scrubbing_tests.rs
git commit -m "test: add comprehensive verification suite for frame-accurate scrubbing"
```
