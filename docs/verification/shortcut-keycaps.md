# Shortcut field keycaps

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

Application shortcut values now display as individual keyboard keycaps inside
their input fields. Modifiers and the final key use a quiet raised-key surface,
rounded border and lower lip; plus separators use muted ink. The native painter
uses the current palette, UI font and shared control density, with visible-ink
vertical centering and bounded clipping/ellipsis for narrow fields. Chords retain
their logical left-to-right order in right-anchored RTL fields.

These are decorative keycaps, not extra focus targets or a new binding recorder.
Clicking or tabbing into the field restores its existing text editor, including
copy/paste, selection, typing and accessible raw value. Focus/blur alone does not
change the shortcut or dirty Preferences. Existing binding strings, validation,
save behavior, defaults, global registration and dispatch are unchanged. Other
text fields remain ordinary inputs. Web Preferences uses equivalent keycaps and
reveals its standard input on focus without changing its dimensions.

## Verification

- Complete native UI suite: **566 passed**, one intentionally ignored.
- The three focused keycap tests cover labels/punctuation, actual pointer focus,
  select/copy/paste/changed flags, and 160 native theme/palette/density/font/RTL/
  width combinations at fractional display scale. Caps/ink are centered and
  clipped, and rendering never rewrites the stored value.
- Web: TypeScript checking and existing preference-file checks passed. The real
  isolated Edge fixture passed 24 theme/palette/RTL/phone/tablet/desktop cases,
  with centered keycaps, unchanged blur, text entry, one commit and fixed geometry.
  It never accesses production configuration or the OS clipboard.

## Before

User-provided native screenshot:

![Before: plain-text shortcut fields](../screenshots/shortcut-keycaps-before.png)

Native after-capture and the exact installed-build receipt follow packaging.
Cafe deployment remains user-deferred while its normal route is offline.
