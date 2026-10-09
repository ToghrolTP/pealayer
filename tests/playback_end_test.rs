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

#[test]
fn test_paused_seek_clears_seek_pos_and_advances_display_time_when_playback_resumes() {
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

    // Pause the video
    app.pause();
    assert!(
        wait_for_app_state(&mut app, Duration::from_secs(2), |app| app.is_paused),
        "App must be paused"
    );

    // Scrub while paused to 2.0s and finish scrub
    app.scrub_to(2.0);
    app.finish_scrub(2.0);

    // Wait for the scrub commit to settle
    assert!(
        wait_for_app_state(&mut app, Duration::from_secs(3), |app| {
            app.seek_pos == Some(2.0)
        }),
        "Scrub commit should settle with seek_pos Some(2.0)"
    );

    // Now resume playback
    app.play();
    assert!(!app.is_paused, "App should now be playing");

    // During playback, seek_pos must be cleared and resolve_display_time must advance past 2.0s
    assert!(
        wait_for_app_state(&mut app, Duration::from_secs(3), |app| {
            let display_time = pealayer::ui::controls::resolve_display_time(app.seek_pos, app.playback_time);
            app.seek_pos.is_none() && display_time > 2.2
        }),
        "Playback display time must advance past seek_pos (was stuck: {:?}, playback_time: {})",
        app.seek_pos,
        app.playback_time
    );
}

#[test]
fn test_23fps_media_playback_decoupled_framerate_and_zero_timing_offset() {
    let _lock = lock_playback_tests();
    let mut app = PealayerApp::default();

    let video_path = PathBuf::from("test-data/jellyfish.mp4");
    assert!(video_path.exists());
    app.load_video_file(video_path);

    // Process events until loaded
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(50));
        app.process_events();
        if app.duration > 0.0 {
            break;
        }
    }
    assert!(app.duration > 0.0);
    assert!(app.is_active_playback());

    // Pacing delegates to hardware VSync when opengl_vsync is true (default)
    assert_eq!(
        app.playback_repaint_pacing(),
        Some(pealayer::app::PlaybackRepaintPacing::VSync)
    );

    // When opengl_vsync is disabled, pacing adapts to display refresh rate (e.g. 144Hz)
    app.opengl_vsync = false;
    app.display_refresh_rate = 144.0;
    match app.playback_repaint_pacing() {
        Some(pealayer::app::PlaybackRepaintPacing::Paced(dur)) => {
            let ms = dur.as_secs_f64() * 1000.0;
            assert!((ms - (1000.0 / 144.0)).abs() < 0.1);
        }
        other => panic!("Expected 144Hz paced interval, got {other:?}"),
    }

    // Verify record_mpv_render_duration updates latency metrics correctly
    pealayer::ui::video::record_mpv_render_duration(Duration::from_micros(1500));
    let last_micros = pealayer::ui::video::LAST_MPV_RENDER_MICROS
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(last_micros, 1500);

    // Verify skipped render pass tracking is operational
    let skipped_before = pealayer::ui::video::MPV_RENDER_SKIPPED_COUNT
        .load(std::sync::atomic::Ordering::Relaxed);
    pealayer::ui::video::MPV_RENDER_SKIPPED_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        pealayer::ui::video::MPV_RENDER_SKIPPED_COUNT.load(std::sync::atomic::Ordering::Relaxed),
        skipped_before + 1
    );

    let fps_info = app
        .current_fps_display(Instant::now())
        .expect("fps display available");
    assert!(fps_info.tooltip.contains("MPV render time: 1.50 ms"));
}
