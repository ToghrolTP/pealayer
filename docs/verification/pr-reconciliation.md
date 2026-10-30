# Pull-request integration and remaining acceptance

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
This is a source-integration ledger, not an installation or physical acceptance receipt.

## Reviewed integrations

| Change | Source disposition | Verification boundary |
| --- | --- | --- |
| Native opcode admission, #223 | Merged; immutable PCController-owned source and generated views retained | Both complete exact-head native/Web matrices and CodeQL passed; local generated-source check passed. Local raw GitHub fetch timed out, while hosted immutable upstream verification passed. |
| Effects Library double-click, #220 | Merged; shared Manage path, action-gutter and inline-draft guards retained | Both complete native/Web matrices and CodeQL passed, including real pointer-event coverage. |
| Ant Design patch, #221 | Merged; reviewed manifest, integrity lock and unchanged MIT license | Complete native/Web/PWA and CodeQL checks passed. |
| Cargo updates, #222 | Merged; reviewed uuid/zip/notify and platform transitive changes | Complete four-platform native/Web and CodeQL checks passed. |
| Decoder clock and exact-zero seek, #217 | Review in progress; paired controller reservation contract must be reconciled | Existing exact-head matrices passed. The 50 ms execution/read and 250 ms freshness limits must not be relaxed. |
| Strip preview command, #212 | One-line command change also appears in #217; keep open until integration is proved | Its old failed aggregate followed a cancelled Linux duplicate; another complete exact-head matrix passed. |
| Strip designer and deliberate probes, #218 | Designer/probe/UI files equal the copies carried by #217; keep open until integration is proved | Both old exact-head matrices passed. Optical/human and installed acceptance remain separate. |
| Craft UI docking, #216 | Draft integration under repair; no claim of complete Phase 6 acceptance | Earlier push passed, PR health failed on real whitespace errors. Updated combined-head platform verification is required. |

## Craft docking repairs

- Preserve both the fork's CoreText dependency and notify's CoreServices dependency
  when resolving the lockfile overlap; retain current opcode admission and the
  merged library-card interactions.
- Keep pane chrome Close/resize authoritative over stale pre-render state, while
  retaining the content callback that docks a pane back into the workspace.
- Save changes to pane visibility, selection, rail enablement and width. Coalesce
  pointer-driven layout saves until release; unchanged frames do not write.
- The docking library already activates a visible requested tab. Report actual
  layout/selection changes so that callers persist those changes too.
- A disabled rail is not a visible panel, and must not swallow a request to open
  that panel in the dock. Reject rail moves when workspace permissions reject
  removing the docked panel, rather than rendering duplicate owners.
- Adapt Craft surfaces/text to the selected Pealayer palette and select stored
  light/dark tokens from the active context theme, including live System changes.

New regressions cover pane Close versus stale state, Dock to Workspace with resize,
disabled-rail ownership, existing-tab activation and palette/theme selection.
Rust parse, whitespace, generated-opcode and low-write policy checks passed locally.
Offline native compilation could not start because the pinned egui fork is not in
the shared cache; this is not a passing native test result. Hosted checks own that
gate. Do not mark the draft ready or merge it until its combined exact head passes.

## Preserved boundaries

The three existing stashes and the detached timeline-toolbar worktree are retained.
No production executable, settings, media draft, firmware, physical output or
PCController/PakarController source is changed by this reconciliation pass.
Platform builds after merges are tracked separately from the reviewed PR matrices.
