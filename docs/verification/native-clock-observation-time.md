# Native position observation time

The independent libmpv observer previously read `time-pos` before several other
synchronous property reads and coordinator locks. It then compared that position
to a projection at application time, and marked the old position fresh at that
later time. A delayed property bundle could therefore create a false clock
discontinuity or conceal observation age.

Read the metadata and coordinator state before position. Bracket the native
position read and retain its conservative start timestamp for the drift
comparison, decoder anchor and observation freshness. A delayed application is
not a new observation. Failed position reads do not refresh observation time.
The existing 250 ms drift/freshness and 50 ms execution limits remain unchanged;
real stalls or stale reads still fail closed. Existing seek, pause, rate and
media-boundary fences remain in force.

`/api/process/status` exposes bounded numeric `playback_clock.observation_timing`
diagnostics: native read duration, age at application, maxima, signed drift at
read time and the first discontinuity's numeric context. A deliberate rebase
clears that fault context. No media paths, frame images or per-poll event logs
are added. The diagnostics do not establish physical chair movement.

Regressions model a correct native position applied 310 ms late: it must match
the projection at read time, keep that original anchor, and remain stale after
250 ms. Separate cases retain both directions of the 250 ms jump boundary,
real unchanged-position stalls, and playback-rate scaling.

Live acceptance is pending. One chair-only replay on the preceding native build
stopped on the discontinuity guard before its later motion cue; its controller
priority lease correctly transitioned Normal → High → Normal. That receipt
does not contain the native read/apply bracket, so it does not establish this
bug as the sole cause. Deploy the exact hosted candidate with the destination's
validated libmpv, inspect the new numeric context if it fails, and complete
unprofiled chair-only short and continuous real-media playback. Do not loosen
guards, fabricate a seek, skip cues or claim a pass from source tests alone.
