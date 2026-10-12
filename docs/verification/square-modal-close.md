# Square modal close controls

The old unsaved-preferences close came from Elegance's text-button path: × was
measured as a short text label with independent horizontal/vertical padding.
That made it a narrow rectangle and left its glyph at the Small text size.

`ui::dialog::close_button` now allocates an exact 32-by-32 point square for paint,
hit-testing and the inset focus outline. The 22-point × is centered by its visible
ink bounds, not its unused font ascent. Body fonts, text-button padding, large
interaction sizes, hovering, pressing and focusing cannot change that square.
Focus uses the app's accent; typography and palette remain host-controlled.

Unsaved Preferences and cue-discard confirmations use shared confirmation chrome
with this close control. The custom cue editor and authority prompts use the
same control. Close/Escape keep the dirty draft; Save/Discard remain explicit.
Backdrop clicks do not silently dismiss unsaved-change confirmations. Footer
actions wrap when necessary. Alert-dialog semantics and a Close accessibility
label are retained. The dependency/cache is not edited or forked; stock text-sized
close controls are prohibited by the repository's UI guidance.

Geometry regressions exercise both native/Persian font preferences at 100%, 125%,
150% and 200%, with exaggerated text padding/button font/interact size and idle,
hover, pressed and focused states. Actual Preferences rendering verifies that
corner-close and Escape preserve both dirty and saved configurations. Existing
cue and authority interaction suites cover their guarded dismissal behavior.

For repeatable native visual inspection, `cargo run --release --example
square-close-preview` uses the production renderer/fonts in a clearly labeled
isolated fixture. It starts no player, server or board connection and saves no
preferences/window state. Inspect/capture it with the maintained read-only
Windows capture script; do not manufacture changes in a user's open draft.

Native/source/fixture checks do not substitute for human acceptance or a physical
production soak. Cafe runtime delivery remains blocked by the recorded launch
denial; source alignment and local installation remain separate evidence.
