use pealayer::mpv::seek::{SeekBackend, SeekController, SeekMode};
use pealayer::ui::controls::resolve_display_time;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Clone, Default)]
struct MockSeekBackend {
    seeks: Arc<Mutex<Vec<(f64, SeekMode)>>>,
    delay: Option<Duration>,
}

impl SeekBackend for MockSeekBackend {
    fn execute_seek(&self, target_time: f64, mode: SeekMode) {
        if let Some(delay) = self.delay {
            thread::sleep(delay);
        }
        self.seeks.lock().unwrap().push((target_time, mode));
    }
}

#[test]
fn test_seek_controller_scrub_and_commit() {
    let backend = MockSeekBackend::default();
    let seeks = backend.seeks.clone();
    let controller = SeekController::new(backend);

    // Request scrub
    controller.request_scrub(10.5);
    thread::sleep(Duration::from_millis(50));

    // Request commit
    controller.request_commit(15.0);
    thread::sleep(Duration::from_millis(50));

    let executed = seeks.lock().unwrap().clone();
    assert!(!executed.is_empty(), "Expected at least one seek executed");

    // First should be scrub, last should be commit to 15.0
    let last = executed.last().unwrap();
    assert_eq!(last.0, 15.0);
    assert_eq!(last.1, SeekMode::Commit);
}

#[test]
fn test_seek_controller_coalesces_rapid_scrub_requests() {
    let backend = MockSeekBackend {
        seeks: Arc::new(Mutex::new(Vec::new())),
        delay: Some(Duration::from_millis(25)),
    };
    let seeks = backend.seeks.clone();
    let controller = SeekController::new(backend);

    // Send 10 rapid scrub requests within 10ms
    for i in 1..=10 {
        controller.request_scrub(i as f64);
        thread::sleep(Duration::from_millis(1));
    }

    // Wait for worker to finish processing
    thread::sleep(Duration::from_millis(150));

    let executed = seeks.lock().unwrap().clone();
    // Because the backend takes 25ms, 10 requests should coalesce to fewer than 10 seeks
    assert!(
        executed.len() < 10,
        "Expected coalescing to reduce seek count from 10, got {}",
        executed.len()
    );
    // The final executed seek must be the latest target (10.0)
    assert_eq!(executed.last().unwrap().0, 10.0);
}

#[test]
fn test_display_time_isolation_during_scrub() {
    let mut playback_time = 5.0;
    let mut seek_pos = None;

    // Normal playback
    assert_eq!(resolve_display_time(seek_pos, playback_time), 5.0);

    // User starts scrubbing to 25.0
    seek_pos = Some(25.0);
    assert_eq!(resolve_display_time(seek_pos, playback_time), 25.0);

    // Delayed playback_time arrives from old frames (e.g. 5.1, 5.2)
    playback_time = 5.2;
    // Display time MUST remain at scrub position 25.0, immune to delayed playback_time
    assert_eq!(resolve_display_time(seek_pos, playback_time), 25.0);

    // Scrub moves to 30.0
    seek_pos = Some(30.0);
    assert_eq!(resolve_display_time(seek_pos, playback_time), 30.0);

    // Scrub completes and settles
    seek_pos = None;
    playback_time = 30.0;
    assert_eq!(resolve_display_time(seek_pos, playback_time), 30.0);
}
