# Pealayer continuation and no-loss handoff

Owning tracker: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80). Post subsequent checkpoints and child-issue links there.

Current continuation: [production deployment and consolidation checkpoint](DEPLOYMENT-AND-MERGE-CHECKPOINT.md).
It reconciles the latest Preferences requests, Cafe's installed candidate and
controller-contract playback blocker, Erfan's staged update, merged PRs #82/#83,
and remaining CI/branch gates. Older observations below are historical and must
not replace that checkpoint or a fresh live check.

## Scope and evidence (2026-10-08)

This is a checkpoint, not a claim that every requested feature is finished or that an entire Windows machine can be erased. GitHub owns source/history; private settings, artwork, media references, and host runtimes must be backed up separately. Never upload credentials, private media URLs, machine configuration, logs, caches, or generated binaries to source control.

- Primary feature branch: `fix/timeline-track-state-buttons`, PR [#77](https://github.com/ToghrolTP/pealayer/pull/77).
- Recovery checkpoints: `713fcf80bcf786fbc80fb334fba3cb283379f889` repairs color-label rendering, segmented elapsed-time input, and audio-device enumeration; `609148114a680c714fbba526efd669c83242bdac` initializes the new elapsed-time fields in the native entry point. Both are on GitHub. CI, not the production host, is compiling the branch.
- This document's commit also makes `scripts/new-icon-pack.ps1` produce native multi-resolution state ICOs and documents their use. Four private host packs were converted; raster masters remain private authoring inputs, not native configured icons.
- Historical live-media branch `drsdavidsoft/remote-live-media-20260930` at `77ad6e804df400bccfb8b91d133b3cbcb2f1d84f` and unified-port branch `drsdavidsoft/unified-control-port-20260930` at `d2fd6593d9dc325b5f61b422ccfc0d98f0b5c89b` were recovered and pushed. They preserve history, **not** a recommendation to merge outdated implementations into current main.
- Native Web/mobile alignment, seekbar chapter dividers, NLE volume controls, Effects Library geometry, and initial palette-adapted Elegance widgets have source implementations. Their presence in source is not installed-build acceptance.
- Main-media audio-output preferences are implemented. `sfx_audio_device` is only a preference scaffold: importing, scheduling, and playing SFX are still required. Device enumeration/runtime switching/hotplug need verification.
- During this audit, the command runner, authenticated GitHub API, fetch, and push worked. A native constructor compilation fault was discovered in CI and repaired. At the latest pre-handoff check the running application disappeared and the HTTP endpoint refused connections; the controller had previously been disconnected. An independently active recovery agent owns runtime investigation. Do not infer production health from earlier observations.
- Another agent owns `release/cafe-shell-recovery` and work in `fix/taskbar-preview-repaint-loop`. Its eventual pushed refs/PR and live deployment report must be checked before wipe-readiness. PR [#79](https://github.com/ToghrolTP/pealayer/pull/79) owns Windows jump-list/icon work; do not overwrite concurrent work.

Final audit for this pass: see [handoff audit](handoff-audit-20261008.md). Recovery refs and PR #81 are now on GitHub, all seven worktrees were clean, and private backup copies were hash-verified off-machine. This does not establish live application health or whole-machine wipe readiness.

## Ready-to-use successor prompt

> Continue ToghrolTP/pealayer using the owning handoff issue and this document. Fetch current GitHub refs, read PR #77 and #79 plus the recovery/CPU owner's latest checkpoint, and coordinate ownership before editing shared worktrees or replacing a running application. Preserve and inspect all existing work; work on a named branch/PR, not a detached local-only commit. Complete the prioritized checklist below, independently verify historical closed work against current code and runtime evidence, and create focused linked issues for genuinely deferred work. Build on GitHub CI or the authorized faster David-PC, never compile/link Rust on the production CAFE-PC. Use Pealayer's own IPC/RPC for application/build diagnostics; extend it in Rust if required. Keep the current alpha contract coherent across Pealayer, PCController, firmware and Web UI rather than adding version-specific compatibility branches. Deploy the exact source build with a destination-validated runtime, verify real media and controller connectivity, then commit/push all useful work and record remote refs, tests, blockers and next action before every handoff. Keep secrets/artwork/runtime settings private. Do not declare the whole machine safe to wipe merely because repository worktrees are clean.

## Prioritized remaining work and acceptance

### 1. Recover a verified production deployment and finalize existing PRs

- [ ] Check CI for the latest PR #77 head; repair remaining errors, integrate current main without losing concurrent work, finish review and merge when ready. Keep PR #79/recovery changes coordinated, not overwritten.
- [ ] Retrieve the exact successful Windows build. Pair executable/assets with the destination's independently validated libmpv, not a DLL from the faster build workstation. Confirm smoke-test exit 0, interactive window, health, JSON-RPC, real media playback, controller connectivity, and absence of new Application Error/WER events. Record the actual runtime commit/hash.
- [ ] Reconfirm all needed tools before installation; reusable dependencies belong in Program Files and machine PATH. On David-PC reuse versionless canonical `C:\development` paths, avoid duplicate compiler PATH entries, and reuse validated downloads. Verify Rust/Go/MinGW/UPX setup only where needed. Report build times on both machines only from measurements; the production no-build restriction remains in force unless the user changes it.
- [ ] Restore the private host state from the separately confirmed backup if necessary. Native default/state icons on this host are Soulayer ICOs. Do not publish private icon masters or surveillance credentials.

### 2. Finish current native UI/media requests

- [ ] Exercise segmented `HH:MM:SS.sss` editing: unchanged blur does not seek; changed blur or explicit Enter seeks; Escape cancels; punctuation cannot be destroyed; typed/pasted invalid characters are rejected; arrow/group selection and Backspace/Delete behave consistently.
- [ ] Verify NLE volume slider, chapter markers in the seekbar, mobile button/icon vertical centering, Effects Library right edges, and stable color-indicator spacing while hovered/focused in the deployed build.
- [ ] Implement SFX asset loading/library entries and audio-effect playback/scheduling. Default output follows system/main settings; allow per-SFX alternate device. Expose audio device/backend preferences with defaults, errors and hotplug handling; wire the existing SFX preference to a real playback engine.
- [ ] Make Preferences > Appearance custom icons compact/collapsed when unconfigured; add Browse, previews, clear/reset, and a unified non-repetitive state editor. Windows native app/taskbar/titlebar/shortcut/PE resource icons must be multi-resolution ICOs. Browser favicon/PWA derivatives may use web-required formats. Verify state transitions. Embedded executable icon patching remains separate from runtime icon configuration, and must happen before signing.
- [ ] Continue selective [egui-elegance](https://github.com/matrix-research-inc/egui-elegance) integration where it improves controls. The initial adapter is `src/ui/mod.rs::sync_elegance_theme`: it supplies Pealayer palette values and restores global egui style after installing widget theme data. Audit all palette roles and widget behavior across both palettes, light/dark and live OS theme changes. Never replace Pealayer's tuned global theme with Elegance's stock theme.

### 3. Clipboard and source browsing

- [ ] Add default-enabled configurable clipboard URL detection, recognizing valid local directories and remote media/directories. Open the appropriate dialog or update its existing input; avoid repeated prompts for the same dismissed value and never expose URL credentials in logs/OSD/issues.
- [ ] Debounce valid user edits and fetch source information/browse directories automatically. Probe proxy-enabled and direct routes concurrently when auto mode is selected; choose the successful route in the checkbox, honoring explicit user overrides. Cancel/ignore stale results and bound all requests.
- [ ] Keep existing list rows stable for fast responses; show a loading state only after roughly 200–300 ms. Add Back inside the list, fix action-button hover geometry, and use restrained transitions only where they improve usability/accessibility.
- [ ] Reverify RTSP live streams and HTTP media, URL recents, API loading, buffered seekbar extent, and seeking only where the source supports it. Use the user's private runtime source, never paste credentials into GitHub.

### 4. External mpv

- [ ] Support external-only playback, synchronized internal+external playback, and Pealayer as remote for an existing mpv. Use native socket/named-pipe JSON IPC, property observation and events, not repeated process queries.
- [ ] Provide helper/script setup if needed, duplex playback/media/seek/volume/mute/speed/track synchronization, reconnect with authoritative refresh, bounded queues and clear ownership to prevent echo loops. Measure responsiveness and repeated-command stress; do not call polling real-time synchronization.

### 5. Timeline/effects and hardware regression audit

These requests have been worked on across multiple PRs. Inspect current code and test; a closed issue or merged PR alone is not evidence of completion. In particular independently audit closed issues/PRs authored by DRSDavidSoft. Reopen or create linked tracking issues for actual gaps.

- [ ] Board-driven and host-driven recording shows sequences live and persists them on Finish. Run Now/Play uses stable `effect:ID` identifiers; structured IPC errors become clear messages, not raw JSON. Publish enables Run; context menu shows valid Run/Stop actions and has no Rename item.
- [ ] Current semantic seat/channel names are used. Feedback is synchronized without indicator flicker. Stress alternating motion at 10–15 commands/second for at least 15 seconds and extended operation without Pealayer/PCController lockups; check bounded queues, timeout/reconnect behavior, and thread responsiveness.
- [ ] Time/duration suffixes normalize on accept/blur; human units default and ms preference work. Effect name/duration edits update all placed timeline cues. Delete removes selected cues; dragging between editor command lanes changes the target channel. Moving effects between folders changes their group; folders have editable icons, dim when empty and omit empty chevrons. Record buttons stay solid red.
- [ ] Directional marquee selection uses blue window/full-containment vs green crossing/intersection semantics correctly. Draggable fixed-size cues show grip bars; text overflow is configurable and ellipsized with full tooltips. Mute/solo/lock buttons have stable hover geometry and semantic track state/dimming.
- [ ] Adding keyframes never seeks unexpectedly. Double-clicking a track opens cue creation with duration/action; the active-track hotkey works. Effect Controls panel supports the current cue types, timing/unit inputs and semantic actions elegantly.
- [ ] Smooth playhead-follow near viewport edge, bring-into-view, configurable/reorderable timeline toolbar, useful optional cue actions, standard shortcuts, and dimmed second-line exact-keyframe timestamps work in actual use.
- [ ] RF learned remotes display correctly with event-driven updates (no once-per-second modal rebuild). Buzzer section exists and is polished; melody catalog uses events plus authoritative refresh when the selector opens. Addressable-strip controls are polished and verified through Pealayer -> PCController -> firmware -> physical LEDs; rainbow identification requires human confirmation before stopping.
- [ ] Trace timing faults and excessive CPU to their real owner, fix bounded/event-driven behavior in the respective project, and remeasure. Coordinate the taskbar-preview repaint work rather than duplicating it.

### 6. System integration, evidence and documentation

- [ ] Verify current IPC/RPC/API channels and JSON/COBS paths both ways. PCController is the sole board driver/master coordinator when connected; board events navigate back to Pealayer. Optional direct Pealayer COBS must acquire exclusive ownership, never compete with PCController for the board.
- [ ] Validate the current unified-port contract and update all reliant projects together. Review the original PCController PR #362 intent against current code and later work; do not resurrect an obsolete version protocol. Use typed errors, stable IDs, authoritative event refresh, and retention of last-known-good state on transient faults instead of legacy/future-build shims.
- [ ] Review related open issues: [#30](https://github.com/ToghrolTP/pealayer/issues/30) Windows readiness, [#51](https://github.com/ToghrolTP/pealayer/issues/51) libmpv, [#60](https://github.com/ToghrolTP/pealayer/issues/60) Windows baseline, [#61](https://github.com/ToghrolTP/pealayer/issues/61) motion lockup, [#62](https://github.com/ToghrolTP/pealayer/issues/62) CPU, [#63](https://github.com/ToghrolTP/pealayer/issues/63) parity, and [#66](https://github.com/ToghrolTP/pealayer/issues/66) frame window. Update the correct owner with reproducible evidence rather than duplicate issues.
- [ ] Preserve polished GitHub Actions names/summaries/artifacts and strengthen focused checks as needed; use real newlines via body files for issue/PR prose. Update user/developer docs and screenshots where useful.
- [ ] Narrate useful progress in Persian using the existing shared Piper setup. If human intervention is genuinely required, coordinate an appropriate PCController buzzer melody and a large Persian message; stop attention loops after acknowledgement, and never issue hazardous motion merely to attract attention.

## Deployment and coordination rules

- No local Rust compile/link or clean builds on the production machine. Prefer existing CI or incremental builds on the faster authorized host; coordinate its active agent first. If Pealayer is running there, replace it only after arranging ownership and using that host's validated runtime.
- Use Pealayer IPC/RPC for running-application build/status diagnostics. Shell commands remain appropriate for repository, toolchain and file transfer work.
- Never copy another host's libmpv runtime into a live deployment. A candidate Program Files runtime that passed smoke-test has crashed on CAFE-PC during actual media initialization with `0xc000001d`. Rejected SHA-256: `0A81C004AAE0EE7D512B9A26E38F66281F9591E84E1663215CC3A36A4BDE6F6A`. Known-good generic CI runtime at commit `52be44c1aceb52bf04db8fe462d27af9b46be5d6`: `D10D0994BD1398813DA87FDB299961B378C71561BAE1D135CB2B641B5864D7EB`. Validate any replacement independently; the two adjacent DLL aliases must match.
- Launch GUI from the logged-in Explorer desktop, not SSH service session 0 or PsExec injection. Verify the actual interactive session rather than assuming a historical session number.
- Remote-access services are intentional: never change Chisel, SSH, RDP, TeamViewer, AnyDesk or RustDesk for this project. Do not parse huge historical rollout logs with PowerShell; use bounded streaming if old context is indispensable.
- Prefer DRSDavidSoft for repository authentication. AtomicDeploy is explicitly allowed when necessary, such as archiving branches containing workflows; use command-scoped credentials without printing secrets or changing shared authentication state.
- Coordinate the ongoing PCController/Pealayer owner and Piper owner before touching their resources. Concurrent workflows make an immediate machine wipe unsafe until owners confirm their checkpoints/backups.

## Before ending every pass

Audit **all relevant** worktrees, including new worktrees created by concurrent agents, for tracked/untracked changes and branches/commits not reachable on GitHub. Commit useful source/tests/docs/scripts to named branches, push and verify exact remote refs through GitHub. Check stale worktree metadata with a dry run. Do not reset/clean/discard user work. Record WIP honestly, tests actually performed, blockers, owning issue/PR and next action. Inventory ignored useful files separately from reconstructible build products. Private runtime settings/artwork/sidecars need a verified off-machine backup; they are not safe to publish. Whole-machine wipe readiness also requires independent backup of other projects, media, credentials, and active task state outside Pealayer.
