# FilmCraft integrated review captures

The eight PNGs in [the capture directory](../screenshots/pr248-integrated) were
captured from the clean Windows x64 build of PR #248 head
b55134ee53da55d903b16e4ccdc5e532b49690f6. The [successful branch build](https://github.com/ToghrolTP/pealayer/actions/runs/42779132839)
supplied the installable artifact. The downloaded executable passed the local
smoke test and reported the expected clean commit. Its SHA-256 is
7c6e4f3a6efa3c705cda1464736ca3d52dcef586f07eb0ffdceb8806a737bdf8.

All images were individually inspected. They show the actual native application
on the same synthetic 5.250-second frame, with settled paused playback,
Studio dark, Compact density, and a 150% display scale. The manifests record
both logical viewports and each PNG's verified SHA-256. Their original
machine-generated inspection flags are retained. This receipt records the
subsequent inspection.

| Workspace renderer | Compact monitor and Standard timeline | FilmCraft monitor and timeline |
| --- | --- | --- |
| CraftUI | [Wide](../screenshots/pr248-integrated/integrated-compact-craft_ui-1200.png) · [Narrow](../screenshots/pr248-integrated/integrated-compact-craft_ui-720.png) · [Manifest](../screenshots/pr248-integrated/integrated-compact-craft_ui-manifest.json) | [Wide](../screenshots/pr248-integrated/integrated-film-craft-craft_ui-1200.png) · [Narrow](../screenshots/pr248-integrated/integrated-film-craft-craft_ui-720.png) · [Manifest](../screenshots/pr248-integrated/integrated-film-craft-craft_ui-manifest.json) |
| egui Dock | [Wide](../screenshots/pr248-integrated/integrated-compact-egui_dock-1200.png) · [Narrow](../screenshots/pr248-integrated/integrated-compact-egui_dock-720.png) · [Manifest](../screenshots/pr248-integrated/integrated-compact-egui_dock-manifest.json) | [Wide](../screenshots/pr248-integrated/integrated-film-craft-egui_dock-1200.png) · [Narrow](../screenshots/pr248-integrated/integrated-film-craft-egui_dock-720.png) · [Manifest](../screenshots/pr248-integrated/integrated-film-craft-egui_dock-manifest.json) |

The screenshots compare two settings in the same build. The fixtures used
isolated settings with hardware disconnected and quit gracefully. This is
local runtime evidence. Production deployment, GPU soak, and physical
hardware acceptance are separate.
