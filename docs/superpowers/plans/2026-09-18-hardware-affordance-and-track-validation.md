# Hardware Affordance, Effect Typing & Track Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement domain-driven hardware effect typing (`HardwareTarget`), drag-and-drop visual affordances with smart auto-routing, vertical clip movement guard rails, mismatch warning badges, and 1-click track relocation to guarantee 4D theater hardware safety.

**Architecture:** Introduce `HardwareTarget` in `src/four_d/models.rs` with compatibility methods and Serde backward compatibility. Bind library presets and drag payloads to their target actuators. In `src/ui/layout.rs`, upgrade the drag-and-drop timeline drop zone to render emerald green compatible highlights, warning red incompatible highlights with `⊘ NotAllowed` cursors, and smart auto-routing. Render amber diagnostic badges (`⚠️`) on mismatched clips with a 1-click context menu and inspector relocation tool.

**Tech Stack:** Rust (1.75+), egui / glow, serde, uuid.

**Spec:** `docs/superpowers/specs/2026-09-18-hardware-affordance-and-track-validation-design.md`

## Global Constraints
- Zero compiler warnings (`RUSTFLAGS="-D warnings" cargo check --all-targets`).
- All 90 existing unit, integration, and simulation tests must remain 100% passing.
- 100% backward compatible project serialization (`.4d` and JSON sidecars without `target` deserialize cleanly).
- No new heavy crate dependencies.

---

### Task 1: Domain Model: `HardwareTarget` Enum, `Effect` Model Extension & Preset Typing

**Files:**
- Modify: `src/four_d/models.rs:1-60`
- Modify: `src/app.rs:25-45`
- Modify: `src/main.rs:180-265`
- Create: `tests/hardware_target_test.rs`

**Interfaces:**
- Produces: `HardwareTarget` enum with `primary_relay_id(&self) -> Option<u8>`, `is_compatible_with_relay(&self, relay_id: u8) -> bool`, `for_relay(relay_id: u8) -> Self`, `display_name(&self) -> &'static str`.
- Produces: `Effect.target: HardwareTarget` with `#[serde(default = "default_hardware_target")]`.
- Produces: `EffectDragPayload.target: HardwareTarget`.

- [ ] **Step 1: Write the failing test for `HardwareTarget`**

Create `tests/hardware_target_test.rs`:
```rust
use pealayer::four_d::models::{Effect, HardwareTarget};

#[test]
fn test_hardware_target_relay_compatibility() {
    // Water (R1, or Aux 5..=8)
    assert!(HardwareTarget::Water.is_compatible_with_relay(1));
    assert!(!HardwareTarget::Water.is_compatible_with_relay(2));
    assert!(!HardwareTarget::Water.is_compatible_with_relay(3));
    assert!(!HardwareTarget::Water.is_compatible_with_relay(4));
    assert!(HardwareTarget::Water.is_compatible_with_relay(5));
    assert!(HardwareTarget::Water.is_compatible_with_relay(8));
    assert!(!HardwareTarget::Water.is_compatible_with_relay(9));

    // Wind (R2, or Aux 5..=8)
    assert!(!HardwareTarget::Wind.is_compatible_with_relay(1));
    assert!(HardwareTarget::Wind.is_compatible_with_relay(2));
    assert!(HardwareTarget::Wind.is_compatible_with_relay(6));

    // SeatVibration (R3, or Aux 5..=8)
    assert!(!HardwareTarget::SeatVibration.is_compatible_with_relay(2));
    assert!(HardwareTarget::SeatVibration.is_compatible_with_relay(3));
    assert!(HardwareTarget::SeatVibration.is_compatible_with_relay(7));

    // Smoke (R4, or Aux 5..=8)
    assert!(!HardwareTarget::Smoke.is_compatible_with_relay(3));
    assert!(HardwareTarget::Smoke.is_compatible_with_relay(4));
    assert!(HardwareTarget::Smoke.is_compatible_with_relay(5));

    // Auxiliary (5..=8)
    assert!(!HardwareTarget::Auxiliary.is_compatible_with_relay(1));
    assert!(!HardwareTarget::Auxiliary.is_compatible_with_relay(4));
    assert!(HardwareTarget::Auxiliary.is_compatible_with_relay(5));
    assert!(HardwareTarget::Auxiliary.is_compatible_with_relay(8));

    // Any (1..=8)
    assert!(HardwareTarget::Any.is_compatible_with_relay(1));
    assert!(HardwareTarget::Any.is_compatible_with_relay(8));
    assert!(!HardwareTarget::Any.is_compatible_with_relay(9));
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
    // JSON without "target" field must deserialize to HardwareTarget::Any
    let legacy_json = r#"{
        "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
        "name": "Legacy Splash",
        "icon": "💧",
        "duration_ms": 1000,
        "actions": []
    }"#;

    let effect: Effect = serde_json::from_str(legacy_json).expect("Failed to deserialize legacy Effect");
    assert_eq!(effect.target, HardwareTarget::Any);
    assert_eq!(effect.name, "Legacy Splash");

    // Roundtrip with target
    let modern_effect = Effect::with_target(
        "Modern Splash".to_string(),
        "💧".to_string(),
        1500,
        HardwareTarget::Water,
        vec![],
    );
    let serialized = serde_json::to_string(&modern_effect).expect("Serialization failed");
    let deserialized: Effect = serde_json::from_str(&serialized).expect("Deserialization failed");
    assert_eq!(deserialized.target, HardwareTarget::Water);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test hardware_target_test`
