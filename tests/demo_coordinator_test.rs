use pealayer::four_d::demo_coordinator::DemoCoordinatorSession;

#[test]
fn test_demo_coordinator_session_connects_and_provisions_profile() {
    if pealayer::four_d::demo_coordinator::resolve_controller_binary().is_none() &&
       std::env::var_os("PEALAYER_PCCONTROLLER_LIBRARY").is_none() &&
       pealayer::four_d::embedded_host::library_candidates().is_empty() {
        eprintln!("Skipping test: pccontroller coordinator or library candidate not available");
        return;
    }
    if pealayer::four_d::virtual_board::VirtualBoardSession::resolve_binary_path().is_none() {
        eprintln!("Skipping test: virtual_board binary not found");
        return;
    }

    let mut session = DemoCoordinatorSession::start().expect("start demo coordinator session");
    let capabilities = session.client.hardware_capabilities().expect("retrieve hardware capabilities");
    assert!(capabilities.board_connected, "virtual board must be connected");
    assert!(capabilities.controls.iter().any(|c| c.key == "seat.a"), "cinema seat motion profile must be provisioned");
    drop(session);
}
