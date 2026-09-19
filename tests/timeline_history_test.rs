use pealayer::four_d::curve::AnalogTrack;
use pealayer::four_d::history::{TimelineSnapshot, UndoStack};
use pealayer::four_d::models::EffectInstance;
use uuid::Uuid;

#[test]
fn test_undo_redo_stack_basic_flow() {
    let mut stack = UndoStack::new(10);
    assert!(!stack.can_undo());
    assert!(!stack.can_redo());

    let state0 = TimelineSnapshot {
        instances: vec![],
        analog_tracks: vec![AnalogTrack::new("Wind", 0)],
        templates: vec![],
    };
    let state1 = TimelineSnapshot {
        instances: vec![],
        analog_tracks: vec![AnalogTrack::new("Wind", 0), AnalogTrack::new("Water", 1)],
        templates: vec![],
    };
    let state2 = TimelineSnapshot {
        instances: vec![],
        analog_tracks: vec![
            AnalogTrack::new("Wind", 0),
            AnalogTrack::new("Water", 1),
            AnalogTrack::new("Vibe", 2),
        ],
        templates: vec![],
    };

    stack.push(state0.clone());
    stack.push(state1.clone());
    assert!(stack.can_undo());
    assert!(!stack.can_redo());
    assert_eq!(stack.undo_len(), 2);
    assert_eq!(stack.redo_len(), 0);

    // Undo from state2 back to state1
    let undone = stack.undo(state2.clone()).expect("Should undo to state1");
    assert_eq!(undone.analog_tracks.len(), 2);
    assert!(stack.can_redo());
    assert_eq!(stack.undo_len(), 1);
    assert_eq!(stack.redo_len(), 1);

    // Redo back to state2
    let redone = stack.redo(undone).expect("Should redo to state2");
    assert_eq!(redone.analog_tracks.len(), 3);
    assert_eq!(stack.undo_len(), 2);
    assert_eq!(stack.redo_len(), 0);
}

#[test]
fn test_undo_stack_max_depth_cap() {
    let mut stack = UndoStack::new(2);
    for i in 0..5 {
        stack.push(TimelineSnapshot {
            instances: vec![],
            analog_tracks: vec![AnalogTrack::new(&format!("Track{}", i), i as u8)],
            templates: vec![],
        });
    }

    let cur = TimelineSnapshot {
        instances: vec![],
        analog_tracks: vec![],
        templates: vec![],
    };
    // Should be capped to 2 undos
    let u1 = stack.undo(cur).unwrap();
    assert_eq!(u1.analog_tracks[0].name, "Track4");
    let u2 = stack.undo(u1).unwrap();
    assert_eq!(u2.analog_tracks[0].name, "Track3");
    assert!(!stack.can_undo());
}

#[test]
fn test_undo_redo_with_effect_instances() {
    let mut stack = UndoStack::default();
    let effect_id = Uuid::new_v4();
    let instance = EffectInstance::new(effect_id, 1000);

    let state0 = TimelineSnapshot {
        instances: vec![],
        analog_tracks: vec![],
        templates: vec![],
    };
    let state1 = TimelineSnapshot {
        instances: vec![instance.clone()],
        analog_tracks: vec![],
        templates: vec![],
    };

    stack.push(state0.clone());
    let undone = stack.undo(state1.clone()).unwrap();
    assert_eq!(undone, state0);
    assert_eq!(undone.instances.len(), 0);

    let redone = stack.redo(undone).unwrap();
    assert_eq!(redone, state1);
    assert_eq!(redone.instances.len(), 1);
    assert_eq!(redone.instances[0].effect_id, effect_id);
}

#[test]
fn test_undo_stack_clear_and_redo_invalidation() {
    let mut stack = UndoStack::new(5);
    let state0 = TimelineSnapshot {
        instances: vec![],
        analog_tracks: vec![],
        templates: vec![],
    };
    let state1 = TimelineSnapshot {
        instances: vec![],
        analog_tracks: vec![AnalogTrack::new("TrackA", 0)],
        templates: vec![],
    };
    let state2 = TimelineSnapshot {
        instances: vec![],
        analog_tracks: vec![AnalogTrack::new("TrackB", 1)],
        templates: vec![],
    };

    stack.push(state0.clone());
    let _ = stack.undo(state1.clone()).unwrap();
    assert!(stack.can_redo());

    // Pushing a new state invalidates the redo stack
    stack.push(state2.clone());
    assert!(!stack.can_redo());
    assert!(stack.can_undo());

    stack.clear();
    assert!(!stack.can_undo());
    assert!(!stack.can_redo());
    assert_eq!(stack.undo(state0.clone()), None);
    assert_eq!(stack.redo(state0), None);
}
