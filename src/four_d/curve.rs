use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Interpolation algorithm used between keyframes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Interpolation {
    /// Holds the previous keyframe value until the next keyframe is reached.
    Step,
    /// Linearly ramps from current keyframe value to the next keyframe value.
    #[default]
    Linear,
    /// Smooth cubic S-curve (Hermite smoothstep) for organic transitions.
    Smooth,
}

/// A point along an analog curve with a normalized output value in [0.0, 1.0].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Keyframe {
    /// Position along the timeline in milliseconds
    pub time_ms: u64,
    /// Actuator intensity level normalized in range 0.0 ..= 1.0
    pub value: f32,
    /// Curve interpolation method applied toward the NEXT keyframe
    #[serde(default)]
    pub interpolation: Interpolation,
}

impl Keyframe {
    pub fn new(time_ms: u64, value: f32, interpolation: Interpolation) -> Self {
        Self {
            time_ms,
            value: value.clamp(0.0, 1.0),
            interpolation,
        }
    }
}

/// A continuous analog curve track mapped to a specific hardware actuator channel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalogTrack {
    /// Unique identifier for this analog track
    pub id: Uuid,
    /// Human-readable name (e.g., "Main Wind Turbine", "Subwoofer Shaker")
    pub name: String,
    /// Target hardware PWM channel index (0 to 15)
    pub channel: u8,
    /// Whether this track is enabled for output
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Whether this track is temporarily muted in playback
    #[serde(default)]
    pub muted: bool,
    /// Whether this track is armed for live motion capture recording
    #[serde(default)]
    pub armed: bool,
    /// Sorted list of keyframes defining the curve
    pub keyframes: Vec<Keyframe>,
}

fn default_true() -> bool {
    true
}

impl AnalogTrack {
    pub fn new(name: impl Into<String>, channel: u8) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            channel,
            enabled: true,
            muted: false,
            armed: false,
            keyframes: Vec::new(),
        }
    }

    /// Adds a keyframe, maintaining sorted chronological order.
    /// If a keyframe already exists at `time_ms`, it is replaced in-place.
    pub fn add_keyframe(&mut self, keyframe: Keyframe) {
        match self.keyframes.binary_search_by_key(&keyframe.time_ms, |k| k.time_ms) {
            Ok(idx) => {
                self.keyframes[idx] = keyframe;
            }
            Err(idx) => {
                self.keyframes.insert(idx, keyframe);
            }
        }
    }

    /// Removes a keyframe at the exact millisecond offset if present.
    pub fn remove_keyframe(&mut self, time_ms: u64) -> bool {
        if let Ok(idx) = self.keyframes.binary_search_by_key(&time_ms, |k| k.time_ms) {
            self.keyframes.remove(idx);
            true
        } else {
            false
        }
    }

    /// Evaluates the curve at the given millisecond timestamp.
    /// Returns normalized intensity in `[0.0, 1.0]`.
    pub fn evaluate(&self, time_ms: u64) -> f32 {
        if self.keyframes.is_empty() {
            return 0.0;
        }

        // Before first keyframe
        if time_ms <= self.keyframes[0].time_ms {
            return self.keyframes[0].value;
        }

        // After last keyframe
        let last_idx = self.keyframes.len() - 1;
        if time_ms >= self.keyframes[last_idx].time_ms {
            return self.keyframes[last_idx].value;
        }

        // Between two keyframes: binary search to find bounding slice
        let right_idx = match self.keyframes.binary_search_by_key(&time_ms, |k| k.time_ms) {
            Ok(idx) => return self.keyframes[idx].value,
            Err(idx) => idx,
        };

        let left = &self.keyframes[right_idx - 1];
        let right = &self.keyframes[right_idx];

        let dt = (right.time_ms - left.time_ms) as f32;
        if dt <= 0.0 {
            return left.value;
        }

        let progress = ((time_ms - left.time_ms) as f32 / dt).clamp(0.0, 1.0);

        match left.interpolation {
            Interpolation::Step => left.value,
            Interpolation::Linear => left.value + progress * (right.value - left.value),
            Interpolation::Smooth => {
                // Cubic Hermite smoothstep: 3*t^2 - 2*t^3
                let smooth_t = progress * progress * (3.0 - 2.0 * progress);
                left.value + smooth_t * (right.value - left.value)
            }
        }
    }

    /// Evaluates the curve and quantizes to 8-bit unsigned integer (0 ..= 255).
    pub fn evaluate_u8(&self, time_ms: u64) -> u8 {
        let val = self.evaluate(time_ms);
        (val * 255.0).round().clamp(0.0, 255.0) as u8
    }

    /// Evaluates the curve and quantizes to 12-bit unsigned integer (0 ..= 4095).
    pub fn evaluate_u16(&self, time_ms: u64) -> u16 {
        let val = self.evaluate(time_ms);
        (val * 4095.0).round().clamp(0.0, 4095.0) as u16
    }
}
