# Native now-playing discovery and KDE Connect

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

## What the consumer actually reads

| Platform | KDE Connect implementation | Pealayer publication |
| --- | --- | --- |
| Windows | [Global SMTC session manager](https://github.com/KDE/kdeconnect-kde/blob/master/plugins/mpriscontrol/mpriscontrolplugin-win.cpp); sessions and metadata/playback/timeline change events | One HWND-owned SMTC session, title/tags, playback status, rate, timeline bounds/position, contextual transport availability and in-memory artwork |
| Linux/BSD | [MPRIS over the user D-Bus](https://github.com/KDE/kdeconnect-kde/blob/master/plugins/mpriscontrol/mpriscontrolplugin.cpp); ownership changes, PropertiesChanged and Seeked | One application-owned MPRIS object per process with real capabilities/rate/volume, stable opaque track identity, metadata and readable local PNG artwork |
| macOS | [Generic media-key fallback](https://github.com/KDE/kdeconnect-kde/blob/master/plugins/mpriscontrol/mpriscontrolplugin-macos.mm) in the inspected KDE Connect source; no rich per-player metadata discovery | Existing Apple Now Playing publication through Souvlaki. Pealayer cannot make this KDE Connect plugin read properties it does not query |

These are native OS interfaces, not a custom KDE Connect network protocol or
another set of global keyboard hooks. The existing master/background policies
and dispatch-time capability/scope checks remain authoritative. The master
“Respond to keyboard media controls” setting must be enabled to publish a native
session; disabling it intentionally unregisters that session.

## Corrected gaps

The former Souvlaki Linux projection always returned rate 1.0, rate limits 1.0,
all transport capabilities true, one root track ID and a non-extrapolated stored
position. It ignored the ID on SetPosition. Its API could not publish the actual
application capabilities/rate. The Linux owner now uses the existing pinned
zbus dependency directly rather than registering alongside that backend.

- Actual rate and rate limits, with a monotonic position anchor and exact paused
  position; duration/position use signed, saturating microseconds.
- Loaded/seekable/playlist-based capabilities. MPRIS ability flags are not the
  single contextual Windows button state: CanPause remains true when paused so
  PlayPause can resume.
- Opaque per-media object paths derived from identity, with no paths, credentials
  or query text in the ID. Stale-ID and invalid SetPosition requests do nothing.
- Metadata, rate, volume and capability changes emit PropertiesChanged. Seeking
  discontinuities emit Seeked; ordinary advancing position does not flood a
  Position property signal. Idle/Stop clears metadata and control availability.
- MPRIS volume reflects mute as zero. A permitted positive remote volume request
  unmutes through the existing command queue, with the same scope recheck; a
  blocked request does not unmute or change volume. Requested property values
  are not optimistically installed before the player reports them.
- Embedded/custom artwork is converted to a bounded PNG and content-addressed
  in the existing application cache. Equal bytes are never rewritten. No media
  cover is fetched remotely and no credentials/source URL are published.
- A worker owns D-Bus I/O, using a one-slot coalescing snapshot mailbox. Startup
  and publication failures retry with bounded backoff; idle work blocks on events.
  Drop/failure invalidates callbacks. There is no periodic D-Bus probe on the UI
  thread, and no second Windows or Linux session registration.

The [MPRIS specification](https://specifications.freedesktop.org/mpris/latest/Player_Interface.html)
defines the property units, track-ID stale-seek guard and change/seek signals.
Unsupported shuffle/repeat modes are not invented or advertised.

## Verification and limits

- Full isolated local Windows library suite: **1,227 passed**, four existing
  ignored checks. It includes metadata/identity, artwork reuse, MPRIS projection,
  exact position/rate, stale seeks, command acknowledgement and scope tests.
- TypeScript and the executable Web media-session projection checks passed.
- `scripts/inspect-os-media-session-windows.ps1` queries the exact Windows session
  API used by KDE Connect and compares it to Pealayer's local API. On the running
  native app, title, paused state, rate, duration and exact position matched;
  artwork and seek availability were present. The probe is read-only, bounded,
  and prints comparison flags rather than private titles/paths/artwork bytes.
- A dedicated Linux workflow runs the adapter against an explicitly private
  dbus-run-session, checking actual discovery/properties, event delivery,
  transport requests and metadata/Stop clearing. The first real-bus run passed:
  [102 platform checks and the live private-bus case](https://github.com/ToghrolTP/pealayer/actions/runs/42570090557).
- Final focused native checks cover the introspected Seeked signal, nonzero
  restored starts, exact paused microsecond seeks and integer paused positions
  beyond f64's exact range. All five MPRIS checks passed locally; the strengthened
  private-bus case is rerun in CI at the final source checkpoint.
- CI also caught two stale inherited guards: Web parity expected the former fixed
  Monitor volume placement, and the timeline pointer fixture constructed private
  Dock-viewer fields. The guards now check the current volume ownership and use
  the public viewer constructor. All 53 Web parity contracts and three local
  pointer regressions passed, without restyling or weakening production controls.

A phone's real KDE Connect view is a separate acceptance gate; a viewing-only
check has been requested. No board output or production playback was actuated.
macOS's generic KDE Connect fallback cannot display rich state; the retained
Souvlaki Apple backend also lacks a public rate/capability publishing API. Do not
call those richer Apple properties complete. Windows KDE Connect itself does
not implement per-application volume, and its relative-seek extrapolation assumes
1x; Pealayer still publishes the truthful playback rate rather than falsifying it.

Linux private-bus CI, package/deployment and the current runtime receipt follow
verification. Cafe deployment remains user-deferred while its route is offline.
