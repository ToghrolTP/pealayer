# Media names, title preferences and transport direction

Owner: [#291](https://github.com/ToghrolTP/pealayer/pull/291), continuing
[#80](https://github.com/ToghrolTP/pealayer/issues/80).

## Presentation and source identity

- Local and remote window titles default to the filename, including extension.
  Preferences → Appearance → Window title has independent Local media title and
  Remote media title selectors: Filename, Filename without extension, Full location.
  The existing live configuration/Preferences API uses
  `local_media_title_format` and `remote_media_title_format`, with values
  `file_name`, `file_stem`, `location`. Applying or previewing configuration uses
  the ordinary runtime reconciler; no restart or alternate title writer is needed.
- Remote presentation decodes URL path bytes and HTML entities once. Literal
  plus signs remain plus signs; an encoded percent stays percent rather than
  being recursively decoded. Control/bidirectional-override characters are
  excluded. Local literal percent/entity filenames are not URL-decoded.
- File-menu and video-menu Recents share one renderer. Both show filenames and
  a globe for remote sources; Open Location history uses the same naming helper.
  Full-location hints are decoded and userinfo-redacted. Root streams fall back
  to their host, not a signed query or credentials.
- Playback, recent-history identity, Copy location and URL-edit actions retain
  the exact original encoded source. Display text is never a playback URL or a
  directory-scope identifier. No URL input normalization rewrites signed sources.
- Native OS media-session fallback titles share the decoded filename helper;
  factual embedded title/artist/album tags still take precedence.
- Status/WS publishes separate `media_display_name` and `media_location_display`
  presentation fields for Web controls and Media Inspector. `current_video`
  remains the encoded, credential-redacted source identity.

## Seek direction

The existing icon constants and commands were already different; that alone did
not satisfy the reported visual result. The native shared transport now paints
explicit mirrored left/right vector triangles, with a terminal bar for frame
stepping. This removes mixed-font/icon fallback and RTL ordering from directional
geometry. Simple and Program Monitor retain their stable buttons, reorder/input
ownership, contextual crossfade and disabled opacity. Both native nudge renderers
reuse the same caption painter; command direction and seek/frame amounts do not
change. Web already has distinct directional icons.

## Verification and remaining gates

The complete isolated serial native suite passes: **1,366 passed, 0 failed,
6 opt-in ignored**. TypeScript, Web media controls, control-bar and OS media
session checks pass. The parallel run passed all new checks but hit the existing
global asset-preview fixture collision (empty shared review snapshot); that test
passed alone and in the complete serial run. This is not recorded as a green
parallel run. Tests
cover decoded Unicode/entities, malformed escapes/URLs, credential redaction,
literal local names, percent/plus preservation, independent saved title choices,
shared Preferences options, actual Recent-menu text and direction-specific vector
paint across scale/direction/mode.

Source tests are not installed-build or screenshot acceptance. The preceding
package was refused while source was being edited; packaging must start from the
next clean committed checkpoint, with no concurrent edits. Existing runtime,
review sessions, private media and historical comparison remain preserved.
Cafe deployment remains explicitly user-deferred/offline.
