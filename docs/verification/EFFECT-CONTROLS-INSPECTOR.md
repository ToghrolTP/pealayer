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

## Installed checkpoint — 9 October 2026

Runtime source: `5db40781b9be238d3fdd5f5f5c4650f7d0c9acaa`, clean.
PR: [#100](https://github.com/ToghrolTP/pealayer/pull/100), based on main after
the branding PR and its test-constructor repair were integrated without rebase.

- TypeScript, production bundle and PWA generation passed. Final PWA identity:
  `8ff4f6138520ee78`, 39 precached resources. Existing chunk-size warning remains.
- Locked native release builds on David took 1m 38s and 1m 22s; the second includes
  the exact 50 ms Web save correction. Fourteen existing compiler warnings remain.
- Seven Windows quick-action resources verified. Packed executable integrity
  passed; both independent David/Cafe staged runtime smoke checks exited 0.
- Executable: 11,145,728 bytes; SHA-256
  `c48aae7de89214e4dba2cc51c891cb69e9cce4542bd4f21880d95e01e3b128aa`.
- Cafe then David installed through destination-runtime-validated API uploads
  and graceful updater restarts. Their live manifests independently report the
  exact clean runtime source and executable above, retaining their different DLLs.
- Cafe's interactive-session process responds, controller service is Running,
  hardware is connected, and paused media position plus all three cues persisted.
  David responds and remains a connected cache-only Cafe consumer, no local
  hardware scheduler, matching paused position and zero reported preview drift.
  No playback or physical output activation was requested for this UI pass.
- Erfan installed the same executable atomically while Pealayer was stopped,
  retained its own DLL, verified a rollback, and left KMPlayer running. Launch and
  playback acceptance there remain deferred under the user's explicit rule.
- Unit regressions remain unexecuted as requested; no slow CI was awaited.
- Native live visual acceptance and phone/tablet/desktop interactive visual
  checks remain pending. The read-only capture refused to capture a non-foreground
  window instead of changing focus or operating the user's desktop. The user was
  asked to bring an existing cue's Effect Controls into view for that check.
