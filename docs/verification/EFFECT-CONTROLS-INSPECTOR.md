# Effect Controls inspector refinement

Owning trackers: [#80](https://github.com/ToghrolTP/pealayer/issues/80),
[Web parity #63](https://github.com/ToghrolTP/pealayer/issues/63).

## Changes

- Docked Effect Controls and native Cue properties continue to call one shared
  renderer, with the existing timeline edits, undo, routing and instance isolation.
- Neutral cards replace the large permanent accent wash. Selection uses a small
  accent edge; the temporary selection ping has constant stroke geometry.
- The cue name is edited once, rather than repeated above its input. Bounded
  headings truncate with complete tooltips. Previous/next selection stays available.
- Start and duration have exact millisecond-backed unit-aware fields, explicit
  decrement/increment buttons, and Ctrl/Command fine / Shift coarse adjustments.
  They appear side-by-side when space permits and stack in narrow panels.
  The full-width placement overview is informational, not a tiny seeking slider.
- State markers have no duration editor. Recorded effects retain fixed duration.
  Resizable holds/ramps retain instance-local duration edits and show their end time.
- Relay/PWM behavior uses a full-width selector. Start value precedes exit/ramp
  value; PWM uses the same slider/numeric control as Hardware Monitor. Relay
  buttons use consistent semantic colors and explain that editing a cue does not
  itself toggle hardware. Unavailable live feedback is not reported as Off.
- Copyable identifiers are collapsed by default. Navigate, duplicate and delete
  actions remain available. No cue, effect, channel or storage contract was removed.
- The Web direct-cue dialog uses matching identity/timing/output sections,
  responsive fields and semantic state buttons, through its existing Rust commands.

## Verification boundaries

Web TypeScript and production/PWA bundle completed. Focused native geometry
regressions were added for exact-time fields and selection-ping width; their
execution and slow CI waits are deferred at the user's request. Native package,
installed host identities and visual acceptance are recorded in the deployment
checkpoint below when verified, not inferred from source.

This is not a claim that every Web sequence/controller-effect editor is now
equivalent to the native dock inspector. This pass refines the existing direct
cue editor; remaining full-editor parity stays tracked in #63.
