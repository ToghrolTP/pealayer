use pealayer::app::PealayerApp;
use pealayer::four_d::curve::{Interpolation, Keyframe};

#[test]
fn test_undo_redo_timeline_keyframe_deletion() {
    let mut app = PealayerApp::default();
    // Add keyframe to existing track 0
    app.timeline.analog_tracks[0].add_keyframe(Keyframe::new(1000, 0.5, Interpolation::Linear));

    // Save initial state
    let snap0 = app.snapshot_timeline();
    app.undo_stack.push(snap0);

    // Modify (delete keyframe)
    app.timeline.analog_tracks[0].keyframes.clear();
    assert_eq!(app.timeline.analog_tracks[0].keyframes.len(), 0);

    // Undo
    let snap1 = app.snapshot_timeline();
    let prev = app.undo_stack.undo(snap1).expect("Failed to undo");
    app.restore_timeline_snapshot(prev);
    assert_eq!(app.timeline.analog_tracks[0].keyframes.len(), 1);
    assert_eq!(app.timeline.analog_tracks[0].keyframes[0].time_ms, 1000);

    // Redo
    let snap2 = app.snapshot_timeline();
    if let Some(next) = app.undo_stack.redo(snap2) {
        app.restore_timeline_snapshot(next);
    }
    assert_eq!(app.timeline.analog_tracks[0].keyframes.len(), 0);
}
