/// The twin draws the coordinator's two normalized applied positions. This
/// estimates pitch and roll for visualization without integrating motion in the UI.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CinemaSeatKinematics {
    pub left_stroke: f32,
    pub right_stroke: f32,
}

impl CinemaSeatKinematics {
    pub fn from_positions([left_stroke, right_stroke]: [f32; 2]) -> Self {
        Self {
            left_stroke,
            right_stroke,
        }
    }

    pub fn pitch_degrees(self) -> f32 {
        ((self.left_stroke + self.right_stroke) * 0.5 - 0.5) * 30.0
    }

    pub fn roll_degrees(self) -> f32 {
        (self.right_stroke - self.left_stroke) * 10.0
    }
}
