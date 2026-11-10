# Media-bound cue timing and scrub preview

Pealayer's independent libmpv observer supplies the presentation clock.
Playback prepares a complete plan with PCController and requires a matching
clock/plan acknowledgement before effects can run. Deadline faults stop
playback; they are not silently converted into successful output.

The local clock now retains integer microseconds. The currently deployed
PCController contract uses integer milliseconds and strict JSON decoding.
Pealayer therefore sends extended timing fields only when
`controller.media.timeline.get` explicitly advertises
`clock_unit: "microseconds"`. Until that controller contract is implemented,
millisecond payloads remain authoritative on the wire. This is not a claim
of microsecond hardware precision or hard real-time guarantees.

## Direct PWM cues

Select a direct PWM cue in the timeline and open Effect Controls. Its envelope
can hold a value, fade between start/end values, blink, or breathe. Fade and
breathe offer linear, quadratic in/out, in/out, smooth-step and step easing.
Drag the endpoint handles in the graph or enter percentages in the controls.
Blink adds a cycle duration and duty percentage. The cue remains resizable.
The same value evaluator drives the graph, live playback, scrub preview and
prepared plan; there is no separately authored curve for each consumer.

Prepared PWM plans sample changing values at approximately 30 Hz while
retaining cue boundaries, blink edges, cycle extrema and authored keyframe
times. Per-track latency compensation advances or delays dispatch without
moving the authored cue on the timeline. Positive compensation dispatches
earlier; negative compensation delays it.

## Scrubbing

In a supported hardware track's menu, use **Preview output while scrubbing**.
PWM defaults to enabled; relays require explicit opt-in. Seeking evaluates
the state at the target time and coalesces channel updates. It does not replay
all actions crossed by the seek. Releasing the scrub sends the final state.
E-STOP blocks these writes.

Motion and opaque controller-owned programs cannot yet be safely reconstructed
at an arbitrary point, so their scrub-preview control is unavailable. Their
normal prepared playback remains unchanged. Web controls for the new envelope
editor and per-track preview settings still need to be exposed through the
shared command contract; the native editor is the implemented surface here.

## Verification and remaining acceptance

Pure state-evaluation tests cover fade/easing, blink duty, breathe cycles,
preview defaults and disable policy. Prepared-plan tests cover exact authored
blink boundaries and latency-adjusted dispatch timestamps. Contract tests
verify strict millisecond peers receive no unsupported fields.

Physical onset accuracy, transport latency, loaded-host stress, opt-in relay
preview and Web editing parity still require acceptance. Controller/firmware
timestamp scheduling and microsecond clock advertisement are the next contract
steps; the current firmware should not be described as exact to one microsecond.
