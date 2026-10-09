# Palette-safe Elegance integration

Continuation of [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
Based on merged external-mpv and UI-smoothness work, not an older UI branch.

## Applied selectively

- Preferences and the shared Audio/Subtitle dialog sections now use Elegance
  Cards with the existing compact padding, left-aligned Phosphor headings,
  disclosures, input controls and stable width constraints. Their surfaces are
  opaque, rather than a translucent stock panel tint.
- Cue metadata uses neutral, case-preserving Elegance badges, not extra brand
  colors. The specialized cue editor, icon/color pickers, segmented time input,
  track interactions and hardware pointer-down controls are retained.
- External-only mpv shows a width-bounded Callout with actual observed idle,
  paused, playing, buffering, waiting or connection-error state. It no longer
  claims a connected player is necessarily playing. Errors retain their actual
  details; there is no synthetic connection progress.
- Preferences' unsaved-change dialog retains its existing Elegance Modal, with
  consistently sized icon-labelled actions. Save follows the user's accent;
  Discard remains semantic red and Cancel neutral.

## Palette and live theme contract

The pinned dependency remains `egui-elegance = 0.16.1`. Its public `Palette`,
`Typography`, `Theme::install` and `Theme::current` APIs were inspected in the
published source, alongside the [upstream API guide](https://github.com/matrix-research-inc/egui-elegance#readme).
The earlier adapter only overrode some stock fields, leaving secondary text,
semantic button shades and typography inconsistent with the host.

`src/ui/elegance_theme.rs` now supplies every palette field. Surfaces, inputs,
text, borders, focus and typography come from the resolved Pealayer style;
semantic/subtle colors come from the existing shared palette catalog. There
is no new theme, palette preference or parallel Web color table. Neutral and
Studio, live OS light/dark changes, system/custom accents and independently
hosted dialog contexts continue to use the existing appearance contract.

Installation immediately restores the exact host style. Neither light/dark
style, the system-theme preference, font sizes, spacing nor interaction defaults
are replaced by Elegance's stock theme. The library's symbol font is appended
as a lowest-priority fallback rather than replacing Pealayer's font registry.
The adapter caches the source style/palette, so repeated sections do not
recompute contrast or reinstall the theme every frame.

Elegance's filled buttons use white ink. Only those fills are hue-preservingly
deepened as needed to reach 4.5:1 white contrast; structural focus and semantic
status colors remain exact. Native adaptive-ink primary controls are unchanged.
Do not introduce filled `Accent::Sky` or stock white-checkmark widgets against
arbitrarily bright focus colors without separately solving foreground contrast.
Do not replace hardware push/hold controls with click-on-release widgets.

## Verification and acceptance

Focused tests cover both palettes and OS light/dark/light transitions, bright
green/white/blue custom accents, equality of both complete host styles before
and after integration, unchanged System preference, cached unchanged style,
filled-button contrast, opaque cards and stable widths at 180/280/520 points.
An external-player notice regression covers actual observed states and errors.

## Installed build (9 October)

Implementation: [PR #108](https://github.com/ToghrolTP/pealayer/pull/108).
Installed source: `1a6943c8fe3b9a2a1ed5f8fec337f28e3fe8d433`; subsequent
documentation-only commits do not change the package's embedded identity.
Executable SHA-256:
`5b74194ad0f18b314ff813822ffa820cd25d38d65550e7987d92f19ec13a57ca`.
Packed/unpacked sizes: 11,319,808 / 45,293,056 bytes.

- David and Cafe installed through the existing graceful chunked updater.
  Fresh local manifest/process APIs confirm the exact source/hash and responsive
  interactive processes. Cafe is still the hardware authority; David is a
  connected consumer with no competing local hardware scheduler.
- Cafe remained paused at 660.8 seconds with all three original cues intact,
  connected hardware and null hardware/sync errors. Theme `dark`, palette
  `native` and accent `system` were preserved; no production appearance settings
  were changed merely for the checks.
- David retained DLL `e56ce67cd00f06a59dc7ed5b97a49a9182a381554c755dc4570a52af1ef30e65`;
  Cafe retained `872827614ed0adfca11e68def5273bcfcaea6acf38bbf1950c35980b59f43a5f`.
  Both host-specific staged runtime smoke checks returned 0.
- Erfan received a stopped-app atomic replacement, retaining DLL
  `0a81c004aae0ee7d512b9a26e38f66281f9591e84e1663215cc3a36a4bde6f6a`.
  Its installed hash matched and runtime smoke returned 0; no GUI was launched.
  Temporary upload/previous-build files were removed after verification; the
  existing pre-feature rollback was preserved. Cafe's temporary upload was also
  removed after its live installed receipt matched.
- Release compilation on David took 1 minute 27 seconds. The four focused
  tests, shared crate-boundary guard, whitespace, seven Windows task icon
  resources and UPX integrity checks passed. No compilation ran on Cafe.
  At inspection, Linux, one Apple-silicon run, Web TypeScript, repository health,
  Actions validation and CodeQL passed; remaining Windows/macOS matrix jobs were
  still running, not waited on or claimed passed.

Human visual review of native Appearance/Audio/Subtitle sections was requested
after installation and remains pending. No new physical-output or audible
acceptance test was performed for this presentation-only pass. These API/build
receipts are not substitutes for interactive visual acceptance or wider Web
parity completion. Existing Web colors/contracts were reused, not redesigned.
