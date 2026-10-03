use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents the smallest unit of a command to a relay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtomicAction {
    /// The non-zero relay ID advertised by the active PCController capability catalog.
    pub relay_id: u8,
    /// The state to set the relay to: true = ON, false = OFF
    pub state: bool,
    /// The exact millisecond offset from the start of the effect when this action should occur
    pub offset_ms: u64,
}

pub type Action = AtomicAction;

/// Capability-derived output target for an effect template.
///
/// Relay identifiers are supplied by PCController or explicit project data;
/// Pealayer does not assign physical meanings or names to numeric outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HardwareTarget {
    Any,
    Relay(u8),
    ControllerMacro,
}

impl HardwareTarget {
    pub fn primary_relay_id(&self) -> Option<u8> {
        match self {
            Self::Any | Self::ControllerMacro => None,
            Self::Relay(relay_id) => Some(*relay_id),
        }
    }

    pub fn is_compatible_with_relay(&self, relay_id: u8) -> bool {
        match self {
            Self::Any => relay_id != 0,
            Self::Relay(target_id) => *target_id == relay_id,
            Self::ControllerMacro => false,
        }
    }

    pub fn for_relay(relay_id: u8) -> Self {
        (relay_id != 0)
            .then_some(Self::Relay(relay_id))
            .unwrap_or(Self::Any)
    }
}

/// Durable reference to a PCController-owned macro. Pealayer schedules the
/// reference against video while PCController retains execution timing and
/// hardware-specific steps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControllerMacroCue {
    pub id: u64,
    pub mode: String,
}

/// Durable reference to an effect advertised by PCController's addressable
/// strip catalog. The coordinator owns rendering and board transport; Pealayer
/// only schedules the stable ID and the authored cue duration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControllerStripEffectCue {
    pub id: String,
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
    /// Opaque controller-owned macro executed as one synchronized cue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller_macro: Option<ControllerMacroCue>,
    /// Opaque PCController strip-effect ID advertised by the active board.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller_strip_effect: Option<ControllerStripEffectCue>,
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
            controller_macro: None,
            controller_strip_effect: None,
        }
    }

    pub fn with_target(
        name: String,
        icon: String,
        duration_ms: u64,
        target: HardwareTarget,
        actions: Vec<AtomicAction>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            icon,
            duration_ms,
            target,
            actions,
            controller_macro: None,
            controller_strip_effect: None,
        }
    }

    pub fn controller_macro(
        name: String,
        icon: String,
        duration_ms: u64,
        macro_id: u64,
        mode: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            icon,
            duration_ms: duration_ms.max(1),
            target: HardwareTarget::ControllerMacro,
            actions: Vec::new(),
            controller_macro: Some(ControllerMacroCue { id: macro_id, mode }),
            controller_strip_effect: None,
        }
    }

    pub fn controller_strip_effect(name: String, duration_ms: u64, effect_id: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            icon: String::new(),
            duration_ms: duration_ms.max(1),
            target: HardwareTarget::ControllerMacro,
            actions: Vec::new(),
            controller_macro: None,
            controller_strip_effect: Some(ControllerStripEffectCue { id: effect_id }),
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

/// An exact timeline-wide timing anchor. Unlike an analog automation
/// keyframe, this does not change a hardware value; it is a named magnetic
/// guide shared by every cue lane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineKeyframe {
    pub id: Uuid,
    pub time_ms: u64,
    #[serde(default)]
    pub label: String,
}

impl TimelineKeyframe {
    pub fn new(time_ms: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            time_ms,
            label: String::new(),
        }
    }
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
    /// Timeline-wide exact timing guides used by cue snapping.
    #[serde(default)]
    pub keyframes: Vec<TimelineKeyframe>,
}

impl Default for Timeline {
    fn default() -> Self {
        Self {
            instances: Vec::new(),
            templates: Vec::new(),
            // A new production project is intentionally empty. Tracks are
            // created by project data or from hardware capabilities that were
            // actually advertised by PCController; example actuators must not
            // appear as if they were connected equipment.
            analog_tracks: Vec::new(),
            keyframes: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_timeline_contains_no_invented_production_data() {
        let timeline = Timeline::default();
        assert!(timeline.instances.is_empty());
        assert!(timeline.templates.is_empty());
        assert!(timeline.analog_tracks.is_empty());
        assert!(timeline.keyframes.is_empty());
    }

    #[test]
    fn exact_timeline_keyframes_are_sorted_deduplicated_and_editable() {
        let mut timeline = Timeline::default();
        let later = timeline.add_keyframe(2_500);
        let earlier = timeline.add_keyframe(750);
        assert_eq!(timeline.add_keyframe(750), earlier);
        assert_eq!(
            timeline
                .keyframes
                .iter()
                .map(|keyframe| keyframe.time_ms)
                .collect::<Vec<_>>(),
            vec![750, 2_500]
        );
        assert!(timeline.move_keyframe(later, 1_250));
        assert!(!timeline.move_keyframe(later, 750));
        assert!(timeline.remove_keyframe(earlier));
        assert_eq!(timeline.keyframes[0].time_ms, 1_250);
    }
}

impl Timeline {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one exact timeline guide, deduplicating by millisecond and keeping
    /// the serialized collection stable and chronological.
    pub fn add_keyframe(&mut self, time_ms: u64) -> Uuid {
        match self
            .keyframes
            .binary_search_by_key(&time_ms, |keyframe| keyframe.time_ms)
        {
            Ok(index) => self.keyframes[index].id,
            Err(index) => {
                let keyframe = TimelineKeyframe::new(time_ms);
                let id = keyframe.id;
                self.keyframes.insert(index, keyframe);
                id
            }
        }
    }

    pub fn move_keyframe(&mut self, id: Uuid, time_ms: u64) -> bool {
        let Some(index) = self.keyframes.iter().position(|keyframe| keyframe.id == id) else {
            return false;
        };
        if self
            .keyframes
            .iter()
            .any(|keyframe| keyframe.id != id && keyframe.time_ms == time_ms)
        {
            return false;
        }
        self.keyframes[index].time_ms = time_ms;
        self.keyframes.sort_by_key(|keyframe| keyframe.time_ms);
        true
    }

    pub fn remove_keyframe(&mut self, id: Uuid) -> bool {
        let before = self.keyframes.len();
        self.keyframes.retain(|keyframe| keyframe.id != id);
        self.keyframes.len() != before
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
