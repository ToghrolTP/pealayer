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
