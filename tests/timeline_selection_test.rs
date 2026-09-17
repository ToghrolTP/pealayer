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

#[test]
fn test_timeline_zoom_scaling_and_clamping() {
    let mut app = PealayerApp::default();
    assert_eq!(app.timeline_zoom, 100.0);

    // Zoom clamping bounds [20.0, 500.0]
    let scroll_in = 5000.0;
    app.timeline_zoom = (app.timeline_zoom + scroll_in * 0.2).clamp(20.0, 500.0);
    assert_eq!(app.timeline_zoom, 500.0);

    let scroll_out = -5000.0;
    app.timeline_zoom = (app.timeline_zoom + scroll_out * 0.2).clamp(20.0, 500.0);
    assert_eq!(app.timeline_zoom, 20.0);

    // Coordinate conversion at zoom = 100.0
    app.timeline_zoom = 100.0;
    let total_seconds = 120.0;
    let relative_x = 250.0_f32;
    let target_time = ((relative_x / app.timeline_zoom) as f64).clamp(0.0, total_seconds);
    assert_eq!(target_time, 2.5);

    // Coordinate conversion at zoom = 200.0
    app.timeline_zoom = 200.0;
    let target_time_zoomed = ((relative_x / app.timeline_zoom) as f64).clamp(0.0, total_seconds);
    assert_eq!(target_time_zoomed, 1.25);
}
