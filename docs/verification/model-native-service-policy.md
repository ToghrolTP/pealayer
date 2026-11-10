# Model-only fixtures and native service bootstrap

Related: RF delivery #181 / #182. Based on #182 checkpoint `6ade13f`.

The model-only default app used by UI tests already has a memory-only hardware
engine and OS-media manager. Its logic nevertheless started the process-global
external-player bridge, SFX worker, audio-device discovery and clipboard observer.
Those detached services retained the first fixture's playback/GUI context and
could overlap real libmpv probe creation/destruction with unrelated model tests.
This is a confirmed isolation defect, not a proven explanation for the Windows
GNU `0xc0000005` failures. Last printed test names do not establish causality.

`NativeServicePolicy` makes startup explicit: the real startup constructor uses
Production, while model-only Default uses ModelOnly. Model commands, existing
scope assertions and libmpv property setters remain exercised. Automatic native
worker startup, OS-session registration, clipboard observation and native audio
refresh are fenced; production behavior is not globally disabled or cfg-skipped.

A regression runs real model logic and verifies service state is unchanged.
A separate, mandatory test child explicitly enables production bootstrap,
requires external/SFX initialization and completion of real native audio discovery,
then exits. It loads/plays no media and sends no board commands. Parent-only
environment/config isolation and bounded child lifetime prevent fixture/global
native state from contaminating the remaining suite. A crashed child fails the
test; this is not an ignored integration test or a rerun-based crash waiver.

No local Rust build/link/test, runtime deployment, GUI inputs or hardware changes
are part of this source lane. Exact-head native CI remains required; RF owner
retains final integration, package/deployment and actual native/physical acceptance.
