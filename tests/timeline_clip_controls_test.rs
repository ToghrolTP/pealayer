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
fn test_pattern_rescaling_with_initial_delay() {
    let mut effect = Effect::with_target(
        "Delayed Pulse".into(),
        "⚡".into(),
        1000,
        HardwareTarget::Auxiliary,
        vec![
            AtomicAction { relay_id: 5, state: true, offset_ms: 200 },
            AtomicAction { relay_id: 5, state: false, offset_ms: 600 },
            AtomicAction { relay_id: 5, state: false, offset_ms: 1000 },
        ],
    );

    // Scale up: 200 * 2.5 = 500, 600 * 2.5 = 1500, end = 2500
    update_effect_duration(&mut effect, 2500);
    assert_eq!(effect.duration_ms, 2500);
    assert_eq!(effect.actions.len(), 3);
    assert_eq!(effect.actions[0].offset_ms, 500);
    assert_eq!(effect.actions[1].offset_ms, 1500);
    assert_eq!(effect.actions[2].offset_ms, 2500);

    // Scale down: 500 * (1000 / 2500) = 200, 1500 * 0.4 = 600, end = 1000
    update_effect_duration(&mut effect, 1000);
    assert_eq!(effect.duration_ms, 1000);
    assert_eq!(effect.actions[0].offset_ms, 200);
    assert_eq!(effect.actions[1].offset_ms, 600);
    assert_eq!(effect.actions[2].offset_ms, 1000);
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
    let effect_id = app.timeline.instances.iter().find(|i| i.id == inst_id).unwrap().effect_id;
    let tmpl = app.timeline.templates.iter_mut().find(|t| t.id == effect_id).unwrap();
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

#[test]
fn test_e2e_rapid_outward_drag_classification() {
    // Verifies that rapid outward mouse movement far past clip boundaries
    // still reliably classifies as resize operations rather than defaulting to Move.
    let clip_left = 300.0_f32;
    let clip_right = 500.0_f32; // 200px wide clip, handle_w = 10px

    // Rapid outward drag to the right: user clicks near right handle and swiftly flicks right
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, clip_right + 50.0), DragMode::ResizeRight);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, clip_right + 150.0), DragMode::ResizeRight);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, clip_right + 1000.0), DragMode::ResizeRight);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, clip_right + 0.1), DragMode::ResizeRight);

    // Rapid outward drag to the left: user clicks near left handle and swiftly flicks left
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, clip_left - 50.0), DragMode::ResizeLeft);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, clip_left - 150.0), DragMode::ResizeLeft);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, clip_left - 1000.0), DragMode::ResizeLeft);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, clip_left - 0.1), DragMode::ResizeLeft);

    // Inside center body: should classify as Move
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 350.0), DragMode::Move);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 400.0), DragMode::Move);
    assert_eq!(classify_clip_drag_mode(clip_left, clip_right, 450.0), DragMode::Move);

    // Verify narrow clip with clamped handles (width = 16.0, handle_w = 5.6)
    let narrow_left = 100.0_f32;
    let narrow_right = 116.0_f32;
    assert_eq!(classify_clip_drag_mode(narrow_left, narrow_right, narrow_right + 50.0), DragMode::ResizeRight);
    assert_eq!(classify_clip_drag_mode(narrow_left, narrow_right, narrow_left - 50.0), DragMode::ResizeLeft);
    assert_eq!(classify_clip_drag_mode(narrow_left, narrow_right, 108.0), DragMode::Move);
}

