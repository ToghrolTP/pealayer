# About dialog layout

## Corrected geometry

- The body previously set its maximum width to the **outer** window width.
  Content could grow beyond the already allocated title bar. The dialog now
  respects the available inner width and uses the shared square Close control,
  with the title constrained to the space remaining beside it.
- The navigation previously reserved a fixed 144-point button width. The rail
  now measures the translated captions and includes both icon and trailing
  padding. Narrow windows use equal-width navigation cells; labels that genuinely
  cannot fit are ellipsized with a full tooltip, never clipped at the border.
- The library identity column previously used an allocation hint, not a reserved
  width. Short names reclaimed that space, moving each description to a different
  position. Every row now reserves the same column width and aligns at the top.
  Narrow content stacks identity, license and purpose rather than overflowing.
- Each About tab has independent scroll memory. Branding, resize/move behavior,
  links, board identity data and Escape-to-close are preserved.
- Library versions come from the package's Cargo.lock during the build. The
  libmpv2 version is explicitly identified as the Rust binding, not the runtime
  DLL version. Packaged dependency notices remain authoritative for licenses.

Historical source evidence: the outer-width assignment and per-row allocation
were introduced in `967b2971`; the fixed navigation widths came from `0ec9ffc2`
and remained when `434e2c4e` adopted the shared navigation control. These are
layout causes, not evidence of a particular host's installed state.

## API-driven review

The same local command is available over IPC/JSON and JSON-RPC/HTTP/WebSocket:

```json
{"command":"open_about","tab":"libraries"}
```

```json
{"jsonrpc":"2.0","id":1,"method":"pealayer.about.open","params":{"tab":"libraries"}}
```

Tabs are `overview` (default), `build`, `board`, and `libraries`. The command opens
and focuses the **local** native About dialog, including in a remote-client
instance. Invalid tabs are rejected locally, never relayed to the authority.
An accepted response means the UI request is queued, not proof it has rendered.
Opening About neither connects to a controller nor changes hardware outputs.

## Verification boundary

Five focused native/API tests pass, covering aligned library columns and narrow stacking, title and
navigation bounds across interaction states/scales, all four tabs' window bounds,
Escape, and local API routing/validation. Twenty shared dialog tests also pass, covering Close geometry,
theme tokens, focus and non-destructive modal behavior.

Packaging and actual native capture are waiting for the existing review Preferences
helper to close normally: its authoring report is `unknown`, so it has not been
discarded or replaced. The previously staged density package does not include this
About follow-up. Cafe's configured SSH endpoint refused connection during this pass.
The PR checkpoint records subsequent activation/capture results. No production or
hardware acceptance is implied.
