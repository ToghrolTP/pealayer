use pealayer::four_d::curve::{AnalogTrack, Interpolation};
use pealayer::four_d::curve_record::{RdpPoint, RecordingSession, simplify_rdp};

#[test]
fn test_rdp_collinear_points_reduced_to_endpoints() {
    // 5 collinear points along y = 0.5 * x
    let points = vec![
        RdpPoint { x: 0.0, y: 0.0 },
        RdpPoint { x: 100.0, y: 50.0 },
        RdpPoint { x: 200.0, y: 100.0 },
        RdpPoint { x: 300.0, y: 150.0 },
        RdpPoint { x: 400.0, y: 200.0 },
    ];
    let simplified = simplify_rdp(&points, 0.01);
    assert_eq!(simplified.len(), 2);
    assert_eq!(simplified[0].x, 0.0);
    assert_eq!(simplified[1].x, 400.0);
}

#[test]
fn test_rdp_preserves_sharp_peaks() {
    let points = vec![
        RdpPoint { x: 0.0, y: 0.0 },
        RdpPoint { x: 100.0, y: 0.0 },
        RdpPoint { x: 200.0, y: 1.0 }, // Peak
        RdpPoint { x: 300.0, y: 0.0 },
        RdpPoint { x: 400.0, y: 0.0 },
    ];
    let simplified = simplify_rdp(&points, 0.05);
    assert_eq!(simplified.len(), 5);
    assert_eq!(simplified[2].x, 200.0);
    assert_eq!(simplified[2].y, 1.0);
}

#[test]
fn test_recording_session_punch_in_and_commit() {
    let mut track = AnalogTrack::new("Wind Fan", 0);
    let track_id = track.id;

    let mut session = RecordingSession::new();
    session.record_sample(track_id, 1000, 0.0);
    session.record_sample(track_id, 1100, 0.25);
    session.record_sample(track_id, 1200, 0.50);
    session.record_sample(track_id, 1300, 0.75);
    session.record_sample(track_id, 1400, 1.0);

    assert_eq!(session.sample_count(track_id), 5);

    session.commit_to_track(&mut track, 0.02, Interpolation::Linear);

    // Collinear ramp from 1000 to 1400ms should be decimated to start & end keyframes
    assert_eq!(track.keyframes.len(), 2);
    assert_eq!(track.keyframes[0].time_ms, 1000);
    assert_eq!(track.keyframes[0].value, 0.0);
    assert_eq!(track.keyframes[1].time_ms, 1400);
    assert_eq!(track.keyframes[1].value, 1.0);

    // Buffer cleared after commit
    assert_eq!(session.sample_count(track_id), 0);
}
