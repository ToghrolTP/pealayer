use eframe::egui;
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

#[test]
fn test_keyframe_drag_state_group_translation() {
    use pealayer::app::KeyframeDragState;
    use pealayer::four_d::curve::{Interpolation, Keyframe};

    let mut app = PealayerApp::default();
    let mut track1 = AnalogTrack::new("Track 1", 0);
    track1.add_keyframe(Keyframe::new(1000, 0.25, Interpolation::Linear));
    track1.add_keyframe(Keyframe::new(2000, 0.50, Interpolation::Linear));
    let t1_id = track1.id;

    let mut track2 = AnalogTrack::new("Track 2", 1);
    track2.add_keyframe(Keyframe::new(1500, 0.75, Interpolation::Smooth));
    let t2_id = track2.id;

    app.timeline.analog_tracks.push(track1);
    app.timeline.analog_tracks.push(track2);

    // Multi-select track1:kf0 and track2:kf0
    app.selected_keyframes.insert((t1_id, 0));
    app.selected_keyframes.insert((t2_id, 0));

    // Construct KeyframeDragState
    let group_originals = vec![
        (t1_id, 0, 1000, 0.25),
        (t2_id, 0, 1500, 0.75),
    ];

    let drag = KeyframeDragState {
        track_id: t1_id,
        keyframe_index: 0,
        start_pointer_pos: egui::pos2(100.0, 350.0),
        original_time_ms: 1000,
        original_value: 0.25,
        group_originals: group_originals.clone(),
    };
    app.active_keyframe_drag = Some(drag.clone());

    // Translate by +500ms and +0.20 value
    let time_delta = 500_i64;
    let val_delta = 0.20_f32;

    for &(tid, kid, orig_t, orig_v) in &drag.group_originals {
        let k_new_t = (orig_t as i64 + time_delta).max(0) as u64;
        let k_new_v = (orig_v + val_delta).clamp(0.0, 1.0);
        if let Some(t) = app.timeline.analog_tracks.iter_mut().find(|t| t.id == tid) {
            if let Some(k) = t.keyframes.get_mut(kid) {
                k.time_ms = k_new_t;
                k.value = k_new_v;
            }
        }
    }

    // Verify both keyframes moved together
    let t1 = app.timeline.analog_tracks.iter().find(|t| t.id == t1_id).unwrap();
    assert_eq!(t1.keyframes[0].time_ms, 1500);
    assert!((t1.keyframes[0].value - 0.45).abs() < 1e-4);

    let t2 = app.timeline.analog_tracks.iter().find(|t| t.id == t2_id).unwrap();
    assert_eq!(t2.keyframes[0].time_ms, 2000);
    assert!((t2.keyframes[0].value - 0.95).abs() < 1e-4);
}

#[test]
fn test_keyframe_magnetic_snapping_5px() {
    let zoom = 100.0_f32; // 100px per second = 0.1 px/ms
    let px_per_ms = zoom / 1000.0;
    let rect_min_x = 0.0_f32;

    let playback_time: f64 = 2.0; // 2000ms playhead
    let playhead_ms = (playback_time * 1000.0).round() as u64;

    // Test 1: Drag to 2040ms -> distance = 40ms * 0.1 = 4.0px (within 5px)
    let raw_t1 = 2040_u64;
    let nearest_sec1 = ((raw_t1 as f64 / 1000.0).round() as u64) * 1000;
    let play_x = rect_min_x + (playhead_ms as f32 * px_per_ms);
    let grid_x1 = rect_min_x + (nearest_sec1 as f32 * px_per_ms);
    let kf_x1 = rect_min_x + (raw_t1 as f32 * px_per_ms);

    let dist_play1 = (kf_x1 - play_x).abs();
    let dist_grid1 = (kf_x1 - grid_x1).abs();

    assert!(dist_play1 <= 5.0);
    assert_eq!(dist_play1, 4.0);
    let mut snapped_t1 = raw_t1;
    if dist_play1 <= 5.0 && dist_play1 <= dist_grid1 {
        snapped_t1 = playhead_ms;
    }
    assert_eq!(snapped_t1, 2000);

    // Test 2: Drag to 2080ms -> distance = 80ms * 0.1 = 8.0px (beyond 5px) -> no snap to playhead
    let raw_t2 = 2080_u64;
    let kf_x2 = rect_min_x + (raw_t2 as f32 * px_per_ms);
    let dist_play2 = (kf_x2 - play_x).abs();
    assert!(dist_play2 > 5.0);

    // Test 3: Snap to 1-second grid boundary (3000ms)
    // Drag to 2970ms -> distance = 30ms * 0.1 = 3.0px <= 5px -> snaps to 3000ms
    let raw_t3 = 2970_u64;
    let nearest_sec3 = ((raw_t3 as f64 / 1000.0).round() as u64) * 1000;
    let grid_x3 = rect_min_x + (nearest_sec3 as f32 * px_per_ms);
    let kf_x3 = rect_min_x + (raw_t3 as f32 * px_per_ms);
    let dist_grid3 = (kf_x3 - grid_x3).abs();

    assert!(dist_grid3 <= 5.0);
    assert_eq!(nearest_sec3, 3000);
}

#[test]
fn test_keyframe_drag_undo_snapshot_commit() {
    use pealayer::app::KeyframeDragState;
    use pealayer::four_d::curve::{Interpolation, Keyframe};

    let mut app = PealayerApp::default();
    let mut track = AnalogTrack::new("Volume", 0);
    track.add_keyframe(Keyframe::new(1000, 0.5, Interpolation::Linear));
    let tid = track.id;
    app.timeline.analog_tracks.push(track);

    let drag = KeyframeDragState {
        track_id: tid,
        keyframe_index: 0,
        start_pointer_pos: egui::pos2(100.0, 350.0),
        original_time_ms: 1000,
        original_value: 0.5,
        group_originals: vec![(tid, 0, 1000, 0.5)],
    };

    // Simulate keyframe moved to 2500ms, 0.9 value
    let t = app.timeline.analog_tracks.iter_mut().find(|t| t.id == tid).unwrap();
    t.keyframes[0].time_ms = 2500;
    t.keyframes[0].value = 0.9;

    // Commit pre-drag snapshot reconstructed from group_originals
    let mut pre_snap = app.snapshot_timeline();
    for &(ptid, pkid, orig_t, orig_v) in &drag.group_originals {
        if let Some(t) = pre_snap.analog_tracks.iter_mut().find(|t| t.id == ptid) {
            if let Some(k) = t.keyframes.get_mut(pkid) {
                k.time_ms = orig_t;
                k.value = orig_v;
            }
        }
    }
    app.undo_stack.push(pre_snap);

    assert!(app.undo_stack.can_undo());

    // Perform Undo
    let current_snap = app.snapshot_timeline();
    let restored = app.undo_stack.undo(current_snap).expect("Undo must succeed");
    app.restore_timeline_snapshot(restored);

    // Verify keyframe returned to original pre-drag state
    let t = app.timeline.analog_tracks.iter().find(|t| t.id == tid).unwrap();
    assert_eq!(t.keyframes[0].time_ms, 1000);
    assert_eq!(t.keyframes[0].value, 0.5);
}
