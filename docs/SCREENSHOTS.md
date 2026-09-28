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

For a custom build, pass `-Branding path\to\brand.json` (the shared
living `application-brand` document) or `-AppName`. With no override, the script
reads `ProductName` from the executable's Win32 version resource.

The script launches isolated English and Persian processes, fixes the window to
1280 by 800 pixels, captures the actual window, and terminates only the process
it created. Each launch uses an isolated settings file and dedicated HTTP,
WebSocket, and loopback IPC ports, so it never stops, forwards into, or replaces
an installed Pealayer instance. `CopyFromScreen` requires the target window to
remain unobscured in the signed-in interactive desktop; the manifest records
that capture method and session type.

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

Additional connected/error/E-STOP evidence must be captured from real state:
connect the intended PCController/board, perform the user action, and rerun the
same capture tool. Do not add production code paths that fabricate those states.
