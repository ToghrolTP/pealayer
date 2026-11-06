# Mutual software updates — delivery receipt

Tracks [Pealayer #240](https://github.com/ToghrolTP/pealayer/issues/240),
[Pealayer PR #241](https://github.com/ToghrolTP/pealayer/pull/241) and
[PCController PR #740](https://github.com/atomicdeploy/PCController/pull/740).
The [contract and implementation comparison](../SOFTWARE-UPDATES.md) explains
the existing owners, shared client, bootstrap routes and bundle format.

## Confirmed results

| Check | Evidence and boundary |
| --- | --- |
| Rust regression suite | 1,136 library tests passed; one pre-existing ignored test and the isolated updater fixture excluded from that run. The updater fixture then passed separately. |
| Shared transfer client | Tests pass for both products, exact transfer ACKs, source SHA/size, terminal truth, no mutation replay, product mismatch rejection, bundles and explicit older-controller bootstrap. |
| Controller artifact and RPC packages | Both complete selected packages passed. After scoped staging, the exact committed/index source was additionally compiled and tested through a Go overlay; unrelated desktop changes/untracked sources were excluded without resetting the checkout. |
| Controller service lifecycle | Commit, health-timeout rollback, failed-start restoration and delayed-readiness/PID fences tested with an injected SCM lifecycle. These tests do not prove a production service restart. |
| Real older-player replacement through controller gateway | An isolated older HTTP-only player was replaced through the controller gateway and restarted with the exact candidate SHA and `health_state: healthy`. Its native process ID changed and authoring inspection remained clean. |
| Real older-controller bootstrap/restart | The shared client used the established controller artifact/update API against an isolated, hardware-disconnected older host. The replacement reached its exact tracked completed operation and running SHA, surviving the stable verification window. |
| Real controller replacement through the player port | Passed using only the player endpoint: `staging` → `awaiting-restart` → `completed/completed`, changed controller process ID, exact image SHA and durable helper commit. `scripts/test-software-update-runtime.mjs` preserves this opt-in acceptance test and rejects staged-as-completed results. |
| Identical controller image | An API request using the already stored current artifact reported `completed/current`, with unchanged process ID and no replacement helper. |
| Real software pulls through one controller port | Both the controller's current image and a registered player's image were read through the controller port, in bounded chunks, and hashed exactly in memory; no duplicate executable download was written. The temporary registry entry was removed. |
| Real bundled candidates through the player port | Both products' candidates were read through the connected controller's confined catalog, in bounded chunks, and hashed exactly in memory. The player candidate used a hardlink, not another executable copy. |
| Real Windows service replacement | [Hosted run](https://github.com/atomicdeploy/PCController/actions/runs/42299993619), Windows job, passed: exact digest, durable completed activation, changed service PID, unchanged service configuration and no board connection. This was the service/activation checkpoint before the later historical-result/no-op follow-up; those follow-ups passed focused tests and real foreground acceptance separately. |
| Canonical player deployment | The product's graceful updater installed the candidate, restarted successfully and acknowledged the expected SHA. A fresh manifest reported healthy; native authoring inspection was clean. Its host-specific libmpv identity was preserved. |

### Artifact identities

These digests identify measured runtime artifacts, not version precedence:

- Player source: `563923beddb1930aa99ecc01e600ec90e441dca7`.
- Player executable: `249867b2161f3dfeb93376e9e3399a60ea4e3fab16eabe6beb070e77188d8ea4`, 60,148,224 bytes.
- Player libmpv: `e56ce67cd00f06a59dc7ed5b97a49a9182a381554c755dc4570a52af1ef30e65`.
- Final isolated controller acceptance build: `87c8deb6cf78f1452a92444af017bde6ebe7fe1eb4adf347be53cebb0e72d0d8`, 56,239,616 bytes, from source `c35afbec` with test version `0.0.0-update-acceptance`. This is an acceptance build, not a release package.

## Remaining acceptance gates

- [x] Real push/restart of the controller through only the player's port. The operator launched the fixture after tool policy blocked launch; subsequent connection, update, verification and closure used existing APIs.
- [x] Real Windows SCM replacement on the disposable runner; the test refuses non-CI execution and any existing service.
- [ ] Final current required checks and safe parent-branch integration. The full controller host matrix passed at the linked service checkpoint. Latest Windows packaging failed before compilation because Chocolatey returned HTTP 504 fetching WinFlexBison; it needs an infrastructure retry once its parent run finishes. Intel macOS packaging passed after separating cold compilation from test execution. Earlier dispatcher placement, API catalog and tooling-environment failures were corrected rather than bypassed.
- [ ] Available remote deployment. Cafe remains explicitly user-deferred; no tunnel or remote-access services were changed or repeatedly probed.

Both PRs remain unmerged while these gates are open. Supplied bundle catalogs
offer explicit candidates, not automatic freshness guesses. An older service
that rejects in-process updates needs one administrative bootstrap/repair; no
gateway claims to bypass a missing update API or Windows service permissions.

Live tests found and fixed premature controller completion: staging now remains
pending across restart until the exact helper commits or rolls back. Pending
host activation blocks overlapping updates. Historical committed results stay
valid after later upgrades; identical candidates skip staging/restart. Focused
tests cover these cases. The earlier isolated candidate was reset after normal
API Quit when its historical-result bug blocked another update; the final
measured transaction then exercised the fixed API path without a manual
replacement during that acceptance run.

## Preservation and coordination

All unrelated controller desktop work remains in its owner's working tree.
Only updater changes and the necessary scoped dispatcher/capability hunks were
published on the separate updater branch. A few inherited tooling-gate imports
were fixed separately. No board outputs or firmware were changed. Test storage,
configs and processes are isolated; current player deployment used the product
updater rather than replacing its live executable externally.
