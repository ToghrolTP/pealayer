# Pealayer alpha contract

- Alpha has one current API and persisted-data contract. Do not add build-number checks, rolling-upgrade branches, old field or method aliases, legacy migrations, or speculative future-version handling.
- Update Pealayer, PCController, firmware, the Web UI, tests, and documentation together when the contract changes. An obsolete build is replaced, not supported in parallel.
- Be tolerant of runtime faults, not obsolete schemas: validate authoritative responses, normalize bounded current-contract values, retain the last known-good state when a refresh is incomplete, and request an authoritative refresh after dropped or reordered events.
- Reject malformed required current-contract fields with a clear error. Never silently publish a false empty/default hardware state after a failed catalog read.
- Capability-driven behavior, OS/runtime safety checks, transport retries, and renderer fallbacks are operational fault tolerance. They are not version-compatibility shims and must not be removed merely because the project is in alpha.
- Use stable semantic IDs and typed machine-readable errors. Do not infer semantic state from labels, human error text, relay numbering, or other incidental presentation data.

## Verification and deployment

- Tests that exercise persistence must receive an isolated `PEALAYER_CONFIG_FILE` from the parent process before startup; never point fixtures at the user's OS configuration or Registry mirror. Pure catalog/model reconciliation must not save configuration or publish engine commands. Do not mutate process environment underneath libmpv/native threads to retrofit isolation.
- Every feature/fix pass includes a host-compatible Cafe-PC deployment and live verification, or an explicit deployment blocker. Do not call a local build delivered to Cafe-PC.
- Preserve and coordinate unpushed or separate host work before replacement. Package against the host's actual libmpv runtime and validate the manifest and runtime smoke test.
- Use application IPC/RPC to request graceful quit before replacement; use the peer updater rather than manually replacing a running executable. The user permits terminating an already-confirmed hung Pealayer process after graceful exit fails; preserve diagnostics first. This is not permission to terminate healthy unrelated processes.
- Never invoke egui Context accessors, widgets, or repaint callbacks from inside input/data/memory/output transactions; the context lock is non-reentrant. Snapshot inputs first, then perform the narrow transaction.
# Continuity and native Windows assets

- On Windows, Pealayer-owned source, caches, staging and runtime files belong under `%LOCALAPPDATA%\Programs\Pealayer`. The canonical source is `source\Pealayer`, the shared Cargo cache is `build-cache\cargo-target`, and the running executable is `bin\pealayer.exe`. `C:\development` is reserved for development tools, not Pealayer checkouts or build artifacts. Use the shared Windows path resolver rather than inventing another checkout/cache path; preserve existing work when relocating it.
- GitHub is the source of truth. Before each handoff, audit all relevant worktrees and local-only commits, push useful source/tests/docs/scripts to named branches, verify exact GitHub refs, and record the owning issue/PR, tests, blockers and next action. Preserve concurrent/user work; never reset or clean it away.
- Never publish credentials, private settings/media URLs, user artwork, caches, logs or generated-only artifacts. Back up private runtime state separately before proposing a machine wipe; clean repository worktrees alone do not establish wipe readiness.
- Windows custom application, playback-state, taskbar, titlebar, shortcut and executable-resource icons must use native multi-resolution ICO files, generated with high-quality resampling. Private raster masters may remain as authoring inputs; browser/PWA assets may use web-required formats. Patch executable resources before signing.
- Consult `docs/verification/agent-handoff.md` for the current continuation checklist, production-host build restriction and host-specific runtime safety rules. Treat its runtime observations as dated evidence, not current health.

## Human-facing history and acceptance

- Keep completed issue/PR summaries concise and product-focused. Put exact source/runtime hashes, measured timestamps and host-specific deployment receipts in linked technical evidence, not repeated finished-task prose. Preserve safety requirements, meaningful version references and actual acceptance limits; do not erase evidence to make an issue look complete.
- Read the commit history before writing changelogs. Group actual changes by domain, maintain curated version notes and correct labels, and publish verified stable milestones with the matching packaged artifacts. Do not attribute later changes or screenshots to an older release. Genuine intermediate captures are useful; reconstructed historical images are not evidence.
- Use `docs/verification/CURRENT-ACCEPTANCE.md` for current remaining gates. A historical unchecked implementation item can be superseded by later delivered work without declaring its human, audio or physical acceptance passed.
- When human intervention is genuinely needed, use the existing APIs for an advertised appropriate buzzer melody with bounded repeats and a large Persian message on the production desktop. Stop attention when acknowledged; never leave an unbounded sound loop or use motion/relays as an alert. Routine progress does not need an alert.
- Diagnose excessive CPU with bounded live samples and trace its actual owning application/path before fixing it. Preserve fast event-driven playback/telemetry while reducing unchanged idle work; a short paused sample is not sustained playing, taskbar-preview or physical-timing acceptance.
