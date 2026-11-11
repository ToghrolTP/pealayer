# Download Center appearance verification

The standalone Windows capture uses the clean source `41b3c0cb4812fb272b1a3dc9d1d2975a904ac924` and a separate empty queue. Its executable SHA-256 is `5b5f2dd9228746bca4e77ca23043b1088e72246e66fd83bf28f18b2e79427db7`.

The 1282 × 986 capture shows Pastel with bundled Inter. Inter owns some of the private-use code points Phosphor uses, so each icon has an explicit symbol font and each caption keeps the chosen UI font. The header, Queue and Settings tabs, engine dropdown, search, and empty state were inspected after the fix. The standalone headless smoke test and all 18 download-core, queue and transfer tests passed. The three opt-in tool fixtures exercised local aria2, yt-dlp, and FFmpeg.

## Shared native window and input styling

The standalone executable now uses the same theme colors, font resolution,
input sizing and focus styling as the player's Download Center. It follows
the saved appearance profile while open. On Windows, the shared DWM adapter
uses the active panel and text colors for the caption.

The isolated Windows review at source `a8624d16` clicked and typed into the URL
and queue search fields. Switching from light Pastel to dark Studio changed
the title bar and content together while the URL draft stayed focused.
The rendered test additionally crosses the compact-layout breakpoint while
editing. The fixture used a separate empty queue and a synthetic URL.

- [Light Pastel and the focused URL](../screenshots/downloader-shared-light-pastel.jpg)
- [Dark Studio after changing the saved profile](../screenshots/downloader-shared-dark-studio.jpg)
