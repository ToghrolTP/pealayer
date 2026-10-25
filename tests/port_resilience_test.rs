use pealayer::four_d::virtual_board::VirtualBoardSession;

#[test]
fn test_virtual_board_spawn_resilience() {
    if VirtualBoardSession::resolve_binary_path().is_none() {
        eprintln!("Skipping test: virtual_board binary not found");
        return;
    }

    let session = VirtualBoardSession::spawn_with_retries(3)
        .expect("spawn virtual board with retries");
    assert!(session.port > 0);
    drop(session);
}
