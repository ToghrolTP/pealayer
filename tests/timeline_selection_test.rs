use pealayer::app::PealayerApp;
use pealayer::four_d::curve::AnalogTrack;
use uuid::Uuid;

#[test]
fn test_app_state_multi_keyframe_selection() {
    let mut app = PealayerApp::default();
    let track_id = Uuid::new_v4();

    assert!(app.selected_keyframes.is_empty());
    assert_eq!(app.timeline_zoom, 100.0);

    // Insert multiple selections
    app.selected_keyframes.insert((track_id, 0));
    app.selected_keyframes.insert((track_id, 1));
    assert_eq!(app.selected_keyframes.len(), 2);

    // Snapshot and restore
    let initial_count = app.timeline.analog_tracks.len();
    let snapshot = app.snapshot_timeline();
    assert_eq!(snapshot.analog_tracks.len(), app.timeline.analog_tracks.len());

    let track = AnalogTrack::new("Extra Track", 2);
    app.timeline.analog_tracks.push(track);
    assert_eq!(app.timeline.analog_tracks.len(), initial_count + 1);

    app.restore_timeline_snapshot(snapshot);
    assert_eq!(app.timeline.analog_tracks.len(), initial_count);
}
