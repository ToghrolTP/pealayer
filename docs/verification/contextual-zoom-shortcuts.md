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

Package, installed runtime and host-specific deployment are separate receipts;
source/egui checks alone are not live keyboard acceptance. Cafe remains
user-deferred offline until its connection returns. The queued native
media-discovery work is preserved in this build, not replaced by an older base.
