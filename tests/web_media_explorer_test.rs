use pealayer::platform::interop::InteropCommand;
use pealayer::server::{WebRuntimeConfig, spawn_web_server_on_listener};
use std::io::{Read, Write};
use std::time::Duration;

fn start_server(
    config: Option<WebRuntimeConfig>,
) -> (
    u16,
    std::sync::mpsc::Sender<String>,
    std::sync::mpsc::Receiver<InteropCommand>,
) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let config = config.unwrap_or_else(|| {
        WebRuntimeConfig::production(
            "Pealayer".into(),
            "en".into(),
            "ltr".into(),
            "system".into(),
            [0, 120, 212],
        )
    });
    let (state, commands) =
        spawn_web_server_on_listener(listener, eframe::egui::Context::default(), config);
    (port, state, commands)
}

fn get(port: u16, path: &str) -> String {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    let request =
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn post(port: u16, path: &str, payload: &str) -> String {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

#[test]
fn test_web_command_aliases_and_browsing() {
    let (port, _state_tx, cmd_rx) = start_server(None);

    std::thread::sleep(Duration::from_millis(100));

    // 1. Test POST /api/player/command with the current typed command shape.
    let payload = r#"{"command":"open","target":"/tmp/test_clip.mp4"}"#;

    let mut stream =
        std::net::TcpStream::connect(("127.0.0.1", port)).expect("Failed to connect to web server");
    let req = format!(
        "POST /api/player/command HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    );
    stream.write_all(req.as_bytes()).unwrap();
    let mut resp = String::new();
    stream.read_to_string(&mut resp).unwrap();
    assert!(resp.contains("200 OK"));

    let received = cmd_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("Did not receive command");
    if let InteropCommand::Open { target } = received {
        assert_eq!(target, "/tmp/test_clip.mp4");
    } else {
        panic!("Expected Open command, got {:?}", received);
    }

    // Remote URLs, including live protocols, travel through the same API
    // command without being coerced into filesystem paths.
    let live_payload = r#"{"command":"open","target":"rtsp://camera.invalid/live"}"#;
    let mut live_stream =
        std::net::TcpStream::connect(("127.0.0.1", port)).expect("Failed to connect to web server");
    let live_request = format!(
        "POST /api/player/command HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        live_payload.len(),
        live_payload
    );
    live_stream.write_all(live_request.as_bytes()).unwrap();
    let mut live_response = String::new();
    live_stream.read_to_string(&mut live_response).unwrap();
    assert!(live_response.contains("200 OK"));
    assert!(matches!(
        cmd_rx.recv_timeout(Duration::from_secs(1)),
        Ok(InteropCommand::Open { target }) if target == "rtsp://camera.invalid/live"
    ));

    // 2. Test set_volume with level
    let mut stream2 =
        std::net::TcpStream::connect(("127.0.0.1", port)).expect("Failed to connect to web server");
    let payload2 = r#"{"command":"set_volume","value":75.0}"#;
    let req2 = format!(
        "POST /api/player/command HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload2.len(),
        payload2
    );
    stream2.write_all(req2.as_bytes()).unwrap();
    let mut resp2 = String::new();
    stream2.read_to_string(&mut resp2).unwrap();
    assert!(resp2.contains("200 OK"));

    let received2 = cmd_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("Did not receive command");
    if let InteropCommand::SetVolume { value } = received2 {
        assert_eq!(value, 75.0);
    } else {
        panic!("Expected SetVolume command, got {:?}", received2);
    }
}

#[test]
fn test_web_fs_browse_endpoint() {
    let (port, _state_tx, _cmd_rx) = start_server(None);

    std::thread::sleep(Duration::from_millis(100));

    let mut stream =
        std::net::TcpStream::connect(("127.0.0.1", port)).expect("Failed to connect to web server");
    let req = format!(
        "GET /api/fs/browse HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    use std::io::{Read, Write};
    stream.write_all(req.as_bytes()).unwrap();
    let mut resp = String::new();
    stream.read_to_string(&mut resp).unwrap();
    assert!(resp.contains("200 OK"));
    assert!(resp.contains("entries"));
}

#[test]
fn status_is_unknown_until_first_authoritative_snapshot() {
    let (port, _state_tx, _cmd_rx) = start_server(None);
    std::thread::sleep(Duration::from_millis(100));

    let status = get(port, "/api/player/status");
    assert!(status.contains("503 Service Unavailable"));
    assert!(status.contains(r#"{"status":"initializing"}"#));
    assert!(!status.contains("\"volume\""));

    let runtime = get(port, "/api/runtime/config");
    assert!(runtime.contains("200 OK"));
    assert!(runtime.contains("\"appName\":"));
    assert!(runtime.contains("\"websocketPath\":\"/ws\""));
    assert!(runtime.contains("\"direction\":"));
}

#[test]
fn config_api_returns_live_settings_and_accepts_validated_patches() {
    let mut config = pealayer::config::AppConfig::default();
    config.hardware_endpoint = Some("pccontroller://config-api-test:8787".to_string());
    pealayer::platform::interop::set_live_config(config);
    let (port, _state_tx, cmd_rx) = start_server(None);
    std::thread::sleep(Duration::from_millis(100));

    let current = get(port, "/api/config");
    assert!(current.contains("200 OK"));
    assert!(current.contains("pccontroller://config-api-test:8787"));

    let response = post(
        port,
        "/api/config",
        r#"{"theme":"dark","show_subseconds":false}"#,
    );
    assert!(response.contains("202 Accepted"));
    let command = cmd_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(matches!(
        command,
        InteropCommand::UpdateConfig { values }
            if values.get("theme").and_then(serde_json::Value::as_str) == Some("dark")
                && values.get("show_subseconds").and_then(serde_json::Value::as_bool) == Some(false)
    ));

    let rejected = post(port, "/api/config", r#"{"unknown_setting":true}"#);
    assert!(rejected.contains("400 Bad Request"));
    assert!(rejected.contains("unknown configuration setting"));

    let invalid_value = post(port, "/api/config", r#"{"volume":999}"#);
    assert!(invalid_value.contains("400 Bad Request"));
    assert!(invalid_value.contains("volume must be between 0 and 130"));
}

#[test]
fn http_websocket_and_ipc_share_one_port() {
    let (port, _state_tx, cmd_rx) = start_server(None);
    std::thread::sleep(Duration::from_millis(100));

    let health = get(port, "/healthz");
    assert!(health.contains("200 OK"));
    assert!(health.contains("\"service\":\"pealayer\""));
    assert!(health.contains("\"rpc\":\"2.0\""));
    assert!(!health.contains("\"transport\""));

    let (mut websocket, _) = tungstenite::connect(format!("ws://127.0.0.1:{port}/ws"))
        .expect("WebSocket must upgrade on the unified port");
    websocket
        .send(tungstenite::Message::Text(r#"{"command":"play"}"#.into()))
        .unwrap();
    assert!(matches!(
        cmd_rx.recv_timeout(Duration::from_secs(1)),
        Ok(InteropCommand::Play)
    ));

    let payload = r#"{"command":"pause"}"#;
    let mut ipc = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    let request = format!(
        "POST /api/ipc HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    );
    ipc.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    ipc.read_to_string(&mut response).unwrap();
    assert!(response.contains("200 OK"));
    assert!(response.contains("\"status\":\"accepted\""));
    assert!(matches!(
        cmd_rx.recv_timeout(Duration::from_secs(1)),
        Ok(InteropCommand::Pause)
    ));
}
