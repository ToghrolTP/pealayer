# Compact saved RF rules · source and live Web verification

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

**Integrated source with bounded live Web acceptance; native and physical gates remain open.** The live coordinator
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


## Live Web acceptance (2026-10-14)

This supersedes the pre-deployment checkpoint above. The runtime owner installed
the exact successful Windows artifact from tested commit
`f88a29a530c1be1e2c2e043ee12b8c26ef3ab82f`, whose complete tree matches the normal
merge `96fc6c2459d540eddf5d94502f3552853cb81264`. The running executable reports
the literal tested commit; it is not relabelled as a merged-head package. The
application's executable-only updater preserved the destination's validated
libmpv pair and a private configuration backup. Current health/RPC respond,
real media remains loaded and paused, and no new relevant Application Error/WER
event was observed during the bounded verification.

Actual Computer Use verified both card right-click Assign RF code and Manage
dialog Assign RF code, correct selected-channel scope and disabled incomplete
Save. At a 325px viewport, the former 520px table in a 220px container is now
four 220px cards. Summary/ID client and scroll widths are both 198px; every Edit
button's right edge is 254px, inside the visible pane. All four gesture summaries
and Edit controls are visible in the private current screenshot. At 1100x780,
the existing desktop table is 811px wide inside a 900px dialog; the temporary
viewport override was reset after checking.

Edit was exercised on Tap in the compact layout and Hold in the desktop layout.
Both opened the correct existing identity, signal, gesture and command. The
verification did not Save, enable/disable a rule, learn/transmit, change policy
or dispatch an actuator action. Controller ownership/generation, the selected
output, its saved recall and reset/error counters remained unchanged across the
UI check. Existing internal gesture rules are preserved. Persian Piper guidance
was played, and the selected-channel form remains available for the human.

Native and physical acceptance remain open. Windows Computer Use capture failed
with FrameArrived/window-capture timeout after one refreshed recovery. The
maintained passive occluded capture was black and its foreground capture showed
another application; neither is evidence of the current native Pealayer UI.
Private captures stay local. The current standalone smoke-test has not been
confirmed after automatic launch review returned `blocked by policy`; it was not
retried through another launcher. The reported physical MOSFET blackout at full
brightness is not declared resolved by these software indicators or by SSR.

No project compilation or test runner ran on the production host. Source tests
and packaging passed on hosted CI; this bounded live Web result complements
those checks. Further physical/controller-runtime work requires coordinated
ownership and independent installed-package provenance, while native capture
and smoke validation retain their explicit verification limits.
