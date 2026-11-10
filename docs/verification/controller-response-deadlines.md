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

## Package and delivery state, 13 October 2026

Clean source `3dc99788972519f1f81050a79e93cd6a4231ee06` is published in
[PR #176](https://github.com/ToghrolTP/pealayer/pull/176). The shared-cache release
compile completed in 4m38s, followed by the 3.98-second downloader compile and
Web/PWA/resource/package checks; regressions and CI were skipped. The executable
SHA-256 is `a49d602f6195c712f3ee68f7e32977da0b840c680c4e7c6b3a855020a6292e1a`.

The packager fell back from the explicitly requested retained-runtime directory,
which lacks a development import library, to David's complete development package.
That workstation DLL is **not** approved for Cafe delivery. A separate low-write
stage `staging/controller-response-deadlines-cafe` hardlinks the candidate and
Cafe's exact retained runtime, SHA-256
`872827614ed0adfca11e68def5273bcfcaea6acf38bbf1950c35980b59f43a5f`.
The maintained smoke/build-info probes passed there with an isolated configuration;
the executable still reports the clean source identity. No destination runtime
DLL is replaced and no production-host compilation or GUI launch was performed.

PCController's paired [PR #659](https://github.com/atomicdeploy/PCController/pull/659)
is already delivered through Cafe's owned updater, with matching installed hash,
unchanged configuration and six live status/PWM NDJSON reads completing in
8.4–23.5 ms. Playback stayed paused, hardware connected/no errors, with desired,
prepared and clock ACK revision 88 matching. Its protected merge checks remain
blocked with normal auto-merge enabled. No CI or policy override was used.

Pealayer replacement remains held until the user confirms Cafe drafts are saved.
There is no claim that the new native candidate is installed, that a physical
output was exercised, or that intermittent network/ACK loss was reproduced.
