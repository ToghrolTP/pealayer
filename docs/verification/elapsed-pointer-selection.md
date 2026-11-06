# Elapsed-time segment selection

The user reported that clicking any part of the elapsed field selected hours,
both when entering edit mode and while already editing.

## Cause and correction

Native rendering restored the selected group before each TextEdit paint. On
mouse release it read the stored cursor to infer the clicked group, but that
cursor could already have been overwritten by the restored hours selection.
Display-mode activation also unconditionally began with group zero.

The native field now retains the TextEdit output and maps the actual pointer
through its painted galley and position, not its saved selection. This respects
centering, font metrics and horizontal scrolling. Its pointer origin includes
the centered galley's negative left bound, matching TextEdit's actual paint
origin. A released activation click cannot retarget a newly expanded edit mask.
Display-mode activation uses
the text actually visible when clicked, including when milliseconds are hidden.
The original add_sized allocation/layout and mask/commit/cancel rules remain.

The Web field had an analogous focus-order problem: focus selected hours before
click read the browser's caret position. Pointer focus now defers programmatic
selection until click has read that position; keyboard focus still selects a
group. Pointer-up/cancel, blur and keyboard input clear the pointer guard.

## Verification and delivery status

- Existing timecode mask, localized digits and seek/blur/Enter/Escape guards pass.
- TypeScript `--noEmit` passes.
- Added a native rendered pointer regression for every visible group when
  entering from display mode, then all four groups while editing, with and
  without displayed milliseconds at 1×, 1.5× and 2× scale. It checks the actual
  retained selection, unchanged value and no seek. All five focused native
  elapsed tests now pass, including this rendered regression. The first run
  exposed the centered-galley origin mismatch; correcting it made all cases pass.
- Browser pointer interaction and installed-native acceptance remain pending.
  A combined build and isolated native verification are now in progress.
- The previously completed taskbar-artwork package does not contain this later
  source change. Cafe deployment remains user-deferred while offline.

Continuation: include this source in the next combined build, run
`ui::controls::tests::elapsed_` with an isolated configuration, verify native and
Web pointer interactions, then deploy through the normal guarded updater.
