# Compact saved RF rules · integration WIP

Owner: [#187](https://github.com/ToghrolTP/pealayer/issues/187).
Shared RF source owner: [PR194](https://github.com/ToghrolTP/pealayer/pull/194).

The runtime owner's private325px screenshot confirms the expanded saved-rule
table clips readable action text/internal IDs and places Edit outside the pane.
The existing editor-first, waveform grouping and four internal gestures are valid;
the fixed-width saved table is not a usable narrow layout.

This isolated checkpoint adds a compact RfSavedRuleCards renderer and namespaced
CSS: complete wrapped summaries/IDs, one retained row per gesture, a separately
reachable Edit column and disabled mutation checkbox when required. It receives
only presentation strings and explicit callbacks; it does not own waveform,
catalog, draft, reception counters, preferences or RPCs. SSR tests render the actual
component, check all four complete rows/Edit labels and zero callback side effects;
CSS contract tests cover shrinking columns and wrapping without hidden content.
The first hosted harness failed before assertions because the current TypeScript
dependency does not expose the old compiler JavaScript API. The repaired harness
uses the existing documented Vite SSR loader with real React/Antd, translation and
CSS modules, middleware-only/no HTTP or WebSocket listener/no watchers, and closes
the fixture in finally. No compiler-version fallback or skipped assertion is added.

**Integrated source; not deployed or visually accepted.** The live coordinator
superseded the former blanket shared-file hold: reversible work proceeds on our
independent PR197 branch, without editing/replacing PR194's branch or history.
RfManager now selects the renderer only in its existing compact layout. Desktop
Table, waveform groups and all other parent consumers remain unchanged. Both
layouts reuse the existing draft clone, previous_name and editor scrolling;
mutation callbacks retain pending/disconnected guards. Parent source-wiring
contracts complement the actual component SSR, not a browser interaction test.

Before merge: inspect Git merge-tree against exact PR194 head
e55b857144c089be669f3a0b41e9f8fef5784097 and publish preservation/conflict evidence;
require fresh exact integrated hosted gates and review, then normal merge. Do not
reset modal reception baselines on resize/refresh, change learning preferences,
autoSave, or replace mappings. Send tested tree/package receipt to the sole runtime
owner and verify325px plus desktop rendering with real callbacks and no output,
learning or binding mutations. Never treat the unintegrated checkpoint's green
checks as verification of this new parent integration.

No local tests/builds or runtime/physical actions. Production screenshots/values
remain local, not committed. Native visual/smoke and physical verification remain
independent gates; a compiled unused renderer is not application acceptance.
