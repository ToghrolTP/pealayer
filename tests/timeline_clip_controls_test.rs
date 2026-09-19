use pealayer::app::{classify_clip_drag_mode, update_effect_duration, DragMode, PealayerApp};
use pealayer::four_d::models::{AtomicAction, Effect, EffectInstance, HardwareTarget};
use uuid::Uuid;

#[test]
fn test_classify_clip_drag_mode_left_right_and_center() {
    let clip_left = 100.0;
    let clip_right = 200.0;

    // Left edge (within 10px)
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 100.0), DragMode::ResizeLeft);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 108.0), DragMode::ResizeLeft);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 110.0), DragMode::ResizeLeft);

    // Right edge (within 10px)
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 200.0), DragMode::ResizeRight);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 192.0), DragMode::ResizeRight);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 190.0), DragMode::ResizeRight);

    // Center (Move)
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 150.0), DragMode::Move);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 111.0), DragMode::Move);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 189.0), DragMode::Move);
}

#[test]
fn test_classify_clip_drag_mode_short_clip() {
    let clip_left = 100.0;
    let clip_right = 110.0; // 10px wide clip
    // Dynamic handle clamping to 35% of width (3.5px)
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 101.0), DragMode::ResizeLeft);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 109.0), DragMode::ResizeRight);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 105.0), DragMode::Move);
}

#[test]
fn test_template_isolation_when_shared() {
    let mut app = PealayerApp::default();
    let template = Effect::with_target(
        "Shared Effect".into(),
        "💧".into(),
        1000,
        HardwareTarget::Water,
        vec![AtomicAction { relay_id: 1, state: true, offset_ms: 0 }],
    );
    let tmpl_id = template.id;
    app.timeline.templates.push(template);

    let inst1 = EffectInstance::new(tmpl_id, 0);
    let inst2 = EffectInstance::new(tmpl_id, 2000);
    let inst1_id = inst1.id;
    let inst2_id = inst2.id;

    app.timeline.instances.push(inst1);
    app.timeline.instances.push(inst2);

    assert_eq!(app.timeline.templates.len(), 1);

    // Isolate inst1
    let new_tmpl_id = app.isolate_template_for_instance(inst1_id).expect("Should isolate");
    assert_ne!(new_tmpl_id, tmpl_id);
    assert_eq!(app.timeline.instances.iter().find(|i| i.id == inst1_id).unwrap().effect_id, new_tmpl_id);
    assert_eq!(app.timeline.instances.iter().find(|i| i.id == inst2_id).unwrap().effect_id, tmpl_id);
    assert_eq!(app.timeline.templates.len(), 2);

    let cloned_template = app.timeline.templates.iter().find(|t| t.id == new_tmpl_id).expect("Cloned template exists");
    assert_eq!(cloned_template.name, "Shared Effect");
    assert_eq!(cloned_template.duration_ms, 1000);
    assert_eq!(cloned_template.target, HardwareTarget::Water);
}

#[test]
fn test_template_isolation_when_not_shared() {
    let mut app = PealayerApp::default();
    let template = Effect::with_target(
        "Solo Effect".into(),
        "⚡".into(),
        1000,
        HardwareTarget::Auxiliary,
        vec![AtomicAction { relay_id: 5, state: true, offset_ms: 0 }],
    );
    let tmpl_id = template.id;
    app.timeline.templates.push(template);

    let inst = EffectInstance::new(tmpl_id, 500);
    let inst_id = inst.id;
    app.timeline.instances.push(inst);

    let result_tmpl_id = app.isolate_template_for_instance(inst_id).expect("Should return existing template");
    assert_eq!(result_tmpl_id, tmpl_id);
    assert_eq!(app.timeline.templates.len(), 1);
    assert_eq!(app.timeline.instances[0].effect_id, tmpl_id);
}

#[test]
fn test_template_isolation_nonexistent_instance() {
    let mut app = PealayerApp::default();
    let random_id = Uuid::new_v4();
    assert_eq!(app.isolate_template_for_instance(random_id), None);
}

