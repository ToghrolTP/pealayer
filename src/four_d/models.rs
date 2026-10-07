use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn nonzero_relay<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    let id = u8::deserialize(deserializer)?;
    if id == 0 {
        Err(serde::de::Error::custom("relay ID must be non-zero"))
    } else {
        Ok(id)
    }
}

/// Represents the smallest unit of a command to a relay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtomicAction {
    /// The non-zero relay ID advertised by the active PCController capability catalog.
    #[serde(deserialize_with = "nonzero_relay")]
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
    Relay(#[serde(deserialize_with = "nonzero_relay")] u8),
    ControllerMacro,
}

impl HardwareTarget {
    pub fn primary_relay_id(&self) -> Option<u8> {
        match self {
            Self::Any | Self::ControllerMacro => None,
            Self::Relay(relay_id) => (*relay_id != 0).then_some(*relay_id),
        }
    }

    pub fn is_compatible_with_relay(&self, relay_id: u8) -> bool {
        match self {
            Self::Any => relay_id != 0,
            Self::Relay(target_id) => relay_id != 0 && *target_id == relay_id,
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

/// A directly-authored value held by one advertised hardware channel for the
/// cue's visible interval. Unlike a recorded/controller-owned sequence this
/// has no intrinsic program length, so its timeline placement may be resized.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectControlCue {
    /// Stable PCController capability key, for example `relay.5` or `pwm.10`.
    pub control_key: String,
    /// Exact normalized value in basis points (0..=10_000).
    pub value_basis_points: u16,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CueDurationPolicy {
    /// Infer from the cue's authoritative source.
    #[default]
    Auto,
    /// The referenced program owns its duration; placement is move-only.
    Intrinsic,
    /// The timeline interval owns its duration and exposes resize handles.
    Resizable,
}

/// Capability-derived timeline lane for a PCController-owned effect.
///
/// This is presentation metadata, not a second effect definition: the living
/// sequence remains owned by PCController and Pealayer stores only its stable
/// reference plus the lane needed to keep an authored timeline readable while
/// the controller is temporarily offline.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum ControllerEffectLane {
    Motion,
    Relay,
    Pwm,
    Lighting,
    Display,
    Rf,
    Audio,
    #[default]
    Sequence,
    Composite,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller_lane: Option<ControllerEffectLane>,
    /// Optional direct channel value. These cues are authored and resized in
    /// Pealayer; recorded macros and lighting programs remain intrinsic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direct_control: Option<DirectControlCue>,
    #[serde(default)]
    pub duration_policy: CueDurationPolicy,
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
            controller_lane: None,
            direct_control: None,
            duration_policy: CueDurationPolicy::Auto,
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
            controller_lane: None,
            direct_control: None,
            duration_policy: CueDurationPolicy::Auto,
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
            controller_lane: Some(ControllerEffectLane::Sequence),
            direct_control: None,
            duration_policy: CueDurationPolicy::Intrinsic,
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
            controller_lane: Some(ControllerEffectLane::Lighting),
            direct_control: None,
            duration_policy: CueDurationPolicy::Intrinsic,
        }
    }

    /// Whether this placement represents a sustained value whose interval is
    /// authored on the media timeline. Controller-owned recordings and strip
    /// programs carry their own timing and therefore are move-only.
    pub fn duration_resizable(&self) -> bool {
        match self.duration_policy {
            CueDurationPolicy::Intrinsic => false,
            CueDurationPolicy::Resizable => true,
            CueDurationPolicy::Auto => {
                self.controller_macro.is_none() && self.controller_strip_effect.is_none()
            }
        }
    }

