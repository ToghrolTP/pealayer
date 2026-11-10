# Download Center appearance verification

The standalone Windows capture uses the clean source `41b3c0cb4812fb272b1a3dc9d1d2975a904ac924` and a separate empty queue. Its executable SHA-256 is `5b5f2dd9228746bca4e77ca23043b1088e72246e66fd83bf28f18b2e79427db7`.

The 1282 × 986 capture shows Pastel with bundled Inter. Inter owns some of the private-use code points Phosphor uses, so each icon has an explicit symbol font and each caption keeps the chosen UI font. The header, Queue and Settings tabs, engine dropdown, search, and empty state were inspected after the fix. The standalone headless smoke test and all 18 download-core, queue and transfer tests passed. The three opt-in tool fixtures exercised local aria2, yt-dlp, and FFmpeg.
