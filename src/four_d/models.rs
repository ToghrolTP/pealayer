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

/// A reusable template or macro defining a sequence of actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    /// Unique identifier for this effect template
    pub id: Uuid,
    /// Human-readable name (e.g., "Water Splash")
    pub name: String,
    /// Path or identifier for the UI icon
    pub icon: String,
    /// Total duration of the effect in milliseconds
    pub duration_ms: u64,
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
            actions,
        }
    }
}

/// A specific placement of an Effect on the main timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Timeline {
    /// The specific instances placed on the timeline
    pub instances: Vec<EffectInstance>,
    /// Available effect templates in this project
    pub templates: Vec<Effect>,
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