    pub fn direct_control(
        name: String,
        icon: String,
        duration_ms: u64,
        control_key: String,
        value_basis_points: u16,
        relay_id: Option<u8>,
    ) -> Self {
        let normalized = value_basis_points.min(10_000);
        let actions = relay_id
            .filter(|id| *id != 0)
            .map(|relay_id| vec![AtomicAction {
                relay_id,
                state: normalized >= 5_000,
                offset_ms: 0,
            }])
            .unwrap_or_default();
        Self {
            id: Uuid::new_v4(),
            name,
            icon,
            duration_ms: duration_ms.max(100),
            target: relay_id.map(HardwareTarget::for_relay).unwrap_or(HardwareTarget::Any),
            actions,
            controller_macro: None,
            controller_strip_effect: None,
            controller_lane: None,
            direct_control: Some(DirectControlCue {
                control_key,
                value_basis_points: normalized,
            }),
            duration_policy: CueDurationPolicy::Resizable,
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

/// Per-project presentation and routing state for one discovered timeline
/// track. The map key is the stable media or PCController channel identity;
/// captions may be renamed without breaking the association.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TimelineTrackState {
    /// Linked tracks participate in this project's timeline. Unlinking keeps
    /// authored data intact but removes the live association until re-linked.
    pub linked: bool,
    /// Hidden tracks remain linked and operational but do not consume a row.
    pub visible: bool,
}

impl Default for TimelineTrackState {
    fn default() -> Self {
        Self {
            linked: true,
            visible: true,
        }
    }
}

pub fn hardware_timeline_track_key(channel_key: &str) -> String {
    format!("hardware:{}", channel_key.trim())
}

pub fn controller_effect_timeline_track_key(lane: ControllerEffectLane) -> String {
    let suffix = match lane {
        ControllerEffectLane::Motion => "motion",
        ControllerEffectLane::Relay => "relay",
        ControllerEffectLane::Pwm => "pwm",
        ControllerEffectLane::Lighting => "lighting",
        ControllerEffectLane::Display => "display",
        ControllerEffectLane::Rf => "rf",
        ControllerEffectLane::Audio => "audio",
        ControllerEffectLane::Sequence => "sequence",
        ControllerEffectLane::Composite => "composite",
    };
    format!("controller-effect:{suffix}")
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Stable link/visibility policy for media, controller-effect, and hardware
    /// tracks. Missing entries deliberately mean linked and visible so a
    /// partially authored current project remains usable and self-correcting.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub track_states: std::collections::BTreeMap<String, TimelineTrackState>,
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
            track_states: std::collections::BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_relay_ids_are_rejected_even_from_imported_data() {
        assert!(serde_json::from_str::<HardwareTarget>(r#"{"Relay":0}"#).is_err());
        assert!(serde_json::from_str::<AtomicAction>(r#"{"relay_id":0,"state":true,"offset_ms":0}"#).is_err());
        assert_eq!(HardwareTarget::Relay(0).primary_relay_id(), None);
        assert!(!HardwareTarget::Relay(0).is_compatible_with_relay(0));
    }

    #[test]
    fn fresh_timeline_contains_no_invented_production_data() {
        let timeline = Timeline::default();
        assert!(timeline.instances.is_empty());
        assert!(timeline.templates.is_empty());
        assert!(timeline.analog_tracks.is_empty());
        assert!(timeline.keyframes.is_empty());
        assert!(timeline.track_states.is_empty());
    }

    #[test]
    fn recorded_programs_are_move_only_but_direct_values_are_resizable() {
        let recorded = Effect::controller_macro(
            "Recorded".into(),
            String::new(),
            2_000,
            7,
            "automatic".into(),
        );
        let lighting = Effect::controller_strip_effect("Lighting".into(), 3_000, "live-id".into());
        let relay = Effect::direct_control(
            "Relay on".into(),
            String::new(),
            1_000,
            "relay.5".into(),
            10_000,
            Some(5),
        );
        assert!(!recorded.duration_resizable());
        assert!(!lighting.duration_resizable());
        assert!(relay.duration_resizable());
        assert_eq!(relay.actions[0].state, true);
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

    #[test]
    fn missing_track_preferences_remain_linked_and_visible() {
        let timeline: Timeline = serde_json::from_str(
            r#"{"instances":[],"templates":[],"analog_tracks":[],"keyframes":[]}"#,
        )
        .expect("current timeline JSON may omit default track preferences");
        assert_eq!(
            timeline.track_state("hardware:relay.5"),
            TimelineTrackState::default()
        );
    }

    #[test]
    fn per_track_link_and_visibility_round_trip() {
        let mut timeline = Timeline::default();
        assert!(timeline.set_track_linked("hardware:relay.5", false));
        assert!(timeline.set_track_visible("media:subtitle:7", false));
        let encoded = serde_json::to_string(&timeline).expect("timeline should serialize");
        let decoded: Timeline =
            serde_json::from_str(&encoded).expect("timeline should deserialize");
        assert!(!decoded.track_state("hardware:relay.5").linked);
        assert!(!decoded.track_state("media:subtitle:7").visible);
        assert_eq!(hardware_timeline_track_key("pwm.12"), "hardware:pwm.12");
    }

    #[test]
    fn controller_cue_session_keeps_only_stable_controller_references() {
        let controller = Effect::controller_macro(
            "Relay 8 - one second".to_string(),
            "sparkle".to_string(),
            1_000,
            8,
            "host".to_string(),
        );
        let controller_id = controller.id;
        let local = Effect::with_target(
            "Local-only draft".to_string(),
            String::new(),
            500,
            HardwareTarget::Relay(5),
            vec![AtomicAction {
                relay_id: 5,
                state: true,
                offset_ms: 0,
            }],
        );
        let local_id = local.id;
        let mut timeline = Timeline::default();
        timeline.templates.extend([controller, local]);
        timeline
            .instances
            .push(EffectInstance::new(controller_id, 10_000));
        timeline
            .instances
            .push(EffectInstance::new(local_id, 20_000));
        timeline.set_track_visible("controller-effect:relay", false);
        timeline.set_track_visible("hardware:relay.5", false);
        timeline.add_keyframe(10_000);

        let session = timeline.controller_cue_session();

        assert!(session.has_controller_cues());
        assert_eq!(session.instances.len(), 1);
        assert_eq!(session.instances[0].start_time_ms, 10_000);
        assert_eq!(session.templates.len(), 1);
        assert_eq!(
            session.templates[0].controller_macro.as_ref().unwrap().id,
            8
        );
        assert!(session.templates[0].actions.is_empty());
        assert_eq!(
            session.track_states.keys().cloned().collect::<Vec<_>>(),
            vec!["controller-effect:relay".to_string()]
        );
        assert!(session.keyframes.is_empty());
    }
}

impl Timeline {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the durable, media-scoped portion required to restore
    /// PCController-owned cues after a Pealayer restart.
    ///
    /// PCController remains the program owner. These retained templates hold
    /// only stable controller references plus cached presentation metadata;
    /// controller program steps are never copied into Pealayer's config.
    pub fn controller_cue_session(&self) -> Self {
        let controller_template_ids = self
            .templates
            .iter()
            .filter(|template| {
                template.controller_macro.is_some() || template.controller_strip_effect.is_some()
            })
            .map(|template| template.id)
            .collect::<std::collections::BTreeSet<_>>();
        let instances = self
            .instances
            .iter()
            .filter(|instance| controller_template_ids.contains(&instance.effect_id))
            .cloned()
            .collect::<Vec<_>>();
        let referenced_template_ids = instances
            .iter()
            .map(|instance| instance.effect_id)
            .collect::<std::collections::BTreeSet<_>>();
        let templates = self
            .templates
            .iter()
            .filter(|template| referenced_template_ids.contains(&template.id))
            .cloned()
            .map(|mut template| {
                template.actions.clear();
                template
            })
            .collect();
        let track_states = self
            .track_states
            .iter()
            .filter(|(key, _)| key.starts_with("controller-effect:"))
            .map(|(key, value)| (key.clone(), *value))
            .collect();

        Self {
            instances,
            templates,
            analog_tracks: Vec::new(),
            keyframes: Vec::new(),
            track_states,
        }
    }

    pub fn has_controller_cues(&self) -> bool {
        self.instances.iter().any(|instance| {
            self.templates.iter().any(|template| {
                template.id == instance.effect_id
                    && (template.controller_macro.is_some()
                        || template.controller_strip_effect.is_some())
            })
        })
    }

    pub fn track_state(&self, key: &str) -> TimelineTrackState {
        self.track_states.get(key).copied().unwrap_or_default()
    }

    pub fn set_track_linked(&mut self, key: impl Into<String>, linked: bool) -> bool {
        let key = key.into();
        let mut next = self.track_state(&key);
        if next.linked == linked {
            return false;
        }
        next.linked = linked;
        self.track_states.insert(key, next);
        true
    }

    pub fn set_track_visible(&mut self, key: impl Into<String>, visible: bool) -> bool {
        let key = key.into();
        let mut next = self.track_state(&key);
        if next.visible == visible {
            return false;
        }
        next.visible = visible;
        self.track_states.insert(key, next);
        true
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
