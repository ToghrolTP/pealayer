# Hardware Monitor hidden-channel disclosure

## Cause and repair

The native `3 hidden` menu used a standalone right-to-left layout with vertical
center alignment. That child inherited the remaining panel height: it centered
the menu in empty space and pushed the channel groups down. The count itself
was not responsible.

The menu now lives inside an intrinsic-height horizontal row. It remains aligned
to the channel cards' right edge, uses the normal themed button padding and
retains the existing four-point gap before the groups. No row is reserved when
there are no hidden channels. The existing Show channel menu and authoritative
controller presentation updates are unchanged; this is a native layout fix,
not a hardware or API contract change.

## Verification

Run with an isolated `PEALAYER_CONFIG_FILE` established by the parent process,
the configured host libmpv and the existing shared Cargo target directory:

```powershell
cargo test --lib --locked --jobs 1 hardware_ -- --test-threads=1
```

Result on 2026-10-10: **43 passed, 0 failed**. The three new regressions render
the actual grid inside the Hardware Monitor's scrolling/collapsible sections:

- Top-right alignment, ordinary menu/group spacing and content-sized allocation
  across 72 combinations: 280/320/720-point widths, 240/900-point heights,
  light/dark themes, compact/regular cards and 1/1.5/2 display scale.
- Primary-click opening of the existing menu, all three hidden channel choices,
  and unchanged channel/playback state when merely opening it.
- No disclosure row when there are no hidden channels.

Existing card width, scroll, drag/drop, manager and hardware-contract checks
also pass. These headless widget/geometry checks are not native screenshot or
physical-output acceptance. No production output action is needed for this fix.

## Deployment

The destination was freshly checked before replacement: paused media, connected
controller, no hardware error or EStop, and a clean canonical main checkout.
Build and updater receipts follow after delivery; no Cafe delivery is claimed
by a local compile alone.

## Human visual acceptance

On Cafe-PC, expand a section with hidden channels. Confirm the disclosure is
directly beneath its section header, at the right edge above the first group,
without the former blank area. Resize the sidebar and open the menu. Native
human visual acceptance remains separate from the automated geometry checks.
