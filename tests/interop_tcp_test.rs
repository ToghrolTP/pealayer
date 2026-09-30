use pealayer::cli::send_remote_command;
use pealayer::platform::interop::{set_live_status, InteropCommand, PlayerStatusResponse};
use std::time::Duration;

#[test]
fn test_loopback_tcp_interop_commands_and_status() {
    unsafe {
        std::env::set_var("PEALAYER_PORT", "18085");
        std::env::set_var("PEALAYER_SOCKET_PATH", format!("/tmp/pealayer_tcp_{}.sock", std::process::id()));
    }
    let ctx = eframe::egui::Context::default();
    let application_identity =
        pealayer::config::resolved_app_name(&pealayer::config::AppConfig::load());
    let (_state_tx, rx) = pealayer::server::spawn_web_server_configured(
        18085,
        ctx,
        pealayer::server::WebRuntimeConfig::production(
            application_identity,
            "en".to_string(),
            "ltr".to_string(),
            "system".to_string(),
        ),
    );

    // Give background TCP listener a moment to bind
    std::thread::sleep(Duration::from_millis(150));

    // 1. Send JSON-RPC Play command
    let response = send_remote_command(r#"{"jsonrpc":"2.0","id":1,"method":"play"}"#)
        .expect("Failed to send JSON-RPC over unified HTTP IPC");

    let cmd = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("Did not receive Play command on channel");
    assert!(matches!(cmd, InteropCommand::Play));

    assert!(response.contains("\"result\":{\"status\":\"accepted\"}"));

    // 2. Set mock live status and query via get_status
    let mock_status = PlayerStatusResponse {
        status: "ok".to_string(),
        playing: true,
        volume: 85.0,
        playback_time: 12.34,
        duration: 100.0,
        current_video: Some("/movies/test.mp4".to_string()),
        ..PlayerStatusResponse::default()
    };
    set_live_status(mock_status);

    let status_line = send_remote_command(
        r#"{"jsonrpc":"2.0","id":2,"method":"get_status"}"#,
    )
    .expect("Failed to query status over unified HTTP IPC");
    assert!(status_line.contains("\"volume\":85.0"));
    assert!(status_line.contains("\"playback_time\":12.34"));
    assert!(status_line.contains("/movies/test.mp4"));
}
