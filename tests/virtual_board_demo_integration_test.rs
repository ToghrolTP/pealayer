use pealayer::four_d::controller::{DEMO_ENDPOINT, is_demo_endpoint};
use pealayer::four_d::demo_coordinator::DemoCoordinatorSession;
use pealayer::four_d::demo_timeline::build_demo_showcase_timeline;

#[test]
fn test_virtual_board_demo_full_lifecycle() {
    if pealayer::four_d::demo_coordinator::resolve_controller_binary().is_none() &&
       std::env::var_os("PEALAYER_PCCONTROLLER_LIBRARY").is_none() &&
       pealayer::four_d::embedded_host::library_candidates().is_empty() {
        eprintln!("Skipping integration test: PCController coordinator binary or library candidate not found");
        return;
    }
    if pealayer::four_d::virtual_board::VirtualBoardSession::resolve_binary_path().is_none() {
        eprintln!("Skipping integration test: virtual_board binary not found");
        return;
    }

    assert!(is_demo_endpoint(DEMO_ENDPOINT));

    // 1. Start Demo coordinator session
    let mut session = DemoCoordinatorSession::start().expect("start demo session");

    // 2. Reconcile capabilities
    let caps = session.client.hardware_capabilities().expect("hardware capabilities");
    assert!(caps.board_connected, "virtual board must report connected");
    assert!(caps.supports_segment_display, "must support segment display");
    assert!(caps.supports_addressable_led, "must support addressable LED");
    assert!(caps.controls.iter().any(|c| c.key == "seat.a"), "cinema seat motion profile must be provisioned");

    // 3. Verify showcase timeline loads valid cues
    let timeline = build_demo_showcase_timeline();
    assert!(!timeline.instances.is_empty(), "showcase timeline must have effect instances");
    assert!(!timeline.analog_tracks.is_empty(), "showcase timeline must have analog curves");

    // 4. Test cue execution through coordinator
    let relay_res = session.client.call("controller.command.execute", serde_json::json!({"command": "relay 1 on"}));
    assert!(relay_res.is_ok(), "relay command execution must succeed");

    let pwm_res = session.client.call("controller.pwm.set", serde_json::json!({"channel": 0, "raw_value": 2048}));
    assert!(pwm_res.is_ok(), "PWM set execution must succeed");

    // 5. Clean teardown
    drop(session);
}
