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

These checks establish source behavior, not physical hardware or human visual
acceptance. Runtime deployment receipts and any remaining acceptance checks are
recorded in the owning PR; do not infer deployment from a library test alone.
