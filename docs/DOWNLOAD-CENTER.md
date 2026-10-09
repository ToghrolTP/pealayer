# Download center and standalone utility

Delivery tracker: [Unified download center](https://github.com/ToghrolTP/pealayer/issues/110).

## Implemented checkpoint — not complete production acceptance

File → Downloads and the Web Downloads surface use one player-independent Rust
service in `crates/download-core`: persistent queue, bounded parallel files,
pause/resume/cancel/reorder, bandwidth limits, measured speed/ETA and charts.
Completed files can be opened explicitly in Pealayer. Unknown sizes never become
fake percentages. Rust advertises available actions and engines to both views.

The same view/service builds into `pealayer-downloader`, with a distinct executable,
Windows product/version metadata and generated multi-resolution ICO. It has no
libmpv dependency and does not launch the player merely by opening.

### Connected engines

- Built-in Rust: HTTP(S), validated byte resume and bandwidth pacing. Native
  segmented transfers remain unfinished; use aria2 for multiple connections.
- External local aria2: Engine settings validates its RPC endpoint and optional
  secret. Queue jobs support 1–16 connections, telemetry, pause/resume/removal,
  speed changes and verified output. Saved transfer IDs are paused on recovery.
  Lost control cannot silently remove an unclaimed transfer. Remote-host endpoints
  are rejected because their files are not this host's owned cache.
- yt-dlp: discovers tools next to the executable or on PATH, supervises a single
  media download, parses structured progress and validates finalized output.
  Format/playlist selection and extraction previews remain unfinished.
- FFmpeg: completed files expose queued Remux MP4 and Extract audio. Remux copies
  streams without overwriting; extraction produces AAC/M4A. Processing resumes
  from the start after pause, unlike HTTP byte resume. Full probe/transcode/filter
  workflows remain unfinished.

Commands do not use a shell or arbitrary caller switches. Both output streams
have bounded buffers. Windows Job Objects/Unix process groups own cancellation.
Shutdown pauses jobs before exit. yt-dlp bandwidth changes require pause/resume
so the actual applied limit stays honest.

Primary contracts: [aria2](https://aria2.github.io/manual/en/html/aria2c.html),
[yt-dlp](https://github.com/yt-dlp/yt-dlp#usage-and-options),
[FFmpeg](https://ffmpeg.org/ffmpeg.html).

### Transfer, storage and buffering safety

- HTTP(S) sources reject embedded credentials. Public snapshots exclude URL query
  and proxy secrets; private queue metadata retains the source needed for resume.
- Jobs own UUID directories and portable filenames, never arbitrary RPC output
  paths. Completion uses a non-overwriting hard-link commit.
- Resume requires a strong ETag or Last-Modified identity and exact Content-Range.
  Full responses restart instead of append; changed/truncated sources fail visibly.
- Restart replaces the partial-file inode, preserving an existing reader's source.
- Direct mode disables environmental proxies; explicit proxy mode honors configured
  or environmental proxy settings. Requests identify Pealayer Downloader.
- Queue changes are persisted, not every chunk. A process-level lock prevents
  conflicting ownership. Interrupted jobs reopen paused. Charts retain 120 samples;
  workers are capped at eight, jobs at 512 and metadata at four MiB.
- Removing a row preserves its saved data. Explicit cache deletion, quotas,
  eviction, normalized-source deduplication and retention remain unfinished.
- `cache::read_range` reads cached contiguous bytes and fetches gaps using origin
  ranges with the identical validator. Sparse or unknown bytes are never fabricated.
  Missing-range playback traffic is separate from download bandwidth and does not
  yet populate a persistent piece cache.
- `GET/HEAD /api/downloads/media?id=…` serves ranges behind existing origin,
  peer-routing and file-access permissions. Complete streaming launch UX and live
  long media playback acceptance remains required. Isolated HTTP acceptance now
  verifies paused cached-prefix/origin-gap bytes, completed GET/HEAD and 416 bounds.

### Shared contracts

Existing native IPC, HTTP `/api/rpc` and WebSocket JSON-RPC:

| Method | Parameters |
| --- | --- |
| `pealayer.downloads.list` | Empty object |
| `pealayer.downloads.add` | `url`; optional `filename`, `use_proxy`, `proxy_url`, `engine`, `connections` |
| `pealayer.downloads.action` | `id`, `action`: pause, resume, cancel, remove, move_up, move_down, remux_mp4, extract_audio |
| `pealayer.downloads.configure` | `max_concurrent` (1–8), `bytes_per_second` (0 = unlimited) |
| `pealayer.downloads.engines.configure` | `aria2_endpoint`, `aria2_secret` |

Web reads require file access; mutations also require control. Peer RPC forwards
to the authority. Native peer mode currently directs users to its Web Downloads
surface, not a second local queue. The standalone utility reports conflicting
ownership rather than stealing it.

## Verification and remaining delivery gates

Sixteen core/local-HTTP/native-widget tests pass: exact payload, pause/resume, ignored Range
restart, changed identity, interrupted body, cancellation, exclusive ownership and
cached-prefix/origin-range correctness, the rendered Resume button's backend
action and recovery from a lost engine-connection reply. Three explicitly invoked installed-engine
tests pass with synthetic local media: aria2, yt-dlp and FFmpeg remux/extraction.
An intermittent yt-dlp fixture failure appeared during one concurrent run and was
not reproduced in subsequent isolated/combined runs; repeated acceptance remains
a gate, not a claim that this intermittent condition is solved.

Main all-target compilation and the complete Web/PWA build pass. An isolated
released backend passed RPC add/pause/resume/cancel plus exact progressive and
completed media HTTP delivery. Web Add/Pause reached the same queue. Rendered
desktop (1280), tablet (768) and phone (390) layouts have no horizontal overflow.
A read-only Win32 capture confirms the standalone window restores the same
completed, cancelled and paused jobs without stealing another process's queue.
Native narrow-window/manual input acceptance, disk-failure and crash-recovery
stress, measured bandwidth, long playback and production deployment remain open.

Visual acceptance caught version-stamped lazy CSS being classified as JavaScript
by Vite's suffix-based preload helper. All surfaces now ship their scoped styles
in the initial application stylesheet, with a build regression gate. Both views
reserve speed-chart geometry before samples arrive instead of shifting rows.

The Windows packager now builds and includes `pealayer-downloader.exe`, verifies
its distinct resources, headless UI/dependency smoke and clean source identity,
and records it in the host manifest. The player and utility share one build
metadata emitter. `--smoke-test` never opens a queue or starts downloads;
`--build-info` never starts the UI. Final package/deployment verification is
coordinated after merge from clean exact main, not a development-worktree build.

Reproduce the focused live HTTP acceptance against a fresh isolated backend:

```text
node scripts/download-fixture-server.mjs <synthetic-media-file>
node scripts/verify-download-api.mjs <loopback-backend-url> <printed-fixture-url>
```

Use dedicated `PEALAYER_CONFIG_FILE`, `PEALAYER_DOWNLOAD_ROOT`, loopback Web port,
and disabled hardware auto-connect/session restore/media keys. Do not run the
fixture against a user's queue. The generated media, private queue and screenshots
belong in canonical staging, never the repository.

Full request completion additionally requires owned/bundled-tool launch/provenance,
cache policy and piece storage, streaming launch, format/playlist/transcode features,
bulk/context/keyboard controls, localization, standalone authority attachment/RPC,
combined-mode launch and cross-platform release packaging. These remain tracked in #110.

Build on the faster host or CI, never production. Publish source before deployment,
use graceful IPC/peer update and destination-validated libmpv. Issue #110 stays open
until these capabilities and live acceptance are delivered.
