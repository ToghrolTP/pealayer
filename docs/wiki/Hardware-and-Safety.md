# Hardware and safety

## Connection states

**Connect** enters a pending state until a transport succeeds or reports a real
error. A reachable coordinator without a board is degraded, not healthy. The
status bar becomes healthy only when the selected transport and required board
state are available.

The hardware monitor shows only live capabilities and names supplied by
PCController. Relay overrides send real commands and distinguish requested state
from board-reported state.

## E-STOP

E-STOP is always available in the desktop chrome. Activating it pauses playback
and disables output. The control uses a code-native vector stop mark, remains
keyboard reachable, and reports the active state in text as well as color.

## Ownership

PCController normally owns the UART. If diagnostic direct serial was explicitly
enabled, Pealayer checks for the coordinator continuously, sends safe-off, and
tears down its serial transport when PCController appears. The diagnostic
override must never be used to create two serial owners.

Board, relay, PWM, sensor, effect, and macro names are external data. They are
displayed verbatim and are not translated or replaced with built-in samples.
