# 📚 Pealayer version history

The original public releases remain v0.1.0 and v0.2.0. Nine retrospective alpha milestones were published in the November 10 backfill. New checkpoints start with v0.12.0-alpha.2. The table uses Asia/Tehran source checkpoint dates.

Read oldest to newest. Each note has ✨ **[Added]**, 🛠️ **[Fixed]**, 🧪 **[Experimental]**, 🐞 **[Known issue]**, packages/CI, genuine screenshots and all merged PRs first included in that source range.

| Version | Source checkpoint | Milestone | Native packages |
| --- | --- | --- | --- |
| 🌱 [v0.1.0](v0.1.0.md) | 2026-09-24 | First public cross-platform packages | 2/4 |
| 🎬 [v0.2.0](v0.2.0.md) | 2026-09-30 | Native shell and semantic hardware authoring | 4/4 |
| 🔌 [v0.3.0-alpha.1](v0.3.0-alpha.1.md) | 2026-10-01 | Capability workspace and unified control port | 3/4 |
| 🌐 [v0.4.0-alpha.1](v0.4.0-alpha.1.md) | 2026-10-08 | Web Studio and live effects recording | 3/4 |
| 🎧 [v0.5.0-alpha.1](v0.5.0-alpha.1.md) | 2026-10-10 | Audio, external mpv and authoring workspace | 4/4 |
| 🎛️ [v0.6.0-alpha.1](v0.6.0-alpha.1.md) | 2026-10-12 | Effects Designer and customizable live status | 2/4 |
| 📡 [v0.7.0-alpha.1](v0.7.0-alpha.1.md) | 2026-10-15 | RF learning, PWM controls and Wayland recovery | 4/4 |
| 🧪 [v0.8.0-alpha.1](v0.8.0-alpha.1.md) | 2026-10-30 | Virtual Board Demo and canonical opcodes | 4/4 |
| 🗂️ [v0.9.0-alpha.1](v0.9.0-alpha.1.md) | 2026-11-05 | Persistent cues and CraftUI workspaces | 4/4 |
| ▶️ [v0.10.0-alpha.1](v0.10.0-alpha.1.md) | 2026-11-09 | Playback recovery and native media sessions | 2/4 |
| 🧰 [v0.11.0-alpha.1](v0.11.0-alpha.1.md) | 2026-11-10 | Command tools, remote folders and sidebar recovery | 4/4 |
| 🎞️ [v0.12.0-alpha.2](v0.12.0-alpha.2.md) | 2026-11-11 | Projects, Virtual Board and native playback | 4/4 |

## 🧭 How to interpret the history

- v0.1.0 includes the June player/dock foundation, July Web/serial work and September binary protocol/curve recording. There were no public releases to invent between those source milestones. The July 23–September 16 and October 15–25 gaps remain visible in the source history.
- v0.5.0-alpha.1 is the versioned identity of the preserved October 10 comparison (`ad4b466`). Existing capture filenames/hashes and historical launchers remain unchanged.
- Retrospective tags select existing commits in main's ancestry. They preserve the original internal binary versions. Starting with v0.12.0-alpha.2, the application version and tag align and the release workflow attests the freshly built archives.
- Recovered CI archives preserve all original file bytes. Container/root names and executable mode bits are normalized. The release manifest retains the original run/artifact digest and file hashes; there is no claim of a new GitHub attestation or new runtime certification.
- Cancelled/failed matrices and unavailable platforms are explicit. v0.5.0-alpha.1 has four packages recovered across two exact-source runs; neither aggregate is falsely marked passed.
- Screenshot provenance distinguishes exact-package review, an earlier included feature build, native/Web and simulated/synthetic content. The screenshot source must be an ancestor of its release source; publication can be later.
- [Current acceptance](../verification/CURRENT-ACCEPTANCE.md) remains authoritative for open human/physical/runtime work. Closed PRs and green CI do not close those gates.

[📝 Changelog](../../CHANGELOG.md) · [Machine-readable ledger](history.json) · [Preserved prior unreleased notes](development-notes-archive.md) · [📦 GitHub releases](https://github.com/ToghrolTP/pealayer/releases)

## ✍️ Preparing the next release

Follow the [milestone publication policy](next.md) and start with the [release-note template](TEMPLATE.md). Keep the milestone emoji, decorated section headings, readable tagged entries and version navigation. The [release-history check](../../scripts/check-release-history.mjs) verifies this presentation alongside source scope and screenshot integrity.
