# Publishing-authority prompt dismissal

Both native and Web authority prompts always provide an explicit Dismiss action,
an upper-corner close control and Escape dismissal. Native also honors its modal
backdrop dismissal response, rather than ignoring it. These actions remain usable
while playing, production-locked or waiting for a handoff acknowledgement.

Dismiss is local UI acknowledgement of the current authority revision. It does
not request, accept, reject, unlock or release authority, remove a pending claim,
change playback or act on the board. Decline remains an explicit rejection, not
a synonym for Cancel. The same revision stays dismissed; a new authoritative
revision can present a new prompt. The owner and observer branches share this
non-mutating dismissal path. Native action rows wrap on narrower viewports.

Native rendered interaction coverage exercises Dismiss, the square upper-corner
close target, Escape and backdrop clicks for owners and observers, locked and
unlocked, with playback active. It checks unchanged authority/pending state,
zero outbound engine commands, no immediate reappearance and a fresh prompt for
the next revision. Web guardrails check shared dismissal independently of the
consent actions; TypeScript/PWA builds check both localized callers.

No live production reservation is changed just to manufacture a test prompt.
Rendered/source checks are distinct from human acceptance of an actual live
handoff. Cafe runtime delivery remains blocked by the recorded smoke-launch
denial; source alignment does not establish a Cafe runtime update.
