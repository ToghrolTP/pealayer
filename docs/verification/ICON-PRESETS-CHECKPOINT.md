# Application icon preservation checkpoint

Owner: [Preferences consolidation PR #87](https://github.com/ToghrolTP/pealayer/pull/87),
tracked from [delivery issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

The previous public Pealayer SVG, PNG and multi-resolution ICO have been
restored byte-for-byte from the parent of the icon replacement in PR #85.
No user/private branding was copied into the repository. Provenance and user
instructions are in [the bundled icon guide](../../assets/icons/README.md).

One Rust-generated Appearance → Interface dropdown selects **Current** or
**Classic (previous)** on both native and Web surfaces. Current remains the
default. The optional custom-icon card stays collapsed when unconfigured;
custom/default/playback-state and deployment/environment overrides keep their
priority. Selection is persisted as `app_icon_preset`.

The native main window resolves the selection at startup and on live config
changes. Windows tray registration now uses the window icon; subsequent
`WM_SETICON` messages refresh the tray using the borrowed, eframe-owned handle.
Web header/favicon/PWA/media-session image URLs follow the authoritative status
revision, so switching icons does not require a playback-state change.
Pre-existing pinned shortcut and executable-resource icons remain separately
owned build/deployment assets, not files rewritten by a runtime preference.

## Validation and delivery gate

- Full Web build, TypeScript, responsive/parity, preference-file, gesture/time,
  messaging, PWA and readable-asset checks passed. The branding refresh guard is
  included in the shared Web parity checks.
- Focused Windows MSVC branding checks passed: both assets decode, ICO packs
  contain multiple native sizes, persistence validates the enum, custom/state
  overrides win, missing custom files fall back, and a live preset change emits
  a window-icon update.
- API/PWA checks passed for the selected preset and real 192/512 PNG output.
  The default-config test also passed. The existing appearance-row test was
  updated to include the new dropdown while keeping Language last.
- All 25 shared-preferences tests passed after updating that expected row/icon
  inventory. In total, 36 focused native tests passed (8 branding, 25 preferences,
  2 PWA/API and 1 default config); this is not an all-suite acceptance claim.
- Cafe was freshly reachable and its API reported connected hardware. Its
  running application was not replaced or reconfigured by this feature pass.
- Production deployment is blocked by the consolidation branch's Windows GNU
  test access violation, independently observed on its preceding head:
  [Windows CI gate](https://github.com/ToghrolTP/pealayer/actions/runs/37834882658/job/113510326225).
  Focused MSVC success is not a full GNU-suite pass or a Cafe delivery claim.

Next: clear the Windows test gate, package the clean source against each
destination's own verified libmpv, deploy with the peer updater, and exercise
both choices through native/Web Preferences on Cafe and David. Preserve custom
host artwork and media settings. Existing worktrees and other owners' changes
were left intact; no duplicate worktree/build-cache root was created.
