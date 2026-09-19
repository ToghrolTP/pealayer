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

#[test]
fn test_undo_redo_clip_resize_restores_duration_and_template() {
    let mut app = PealayerApp::default();
    let template = Effect::with_target(
        "Wind Gust".into(),
        "💨".into(),
        1000,
        HardwareTarget::Wind,
        vec![
            AtomicAction { relay_id: 2, state: true, offset_ms: 0 },
            AtomicAction { relay_id: 2, state: false, offset_ms: 1000 },
        ],
    );
    let tmpl_id = template.id;
    app.timeline.templates.push(template);

    let instance = EffectInstance::new(tmpl_id, 500);
    let inst_id = instance.id;
    app.timeline.instances.push(instance);

    // Simulate drag start: push undo snapshot, isolate template if shared
    app.undo_stack.push(app.snapshot_timeline());
    app.isolate_template_for_instance(inst_id);

    // Resize to 3500ms
    let tmpl = app.timeline.templates.iter_mut().find(|t| t.id == inst_id || t.id == tmpl_id).unwrap();
    update_effect_duration(tmpl, 3500);

    assert_eq!(app.timeline.templates.iter().find(|t| t.id == tmpl_id).unwrap().duration_ms, 3500);

    // Undo resize
    let current = app.snapshot_timeline();
    let prev = app.undo_stack.undo(current).expect("Should undo");
    app.restore_timeline_snapshot(prev);

    assert_eq!(app.timeline.templates.iter().find(|t| t.id == tmpl_id).unwrap().duration_ms, 1000);
    assert_eq!(app.timeline.templates.iter().find(|t| t.id == tmpl_id).unwrap().actions[1].offset_ms, 1000);

    // Redo resize
    let current = app.snapshot_timeline();
    let next = app.undo_stack.redo(current).expect("Should redo");
    app.restore_timeline_snapshot(next);

    assert_eq!(app.timeline.templates.iter().find(|t| t.id == tmpl_id).unwrap().duration_ms, 3500);
    assert_eq!(app.timeline.templates.iter().find(|t| t.id == tmpl_id).unwrap().actions[1].offset_ms, 3500);
}

#[test]
fn test_undo_redo_clip_move_restores_start_time() {
    let mut app = PealayerApp::default();
    let template = Effect::with_target(
        "Water Splash".into(),
        "💧".into(),
        1200,
        HardwareTarget::Water,
        vec![AtomicAction { relay_id: 1, state: true, offset_ms: 0 }],
    );
    let tmpl_id = template.id;
    app.timeline.templates.push(template);

    let instance = EffectInstance::new(tmpl_id, 1000);
    let inst_id = instance.id;
    app.timeline.instances.push(instance);

    // Drag start on move
    app.undo_stack.push(app.snapshot_timeline());

    // Move to 3200ms
    app.timeline.instances.iter_mut().find(|i| i.id == inst_id).unwrap().start_time_ms = 3200;
    assert_eq!(app.timeline.instances[0].start_time_ms, 3200);

    // Undo move
    let current = app.snapshot_timeline();
    let prev = app.undo_stack.undo(current).expect("Should undo");
    app.restore_timeline_snapshot(prev);

    assert_eq!(app.timeline.instances[0].start_time_ms, 1000);

    // Redo move
    let current = app.snapshot_timeline();
    let next = app.undo_stack.redo(current).expect("Should redo");
    app.restore_timeline_snapshot(next);

    assert_eq!(app.timeline.instances[0].start_time_ms, 3200);
}

