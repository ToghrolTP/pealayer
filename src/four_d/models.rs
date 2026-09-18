use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents the smallest unit of a command to a relay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtomicAction {
    /// The target relay ID (1 to 8)
    pub relay_id: u8,
    /// The state to set the relay to: true = ON, false = OFF
    pub state: bool,
    /// The exact millisecond offset from the start of the effect when this action should occur
    pub offset_ms: u64,
}

pub type Action = AtomicAction;

/// Hardware actuator target category for an effect template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HardwareTarget {
    Water,          // Relay 1 (Water Valve)
    Wind,           // Relay 2 (Wind Fan)
    SeatVibration,  // Relay 3 (Seat Vibration)
    Smoke,          // Relay 4 (Smoke Machine)
    Auxiliary,      // Relays 5..=8 (Aux Triggers)
    Any,            // Unconstrained cues
}

impl HardwareTarget {
    pub fn primary_relay_id(&self) -> Option<u8> {
        match self {
            Self::Water => Some(1),
            Self::Wind => Some(2),
            Self::SeatVibration => Some(3),
            Self::Smoke => Some(4),
            Self::Auxiliary => Some(5),
            Self::Any => None,
        }
    }

    pub fn is_compatible_with_relay(&self, relay_id: u8) -> bool {
        match self {
            Self::Water => relay_id == 1 || (5..=8).contains(&relay_id),
            Self::Wind => relay_id == 2 || (5..=8).contains(&relay_id),
            Self::SeatVibration => relay_id == 3 || (5..=8).contains(&relay_id),
            Self::Smoke => relay_id == 4 || (5..=8).contains(&relay_id),
            Self::Auxiliary => (5..=8).contains(&relay_id),
            Self::Any => (1..=8).contains(&relay_id),
        }
    }

    pub fn for_relay(relay_id: u8) -> Self {
        match relay_id {
            1 => Self::Water,
            2 => Self::Wind,
            3 => Self::SeatVibration,
            4 => Self::Smoke,
            5..=8 => Self::Auxiliary,
            _ => Self::Any,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Water => "Water Valve",
            Self::Wind => "Wind Fan",
            Self::SeatVibration => "Seat Vibration",
            Self::Smoke => "Smoke Machine",
            Self::Auxiliary => "Aux Relay",
            Self::Any => "General Cue",
        }
    }
}

pub fn default_hardware_target() -> HardwareTarget {
    HardwareTarget::Any
}

/// A reusable template or macro defining a sequence of actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effect {
    /// Unique identifier for this effect template
    pub id: Uuid,
    /// Human-readable name (e.g., "Water Splash")
    pub name: String,
    /// Path or identifier for the UI icon
    pub icon: String,
    /// Total duration of the effect in milliseconds
    pub duration_ms: u64,
    /// Hardware actuator target type for track compatibility
    #[serde(default = "default_hardware_target")]
    pub target: HardwareTarget,
    /// List of actions that make up this effect
    pub actions: Vec<AtomicAction>,
}

impl Effect {
    pub fn new(name: String, icon: String, duration_ms: u64, actions: Vec<AtomicAction>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            icon,
            duration_ms,
            target: HardwareTarget::Any,
            actions,
        }
    }

    pub fn with_target(name: String, icon: String, duration_ms: u64, target: HardwareTarget, actions: Vec<AtomicAction>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            icon,
            duration_ms,
            target,
            actions,
        }
    }
}

/// A specific placement of an Effect on the main timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectInstance {
    /// Unique identifier for this instance on the timeline
    pub id: Uuid,
    /// Reference to the template Effect
    pub effect_id: Uuid,
    /// The start time in milliseconds relative to the start of the video
    pub start_time_ms: u64,
}

impl EffectInstance {
    pub fn new(effect_id: Uuid, start_time_ms: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            effect_id,
            start_time_ms,
        }
    }
}

/// The entire sequence of effects programmed for a video.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    /// The specific instances placed on the timeline
    pub instances: Vec<EffectInstance>,
    /// Available effect templates in this project
    pub templates: Vec<Effect>,
    /// Continuous analog curve tracks (e.g. PWM fan curves, rumblers)
    #[serde(default)]
    pub analog_tracks: Vec<crate::four_d::curve::AnalogTrack>,
}

impl Default for Timeline {
    fn default() -> Self {
        Self {
            instances: Vec::new(),
            templates: Vec::new(),
            analog_tracks: vec![
                crate::four_d::curve::AnalogTrack::new("Wind Turbine", 0),
                crate::four_d::curve::AnalogTrack::new("Seat Rumble", 1),
            ],
        }
    }
}

impl Timeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let timeline = serde_json::from_reader(reader)?;
        Ok(timeline)
    }

    pub fn save_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)?;
        Ok(())
    }
}
