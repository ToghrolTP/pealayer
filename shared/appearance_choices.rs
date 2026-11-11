use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UiAnimationMode {
    #[default]
    System,
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UiControlDensity {
    ExtraCompact,
    #[default]
    Compact,
    Regular,
    Comfortable,
    Spacious,
}
impl UiControlDensity {
    pub const ALL: [Self; 5] = [
        Self::ExtraCompact,
        Self::Compact,
        Self::Regular,
        Self::Comfortable,
        Self::Spacious,
    ];

    pub const fn base_height(self) -> f32 {
        match self {
            Self::ExtraCompact => 18.0,
            Self::Compact => 24.0,
            Self::Regular => 28.0,
            Self::Comfortable => 32.0,
            Self::Spacious => 36.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UiFontFamily {
    #[default]
    System,
    Bundled,
    Vazirmatn,
    Custom,
}