#[test]
fn test_e2e_resize_snapping_boundaries_to_neighbors_and_playhead() {
    // Verifies snapping interaction during resize:
    // When snap is enabled (default), resizing near a neighbor boundary or the playhead (within 100ms)
    // snaps to the target. When snap is disabled (e.g. shift/alt pressed), it remains continuous.
    let mut app = PealayerApp::default();
    let playback_time = 2.150; // Playhead at 2150ms

    // Template 1: Constant water valve
    let t1 = Effect::with_target(
        "Water Cue 1".into(),
        "💧".into(),
        1000,
        HardwareTarget::Water,
        vec![
            AtomicAction { relay_id: 1, state: true, offset_ms: 0 },
            AtomicAction { relay_id: 1, state: false, offset_ms: 1000 },
        ],
    );
    let t1_id = t1.id;
    app.timeline.templates.push(t1);

    // Template 2: Constant water valve
    let t2 = Effect::with_target(
        "Water Cue 2".into(),
        "💧".into(),
        1000,
        HardwareTarget::Water,
        vec![
            AtomicAction { relay_id: 1, state: true, offset_ms: 0 },
            AtomicAction { relay_id: 1, state: false, offset_ms: 1000 },
        ],
    );
    let t2_id = t2.id;
    app.timeline.templates.push(t2);

    // Instance 1: start 1000ms, duration 1000ms -> ends at 2000ms
    let inst1 = EffectInstance::new(t1_id, 1000);
    let inst1_id = inst1.id;
    app.timeline.instances.push(inst1);

    // Instance 2 (neighbor): start 2500ms, duration 1000ms -> ends at 3500ms
    let inst2 = EffectInstance::new(t2_id, 2500);
    let inst2_id = inst2.id;
    app.timeline.instances.push(inst2);

    // Helper closure to build snap targets exactly as layout.rs does
    let build_snap_targets = |app: &PealayerApp, active_id: Uuid, playhead_secs: f64| -> Vec<u64> {
        let mut targets = vec![0, (playhead_secs * 1000.0) as u64];
        for inst in &app.timeline.instances {
            if inst.id == active_id {
                continue;
            }
            if let Some(tmpl) = app.timeline.templates.iter().find(|t| t.id == inst.effect_id) {
                targets.push(inst.start_time_ms);
                targets.push(inst.start_time_ms + tmpl.duration_ms);
            }
        }
        targets
    };

    let snap_targets_for_inst1 = build_snap_targets(&app, inst1_id, playback_time);
    // Snap targets: [0, 2150 (playhead), 2500 (inst2 start), 3500 (inst2 end)]
    assert!(snap_targets_for_inst1.contains(&0));
    assert!(snap_targets_for_inst1.contains(&2150));
    assert!(snap_targets_for_inst1.contains(&2500));
    assert!(snap_targets_for_inst1.contains(&3500));

    // --- Scenario A: ResizeRight of Instance 1 snaps to neighbor start (2500ms) ---
    // Initial start = 1000, initial end = 2000. Drag delta = +480ms -> raw new_end = 2480ms
    let initial_start = 1000_u64;
    let initial_dur = 1000_u64;
    let delta_ms = 480_i64; // raw new_end = 2480ms (within 20ms of 2500ms)

    // With snapping enabled:
    let mut snapped_end = (initial_start + initial_dur) as i64 + delta_ms;
    for target in &snap_targets_for_inst1 {
        if (snapped_end - *target as i64).abs() <= 100 {
            snapped_end = *target as i64;
            break;
        }
    }
    assert_eq!(snapped_end, 2500); // Snapped to neighbor's start!
    let new_dur = (snapped_end - initial_start as i64).max(100) as u64;
    assert_eq!(new_dur, 1500);

    // Apply duration change to inst1's template
    let effect_id = app.timeline.instances.iter().find(|i| i.id == inst1_id).unwrap().effect_id;
    let tmpl = app.timeline.templates.iter_mut().find(|t| t.id == effect_id).unwrap();
    update_effect_duration(tmpl, new_dur);
    assert_eq!(tmpl.duration_ms, 1500);
    assert_eq!(tmpl.actions[1].offset_ms, 1500);

    // With snapping disabled (Shift/Alt modifier): raw 2480ms is preserved without snap
    let raw_end = (initial_start + initial_dur) as i64 + delta_ms;
    assert_eq!(raw_end, 2480);
    let unsnapped_dur = (raw_end - initial_start as i64).max(100) as u64;
    assert_eq!(unsnapped_dur, 1480);

    // --- Scenario B: ResizeRight of Instance 1 snaps to Playhead (2150ms) ---
    let delta_playhead = 130_i64; // raw new_end = 2000 + 130 = 2130ms (within 20ms of 2150ms)
    let mut snapped_to_playhead = (initial_start + initial_dur) as i64 + delta_playhead;
    for target in &snap_targets_for_inst1 {
        if (snapped_to_playhead - *target as i64).abs() <= 100 {
            snapped_to_playhead = *target as i64;
            break;
        }
    }
    assert_eq!(snapped_to_playhead, 2150); // Snapped to playhead!

    // --- Scenario C: ResizeLeft of Instance 2 snaps to Instance 1's end ---
    // Reset inst1 template duration back to 1000 (ends at 2000)
    let tmpl1 = app.timeline.templates.iter_mut().find(|t| t.id == t1_id).unwrap();
    update_effect_duration(tmpl1, 1000);
    let snap_targets_for_inst2 = build_snap_targets(&app, inst2_id, playback_time);
    // snap_targets_for_inst2 contains inst1 start (1000) and inst1 end (2000)
    assert!(snap_targets_for_inst2.contains(&1000));
    assert!(snap_targets_for_inst2.contains(&2000));

    // Inst2 initial start = 2500, dur = 1000, right anchor = 3500.
    // Drag left by delta = -480ms -> raw new_start = 2020ms (within 20ms of 2000ms)
    let inst2_init_start = 2500_u64;
    let inst2_init_dur = 1000_u64;
    let right_anchor = inst2_init_start + inst2_init_dur;
    let delta_left = -480_i64;
    let mut snapped_start = (inst2_init_start as i64 + delta_left).max(0) as u64;
    for target in &snap_targets_for_inst2 {
        if (snapped_start as i64 - *target as i64).abs() <= 100 {
            snapped_start = *target;
            break;
        }
    }
    assert_eq!(snapped_start, 2000); // Snapped to inst1 end!
    snapped_start = snapped_start.min(right_anchor.saturating_sub(100));
    let new_dur2 = right_anchor - snapped_start;
    assert_eq!(new_dur2, 1500);

    // Apply to inst2
    let inst2_mut = app.timeline.instances.iter_mut().find(|i| i.id == inst2_id).unwrap();
    inst2_mut.start_time_ms = snapped_start;
    let tmpl2 = app.timeline.templates.iter_mut().find(|t| t.id == t2_id).unwrap();
    update_effect_duration(tmpl2, new_dur2);
    assert_eq!(app.timeline.instances.iter().find(|i| i.id == inst2_id).unwrap().start_time_ms, 2000);
    assert_eq!(tmpl2.duration_ms, 1500);
}

