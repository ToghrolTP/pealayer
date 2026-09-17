use crate::four_d::curve::AnalogTrack;
use crate::four_d::models::EffectInstance;

/// Represents a point-in-time state of user-editable timeline elements.
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineSnapshot {
    pub instances: Vec<EffectInstance>,
    pub analog_tracks: Vec<AnalogTrack>,
}

/// Bounded undo/redo history stack.
#[derive(Debug, Clone)]
pub struct UndoStack {
    undo_stack: Vec<TimelineSnapshot>,
    redo_stack: Vec<TimelineSnapshot>,
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
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: max_history.max(1),
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn push(&mut self, snapshot: TimelineSnapshot) {
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(snapshot);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: TimelineSnapshot) -> Option<TimelineSnapshot> {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(current);
            Some(prev)
        } else {
            None
        }
    }

    pub fn redo(&mut self, current: TimelineSnapshot) -> Option<TimelineSnapshot> {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(current);
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
