use pealayer::four_d::models::{Effect, HardwareTarget};

#[test]
fn test_hardware_target_relay_compatibility() {
    // Water: relay 1 and aux 5..=8
    assert!(HardwareTarget::Water.is_compatible_with_relay(1));
    for r in 5..=8 {
        assert!(HardwareTarget::Water.is_compatible_with_relay(r));
    }
    assert!(!HardwareTarget::Water.is_compatible_with_relay(2));
    assert!(!HardwareTarget::Water.is_compatible_with_relay(3));
    assert!(!HardwareTarget::Water.is_compatible_with_relay(4));
    assert!(!HardwareTarget::Water.is_compatible_with_relay(0));
    assert!(!HardwareTarget::Water.is_compatible_with_relay(9));

    // Wind: relay 2 and aux 5..=8
    assert!(HardwareTarget::Wind.is_compatible_with_relay(2));
    for r in 5..=8 {
        assert!(HardwareTarget::Wind.is_compatible_with_relay(r));
    }
    assert!(!HardwareTarget::Wind.is_compatible_with_relay(1));
    assert!(!HardwareTarget::Wind.is_compatible_with_relay(3));
    assert!(!HardwareTarget::Wind.is_compatible_with_relay(4));

    // SeatVibration: relay 3 and aux 5..=8
    assert!(HardwareTarget::SeatVibration.is_compatible_with_relay(3));
    for r in 5..=8 {
        assert!(HardwareTarget::SeatVibration.is_compatible_with_relay(r));
    }
    assert!(!HardwareTarget::SeatVibration.is_compatible_with_relay(1));
    assert!(!HardwareTarget::SeatVibration.is_compatible_with_relay(2));
    assert!(!HardwareTarget::SeatVibration.is_compatible_with_relay(4));

    // Smoke: relay 4 and aux 5..=8
    assert!(HardwareTarget::Smoke.is_compatible_with_relay(4));
    for r in 5..=8 {
        assert!(HardwareTarget::Smoke.is_compatible_with_relay(r));
    }
    assert!(!HardwareTarget::Smoke.is_compatible_with_relay(1));
    assert!(!HardwareTarget::Smoke.is_compatible_with_relay(2));
    assert!(!HardwareTarget::Smoke.is_compatible_with_relay(3));

    // Auxiliary: aux 5..=8 only
    for r in 1..=4 {
        assert!(!HardwareTarget::Auxiliary.is_compatible_with_relay(r));
    }
    for r in 5..=8 {
        assert!(HardwareTarget::Auxiliary.is_compatible_with_relay(r));
    }
    assert!(!HardwareTarget::Auxiliary.is_compatible_with_relay(0));
    assert!(!HardwareTarget::Auxiliary.is_compatible_with_relay(9));

    // Any: all relays 1..=8
    for r in 1..=8 {
        assert!(HardwareTarget::Any.is_compatible_with_relay(r));
    }
    assert!(!HardwareTarget::Any.is_compatible_with_relay(0));
    assert!(!HardwareTarget::Any.is_compatible_with_relay(9));

    // for_relay mapping
    assert_eq!(HardwareTarget::for_relay(1), HardwareTarget::Water);
    assert_eq!(HardwareTarget::for_relay(2), HardwareTarget::Wind);
    assert_eq!(HardwareTarget::for_relay(3), HardwareTarget::SeatVibration);
    assert_eq!(HardwareTarget::for_relay(4), HardwareTarget::Smoke);
    for r in 5..=8 {
        assert_eq!(HardwareTarget::for_relay(r), HardwareTarget::Auxiliary);
    }
    assert_eq!(HardwareTarget::for_relay(0), HardwareTarget::Any);
    assert_eq!(HardwareTarget::for_relay(9), HardwareTarget::Any);

    // display_name
    assert_eq!(HardwareTarget::Water.display_name(), "Water Valve");
    assert_eq!(HardwareTarget::Wind.display_name(), "Wind Fan");
    assert_eq!(HardwareTarget::SeatVibration.display_name(), "Seat Vibration");
    assert_eq!(HardwareTarget::Smoke.display_name(), "Smoke Machine");
    assert_eq!(HardwareTarget::Auxiliary.display_name(), "Aux Relay");
    assert_eq!(HardwareTarget::Any.display_name(), "General Cue");
}

#[test]
fn test_hardware_target_primary_relays() {
    assert_eq!(HardwareTarget::Water.primary_relay_id(), Some(1));
    assert_eq!(HardwareTarget::Wind.primary_relay_id(), Some(2));
    assert_eq!(HardwareTarget::SeatVibration.primary_relay_id(), Some(3));
    assert_eq!(HardwareTarget::Smoke.primary_relay_id(), Some(4));
    assert_eq!(HardwareTarget::Auxiliary.primary_relay_id(), Some(5));
    assert_eq!(HardwareTarget::Any.primary_relay_id(), None);
}

#[test]
fn test_effect_serde_backward_compatibility() {
    // 1. Deserializing legacy JSON without "target" field should default to HardwareTarget::Any
    let legacy_json = r#"{
        "id": "00000000-0000-0000-0000-000000000001",
        "name": "Legacy Splash",
        "icon": "💧",
        "duration_ms": 1500,
        "actions": []
    }"#;
    let effect: Effect = serde_json::from_str(legacy_json).expect("Legacy JSON should deserialize cleanly");
    assert_eq!(effect.target, HardwareTarget::Any);
    assert_eq!(effect.name, "Legacy Splash");
    assert_eq!(effect.duration_ms, 1500);

    // 2. Deserializing new JSON with explicit target field
    let new_json = r#"{
        "id": "00000000-0000-0000-0000-000000000002",
        "name": "Wind Blast",
        "icon": "💨",
        "duration_ms": 2000,
        "target": "Wind",
        "actions": []
    }"#;
    let effect: Effect = serde_json::from_str(new_json).expect("New JSON should deserialize cleanly");
    assert_eq!(effect.target, HardwareTarget::Wind);

    // 3. Effect::new defaults target to Any
    let effect_new = Effect::new("Test Cue".to_string(), "⚡".to_string(), 500, vec![]);
    assert_eq!(effect_new.target, HardwareTarget::Any);

    // 4. Effect::with_target assigns specified target
    let effect_targeted = Effect::with_target(
        "Mist Spray".to_string(),
        "🌫".to_string(),
        3000,
        HardwareTarget::Water,
        vec![],
    );
    assert_eq!(effect_targeted.target, HardwareTarget::Water);

    // 5. Roundtrip serialization preserves target
    let serialized = serde_json::to_string(&effect_targeted).expect("Serialization failed");
    let deserialized: Effect = serde_json::from_str(&serialized).expect("Deserialization failed");
    assert_eq!(deserialized, effect_targeted);
    assert_eq!(deserialized.target, HardwareTarget::Water);
}
