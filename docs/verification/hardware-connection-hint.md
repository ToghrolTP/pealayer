# Plain-language hardware connection messages

Hardware Monitor previously placed the engine's complete connection diagnostic
directly into the empty-state card. The local-library probe and external
connection failure can both be useful diagnostic evidence, but DLL names,
environment variables and nested transport errors are not default user copy.

The disconnected/retrying card now says:

> Couldn't connect to PCController.
> Check that it is running, then try again.

A connected controller whose controls could not be loaded instead says:

> Couldn't load the hardware controls. Try refreshing the connection.

The choice comes from existing connection/catalog state, not English error-text
parsing. Board absence and programming/recovery remain separate states; this
change does not claim a physical USB disconnection from a failed host connection.

A circled information control exposes the original diagnostic on hover or click.
Click keeps it available for reading/selecting/copying on touch and keyboard
interfaces too. The native menu and Web popover are width/height bounded and
the diagnostic is copyable. No original error is discarded, and there is no
new RPC contract, connection attempt, output command or polling timer.
English and Persian application copy are maintained on both surfaces.

## Verification

- Four native Hardware Monitor tests pass, including actual info-button clicks
  at 220/320/640-point widths in both themes, hidden-by-default diagnostics,
  retained originals, catalog/transport distinction and command-free rendering.
- Web model/controller-error checks and TypeScript passed.
- The real headless-browser fixture passes five unavailable states at
  390/768/1440 widths in both themes. It opens the info control, checks full
  diagnostics, verifies no overflow and confirms that no command was sent.
  It uses isolated fixture data, not a production board or connection.

These are actual browser captures of that isolated fixture, not production
connection/board evidence:

![Phone-width default summary with circled information control](../screenshots/hardware-connection-hint-web.png)

![Expanded technical details in the light theme](../screenshots/hardware-connection-details-web.png)

The full native suite passed: **1,117 passed, one ignored**. Production Windows
packaging passed TypeScript/Web/PWA integrity, native resources, all seven
quick-action icon checks, libmpv smoke and downloader smoke. Packaging skipped
duplicate regression runs; the native suite and focused Web/browser checks
above ran separately.

The graceful updater installed source
`6d678c71749ac8b6ef280e2b88dfc98395c5b419` into the canonical local application.
Fresh manifest readback matched executable SHA-256
`ff0a8706c2f59204dbf1c6c31675a2863905b39f780ba247d3b145c520b27484`.
Process-local authoring was clean before and after replacement, and CraftUI and
System theme settings were retained. No hardware commands were sent for these
checks. A private read-only capture of the installed native UI confirms the
short message and information control; that full-window image includes user
media and is not published.

The restart also exposed a separate tiny-window condition despite valid saved
application dimensions. The existing Maximize API restored a usable window;
this is a recovery, not a source fix. Investigation belongs to
[the native window integration issue](https://github.com/ToghrolTP/pealayer/issues/34).
The additional eframe window-settings persistence is a candidate to examine,
not a proven cause. The running application's window should not be manipulated
again merely to improve this report's captures.

Cafe deployment remains user-deferred until its normal connection returns.

## Message alignment and deliberate line break

The native card's info button was allocated first in a reversed horizontal row.
Its following shrink-to-fit Label inherited that placement, so the entire
English message block was right-anchored even with left-aligned glyphs inside
it. The fix reserves the complete remaining text width and gives it its own
application-direction layout. English starts at the card's leading margin;
Persian stays right-aligned. The info button remains at the logical trailing
edge, independently of caption alignment.

The instruction now begins on its own logical line in English and Persian, on
both native and Web surfaces. Narrow panels still wrap normally. Web uses
`white-space: pre-line` and logical `text-align: start`, not a physical-left
override. Raw diagnostics and info/copy actions are unchanged; no connection,
authority or board output is touched by rendering or inspecting the card.

Five focused native checks passed, including actual info-button clicks and a
paint matrix at 220/390/640-point widths, 1/1.5/2 scales, both themes/directions
and with/without diagnostics. It checks every rendered line's physical leading
edge and the explicit two-line English layout when space permits. Web
availability/message checks also passed. Fixtures use a transport-free engine
and private parent-provided config/instance identities.

Installed follow-up receipt and native before/after captures pending.
