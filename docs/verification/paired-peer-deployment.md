# Cafe authority and Erfan consumer delivery

## Observed 13 October 2026 (Tehran time)

The user requested current PCController/Pealayer on both machines, with Cafe
as authority and Erfan as its remote. Tests/CI waiting were skipped at the
user's request. Package integrity, resource, Web build guardrails and retained
destination-runtime smoke probes ran; no Rust build ran on either destination.

| Destination | PCController | Pealayer | Role/status |
| --- | --- | --- | --- |
| Cafe-PC | Updated and terminal-verified | Updated through peer updater | Authority, native interactive session 4, paused, COM3 connected |
| Erfan-Gaming | Updated and terminal-verified | New candidate staged; updater invocation blocked by tool policy | Existing older build is a connected consumer, interactive session 1, no local hardware scheduler |

Pealayer candidate source: `d42cfeb8d08d1c662aea9683f27cca326bdd9b39`,
clean source, Windows MSVC. Player SHA-256:
`26bbc3fc5ffb9af1569f229bc8ff3008af466b89ed8da7d1f0d676f220773630`.
Downloader SHA-256:
`c189ed4b9e1c82a2abe34aea881a40778d40eae9912f3c63bf23a5aae3cb534c`.
Source is on PR #174; this deployment is not evidence that its existing Windows
GNU access-violation gate passed or that all PRs were merged.

PCController executable SHA-256 on both running processes:
`8f2153135cfe21f092d2baa7f2b6aa56321a72b151fb85fd7c19543b2648d182`.
Host source digest:
`d5d1b206b36870a12b69693ee07978275eb9cbf569c2d0a4e4272161469853b8`.
Build source ref: PCController `9f9c37b75b31b6b094375d0256e0c8c6467f3e9f`.
Controller-owned updates returned `terminal_verified=true` for
`op-07cfe167cfd6cecf` (Cafe) and `op-6e8a444de56f75b8` (Erfan).
Matching C ABI, programmer helper, icons, notices and manifests were published
while Pealayer was stopped, with hash-checked, low-write hardlinks and named
rollback for replaced files. Configuration/logs were not overwritten.

## Runtime and routing

Cafe peer update `update-55a4b07c-3450-44df-a9a7-7a9e900ea4e9` completed.
Its process-local source/digest matched the candidate after restart. Fresh
retained-runtime smoke passed on **both** destinations; this supersedes the
historical Cafe smoke-launch-denial as a current blanket deployment blocker.
It does not justify bypassing a newly blocked command.

Cafe retained `872827614ed0adfca11e68def5273bcfcaea6acf38bbf1950c35980b59f43a5f`;
Erfan retained `0a81c004aae0ee7d512b9a26e38f66281f9591e84e1663215cc3a36a4bde6f6a`.
Neither host received David's DLL or the other's runtime. Adjacent aliases match.

Cafe's stored Pealayer listener addresses changed from loopback-only to
`127.0.0.1` plus its observed Ethernet address `192.168.1.185`. Application
restart was required before that listener appeared. No wildcard, firewall,
SSH/Chisel/RDP service or global PowerShell policy was changed. Repository
probe scripts used process-scoped RemoteSigned after the default SSH session
refused loading .ps1 files.

Erfan's existing Interactive task now launches:

```text
pealayer.exe --connect pealayer://192.168.1.185:8080 --client-port 8081
```

Fresh consumer diagnostics: connected=true, local_storage=cache_only,
local_hardware_scheduler=false, no command/transport error and zero paused
preview drift. Its process-local update manifest was checked against its own
process identity before preparing an update; update does not forward Quit or
replace Cafe. The new Erfan updater invocation was then rejected before
execution by tool policy. No alternate updater/activation route was attempted.
Erfan is still source `9b72fac169238636faa1a7c5f92fb006b383e800`, player digest
`6a26992d77cead8b2d0ae3c9f8232292b453be7ea8ced27784b9209ceeb9574f`.

Cafe owns publishing as `pealayer:CAFE-PC:8080`, unlocked with no pending
handoffs. Sampled preparation/clock ACK revisions matched; no current hardware
error or failed clock update was reported. The prepared plan was empty and
paused: this is not cue execution or physical timing acceptance. Relays were
zero and both seats stopped; board firmware identity remained `261012050150`.
No Play, effect Run, buzzer tone, flash or reset was requested.

## Preservation and remaining action

Before/after private JSON backups reside under David's program-owned
`verification/peer-deployment/{cafe-pc,erfan-gaming}`, not Git. Initial digests:
Cafe `879c897ab873871ac14cccf288a750e7e9f8fdeddb07e1195f60f5490d666d83`;
Erfan `8af354d2d4738a4eb8a9409cbec3d722a842b703d7051d121f7022afc73ec7c6`.
Erfan's config remained byte-identical while acting as a consumer. Cafe's
four-step draft retained every original field and step; new repeat defaults
were added by current serialization. Its pre-launch saved one-cue session
belongs to a different media target than the restored movie and was not
attached to that movie. That exact older session remains in the private backup;
do not overwrite the active timeline to restore it without selecting its media
and checking for new authoring work.

The user can run the staged updater **on Erfan**:

```powershell
& "$env:LOCALAPPDATA\Programs\Pealayer\staging\peer-deployment\pealayer.exe" --deploy-to http://127.0.0.1:8081 | Out-Host
```

After it finishes, verify process-local source/digest and consumer role, then
publish the staged downloader/font and destination manifest while that process
is gracefully stopped. The staged `deploy-adjuncts.ps1 -PlayerUpdated` performs
those hash-fenced adjunct updates; do not call this installed until verified.
Complete native UI, audio, media playback, physical cue timing and soak remain
separate gates. Current API/paused connectivity is not production certification.
