# OS media-session completion

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).
The existing [master/background policy](background-media-controls.md) remains
independent of ordinary keyboard shortcuts and system volume/mute keys.

## Completed current paths

- Cached media inspection supplies actual title, artist and album tags to native
  and Web sessions. Unknown artist/album are omitted, not invented from branding.
  Filename fallback excludes remote credentials/query strings. File replacement
  clears cached metadata immediately; metadata/playlist changes use mpv events,
  not per-frame property queries.
- One Windows HWND-owned SMTC adapter replaces the incomplete Windows Souvlaki
  adapter; they do not register concurrently. Transport, absolute position and
  playback-rate requests enter the existing shared command queue. Every callback
  has its own removal token and the registration's active guard. Partial setup
  failure also removes already-added callbacks. Background/master/capability
  permission is rechecked at dispatch, including same-frame config changes.
- Windows advertises contextual Play/Pause/Stop/Next/Previous/seek buttons, actual
  playback rate, finite timeline bounds, Music/Video metadata and stopped/EOF
  state. Idle disables the SMTC surface without adding another key listener.
  Metadata and old artwork are cleared on replacement/Stop/disable/drop.
- Windows artwork uses a cached, bounded, alpha-preserving PNG of the configured
  playback/app icon. The WinRT stream is retained and reused in memory: no artwork
  temporary file, per-frame disk write or remote-cover fetch. This is branding
  artwork, not an assertion that embedded album covers have been extracted.
  Other native adapters use configured local artwork URLs when available.
- Playing position publication is bounded to measured 250 ms advances; backward
  jumps, pause/stop and metadata replacement are immediate. Paused exact positions
  remain exact. Failed publication does not advance the success cache; native
  registration/publication failures retry with a five-second backoff.
- Browser metadata, handler registration and position publication are separate.
  Playback-time updates no longer reinstall every action callback. Current
  capabilities and preferences are revalidated by retained callbacks. Missing,
  disabled, nonseekable or live media clears old finite position state explicitly.
  Browser seeks use the configured interval or exact validated OS-supplied offset.

The status API's `media_session`, `media_session_capabilities` and
`quick_seek_seconds` fields are shared current-contract data. Native peer clients
derive metadata from the authority's existing MediaView; no preview decoder is
promoted into a second player authority.

Windows uses the documented [manual SMTC control](https://learn.microsoft.com/en-us/windows/apps/develop/media-playback/system-media-transport-controls)
and [display updater](https://learn.microsoft.com/en-us/uwp/api/windows.media.systemmediatransportcontrolsdisplayupdater)
paths. OS selection of the active session remains OS-owned. This does not hijack
another application's media buttons or system volume.

## Explicit limits and acceptance

Repeat/shuffle are not advertised: the player has no corresponding shared modes.
Embedded-cover extraction is not implemented. Linux/macOS retain Souvlaki's
backend support; its API does not expose native rate/capability publication on
those platforms. Shared event gating, metadata and bounded playback publication
still apply. Browser action/position availability is browser-dependent.

Pure/memory-only native tests cover metadata, playlist boundaries, callback
lifetime/rate validation, scope rechecks, signed WinRT positions and overflow,
bounded alpha artwork and publication cadence. Web tests execute projections and
validate integration, including stale-capability guards and finite-to-live clearing.
No test registers a live OS media session, opens media or actuates the board.

The full isolated local Windows MSVC suite passed **875 tests**, with zero
failures and one pre-existing ignored test. The additional WinRT stream test
round-trips artwork bytes in memory without an HWND or session registration.
The full Web TypeScript/behavior/responsive/parity/production/PWA pipeline passed,
including the new executable session projection tests and 51 parity contracts.

Local/Cafe installation remains blocked by preserved generated Web files and
existing launch restrictions. The known Windows GNU native access violation is
an independent platform acceptance gate. Source/tests are not physical media-key,
SMTC-flyout, lock-screen, thumbnail or Linux/macOS runtime acceptance. After an
approved update, test file replacement/Stop, real tags/artwork, finite/live seek,
speed, EOF and playlist boundaries, both focus policies, disable/re-enable and
another player to confirm routing and no double dispatch.
