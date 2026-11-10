# Native seek completion at a nonzero first frame

Owner: [#219](https://github.com/ToghrolTP/pealayer/issues/219).
Related playback work: [#217](https://github.com/ToghrolTP/pealayer/pull/217).

A verified 25 fps file displays its first frame at 80 ms. An exact zero seek
completed in libmpv, but the UI required a position within one frame plus 2 ms
of zero and retained `seek_pending` indefinitely. The hardware clock correctly
reported the decoded 80 ms position. In an isolated headless probe using the
destination runtime, `demuxer-start-time` was zero, so clamping against that
property would not identify the first displayable frame.

Only native exact-zero commits now use a dedicated libmpv completion client.
The worker queues a tagged asynchronous command and remains available for
coalescing. The UI consumes the matching command reply, a subsequent seek event
and playback restart from that same ordered client. It also requires the worker
dispatch acknowledgement, `seeking=false` and a finite nonnegative decoder
position. A stale preview restart, another request's reply or an old media epoch
cannot release this fence. Raw typed event headers retain error userdata that
the high-level wrapper otherwise discards on failed command replies.

The request's zero remains available as `settled_seek_target`; playback time,
the engine notification and the independent decoder clock retain the observed
first-frame position. This does not shift cues or advertise an invented zero
clock. Ordinary nonzero commits keep the existing frame-position guard and
paused sub-frame authoring behavior. Peer/external seek routes are unchanged.

Media replacement, cancellation, queue loss, matching command failure and a new
seek revoke the old completion. Request tokens are process-wide to avoid reuse
against test/window clients that outlive a controller. Pause also revokes implicit
resume intent, so a delayed completion cannot restart playback after Pause.
Already submitted decoder work may finish, but cannot settle another request.
Native asynchronous submission and revoke/new-request share a tiny queue gate;
ordinary synchronous backends never hold it. Cancellation therefore cannot
return and then permit an obsolete native zero command to be submitted. Before
the next zero command, old completion-client boundaries are drained after revoke
and before the new epoch/request is queued.

Pure regressions cover event ordering, stale IDs/epochs, matching errors,
replacement/overflow, nonzero frame tolerance and truthful first-frame position.
Controller tests retain coalescing coverage and explicit queue rejection.
Deterministic dispatch barriers exercise cancellation before native submission;
an isolated synthetic Y4M/libmpv fixture verifies the first zero seek after
draining the file-load boundary. Test decoder clients live for the process so
Windows asynchronous teardown cannot race another fixture.
Rust syntax parsing and build-policy checks are local; native compilation/tests
run on hosted CI. Keep the installed c48 runtime fixed during the companion
controller comparison; deployment and real-media zero-seek validation remain
separate gates. The 50 ms cue and 250 ms freshness limits are unchanged.
