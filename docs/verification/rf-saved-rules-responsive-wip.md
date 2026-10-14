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

**Not integrated or delivered.** The active PR194 author has not yet acknowledged
the shared-file lane. RFManager/rf.rs and the author's branch are untouched. The
component is intentionally not called by the application until that coordination
is cleared. Installing this checkpoint alone will not fix the production table.

Next: obtain owner acknowledgment/merged194 base; import the renderer only for the
manager's existing compact layout, preserve desktop Table and grouped waveform
headers/receipt indicators, and reuse the existing explicit Edit and enable
callbacks (including previous_name and dirty-draft behavior). Do not reset modal
reception baselines on resize/refresh, change learning preferences, autoSave, or
replace mappings. Then require exact integrated hosted gates and normal merge,
send tested tree/package receipt to the sole runtime owner and verify325px plus
desktop rendering with real callbacks and no output/learning/binding mutations.

No local tests/builds or runtime/physical actions. Production screenshots/values
remain local, not committed. Native visual/smoke and physical verification remain
independent gates; a compiled unused renderer is not application acceptance.
