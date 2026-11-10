use crate::four_d::curve::AnalogTrack;
use crate::four_d::models::{Effect, EffectInstance, TimelineKeyframe, TimelineTrackState};

/// Represents a point-in-time state of user-editable timeline elements.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TimelineSnapshot {
    pub instances: Vec<EffectInstance>,
    pub analog_tracks: Vec<AnalogTrack>,
    pub templates: Vec<Effect>,
    pub keyframes: Vec<TimelineKeyframe>,
    pub track_states: std::collections::BTreeMap<String, TimelineTrackState>,
    pub track_muted: std::collections::BTreeSet<u8>,
    pub track_soloed: std::collections::BTreeSet<u8>,
    pub track_locked: std::collections::BTreeSet<u8>,
    pub media_target: Option<String>,
}

impl From<&crate::four_d::models::Timeline> for TimelineSnapshot {
    fn from(timeline: &crate::four_d::models::Timeline) -> Self {
        Self {
            instances: timeline.instances.clone(),
            analog_tracks: timeline.analog_tracks.clone(),
            templates: timeline.templates.clone(),
            keyframes: timeline.keyframes.clone(),
            track_states: timeline.track_states.clone(),
            track_muted: timeline.track_muted.clone(),
            track_soloed: timeline.track_soloed.clone(),
            track_locked: timeline.track_locked.clone(),
            media_target: timeline.media_target.clone(),
        }
    }
}

use std::collections::VecDeque;

/// Bounded undo/redo history stack.
#[derive(Debug, Clone)]
pub struct UndoStack {
    undo_stack: VecDeque<TimelineSnapshot>,
    redo_stack: VecDeque<TimelineSnapshot>,
    max_history: usize,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(50)
    }
}

impl UndoStack {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_history: max_history.max(1),
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_len(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo_stack.len()
    }

    pub fn push(&mut self, snapshot: TimelineSnapshot) {
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(snapshot);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: TimelineSnapshot) -> Option<TimelineSnapshot> {
        if let Some(prev) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(current);
            Some(prev)
        } else {
            None
        }
    }

    pub fn redo(&mut self, current: TimelineSnapshot) -> Option<TimelineSnapshot> {
        if let Some(next) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(current);
            Some(next)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
