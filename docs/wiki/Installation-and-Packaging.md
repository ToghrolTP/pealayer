# Installation and packaging

## Windows

Install Rust and place the libmpv runtime plus the import library for the active
Rust host in `%ProgramFiles%\MPV` or set `LIBMPV_DIR`.

```powershell
pwsh -File scripts/run-windows.ps1 -BuildOnly
pwsh -File scripts/package-windows.ps1
```

MSVC requires `mpv.lib`; GNU requires `libmpv.dll.a` or `libmpv.a`. The package
step validates Win32 identity resources, the bundled runtime, executable smoke,
and package hashes before publishing to the canonical `bin` directory.

## Linux

Install the Rust toolchain, libmpv development package, and the platform GUI
dependencies required by `eframe`, then run:

```bash
cargo build --release --locked
cargo test --all-targets --locked
```

Use CI artifacts when reproducing a reviewed build. Always record the commit and
artifact hash used for deployment or screenshots.

## Portable configuration

Place `portable.flag` or `pealayer.json` beside the executable to store settings
under the local `config` directory instead of the user profile.
