# Designer interaction consistency

Native and both Web authoring entry points share these behaviors:

- Caption editing keeps a fixed font, baseline, inset and row height; borders
  are painted inside the native slot. Searchable compact icon selectors reserve
  the normal glyph slot separately from their chevron.
- Hover action slots fade together. Appearance has System default / Enabled /
  Disabled animations and transitions. System follows the Windows animation
  setting in native Windows and the browser reduced-motion preference on Web;
  other native hosts retain their toolkit animation default. Explicit Disabled
  also removes Web smooth scrolling. This is not an accessibility-policy claim
  for every Linux desktop or macOS setting.
- Name/icon edits preview in visible library/navigation surfaces without
  mutating saved definitions. Discard restores the last original/acknowledged
  draft. Queued controller Save is not an acknowledgement. Existing save ACK
  reconciliation updates catalog identities and placed copies, preserving
  custom cue names and unknown semantic IDs.
- Preferences and Effects Designer use the same preset/Custom popup. Custom
  expands in place with session-only recent/application swatches, HSV and HEX.
  Popup bounds/scrolling preserve access on short screens. Six-digit RGB effect
  metadata requires the paired PCController custom-color change; no firmware
  schema or output command changes.
- Quantize, Remove delay, selection transforms and audio preview/delete actions
  reflect their context. Raw lighting JSON and JSON command responses are under
  a collapsed Technical disclosure rather than an always-visible dump.
- Timeline lane selection determines Add step and Insert placement. Blank
  double-click adds there; the additional Add track selector creates a lane and
  its first step from declared controls/capabilities. LCD and segment display
  lanes are distinct. Double-click/Enter selects a source and focuses properties.
  Ctrl+A/C/V/D, Delete and arrow nudges are scoped away from text inputs and open
  popups. Clipboard insertion preserves relative timing and validates bounds
  before an atomic insertion. Repeat ghosts remain projections of source cues.
- Owned field/action/icon-preset captions have semantic icons and English/Persian
  translations. User-authored names and stable machine keys stay untouched.

Verification includes isolated Windows MSVC native tests, light/dark caption and
popup fixtures at 100/125/150/200% scaling, source translation parity, TypeScript,
pure lane/retarget/repeat tests and an unsaved live Web draft against the local
backend. No Save/Publish/Run or physical output is used during UI QA. This does
not establish human acceptance, physical timing, or production soak.

Every delivery retains its exact runtime/hash receipt privately. Cafe Pealayer
runtime replacement remains blocked by the recorded smoke-launch denial; source
alignment or a local package is not a Cafe Pealayer deployment. Do not retry that
denied launch through another route. Controller delivery is separate.
