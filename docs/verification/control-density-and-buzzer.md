# Shared control density and contextual buzzer actions

Preferences / Appearance / Control density uses a five-step slider: Extra compact
(18 points), Compact (24), Regular (28), Comfortable (32), and Spacious (36). Configuration
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

## Restored original density

The parent of `88741e3` pinned egui 0.36.2 and did not override its spacing:
`Spacing::default` supplied `interact_size.y = 18` and `button_padding.y = 1`.
Earlier `2b079a9` had explicitly removed the 30-point theme override. The
unification in `88741e3` then introduced a 24-point Compact floor; `b455f59`
added the 28/32/36 choices without restoring the original minimum.

Extra compact restores that 18-point minimum **and** 1-point padding. Using the
new icon-derived padding would enlarge the restored controls again. These are
logical points, not physical screen pixels. Large fonts may expand the height.
Existing saved choices and the Compact default are unchanged. Native and Web
share the `extra_compact` value; Web control line-height also accommodates 18px
instead of its default 22px text line forcing inputs taller.
The exact-size test checks actual input rectangles and text origins across hover,
both themes and fractional scales. The shared input/selector/menu matrix and
serialization tests now include all five choices. Repository instructions retain
the historical baseline so future theme work cannot silently drop it again.

The restored-size pass also exposed PWM step buttons reserving a square but
allocating additional horizontal text padding. Their measured width now matches
the font-aware square reserved by the row, including narrow cards. Ten focused
native density tests, six Web appearance tests, responsive/parity guards and
TypeScript checking pass. This is source/geometry evidence, not a deployment
receipt: Cafe's SSH endpoint was unreachable during this pass, and launching
the isolated native review app remains blocked by execution policy.

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
all five densities, both themes, four scales and 12/14/20-point fonts. Hover/selected
and focused loop states retain exact caption and neighbor geometry. Existing
rename, dropdown keyboard, Custom expansion, swatches and cancellation coverage
is retained. Native tests use inert fixtures; Web guards verify the shared policy.
The four-choice contract/serialization and geometry coverage is updated;
tests and CI were not run for this iteration at the user's request. Compiler
checks are source validation, not installed UI or screenshot acceptance.

The later user confirmation that drafts were saved allowed Cafe's approved
native update, documented in the [issue 80 delivery receipt](issue80-native-delivery.md).
The four-step density changes are installed there, with authoring state and its
own runtime preserved. No physical buzzer was used. Actual control interaction,
usable installed-UI capture and local/Erfan latest activation remain open.
