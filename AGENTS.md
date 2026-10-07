# Pealayer alpha contract

- Alpha has one current API and persisted-data contract. Do not add build-number checks, rolling-upgrade branches, old field or method aliases, legacy migrations, or speculative future-version handling.
- Update Pealayer, PCController, firmware, the Web UI, tests, and documentation together when the contract changes. An obsolete build is replaced, not supported in parallel.
- Be tolerant of runtime faults, not obsolete schemas: validate authoritative responses, normalize bounded current-contract values, retain the last known-good state when a refresh is incomplete, and request an authoritative refresh after dropped or reordered events.
- Reject malformed required current-contract fields with a clear error. Never silently publish a false empty/default hardware state after a failed catalog read.
- Capability-driven behavior, OS/runtime safety checks, transport retries, and renderer fallbacks are operational fault tolerance. They are not version-compatibility shims and must not be removed merely because the project is in alpha.
- Use stable semantic IDs and typed machine-readable errors. Do not infer semantic state from labels, human error text, relay numbering, or other incidental presentation data.

# Continuity and native Windows assets

- GitHub is the source of truth. Before each handoff, audit all relevant worktrees and local-only commits, push useful source/tests/docs/scripts to named branches, verify exact GitHub refs, and record the owning issue/PR, tests, blockers and next action. Preserve concurrent/user work; never reset or clean it away.
- Never publish credentials, private settings/media URLs, user artwork, caches, logs or generated-only artifacts. Back up private runtime state separately before proposing a machine wipe; clean repository worktrees alone do not establish wipe readiness.
- Windows custom application, playback-state, taskbar, titlebar, shortcut and executable-resource icons must use native multi-resolution ICO files, generated with high-quality resampling. Private raster masters may remain as authoring inputs; browser/PWA assets may use web-required formats. Patch executable resources before signing.
- Consult `docs/verification/agent-handoff.md` for the current continuation checklist, production-host build restriction and host-specific runtime safety rules. Treat its runtime observations as dated evidence, not current health.
