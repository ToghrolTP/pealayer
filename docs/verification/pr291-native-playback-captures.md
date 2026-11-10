# Native playback and Remote Location captures

These images show the isolated Windows build at source commit `e1a1367e6e93bd618d31424ccb9bdcc3839619e2`. The running process reported a clean source tree and executable SHA-256 `4d9a30a0fdbe9265fa5d7dc6cb81601b95cc0cec20b04297510ae634016b4eb4`. This evidence commit adds captures only.

- Simple playback used a generated, muted 24-second gradient clip with the transport pinned. Play advanced the elapsed position and Pause settled at 8.708 seconds. The Simple menu and status bar were hidden.
- The Remote Location grid used a loopback HTTP fixture with a verified parent, one directory and three generated clips. The application decoded all three thumbnails. Item Properties reported the same filename, media type, size, date and URL as the verified listing.
- The captures remove the transparent window margins while retaining the actual app pixels. Each image is 1275 × 794. No production settings, media or hardware were involved.

| Capture | SHA-256 |
| --- | --- |
| [Simple playback transport](../screenshots/pr291-simple-transport.png) | fd1678043561e624c9919a9a0a09c9b5949c40a49dd30e151e08011f04336308 |
| [Remote Location grid and decoded thumbnails](../screenshots/pr291-remote-grid.png) | fe6953750b2fca2503b685f89dc475c452562778d11cd57f56e0c7897a3bf1f8 |
| [Remote Location item properties](../screenshots/pr291-remote-properties.png) | f4980b6f15bb8fa69c4707811158c0feb352f85c90d408daefb6ce8eec6d1329 |
