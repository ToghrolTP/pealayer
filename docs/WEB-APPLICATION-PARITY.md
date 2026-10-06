# Pealayer Web application parity ledger

GitHub tracking issue: [#63](https://github.com/ToghrolTP/pealayer/issues/63)

The Web surface is a first-class Pealayer application. It is not a narrated
status page and it must not recreate domain rules already owned by Rust, mpv,
or PCController. This ledger records the remaining differences against the
native egui application in product language rather than commit identifiers.

## Definition of parity

A surface is complete only when it uses the same authoritative state and
commands as egui, offers the same meaningful operations, handles keyboard,
pointer, touch and context-menu input, remains usable at phone through desktop
widths, and has been exercised against live state rather than sample data.

## Current product ledger

| Area | Current state | Completion evidence | Remaining work |
| --- | --- | --- | --- |
| Application shell | In progress | Installable SPA/PWA, responsive navigation, shared appearance and connection state | Finish application-style command header, footer/status items and compact/mobile command access |
| Playback surface | In progress | Browser-native video now uses Pealayer's seekable byte-range endpoint and follows the Rust/mpv clock; unsupported browser codecs fall back to the backend frame surface | Add negotiated low-latency transcoded canvas stream over WebSocket, browser capability reporting and stream diagnostics |
| Transport | Mostly complete | Play/pause, seek, volume, mute, rate, chapters, thumbnail preview and remote-folder previous/next use shared commands | Complete track selectors, frame-step state, contextual shortcuts and exact buffer visualization |
| Hardware monitor | In progress | Live advertised controls, press-time relay actions, clickable indicators, PWM, motion hold/release, drag ordering and complete card context menu | Unify custom icons with egui, bulk management, bindings, channel timeline actions and every advertised board setting |
| Emergency stop | Complete for Web interaction | Filled red danger control, immediate pointer-down dispatch and shared interlock state | Continue physical-board acceptance whenever hardware is attached |
| Front panel | In progress | Live seven-segment data and K1–K4 commands are sourced from PCController | Match the complete egui board-information experience, segment renderer, LCD/settings tabs and contextual commands |
| Effects library | In progress | Live PCController catalog, groups, create/manage/rename/play/delete, drag source and context menus | Finish group management, all custom picker behavior and visual parity at every responsive width |
| Effect editor and recording | In progress | Sequence editing and board/app recording share PCController effect contracts | Complete professional multi-lane editor, selection, easing, fades, repeat/blink authoring, offline drafts and conflict handling |
| Timeline | In progress | Web now receives and renders the native ordered track inventory, including media, effect and hardware lanes; cues move and conditionally resize | Port native selection, keyframes, M/S/L, track context menus, track routing, keyboard editing, snapping, vertical reordering and exact scroll/navigation behavior |
| Media library | In progress | Browse/play/thumbnails and native path requests work; Windows extended path prefixes are removed from breadcrumbs | Add complete file context menus, metadata, safe rename workflows and richer remote-folder parity |
| Preferences | In progress | Rust-generated preference contract drives the Web controls and appearance is synchronized | Ensure every native setting/control type and import/export workflow is represented and visually verified |
| Dialogs | In progress | Connection, remote location, channel management, RF management, effect editing and workspace management exist | Add full About, media/track properties, audio, subtitles, board information, update, bindings and remaining native dialogs without duplicating state logic |
| Messaging and OSD | Mostly complete | Shared toasts and the native configurable OSD contract render across Web, egui, HTTP, JSON-RPC and WebSocket | Complete icon-name coverage and visual acceptance for every custom anchor/color combination |
| Updates | In progress | URL update and truthful byte progress exist | Replace explanatory filler with contextual state/actions and complete peer/CI source selection parity |
| Footer/status bar | In progress | Web renders live connection transport, board, playback, update and active-surface state | Add the shared visibility/order contract plus hide/show context menus and transient messages matching egui |
| Responsive and accessibility | In progress | Automated phone/desktop layout contracts, modal height constraints and reduced-motion handling exist | Add screenshot interaction passes at phone, tablet, narrow desktop, desktop, light, dark, keyboard-only and touch sizes |
| Headless operation | In progress | Rust backend hosts the PWA, REST, WebSocket, IPC and range-serving media endpoint | Remove the native-window dependency for truly display-less startup and complete transcoded streaming fallback |

## Next acceptance passes

1. Finish the shared timeline command contract so Web track menus invoke the
   same select, link, show, mute, solo, lock and manage operations as egui.
2. Introduce a negotiated video pipeline: direct byte-range media when the
   browser supports it, otherwise a bounded low-latency encoded stream delivered
   to a canvas with clock and buffer feedback.
3. Render the shared OSD and configurable footer/status-bar models.
4. Complete the board-information and front-panel dialogs from advertised
   PCController capabilities—never hardcoded demo values.
5. Audit every egui modal against its Web renderer and close one measurable row
   at a time with dark/light and responsive evidence.

## Regression rules

- Hardware state changes occur on primary pointer-down; keyboard activation
  remains click/Enter/Space compatible and commands fire exactly once.
- Right click opens item-specific actions and never starts dragging.
- The Rust snapshot owns track identity, order, visibility and capability data.
- No synthetic board, effect, progress, media metadata or connection state.
- Persistent prose must be actionable; obvious update narration is omitted.
- A visually similar control is not parity unless it calls the same command and
  reacts to the same authoritative event stream.
