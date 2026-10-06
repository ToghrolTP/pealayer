# Remote Pealayer sessions

Pealayer can operate as an authority or a consumer. The authority owns the
player, preferences, workspace, timeline, history and PCController connection.
A consumer renders the same native controls but forwards mutations to that
authority. It does not discover PCController, open local media, or execute a
second hardware timeline. Application/font/DLL resources still come from the
installed program; mutable consumer storage is limited to caches.

## Connect

Enable Web UI and **Allow control** and **Allow configuration access** on the
authority. File retrieval/browsing/download permission is enabled by default.
Select the appropriate listening interfaces in its Web preferences.

Run the installed executable:

    Pealayer.exe --connect pealayer://SERVER:8080 --client-port 8081

The positional form works too:

    Pealayer.exe pealayer://SERVER:8080 --client-port 8081

File → Connect to Pealayer opens an endpoint/port dialog and launches a consumer
window. Each local consumer port has its own single-instance IPC identity.
The consumer's http://127.0.0.1:8081 serves the bundled SPA and relays its API,
RPC, HTTP IPC and WebSocket commands upstream. Its WebSocket state messages
contain the authoritative status, not the consumer decoder's guessed state.
Native window close closes the consumer; a remote Quit command quits the
authority. Window placement/fullscreen commands in native controls affect that
consumer's window, while session commands affect the authority.

## Media and server files

For a server-local file the preview uses GET /api/fs/file?path=URL_ENCODED_PATH.
This streams a regular file through bounded buffers and implements HEAD,
single byte ranges (including suffix/open-ended ranges), 206/416, Content-Length,
Content-Range, Accept-Ranges, MIME, download disposition, Last-Modified, weak
ETag and conditional requests. Unsupported multipart ranges are ignored and
served as a complete 200 response, as permitted by HTTP. Movies are not loaded
into memory. The client identifies requests with a Pealayer User-Agent.

HTTP(S)/RTSP/RTMP media URLs are opened directly by the consumer. A decoder,
network or permission failure disables only its preview, not remote controls.
Revoking file access hides the preview source but does not prevent connecting.
File → Open and project/config import/export use a server-files dialog, not a
consumer-native picker. Typed paths are server paths. Configuration exports
and project saves execute on the authority.

## Synchronization and safety

GET /api/peer/session provides typed configuration, live hardware capabilities,
timeline, media source, pause/rate and a libmpv clock sample. It also reports
active consumer identities. Consumers sample at 10 Hz and estimate transport
delay from round-trip measurements. The preview follows pause, rate and
position, correcting drift above 80 ms with exact seeks at most every 500 ms.
This is measured synchronization, not a claim of zero network latency or
sample-exact distributed playback. Hardware scheduling runs only on the
authority and therefore does not depend on the consumer's frame rate.

GET /api/client/status on the consumer exposes link age, round-trip time,
estimated server position, decoded position and preview drift.

Commands are not retried after an ambiguous acknowledgement. Queued mutations
expire after 500 ms and controls fail closed after two seconds without a fresh
snapshot. Live hardware intents reuse latest-value coalescing by stable channel
key; E-STOP remains enforced by the authority/PCController contract. Timeline
replacement checks the previously observed timeline and rejects conflicting
edits instead of overwriting newer work. Configuration Save waits for the
authority's actual disk-save/application acknowledgement.

Preferences currently reuse the embedded editor in consumer mode so a native
helper cannot accidentally edit a local profile. Server window geometry and
egui scroll/window memory are preserved rather than overwritten by consumers.

## Access boundaries

These APIs follow existing Web control/configuration/file/update permissions.
The built-in server does not add authentication or transport encryption to plain
HTTP. Expose it only on trusted networks or behind an authenticated TLS proxy.
An HTTPS origin can be supplied instead of pealayer://. Routing headers reject
loops. File access is broad host-file access: disable it where inappropriate.

Validation evidence and deployment results are recorded separately; compilation
alone does not establish two-host playback or physical hardware correctness.
