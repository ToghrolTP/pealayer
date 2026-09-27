use pealayer::app::PealayerApp;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

#[test]
fn test_app_playback_finish_replay_and_seek() {
    let mut app = PealayerApp::default();
    let video_path = PathBuf::from("test-data/jellyfish.mp4");
    assert!(video_path.exists(), "test video must exist");

    app.load_video_file(video_path);

    // Process initial events
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(50));
        app.process_events();
        if app.duration > 0.0 {
            break;
        }
    }
    assert!(app.duration > 0.0, "Duration should be loaded");

    // Seek close to the end (e.g. 9.8s out of ~10.01s)
    app.scrub_to(app.duration - 0.2);
    app.finish_scrub(app.duration - 0.2);

    // Let it play to the end
    let mut reached_finish = false;
    for _ in 0..60 {
        thread::sleep(Duration::from_millis(50));
        app.process_events();
        if app.is_playback_finished() {
            reached_finish = true;
            break;
        }
    }
    assert!(reached_finish, "Playback should reach finished state");
    assert!(app.is_paused, "Video should be paused when playback finishes");

    // 1. Test seeking backwards from finished state
    app.seek_relative(-5.0);
    thread::sleep(Duration::from_millis(150));
    app.process_events();

    assert!(!app.is_playback_finished(), "Should no longer be in finished state after seeking back");
    assert!(
        app.playback_time >= 4.0 && app.playback_time <= 6.0,
        "Playback time after seek -5s should be around 5.0s, was {}",
        app.playback_time
    );

    // 2. Play from here to end again
    app.toggle_playback();
    assert!(!app.is_paused, "Should be playing after toggle_playback");

    // Wait until it finishes again
    for _ in 0..120 {
        thread::sleep(Duration::from_millis(50));
        app.process_events();
        if app.is_playback_finished() {
            break;
        }
    }
    assert!(app.is_playback_finished(), "Should reach finished state again");

    // 3. Test replay at finished state
    app.toggle_playback();
    thread::sleep(Duration::from_millis(200));
    app.process_events();

    assert!(!app.is_playback_finished(), "Should not be finished after replay");
    assert!(!app.is_paused, "Should be playing after replay");
    assert!(
        app.playback_time < 2.0,
        "Playback time after replay should be near 0, was {}",
        app.playback_time
    );
}

#[test]
fn test_app_playback_finish_scrub_and_move_around() {
    let mut app = PealayerApp::default();
    let video_path = PathBuf::from("test-data/jellyfish.mp4");
    assert!(video_path.exists(), "test video must exist");

    app.load_video_file(video_path);

    for _ in 0..20 {
        thread::sleep(Duration::from_millis(50));
        app.process_events();
        if app.duration > 0.0 {
            break;
        }
    }
    assert!(app.duration > 0.0);

    // Seek directly to near the end
    app.scrub_to(app.duration - 0.15);
    app.finish_scrub(app.duration - 0.15);

    for _ in 0..60 {
        thread::sleep(Duration::from_millis(50));
        app.process_events();
        if app.is_playback_finished() {
            break;
        }
    }
    assert!(app.is_playback_finished(), "Playback should reach finished state");

    // Scrub to 2.5s
    app.scrub_to(2.5);
    app.finish_scrub(2.5);
    thread::sleep(Duration::from_millis(150));
    app.process_events();

    assert!(!app.is_playback_finished(), "Should not be finished after scrub to 2.5s");
    let pos = app.seek_pos.unwrap_or(app.playback_time);
    assert!((pos - 2.5).abs() < 1.0, "Position should be near 2.5s, was {}", pos);

    // Now unpause and verify playback runs normally
    app.play();
    assert!(!app.is_paused);
    thread::sleep(Duration::from_millis(200));
    app.process_events();
    assert!(app.playback_time > 1.5, "Playback should progress from scrubbed position");
}
