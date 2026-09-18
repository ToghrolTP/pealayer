use pealayer::app::{EffectDragPayload, PealayerApp};
use pealayer::four_d::models::{Effect, HardwareTarget};
use pealayer::four_d::patterns::generate_constant;

#[test]
fn test_track_drop_compatibility_rules() {
    let water_effect = Effect::with_target(
        "Water Splash".to_string(),
        "💧".to_string(),
        1500,
        HardwareTarget::Water,
        generate_constant(1, true, 1500),
    );

    // Compatible with R1 (Water)
    assert!(water_effect.target.is_compatible_with_relay(1));
    // Incompatible with R2, R3, R4
    assert!(!water_effect.target.is_compatible_with_relay(2));
    assert!(!water_effect.target.is_compatible_with_relay(3));
    assert!(!water_effect.target.is_compatible_with_relay(4));
    // Compatible with Aux R5..=R8
    for r in 5..=8 {
        assert!(water_effect.target.is_compatible_with_relay(r), "Water should be compatible with Aux R{}", r);
    }

    let wind_effect = Effect::with_target(
        "Wind Gale".to_string(),
        "🌀".to_string(),
        5000,
        HardwareTarget::Wind,
        generate_constant(2, true, 5000),
    );
    // Incompatible with R1, R3, R4
    assert!(!wind_effect.target.is_compatible_with_relay(1));
    assert!(!wind_effect.target.is_compatible_with_relay(3));
    assert!(!wind_effect.target.is_compatible_with_relay(4));
    // Compatible with R2
    assert!(wind_effect.target.is_compatible_with_relay(2));
    // Compatible with Aux R5..=R8
    for r in 5..=8 {
        assert!(wind_effect.target.is_compatible_with_relay(r), "Wind should be compatible with Aux R{}", r);
    }
}

#[test]
fn test_mismatched_instance_detection() {
    let water_effect = Effect::with_target(
        "Water Splash".to_string(),
        "💧".to_string(),
        1500,
        HardwareTarget::Water,
        generate_constant(2, true, 1500), // Mistakenly configured on relay 2
    );

    // Relay 2 is Wind Fan, which is incompatible with HardwareTarget::Water
    let instance_relay = 2;
    let is_mismatched = !water_effect.target.is_compatible_with_relay(instance_relay);
    assert!(is_mismatched, "Instance on Relay 2 should be flagged as mismatched for Water effect");

    // Compatible instances
    assert!(water_effect.target.is_compatible_with_relay(1));
    assert!(water_effect.target.is_compatible_with_relay(5));
}

#[test]
fn test_smart_auto_routing_and_rejection_logic() {
    let mut app = PealayerApp::default();

    let water_payload = EffectDragPayload {
        name: "Water Splash".to_string(),
        icon: "💧".to_string(),
        duration_ms: 1500,
        target: HardwareTarget::Water,
        actions: generate_constant(1, true, 1500),
    };

    // 1. Drop on compatible track R1 (track_index = 2) -> Accepted
    let accepted_r1 = app.handle_effect_drop(&water_payload, 2, 1.0);
    assert!(accepted_r1, "Drop on compatible track R1 should succeed");
    assert_eq!(app.timeline.instances.len(), 1);
    let inst1 = &app.timeline.instances[0];
    let tmpl1 = app.timeline.templates.iter().find(|t| t.id == inst1.effect_id).unwrap();
    assert_eq!(tmpl1.target, HardwareTarget::Water);
    assert_eq!(tmpl1.actions[0].relay_id, 1);
    assert_eq!(inst1.start_time_ms, 1000);

    // 2. Drop on incompatible track R2 (track_index = 3) -> Rejected
    let rejected_r2 = app.handle_effect_drop(&water_payload, 3, 2.0);
    assert!(!rejected_r2, "Drop on incompatible track R2 should be rejected");
    assert_eq!(app.timeline.instances.len(), 1, "Instances count should not change on rejected drop");
    assert!(
        app.last_osd_message()
            .map(|msg| msg.contains("Placement Rejected"))
            .unwrap_or(false),
        "Rejection should emit an explanatory OSD warning"
    );

    // 3. Drop on compatible Aux track R5 (track_index = 6) -> Accepted
    let accepted_aux = app.handle_effect_drop(&water_payload, 6, 3.0);
    assert!(accepted_aux, "Drop on compatible Aux track R5 should succeed");
    assert_eq!(app.timeline.instances.len(), 2);
    let inst2 = &app.timeline.instances[1];
    let tmpl2 = app.timeline.templates.iter().find(|t| t.id == inst2.effect_id).unwrap();
    assert_eq!(tmpl2.actions[0].relay_id, 5);

    // 4. Smart Auto-Routing: Dropping on empty canvas / header space (track_index < 2 or track_index > 9)
    // Dropping Water Splash on Video Header (track_index = 0) -> Auto-routed to R1
    let routed_empty = app.handle_effect_drop(&water_payload, 0, 4.0);
    assert!(routed_empty, "Dropping on neutral canvas space should auto-route to primary track");
    assert_eq!(app.timeline.instances.len(), 3);
    let inst3 = &app.timeline.instances[2];
    let tmpl3 = app.timeline.templates.iter().find(|t| t.id == inst3.effect_id).unwrap();
    assert_eq!(tmpl3.actions[0].relay_id, 1, "Water should auto-route to primary track R1");

    // 5. Smart Auto-Routing: Dropping Wind Gale on empty space (track_index = 10) -> Auto-routed to R2
    let wind_payload = EffectDragPayload {
        name: "Wind Gale".to_string(),
        icon: "🌀".to_string(),
        duration_ms: 5000,
        target: HardwareTarget::Wind,
        actions: generate_constant(2, true, 5000),
    };
    let routed_wind = app.handle_effect_drop(&wind_payload, 10, 5.0);
    assert!(routed_wind, "Wind drop on empty space should auto-route to primary track R2");
    assert_eq!(app.timeline.instances.len(), 4);
    let inst4 = &app.timeline.instances[3];
    let tmpl4 = app.timeline.templates.iter().find(|t| t.id == inst4.effect_id).unwrap();
    assert_eq!(tmpl4.actions[0].relay_id, 2, "Wind should auto-route to primary track R2");

    // 6. Track locked guard rails:
    // Lock track R1 (Water)
    app.lock_track(1, true);
    let locked_drop = app.handle_effect_drop(&water_payload, 2, 6.0);
    assert!(!locked_drop, "Drop on locked track should be blocked");
    assert_eq!(app.timeline.instances.len(), 4);

    // Auto-routing to locked primary track should also be blocked
    let locked_auto_route = app.handle_effect_drop(&water_payload, 11, 7.0);
    assert!(!locked_auto_route, "Auto-routing to a locked primary track should be blocked");
    assert_eq!(app.timeline.instances.len(), 4);

    // 7. Verify undo stack captured successful drops
    assert_eq!(app.undo_stack.undo_len(), 4, "4 successful drops should push 4 undo snapshots");
}
