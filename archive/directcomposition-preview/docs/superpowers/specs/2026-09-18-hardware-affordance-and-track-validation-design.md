# Pealayer: Hardware Affordance, Effect Typing & Track Validation Design Specification

- **Date:** 2026-09-18
- **Status:** Approved
- **Domain:** NLE Timeline, Effects Library, 4D Hardware Safety, UX Guard Rails

---

## 1. Context & Motivation

In physical 4D cinema and theater show control systems, each physical track row on the timeline maps directly to a physical hardware relay (e.g. `R1` = Water Valve, `R2` = Wind Fan, `R3` = Seat Vibration, `R4` = Smoke Machine, `R5`–`R8` = Auxiliary Relays).

Previously:
1. Dragging an effect from the **Effects Library** (e.g., `Water Splash` or `Seat Rumble`) allowed dropping it onto any track row indiscriminately.
2. Dropping an effect rewritten its target relay ID to whatever row index the cursor landed on.
3. This led to dangerous physical cross-contamination (e.g., placing `Water Splash` on `R2: Wind Fan` blowing high-speed air during water scenes, or pulsing high-duty cycles on delicate smoke machine pumps).
4. Pre-existing projects lacked visual mismatch diagnostics or recovery mechanisms.

This specification introduces a domain-driven **Hardware Affordance & Validation Subsystem** to guarantee physical hardware safety, prevent authoring errors (Nielsen Heuristic #5: Error Prevention), and provide 1-click recovery tools (Nielsen Heuristic #9: Help Users Recognize, Diagnose, and Recover from Errors).

---

## 2. Architecture & Data Model

### 2.1. `HardwareTarget` Enum (`src/four_d/models.rs`)
We define a formal hardware category for theatrical actuators:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HardwareTarget {
    Water,          // Relay 1 (Water Valve / Mist Solenoids)
    Wind,           // Relay 2 (Wind Fan Blowers)
    SeatVibration,  // Relay 3 (Seat Rumble / Shakers)
    Smoke,          // Relay 4 (Smoke Machine / Fog Nozzles)
    Auxiliary,      // Relays 5..=8 (Auxiliary Triggers)
    Any,            // General-purpose / unconstrained cues
}

impl HardwareTarget {
    /// Returns the primary default hardware relay ID (1-based).
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

    /// Checks if a physical relay ID (1-based) is compatible with this target.
    ///
    /// Rules:
    /// - Water is strictly compatible with Relay 1, or Aux Relays (5..=8).
    /// - Wind is strictly compatible with Relay 2, or Aux Relays (5..=8).
    /// - SeatVibration is strictly compatible with Relay 3, or Aux Relays (5..=8).
    /// - Smoke is strictly compatible with Relay 4, or Aux Relays (5..=8).
    /// - Auxiliary is compatible with any Aux Relay (5..=8).
    /// - Any is compatible with all relays (1..=8).
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

    /// Infers the default HardwareTarget for a given relay ID.
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

    /// User-friendly label for badges and tooltips.
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
```

### 2.2. Model Extension & Backward Compatibility
In [`Effect`](file:///home/toghrol/Documents/rust/pealayer/src/four_d/models.rs):
```rust
fn default_hardware_target() -> HardwareTarget {
    HardwareTarget::Any
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effect {
    pub id: Uuid,
    pub name: String,
    pub icon: String,
    pub duration_ms: u64,
    #[serde(default = "default_hardware_target")]
    pub target: HardwareTarget,
    pub actions: Vec<Action>,
}
```
* **Serde Safety:** All legacy project files without a `target` field automatically deserialize cleanly with `HardwareTarget::Any` (or infer from `actions.first()`).

### 2.3. Preset & Drag Payload Typing
* `EffectPreset` and `EffectDragPayload` carry `pub target: HardwareTarget`.
* Presets initialized in `src/main.rs`:
  * `Water Splash`, `Mist Spray`: `HardwareTarget::Water`
  * `Wind Blast`, `Wind Gale`: `HardwareTarget::Wind`
  * `Seat Rumble`, `Seat Shake`: `HardwareTarget::SeatVibration`
  * `Smoke Blast`, `Fog Screen`: `HardwareTarget::Smoke`
  * `Aux Trigger A`, `Aux Trigger B`: `HardwareTarget::Auxiliary`

---

## 3. Interaction & UX Design System

### 3.1. Drag-and-Drop Active States (`src/ui/layout.rs`)
When dragging an effect from the library over the timeline canvas:
1. **Compatible Track Rows**:
   * Background: Translucent emerald green (`rgba(0, 255, 136, 40)`).
   * Border: 1.5px solid emerald green (`#00FF88`).
   * Cursor: `CursorIcon::Copy` (`+`).
   * Tooltip: *"Drop to place {name} on R{relay}: {display_name}"*.
2. **Incompatible Track Rows**:
   * Background: Translucent warning red (`rgba(255, 70, 70, 40)`).
   * Border: 1.0px solid warning red (`#FF4646`).
   * Cursor: `CursorIcon::NotAllowed` (`⊘`).
   * Tooltip: *"⊘ Incompatible Track: '{name}' cannot be placed on R{relay}. Use R{expected} or Aux."*.
3. **Primary Track Beacon**:
   * The designated primary track for the dragged effect displays an active guide border, directing the user's eye to the ideal row.

### 3.2. Drop Execution Rules
* **On Compatible Track:** Instantiates clip on that track, maintaining `target_relay` and duration.
* **On Empty Grid Space or Track Header:**
  * **Smart Auto-Routing:** If dropped on neutral space or top ruler, automatically places the clip on its primary hardware track (`target.primary_relay_id()`) at the drop timestamp.
* **On Incompatible Track:**
  * Drop is strictly rejected (no hazard created).
  * Emits an explanatory OSD warning: *"Placement Rejected: '{name}' cannot be placed on {track_name}"*.

### 3.3. Moving Existing Clips Vertically
* When dragging existing clips up or down:
  * The vertical row transition is tested against `effect.target.is_compatible_with_relay(new_relay)`.
  * If the candidate row is incompatible, vertical movement is clamped / rejected, maintaining the clip on its valid track.

### 3.4. Pre-Existing Mismatch Diagnostics & 1-Click Recovery
For legacy or misconfigured projects:
1. **Diagnostic Mismatch Detection**:
   * If `!effect.target.is_compatible_with_relay(instance_relay)`:
   * **Visual Badge**: Clip title prefixed with amber emblem: `⚠️ {name}`.
   * **Border**: Rendered with warning amber stroke (`#F59E0B`).
   * **Tooltip**: Explains the exact physical hazard and the recommended track.
2. **1-Click Relocation**:
   * **Right-Click Context Menu**:
     * Action: `⚡ Relocate to R{primary_id}: {primary_name}`
     * Executes atomic move, pushes undo snapshot to `undo_stack`, and recompiles engine queue.
   * **Effect Controls Inspector**:
     * Displays an amber hazard banner with a `[ Relocate to Recommended Track ]` button.

---

## 4. Test Strategy

1. **Unit Tests (`tests/hardware_target_test.rs`)**:
   * Test `HardwareTarget::is_compatible_with_relay` for all enum variants against relays 1..=8 and invalid relays.
   * Test `HardwareTarget::primary_relay_id` mappings.
   * Test Serde backward compatibility: deserializing legacy JSON without `target` field defaults to `Any`.
2. **UI & Drag Validation Tests (`tests/hardware_affordance_integration_test.rs`)**:
   * Verify smart auto-routing of empty space drops to primary relays.
   * Verify rejection of drops on incompatible tracks.
   * Verify 1-click relocation updates instance relay and engine queue.
3. **Regression Tests**:
   * Ensure all 90 existing tests pass with 0 compiler warnings (`-D warnings`).
