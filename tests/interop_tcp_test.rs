use pealayer::platform::interop::{
    set_live_status, spawn_interop_listener, InteropCommand, PlayerStatusResponse,
};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc::channel;
use std::time::Duration;

#[test]
fn test_loopback_tcp_interop_commands_and_status() {
    unsafe {
        std::env::set_var("PEALAYER_IPC_PORT", "18085");
        std::env::set_var("PEALAYER_SOCKET_PATH", format!("/tmp/pealayer_tcp_{}.sock", std::process::id()));
    }
    let (tx, rx) = channel::<InteropCommand>();
    let ctx = eframe::egui::Context::default();
    let application_identity =
        pealayer::config::resolved_app_name(&pealayer::config::AppConfig::load());
    spawn_interop_listener(tx, ctx, application_identity);

    // Give background TCP listener a moment to bind
    std::thread::sleep(Duration::from_millis(150));

    let mut stream = TcpStream::connect("127.0.0.1:18085").expect("Failed to connect to loopback IPC");
    stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();

    // 1. Send JSON-RPC Play command
    stream
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"play\"}\n")
        .unwrap();
    stream.flush().unwrap();

    let cmd = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("Did not receive Play command on channel");
    assert!(matches!(cmd, InteropCommand::Play));

    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut response_line = String::new();
    reader.read_line(&mut response_line).unwrap();
    assert!(response_line.contains("\"result\":{\"status\":\"accepted\"}"));

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

    stream
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"get_status\"}\n")
        .unwrap();
    stream.flush().unwrap();

    let mut status_line = String::new();
    reader.read_line(&mut status_line).unwrap();
    assert!(status_line.contains("\"volume\":85.0"));
    assert!(status_line.contains("\"playback_time\":12.34"));
    assert!(status_line.contains("/movies/test.mp4"));
}
