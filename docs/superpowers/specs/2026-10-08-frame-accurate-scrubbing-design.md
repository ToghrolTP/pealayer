# Frame-Accurate Scrubbing and Decoded Frame Cache Design

## 1. Overview & Motivation

In non-linear editors (NLEs) and professional grading suites such as Adobe Premiere Pro and DaVinci Resolve, dragging the timeline playhead scrubs through absolute individual frames with near-zero latency. Video editors rely on this responsiveness to perform frame-precise cuts, inspect motion, and align visual keyframes with external effects.

In Pealayer, timeline scrubbing historically utilized `absolute+keyframes` seeks via `libmpv2` (`src/mpv/seek.rs`) during active drag gestures to keep the UI responsive, deferring exact seeks (`absolute+exact`) until mouse release. Because long-GOP video encodings (H.264, HEVC, AV1) often place keyframes (I-frames) several seconds apart, scrubbing across a clip causes the primary video display to jump coarsely or remain frozen until the drag ends.

This specification establishes an industry-standard, high-performance frame-accurate scrubbing subsystem in Pealayer. By combining a thread-safe in-memory decoded frame cache (L1 ring buffer), tuned asynchronous exact seeking with hardware frame-dropping in MPV, and an instant pseudo-frame fallback hierarchy, the primary viewport maintains locked, responsive visual feedback at up to 60 fps without freezing the UI or requiring lengthy offline transcoding.

---

## 2. Invariants & Requirements

1. **Primary Viewport Responsiveness**: While dragging the timeline seekbar, the primary video surface must continuously update. The viewport must never freeze, flash black, or clear its current frame while decoding is in flight.
2. **Immediate Exact Frame Presentation**: When the playhead matches a frame present in the in-memory cache, that frame must render in $< 1\text{ ms}$ without queuing a redundant decode command to MPV.
3. **Bounded Memory Ceiling**: The in-memory frame cache must operate under a strict, configurable memory ceiling (default: 128 MB). When memory exceeds the limit, frames furthest temporally from the current playhead position are evicted first.
4. **Hardware Decoder Protection**: Rapid mouse movement must not inundate the MPV command queue. Intermediate seek targets must be coalesced so the decoder only processes the latest requested position once ready.
5. **Deterministic Commit**: On mouse release (`finish_scrub`), the engine must dispatch an authoritative exact seek and synchronize audio/clock states once decoding is complete.
6. **Pealayer Alpha Contract Compliance**: No version-compatibility shims, legacy migrations, or fallback build branches. Errors must be typed and machine-readable; state must not be inferred from presentation strings.
7. **Git Isolation**: All implementation work must occur in an isolated feature branch (`feature/frame-accurate-scrubbing`) and be verified against automated tests prior to merging.

---

## 3. Architecture & Data Structures

```
[ User Drags Seek Bar ]
          │
          ▼
   SeekController (Debounced & Coalesced)
          │
          ├───► 1. Check L1 FrameCache (RAM)
          │          │
          │          ├──► [HIT] Display cached frame immediately (< 1 ms, 60 fps)
          │          │
          │          └──► [MISS]
          │                 ├─► A. Show nearest pseudo-frame (closest RAM frame or disk thumbnail)
          │                 └─► B. Dispatch async MPV exact scrub (hr-seek=yes, hr-framedrop=yes)
          │                            │
          ▼                            ▼
[ Video Viewport ] ◄─────── [ MPV Decodes Frame ] ───► Store into L1 FrameCache
```

### 3.1 `FrameCache` & `CachedFrame` (`src/mpv/cache.rs`)

A dedicated, thread-safe memory cache stores recently decoded frames.

```rust
use std::sync::Arc;

/// A decoded video frame retained in memory for instant scrub preview.
#[derive(Clone, Debug)]
pub struct CachedFrame {
    /// Presentation timestamp of the frame in seconds.
    pub pts: f64,
    /// Width of the frame in physical pixels.
    pub width: u32,
    /// Height of the frame in physical pixels.
    pub height: u32,
    /// RGBA8 pixel data.
    pub rgba: Arc<[u8]>,
    /// Estimated byte size of this frame in memory.
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
    /// Ordered frames sorted by presentation timestamp (PTS).
    frames: Vec<CachedFrame>,
    /// Maximum allowed memory footprint in bytes (e.g. 128 MB).
    max_bytes: usize,
    /// Current approximate memory usage in bytes.
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

    /// Query for a frame matching the target time within an acceptable frame tolerance.
    pub fn query_exact(&self, target_pts: f64, tolerance: f64) -> Option<CachedFrame> {
        self.frames
            .iter()
            .find(|f| (f.pts - target_pts).abs() <= tolerance)
            .cloned()
    }

    /// Find the temporally closest available frame to serve as a pseudo-frame fallback.
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

    /// Insert a newly decoded frame, evicting the furthest frames when exceeding memory budget.
    pub fn insert(&mut self, frame: CachedFrame, current_playhead: f64) {
        // Prevent duplicate insertion for identical PTS
        if let Some(pos) = self.frames.iter().position(|f| (f.pts - frame.pts).abs() < 0.001) {
            self.current_bytes -= self.frames[pos].byte_size;
            self.frames[pos] = frame.clone();
            self.current_bytes += frame.byte_size;
            return;
        }

        self.current_bytes += frame.byte_size;
        self.frames.push(frame);

        // Evict frames furthest from current_playhead when exceeding max_bytes
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

    /// Clear all frames upon media unload or file change.
    pub fn clear(&mut self) {
        self.frames.clear();
        self.current_bytes = 0;
    }
}
```

