#[cfg(target_os = "windows")]
use pealayer::app::PealayerApp;
use pealayer::platform::windows::{
    THUMB_BUTTON_PLAYPAUSE, TRAY_CMD_PLAYPAUSE, TaskbarProgressFlag, WINDOWS_QUICK_ACTIONS,
    compute_taskbar_state_with_error, thumbnail_button_tooltip, tray_menu_label,
};

#[test]
fn test_windows_shell_suite_end_to_end_states() {
    // 1. Taskbar state with error
    let state = compute_taskbar_state_with_error(0.0, 100.0, false, true);
    assert_eq!(state.to_progress_flag(), TaskbarProgressFlag::Error);

    // 2. Toolbar tooltips
    assert_eq!(
        thumbnail_button_tooltip(THUMB_BUTTON_PLAYPAUSE, true, false, false),
        "Play"
    );
    assert_eq!(
        thumbnail_button_tooltip(THUMB_BUTTON_PLAYPAUSE, false, false, false),
        "Pause"
    );

    // 3. Tray menu labels
    assert_eq!(tray_menu_label(TRAY_CMD_PLAYPAUSE, true), "Play");
    assert_eq!(tray_menu_label(TRAY_CMD_PLAYPAUSE, false), "Pause");
    assert!(
        WINDOWS_QUICK_ACTIONS
            .iter()
            .any(|action| action.title == "Preferences")
    );
}

#[test]
#[cfg(target_os = "windows")]
fn test_windows_shell_lifecycle_integration() {
    let mut app = PealayerApp::default();
    assert_eq!(app.window_handle, None);
    assert!(!app.shell_initialized);

    // Initial state update without window handle is a safe no-op
    app.update_shell_state();

    // Register a simulated HWND. Windows must reject it instead of claiming
    // that the tray/message hook was installed; the app will retry when the
    // real eframe HWND arrives. This app-owned lifecycle belongs to Windows;
    // non-Windows no-op contracts are exercised directly below, without an
    // unrelated native media-player lifecycle.
    app.window_handle = Some(42);
    app.ensure_shell_initialized();
    assert!(!app.shell_initialized);

    // A retry with the same invalid handle must remain safe and truthful.
    app.ensure_shell_initialized();
    assert!(!app.shell_initialized);

    // State update with playback time, duration, paused, and error
    app.playback_time = 15.0;
    app.duration = 60.0;
    app.is_paused = true;
    app.show_error = Some("Test playback error".to_string());
    app.update_shell_state();

    // Exit teardown
    let hwnd = app.window_handle.unwrap();
    let _ = pealayer::platform::windows::remove_system_tray_icon(hwnd);
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_non_windows_shell_shims_are_safe_no_ops() {
    use pealayer::platform::windows;

    // Do not construct PealayerApp/libmpv merely to check Windows-only shims.
    // macOS/Linux playback and native-client lifecycles have separate coverage.
    for hwnd in [0, 42] {
        assert!(windows::install_shell_message_hook(hwnd).is_ok());
        assert!(
            windows::init_taskbar_thumbnail_toolbar(hwnd, true, false, false, false, true).is_ok()
        );
        assert!(windows::register_system_tray_icon(hwnd, "Shell fixture").is_ok());
        assert!(
            windows::update_taskbar_thumbnail_buttons(hwnd, false, true, false, true, true).is_ok()
        );
        assert!(windows::update_video_taskbar_thumbnail(hwnd, None).is_ok());
        windows::update_windows_taskbar_state_ext(15.0, 60.0, true, true);
        assert!(windows::remove_system_tray_icon(hwnd).is_ok());
    }
}
