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
