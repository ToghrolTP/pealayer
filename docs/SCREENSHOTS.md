# Screenshot acceptance workflow

Screenshots are release evidence, not production data. The update scripts start
the exact executable supplied by the caller, select only application-owned
appearance settings, and capture the real runtime state. They do not add sample
serial devices, relays, effects, telemetry, media, or PCController capabilities.

## Windows

Run in the signed-in interactive desktop session after building or downloading
the exact PR artifact:

```powershell
pwsh -File scripts/update-screenshots-windows.ps1 `
  -Executable target/release/pealayer.exe `
  -Theme dark -Locale en,fa
```

Capture the same Preferences editor in its native-window host by selecting the
dedicated surface (the default remains the main application window):

```powershell
pwsh -File scripts/update-screenshots-windows.ps1 `
  -Executable target/release/pealayer.exe `
  -Surface preferences -Theme dark -Locale en
```

For a custom build, pass `-Branding path\to\brand.json` (the shared
living `application-brand` document) or `-AppName`. With no override, the script
reads `ProductName` from the executable's Win32 version resource.

The script launches isolated English and Persian processes, captures each
app-owned native window at its initialized dimensions, and terminates only the
process it created. Each launch uses an isolated settings file and dedicated
unified control port, so it never stops, forwards into, or replaces
an installed Pealayer instance. The updater tries
`PrintWindow(PW_RENDERFULLCONTENT)` first, detects the blank frame produced by
some OpenGL drivers, and then tries the target window's composed device context
before using screen pixels as a final fallback. It enables per-monitor DPI
awareness and derives the fallback frame from the native client bounds. The
screen-pixel fallback requires the target window to remain
foreground and unobscured; the per-capture manifest records which method was
used and the session type.

## Linux

Run from an X11 graphical session with `xdotool`, ImageMagick `import`,
`sha256sum`, and Git present. On Ubuntu the first two tools are normally
provided by the `xdotool` and `imagemagick` packages. Native Wayland sessions
do not permit the X11 window discovery/capture flow; use an Xorg login session
or an explicitly configured XWayland test session.

Validate the environment without creating an output directory or launching the
application:

```bash
scripts/update-screenshots-linux.sh --check "$PWD/target/release/pealayer"
```

Then capture from the same signed-in graphical session:

```bash
scripts/update-screenshots-linux.sh "$PWD/target/release/pealayer"
```

Both scripts write PNGs under `docs/screenshots/` and a manifest containing the
source commit, executable SHA-256, locale, direction, theme, dimensions, and
image SHA-256. Commit the images and manifest together. A screenshot is stale
when the manifest commit or executable hash no longer matches the artifact under
review.

The Windows updater enumerates that launched PID's visible top-level windows,
ignores tiny renderer helper HWNDs, verifies ownership again, and crops to the
largest app window's bounds. It rejects captures that
remain blank after the OpenGL screen-pixel fallback. A firewall prompt,
terminal, desktop, or unrelated window in a capture is a failed acceptance run
and must not be committed.

Additional connected/error/E-STOP evidence must be captured from real state:
connect the intended PCController/board, perform the user action, and rerun the
same capture tool. Do not add production code paths that fabricate those states.
