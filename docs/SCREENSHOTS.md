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

The script launches isolated English and Persian processes, fixes the window to
1280 by 800 pixels, captures the actual window, and terminates only the process
it created. It never stops or replaces an installed Pealayer instance.

## Linux

Run from the graphical session with `xdotool` and ImageMagick `import` present:

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
