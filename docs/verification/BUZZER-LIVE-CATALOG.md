# Buzzer and live melody catalog

The Hardware Monitor exposes PCController's configured melodies, repeats or
continuous loop, immediate stop, actual playing/board-mute state, and collapsible
tone testing. The sequence editor can insert the same melodies as editable notes.

## Current contract and repaired gaps

- PCController's `controller.melodies.list` reads watched host configuration.
  Its `melodies.changed` event requests a fresh authoritative catalog; the
  engine retains the last good catalog on a malformed or failed read.
- Opening either native melody selector refreshes on the actual popup-opening
  edge, including keyboard activation. Both Web selectors share one searchable
  control and refresh each time it opens. This is an asynchronous authoritative
  pull, not a guarantee that a cached first paint is already the response.
- Native refresh, melody, tone and stop now use the existing session interop
  commands. Previously they called the local engine directly, which is
  deliberately disconnected when David consumes Cafe's Pealayer. Commands now
  reach the authority exactly like their Web/API counterparts.
- Low-rate `output` start/finish events request authoritative buzzer state;
  no playing state is inferred from prose. High-rate `buzzer.note` events do not
  cause catalog refreshes. Catalog-change bursts coalesce through the existing
  engine flag; reconnect and slow recovery refresh remain intact.
- Invalid catalog rows now reject the refresh instead of silently filtering to
  an incomplete or false-empty list. Unique names and bounded notes/durations
  follow the current contract. Last-good state survives failed refreshes.
- Native controls fit available width and use the current semantic palette.
  Web choices show icons, searchable names and right-aligned human durations;
  diagnostic controls collapse and small-width layouts stack. Board-unavailable
  play/tone/stop controls are disabled rather than accepted as inert actions.

## Verification gates

Focused event/parser/UI and Web parity regression checks are updated but remain
unrun under the user's test-suite/slow-CI deferral. Build/type-check, compatible
runtime smoke, exact installed identity, live catalog comparison, dropdown/input
acceptance and physical audibility are separate gates. Do not infer physical
buzzer sound from a successful command or use motion/relay outputs for this check.
Do not replace another host's libmpv or build Rust on production Cafe.

### Installed checkpoint — 9 October 2026

- [PR #98](https://github.com/ToghrolTP/pealayer/pull/98) is stacked on PR #97.
  Runtime source is the clean `a93b2315fee94d7853a48149a49490c3e03ca047`;
  the follow-up fixes the parity guard's file-reader reference, not runtime code.
  Main remains untouched; no PCController/firmware contract changes were needed.
- TypeScript and Vite/PWA builds passed; native incremental release finished
  in 1 minute 23 seconds. Seven Windows icon resources, UPX integrity and
  David/Cafe-retained-runtime startup smokes passed. Focused suites were not run.
- Canonical executable SHA-256:
  `d8f6e67489d76651089f945dd3291d9344dcbaf5ed34fd685131dfee121a494d`
  (11,126,272 bytes); embedded Web/PWA: `8497651db53865ae`.
- Cafe was updated through its graceful begin/chunk/finish API, confirmed healthy,
  then David was updated through its own process-local updater. Both APIs now
  report the exact source/hash/PWA, each retaining its own destination runtime.
  Hardware is connected without error; media remains paused at 660.800 seconds.
- A direct read-only `controller.melodies.list` on Cafe returned 12 definitions
  matching Pealayer's inventory. David's
  `pealayer.hardware.catalog.refresh` JSON-RPC was accepted, with no consumer
  command/transport error. David remains cache-only with no second scheduler
  and zero paused preview drift in the checked sample. These reads establish
  inventory/transport health, not physical buzzer sound or change-event latency.
- Erfan was updated offline by atomic replacement. Installed/rollback hashes and
  unchanged destination libmpv were verified. KMPlayer remains running; Pealayer
  was deliberately not launched there. Its interactive acceptance remains pending.
- Human acceptance requested for mouse/keyboard selector opening, layout and a
  short melody's audibility. Catalog-change event latency and phone/tablet/desktop
  visual inspection remain pending; responsive styles exist but screenshots or
  actual gestures have not been verified in this pass. No physical buzzer,
  motion, relay or PWM output was actuated by this pass.
