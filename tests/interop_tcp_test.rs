use pealayer::platform::interop::{
    InteropCommand, PlayerStatusResponse, set_live_status, spawn_interop_listener,
};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc::channel;
use std::time::Duration;

#[test]
fn test_loopback_tcp_interop_commands_and_status() {
    let (tx, rx) = channel::<InteropCommand>();
    let ctx = eframe::egui::Context::default();
    spawn_interop_listener(tx, ctx);

    // Give background TCP listener a moment to bind
    std::thread::sleep(Duration::from_millis(100));

    let mut stream =
        TcpStream::connect("127.0.0.1:8082").expect("Failed to connect to loopback IPC");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();

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
    assert!(response_line.contains("\"result\":{\"status\":\"ok\"}"));

    // 2. Set mock live status and query via get_status
    let mock_status = PlayerStatusResponse {
        status: "ok".to_string(),
        playing: true,
        volume: 85.0,
        playback_time: 12.34,
        duration: 100.0,
        current_video: Some("/movies/test.mp4".to_string()),
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
