use pealayer::app::PealayerApp;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

static PLAYBACK_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_playback_tests() -> std::sync::MutexGuard<'static, ()> {
    PLAYBACK_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn wait_for_app_state(
    app: &mut PealayerApp,
    timeout: Duration,
    mut predicate: impl FnMut(&PealayerApp) -> bool,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        app.process_events();
        if predicate(app) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn test_app_playback_finish_replay_and_seek() {
    let _lock = lock_playback_tests();
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
    assert!(
        app.is_paused,
        "Video should be paused when playback finishes"
    );

    // 1. Test seeking backwards from finished state
    let expected_seek_time = (app.playback_time - 5.0).clamp(0.0, app.duration);
    app.seek_relative(-5.0);
    assert!(
        wait_for_app_state(&mut app, Duration::from_secs(3), |app| {
            !app.is_playback_finished() && (app.playback_time - expected_seek_time).abs() < 1.0
        }),
        "Playback should leave the finished state and reach {expected_seek_time:.2}s after seeking back; last observed position was {:.2}s",
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
    assert!(
        app.is_playback_finished(),
        "Should reach finished state again"
    );

    // 3. Test replay at finished state
    app.toggle_playback();
    assert!(
        wait_for_app_state(&mut app, Duration::from_secs(3), |app| {
            !app.is_playback_finished() && !app.is_paused && app.playback_time < 2.0
        }),
        "Replay should restart playback near 0s; last observed position was {:.2}s (paused: {}, finished: {})",
        app.playback_time,
        app.is_paused,
        app.is_playback_finished()
    );
}

#[test]
fn test_app_playback_finish_scrub_and_move_around() {
    let _lock = lock_playback_tests();
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
    assert!(
        app.is_playback_finished(),
        "Playback should reach finished state"
    );

    // Scrub to 2.5s and verify mpv reports the committed position. The seek is
    // asynchronous and can take longer on resource-constrained CI runners.
    app.scrub_to(2.5);
    app.finish_scrub(2.5);
    assert!(
        wait_for_app_state(&mut app, Duration::from_secs(3), |app| {
            !app.is_playback_finished() && (app.playback_time - 2.5).abs() < 1.0
        }),
        "Playback should leave the finished state and settle near 2.5s; last observed position was {:.2}s",
        app.playback_time
    );

    // Now unpause and verify playback runs normally
    let playback_start = app.playback_time;
    app.play();
    assert!(!app.is_paused);
    assert!(
        wait_for_app_state(&mut app, Duration::from_secs(3), |app| {
            app.playback_time > playback_start + 0.1
        }),
        "Playback should progress from the scrubbed position; started at {playback_start:.2}s and last observed {:.2}s",
        app.playback_time
    );
}
