use pealayer::four_d::models::{Effect, HardwareTarget};

#[test]
fn relay_targets_are_exact_and_have_no_builtin_physical_semantics() {
    let target = HardwareTarget::for_relay(42);
    assert_eq!(target, HardwareTarget::Relay(42));
    assert_eq!(target.primary_relay_id(), Some(42));
    assert!(target.is_compatible_with_relay(42));
    assert!(!target.is_compatible_with_relay(1));

    assert_eq!(HardwareTarget::for_relay(0), HardwareTarget::Any);
    assert_eq!(HardwareTarget::Any.primary_relay_id(), None);
    assert!(HardwareTarget::Any.is_compatible_with_relay(42));
    assert!(!HardwareTarget::Any.is_compatible_with_relay(0));
}

#[test]
fn effect_target_round_trip_keeps_only_the_explicit_output_id() {
    let effect = Effect::with_target(
        "Advertised output cue".to_string(),
        String::new(),
        750,
        HardwareTarget::Relay(42),
        vec![],
    );
    let json = serde_json::to_string(&effect).unwrap();
    assert!(!json.contains("Water"));
    assert!(!json.contains("Wind"));
    assert!(!json.contains("Smoke"));
    let decoded: Effect = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.target, HardwareTarget::Relay(42));
}
