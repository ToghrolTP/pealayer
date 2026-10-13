# Relay actions and card caption centering

Native normal relay cards previously selected OFF through egui's default
selection color, while only selected ON received a green override. Compact
controls used ordinary accent-derived button visuals. Web selected ON/OFF
used Ant Design's `primary` styling; mobile appearance preferences likewise
applied the accent to filled/tonal power controls.

Native/Web/mobile power actions now retain green ON and red OFF semantics.
Selection emphasizes the same semantic tone rather than changing its meaning.
Native and mobile disabled actions remain muted, and existing lock, E-STOP,
programming, owner, busy and command-routing guards are unchanged. Mobile still
supports outlined, tonal and filled appearances and contrast-aware filled text.

Native headers mixed fixed 24-point caption/editor/icon slots with controls
that can grow under density and font preferences. A later larger child can
grow a horizontal row after earlier children were positioned. Both card modes
now reserve density, Body/Monospace font and compact-action height before
allocating any child. Caption and inline rename retain the existing same font,
four-point inset, fixed slot, logical alignment and inside-only editor stroke.
This changes their shared row placement, not either text painter independently.

Mobile captions, facts, action labels and state pills trim outer theme leading
consistently so text groups center beside fixed icons, while wrapped internal
line spacing and accessibility scaling remain. Future mobile inline rename
must reuse the caption's actual metrics rather than a stock input layout.

Offline cargo check, Web TypeScript compilation and Dart analyzer passed.
Geometry and semantic-style regressions were added but not executed in this
root iteration, per the user's tests/CI skip request. No hardware command or
tone was used for styling verification. Native installation remains gated by
the preserved generated Web distribution and approved host/runtime packaging.
Mobile source is handed to the active Flutter owner for one combined next APK;
the existing Android delivery receipt is not proof of these new changes.
