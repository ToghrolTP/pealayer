# Playback and authoring

## Opening media

Use **File → Open Video File**, **Open Location / URL**, command-line targets, or
native drag and drop. Multiple dropped media files form a deterministic queue.
Subtitle files are attached to the player; timeline JSON is loaded as a project.

Recent entries are actual history and remain visually distinct from currently
available files. Pealayer does not populate example paths.

## Subtitles and audio

Audio and subtitle selectors are populated from libmpv tracks. External audio
and subtitle files can be loaded from their settings dialogs. The bundled
Vazirmatn fallback supports Persian and Arabic subtitle shaping.

## Experience timeline

The docked authoring workspace contains the program monitor, effect controls,
effects library, hardware monitor, and timeline. The effects library is derived
from compatible macros advertised by PCController. A macro that cannot be
represented faithfully as a relay effect remains factual catalog information
and is not silently converted.

Edits update the execution queue through the same engine used by playback.
Locked tracks reject placement. E-STOP pauses playback and disables hardware
output immediately.
