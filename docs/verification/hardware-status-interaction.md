# Hardware interaction and status customization

This pass progresses #80, not a completion of its physical/audio, idle CPU,
recording repeat, Web/video, GPU or release gates. The user authorized discarding
running Pealayer drafts. The active local effect working copy was cleared using
the live config API after a named private settings backup; saved effects and
settings were not deleted. Closing/replacing the process clears transient inline
editors. No physical relay, PWM, motion, buzzer or firmware test is implied.

## Corrected causes

- Relay ON/OFF buttons previously applied semantic ink, fill and border even
  when unselected. Only the observed selected state now receives green/red;
  the other action uses the normal neutral button. Disabled/unknown states stay
  muted. Native and Web use the same rule.
- Latest-state input coalescing was also applied to relay presses. A queued ON
  could be replaced by OFF before dispatch. Relay edges now retain FIFO order
  in a bounded 64-item queue, while continuous PWM and motion retain their
  latest-state/final-STOP behavior. Full queues and inputs older than 500 ms
  fail explicitly, never replay after reconnect or bypass E-STOP.
- Indicator toggle previously derived ON/OFF from rendered telemetry, which
  can lag a prior click. It now asks the coordinator to toggle after the prior
  edge. The coordinator checks authoritative state and existing safety rules.
- A complete pointer press/release in a single UI frame made
  `is_pointer_button_down_on()` false and lost the mouse-down action. A click
  belonging to that response now qualifies on the same press frame, but its
  later release cannot dispatch twice.
- The local worker's unconditional 5 ms sleep becomes an interruptible channel
  wait. Pending peer intents no longer wait an extra 20 ms between edges.
  In-flight transport/board ACKs still impose real latency: this is not a
  zero-millisecond physical-edge guarantee. Do not retry a mutation or report
  an optimistic indicator as observed board state.
- The toolbar endpoint inherited a fixed 20-point, top-down wrapper introduced
  in `56d3946a` (30 September). Shared density controls grew beyond it. The
  endpoint wrapper now shares density height and a vertically centered layout.
- Workspace text was end-aligned in a fixed slot, leaving unused space between
  the leading icon and short caption; it omitted the prefix. It now reads
  `Workspace: <renamable profile>`, starts beside the icon and uses a stable
  190-point trailing slot. Updating values do not resize it.
- `a242919` (12 October status widget rewrite) replaced the connection circle
  and `board name via <active transport>` presentation with a plug and board
  title. These are now explicit context-menu choices: Plug (default), Circle,
  controller symbol or None, plus Connection mode (default hidden). The mode
  uses the actual engine transport, including `external:tcp`, not an inferred
  endpoint type. Enabling it selects a fixed wider slot.
- The icon picker previously exposed 20 presets. Native and Web now share the
  full pinned Phosphor regular catalog and font through `/api/icons/catalog`
  and `/api/icons/font`, retaining semantic saved keys and adding Window,
  Chandelier/Loostr, Stage light and Garden aliases. Native search uses the full
  catalog; Web loads it once with a deadline, retaining common presets offline.
- Choice sliders now paint every discrete notch, including all four density
  levels. The melody refresh action used `small_button` and a fixed 32-point
  reservation; it now uses an exact shared-density square beside the selector.

## Status actions

FPS offers source selection, current value copy and measured-source detail.
Connection offers icon/mode selection and endpoint copy. Measurements offer
value copy, bounds/color settings and opening that metric in the log chart.
RGB offers observed color copy. Warnings/HOT expose current warning details and
temperature logging. Timing opens Timeline; E-STOP release uses the existing
confirmation path. Progress exposes operation details; Message offers copy and
dismiss. Workspace manages profiles. Chart offers metric selection, pause,
clear and copying bounded RAM history as CSV. Existing hide, start/end placement,
ordering and font controls remain. Menus must not execute actions when opened.

## Evidence and remaining delivery

`cargo check --locked --tests` compiles source and regression fixtures without
executing them. Web TypeScript compilation and narrow Go vet pass. Tests and CI
remain skipped per user instruction. New pure regressions cover relay ordering,
queue bounds, one-frame clicks and full catalog resolution; existing geometry
fixtures retain display/inline-editor and trailing-anchor checks.

Package/installed identities and live visual verification are recorded separately
after publication. Neither static checks nor VirtualBoard results establish the
physical latency, sustained load, moving-video or audio acceptance gates of #80.

## Package and activation checkpoint

The Windows package completed on 13 October against clean application source
`2bf892e4ea53319dae2a771bfbd9fcdd9a194d62`. Release compilation took 2m09s;
the executable SHA-256 is
`6d21043fa72e590951895c3b495f13130f68268afb07f9d64efb8ae7050d6db3`.
Web/PWA integrity, Windows resources and libmpv smoke passed; tests/CI stayed
skipped. David's retained libmpv remains
`e56ce67cd00f06a59dc7ed5b97a49a9182a381554c755dc4570a52af1ef30e65`.

The approved local peer-updater CLI launch was rejected by execution policy
before running. No alternate tool, launch path, manual executable replacement
or policy bypass was attempted. The current local Pealayer remains source
`3dc99788972519f1f81050a79e93cd6a4231ee06`, PID101304, paused/connected and
without a requested Play. Cafe Pealayer remains closed by the user's prior
request; no Cafe launch or foreign-runtime deployment was attempted. Local and
Cafe delivery of this package are **not complete**.

Windows computer-use capture initialized and selected the actual local window,
but capture failed twice, including a fresh selection, with
`IGraphicsCaptureItemInterop.CreateForMonitor ... 0x8007041D`. No current or
after screenshot was produced. This does not establish visual acceptance.

Source PR [#179](https://github.com/ToghrolTP/pealayer/pull/179) is stacked on
the paired live-value PR #178. PCController's related Web/TUI change is in
[#662](https://github.com/atomicdeploy/PCController/pull/662), preserving mobile
#661 and WS #660. Both attempted chat attachments were rejected by the chat's
100-artifact limit; GitHub source refs and PR links are intact.
