# Controller response deadlines and PWM confirmation

The reported status request waited 1500.8 ms behind an addressable-LED request
registered for 1550.4 ms. PCController previously allowed that LED request to
hold the only serial slot for two seconds, while status refresh expires at
1.5 seconds. LED work now queues below ordinary status/control work, with a
750 ms response bound once admitted. It never preempts a command already on the
wire, and shorter prepared-timeline deadlines remain unchanged. Repeated missing
strip ACKs stop the stream after three failed frames rather than looping forever;
no physical clear or optimistic output confirmation is fabricated.

PWM mutation and all-channel readback previously inherited the 15-second RPC
budget, but Pealayer's ordinary command socket waits three seconds. PCController
now gives PWM write/readback one shared two-second budget across RPC and HTTP.
A received mutation ACK followed by a failed readback is a typed
`confirmation_unavailable` error, not a claim that the write was rejected or the
coordinator disconnected. The write is never automatically replayed. A failed
mutation does not proceed to readback. Locks, ownership and E-STOP remain enforced.

Pealayer's TCP preparation temporarily allowed a 15-second read timeout, but
early write/read/decode/EOF failures bypassed restoration. All paths now restore
the original short socket deadline. Unrelated/late replies also consume the
same absolute response budget instead of restarting it. Transport failures
still discard the socket and pause unsafe playback; server-side rejection or
readback failure does not become a false TCP disconnect.

Added pure/mock regressions for queue priority, missing LED ACKs, shared PWM
deadlines, ACK/readback distinction, and restoration after preparation failures.
Tests and CI remain skipped at the user's request. Builds and live read-only
verification are recorded separately below; no PWM, relay, motion or tone output
is sent merely to reproduce an error.
