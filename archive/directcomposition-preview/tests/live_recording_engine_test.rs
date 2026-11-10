use pealayer::four_d::engine::{EngineMessage, spawn_engine};

#[test]
fn test_live_actuator_override_message() {
    let handle = spawn_engine();
    let res = handle.sender.send(EngineMessage::LiveActuatorOverride {
        channel: 2,
        value: 180,
    });
    assert!(res.is_ok());
}