---

## 4. Seeking & Decoding Pipeline

### 4.1 High-Precision Scrub Backend (`src/mpv/seek.rs`)

`SeekMode::Scrub` commands are upgraded from `absolute+keyframes` to high-precision scrubbing:

1. **Decoder Configuration**:
   - `hr-seek = yes`: Directs MPV to decode to the precise requested timestamp rather than snapping to the previous keyframe.
   - `hr-seek-framedrop = yes`: Enables video decoder framedropping during seeks, discarding intermediate non-reference frames while advancing to the target PTS.
   - `video-sync = display-desync`: Disables waiting for display V-Sync during scrub decoding to minimize render pipeline latency.
2. **Execution in `MpvSeekBackend`**:
   ```rust
   match mode {
       SeekMode::Scrub => {
           let _ = self.mpv.command("seek", &[&t_str, "absolute+exact"]);
       }
       SeekMode::Commit => {
           let _ = self.mpv.command("seek", &[&t_str, "absolute+exact"]);
       }
   }
   ```
3. **Fast-Path Cache Interception**:
   Before submitting an expensive MPV command, `SeekController` queries the shared `FrameCache`. If `query_exact(target_time, tolerance)` returns a hit:
   - The cached frame is dispatched directly to the UI rendering state.
   - The MPV command is bypassed, conserving hardware decoder bandwidth.

---

## 5. Viewport Rendering & Pseudo-Frame Fallback

### 5.1 Presentation Hierarchy in `src/ui/video.rs`

During an active scrub session (`app.is_scrubbing == true`):

1. **Exact Frame (Tier 1)**:
   - If MPV emits a new decoded frame for the target time, it is rendered into `video_fbo` via `rc.0.render` and registered into `FrameCache`.
2. **L1 Pseudo-Frame (Tier 2)**:
   - If MPV has not yet completed decoding the target PTS, the viewport samples `FrameCache::query_nearest(target_time)`.
   - If a near frame is found within the local temporal window, it is rendered to the viewport canvas, providing continuous motion without pausing.
3. **L2 Disk Thumbnail Proxy (Tier 3)**:
   - If the playhead jumped across a large distance with no RAM cache hits, the viewport queries the pre-rendered seek thumbnail (`get_or_generate_seek_thumbnail` from `src/server/thumbnails.rs`) and renders the scaled image until MPV's exact frame arrives.
4. **FBO Retention**:
   - The OpenGL framebuffer `video_fbo` is never cleared to black during seeks. The last rendered content remains visible until the replacement frame is ready to paint.

---

## 6. Git Isolation & Workflow Strategy

1. **Branch Isolation**:
   - All development takes place on branch `feature/frame-accurate-scrubbing`.
   - The working tree remains isolated from `main` until full validation is complete.
2. **Atomic Milestones**:
   - **Milestone 1**: `FrameCache` unit module and memory management tests.
   - **Milestone 2**: `SeekController` integration (cache queries, coalescing, and exact scrub mode).
   - **Milestone 3**: `VideoSurface` rendering hook and pseudo-frame fallback presentation.
   - **Milestone 4**: End-to-end automated tests, performance benchmarks, and merge to `main`.

---

## 7. Testing & Verification Plan

### 7.1 Automated Unit Tests
- `FrameCache` boundary and eviction tests:
  - Verify exact query match within tolerance ($t \pm \frac{1}{2 \cdot \text{fps}}$).
  - Verify rejection outside tolerance window.
  - Verify nearest query returns the closest frame across forward and reverse seeks.
  - Verify memory usage remains $\le \text{max\_bytes}$ when inserting frames exceeding the ceiling.
  - Verify clear on media change.
- `SeekController` test suite:
  - Rapid seek coalescing: verify only the latest scrub request is dispatched when the backend is busy.
  - Mode transition: verify commit seeks supersede pending scrub seeks.

### 7.2 Integration & Alpha Contract Verification
- Run full automated test suite: `cargo test`.
- Ensure clean code formatting and linting: `cargo fmt --check` and `cargo clippy`.
- Validate that no legacy migration paths, deprecated field shims, or speculative version logic were introduced.