#[test]
fn test_pattern_rescaling_preserves_choreography() {
    let mut effect = Effect::with_target(
        "Strobe".into(),
        "⚡".into(),
        1000,
        HardwareTarget::Auxiliary,
        vec![
            AtomicAction { relay_id: 5, state: true, offset_ms: 0 },
            AtomicAction { relay_id: 5, state: false, offset_ms: 250 },
            AtomicAction { relay_id: 5, state: true, offset_ms: 500 },
            AtomicAction { relay_id: 5, state: false, offset_ms: 1000 },
        ],
    );

    update_effect_duration(&mut effect, 2000);
    assert_eq!(effect.duration_ms, 2000);
    assert_eq!(effect.actions.len(), 4);
    assert_eq!(effect.actions[0].offset_ms, 0);
    assert_eq!(effect.actions[1].offset_ms, 500); // 250 * 2
    assert_eq!(effect.actions[2].offset_ms, 1000); // 500 * 2
    assert_eq!(effect.actions[3].offset_ms, 2000); // 1000 * 2
}

#[test]
fn test_pattern_rescaling_constant_effect() {
    let mut effect = Effect::with_target(
        "Constant".into(),
        "🌊".into(),
        1000,
        HardwareTarget::Water,
        vec![
            AtomicAction { relay_id: 1, state: true, offset_ms: 0 },
            AtomicAction { relay_id: 1, state: false, offset_ms: 1000 },
        ],
    );

    update_effect_duration(&mut effect, 3500);
    assert_eq!(effect.duration_ms, 3500);
    assert_eq!(effect.actions.len(), 2);
    assert_eq!(effect.actions[0].offset_ms, 0);
    assert_eq!(effect.actions[1].offset_ms, 3500);
}

#[test]
fn test_pattern_rescaling_single_action() {
    let mut effect = Effect::with_target(
        "Single".into(),
        "💨".into(),
        500,
        HardwareTarget::Wind,
        vec![
            AtomicAction { relay_id: 2, state: true, offset_ms: 0 },
        ],
    );

    update_effect_duration(&mut effect, 1200);
    assert_eq!(effect.duration_ms, 1200);
    assert_eq!(effect.actions.len(), 1);
    assert_eq!(effect.actions[0].offset_ms, 0);
}

#[test]
fn test_classify_clip_drag_mode_outer_bounds_and_clamping() {
    let clip_left = 100.0;
    let clip_right = 200.0;

    // Pointer dragged outward past the left edge
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 95.0), DragMode::ResizeLeft);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 50.0), DragMode::ResizeLeft);

    // Pointer dragged outward past the right edge
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 205.0), DragMode::ResizeRight);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 250.0), DragMode::ResizeRight);

    // Exact handle boundary transitions (handle_w = 10.0)
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 110.0), DragMode::ResizeLeft);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 110.001), DragMode::Move);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 189.999), DragMode::Move);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 190.0), DragMode::ResizeRight);
}

#[test]
fn test_press_origin_drag_mode_resolution_outward_drag() {
    let clip_left = 200.0;
    let clip_right = 350.0;

    // Scenario 1: User presses down on the right handle (x = 348.0)
    let press_origin_right = 348.0;
    // Pointer drags outward past clip boundary before drag_started threshold is crossed
    let latest_pointer_right = 365.0;
    // Resolving from press_origin correctly determines ResizeRight
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, press_origin_right), DragMode::ResizeRight);
    // Note: Outward position also resolves to ResizeRight
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, latest_pointer_right), DragMode::ResizeRight);

    // Scenario 2: User presses down on the left handle (x = 203.0)
    let press_origin_left = 203.0;
    let latest_pointer_left = 185.0; // Outward past left boundary
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, press_origin_left), DragMode::ResizeLeft);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, latest_pointer_left), DragMode::ResizeLeft);

    // Scenario 3: User presses center body
    let press_origin_center = 275.0;
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, press_origin_center), DragMode::Move);
}

#[test]
fn test_handle_geometry_thresholds() {
    // Width = 100.0 -> handle_w = 10.0
    let w1 = 100.0_f32;
    let handle_w1 = (w1 * 0.35).min(10.0);
    assert_eq!(handle_w1, 10.0);
    assert!(w1 >= 14.0);

    // Width = 20.0 -> handle_w = 7.0
    let w2 = 20.0_f32;
    let handle_w2 = (w2 * 0.35).min(10.0);
    assert_eq!(handle_w2, 7.0);
    assert!(w2 >= 14.0);

    // Width = 10.0 -> handle_w = 3.5
    let w3 = 10.0_f32;
    let handle_w3 = (w3 * 0.35).min(10.0);
    assert_eq!(handle_w3, 3.5);
    assert!(w3 < 14.0); // Below visual grip threshold
}


