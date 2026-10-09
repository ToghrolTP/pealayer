# Timeline toolbar: immediate edits and stable navigation

9 October 2026. Follow-up to the merged consolidation checkpoint, not a claim
that every historical Web/native requirement is complete.

## Why checking a control closed the workspace

`DockArea` temporarily borrows the real dock state during panel drawing. The
old overflow checkbox called `save_config()` from inside that drawing pass.
The full snapshot therefore contained the empty placeholder dock. In consumer
mode the same call synchronously posted the configuration to the authority,
blocking the UI on the network and publishing the empty workspace there.
This was a persistence-boundary error, not lost source or intentional behavior.

Toolbar edits now change their local presentation immediately and enqueue only
four existing/shared configuration fields: visibility, follow-playhead, icon
order and hidden icons. They never send dock, media, session or effect data.
Local persistence runs in application logic outside painting. Remote saves use
the existing bounded peer dispatcher and compare-and-set config API, with an
acknowledgement channel. Further edits coalesce behind the in-flight request;
stale snapshots cannot undo the pending presentation. Rejection or an unconfirmed
request restores authoritative settings and produces a scoped warning.
The authority and consumer apply toolbar-only snapshots without restoring a
workspace or resetting player/effect state. General saves attempted during dock
drawing are also deferred until the actual dock has been returned.

## Controls and presentation

- Drag an icon with the primary button onto either half of another icon to
  insert before/after it. A guide and translucent icon follow the drag. Other
  channel/cue drag payloads are separate and cannot reorder this toolbar.
- During a toolbar drag, the overflow button crossfades to a red trash target.
  Dropping there hides that icon. The overflow checklist restores it; hidden
  actions retain their ordering slots. Reset returns to application defaults.
- Preferences → Input → Timeline navigation → **Show timeline toolbar** hides
  or restores the complete toolbar without disabling timeline navigation.
  This setting belongs to the shared Rust preference contract used by Web UI.
- Shift+Left/Right pans continuously while held when the timeline has keyboard
  focus. A short pan-button click retains one-step eased navigation; holding a
  pan button for 300 ms pans continuously. Dragging instead reorders the icon.
  Held navigation is time-based, bounded after stalls, and stops on release.
- Every icon has a fixed 26-point hit area. Selection strokes paint inside it;
  hover/follow state cannot shift adjacent controls. The opaque toolbar covers
  the full pixel-aligned ruler height and right edge. Its bounds depend on the
  viewport, never the content pan offset. Overflow uses a compact checklist.
- Workspace-owned sublayers remain below Add cue and other modal backdrops.
  Keyboard navigation does not run behind an open menu or modal.

## Acceptance and delivery

Implementation checkpoint: source regressions were added for narrow config
patches, deferred dock saves, ordering, invariant button bounds, pixel-aligned
toolbar geometry and held-pan timing. **They have not been run**, per the user's
request to defer tests and slow CI waits. Formatting and diff hygiene are not
runtime verification. An executable-only native build and host-specific graceful
deployment remain required; record results below when actually observed.

Manual acceptance on the deployed app (no hardware actuation required):

1. Keep several workspace panels open; check/uncheck several overflow rows.
   Each icon changes immediately, panels remain open, and remote preference
   read-back eventually agrees. Reopen the app and verify persistence.
2. Drag an icon left/right, then onto the fading red trash target. Restore it
   in the checklist. Verify primary drag does not invoke the original action.
3. Toggle the full toolbar preference and restore it. Check Follow on/off and
   hover states do not shift any neighbor or change the header height.
4. Hold Shift+Left/Right with canvas focus, then hold the Pan buttons. Release
   and verify navigation stops. Opening Add cue blocks the controls underneath.
5. Pan/zoom at the host's actual DPI. The toolbar stays flush right without
   one-pixel jitter or ruler ticks painted behind it; the menu remains compact.

Build, deployment and visual acceptance: pending at this source checkpoint.
