## Native UI/media checkpoint and continuation

Owning continuation tracker: [#80](https://github.com/ToghrolTP/pealayer/issues/80). Full acceptance checklist and successor prompt: [agent handoff](docs/verification/agent-handoff.md).

### Included in this branch

- Stable semantic mute/solo/lock track controls, cue drag/selection affordances, and timeline authoring improvements from the earlier PR checkpoints.
- Elegance widget palette adapter that retains Pealayer global styling and live system-theme behavior, plus redesigned Effect Controls.
- NLE volume control, seekbar chapter markers, native/Web/mobile alignment and Effects Library geometry fixes.
- Main audio-output device preference and enumeration; optional SFX output preference scaffold **only**. SFX asset loading/playback/scheduling is not yet implemented.
- Color-selector text spacing and a fixed-pattern segmented elapsed-time editor. Unchanged blur does not seek, Enter explicitly seeks, Escape cancels; constructor initialization is repaired in `609148114a680c714fbba526efd669c83242bdac`.
- Native multi-resolution ICO generation for application and playback states, with documentation and durable continuity instructions. Private artwork and host configuration are excluded from this PR.

### Validation and limitations

- GitHub push/API/ref access and the local command runner work again. The earlier native CI constructor error was repaired; consult current checks on the exact head rather than assuming every platform is green.
- PowerShell icon script parsing and generation succeeded for four private packs. Each ICO contains 16, 24, 32, 48, 64, 128 and 256-pixel images. `git diff --check` passes.
- No Rust build/link or clean build was run on the slow production machine. Native interactive acceptance of the new editor/audio controls remains required on a successful CI/David-PC build.
- Runtime recovery and Windows jump-list work are independently owned and must be coordinated with PR #79 and the recovery branch. A previously healthy HTTP endpoint later became unavailable during that work; this PR does not claim current deployment or board health.
- Do not auto-close #80 with this PR: it tracks the remaining media, clipboard, external-mpv, SFX and regression work.
