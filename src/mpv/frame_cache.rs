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
        if let Some(pos) = self
            .frames
            .iter()
            .position(|f| (f.pts - frame.pts).abs() < 0.001)
        {
            self.current_bytes = self
                .current_bytes
                .saturating_sub(self.frames[pos].byte_size);
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
