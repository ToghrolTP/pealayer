# Integrated sequence capture

The separate Record effect card/form has been removed. The native sequence
editor, Web Effects Library editor and Web timeline editor use the current
effect's metadata and put capture controls beside sequence editing actions.

1. Open a sequence through Effects Library → New effect or Manage.
2. Set its name, group, icon and color using the existing identity controls.
3. Choose a capture clock and press Record. Current edits are published before
   capture starts; PCController acknowledges publication before appending.
4. Use the live board/application controls. Actions appear in the same sequence.
5. Finish updates that effect. Discard take removes only the new capture.
6. To overwrite, delete the existing steps (Web: Clear steps), then Record.

PCController remains the catalog owner. Capture retains the same ID and
metadata, appends at the authored end (durations/repeats/fades included), and
does not change the playback policy or insert a new timeline cue. Existing
timeline templates refresh after the authoritative recorded catalog arrives.
Concurrent edits/deletions are rejected without losing the take.

Software checks are scoped to append, discard, cleared replacement, strict
clock wrap, full authored span, overflow, conflict retention and board-retained
download-prefix preservation. Native screenshot capture remains unavailable:
Windows Graphics Capture fails with capture-service timeout 0x8007041D. Native
visual acceptance must not be claimed from Web screenshots or compiler checks.
