# RF reception and shared learning controls

Native and Web RF managers display reception indicators and modal-local packet
counts for every known waveform, including unassigned learned records and
unlearned observed codes. Identity includes code, bits and protocol. Unlearned
rows use muted text; receiving indicators remain semantically green. Counts use
PCController cumulative reception metadata so coalesced redraws do not drop
repeats. Refresh preserves the baseline; closing/reopening resets it. Presence
expires after 250 ms of silence without a perpetual repaint/network poll.

RF Settings edit the coordinator's saved preferences, not a Pealayer-local copy:
timed learning is off, the optional duration is 60 seconds, and continuous
learning is off by default. Learn uses those preferences; explicit Stop remains
available while learning. Unknown observations can be assigned on the host or
explicitly added, unassigned, to a capable board via the verified replacement
API. Merely opening the dialog or receiving a code never learns or actuates it.

PCController's normal Web RF manager and TUI settings follow the same lifecycle;
hard-coded guided A/B/C/D entry points are hidden for later pattern profiles.
The controller contract and firmware limitations are documented in
`atomicdeploy/PCController/docs/RF-RECEPTION-AND-LEARNING.md`.

Keep source/type-check/unit verification separate from Cafe deployment, physical
RF press/hold reception, EEPROM acceptance and human visual acceptance. Do not
restart a running authoring instance until its drafts are safe.

## Native dialog parity audit (14 October 2026)

The native editor now retains draft baselines independently of catalog refreshes.
Close/Escape, a new channel/assignment, or another board-map selection cannot
silently replace edited values. The shared square-close confirmation supports
Cancel/keep editing and explicit Discard; failed saves remain dirty. Save only
marks the submitted draft clean after ACK. Settings ACK updates the shared
catalog preference baseline immediately instead of leaving Save enabled until
the next refresh. Removing a learned button, host rule or board action requires
explicit confirmation. Pending operations disable editors and replacement; live
connection/capability/learning guards disable unavailable board mutations.

Saved waveform groups keep semantic identity across catalog reordering. Each
gesture has a separately reachable square Edit action and wrapped action summary;
internal rule names/execution details are expandable. Tabs wrap at narrow widths,
the title reserves trailing close/refresh geometry, and board editing appears only
after selecting a learned code. Field labels/transmit input use shared control
metrics and centered single-line styling. Advertised board behavior labels and
renamable channel names are authoritative; a same-code/different-bits/protocol
board mapping no longer produces a false duplicate-action warning. Native owned
copy and receiving tooltips use the existing English/Persian translations.

Incomplete/non-RF tuples do not produce phantom known buttons. A changed host
instance or board generation cannot inherit another session's partial learned
catalog or last-seen timestamps. Observing a replacement generation invalidates
the old learned entry sample. A changed catalog owner/generation resets the packet
baseline to that owner's current cumulative counts, not its earlier history.
Learning defaults, waveform grouping, and refresh-preserved session counters are
unchanged; no new RPC or firmware schema is introduced. The indefinite learning
view no longer schedules an otherwise idle 250 ms repaint loop; push activity and
bounded receiving-expiry repaint still update its indicators.

Verification: focused native RF model/UI fixtures, all five tabs at 360/480/980
pixels, square actions at four densities/three fonts/three scales, pending/failed
save, Close/Escape, complete tuple and owner/generation regressions. Fixtures use
the memory-only engine and a parent-isolated configuration, never a live RF send,
learn, map or actuator command. These are source/layout tests, not deployed-window
visual or physical acceptance. The Cafe-compatible package and deployment receipt
are recorded on the owning PR; replacement requires fresh draft-safe approval.
