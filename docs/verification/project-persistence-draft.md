# Project persistence: preserved implementation draft

Owner: [issue #226](https://github.com/ToghrolTP/pealayer/issues/226).
Branch: `feat/project-scene-persistence`, stacked on the UI work in PR #225.
This is not a production-ready feature or deployment candidate.

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
- Scene capture/apply: distinguish unsampled state from false/off, reconcile
  high-level motion with its raw relay channels, and provide an explicit safe
  apply selection. The initial Apply implementation rejects motion and reserved
  relays, so a mixed captured scene is not yet a usable full-scene apply.
- Preserve selected track/filter/order and all workspace/dialog-owned geometry
  and scroll state. Reconcile the older raw timeline save/load actions with the
  one current document contract rather than exposing conflicting formats.
- Offline portability: resolve SFX references and controller program dependencies;
  report missing dependencies before playback without inventing definitions.
- Discovery: verify association identity, preserve requested media when the project
  excludes media, handle ambiguity/errors, and correct the multiline folders input.
- Editors: protect dirty project drafts in authoring inspection, avoid per-frame
  full snapshots, validate browser imports before rendering, and finish remote
  component export, scene editing and partial-component controls.
- Move disk operations off the UI thread where necessary; propagate write and
  association errors rather than silently ignoring them.
- Round-trip/malformed/safety/permission tests, actual native/Web screenshots at
  responsive widths, authority/consumer checks and host-compatible deployment.

No scene load is permission to actuate hardware. User files, private review
profiles and generated Web build output are not part of this checkpoint.
