# Versioned release-history backfill

The release-history source checkpoint is main `0cde16fd5d59e6068d43f81e55b0c2e0f35aca7e`.
The [chronological index](../releases/README.md), [curated notes](../releases/)
and [machine-readable ledger](../releases/history.json) describe eleven versions:
the two original public releases and nine retrospective alpha milestones.

## Historical identity and scope

- Original v0.1.0 remains `d798109c0387ce70ea283a12593216583247b15e`.
- GitHub's published annotated v0.2.0 resolves to
  `d1c4956f8b592cae6f1cd2de065cc3b684da3df3`. A divergent local historical tag
  was discovered and preserved; neither tag nor original download is replaced.
- The former October 10 comparison is v0.5.0-alpha.1, exactly
  `ad4b466fbe40e4556edf4bd9594d3f4eb468329c`. Existing image filenames, hashes,
  technical dates and preserved comparison runtime remain intact.
- The source graph assigns 208 merged PRs exactly once to the first release
  containing their merge commit. Direct pre-PR history is covered in v0.1.0;
  commit-range links preserve the complete inventory. Closed, unmerged PRs are
  not counted as shipped changes.
- PRs #227, #248, #277 and #278 are excluded from the pinned final tag. No
  concurrent terminal-host edits or other local work are incorporated here.

The new alpha names describe historical source milestones. The old binaries'
embedded Cargo/package versions remain unchanged, commonly 0.2.0. This backfill
does not pretend they were rebuilt with newly numbered executable resources.
New publication dates are real; checkpoint dates are displayed separately.

## Package evidence

Thirty native packages were recovered from successful individual jobs for their
exact source commits, while six original release assets are retained. The
release attachments record original workflow/artifact IDs, GitHub artifact ZIP
digests, original member hashes and final archive hashes. Each input ZIP is
checked against its API digest and CRCs; each output member is compared byte for
byte after repackaging. Only container/root names and executable permission bits
are normalized. No new compilation or attestation is claimed.

| Version | Available native platforms | Aggregate limitation |
| --- | --- | --- |
| v0.1.0 | Linux, Windows | Original two-platform release |
| v0.2.0 | All four | Original published release |
| v0.3.0-alpha.1 | Linux, Windows, Intel macOS | Apple silicon cancelled |
| v0.4.0-alpha.1 | Linux, Windows, Intel macOS | Apple silicon cancelled |
| v0.5.0-alpha.1 | All four, across two exact-source runs | Each aggregate has a different cancelled macOS job |
| v0.6.0-alpha.1 | Linux, Intel macOS | Windows tests failed; Apple silicon Web artifact download failed |
| v0.7.0-alpha.1 | All four | Exact-source aggregate passed |
| v0.8.0-alpha.1 | All four | Exact-source aggregate passed |
| v0.9.0-alpha.1 | All four | Exact-source aggregate passed |
| v0.10.0-alpha.1 | Linux, Apple silicon | Windows remote-parent fixture WouldBlock; Intel macOS cancelled |
| v0.11.0-alpha.1 | All four | Exact-source aggregate passed |

The v0.10 Windows failure is the accepted HTTP fixture's non-blocking socket
read (`WouldBlock`, Windows error 10035), not evidence that all PR-level checks
failed. Conversely, earlier PR success is not a passing exact merge matrix.

## Screenshot inspection

Twenty-five captioned image placements include existing native/Web feature
captures and four newly preserved exact-package images across v0.3, v0.7 and
v0.8. The new review instances used isolated empty settings, disabled hardware
autoconnect/media restoration, muted audio and no loaded media. Only windows
and a selector were reviewed; no device connection or demo execution occurred.
All owned review instances closed normally. Production runtime was not replaced.

Computer Use returned a 4×4 transient image before the full application frame;
that export was rejected. The final selected full frames were visually inspected
and preserved as the original JPEG bytes, with manifests. An archived GIF whose
opening frame showed the recording desktop was also excluded from release notes.
The remaining archived images were inspected together and retain their existing
provenance, native/Web distinctions and synthetic/private-media boundaries.

Every capture's source or publication ancestor is checked against its release
source. Some screenshots show earlier included feature builds: their captions
say so, and do not claim exact final-tag executable identity. The June status
bar's historical placeholder memory/readiness is explicitly identified.

## Validation and remaining acceptance

`node scripts/check-release-history.mjs --git` verifies version ordering,
source/capture ancestry, 208 unique PR assignments, the 25 image hashes and
36 native package identities, all required note sections and download links.
The default mode runs in CI without requiring a full Git checkout. Actionlint
and whitespace checks cover the workflow integration.

Each release includes a checksum list, source/package manifest, screenshot
provenance, screenshots archive and complete notes. Publication must be read
back to verify tags, release bodies, uploaded digests, rendered images and the
original asset identities; a local draft is not a delivered release.

The [current acceptance index](CURRENT-ACCEPTANCE.md) remains in force. This
documentation/package recovery does not establish Cafe deployment, human
audio/visual acceptance, physical timing/output behavior, GPU long playback,
native Apple media completeness or soak acceptance. Known failures and later
fixes are retained in each version's **Bugs and problems in this build** section.
