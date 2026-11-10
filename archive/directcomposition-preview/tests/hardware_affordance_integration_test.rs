use pealayer::app::{EffectDragPayload, PealayerApp};
use pealayer::four_d::controller::{HardwareCapabilities, HardwareOutput};
use pealayer::four_d::models::HardwareTarget;
use pealayer::four_d::patterns::generate_constant;

fn install_advertised_relay(app: &PealayerApp, id: u8, name: &str) {
    app.update_hardware_capabilities(Some(HardwareCapabilities {
        board_connected: true,
        board_name: "Fixture board".to_string(),
        capability_bits: 0,
        active_relays: Default::default(),
        relays: vec![HardwareOutput {
            id,
            key: format!("relay.{id}"),
            name: name.to_string(),
            role: "fixture".to_string(),
            control: "relay".to_string(),
        }],
        pwm_channels: vec![],
        macros: vec![],
        ..Default::default()
    }));
}

#[test]
fn effect_drop_uses_only_the_live_advertised_output() {
    let mut app = PealayerApp::default();
    install_advertised_relay(&app, 42, "Seat left");
    let payload = EffectDragPayload {
        name: "Controller cue".to_string(),
        icon: String::new(),
        duration_ms: 500,
        target: HardwareTarget::Relay(42),
        actions: generate_constant(42, true, 500),
        controller_macro: None,
        controller_strip_effect: None,
        controller_lane: None,
    };

    assert!(app.handle_effect_drop(&payload, 0, 1.0));
    let instance = app.timeline.instances.last().unwrap();
    let effect = app
        .timeline
        .templates
        .iter()
        .find(|item| item.id == instance.effect_id)
        .unwrap();
    assert_eq!(effect.actions[0].relay_id, 42);
    assert_eq!(effect.target, HardwareTarget::Relay(42));

    app.lock_track(42, true);
    assert!(!app.handle_effect_drop(&payload, -1, 2.0));
    app.lock_track(42, false);
    assert!(app.handle_effect_drop(&payload, -1, 2.0));

    app.update_hardware_capabilities(None);
    assert!(!app.handle_effect_drop(&payload, -1, 3.0));
}

#[test]
fn a_different_advertised_output_rejects_the_payload() {
    let mut app = PealayerApp::default();
    install_advertised_relay(&app, 77, "Rear actuator");
    let payload = EffectDragPayload {
        name: "Controller cue".to_string(),
        icon: String::new(),
        duration_ms: 500,
        target: HardwareTarget::Relay(42),
        actions: generate_constant(42, true, 500),
        controller_macro: None,
        controller_strip_effect: None,
        controller_lane: None,
    };
    assert!(!app.handle_effect_drop(&payload, 0, 1.0));
    assert!(app.timeline.instances.is_empty());
}
