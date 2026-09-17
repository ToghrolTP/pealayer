use std::time::Instant;

/// Captures physical and virtual inputs (keyboard, slider, gamepad) and normalizes to 0.0 ..= 1.0.
#[derive(Debug, Clone)]
pub struct InputCaptureState {
    pub current_throttle: f32,
    pub last_update: Instant,
    pub ramp_rate_per_sec: f32,
}

impl Default for InputCaptureState {
    fn default() -> Self {
        Self {
            current_throttle: 0.0,
            last_update: Instant::now(),
            ramp_rate_per_sec: 1.5, // Full stroke 0 -> 1 in ~0.67s
        }
    }
}

impl InputCaptureState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_throttle(&mut self, value: f32) {
        self.current_throttle = value.clamp(0.0, 1.0);
        self.last_update = Instant::now();
    }

    pub fn ramp_throttle(&mut self, delta: f32) {
        self.current_throttle = (self.current_throttle + delta).clamp(0.0, 1.0);
        self.last_update = Instant::now();
    }

    /// Updates throttle based on keyboard arrow/WASD inputs.
    pub fn update_from_keyboard(&mut self, up_pressed: bool, down_pressed: bool, dt_secs: f32) {
        if up_pressed && !down_pressed {
            self.ramp_throttle(self.ramp_rate_per_sec * dt_secs);
        } else if down_pressed && !up_pressed {
            self.ramp_throttle(-self.ramp_rate_per_sec * dt_secs);
        }
    }
}
