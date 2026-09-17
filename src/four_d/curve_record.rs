use std::collections::HashMap;
use uuid::Uuid;
use crate::four_d::curve::{AnalogTrack, Interpolation, Keyframe};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RdpPoint {
    pub x: f64,
    pub y: f64,
}

/// Perpendicular distance from point p to line segment (a -> b)
fn perpendicular_distance(p: RdpPoint, a: RdpPoint, b: RdpPoint) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let length_sq = dx * dx + dy * dy;
    if length_sq == 0.0 {
        let px = p.x - a.x;
        let py = p.y - a.y;
        return (px * px + py * py).sqrt();
    }
    let num = (dy * p.x - dx * p.y + b.x * a.y - b.y * a.x).abs();
    num / length_sq.sqrt()
}

/// Simplifies a 2D curve using the Ramer-Douglas-Peucker (RDP) algorithm.
pub fn simplify_rdp(points: &[RdpPoint], epsilon: f64) -> Vec<RdpPoint> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut max_dist = 0.0;
    let mut max_idx = 0;
    let first = points[0];
    let last = points[points.len() - 1];

    for (i, &p) in points.iter().enumerate().skip(1).take(points.len() - 2) {
        let dist = perpendicular_distance(p, first, last);
        if dist > max_dist {
            max_dist = dist;
            max_idx = i;
        }
    }

    if max_dist > epsilon {
        let left = simplify_rdp(&points[..=max_idx], epsilon);
        let right = simplify_rdp(&points[max_idx..], epsilon);
        let mut result = left;
        result.pop(); // Remove duplicate junction point
        result.extend(right);
        result
    } else {
        vec![first, last]
    }
}

/// Manages high-frequency live recording buffers and keyframe decimation.
#[derive(Debug, Clone, Default)]
pub struct RecordingSession {
    /// In-flight recorded samples keyed by AnalogTrack ID
    pub buffers: HashMap<Uuid, Vec<(u64, f32)>>,
}

impl RecordingSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_sample(&mut self, track_id: Uuid, time_ms: u64, value: f32) {
        let buf = self.buffers.entry(track_id).or_default();
        buf.push((time_ms, value.clamp(0.0, 1.0)));
    }

    pub fn sample_count(&self, track_id: Uuid) -> usize {
        self.buffers.get(&track_id).map(|b| b.len()).unwrap_or(0)
    }

    pub fn clear(&mut self) {
        self.buffers.clear();
    }

    pub fn get_live_samples(&self, track_id: Uuid) -> Option<&[(u64, f32)]> {
        self.buffers.get(&track_id).map(|v| v.as_slice())
    }

    /// Decimates raw samples and commits keyframes into the target track.
    pub fn commit_to_track(
        &mut self,
        track: &mut AnalogTrack,
        epsilon: f64,
        interpolation: Interpolation,
    ) {
        if let Some(samples) = self.buffers.remove(&track.id) {
            if samples.is_empty() {
                return;
            }

            let rdp_points: Vec<RdpPoint> = samples
                .iter()
                .map(|&(t, v)| RdpPoint {
                    x: t as f64,
                    y: v as f64 * 1000.0, // Scale value to match millisecond scale proportions
                })
                .collect();

            // Epsilon is scaled to match the normalized value scale (0.0..=1.0)
            let scaled_epsilon = epsilon * 1000.0;
            let simplified = simplify_rdp(&rdp_points, scaled_epsilon);

            let min_time = samples.iter().map(|s| s.0).min().unwrap_or(0);
            let max_time = samples.iter().map(|s| s.0).max().unwrap_or(0);

            // Remove existing keyframes within the recorded time range (punch-in replace)
            track.keyframes.retain(|k| k.time_ms < min_time || k.time_ms > max_time);

            for pt in simplified {
                let time_ms = pt.x.round().max(0.0) as u64;
                let value = (pt.y / 1000.0).clamp(0.0, 1.0) as f32;
                track.add_keyframe(Keyframe::new(time_ms, value, interpolation));
            }
        }
    }
}
