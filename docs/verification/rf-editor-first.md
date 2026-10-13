# RF assignment editor and saved-button grouping

Owner: [#187](https://github.com/ToghrolTP/pealayer/issues/187).

Native and Web assignment editors appear before saved bindings. Saved rules are
collapsed under the complete code/bits/protocol waveform, so four gesture rules
represent one physical key instead of four required buttons. Opening a channel
assignment scopes the saved list by the advertised stable action identity or the
exact current PWM board-command grammar, never names or displayed channel numbers.
Unknown advanced commands remain visible in the general RF manager. Nothing is
deleted, rewritten, enabled or saved by grouping, opening or catalog events.
Explicit Edit returns to the editor; refresh events do not reset a dirty draft.

Known PWM toggle/roll/remember commands show the current channel name. Raw command
text remains available in the editor. Signal identity includes bits and protocol;
colliding codes must not merge different waveforms. Enable changes retain explicit
previous_name and require a live connection, and rejected collisions cannot silently
replace another assignment. Existing board/motion mappings remain unchanged.

PWM remembered_persisted is consumed only alongside valid, owner/generation/revision
fenced memory. Malformed metadata retains last-good state; a different raw value
cannot inherit a missing persistence flag. Native/Web label confirmed profile
persistence only on the authoritative true flag. Unknown provenance is described
as coordinator-owned memory, never fabricated restart/reconnect loss or persistence.
This remains separate from full/restore policy and absolute percent setters.

## Remaining paired delivery gates

PCController #679 owns one-key tap/hold/repeat/hold-end and profile recall; #681
owns atomic binding-name collision protection. A one-button preset must use actual
catalog gestures and existing binding.put, with explicit user waveform/channel
confirmation and honest partial-failure handling, not an invented preset API,
build-version branch, automatic Save or guessed opcode. That preset is not
implemented by this focused presentation change.

No tests/builds, runtime changes, mappings or output commands are run on Cafe by
the source lane. Native/Web helper regressions and memory-provenance tests are
added for hosted CI. RF runtime owner alone performs paired installation and live
native/Web checks; source tests do not establish RF learning, physical motion or
brightness acceptance. Private production captures remain local.
