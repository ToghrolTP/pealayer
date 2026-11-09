# Contextual −/+ zoom shortcuts

The built-in keyboard-shortcut policy is enabled by default. Minus zooms out;
Plus or Equals zooms in, including Shift+= and keypad Plus as delivered by egui.
The main timeline no longer requires a preliminary canvas click. These are
editor-view shortcuts, not playback seeking, browser zoom or hardware commands.

The front visible editor/dialog owns keyboard zoom. The Effect Designer applies
it to its sequence view using its existing center-anchored zoom/scroll path.
Other dialogs without a zoomable view block the timeline behind them. Native
modal layers, foreground/ordinary windows, child layers, popup ownership and
text-edit focus remain respected. A focused field retains normal −/+ typing;
Ctrl/Cmd/Alt and underscore are not reinterpreted as editor zoom. Key repeat is
supported and an input is consumed only once per frame.

The existing master preference `keyboard_shortcuts_enabled` remains the single
enable policy; no duplicate binding store or second key listener is introduced.
It was already true in the inspected current instance, with no bare Minus/Equal
hardware binding conflict. No unrelated setting was rewritten. The shortcuts
dialog now includes the zoom keys.

Five real egui event/render checks passed: exact default/modifier/repeat routing,
front Window/Modal ownership and non-zoom dialog shielding, field/disabled-key
preservation, actual main timeline scale without canvas focus or seeking, and
the actual sequence view changing scale without changing cue values/timing.
The existing designer wheel-navigation regression is retained as a separate
check, preserving pointer anchoring and outside-canvas wheel ownership.

## Local delivery receipt

- Clean packaged source: `85993976d334eb5c858af3ec4c6d54c797c81f66`.
- Installed canonical executable SHA-256:
  `59a6e37c8390d4d77011f413d7eb5f25b0b082247062c9f807ffeeb969acda98`.
- Reused canonical staging/cache; release build, native/quick-action resources,
  libmpv smoke and Web bundle integrity passed.
- Fresh native authoring was clean. The graceful updater restarted the existing
  app and acknowledged the exact executable digest; its local process API
  independently reports the packaged source. The historical comparison was
  left untouched, and `keyboard_shortcuts_enabled` remains true.
- Paused playback stayed at exactly 578286 ms. Native OS media-session readback
  still matches title/state/position/duration/rate, with artwork present; the
  queued media-discovery fixes are now installed too.

Source/egui checks and installed identity are not physical keyboard acceptance;
the user has been asked to verify the keys in the two views and a text field.
Cafe remains user-deferred offline until its connection returns. General CI and
the stacked PR merge are separate gates, not reasons to replace the validated
runtime with an older base.
