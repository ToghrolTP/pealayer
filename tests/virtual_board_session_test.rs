use pealayer::four_d::virtual_board::VirtualBoardSession;
use std::net::TcpStream;

#[test]
fn test_virtual_board_binary_resolution_and_session_spawn() {
    let binary = VirtualBoardSession::resolve_binary_path();
    if binary.is_none() {
        eprintln!("Skipping test: virtual_board binary not found in dev or env path");
        return;
    }
    let session = VirtualBoardSession::spawn().expect("spawn virtual board session");
    assert!(session.port > 0, "session must bind to a positive loopback port");

    // Verify TCP socket accepts connections
    let stream = TcpStream::connect(("127.0.0.1", session.port));
    assert!(stream.is_ok(), "TCP connection to virtual board must succeed");
    drop(stream);

    drop(session);
}

#[test]
fn test_virtual_board_piped_console_command() {
    let binary = VirtualBoardSession::resolve_binary_path();
    if binary.is_none() {
        eprintln!("Skipping test: virtual_board binary not found in dev or env path");
        return;
    }
    let session = VirtualBoardSession::spawn().expect("spawn virtual board session");
    assert!(session.port > 0);

    let result = session.send_command("status");
    assert!(result.is_ok(), "sending console command must succeed");

    let result_door = session.send_command("door closed");
    assert!(result_door.is_ok(), "sending door command must succeed");
}
