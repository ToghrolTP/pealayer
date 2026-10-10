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

- Clean packaged source: `bf3980797ef1fa1a369bae9827e511dcc98eea44`,
  [PR #127](https://github.com/ToghrolTP/pealayer/pull/127).
- Canonical `build.cmd -SkipTests -NoUpx` reused the shared Cargo cache and
  existing staging directory. Release compile/link, native resource validation,
  executable/downloader smoke tests and Web/PWA guardrails passed. The package
  skipped full-suite execution; the 43 focused tests ran separately.
- Cafe-PC's validated libmpv stayed unchanged:
  `872827614ed0adfca11e68def5273bcfcaea6acf38bbf1950c35980b59f43a5f`.
  The candidate also passed smoke testing with that destination runtime.
- At 2026-10-10 16:27 UTC, graceful peer operation
  `update-681335c1-24cd-4293-aba8-322e75240426` completed and acknowledged the
  installed executable. Live manifest matched the clean source above and SHA-256
  `ec5816beadad7a5e1227ba70c18f89ec914e2b6b4a6df1c70e53eee1d98534f6`.
- Native canonical executable ran as PID 29832 in interactive session 1.
  Authoritative status was paused, controller-connected, no hardware error or
  EStop, NLE workspace and zero original cues. Exact loaded media, complete cues,
  workspace and paused position were preserved. No controller reconnect retry
  was needed and no new application/WER runtime faults were observed.
- Only the executable was transferred; no Rust compilation occurred on Cafe-PC
  and no private media/settings were published. Native human visual acceptance
  and platform CI remain distinct from this runtime/geometry receipt.

## Human visual acceptance

On Cafe-PC, expand a section with hidden channels. Confirm the disclosure is
directly beneath its section header, at the right edge above the first group,
without the former blank area. Resize the sidebar and open the menu. Native
human visual acceptance remains separate from the automated geometry checks.
