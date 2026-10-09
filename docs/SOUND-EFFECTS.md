# Sound effects and audio outputs

Sound effects (SFX) are audio files in the Effects Library. They play on the
Pealayer computer, not on the PCController buzzer. A board is not required.

## Start with one sound

1. In Effects Library choose **New audio effect** (speaker button in egui).
2. Enter a name, choose a group and icon, then **Browse** for a sound file or
   enter an HTTP(S) URL. WAV, MP3, FLAC, Ogg, AAC, M4A and Opus are supported
   when the installed libmpv can decode them.
3. Set volume and optionally select an output. **Save** reads the actual audio
   duration in the background; import failures appear as error messages.
4. **Preview** plays the saved sound without replacing or seeking the movie.
   **Stop** stops the preview. Closing the editor does not stop the movie.
5. Drag the library card onto **Audio effects** in the media timeline, or use
   **Add cue at playhead**. Move the cue to set its start. Its duration comes
   from the file and is deliberately not resizable.

The library persists in Pealayer's configuration. Placed cues persist with the
media session. Files remain at their original location; importing is not a
copy. Keep those files accessible, including after restarting the application.

## Choose an output

Preferences → Playback → Audio output contains two selectors:

- **Media output device / backend** defaults to **System default** (`auto`).
- **SFX output device / backend** defaults to **Same as media output**. Select
  another device to send all sounds to a separate output on the same PC.

Each sound also has an **Output device / backend** selector. The order is:
per-sound override → SFX preference → media preference → system default.
Backend-qualified device IDs come directly from libmpv; Pealayer does not
combine them with a conflicting, independently selected audio backend.
Outputs are refreshed when a selector opens and periodically. Unavailable
saved outputs stay selected instead of silently changing to another speaker.

## Playback and remote operation

Independent audio-only libmpv voices follow the movie's observed position,
pause/buffering, seek epoch and playback rate. Sounds are preloaded up to
400 ms before their start; drift corrections use exact audio seeks. E-STOP
stops previews and scheduled sounds. At most 16 voices may overlap; exceeding
the limit produces an error, not an unbounded decoder allocation.

This is software-clock synchronization, **not a sample-accurate guarantee**:
device latency, network buffering and decoding affect audible onset. Keep
timing-sensitive sounds on the authority computer's local storage.

The Web editor and native editor use the same Rust library, timeline and
commands. In remote-consumer mode, Browse lists the authority's files and
sound output/device selection happens on the authority PC. Consumers do not
open a second sound engine or write a duplicate library. Main media proxy
preferences also govern HTTP(S) sound imports/playback.

## Automation

Use `controller_effect.save` with `kind: "audio"`, a UUID `id`, name, category,
icon and `program: {"source":"...","volume":100,"output_device":""}`.
The import is asynchronous; an accepted command is not proof of a successful
decode. Wait for its `sfx:<UUID>` entry in `controller_effects`.

Existing `controller_effect.play`, `controller_effect.delete` and
`controller_effect_cue.add` accept that stable reference. `audio_effect.stop`
stops audio previews only; `audio.outputs.refresh` rediscovers outputs.
These commands share HTTP JSON-RPC, WebSocket and IPC dispatch. The player
status exposes `audio_devices`, `audio_import_pending`, `audio_preview_ids`
and actual `audio_voices` (decoder time, pause and active backend).
