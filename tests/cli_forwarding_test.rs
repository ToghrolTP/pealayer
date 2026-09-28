use pealayer::cli::{launch_request, send_remote_command, try_forward_launch_request, CliOptions};
use pealayer::platform::interop::{spawn_interop_listener, InteropCommand};
use std::sync::mpsc::channel;
use std::time::Duration;

#[test]
fn test_cli_remote_and_single_instance_forwarding() {
    let sock = format!("/tmp/pealayer_fwd_{}.sock", std::process::id());
    unsafe {
        std::env::set_var("PEALAYER_IPC_PORT", "18084");
        std::env::set_var("PEALAYER_SOCKET_PATH", &sock);
    }
    let (tx, rx) = channel::<InteropCommand>();
    let ctx = eframe::egui::Context::default();
    let application_identity =
        pealayer::config::resolved_app_name(&pealayer::config::AppConfig::load());
    spawn_interop_listener(tx, ctx, application_identity);

    std::thread::sleep(Duration::from_millis(100));

    // 1. Test forwarding an open target
    let request = launch_request(&CliOptions {
        target: Some("test_video.mkv".to_string()),
        fullscreen: true,
        volume: Some(65.0),
    });
    let forwarded = try_forward_launch_request(&request);
    assert!(forwarded);

    let cmd = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive forwarded open command");
    if let InteropCommand::Launch { request } = cmd {
        assert_eq!(request.target.as_deref(), Some("test_video.mkv"));
        assert!(request.fullscreen);
        assert_eq!(request.volume, Some(65.0));
        assert!(request.activate);
        assert!(!request.operation_id.is_empty());
        assert!(request.sender_working_directory.is_some());
        assert!(!request.application_identity.is_empty());
    } else {
        panic!("Expected Launch command");
    }

    // A retry reuses the same operation ID and receives the cached accepted
    // result without dispatching the launch a second time.
    assert!(try_forward_launch_request(&request));
    assert!(rx.recv_timeout(Duration::from_millis(150)).is_err());

    // 2. Test sending remote commands
    // 2a. Pause
    let resp = send_remote_command("pause").expect("Failed to send remote command");
    assert!(resp.contains("\"status\":\"accepted\""));

    let cmd2 = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive remote pause command");
    assert!(matches!(cmd2, InteropCommand::Pause));

    // 2b. Play
    let resp = send_remote_command("play").expect("Failed to send remote play");
    assert!(resp.contains("\"status\":\"accepted\""));
    let cmd3 = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive remote play command");
    assert!(matches!(cmd3, InteropCommand::Play));

    // 2c. Seek
    let resp = send_remote_command("seek 45.2").expect("Failed to send remote seek");
    assert!(resp.contains("\"status\":\"accepted\""));
    let cmd4 = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive remote seek command");
    if let InteropCommand::Seek { seconds } = cmd4 {
        assert!((seconds - 45.2).abs() < 1e-4);
    } else {
        panic!("Expected Seek command, got {:?}", cmd4);
    }

    // 2d. Volume
    let resp = send_remote_command("volume 65.0").expect("Failed to send remote volume");
    assert!(resp.contains("\"status\":\"accepted\""));
    let cmd5 = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive remote volume command");
    if let InteropCommand::SetVolume { value } = cmd5 {
        assert!((value - 65.0).abs() < 1e-4);
    } else {
        panic!("Expected SetVolume command, got {:?}", cmd5);
    }

    // 2e. Raw JSON
    let resp = send_remote_command(r#"{"command":"toggle_pause"}"#).expect("Failed to send raw JSON");
    assert!(resp.contains("\"status\":\"accepted\""));
    let cmd6 = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive toggle_pause command");
    assert!(matches!(cmd6, InteropCommand::TogglePause));

    // 2f. Open command with special characters and path
    let resp = send_remote_command("open /my videos/clip 1.mp4").expect("Failed to send remote open");
    assert!(resp.contains("\"status\":\"accepted\""));
    let cmd7 = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive remote open command");
    if let InteropCommand::Open { target } = cmd7 {
        assert_eq!(target, "/my videos/clip 1.mp4");
    } else {
        panic!("Expected Open command, got {:?}", cmd7);
    }
}
