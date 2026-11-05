# Hardware Monitor: disconnected and empty states

## Regression and history

The RF relocation in `29f4167` changed its Monitor predicate from an advertised
RF capability to `!is_connected || supports_rf_transmit`, explicitly exposing it
while offline. That was the wrong scope for an offline editor: Hardware Monitor
must show only live, advertised hardware controls.

The bare disconnected label predates that change. Capability-driven Monitor work
introduced the text-only state; `1189cb5` shortened the connection guidance to
“Hardware Disconnected.” This audit did not establish a prior polished card to
restore exactly, so the current fix implements a palette-adapted replacement
rather than claiming to have recovered an unverified historical design.

An adjacent bug also allowed retained `board_connected=true` capabilities to
paint a green Connected board and channel cards after the coordinator transport
had disconnected. A retained catalog is not discarded, but never counts as a
live connection in Monitor presentation.

## Behavior

- Native and Web require both live controller transport and board connection for
  board/channel/telemetry cards. RF additionally requires the advertised RF
  capability and available outputs; programming/latch state cannot expose it.
- Disconnected, intentionally connecting, missing capability catalog, missing
  board and programming/recovery have distinct headings and factual details.
- A bordered, rounded empty-state card uses the existing live palette, readable
  wrapping text and compact actions instead of an RF card and a weak one-liner.
- Connect is offered only for a configured endpoint without a pending attempt.
  Refresh is offered for a reachable coordinator. Preferences stays accessible.
  Rendering/opening Monitor never connects, refreshes, claims authority, sends RF
  or actuates an output. These operations require an explicit user action.
- `pealayer.hardware.connection` / `set_hardware_connection` use the existing
  typed control dispatch and `ReconfigureEndpoint` engine message, including
  authority forwarding. The command does not rewrite endpoints or auto-connect
  preferences. The Web Preferences button navigates within Web, not native UI.
- Connected-board cards retain their existing programming/latch warning. Global
  RF authoring and saved remotes are not deleted by this presentation correction.

## Verification and deployment

Focused tests cover stale transport/board combinations, missing/unsupported RF,
actual native Monitor paint, narrow light/dark card geometry, existing vertical
scrolling, Web state distinctions and explicit connection RPC parsing. Browser
fixtures use the actual HardwareTab at phone/tablet/desktop widths with a command
recorder, never a real board. See the PR checkpoint for completed results.

Completed locally: 62 focused native hardware regressions, TypeScript and the
Web prebuild suite; actual browser interactions across all four unavailable
states at 390/768/1440 pixels in light and dark themes. The native paint tests
also check the real Monitor branch, not only an isolated visibility predicate.

Runtime and physical acceptance remain separate. Before packaging or replacing
an app, re-query its local authoring endpoint; use the host-compatible graceful
updater. A closed API/SSH route is a deployment blocker, not successful delivery.

The unfinished external-asset importer is preserved separately on
`feat/external-asset-drops` and is not part of this fix or its package.
