# Shared messages and toasts

Pealayer owns one transient message service, `pealayer.messages.v1`. Native egui and Web/PWA render the same active set; native dismissal or web dismissal changes that set for every subscriber. IPC, HTTP, JSON commands and JSON-RPC/WebSocket use the existing unified application dispatcher. This is not a second OSD engine: existing OSD/status commands keep their behavior.

## Publish

Native IPC / command-line:

```powershell
& "$env:LOCALAPPDATA\Programs\Pealayer\bin\pealayer.exe" --remote 'toast Hardware ready'
& "$env:LOCALAPPDATA\Programs\Pealayer\bin\pealayer.exe" --remote '{"command":"publish_toast","id":"job.42","title":"Recording","message":"Effect saved","severity":"success","timeout_ms":5000}'
```

HTTP: `POST /api/messages` with JSON body:

```json
{"id":"job.42","title":"Recording","message":"Effect saved","severity":"success","timeout_ms":5000}
```

JSON-RPC via `POST /api/rpc`, `/api/ipc`, native IPC or `/ws`:

```json
{"jsonrpc":"2.0","id":42,"method":"pealayer.toast.show","params":{"id":"job.42","message":"Effect saved","severity":"success","timeout_ms":5000}}
```

Native application components use `InteropCommand::PublishToast`, or `messaging::publish(request, source)` followed by a repaint. The latter is the shared Rust service, not a separate native-only queue. Sources are assigned by the host dispatcher. Web commands still obey Web control permissions and existing request authentication.

## Read and dismiss

- `GET /api/messages`: current message snapshot.
- `/ws`: existing player-state broadcasts include `messages` alongside other live state. Changes trigger a state publication without waiting for the ordinary sync interval (when live sync is enabled).
- `pealayer.messages.state`: read-only JSON-RPC status response with the same `messages` member, also available through `pealayer --remote '<JSON-RPC request>'`.
- Dismiss: `{"command":"dismiss_toast","id":"job.42"}` or JSON-RPC `pealayer.toast.dismiss`, params `{"id":"job.42"}`. Close buttons use the same host service.
- Discover the contract in `GET /api/player/commands`.

The acknowledgement means queued/accepted, not synchronously painted. Observe the snapshot ID/revision to verify application. A future TUI can subscribe to `/ws`, replace its active set from `messages`, format `severity`, `title` and `message`, and publish/dismiss through these same commands. No standalone Pealayer TUI is claimed here.

## Lifetime and limits

Severity: `info` (default), `success`, `warning`, `error`. Timeout defaults to 5000 ms; allowed 500–300000 ms, or 0 for a message retained until dismissed. Text: 1–2048 characters; optional title: up to 120. IDs are optional (host generates UUIDs) or 1–128 ASCII letters/digits/`._-`; reusing an ID updates rather than duplicates a toast.

The service retains at most 32 active messages, oldest evicted first; interfaces show the four newest, with older active messages available in the snapshot. Text is plain, never executable HTML. Each process restart has a new `instance_id`, plus a revision counter. Messages are not persisted to disk/browser storage or replayed as history, and the Web UI suppresses stale notices while disconnected. Long messages scroll inside bounded cards. No synthetic startup/test notices are shipped.

Toasts are in-app notifications, not OS notification-center alerts. OS system notifications and OSD remain independent existing features.
