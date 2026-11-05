# Drop assets into your workspace

Pealayer uses the same importer from native file drops, the Web application's
**Import assets** button and its IPC/RPC commands. Existing library, playback
and designer controls remain the owners of the imported data.

| Drop | Result |
| --- | --- |
| Audio onto Timeline | Probe and save an SFX library entry, then place its cue at the pointer's exact time on the Audio effects lane. Pause first. |
| Audio onto Effects Library | Add an SFX entry. Reuse it by dragging its library card into Timeline. |
| Audio onto the sound-effect source field | Change that draft's source only. Save reads its duration using the existing SFX engine. |
| Video or audio onto Player | Open it with the existing player; additional video files enter its playlist. |
| Subtitle | Attach and select it using the existing subtitle track controls. A movie and its subtitles may be dropped together. |
| Effect definition JSON / `effects.json` catalog | Review definitions individually, then open the existing Effect properties designer as a new copy. Save/Publish there, then drag its saved library card into Timeline. Hardware programs stay owned by PCController. |
| Scene JSON | Review sampled channel values, then add set-and-keep cues on their mapped relay/PWM channels. This does not run the scene or invent values for unsampled channels. |
| Timeline JSON | Review, then append its cues, guides and curves at the drop position. Existing media, playhead, viewport and track policies remain unchanged. |

## Arrange and reuse

Opening a different movie still respects unsaved timeline work. Native and Web
show Save/Discard/Cancel rather than replacing it. Subtitles from a mixed drop
wait for that decision and the requested movie's FileLoaded event; Cancel removes
only those pending attachments. Additional playlist files must be dropped again
after confirming a pending media switch. Automation can explicitly choose via
`pealayer.timeline.resolve_pending` with `decision: save`, `discard` or `cancel`;
the importer never makes that decision for the user. Saving without a local
media sidecar uses the existing separate timeline/project save workflow.

Timeline audio drops use the pointed time, not the playhead or nearest keyframe.
A sound remains on the Audio effects lane even when dropped over another row;
an imported scene/timeline retains its own channel mapping. Locked destinations
are rejected. Curves for the same PWM channel merge without changing that
channel's existing policies; a colliding point time rejects the import instead
of silently overwriting it. Timeline import is one undoable edit.

JSON files use the current PCController effect descriptor/catalog, editor save
payload or Pealayer scene/timeline document
formats. The existing timeline Save format is also accepted. Project/media/
workspace restoration is deliberately not triggered by dropping a file: that
would replace the current session, not add an asset. Project restore remains a
separate workflow. For JSON with relative audio dependencies, include those
audio files in the same upload; uploaded siblings retain their filenames.
Catalog imports bring effect definitions and group membership into the designer;
they do not automatically overwrite group icons or create empty groups. Retain
the source catalog for PCController's own full-library import if those are needed.

## Browser and remote clients

Browser and native consumer files are transferred to the session owner in
ordered, bounded chunks. Client-local paths are never interpreted as paths on
the server. Transfers check the owner identity, show progress in Web and can be
cancelled there. A failed batch removes its registered temporary/completed
uploads that have not been retained by an import or a source field. Successful
uploads are durable authoring inputs, not expendable thumbnail cache files.

Limits: 64 files per request, 8 GiB per transferred file, eight concurrent active
file transfers and 32 MiB per JSON document. Filename traversal, Windows device
names, chunk overrun/reordering and incomplete finalization are rejected. Web
imports require file access, control and configuration permissions. A drop does
not publish an effect, acquire hardware authority or execute a scene.

## API

`pealayer.assets.import` takes `paths` and `destination`:

```json
{
  "paths": ["/authoring/sound.wav"],
  "destination": {
    "surface": "timeline",
    "start_time_ms": 10429,
    "track_key": "controller-effect:audio"
  }
}
```

Paths belong to the process receiving a direct IPC request. Use the upload API
for browser files or call the native consumer, which transfers its local files.
Upload receipts carry `owner_id`; include it in the final import request.
Other destinations are `player`, `library` and draft-correlated `audio-source`.
`pealayer.assets.review` takes the queued review's UUID and `accept: true/false`.
The shared status/WebSocket snapshot includes `asset_imports`; `/api/assets`
returns that same review queue. `/api/player/commands` advertises the contract.

Upload endpoints under `/api/assets/upload`: `POST /begin`, ordered `PUT /chunk`,
`POST /finish`, `DELETE` for a file, `DELETE /batch` for an unretained batch,
and `POST /retain` for an accepted draft source. Chunk/finish/cancel URLs include
the received owner identity. Command acceptance means queued; actual probe,
mapping and locked-track failures are reported through the existing error/toast
system. Do not retry a lost final command blindly.