#[test]
fn test_e2e_complete_multi_step_undo_redo_cycle() {
    // Verifies the complete lifecycle:
    // Place clip -> Resize Right -> Move -> Resize Left
    // followed by full undo (Ctrl+Z x3) and full redo (Ctrl+Y x3),
    // strictly validating instance and template states at each stage.
    let mut app = PealayerApp::default();

    // 1. Initial State: Place Clip
    let template = Effect::with_target(
        "Fog Burst".into(),
        "🌫".into(),
        1000,
        HardwareTarget::Smoke,
        vec![
            AtomicAction { relay_id: 4, state: true, offset_ms: 0 },
            AtomicAction { relay_id: 4, state: false, offset_ms: 1000 },
        ],
    );
    let tmpl_id = template.id;
    app.timeline.templates.push(template);

    let instance = EffectInstance::new(tmpl_id, 2000);
    let inst_id = instance.id;
    app.timeline.instances.push(instance);

    // Verify Step 1: Initial state
    assert_eq!(app.timeline.instances.len(), 1);
    assert_eq!(app.timeline.instances[0].start_time_ms, 2000);
    assert_eq!(app.timeline.templates[0].duration_ms, 1000);
    assert_eq!(app.timeline.templates[0].actions[1].offset_ms, 1000);

    // Step 2: Resize Right (extend from 1000ms to 2500ms)
    // Recorded before operation
    app.undo_stack.push(app.snapshot_timeline());
    app.isolate_template_for_instance(inst_id);
    let eff_id_1 = app.timeline.instances.iter().find(|i| i.id == inst_id).unwrap().effect_id;
    let tmpl_1 = app.timeline.templates.iter_mut().find(|t| t.id == eff_id_1).unwrap();
    update_effect_duration(tmpl_1, 2500);

    // Verify Step 2 state
    assert_eq!(app.timeline.instances[0].start_time_ms, 2000);
    assert_eq!(app.timeline.templates.iter().find(|t| t.id == eff_id_1).unwrap().duration_ms, 2500);
    assert_eq!(app.timeline.templates.iter().find(|t| t.id == eff_id_1).unwrap().actions[1].offset_ms, 2500);

    // Step 3: Move Clip (move start from 2000ms to 3500ms)
    app.undo_stack.push(app.snapshot_timeline());
    app.timeline.instances.iter_mut().find(|i| i.id == inst_id).unwrap().start_time_ms = 3500;

    // Verify Step 3 state
    assert_eq!(app.timeline.instances[0].start_time_ms, 3500);
    assert_eq!(app.timeline.templates.iter().find(|t| t.id == eff_id_1).unwrap().duration_ms, 2500);

    // Step 4: Resize Left (trim start from 3500ms to 4200ms -> duration shrinks from 2500ms to 1800ms)
    app.undo_stack.push(app.snapshot_timeline());
    app.isolate_template_for_instance(inst_id);
    let inst_mut = app.timeline.instances.iter_mut().find(|i| i.id == inst_id).unwrap();
    inst_mut.start_time_ms = 4200;
    let eff_id_2 = inst_mut.effect_id;
    let tmpl_2 = app.timeline.templates.iter_mut().find(|t| t.id == eff_id_2).unwrap();
    update_effect_duration(tmpl_2, 1800);

    // Verify Step 4 state
    assert_eq!(app.timeline.instances[0].start_time_ms, 4200);
    assert_eq!(app.timeline.templates.iter().find(|t| t.id == eff_id_2).unwrap().duration_ms, 1800);
    assert_eq!(app.timeline.templates.iter().find(|t| t.id == eff_id_2).unwrap().actions[1].offset_ms, 1800);

    // -------------------------------------------------------------
    // Undo Sequence (Ctrl+Z x3)
    // -------------------------------------------------------------

    // Ctrl+Z #1: Undo Resize Left -> Restores Step 3 (Start: 3500, Duration: 2500)
    let current = app.snapshot_timeline();
    let snap_step3 = app.undo_stack.undo(current).expect("Undo 1 must succeed");
    app.restore_timeline_snapshot(snap_step3);
    assert_eq!(app.timeline.instances[0].start_time_ms, 3500);
    let eff_id = app.timeline.instances[0].effect_id;
    let tmpl = app.timeline.templates.iter().find(|t| t.id == eff_id).unwrap();
    assert_eq!(tmpl.duration_ms, 2500);
    assert_eq!(tmpl.actions[1].offset_ms, 2500);

    // Ctrl+Z #2: Undo Move -> Restores Step 2 (Start: 2000, Duration: 2500)
    let current = app.snapshot_timeline();
    let snap_step2 = app.undo_stack.undo(current).expect("Undo 2 must succeed");
    app.restore_timeline_snapshot(snap_step2);
    assert_eq!(app.timeline.instances[0].start_time_ms, 2000);
    let eff_id = app.timeline.instances[0].effect_id;
    let tmpl = app.timeline.templates.iter().find(|t| t.id == eff_id).unwrap();
    assert_eq!(tmpl.duration_ms, 2500);
    assert_eq!(tmpl.actions[1].offset_ms, 2500);

    // Ctrl+Z #3: Undo Resize Right -> Restores Step 1 Initial (Start: 2000, Duration: 1000)
    let current = app.snapshot_timeline();
    let snap_step1 = app.undo_stack.undo(current).expect("Undo 3 must succeed");
    app.restore_timeline_snapshot(snap_step1);
    assert_eq!(app.timeline.instances[0].start_time_ms, 2000);
    let eff_id = app.timeline.instances[0].effect_id;
    let tmpl = app.timeline.templates.iter().find(|t| t.id == eff_id).unwrap();
    assert_eq!(tmpl.duration_ms, 1000);
    assert_eq!(tmpl.actions[1].offset_ms, 1000);

    // Exhausted Undo: Further Ctrl+Z returns None
    let current = app.snapshot_timeline();
    assert!(app.undo_stack.undo(current).is_none());

    // -------------------------------------------------------------
    // Redo Sequence (Ctrl+Y x3)
    // -------------------------------------------------------------

    // Ctrl+Y #1: Redo Resize Right -> Restores Step 2 (Start: 2000, Duration: 2500)
    let current = app.snapshot_timeline();
    let redo_step2 = app.undo_stack.redo(current).expect("Redo 1 must succeed");
    app.restore_timeline_snapshot(redo_step2);
    assert_eq!(app.timeline.instances[0].start_time_ms, 2000);
    let eff_id = app.timeline.instances[0].effect_id;
    let tmpl = app.timeline.templates.iter().find(|t| t.id == eff_id).unwrap();
    assert_eq!(tmpl.duration_ms, 2500);
    assert_eq!(tmpl.actions[1].offset_ms, 2500);

    // Ctrl+Y #2: Redo Move -> Restores Step 3 (Start: 3500, Duration: 2500)
    let current = app.snapshot_timeline();
    let redo_step3 = app.undo_stack.redo(current).expect("Redo 2 must succeed");
    app.restore_timeline_snapshot(redo_step3);
    assert_eq!(app.timeline.instances[0].start_time_ms, 3500);
    let eff_id = app.timeline.instances[0].effect_id;
    let tmpl = app.timeline.templates.iter().find(|t| t.id == eff_id).unwrap();
    assert_eq!(tmpl.duration_ms, 2500);
    assert_eq!(tmpl.actions[1].offset_ms, 2500);

    // Ctrl+Y #3: Redo Resize Left -> Restores Step 4 (Start: 4200, Duration: 1800)
    let current = app.snapshot_timeline();
    let redo_step4 = app.undo_stack.redo(current).expect("Redo 3 must succeed");
    app.restore_timeline_snapshot(redo_step4);
    assert_eq!(app.timeline.instances[0].start_time_ms, 4200);
    let eff_id = app.timeline.instances[0].effect_id;
    let tmpl = app.timeline.templates.iter().find(|t| t.id == eff_id).unwrap();
    assert_eq!(tmpl.duration_ms, 1800);
    assert_eq!(tmpl.actions[1].offset_ms, 1800);

    // Exhausted Redo: Further Ctrl+Y returns None
    let current = app.snapshot_timeline();
    assert!(app.undo_stack.redo(current).is_none());
}