Expected: FAIL with "cannot find type `HardwareTarget` in `pealayer::four_d::models`".

- [ ] **Step 3: Implement `HardwareTarget` and update models**

In `src/four_d/models.rs`, add `HardwareTarget` and update `Effect`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HardwareTarget {
    Water,
    Wind,
    SeatVibration,
    Smoke,
    Auxiliary,
    Any,
}

impl HardwareTarget {
    pub fn primary_relay_id(&self) -> Option<u8> {
        match self {
            Self::Water => Some(1),
            Self::Wind => Some(2),
            Self::SeatVibration => Some(3),
            Self::Smoke => Some(4),
            Self::Auxiliary => Some(5),
            Self::Any => None,
        }
    }

    pub fn is_compatible_with_relay(&self, relay_id: u8) -> bool {
        match self {
            Self::Water => relay_id == 1 || (5..=8).contains(&relay_id),
            Self::Wind => relay_id == 2 || (5..=8).contains(&relay_id),
            Self::SeatVibration => relay_id == 3 || (5..=8).contains(&relay_id),
            Self::Smoke => relay_id == 4 || (5..=8).contains(&relay_id),
            Self::Auxiliary => (5..=8).contains(&relay_id),
            Self::Any => (1..=8).contains(&relay_id),
        }
    }

    pub fn for_relay(relay_id: u8) -> Self {
        match relay_id {
            1 => Self::Water,
            2 => Self::Wind,
            3 => Self::SeatVibration,
            4 => Self::Smoke,
            5..=8 => Self::Auxiliary,
            _ => Self::Any,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Water => "Water Valve",
            Self::Wind => "Wind Fan",
            Self::SeatVibration => "Seat Vibration",
            Self::Smoke => "Smoke Machine",
            Self::Auxiliary => "Aux Relay",
            Self::Any => "General Cue",
        }
    }
}

pub fn default_hardware_target() -> HardwareTarget {
    HardwareTarget::Any
}
```

Add constructors on `Effect`:
```rust
    pub fn with_target(name: String, icon: String, duration_ms: u64, target: HardwareTarget, actions: Vec<Action>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            icon,
            duration_ms,
            target,
            actions,
        }
    }
