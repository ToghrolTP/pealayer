# Window/media control: measured acceptance

Contract and usage: [window and media automation](../WINDOW-AND-MEDIA-CONTROL.md).

## Source and package

- Native source: `c75ca0e703e721b1f32988759a919afcbd032e76`.
- Verified executable SHA-256:
  `f76a025b9f9689097068deda8061ddae7aaee29a7a66ac62a6f51bdb646a9b10`.
- Full final-source native library run: **1,141 passed, zero failed, one ignored**.
- Windows release package passed Web/PWA build guardrails, Windows resources,
  all seven taskbar quick-action icon checks, libmpv smoke and companion downloader
  smoke. Packaging skipped duplicate test runs and UPX; the full library result
  above is separate. Existing compiler warnings are not claimed fixed.

## Real native process, not dispatcher mocks

The isolated fixture used its own config, identity and loopback TCP/UDP port
18094. It generated a 12-second 320×180/24 FPS silent video with two chapter
markers. No production media, hardware connection, global media keys or global
hardware bindings were used. Each transport opened that actual file, queried
cached media details and exercised window and playback state.

| Transport | Placement and window states | Playback/pause/seek while minimized | Metadata, chapters, frame step, volume/mute/rate, Stop |
| --- | --- | --- | --- |
| HTTP typed JSON | Passed | Passed | Passed |
| HTTP JSON-RPC | Passed | Passed | Passed |
| HTTP IPC | Passed | Passed | Passed |
| Raw TCP NDJSON | Passed | Passed | Passed |
| Windows named pipe | Passed | Passed | Passed |
| WM_COPYDATA with asynchronous JSON-RPC replies | Passed | Passed | Passed |
| WebSocket | Passed | Passed | Passed |
| Existing binary UDP RPC/client | Passed | Passed | Passed |

Placement assertions checked actual inner size and outer position, not only
accepted requests. Normal/maximized/fullscreen transitions, minimized flags and
preserved restore bounds were polled. Actual playback position advanced while
minimized, then paused and settled at the requested seek. Playback commands did
not restore the minimized window. Chapter count/current index, 24 FPS, loaded
metadata, volume/mute/rate and Stop/unload were independently read back. JSON-RPC
correlation was checked, including bidirectional Windows-message responses.

The fixture closed normally after its own fresh authoring report was clean.
Private logs, generated media, config and results JSON remain under the program
root's `verification/control-contract-pass1`; they are not source artifacts.
This is real Windows native/runtime acceptance, not physical hardware, audio
listening, every media format, every OS or a sustained-load benchmark.

## Running comparison and deployment boundary

The current egui Dock comparison was clean and closed through its local API,
then reopened against the verified staged executable with the same independent
profile on **18093**. Its native authoring API reports the exact source above
and clean state. `POST /api/window` maximized it; `GET /api/window` confirmed the
actual maximized, non-minimized, non-fullscreen state and no placement error.
The comparison launcher now accepts an explicit `-CurrentExecutable` so a
verified candidate can be inspected without overwriting the regular installation.

The [October 10 historical package](october10-ui-comparison.md) remains running
and retained. Its executable hash is unchanged. Original published comparison
screenshots still identify the earlier current source used for those captures;
they were not silently relabelled as captures of this later API build.

**Regular installation is not yet updated:** its fresh authoring report says
`preferences_helper: unknown`, `safe_to_replace_native: false`. The user has been
asked to save/discard and close that Preferences window. No draft was discarded,
no healthy process was killed and no alternate shell or policy bypass was used.
After acknowledgement, recheck authoring, use the graceful peer updater for
8080, and verify the native source/hash and window query on the restarted app.
Cafe deployment remains explicitly user-deferred while its connection is absent;
do not call this local fixture or staged comparison a Cafe delivery.
