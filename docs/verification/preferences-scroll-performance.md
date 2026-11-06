# Native Preferences wheel responsiveness

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

The reported symptom is stuttering mouse-wheel scrolling in the separately
hosted native Preferences window, not the Web editor.

## Work removed from scroll frames

- Resolve the effective language once per editor pass and pass it to numeric
  context menus too. Resolving System for every caption opened/read the Windows
  locale Registry key repeatedly. Resolution remains frame-local, so changing
  language or the operating-system locale is not frozen by a permanent cache.
- Serialize immutable numeric Reset defaults once per process, instead of
  rebuilding the complete default configuration for every numeric row, including
  offscreen rows. Live configuration, device catalogs and drafts are not cached.
- Resolve the config path when attaching the file watcher or actually reloading,
  not on every helper frame. The normal installed path resolution otherwise
  checks three portable-marker files on disk. File watching and live preview
  remain enabled; ordinary wheel rendering no longer makes these storage probes.

These repeated paths predate CraftUI. Font installation already has a selection
cache and was not replaced or disabled. Scroll physics, touchpad input, palette,
per-section offsets, Save/Discard and native-window ownership are unchanged.

## Verification and limits

The actual editor renderer is exercised with continuous pixel wheel events and
Windows-style line notches. Regression checks cover nondecreasing offsets,
scheduled repaint of the smoothed notch tail, settling without endless redraw,
unchanged draft values, narrow/wide layouts and unchanged numeric Reset values.
Existing Preferences tests cover independent section offsets, live OS theme,
density/animation changes, unsaved drafts and card containment.

The same 120-frame unoptimized renderer fixture took 1.261 seconds before and
1.045 seconds after removing repeated locale/default work (about 17% less total
fixture time). This includes startup/layout and is not a display-FPS benchmark.
The helper's additional filesystem-probe removal is outside that editor-only
measurement. These measurements do not prove the user's live wheel symptom is
fully resolved; installed-build interaction acceptance remains necessary.

The regular application must not be replaced while its native Preferences draft
is unknown. Preserve it until saved/discarded and closed. The retained October
10 comparison stays untouched. Cafe deployment remains user-deferred until its
normal connection returns; local source/tests are not remote delivery.

## Candidate receipt

- Final native source: `45141dcd76a13a4ec91e59b056422437897d9631`.
- All 29 Preferences tests passed on that source, including existing appearance,
  layout, Save/Discard and section-navigation regressions.
- Clean-source Windows packaging, embedded Web/PWA integrity, runtime smoke and
  all seven quick-action icon-resource checks passed.
- Packaged executable SHA-256:
  `93feea9af522c011436663e849ec38a0fc8815364ca483113a317eeafdd38a2c`.
- The independent current Dock comparison restarted with this exact source;
  its local authoring API confirmed the running identity before Preferences was
  opened/navigated through the shared command API. No hardware was connected or
  actuated. The retained October 10 executable digest is unchanged.
- Regular deployment remains blocked: the fresh authoring inspection still
  reports an unknown native Preferences draft. No automatic discard/restart was
  attempted. Actual mouse-wheel smoothness awaits user interaction acceptance.

Private final-test and package logs are retained in the canonical program root's
`verification` directory, not published as configuration or media evidence.
