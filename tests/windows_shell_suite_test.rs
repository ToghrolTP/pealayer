use pealayer::app::PealayerApp;
use pealayer::platform::windows::{
    THUMB_BUTTON_PLAYPAUSE, TRAY_CMD_PLAYPAUSE, TaskbarProgressFlag,
    compute_taskbar_state_with_error, thumbnail_button_tooltip, tray_menu_label,
};

#[test]
fn test_windows_shell_suite_end_to_end_states() {
    // 1. Taskbar state with error
    let state = compute_taskbar_state_with_error(0.0, 100.0, false, true);
    assert_eq!(state.to_progress_flag(), TaskbarProgressFlag::Error);

    // 2. Toolbar tooltips
    assert_eq!(
        thumbnail_button_tooltip(THUMB_BUTTON_PLAYPAUSE, true),
        "Play"
    );
    assert_eq!(
        thumbnail_button_tooltip(THUMB_BUTTON_PLAYPAUSE, false),
        "Pause"
    );

    // 3. Tray menu labels
    assert_eq!(tray_menu_label(TRAY_CMD_PLAYPAUSE, true), "Play");
    assert_eq!(tray_menu_label(TRAY_CMD_PLAYPAUSE, false), "Pause");
}

#[test]
fn test_windows_shell_lifecycle_integration() {
    let mut app = PealayerApp::default();
    assert_eq!(app.window_handle, None);
    assert!(!app.shell_initialized);

    // Initial state update without window handle is a safe no-op
    app.update_shell_state();

    // Register a simulated HWND. Windows must reject it instead of claiming
    // that the tray/message hook was installed; the app will retry when the
    // real eframe HWND arrives. Non-Windows shims remain successful no-ops.
    app.window_handle = Some(42);
    app.ensure_shell_initialized();
    #[cfg(target_os = "windows")]
    assert!(!app.shell_initialized);
    #[cfg(not(target_os = "windows"))]
    assert!(app.shell_initialized);

    // A retry with the same invalid handle must remain safe and truthful.
    app.ensure_shell_initialized();
    #[cfg(target_os = "windows")]
    assert!(!app.shell_initialized);
    #[cfg(not(target_os = "windows"))]
    assert!(app.shell_initialized);

    // State update with playback time, duration, paused, and error
    app.playback_time = 15.0;
    app.duration = 60.0;
    app.is_paused = true;
    app.show_error = Some("Test playback error".to_string());
    app.update_shell_state();

    // Exit teardown
    let hwnd = app.window_handle.unwrap();
    #[cfg(not(target_os = "windows"))]
    assert!(pealayer::platform::windows::remove_system_tray_icon(hwnd).is_ok());
    #[cfg(target_os = "windows")]
    let _ = pealayer::platform::windows::remove_system_tray_icon(hwnd);
}
