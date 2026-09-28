use pealayer::platform::interop::InteropCommand;
use pealayer::server::spawn_web_server;
use std::io::{Read, Write};
use std::time::Duration;

fn get(port: u16, path: &str) -> String {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

#[test]
fn test_web_command_aliases_and_browsing() {
    let ctx = eframe::egui::Context::default();
    let (_state_tx, cmd_rx) = spawn_web_server(18080, 18081, ctx);

    std::thread::sleep(Duration::from_millis(100));

    // 1. Test POST /api/player/command with open_video and path
    let payload = r#"{"command":"open_video","path":"/tmp/test_clip.mp4"}"#;

    let mut stream = std::net::TcpStream::connect("127.0.0.1:18080").expect("Failed to connect to web server");
    let req = format!(
        "POST /api/player/command HTTP/1.1\r\nHost: 127.0.0.1:18080\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    );
    stream.write_all(req.as_bytes()).unwrap();
    let mut resp = String::new();
    stream.read_to_string(&mut resp).unwrap();
    assert!(resp.contains("200 OK"));

    let received = cmd_rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive command");
    if let InteropCommand::Open { target } = received {
        assert_eq!(target, "/tmp/test_clip.mp4");
    } else {
        panic!("Expected Open command, got {:?}", received);
    }

    // 2. Test set_volume with level
    let mut stream2 = std::net::TcpStream::connect("127.0.0.1:18080").expect("Failed to connect to web server");
    let payload2 = r#"{"command":"set_volume","level":75.0}"#;
    let req2 = format!(
        "POST /api/player/command HTTP/1.1\r\nHost: 127.0.0.1:18080\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload2.len(),
        payload2
    );
    stream2.write_all(req2.as_bytes()).unwrap();
    let mut resp2 = String::new();
    stream2.read_to_string(&mut resp2).unwrap();
    assert!(resp2.contains("200 OK"));

    let received2 = cmd_rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive command");
    if let InteropCommand::SetVolume { value } = received2 {
        assert_eq!(value, 75.0);
    } else {
        panic!("Expected SetVolume command, got {:?}", received2);
    }
}

#[test]
fn test_web_fs_browse_endpoint() {
    let ctx = eframe::egui::Context::default();
    let (_state_tx, _cmd_rx) = spawn_web_server(18082, 18083, ctx);

    std::thread::sleep(Duration::from_millis(100));

    let mut stream = std::net::TcpStream::connect("127.0.0.1:18082").expect("Failed to connect to web server");
    let req = "GET /api/fs/browse HTTP/1.1\r\nHost: 127.0.0.1:18082\r\nConnection: close\r\n\r\n";
    use std::io::{Read, Write};
    stream.write_all(req.as_bytes()).unwrap();
    let mut resp = String::new();
    stream.read_to_string(&mut resp).unwrap();
    assert!(resp.contains("200 OK"));
    assert!(resp.contains("entries"));
}

#[test]
fn status_is_unknown_until_first_authoritative_snapshot() {
    let ctx = eframe::egui::Context::default();
    let (_state_tx, _cmd_rx) = pealayer::server::spawn_web_server_configured(
        18084,
        18085,
        ctx,
        pealayer::server::WebRuntimeConfig::production(
            "Workshop Player".to_string(),
            18085,
            "fa".to_string(),
            "rtl".to_string(),
            "dark".to_string(),
        ),
    );
    std::thread::sleep(Duration::from_millis(100));

    let status = get(18084, "/api/player/status");
    assert!(status.contains("503 Service Unavailable"));
    assert!(status.contains(r#"{"status":"initializing"}"#));
    assert!(!status.contains("\"volume\""));

    let runtime = get(18084, "/api/runtime/config");
    assert!(runtime.contains("200 OK"));
    assert!(runtime.contains("Workshop Player"));
    assert!(runtime.contains("\"wsPort\":18085"));
    assert!(runtime.contains("\"direction\":\"rtl\""));
}
