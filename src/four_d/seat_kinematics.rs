#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CinemaSeatKinematics {
    pub left_stroke: f32,
    pub right_stroke: f32,
    pub last_tick_ms: u64,
}

impl Default for CinemaSeatKinematics {
    fn default() -> Self {
        Self {
            left_stroke: 0.5,
            right_stroke: 0.5,
            last_tick_ms: 0,
        }
    }
}

impl CinemaSeatKinematics {
    /// Actuator full stroke travel speed (fraction of stroke per second).
    /// Full travel 0.0 to 1.0 takes 2.5 seconds (0.40 / sec).
    pub const SLEW_RATE_PER_SEC: f32 = 0.40;

    /// Maximum pitch rotation angle in degrees (+15° up / tilt back, -15° down / tilt forward).
    pub const MAX_PITCH_DEGREES: f32 = 15.0;

    /// Maximum roll rotation angle in degrees (+10° roll right, -10° roll left).
    pub const MAX_ROLL_DEGREES: f32 = 10.0;

    /// Updates actuator cylinder strokes based on active directions and elapsed time delta.
    ///
    /// - `dt_seconds`: Frame delta time in seconds, clamped to [0.0, 0.050] to avoid leap jumps.
    /// - `left_dir`: Normalized command direction for left actuator (-1.0 = down, 0.0 = stop, +1.0 = up).
    /// - `right_dir`: Normalized command direction for right actuator (-1.0 = down, 0.0 = stop, +1.0 = up).
    /// - `interlock_tripped`: If true (e.g. door open or E-STOP), motion is immediately frozen.
    pub fn update(&mut self, dt_seconds: f32, left_dir: f32, right_dir: f32, interlock_tripped: bool) {
        if interlock_tripped {
            return;
        }
        let dt = dt_seconds.clamp(0.0, 0.250);
        let left_delta = left_dir * Self::SLEW_RATE_PER_SEC * dt;
        let right_delta = right_dir * Self::SLEW_RATE_PER_SEC * dt;

        self.left_stroke = (self.left_stroke + left_delta).clamp(0.0, 1.0);
        self.right_stroke = (self.right_stroke + right_delta).clamp(0.0, 1.0);
    }

    /// Calculated Pitch angle in degrees: positive = tilt back, negative = tilt forward.
    #[inline]
    pub fn pitch_degrees(&self) -> f32 {
        ((self.left_stroke + self.right_stroke) * 0.5 - 0.5) * (Self::MAX_PITCH_DEGREES * 2.0)
    }

    /// Calculated Roll angle in degrees: positive = roll right, negative = roll left.
    #[inline]
    pub fn roll_degrees(&self) -> f32 {
        (self.right_stroke - self.left_stroke) * Self::MAX_ROLL_DEGREES
    }

    /// Pitch angle in radians for affine 2D transforms.
    #[inline]
    pub fn pitch_radians(&self) -> f32 {
        self.pitch_degrees().to_radians()
    }

    /// Roll angle in radians for affine 2D transforms.
    #[inline]
    pub fn roll_radians(&self) -> f32 {
        self.roll_degrees().to_radians()
    }
}