#[test]
fn test_undo_redo_multi_action_pulse_pattern_preservation() {
    let mut app = PealayerApp::default();
    let template = Effect::with_target(
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
    let tmpl_id = template.id;
    app.timeline.templates.push(template);

    // Two instances sharing the template
    let inst1 = EffectInstance::new(tmpl_id, 500);
    let inst2 = EffectInstance::new(tmpl_id, 3000);
    let inst1_id = inst1.id;
    let inst2_id = inst2.id;
    app.timeline.instances.push(inst1);
    app.timeline.instances.push(inst2);

    // User resizes inst1:
    // 1. Snapshot taken on drag start
    app.undo_stack.push(app.snapshot_timeline());
    // 2. Template isolated
    let isolated_tmpl_id = app.isolate_template_for_instance(inst1_id).expect("Should isolate");
    assert_ne!(isolated_tmpl_id, tmpl_id);

    // 3. Update duration of isolated template to 2000ms
    let isolated_tmpl = app.timeline.templates.iter_mut().find(|t| t.id == isolated_tmpl_id).unwrap();
    update_effect_duration(isolated_tmpl, 2000);

    // Verify inst1's isolated template has scaled offsets
    assert_eq!(isolated_tmpl.duration_ms, 2000);
    assert_eq!(isolated_tmpl.actions[0].offset_ms, 0);
    assert_eq!(isolated_tmpl.actions[1].offset_ms, 500);
    assert_eq!(isolated_tmpl.actions[2].offset_ms, 1000);
    assert_eq!(isolated_tmpl.actions[3].offset_ms, 2000);

    // Sibling inst2's original template must be untouched
    let original_tmpl = app.timeline.templates.iter().find(|t| t.id == tmpl_id).unwrap();
    assert_eq!(original_tmpl.duration_ms, 1000);
    assert_eq!(original_tmpl.actions[1].offset_ms, 250);
    assert_eq!(original_tmpl.actions[2].offset_ms, 500);
    assert_eq!(original_tmpl.actions[3].offset_ms, 1000);

    // 4. Undo the resize operation
    let current = app.snapshot_timeline();
    let prev = app.undo_stack.undo(current).expect("Should undo");
    app.restore_timeline_snapshot(prev);

    // Both instances should now point to the original template, with original choreography
    assert_eq!(app.timeline.templates.len(), 1);
    let inst1_restored = app.timeline.instances.iter().find(|i| i.id == inst1_id).unwrap();
    let inst2_restored = app.timeline.instances.iter().find(|i| i.id == inst2_id).unwrap();
    assert_eq!(inst1_restored.effect_id, tmpl_id);
    assert_eq!(inst2_restored.effect_id, tmpl_id);

    let tmpl_restored = app.timeline.templates.iter().find(|t| t.id == tmpl_id).unwrap();
    assert_eq!(tmpl_restored.duration_ms, 1000);
    assert_eq!(tmpl_restored.actions[1].offset_ms, 250);
    assert_eq!(tmpl_restored.actions[2].offset_ms, 500);
    assert_eq!(tmpl_restored.actions[3].offset_ms, 1000);
}

#[test]
fn test_inspector_duration_slider_range_up_to_60s() {
    let mut app = PealayerApp::default();
    let template = Effect::with_target(
        "Long Wind".into(),
        "💨".into(),
        5000,
        HardwareTarget::Wind,
        vec![
            AtomicAction { relay_id: 2, state: true, offset_ms: 0 },
            AtomicAction { relay_id: 2, state: false, offset_ms: 5000 },
        ],
    );
    let tmpl_id = template.id;
    app.timeline.templates.push(template);

    let instance = EffectInstance::new(tmpl_id, 0);
    let inst_id = instance.id;
    app.timeline.instances.push(instance);

    // Snapshot before inspector change
    app.undo_stack.push(app.snapshot_timeline());

    // Isolate template if shared
    app.isolate_template_for_instance(inst_id);

    // Slider set to maximum 60,000ms (60s)
    let new_dur = 60000_u64;
    let tmpl = app.timeline.templates.iter_mut().find(|t| t.id == tmpl_id).unwrap();
    update_effect_duration(tmpl, new_dur);

    assert_eq!(tmpl.duration_ms, 60000);
    assert_eq!(tmpl.actions.len(), 2);
    assert_eq!(tmpl.actions[1].offset_ms, 60000);

    // Undo reverts back to 5000ms
    let current = app.snapshot_timeline();
    let prev = app.undo_stack.undo(current).expect("Should undo");
    app.restore_timeline_snapshot(prev);

    let restored_tmpl = app.timeline.templates.iter().find(|t| t.id == tmpl_id).unwrap();
    assert_eq!(restored_tmpl.duration_ms, 5000);
    assert_eq!(restored_tmpl.actions[1].offset_ms, 5000);
}



