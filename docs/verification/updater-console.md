# Updater console

The updater is a human-facing CLI by default, with a compact heading, semantic
PortableShell-inspired colors, factual byte progress, average transfer speed,
ETA when a total is known, and explicit verification/restart/acknowledgement.
It does not replace the terminal palette, clear the screen or hide the cursor.
The implementation reuses the existing Rust updater and its status contract.

## Usage

```powershell
& .\pealayer.exe --deploy-to http://127.0.0.1:8080
& .\pealayer.exe --deploy-to http://127.0.0.1:8080 2>&1 | Tee-Object update.log
& .\pealayer.exe --update-from https://example.invalid/pealayer.exe --sha256 <digest>
& .\pealayer.exe --update-status --json
```

Put output options **after** the update command. `--color auto` is the default:
colors require a terminal, no `NO_COLOR` environment variable, and a non-dumb
terminal. `--color always` explicitly forces ANSI colors; `--no-color` or
`--color never` disables them. Piped/Tee output is line-oriented without cursor
escapes by default. Transfer lines are throttled rather than printed per chunk.
An interactive color terminal updates its transfer line in place.

`--json` emits only the existing final status document to stdout, with no
headings/progress/ANSI. Failures emit a JSON error to stderr and retain exit
code 4. Automation that parsed the former default JSON must now request it.
The HTTP/RPC/WebSocket update API is unchanged.

Transfer speed is measured from observed byte deltas, not an invented network
rate. Verification and graceful restart have no percent or ETA. A transfer at
100% is not an installation receipt; success requires the existing peer-build
acknowledgement or an authoritative already-current result. A status query can
show an in-progress or idle state without calling it success.

Windows GUI-subsystem updater commands may attach to their parent console
when inherited handles are absent, while preserving valid redirected handles.
They never allocate another console, and normal GUI startup remains unchanged.
Terminal mode is restored before updater exit. Console write/flush failures
(including a closed pipe) cannot cancel a deployment.

Untrusted human-facing messages have terminal controls removed; displayed URL
credentials, query strings and fragments are redacted. Machine status remains
the existing API document; treat its source metadata as potentially private.

## Verification and delivery

Focused tests cover phase transitions, no stale verification percentage,
speed/ETA, unknown sizes, bounded redirected output, terminal color policy,
JSON purity, redaction, narrow output and broken pipes. The loopback HTTP
fixture exercises URL-update callbacks through downloading/verifying/restarting
and withholds completion until the expected build manifest appears. It does
not replace an executable or actuate hardware.

The full native library suite passed: **1,115 passed, one ignored**. Windows
packaging passed the production TypeScript/Web/PWA checks, native resources,
all seven quick-action icons, libmpv smoke and downloader smoke. Packaging
skipped duplicate native/Web regression runs; the full native suite ran
separately. No new browser interaction changes are claimed.

The packaged executable passed actual process/loopback checks for plain output,
forced semantic ANSI, JSON-only completion, and JSON failure on stderr with
exit code 4. This is captured program output, not a hand-written console mock.
Repeat those bounded, hardware-free checks with:

```powershell
.\scripts\verify-updater-console.ps1 -Executable <packaged-exe> -EvidenceDirectory <existing-private-directory>
```

The graceful updater then installed and restarted the canonical local app.
Its process-local authoring endpoint reported clean before replacement. Fresh
manifest readback matched source
`b75a27b68ae9de356a9bb219ac24ada47754b9ad` and executable SHA-256
`086c496fb10caf0b822e1f0a0320f95bbb67c0315eb6284fce2763fb6c7ae96a`.
The restarted process remained clean and retained CraftUI/System settings.
The actual upload/restart receipt completed in about six seconds, with no
stale verification percentage. Private console evidence is under the program
root's `verification` directory, not committed logs or user configuration.

Cafe deployment remains deferred at the user's request until its normal
connection returns; local fixtures and installation do not imply remote
installation or physical acceptance.
