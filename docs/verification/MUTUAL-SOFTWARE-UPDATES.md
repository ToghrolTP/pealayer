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
| Real software pulls through one controller port | Both the controller's current image and a registered player's image were read through the controller port, in bounded chunks, and hashed exactly in memory; no duplicate executable download was written. The temporary registry entry was removed. |
| Canonical player deployment | The product's graceful updater installed the candidate, restarted successfully and acknowledged the expected SHA. A fresh manifest reported healthy; native authoring inspection was clean. Its host-specific libmpv identity was preserved. |

### Artifact identities

These digests identify measured runtime artifacts, not version precedence:

- Player source: `563923beddb1930aa99ecc01e600ec90e441dca7`.
- Player executable: `249867b2161f3dfeb93376e9e3399a60ea4e3fab16eabe6beb070e77188d8ea4`, 60,148,224 bytes.
- Player libmpv: `e56ce67cd00f06a59dc7ed5b97a49a9182a381554c755dc4570a52af1ef30e65`.
- Isolated controller replacement: `6a0d59a336b7e2e7e22ae1588ce44bcfe1f253e8dad9a8a11a93e94c9fe7c630`, 56,223,232 bytes. This was the exact staged checkpoint before later service-readiness/test-only follow-ups; it is not the final release package.

## Remaining acceptance gates

- [ ] Real push/restart of the controller through only the player's reachable port. The disconnected older-controller fixture was bootstrapped; relaunching the isolated player fixture was blocked by tool policy and a manual launch was requested. Do not substitute a direct-controller test for this route's runtime proof.
- [ ] Real Windows SCM replacement on the disposable hosted runner. The guarded test refuses non-CI execution and refuses to adopt any existing service. It checks exact digest, durable operation completion, changed running PID, unchanged service configuration and no board connection.
- [ ] Final clean cross-platform packages and current PR checks. An earlier Intel macOS test step exhausted its 15-minute allowance during cold compilation plus serial tests; compilation now has a separate bounded step. Earlier controller CI also caught misplaced selectively staged updater blocks and missing API catalog entries; both were corrected, followed by exact-index verification. Base tooling environment-gate omissions were fixed without staging the other contributor's preview work.
- [ ] Available remote deployment. Cafe remains explicitly user-deferred; no tunnel or remote-access services were changed or repeatedly probed.

Both PRs remain unmerged while these gates are open. Supplied bundle catalogs
offer explicit candidates, not automatic freshness guesses. An older service
that rejects in-process updates needs one administrative bootstrap/repair; no
gateway claims to bypass a missing update API or Windows service permissions.

## Preservation and coordination

All unrelated controller desktop work remains in its owner's working tree.
Only updater changes and the necessary scoped dispatcher/capability hunks were
published on the separate updater branch. A few inherited tooling-gate imports
were fixed separately. No board outputs or firmware were changed. Test storage,
configs and processes are isolated; current player deployment used the product
updater rather than replacing its live executable externally.
