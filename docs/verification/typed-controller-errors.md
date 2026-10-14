# Typed controller error presentation

Owner: [#187](https://github.com/ToghrolTP/pealayer/issues/187).

The runtime owner reproduced long nested Windows socket-timeout paragraphs in
RF alerts and connection-loss notifications. ControllerRpcError existed, but TCP
I/O was flattened to message strings and selected-coordinator validation used
the string-only call path. The cause could not reach native/Web presentation.

TCP I/O now derives timeout/connection_unavailable from std::io::ErrorKind, not
English text, numeric text fragments or build versions. Explicit expired response
deadlines are typed timeout too. Validation retains metadata through the fresh
connection, ping, registration and snapshot. The identity admission check remains
mandatory; transport failure is still distinct from a structured server rejection.
The existing connect/read budgets (300/500 ms selected probe, 2/3 seconds normal
connection, special preparation deadline) and timeout restoration are unchanged.
There is no automatic retry of an uncertain mutation, output or binding Save.
This classification slice covers the observed TCP path; no prose-based classifier
is added to the UDP/in-process adapters that own their separate error paths.

Native/Web RF headlines use structured kind/transport_failed, with translated
connection/current-state guidance and expandable original message plus metadata.
Unknown server errors preserve their actual messages, not a fabricated timeout.
Actual connection-loss notifications derive their concise message from the
existing connection/board-loss state, not parsing the stored diagnostic. Shared
native/Web toasts carry optional Details, bounded to 8192 characters. Longer
diagnostics disclose truncation; the original remains in the existing private
engine error state. Toast lifetime, identity, dismiss and offline non-replay stay
unchanged. No diagnostic is published to GitHub or written to disk by rendering.

## Verification and delivery

Hosted fixtures cover multilingual I/O causes, misleading error prose, a silent
isolated loopback ping timeout, typed-only RF captions, bounded shared Details and
native/Web source wiring. Tests touching configuration inherit the isolated
PEALAYER_CONFIG_FILE from CI. No local compilation/tests or production socket,
runtime, output, policy or RF writes are performed by this source lane.

This branch integrates normally merged safety192 with a merge commit preserving
both changes. Require the exact resulting hosted gates before merging/delivering.
Runtime owner alone deploys the verified package
against the validated destination DLL and checks real UI/error disclosure, health,
RPC, PCController connectivity, real media and crash events. This does not prove
that the underlying intermittent controller timeout or native capture problem is
fixed; those remain independent runtime/source-owner investigations.
