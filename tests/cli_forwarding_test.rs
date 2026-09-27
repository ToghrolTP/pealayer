use pealayer::cli::{send_remote_command, try_forward_to_existing_instance};
use pealayer::platform::interop::{spawn_interop_listener, InteropCommand};
use std::sync::mpsc::channel;
use std::time::Duration;

#[test]
fn test_cli_remote_and_single_instance_forwarding() {
    let (tx, rx) = channel::<InteropCommand>();
    let ctx = eframe::egui::Context::default();
    spawn_interop_listener(tx, ctx);

    std::thread::sleep(Duration::from_millis(100));

    // 1. Test forwarding an open target
    let forwarded = try_forward_to_existing_instance("test_video.mkv");
    assert!(forwarded);

    let cmd = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive forwarded open command");
    if let InteropCommand::Open { target } = cmd {
        assert_eq!(target, "test_video.mkv");
    } else {
        panic!("Expected Open command");
    }

    // 2. Test sending remote commands
    // 2a. Pause
    let resp = send_remote_command("pause").expect("Failed to send remote command");
    assert!(resp.contains("\"status\":\"ok\""));

    let cmd2 = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive remote pause command");
    assert!(matches!(cmd2, InteropCommand::Pause));

    // 2b. Play
    let resp = send_remote_command("play").expect("Failed to send remote play");
    assert!(resp.contains("\"status\":\"ok\""));
    let cmd3 = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive remote play command");
    assert!(matches!(cmd3, InteropCommand::Play));

    // 2c. Seek
    let resp = send_remote_command("seek 45.2").expect("Failed to send remote seek");
    assert!(resp.contains("\"status\":\"ok\""));
    let cmd4 = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive remote seek command");
    if let InteropCommand::Seek { seconds } = cmd4 {
        assert!((seconds - 45.2).abs() < 1e-4);
    } else {
        panic!("Expected Seek command, got {:?}", cmd4);
    }

    // 2d. Volume
    let resp = send_remote_command("volume 65.0").expect("Failed to send remote volume");
    assert!(resp.contains("\"status\":\"ok\""));
    let cmd5 = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive remote volume command");
    if let InteropCommand::SetVolume { value } = cmd5 {
        assert!((value - 65.0).abs() < 1e-4);
    } else {
        panic!("Expected SetVolume command, got {:?}", cmd5);
    }

    // 2e. Raw JSON
    let resp = send_remote_command(r#"{"command":"toggle_pause"}"#).expect("Failed to send raw JSON");
    assert!(resp.contains("\"status\":\"ok\""));
    let cmd6 = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive toggle_pause command");
    assert!(matches!(cmd6, InteropCommand::TogglePause));

    // 2f. Open command with special characters and path
    let resp = send_remote_command("open /my videos/clip 1.mp4").expect("Failed to send remote open");
    assert!(resp.contains("\"status\":\"ok\""));
    let cmd7 = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive remote open command");
    if let InteropCommand::Open { target } = cmd7 {
        assert_eq!(target, "/my videos/clip 1.mp4");
    } else {
        panic!("Expected Open command, got {:?}", cmd7);
    }
}
