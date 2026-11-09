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
- OS-session Quit is explicitly process-local. A remote-consumer frontend must
  close itself rather than forward this desktop request into shutting down its
  remote authority; ordinary explicit remote transport commands are unchanged.

The [MPRIS specification](https://specifications.freedesktop.org/mpris/latest/Player_Interface.html)
defines the property units, track-ID stale-seek guard and change/seek signals.
Unsupported shuffle/repeat modes are not invented or advertised.

## Verification and limits

- Full isolated local Windows library suite: **1,229 passed**, four existing
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
  transport requests and metadata/Stop clearing. The strengthened real-bus run
  [passed at the final packaged source checkpoint](https://github.com/ToghrolTP/pealayer/actions/runs/42572934634):
  104 platform checks and the actual private-bus case.
- Final focused native checks cover the introspected Seeked signal, nonzero
  restored starts, exact paused microsecond seeks and integer paused positions
  beyond f64's exact range. All five MPRIS checks passed locally; the strengthened
  private-bus case also passed in the linked Linux CI run.
- CI also caught two stale inherited guards: Web parity expected the former fixed
  Monitor volume placement, and the timeline pointer fixture constructed private
  Dock-viewer fields. The guards now check the current volume ownership and use
  the public viewer constructor. All 53 Web parity contracts and three local
  pointer regressions passed, without restyling or weakening production controls.
- Four real-decoder playback integration checks passed. Their inherited FPS
  assertion now checks typed metrics rather than a removed tooltip string; Play
  during an exact seek is correctly checked after decoder acknowledgement, not
  mistaken for synchronous completion. Production seek behavior is unchanged.
- The general CI matrix exposed those stale fixture references and an upstream
  UPX download HTTP 504. Downloads retain checksum verification and now use
  bounded transport retries. A focused green Linux run is not a claim that the
  whole cross-platform release matrix or Apple runtime has passed.

A phone's real KDE Connect view is a separate acceptance gate; a viewing-only
check has been requested. No board output or production playback was actuated.
macOS's generic KDE Connect fallback cannot display rich state; the retained
Souvlaki Apple backend also lacks a public rate/capability publishing API. Do not
call those richer Apple properties complete; [#269](https://github.com/ToghrolTP/pealayer/issues/269)
tracks their native implementation and actual Apple-runtime verification.
Windows KDE Connect itself does
not implement per-application volume, and its relative-seek extrapolation assumes
1x. Its artwork cache identity also depends on track tags, not artwork bytes.
Pealayer still publishes truthful metadata/rate/artwork rather than inventing
track changes to work around a consumer limitation.

## Package and handoff receipt

- Packaged source: `fa4213d013fb7ecf6e83116950cc99e3f34b09b4`.
- Executable SHA-256: `586336102e5298c6847af5d135694ef5537ed7f898be51157d29636610362c99`.
- Reused canonical staging: `%LOCALAPPDATA%\Programs\Pealayer\staging\preferences-organization`.
- Embedded `--build-info` independently reports the same clean source identity.
  The current `host-manifest.json`, not old loose staging diagnostics, is the
  package receipt. The selected host libmpv is unchanged, SHA-256
  `e56ce67cd00f06a59dc7ed5b97a49a9182a381554c755dc4570a52af1ef30e65`.
- Native release build, libmpv smoke, quick-action/native resources, Web bundle
  integrity and standalone downloader resource/headless checks passed. Packaging
  deliberately skipped repeating the already-run native/Web regression suites.

**Not installed:** fresh local authoring inspection still reports the running
Preferences helper as unknown, so its edits are preserved. Save/discard those
edits and close the helper, then recheck authoring before invoking the graceful
updater. Cafe deployment remains user-deferred while its route is offline.
Neither gate is waived by a successful package or Windows readback of the
already-running build. The general cross-platform release matrix remains a
separate pending gate; do not merge or publish a stable release on this receipt
alone. The paired-phone view and Apple work in #269 remain explicit.
