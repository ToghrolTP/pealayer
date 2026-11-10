# Transport intent and production acceptance

The playback button describes the next action, derived from the authoritative
player state: **Play/Replay is green; Pause is amber**. Native Stop is red and
muted controls are amber. These use the existing shared light/dark palette, not
the configurable decorative accent. Neutral and unavailable controls keep the
theme's muted colors. All borders retain their width; Web hover and press no
longer translate the button or add a large accent glow.

Simple and NLE share the native transport styling helper. Web Player and
Timeline share `PlaybackButton`, including its accessible action label and
no-media/unknown-state disabling. Clicking does not invent a playing state:
the existing command is sent, and the next authoritative snapshot changes the
icon and color.

The NLE Stop button previously wrote directly to the local mpv instance and
overwrote local paused/time fields. A remote consumer could therefore stop its
preview without stopping the authority. It now uses the existing session-owned
`InteropCommand::Stop`, matching IPC, media keys and the Web command contract:
Stop closes the current media. It does not create a second remote-only path.
Recording punch-out is preserved at the session authority before closing media,
so remote requests do not lose the NLE button's previous capture behavior.

## Current acceptance limits

- The exact final PR head passed TypeScript/Vite/PWA verification and the full
  locked Rust, integration, shell, PCController, Web, and doc-test suite. These
  source gates remain distinct from installed playback acceptance.
- Cafe's fresh inventory found its controller service running but Pealayer
  stopped. Windows recorded a canonical-executable fail-fast crash, so final
  delivery must include real startup, Play, increasing media time, Pause,
  controller connectivity and a fresh crash-log check. Old static deployment
  manifests cannot establish the current executable's identity.
- Erfan-Gaming is reachable but KMPlayer is running. The user's no-launch
  constraint remains in force; neither staging nor file hashes certify actual
  Pealayer playback there. Coordinate a playback window with the user.
- The consolidation owner handles final-main packaging and graceful peer
  deployment using each destination's validated libmpv. No build occurs on Cafe.

Record exact installed source and live observations in the owning issue before
marking either machine delivered and usable.
