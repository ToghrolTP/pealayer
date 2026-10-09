# Mobile button content alignment

Trackers: [#80](https://github.com/ToghrolTP/pealayer/issues/80) and
[Web parity #63](https://github.com/ToghrolTP/pealayer/issues/63).

## Source audit and repair

The older bottom-navigation problem already has a specific repair: Ant's
collapsed-menu selectors otherwise keep a hidden label in layout. The current
phone rules hide that label and center the entire icon; those rules are retained.

Remaining button paths used inconsistent geometry. NLE play and volume/mute
overrode Ant Design's inline flex with inline grid. Ant Design 6 also inserts an
NBSP pseudo-element before button icon content to influence text baselines, while
the SVG itself is inline. That line box is not the glyph's visual extent, so its
baseline spacing can offset icons in compact zero-padding and mixed-size buttons.

The repair keeps a single centered inline-flex geometry for button roots and icon
wrappers, makes button SVGs block-level within those wrappers, and removes only
the button-icon baseline spacer. It covers normal buttons, icon-only buttons and
buttons in portaled dialogs/dropdowns; colors, dimensions, loading animations,
focus treatment, RTL ordering, labels and command handlers are unchanged.

The NLE play and volume-specific grid overrides now use that same flex layout.
Short-screen/mobile dialog footers explicitly center mixed-height buttons too.
There are no icon top offsets, transforms or mobile-only pixel nudges.
Intentional left alignment in stacked page headers and bottom alignment of
labeled sequence-editor inputs is not mistaken for button-content misalignment.

## Verification limits

TypeScript and production/PWA generation passed. Responsive CSS guardrails were
extended for root/wrapper/SVG layout, spacer removal and transport/footer overrides.
The coordinating task ran `node web_ui/scripts/test-responsive-layout.mjs` and
reported a pass. Other test suites and slow CI waits remain deferred under the
user's standing request. A current screenshot identifying any remaining affected mobile
button was requested; real phone/tablet/desktop visual acceptance is pending.
Do not turn source inspection or compilation into a claim of screenshot proof.

This pass changes presentation only, not Rust/API contracts.

## Installed checkpoint — 9 October 2026

PR: [#102](https://github.com/ToghrolTP/pealayer/pull/102).
Final clean runtime source: `06d410afbfe1bef1d25ea541596c7b9cf1aa16ff`.
Main's merged performance PR #103 was incorporated before the final package;
the earlier mobile-only package was an intermediate, not the final delivery.

- TypeScript, production/PWA generation and diff checks passed. The coordinating
  task independently ran `node web_ui/scripts/test-responsive-layout.mjs`: passed.
  Other test suites and slow CI waits stayed deferred as requested.
- Combined locked native release build on David took 1m 25s, with fourteen existing
  warnings. No compilation was performed on Cafe. Seven Windows quick-action
  resources, packed-executable integrity and independent David/Cafe staged
  runtime smoke checks passed; both smoke processes exited 0.
- Final executable: 11,145,216 bytes; SHA-256
  `24783c893eafe530b0908364e04e33229b0b77e09b3712384830773579a16f7b`.
  Its embedded build identity is clean and matches the final runtime source above.
- Cafe's first upload returned HTTP 503 after one acknowledged chunk. Its
  receiving status was inspected and the incomplete operation aborted through
  the API; the server confirmed removal of that staging file. The final upload
  used smaller chunks with bounded, acknowledgement-aware retries and succeeded.
  Cafe and then David restarted through the graceful, destination-DLL-validated
  updater. No healthy application was forcibly terminated.
- Both process-local live update manifests report the exact final runtime source
  and executable. Their distinct destination DLLs were retained. Cafe's interactive
  process responds, PCController service is Running, hardware is connected, and
  the paused media position plus all three cues persisted. David responds and
  remains a connected cache-only Cafe consumer with no local hardware scheduler
  and zero reported preview drift. No physical output or playback was activated
  for this layout-only pass.
- HTTP checks on both running instances confirm the centered root rule and removed
  icon spacer in the served `assets/app.css`. PWA identity is `a9f1358189ee24b6`,
  with 39 precached resources. This proves delivery, not browser-rendered alignment.
- Erfan's stopped installation was updated atomically and the final executable,
  original rollback and unchanged destination DLL verified. KMPlayer remained
  running; Pealayer was not launched, as required. The intermediate executable
  was removed after replacement; the pre-pass rollback remains recoverable.
- Canonical/private host manifests were refreshed. Current mobile screenshots and
  phone/tablet/desktop interactive visual acceptance remain pending the requested
  affected-button example. No screenshot proof or complete Web parity is claimed.