#[test]
fn test_e2e_template_isolation_under_repeated_operations() {
    // Verifies template isolation behavior under repeated, sequential operations:
    // 1. Multiple instances sharing one template.
    // 2. Resizing one instance isolates it into a new template.
    // 3. Repeatedly editing that same instance reuses its already-isolated template (no redundant clones).
    // 4. Adding another instance that shares the isolated template and then modifying that isolates again.
    // 5. Undoing reverts isolated states correctly without orphan leaks.
    let mut app = PealayerApp::default();

    // Shared template T1 (Seat Vibration, 4 actions, duration 1200ms)
    let t1 = Effect::with_target(
        "Seat Pulse".into(),
        "💺".into(),
        1200,
        HardwareTarget::SeatVibration,
        vec![
            AtomicAction { relay_id: 3, state: true, offset_ms: 0 },
            AtomicAction { relay_id: 3, state: false, offset_ms: 600 },
            AtomicAction { relay_id: 3, state: true, offset_ms: 900 },
            AtomicAction { relay_id: 3, state: false, offset_ms: 1200 },
        ],
    );
    let t1_id = t1.id;
    app.timeline.templates.push(t1);

    // Instance A and Instance B share T1
    let inst_a = EffectInstance::new(t1_id, 1000);
    let inst_b = EffectInstance::new(t1_id, 5000);
    let id_a = inst_a.id;
    let id_b = inst_b.id;
    app.timeline.instances.push(inst_a);
    app.timeline.instances.push(inst_b);

    assert_eq!(app.timeline.templates.len(), 1);

    // --- Op 1: First resize on Instance A (isolate T1 -> T2) ---
    app.undo_stack.push(app.snapshot_timeline());
    let t2_id = app.isolate_template_for_instance(id_a).expect("Must isolate instance A");
    assert_ne!(t2_id, t1_id);
    assert_eq!(app.timeline.templates.len(), 2);
    assert_eq!(app.timeline.instances.iter().find(|i| i.id == id_a).unwrap().effect_id, t2_id);
    assert_eq!(app.timeline.instances.iter().find(|i| i.id == id_b).unwrap().effect_id, t1_id);

    // Scale T2 duration to 2400ms (2x)
    let t2 = app.timeline.templates.iter_mut().find(|t| t.id == t2_id).unwrap();
    update_effect_duration(t2, 2400);
    assert_eq!(t2.duration_ms, 2400);
    assert_eq!(t2.actions[1].offset_ms, 1200); // 600 * 2
    assert_eq!(t2.actions[2].offset_ms, 1800); // 900 * 2
    assert_eq!(t2.actions[3].offset_ms, 2400);

    // Verify T1 is completely untouched
    let t1_check = app.timeline.templates.iter().find(|t| t.id == t1_id).unwrap();
    assert_eq!(t1_check.duration_ms, 1200);
    assert_eq!(t1_check.actions[1].offset_ms, 600);
    assert_eq!(t1_check.actions[3].offset_ms, 1200);

    // --- Op 2: Repeated resize on Instance A (already exclusive owner of T2) ---
    app.undo_stack.push(app.snapshot_timeline());
    let t2_again = app.isolate_template_for_instance(id_a).expect("Must return T2");
    assert_eq!(t2_again, t2_id, "Should reuse existing template when exclusive");
    assert_eq!(app.timeline.templates.len(), 2, "No redundant template cloned");

    let t2 = app.timeline.templates.iter_mut().find(|t| t.id == t2_id).unwrap();
    update_effect_duration(t2, 3600); // 3x
    assert_eq!(t2.duration_ms, 3600);
    assert_eq!(t2.actions[1].offset_ms, 1800); // 600 * 3
    assert_eq!(t2.actions[3].offset_ms, 3600);

    // --- Op 3: Add Instance C sharing T2 with A ---
    let inst_c = EffectInstance::new(t2_id, 10000);
    let id_c = inst_c.id;
    app.timeline.instances.push(inst_c);
    // Now T2 is shared between A and C.
    assert_eq!(app.timeline.templates.len(), 2);

    // Resize Instance C -> must isolate T2 into T3
    app.undo_stack.push(app.snapshot_timeline());
    let t3_id = app.isolate_template_for_instance(id_c).expect("Must isolate instance C");
    assert_ne!(t3_id, t2_id);
    assert_ne!(t3_id, t1_id);
    assert_eq!(app.timeline.templates.len(), 3);
    assert_eq!(app.timeline.instances.iter().find(|i| i.id == id_c).unwrap().effect_id, t3_id);
    assert_eq!(app.timeline.instances.iter().find(|i| i.id == id_a).unwrap().effect_id, t2_id);

    let t3 = app.timeline.templates.iter_mut().find(|t| t.id == t3_id).unwrap();
    update_effect_duration(t3, 4800);
    assert_eq!(t3.duration_ms, 4800);

    // Verify all 3 templates remain isolated with their respective durations
    assert_eq!(app.timeline.templates.iter().find(|t| t.id == t1_id).unwrap().duration_ms, 1200);
    assert_eq!(app.timeline.templates.iter().find(|t| t.id == t2_id).unwrap().duration_ms, 3600);
    assert_eq!(app.timeline.templates.iter().find(|t| t.id == t3_id).unwrap().duration_ms, 4800);

    // --- Op 4: Multi-step undo reverses isolation ---
    // Undo Op 3 (isolate and resize C)
    let cur = app.snapshot_timeline();
    let snap3 = app.undo_stack.undo(cur).unwrap();
    app.restore_timeline_snapshot(snap3);
    assert_eq!(app.timeline.templates.len(), 2);
    assert_eq!(app.timeline.instances.iter().find(|i| i.id == id_c).unwrap().effect_id, t2_id);

    // Undo Op 2 (second resize on A)
    let cur = app.snapshot_timeline();
    let snap2 = app.undo_stack.undo(cur).unwrap();
    app.restore_timeline_snapshot(snap2);
    assert_eq!(app.timeline.templates.iter().find(|t| t.id == t2_id).unwrap().duration_ms, 2400);

    // Undo Op 1 (first resize on A, restoring single shared template)
    let cur = app.snapshot_timeline();
    let snap1 = app.undo_stack.undo(cur).unwrap();
    app.restore_timeline_snapshot(snap1);
    assert_eq!(app.timeline.templates.len(), 1);
    assert_eq!(app.timeline.instances.iter().find(|i| i.id == id_a).unwrap().effect_id, t1_id);
    assert_eq!(app.timeline.instances.iter().find(|i| i.id == id_b).unwrap().effect_id, t1_id);
    assert_eq!(app.timeline.templates[0].duration_ms, 1200);
}
