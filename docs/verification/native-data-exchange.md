# Native file feedback, clipboard and attention

Related tracking: [Windows file exchange](https://github.com/ToghrolTP/pealayer/issues/30),
[Win32 integration](https://github.com/ToghrolTP/pealayer/issues/34),
[acceptance](https://github.com/ToghrolTP/pealayer/issues/80).

## Implemented paths

| User action | Result |
| --- | --- |
| Hover a movie over the player | Shell drop caption and application play icon/name |
| Hover a subtitle | Load-subtitle caption/icon, not a misleading video caption |
| Hover audio over player / library / source field | Play / Add SFX / Use audio source, respectively |
| Hover multiple files | Bounded asset-count caption; existing movie-before-subtitle import ordering |
| Paste Explorer files or file URI lists | Existing asset ingestion, pinned remote file upload where applicable; no source file deletion |
| Paste into timeline | SFX/document placement at playhead; movie/subtitle handling stays with the media owner |
| Paste HTTP(S) URL outside text fields | Existing verified remote-file/directory browser and proxy policy |
| Copy/Cut/Paste timeline cues | Exact relative integer timing, complete templates/values/references, fresh pasted IDs and one undo edit |
| Copy/Cut/Paste selected sequence cues | Existing sequence selection and repetition validation; context menu and toolbar actions |
| Copy/Cut/Paste strip layer | New layer identity, active segment target, complete layer values, atomic program validation |
| New important notification | Three taskbar flashes when backgrounded, at most one request per five seconds; activation stops flashing |

Clipboard reads occur on explicit Paste. Existing configurable copied-URL
observation remains a separate opt-in/default preference path; it does not paste
into fields or run cues. Ordinary focused text editors retain their clipboard
events. Modal backdrops do not become external asset drop targets.

## Windows ownership and limits

The main HWND owns an OLE `IDropTarget` and `IDropTargetHelper`. The generic
winit target is replaced only there; accepted files become normal egui hovered
and dropped events and reuse `assets` ingestion. The framework owns its remaining
window lifecycle. The target supplies `DROPDESCRIPTION`; Windows supplies the
source drag image and standard Copy decoration. Application overlays supply
semantic Phosphor icons. This is not a claim that Shell offers arbitrary custom
action glyphs through `DROPDESCRIPTION`.

Incoming `STGMEDIUM` objects are released, never `DragFinish`ed. Clipboard
`CF_HDROP` belongs to the clipboard and is never freed. File batches are bounded
to 64, paths to Windows length limits, authoring text to 8 MiB. Structured copies
publish Unicode text plus the registered `Pealayer.Authoring` format where
available; unrelated clipboard data is never tagged after a concurrent change.
Explorer Cut is interpreted as importing/copying, never moving/deleting files.

Cut removes cues only after a successful clipboard write and lock validation.
Paste uses existing timeline validation, lock checks, ID allocation and engine
commit. It does not seek or run a scene. Remote native cue Cut/Paste remains
explicitly guarded rather than silently mutating the consumer's local timeline;
cross-authority authoring/asset-reference transfer needs its own verified path.

Taskbar attention uses bounded `FlashWindowEx`, not foreground activation or
restoration. Important local and API/RPC-published warning/error toasts share
the same trigger. A toast expiry or unchanged status refresh is not a new event.
No physical buzzer is used for routine notification verification.

## Evidence and remaining acceptance

The first full native suite passed 1,151 tests, with two opt-in tests ignored.
This includes the real Windows Shell data-object test: an in-memory Unicode
`CF_HDROP`, a real `DROPDESCRIPTION`, COM target callbacks and exactly-once egui
delivery. It never touches the user's clipboard, media, window or hardware.
Final-source suite: **1,153 passed, zero failed, two opt-in tests ignored**.
Additional regressions cover Cut failure/lock atomicity, last-drop-point
retention, notification deduplication and strip validation. Existing strip editor
phone/tablet/desktop bounds regressions pass with the added controls. This does
not establish real Explorer or installed UI acceptance.

Still required before claiming complete acceptance:

- Real Explorer hover/drop, ordinary text-field shortcuts and cue Cut/Paste in
  the installed app; Shell image behavior can depend on the drag source.
- Background warning delivered through the API: visible bounded taskbar flashing
  without focus theft; stop after activation.
- Cross-authority native cue clipboard relay and full browser clipboard parity.
- Host-compatible Cafe deployment remains user-deferred until its connection returns.

## Installed checkpoint

Clean package source: `84ea45d45f90f157b382b8e8733885eed0ef77f7`.
Executable SHA-256:
`6742a8d2daf654370f53db14789b5dcdcd9f155db732f217e047641f63925777`.
The retained host libmpv SHA-256 is
`e56ce67cd00f06a59dc7ed5b97a49a9182a381554c755dc4570a52af1ef30e65`.

The clean package passed Web compilation/PWA integrity, Windows resource and
seven Jump List icon checks, downloader identity/smoke and native libmpv smoke.
The optimized application build took 8m 09s, including rebuilt windowing
dependencies. Previously completed native tests were not repeated by packaging.
The graceful peer updater acknowledged the installed digest in 16.4s. Fresh
process-local status reports that source/digest and clean authoring state; the
regular app is paused with no media loaded after the user's normal close.

A one-time warning was accepted through the installed HTTP command path and
appeared in the message snapshot. Native window queries before and after both
reported backgrounded, non-minimized state: no focus/restore was performed.
Visible taskbar flashing still needs human confirmation, not API inference.

The historical executable is unchanged and running with its isolated profile
on 18092; the regular app owns 8080. Its direct executable launch had previously
used the system profile, causing the port collision. The tool policy blocked
moving its private config to its supported portable `pealayer.json`; no move
was performed or retried through another path. The reviewed, parse-checked
`scripts/make-history-portable.ps1` is provided for the user to run after closing
only that historical app normally. It verifies executable identity/isolation,
refuses live processes or an existing destination, moves rather than duplicates
the profile, then launches the original executable. The comparison launcher
recognizes that portable profile when present. Permanent direct-launch isolation
is not claimed until the user completes it and the live port is verified.

Windows ownership follows Microsoft's [Shell data-object](https://learn.microsoft.com/en-us/windows/win32/shell/dataobject)
and [clipboard format](https://learn.microsoft.com/en-us/windows/win32/shell/clipboard)
contracts. Real Explorer and clipboard interaction, cross-authority/browser
clipboard parity and deferred Cafe delivery remain open.
