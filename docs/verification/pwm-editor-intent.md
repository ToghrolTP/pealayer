# PWM editor intent, not telemetry feedback

The native editor's egui slider used always-clamping with a 0.1% step.
egui 0.36.2 rounds the incoming value during rendering and marks that response
changed. A factual 50.013% readback therefore became a live 50% command without
an operator edit. Repeating authoritative readbacks could repeat these writes.
Numeric blur also committed unconditionally, including unchanged rounded text.

The shared editor now clamps/rounds only edits. A per-widget dirty editing
session retains the operator's draft across telemetry updates, commits actual
edits on blur/Enter/drag release, and never sends an unchanged blur. Escape
cancels a pending commit, not an already acknowledged physical command. The
bounded live drag cadence and final convergence write remain unchanged.

Channel detail initialization uses the complete channel observation and the
coordinator's logical percentage. The last-single-channel marker is not the
only channel's state. Unknown observations remain unknown in the shared lookup;
opening or displaying an editor does not authorize an output write.

Regressions cover fractional idle renders producing no engine messages, a
100% commit followed by unchanged focus/blur producing no zero/duplicate,
positive drafts retained through stale telemetry, cancel, and selecting channel
2 from the full vector while another channel is the last single-channel update.
Tests and compilation are hosted-only for this pass. Source checks are not
installed native or physical MOSFET acceptance.

This is a confirmed Pealayer input/feedback defect, not a claim about electrical
behavior at full duty. Concurrent writes were also observed after Pealayer was
quit, so the coordinator and other clients require independent investigation.
Preserve existing RF/motion mappings and use only the human-approved output
under the runtime owner's exclusive control during final acceptance.
