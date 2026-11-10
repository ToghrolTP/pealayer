# Project persistence: preserved implementation draft

Owner: [issue #226](https://github.com/ToghrolTP/pealayer/issues/226).
Branch: `feat/project-scene-persistence`, originally stacked on the UI work in PR #225.
This is not a production-ready feature or deployment candidate.

## Merge audit repairs (5 November 2026)

The draft is now based on merged UI main. Document import has a typed authority
validation operation before Web rendering. Native project drafts (including
component inclusion changes) and pending operations participate in authoring
inspection; reopening cannot overwrite the editor. Project capture no longer
queries the entire live session on every editor frame. Timeline documents retain
selected track and search filter, and folder preferences use multiline controls.

Media restoration now defers timeline/workspace commit until a matching decoder
FileLoaded and exact-seek submission, cancels pending prepared Play, and retains
the old authoring state on decoder failure. Failure/30-second timeout attempts
one paused reopen of the previous media; it never resumes hardware playback.
External mpv uses its own FileLoaded revision, not a general metadata change.
Explicit Open/Close cancels pending project restoration. Decoder seek submission
is not yet exact-frame/readback acceptance; same-path event ordering and rollback
still require live verification.
Association-write failures are surfaced separately from a completed document save.

The full native library suite passed with an isolated configuration: 1089 passed,
zero failed, one ignored. This includes 14 pure scene operation tests, document
round-trips/linked audio references, decoder-event revision and editor draft
preservation. Web TypeScript checking passed. Remaining acceptance items below
still apply; these incremental repairs do not make the draft merge-ready.

Scene capture distinguishes a sampled all-Off relay mask from missing telemetry.
Explicit safe output selection uses the coordinated `controller.output.set`
contract in PCController PR #727. Plans pin the host, board, connection generation,
profile revision and PWM curve revision; each write requires matching typed ACK
and raw readback. There is only one in-flight request, a 30-second deadline, no
automatic retry/rollback, and explicit acknowledged/failed/unknown/skipped reports.
Cancellation stops after the current call. An uncertain pending call continues
blocking new scene work until the transport finishes, even after plan timeout.

PWM Off recall can advance the current board-profile revision. A multi-output
plan intentionally stops if that changes its pin; it must not silently refresh
and continue. Separating configuration identity from mutable recall, or an
explicit reviewed continuation boundary, remains an acceptance decision.

Discovery verifies media identity before automatic restoration, including when
saved media options are excluded. Ambiguous/wrong-identity associations preserve
the user's requested Open and any existing metadata draft. Native consumers can
export captured components locally without treating a component export as a
saved parent project. Web authority changes invalidate apply confirmation and
block mutation from a draft captured on the previous authority.

## Current reconciliation

The branch retains the complete shared authoring-document model from PR #233,
including portable linked audio and validation. Both histories are preserved.

For a seekable source, project restoration now waits for the requested exact
seek to settle in the active, paused decoder before committing timeline and
workspace. Native restoration requires a post-request start event and matching
media identity. External restoration requires a fresh loaded revision. A
rejected, superseded, or timed-out seek retains existing authoring data.

Native and Web editors record the responding Pealayer instance when reading or
capturing a document. Remote mutations carry that owner to the receiving API,
which rejects a stale owner even if the host address remained the same. Review
the same-path decoder ordering, rollback, and authority changes in live isolated
fixtures before accepting the draft.

## Preserved work

Typed scene, timeline, media, workspace and project documents; bounded reads and
atomic writes; embedded and external components; initial discovery preferences;
native and Web metadata editors; and a shared `pealayer.documents` RPC/peer path.
Initial native checking and Web TypeScript checking passed. Focused Preferences
tests also compiled against this draft, but do not validate project semantics.

## Required next work before acceptance

- Transactional restoration: validate all components before mutating media or
  workspace, handle rollback and stale FileLoaded events, and retain the normal
  media/history/cache lifecycle. Test exact millisecond position and pause state.
- Scene capture/apply: verify the coordinated output contract end-to-end, including
  partial failure, cancellation and the PWM recall revision boundary. Raw motion
  and firmware-owned channels remain data-only; safe general-purpose selections
  are explicit, never a command to apply every captured value.
- Preserve selected track/filter/order and all workspace/dialog-owned geometry
  and scroll state. Reconcile the older raw timeline save/load actions with the
  one current document contract rather than exposing conflicting formats.
- Offline portability: resolve SFX references and controller program dependencies;
  report missing dependencies before playback without inventing definitions.
- Discovery: verify native/Web ambiguity flows and requested-media restoration in
  live decoder sessions, including missing linked files and stale decoder events.
- Editors: exercise the repaired dirty-draft/import/export/partial-selection flows
  at responsive widths and confirm their actual authority ownership.
- Move disk operations off the UI thread where necessary; propagate write and
  association errors rather than silently ignoring them.
- Round-trip/malformed/safety/permission tests, actual native/Web screenshots at
  responsive widths, authority/consumer checks and host-compatible deployment.

No scene load is permission to actuate hardware. User files, private review
profiles and generated Web build output are not part of this checkpoint.

## Reconciliation evidence

- The isolated Windows build passed 27 distinct focused native tests, including
  document round-trips, linked audio resolution, the decoder fence, authority
  permissions, scene operation safety, and dirty-draft preservation. Native code
  under test is the same as in the Web capture revision.
- Web TypeScript, 53 application parity checks, responsive guardrails, and the
  production Web build passed.
- [Web screenshots and capture manifest](../screenshots/pr227) show the actual
  built Project metadata editor at desktop and mobile widths, plus media
  metadata and the millisecond position control. Chromium checked that the
  dialog fits both widths and keeps its footer reachable. A synthetic authority
  replacement returned a conflict and the browser surfaced it at both widths.
  Synthetic project data exercises the visual and browser request path only.
- [Native editor capture](../screenshots/pr227/project-metadata-native-discovery.png)
  shows the real Project metadata modal after opening media with two valid
  synthetic project associations. It uses the same disconnected isolated build
  as the earlier reconciliation, with capture provenance alongside the image.
- [Native and external decoder fixture](project-persistence-decoder-fixture.json)
  verifies same-path restoration at exactly 3.640 seconds and a live HTTP
  409 for a stale authority. A corrupt media file restored the earlier paused
  frame and left the prior project and timeline intact in both decoder modes.
  Rollback completed in about one second internally and one and a half seconds
  through managed external mpv. Reordered/stale event fault injection, portable
  controller dependencies, scene output and production deployment remain open.

### Loopback scene verification

The [Virtual Board scene fixture](project-persistence-virtual-scene-fixture.json)
used a paused synthetic video, a separately owned loopback coordinator, and an
in-memory board with host peripherals disabled. Staging made no output change
and apply without confirmation returned HTTP 400. An explicitly selected relay
changed with a correlated controller acknowledgement and matching sampled
readback. An unselected relay stayed unchanged. A second confirmed operation
restored the original output. Both changes appeared in scene capture without a
manual refresh.

The fixture exposed two integration gaps. The coordinator advertises relay
transport as control relay and general-purpose ownership as role user-output.
Scene preflight now consumes both fields and rejects firmware-owned roles.
IPC-only coordinators may lack push telemetry, so the completed operation
requests one refresh through the existing catalog path.

After reconciling Terminal on main, 14 scene operation tests, the document
permission test, the external decoder failure revision test, and the Web
TypeScript and guardrails passed. Isolated native and external decoder
restoration and corrupt-file rollback also passed again. Physical output, PWM
recall revision, cancellation under live transport failure, and deployment
remain separate acceptance gates.

### Standalone timelines

Explicit Timeline Save As uses the same bounded, atomic timeline document as
the Project editor. It keeps the playhead, zoom, scroll, selection and track
state. Native Open Timeline and the peer file API use the same reader and
decoder-confirmed restore path. The Sidecar action still writes the raw
media-sidecar format and Open Timeline can read those files without imposing a
saved seek position they did not contain. Tagged files of another document
kind are rejected rather than reinterpreted as raw timelines.

The [live timeline fixture](project-persistence-timeline-fixture.json) saved a
3.640-second paused position through the peer file API, moved the playhead and
reopened the typed file at the exact saved position. It saved and reopened a
raw sidecar while retaining the 5.120-second paused position and decoder
revision. It also verified that the timeline action rejects a scene document.
The isolated document and project editor suite passed all 12 focused tests.
