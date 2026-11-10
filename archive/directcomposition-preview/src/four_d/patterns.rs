use crate::four_d::models::AtomicAction;

/// Generates a blinking pattern for a relay
pub fn generate_blink(relay_id: u8, interval_ms: u64, duration_ms: u64) -> Vec<AtomicAction> {
    let mut actions = Vec::new();
    let mut current_offset = 0;
    let mut state = true; // Start with ON

    while current_offset < duration_ms {
        actions.push(AtomicAction {
            relay_id,
            state,
            offset_ms: current_offset,
        });

        current_offset += interval_ms;
        state = !state;
    }

    // Ensure it ends up OFF if it's currently ON and we've passed the duration
    // Actually, maybe we shouldn't force an OFF here if the engine resets, but it's cleaner
    // to have effects clean up after themselves. Let's add an explicit OFF at the end if it's not already.
    if state {
        actions.push(AtomicAction {
            relay_id,
            state: false,
            offset_ms: duration_ms,
        });
    }

    actions
}

/// Generates a constant state (usually ON) for the duration
pub fn generate_constant(relay_id: u8, state: bool, duration_ms: u64) -> Vec<AtomicAction> {
    vec![
        AtomicAction {
            relay_id,
            state,
            offset_ms: 0,
        },
        AtomicAction {
            relay_id,
            state: !state,
            offset_ms: duration_ms,
        },
    ]
}
