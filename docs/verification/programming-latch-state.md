# Programming latch is not a disconnected coordinator

## Cause

Legacy relay/PWM/motion/seek output paths flattened structured RPC errors and
marked the controller disconnected on every rejection. A valid programming
safety rejection was therefore treated like a failed socket. Startup capability
or E-STOP reassertion rejection could make the same mistake. Separately, the
controller cleared settings when intentionally releasing UART, losing the only
available programming-latch flag; lifecycle prose did not supply authoritative
replacement knowledge.

## Paired current contract

PCController now advertises a typed `programming` snapshot/event object with
active phase, pending restoration, intentional UART handoff, verified latch
knowledge/value, device, connection generation and monotonic revision/time.
Pealayer validates this object rather than guessing from error text or stages.
Low-rate snapshots bind it to `host_instance_id`; old revisions/generations and
foreign-source notifications cannot replace a newer state. Settings/programming
edges request an authoritative catalog, and a slow pre-handoff catalog baseline
cannot erase newer observations. Changed state invalidates the hardware cache.

Native status/menu/Hardware Monitor and Web status/monitor surfaces distinguish
Programming, Programming latch active, recovery pending, unavailable application
UART and a genuine PCController/peer transport loss. Raw board/controller
connection facts remain intact. An intentional handoff does not emit the
disconnect toast or trigger pause-on-disconnect; real coordinator loss still does.
Unknown historical latch data is never described as an active or cleared latch.

Output commands preserve typed domain-versus-wire errors. Rejection keeps the
authenticated stream and does not replay the mutation; actual wire failure still
discards it. Programming-disabled output controls remain unavailable while
channel presentation management stays editable. Safe Stop/E-STOP semantics remain;
the tray blocks ordinary board actions and E-STOP release during programming.

## Checks and remaining gates

Pure state tests cover latch/handoff/recovery/unknown/genuine-loss distinctions
and malformed knowledge. A loopback RPC fixture rejects relay/PWM/motion/release
commands and then accepts Ping on that same connection, checking no replay.
Memory-only engine fixtures verify snapshot source/order/cache invalidation and
wire-failure disposal. A real native status renderer fixture rejects a false
Disconnected caption. App notice tests distinguish handoff from coordinator loss.
Web state/availability regressions and the full Web/PWA pipeline passed.

No physical programming, reset, settings/latch mutation or output actuation was
used for verification. Installed paired runtime and live screenshot acceptance
remain pending behind the existing local/Cafe launch and clean-package gates.
The retained untracked generated Web outputs remain private; no denied cleanup
or alternate launcher was attempted. Deliver current PCController and Pealayer
packages together with each host's validated runtime, then verify the advertised
state on safe real media/hardware. Never clear the latch automatically.