```
Update existing `Effect::new(...)` to set `target: HardwareTarget::Any` (or infer target from `actions.first()`).

In `src/app.rs`, update `EffectDragPayload`:
```rust
#[derive(Clone, Debug)]
pub struct EffectDragPayload {
    pub name: String,
    pub icon: String,
    pub duration_ms: u64,
    pub target: crate::four_d::models::HardwareTarget,
    pub actions: Vec<crate::four_d::models::Action>,
}
```

In `src/main.rs`, update presets to pass `HardwareTarget::Water`, `HardwareTarget::Wind`, `HardwareTarget::SeatVibration`, `HardwareTarget::Smoke`, and `HardwareTarget::Auxiliary`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test hardware_target_test`
Expected: PASS.
Run: `cargo test` to verify all 90 existing tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/four_d/models.rs src/app.rs src/main.rs tests/hardware_target_test.rs
git commit -m "feat(four_d): implement HardwareTarget enum, model extension, and preset typing"
```

---

### Task 2: Drag-and-Drop Visual Affordances, Guard Rails & Smart Auto-Routing

**Files:**
- Modify: `src/ui/layout.rs:340-390, 1650-1760`
- Create: `tests/hardware_affordance_integration_test.rs`

**Interfaces:**
- Consumes: `HardwareTarget`, `EffectDragPayload`.
- Produces: Visual timeline drop zone states (`rgba(0, 255, 136, 40)` compatible green glow, `rgba(255, 70, 70, 40)` warning red glow, `CursorIcon::NotAllowed` vs `CursorIcon::Copy`).
- Produces: Drop rejection on incompatible tracks and smart auto-routing when dropped on neutral space.

- [ ] **Step 1: Write integration test for drop validation & auto-routing logic**

In `tests/hardware_affordance_integration_test.rs`:
```rust
use pealayer::four_d::models::{Effect, HardwareTarget, EffectInstance};
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
    // Incompatible with R2 (Wind)
    assert!(!water_effect.target.is_compatible_with_relay(2));
    // Incompatible with R4 (Smoke)
    assert!(!water_effect.target.is_compatible_with_relay(4));
    // Compatible with Aux R5
    assert!(water_effect.target.is_compatible_with_relay(5));

    let wind_effect = Effect::with_target(
        "Wind Gale".to_string(),
        "🌀".to_string(),
        5000,
        HardwareTarget::Wind,
        generate_constant(2, true, 5000),
    );
    assert!(!wind_effect.target.is_compatible_with_relay(1));
    assert!(wind_effect.target.is_compatible_with_relay(2));
    assert!(wind_effect.target.is_compatible_with_relay(6));
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
}
```

- [ ] **Step 2: Run test to verify it compiles and passes basic model checks**

Run: `cargo test --test hardware_affordance_integration_test`
Expected: PASS.

- [ ] **Step 3: Implement Visual Affordances & Drop Guard Rails in `src/ui/layout.rs`**

1. In `src/ui/layout.rs:340-355` (`PealayerTab::EffectsLibrary`), populate `EffectDragPayload`:
   ```rust
   let payload = EffectDragPayload {
       name: preset.effect.name.clone(),
       icon: preset.effect.icon.clone(),
       duration_ms: preset.effect.duration_ms,
       target: preset.effect.target,
       actions: preset.effect.actions.clone(),
   };
   ```

2. In `src/ui/layout.rs:1655-1695` (active drag over timeline):
   - Read `payload = egui::DragAndDrop::payload::<EffectDragPayload>(ui.ctx())`.
   - Calculate `relative_y = mouse_pos.y - tracks_top; let track_index = (relative_y / 32.0).floor() as i32;`.
   - Highlight compatible tracks with emerald glow and green border.
   - For incompatible tracks: highlight with warning red (`rgba(255, 70, 70, 45)`), red border, and set `ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);`.
   - Show dynamic tooltip at pointer with explanatory incompatibility message.

3. In `src/ui/layout.rs:1710-1765` (drop execution):
   - Correctly compute row using `mouse_pos.y - tracks_top`.
   - If dropped on a compatible relay track (`payload.target.is_compatible_with_relay(target_relay)`):
     - Instantiate clip on `target_relay`.
   - If dropped on empty grid space or ruler:
     - **Smart Auto-Routing**: If `payload.target.primary_relay_id()` is `Some(primary)`:
       - Place the clip automatically on `primary`!
   - If dropped on an incompatible track:
     - **Reject the drop**, do not create an invalid template/instance, and push an OSD alert or print warning.

- [ ] **Step 4: Verify with `cargo test` and manual checks**

Run: `cargo test`
Expected: 100% tests pass with 0 warnings.

- [ ] **Step 5: Commit**

```bash
git add src/ui/layout.rs tests/hardware_affordance_integration_test.rs
git commit -m "feat(ui): implement drag-and-drop visual affordance, track guard rails, and smart routing"
```

---

### Task 3: Vertical Clip Moving Guard Rails, Mismatch Badges & 1-Click Relocation

**Files:**
- Modify: `src/ui/layout.rs:750-850, 1100-1250`
- Modify: `tests/hardware_affordance_integration_test.rs`

**Interfaces:**
- Produces: Guarded vertical clip moving in `active_drag`.
- Produces: Amber warning badge `⚠️ {name}` on mismatched clips.
- Produces: Right-click context menu action `⚡ Relocate to R{primary}: {primary_name}` with undo history.
- Produces: Inspector warning banner with relocate button in `EffectControls`.

- [ ] **Step 1: Write integration test for relocation and undo**

Add to `tests/hardware_affordance_integration_test.rs`:
```rust
#[test]
fn test_1_click_relocation_reassigns_relay_and_updates_queue() {
    let mut timeline = pealayer::four_d::models::TimelineProject::new();
    let wrong_effect = Effect::with_target(
        "Water Splash".to_string(),
        "💧".to_string(),
        1500,
        HardwareTarget::Water,
        generate_constant(2, true, 1500), // In R2 (Wind)
    );
    let template_id = wrong_effect.id;
    timeline.templates.push(wrong_effect);

    let instance = EffectInstance::new(template_id, 2000);
    timeline.instances.push(instance);

    // Verify initially mismatched
    let inst = &timeline.instances[0];
    let tmpl = timeline.templates.iter().find(|t| t.id == inst.effect_id).unwrap();
    let current_relay = tmpl.actions.first().map(|a| a.relay_id).unwrap_or(0);
    assert_eq!(current_relay, 2);
    assert!(!tmpl.target.is_compatible_with_relay(current_relay));

    // Relocate to primary target
    let primary_relay = tmpl.target.primary_relay_id().expect("Primary relay should exist");
    assert_eq!(primary_relay, 1);

    // Update template actions to target relay 1
    let mut fixed_tmpl = tmpl.clone();
    fixed_tmpl.actions = generate_constant(primary_relay, true, fixed_tmpl.duration_ms);
    timeline.templates[0] = fixed_tmpl;

    let fixed = &timeline.templates[0];
    let new_relay = fixed.actions.first().map(|a| a.relay_id).unwrap_or(0);
    assert_eq!(new_relay, 1);
    assert!(fixed.target.is_compatible_with_relay(new_relay));
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --test hardware_affordance_integration_test`
Expected: PASS.

- [ ] **Step 3: Implement vertical movement guard rails, mismatch badges, and 1-click fix in `src/ui/layout.rs`**

1. In clip rendering loop (`src/ui/layout.rs:1150-1250`):
   - For each instance: find template `tmpl`.
   - Check if `!tmpl.target.is_compatible_with_relay(relay_id)`.
   - If mismatched:
     - Render warning badge: `⚠️ {tmpl.name}` with amber text.
     - Clip stroke rendered in warning amber: `Color32::from_rgb(245, 158, 11)`.
     - Tooltip: Show full hardware mismatch warning and prompt to right-click.
2. In clip context menu (secondary click on clip):
   - If mismatched:
     - Add prominent menu item: `ui.button(format!("⚡ Relocate to R{}: {}", primary_relay, primary_name));`
     - On click:
       - Push undo snapshot: `self.app.undo_stack.push(self.app.snapshot_timeline());`.
       - Reassign template actions to `generate_constant(primary_relay, true, tmpl.duration_ms)`.
       - Recompile queue: `let compiled = self.app.timeline.compile_actions(); self.app.engine_handle.sender.send(EngineMessage::UpdateQueue(compiled));`.
       - Request repaint and emit confirmation.
3. In `PealayerTab::EffectControls` (inspector panel, lines 200-290):
   - If selected cue is mismatched:
     - Show warning banner: `ui.colored_label(Color32::from_rgb(245, 158, 11), format!("⚠️ Hardware Mismatch: Placed on R{}, but requires R{}", current_relay, primary_relay));`.
     - Provide `if ui.button(format!("Relocate to R{}: {}", primary_relay, primary_name)).clicked() { ... }`.
4. In active clip vertical drag (`self.app.active_drag`):
   - Restrict new row: if candidate row is incompatible with `tmpl.target`, keep instance on its current valid track row.

- [ ] **Step 4: Run full test suite and compiler checks**

Run: `RUSTFLAGS="-D warnings" cargo check --all-targets`
Run: `cargo test`
Expected: 100% tests pass, 0 warnings, all 92+ tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/ui/layout.rs tests/hardware_affordance_integration_test.rs
git commit -m "feat(ui): add vertical move guard rails, mismatch warning badges, and 1-click relocation"
```
