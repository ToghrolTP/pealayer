# Bundled application icons

`classic.svg`, `classic.png` and `classic.ico` preserve the previous public
Pealayer artwork exactly as it appeared before commit
`2931171c6ff4e9028eff32225f0d33d566acb1b2` replaced it. The SVG is the editable
master, PNG serves cross-platform/Web imagery, and ICO preserves the original
Windows multi-resolution pack. These are application assets, not user artwork.

The current artwork remains at `assets/pealayer-icon.svg`,
`assets/pealayer-icon.png` and `assets/icon.ico`.

Choose **Current** or **Classic (previous)** under Preferences → Appearance →
Interface → Application icon. `app_icon_preset` is stored in the shared config;
egui and Web preferences use the same Rust control definition. Selection does
not extract or duplicate files on the user's disk. Explicit application or
playback-state custom icons and deployment/environment branding keep priority.

The running window/taskbar, tray, Web branding, favicon and PWA artwork use the
resolved selection. The executable's embedded resource and pre-existing pinned
shortcuts remain build/deployment assets; changing a runtime preference does
not rewrite an executable or refresh Explorer's pinned-shortcut cache.
