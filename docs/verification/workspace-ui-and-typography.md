# Workspace presentation and typography

## User controls

Preferences → Appearance → Interface offers **CraftUI** (default) and
**egui Dock**. This changes docking chrome, not the media engine or panel data.
Both presentations use the same workspace/profile, panel visibility, commands,
timeline, hardware controls and palette. Switching does not reload media.
Unchanged accordion/dock nodes and floating-window identity/placement survive
the egui Dock projection; an explicitly rearranged group becomes its edited
tab/split layout. Workspace locking and modal input ownership remain enforced.
The optional sidebar remains available in either presentation.

**Group Preferences navigation** shows uppercase section labels above the
navigation buttons. It can be disabled without changing section identities or
discarding a draft. Narrow windows use the compact section selector. The rail
scrolls independently in short windows. PakarController's DEVICE, CONNECTIONS
and CONTROLLERS labels were already present in Device Settings; they had not
been removed. The companion change makes grouping configurable there and in
Pakar's application Preferences. Both applications offer both workspace
presentations; Pakar keeps egui Dock as its default. Its adapter pins CraftUI's
egui 0.36 line to avoid mixing incompatible egui runtimes.

Typography provides independent **UI font** and **Monospace font** choices.
System font is the default for both; Choose installed font opens a searchable
host font-family selector. Changes preview live, Save confirms and Discard
restores. The Web application uses the same preference contract and appearance
events. A browser can use a selected family only if installed on its own device;
otherwise it retains native CSS fallbacks rather than downloading private fonts.

## Root cause and implementation

- Monospace previously defaulted to bundled Hack. System selection relied on
  hard-coded files, including one Linux monospace path, rather than the DE choice.
- Fontique now resolves installed fonts via DirectWrite, CoreText or fontconfig.
  Windows uses NONCLIENTMETRICS for its UI family; AppKit supplies the macOS UI
  and user fixed-pitch families (not Fontique's generic Courier fallback). Linux checks the active
  GNOME/Cinnamon/MATE, KDE or XFCE preference first, with bounded helper calls,
  then native generic fallback. TTC face indices are retained. No copied font
  files, downloaded font packages or replacement application theme are needed.
- Existing explicit font choices remain explicit; selecting System font adopts
  the new native resolution. OS font preference changes are read when fonts are
  installed (startup or changing font selection), not continuously polled.
- Status emphasis resolves the selected family's semibold face/weight rather
  than substituting unrelated Arial/Courier bold faces. Font size, icon fallback
  and fixed hit geometry are retained.
- The egui Dock dependency pins the reviewed egui 0.37 port in upstream PR354.
  CraftUI remains the canonical workspace model, avoiding competing persisted
  layouts and duplicate panel implementations.

## Evidence and limits

Local native library suite: 1,027 passed, one ignored. Additional Preferences
suite after searchable-font controls: 20 passed. Coverage includes panel/active
tab/hidden state, finite floating geometry and identity before first paint,
unchanged accordion/fixed split preservation, monospace advances, icon fallback,
preview/discard and semantic preference icons. Web build and appearance tests
are recorded with the publishing checkpoint.

An isolated Windows review instance rendered the actual CraftUI workspace and
grouped Preferences. Read-only Win32 captures were inspected for nonblank native
content, legible labels, consistent grouped navigation and footer placement.
Private captures: `verification/craftui-preferences.png` (workspace) and
`verification/grouped-preferences.png` under the canonical program root.
These are review-build visuals, not a production deployment receipt.
Linux DE and macOS font resolution require native desktop verification.
Cafe's configured SSH endpoint refused connections during this pass. No hardware
outputs were changed. Pakar's build remains gated by stale donor comparisons;
the grouping change must not be described as a tested Pakar binary until those
evidence checks are reconciled by its owner.
