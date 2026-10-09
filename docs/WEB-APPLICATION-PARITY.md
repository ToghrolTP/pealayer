# Pealayer Web application parity ledger

GitHub tracking issue: [#63](https://github.com/ToghrolTP/pealayer/issues/63)

The Web surface is a first-class Pealayer application. It is not a narrated
status page and it must not recreate domain rules already owned by Rust, mpv,
or PCController. This ledger records the remaining differences against the
native egui application in product language rather than commit identifiers.

## Definition of parity

A surface is complete only when it uses the same authoritative state and
commands as egui, offers the same meaningful operations, handles keyboard,
pointer, touch and context-menu input, remains usable at phone through desktop
widths, and has been exercised against live state rather than sample data.

## Current product ledger

Selective native widget integration (9 October): [palette-safe Elegance](verification/PALETTE-SAFE-ELEGANCE.md)
uses the existing Pealayer appearance contract for every library palette role,
live OS scheme and typography, without replacing the global style or adding a
second Web theme. Compact Preferences/Audio/Subtitle cards, neutral cue badges
and truthful external-player feedback adopt the library; specialized controls
and hardware input semantics remain intact. Deployment/visual acceptance is
tracked independently from source tests.

External mpv (9 October): [modes and protocol](external-mpv.md) adds the same
event-driven player adapter to native/Web/IPC/peer controls and the hardware
clock: external-only, managed or attached dual preview, and remote control of
existing mpv. Preferences use the shared contract; player snapshots expose
actual connection/ownership/errors. Real Windows IPC tests verified bidirectional
state, muted preview, reconnect/no replay, owned-process cleanup and 32 command
acknowledgements in about 21 ms. Linux/macOS and human visual/audio acceptance
remain explicit; no claim of frame-locked independent decoders or complete parity.
[Delivery evidence](verification/EXTERNAL-MPV-DELIVERY.md) records the installed
David/Cafe/Erfan package, retained host runtimes and remaining acceptance checks.

Copied links and source browsing (9 October): [remote-location workflow](remote-folders.md)
adds default-enabled text-only clipboard URL observation, debounced live inputs,
parallel proxy/direct validation with manual overrides, a stable refresh/loading
region and in-list Back navigation. Browser permission boundaries remain explicit;
deployment and actual clipboard/dialog acceptance are recorded in the focused PR.

Sound effects (9 October): [SFX and audio routing](SOUND-EFFECTS.md) adds a
shared host audio-effect catalog, native/Web import and preview editors,
intrinsic Audio-effects cues, main/SFX/per-effect output selection and live
output discovery. [Delivery evidence](verification/SFX-AUDIO-DELIVERY.md) records
the installed David/Cafe/Erfan package and live Cafe preview, EOF, timeline and
remote-command checks. Human listening and interactive mobile acceptance remain
explicit; this does not declare overall parity done.

Live branding (9 October): [surface reconciliation](verification/LIVE-BRANDING-SURFACES.md)
removes stale tray tooltips, the built-in-only About logo and startup-only Web/PWA
identity. Preferences and native media sessions use configured branding; browser
updates reuse authoritative config revisions without disconnecting controls.
OS-managed pinned/installed branding remains a packaging/cache boundary, not a
reason to rewrite the running executable.
The installed David/Cafe builds passed a reversible API preview with native
title/icon and live runtime/manifest verification; Erfan is installed without
launching alongside KMPlayer. Tray hover and dialog visual acceptance are pending
the user's confirmation, not inferred from compilation.

NLE volume (9 October): [responsive transport correction](verification/NLE-VOLUME-AND-RESPONSIVE-TRANSPORT.md)
separates seeking from the native action row and exposes a full-width volume
strip on narrow monitor panels. Web Simple/NLE now share volume, mute and
percentage controls using the same session commands. The build is installed on
David and Cafe with live volume/mute synchronization verified; Erfan is installed
without launching alongside KMPlayer. Narrow native layout was inspected through
a DPI-aware Win32 capture. Phone/tablet Web gesture acceptance remains pending
human verification; deferred test suites are not claimed as executed.

Hardware cue modes (9 October): [authoring and execution contract](HARDWARE-CUE-MODES.md)
separates compact move-only Set and keep markers from resizable timed holds and
linear PWM ramps, with explicit exit values, native/Web editors and shared
prepared-plan compilation. Source/build checks are not installed acceptance:
tests remain deferred at the user's request; Cafe deployment, native/Web gesture
inspection and safe physical timing verification are owned by consolidation.

Native toolbar follow-up (9 October): [toolbar interactions](verification/TIMELINE-TOOLBAR-INTERACTIONS.md)
tracks immediate, workspace-safe saves, drag ordering/hiding, continuous held
navigation and fixed ruler geometry. The full-toolbar visibility preference
uses the shared Rust preference contract, so Web preferences can configure the
native authority too. This is **not** a claim that React already has equivalent
draggable ruler controls; that counterpart remains part of timeline parity.

| Area | Current state | Completion evidence | Remaining work |
| --- | --- | --- | --- |
| Application shell | In progress | Installable SPA/PWA, responsive navigation, shared appearance and connection state; footer now consumes the native persisted visibility contract and exposes right-click hide/show actions | Finish application-style command header and compact/mobile command access |
| Playback surface | In progress | Browser-native video now uses Pealayer's seekable byte-range endpoint and follows the Rust/mpv clock; unsupported browser codecs fall back to the backend frame surface | Add negotiated low-latency transcoded canvas stream over WebSocket, browser capability reporting and stream diagnostics |
| Transport | Mostly complete | Play/pause, seek, volume, mute, rate, chapters, thumbnail preview and remote-folder previous/next use shared commands; video, audio and subtitle selectors now consume live libmpv track metadata and invoke the same validated selection commands in Simple and NLE layouts; the primary timeline action has a fixed circular hit target | Complete frame-step state, contextual shortcuts and exact buffer visualization |
| Hardware monitor | In progress | Live advertised controls, pointer-down relay actions with immediate optimistic feedback and authoritative reconciliation, clickable indicators, PWM, motion hold/release, drag ordering, custom channel icons and complete card context menu | Complete bulk management, bindings, channel timeline actions and every advertised board setting |
| Emergency stop | Complete for Web interaction | Filled red danger control, immediate pointer-down dispatch and shared interlock state | Continue physical-board acceptance whenever hardware is attached |
| Front panel | In progress | Live PCController masks now render as four illuminated seven-segment glyphs, with brightness/activity/blink state plus K1–K4 commands and LCD text | Match the complete egui board-information tabs, settings and contextual commands |
| Effects library | In progress | Live PCController catalog, groups, create/manage/rename/play/delete, drag source and context menus | Finish group management, all custom picker behavior and visual parity at every responsive width |
| Effect editor and recording | In progress | Sequence editing and board/app recording share PCController effect contracts | Complete professional multi-lane editor, selection, easing, fades, repeat/blink authoring, offline drafts and conflict handling |
| Timeline | In progress | Web receives the native ordered track inventory and its authoritative selected, linked, visible, muted, soloed and locked state; row selection, action buttons and right-click menus invoke the same validated Rust commands as egui; cues move and conditionally resize | Complete keyframe editing, browser-native manage dialogs, track routing/selectors, keyboard editing, snapping, vertical reordering and exact scroll/navigation behavior |
| Media library | In progress | Browse/play/thumbnails and native path requests work; Windows extended path prefixes are removed from breadcrumbs; file and folder rows expose open/play/copy/refresh context actions | Add metadata, safe rename workflows and richer remote-folder parity |
| Preferences | In progress | Rust-generated controls and ordered groups drive both surfaces; Current and Classic application-icon presets share resolution and live refresh, while custom application icons share file metadata, optional disclosure, Browse and Reset; duplicate cards/help are guarded and Config file closes Advanced | Finish import/export parity and visual acceptance of the deployed native/Web layout at responsive widths; see [Preferences organization](PREFERENCES-ORGANIZATION.md) |
| Dialogs | In progress | Connection, remote location, channel management, RF management, effect editing and workspace management exist | Add full About, media/track properties, audio, subtitles, board information, update, bindings and remaining native dialogs without duplicating state logic |
| Messaging and OSD | Mostly complete | Shared toasts and the native configurable OSD contract render across Web, egui, HTTP, JSON-RPC and WebSocket | Complete icon-name coverage and visual acceptance for every custom anchor/color combination |
| Updates | In progress | URL update and truthful byte progress exist | Replace explanatory filler with contextual state/actions and complete peer/CI source selection parity |
| Footer/status bar | Mostly complete | Web renders configured hardware, physical status RGB, telemetry, frame/playback state, warnings, transient update/OSD messages, E-STOP and workspace state; persisted hide/show controls work from the whole bar or individual items | Add shared item ordering and complete narrow/mobile prioritization acceptance |
| Responsive and accessibility | In progress | Automated phone/desktop layout contracts, modal height constraints and reduced-motion handling exist | Add screenshot interaction passes at phone, tablet, narrow desktop, desktop, light, dark, keyboard-only and touch sizes |
| Headless operation | In progress | Rust backend hosts the PWA, REST, WebSocket, IPC and range-serving media endpoint | Remove the native-window dependency for truly display-less startup and complete transcoded streaming fallback |

## Next acceptance passes

1. Introduce a negotiated video pipeline: direct byte-range media when the
   browser supports it, otherwise a bounded low-latency encoded stream delivered
   to a canvas with clock and buffer feedback.
2. Complete the configurable footer/status-bar visibility and ordering model.
3. Complete the board-information and front-panel dialogs from advertised
   PCController capabilities—never hardcoded demo values.
4. Audit every egui modal against its Web renderer and close one measurable row
   at a time with dark/light and responsive evidence.

## Completed acceptance checkpoints

### Explicit frame-rate source and reactive hardware status

The footer's frame-rate source is now a persisted choice: **Media frame rate**
(default, libmpv `container-fps`, unchanged by playback speed or pointer activity)
or **UI render rate** (measured native frame cadence, not monitor refresh Hz).
Click the indicator to toggle; its icon-bearing context menu selects either
source or hides it. Preferences → Advanced → Status bar exposes visibility and
source through the same Rust contract used by Web Preferences. Unavailable
measurements display a dash rather than a fabricated 60 FPS. Web UI labels the
native render measurement explicitly; it does not substitute browser RAF cadence.

Regression evidence:

- `ce85904` changed the native footer to automatic interaction-based switching
  for 1.5 seconds and substituted display refresh Hz for missing measurements.
  This made pointer movement change the meaning of an otherwise identical FPS
  label. Source mode is now explicit; the existing stable container FPS observer
  is retained. Remote consumers copy the authority's media FPS, not their local
  decoder's incidental estimate.
- The idle CPU reduction (`6510c35` and related publication changes) was valid,
  but paused sessions were classified as idle even while hardware RGB/telemetry
  changed. Web/peer status could therefore lag by a second. Hardware changes
  now use the configured Web sync cadence, with a single trailing repaint for
  a final rate-limited update; unchanged idle sessions retain the slow backstop.
- Peer repaint comparison omitted the typed `session.hardware` snapshot, and
  the consumer engine could install it after the UI's repaint. Both paths now
  wake on real hardware changes, including after installation. The existing
  WebSocket supports `peer.hardware.subscribe` / `peer.hardware`, reusing the
  typed `HardwareCapabilities` snapshot without fetching mpv/config/workspace
  state for each RGB update. The server sends changed-only snapshots at most
  ten times per second. Paused full-session health pulls stay at one second;
  unavailable/disabled streaming uses the lightweight `/api/peer/hardware`
  fallback only while a board is connected. A generation fence prevents a slow
  full-session HTTP response overwriting newer pushed hardware. Both reads use
  the existing configuration-access permission. Missing payload fields or
  changed source identities retain known-good values and require refresh.
- Footer saves are narrow, acknowledged and asynchronous for consumers; a
  click no longer waits for remote HTTP or re-applies the workspace/player.
  Numeric widths are stable to prevent status values shifting neighboring items.

Acceptance is intentionally split: TypeScript/Vite and native BuildOnly results
belong in the linked PR checkpoint. Tests, responsive screenshots and installed
Cafe/David verification remain pending until explicitly performed. No hardware
actuation is required for these display changes. The prior updater execution-
policy blocker is not a completed deployment and must not be bypassed.

- **API-first local lifecycle:** Native connection UI, native IPC, JSON-RPC,
  HTTP and WebSocket use one process connection command. Process status,
  graceful local quit and consumer-role changes do not relay to the authority;
  normal session operations still do. A role change validates the target first
  and requires a paused/unlocked, acknowledged publication release when needed.
  Installed-host verification is tracked separately in the deployment ledger.

- **Publishing handoff policy and remote alternative:** Native and Web
  Preferences share the owner's opt-in unattended-handoff setting. The Rust
  observer pauses actual playback and requires a fresh matching clock echo
  before using PCController's existing safe acceptance contract. Production
  lock is never overridden. Conflict controls offer Connect to authority using
  the registered owner's validated Web origin; native permits editing the
  address for a tunnel. Host deployment and live acceptance are tracked in the
  [deployment checkpoint](verification/DEPLOYMENT-AND-MERGE-CHECKPOINT.md),
  separately from source and automated verification.
- **Shared media-track selection:** Rust publishes the current libmpv video,
  audio and subtitle inventory with selected/default/forced/external metadata.
  Simple and NLE Web layouts reuse one selector component and the validated
  `pealayer.media.track.select` / `pealayer.media.track.disable` commands, so
  the browser does not infer or locally override track state.
- **Shared timeline track controls:** The Rust snapshot now publishes track
  selection and M/S/L capability/state, and both Web pointer actions and context
  menus use `pealayer.timeline.track.update` or
  `pealayer.timeline.track.manage`. Track keys are validated once at the shared
  transport boundary; the Web client contains no channel-specific mutation
  rules.
- **Immediate hardware interaction and board presentation:** Relay buttons and
  indicators update on primary pointer-down, then reconcile with the next
  PCController snapshot instead of appearing inert while the board command is
  already in flight. Custom channel icons are shared with effects, and the
  front panel renders actual segment masks instead of hexadecimal debug text.
- **Shared status-bar configuration:** Web reads and writes the native
  `status_bar` configuration, renders only live state, and offers right-click
  hide/show actions without creating a second browser-only preference model.

## Regression rules

- **Buzzer catalog and control:** PCController owns configured melodies. Native
  and Web selectors refresh whenever opened, including keyboard activation;
  catalog events and low-rate output edges obtain authoritative state. Native
  controls use the same session commands as Web/API so remote consumers reach
  their authority instead of an inactive local engine. Searchable Web choices
  share icons/duration details with the effects editor, and diagnostic tones
  collapse. Malformed refreshes preserve the last-good list rather than render
  false-empty state. See [delivery and remaining physical/UI gates](verification/BUZZER-LIVE-CATALOG.md).

- **Elapsed-time editing:** Native and both Web transports use a fixed
  `HH:MM:SS.sss` segment mask. Unchanged blur does not seek; changed blur and
  explicit Enter use the existing seek path, while Escape always cancels.
  Group navigation, separator skipping, invalid-input rejection and local-digit
  normalization preserve the format. Native selection is painted in the same
  frame as edits. See [installed checkpoint and input acceptance](verification/ELAPSED-TIME-SEGMENT-EDITOR.md).

- **Channel feedback and track-state controls:** Web consumes Rust's computed
  indicator color and semantic motion direction rather than painting every
  active output green. Fixed-size Mute/Solo/Lock actions use shared track updates,
  semantic colors and track exclusion. Direct PWM mute/solo exclusions are also
  applied to live and prepared playback. Native insert-first cue properties now
  reuse the dock editor; Web cue-editor consolidation is still a follow-up.
  See [checkpoint and remaining installed acceptance](verification/CHANNEL-FEEDBACK-AND-CUE-PROPERTIES.md).

- **Seekbar markers and scrub settlement:** Simple and NLE share typed
  `scrub_to` / `finish_scrub` gestures, live coalesced previews, and correlated
  decoder settlement before discarding the local thumb position. Shared Rust
  configuration supplies subdued chapter ticks, active-chapter background and
  red named project keyframes. Native/Web color pickers share semantic
  swatches. Source/build progress and pending installed-host acceptance are
  described in [Seekbar markers and seeking](SEEKBAR-MARKERS-AND-SEEKING.md).

- Hardware state changes occur on primary pointer-down; keyboard activation
  remains click/Enter/Space compatible and commands fire exactly once.
- Right click opens item-specific actions and never starts dragging.
- The Rust snapshot owns track identity, order, visibility and capability data.
- No synthetic board, effect, progress, media metadata or connection state.
- Persistent prose must be actionable; obvious update narration is omitted.
- A visually similar control is not parity unless it calls the same command and
  reacts to the same authoritative event stream.

## Effect Controls refinement

The native dock inspector and cue modal retain one shared renderer with compact,
responsive exact-time fields, semantic values, fixed-duration rules and collapsed
identifiers. The Web direct-cue modal now follows the same timing/output grouping
and state colors. Full controller/sequence inspector parity remains outstanding;
this visual refinement is not that broader completion claim. See
[scope and verification](verification/EFFECT-CONTROLS-INSPECTOR.md).

## Mobile button centering

Shared Web button roots, icon wrappers and SVGs now use centered flex geometry,
including portal actions. The icon baseline spacer and inconsistent transport
grid overrides are removed; existing collapsed navigation centering is retained.
The focused responsive guardrails passed, and the combined package is installed
on Cafe and David with the corrected CSS confirmed over HTTP. Erfan is updated
without launching over KMPlayer. These are delivery checks, not visual proof.
Current mobile visual acceptance is still pending, not inferred from CSS/build.
See [cause, scope and verification](verification/MOBILE-BUTTON-CENTERING.md).
