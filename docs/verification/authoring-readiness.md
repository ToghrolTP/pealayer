# Process-local draft inspection

`GET /api/process/authoring` requests a fresh inspection on the native UI thread.
It returns the process ID/build identity, per-domain `clean`, `dirty`, `busy` or
`unknown`, aggregate state and `safe_to_replace_native`. No draft text, JSON,
settings value, media path or RF assignment leaves the process. This endpoint is
excluded from peer forwarding even when the window controls a remote authority.

The existing saved baselines are reused for Effects Designer, RF, Preferences,
cue dialogs and board settings. New nonempty effects are unsaved even when they
match an initial UI baseline. Pending publication, RF/board operations, import
and recording are busy, not editor drafts. Inline effect identity, group/channel
property/hardware-binding editors and a separate Preferences helper have no
complete exposed saved baseline: while open they report unknown, not false clean.
External browser/mobile-local drafts are explicitly outside this native-process
scope. Replacing a host must not be presented as discarding client-local drafts.

Requests are correlated, bounded to eight queued inspections and a three-second
deadline. The one query wakes the UI; no repaint timer, network polling, hardware
command, configuration write, save/discard or quit is added. An unresponsive UI
returns HTTP 503 with unknown and false readiness. A late reply cannot satisfy
another request. The internal GUI command cannot be deserialized by clients.

Deployment tooling/agents must inspect this endpoint before replacement instead
of asking a generic draft question. Clean means the inspected native domains are
safe; dirty/unknown needs an existing save/discard decision, and busy needs its
operation to finish. A user's explicit discard permission may authorize that
deployment, but is not permission to interrupt hardware programming. This pass
has explicit discard permission for Cafe. Missing inspection in the previously
installed build is not clean; its update is the authorized bootstrap, not a
permanent old-schema fallback in application code.

Focused isolated tests cover real new drafts/pending saves, non-disclosing
responses, correlation, timeout and clean/dirty/busy/unknown discrimination.
Native query delivery and installed source/digests are verified after deployment.
