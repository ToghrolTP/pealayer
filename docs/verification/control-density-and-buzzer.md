# Shared control density and contextual buzzer actions

Preferences / Appearance / Control density uses a four-step slider: Compact
(24 points), Regular (28), Comfortable (32), and Spacious (36). Configuration
stores the semantic choice, not a slider index; existing Compact/Comfortable
settings retain their sizing. Web commits the choice on release or keyboard
completion rather than persisting every drag update. Native edits preview in
the existing Preferences draft and follow its Save/Discard behavior.
Native inputs, ordinary and custom/color selectors,
and single-line menu choices use `ui::control_density`; large fonts can grow
them together. Web follows the same shared appearance setting through Ant
Design tokens and the custom color menu's shared height variable. Saving,
preview, discarding and cross-client appearance reconciliation use the existing
preferences path, not a separate per-widget setting.

The mismatch was introduced in `277647be`: the integrated Custom color selector
used the fixed 32-point dropdown `ROW_HEIGHT` for its closed trigger. Other
selectors and inputs retained stock compact sizing. It also drew a font chevron
while ordinary ComboBox used egui's filled triangle. The constant is removed;
native selectors share an open-stroke chevron. ComboBox salt hashing and popup
keyboard/focus ownership are preserved. Inline captions still edit within their
matching display slot, rather than growing only when editing.

The buzzer loop toggle no longer uses egui's selectable button frame, whose
appearance-dependent stroke enters atom layout. It reserves padding once and
paints hover/selection/focus stroke inside its stable rectangle. Text and the
neighboring Repeats input do not shift. Stop uses a square icon and is disabled
while the coordinator-owned buzzer lane is idle, disconnected or busy; E-STOP
prevents starting but does not prevent stopping.

Paired PCController PR [#650](https://github.com/atomicdeploy/PCController/pull/650)
tracks explicit tone tests in the existing exclusive buzzer lane, keeping
synchronous command ACK, bounded expiry and immediate zero-frequency stop.
Without that change a long tone tester command had no tracked activity and
would become impossible to cancel through a contextual Stop. Native, Web and
tray use the same advertised activity; no tone/melody catalog writes or firmware
changes are required. Autonomous board-origin beeps remain separate note events,
not proof of owned melody/tone playback.

## Verification and delivery boundary

Rendered tests compare input/ComboBox/color-trigger/color-field/menu heights in
all four densities, both themes, four scales and 12/14/20-point fonts. Hover/selected
and focused loop states retain exact caption and neighbor geometry. Existing
rename, dropdown keyboard, Custom expansion, swatches and cancellation coverage
is retained. Native tests use inert fixtures; Web guards verify the shared policy.
The four-choice contract/serialization and geometry coverage is updated;
tests and CI were not run for this iteration at the user's request. Compiler
checks are source validation, not installed UI or screenshot acceptance.

The current authoring process is deliberately not restarted while user readiness
is pending. Runtime delivery and a current installed-build screenshot are not
established by these source/render tests. Cafe GUI smoke-launch remains denied;
source alignment alone is not Cafe runtime delivery. No physical buzzer was used.
