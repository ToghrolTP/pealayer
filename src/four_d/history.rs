use crate::four_d::curve::AnalogTrack;
use crate::four_d::models::EffectInstance;

/// Represents a point-in-time state of user-editable timeline elements.
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineSnapshot {
    pub instances: Vec<EffectInstance>,
    pub analog_tracks: Vec<AnalogTrack>,
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
