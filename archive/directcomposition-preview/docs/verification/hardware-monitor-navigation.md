# Hardware Monitor navigation and channel dragging

## Cause and repair

- The dock deliberately disables its own Hardware Monitor scroll bars, but the panel did not supply an internal scroll area. Channel cards therefore extended past the available viewport. The panel now owns a bounded vertical scroll area beneath its fixed dashboard header.
- Hardware Monitor cards and Manage channels rows shared drag payload and source-rectangle IDs for each channel. Rendering both surfaces overwrote the grab geometry; panel release cleanup also consumed modal drags before the manager could complete its drop. Drag payloads, source rectangles and handle animation are now scoped by origin surface and native viewport. Primary-button release alone completes or cancels a channel drag.
- Existing panel card rendering, pointer-offset calculation, ordering persistence and live control commands are retained. Each surface reorders its own channels without transforming the other copy of the channel.

## Focused regression checks

Run from the canonical Pealayer checkout:

```powershell
cargo test --lib --locked --jobs 1 hardware_ -- --test-threads=1
```

Result: **33 passed, 0 failed**, including existing channel actions, card width, light-mode, manager layout and hardware-contract checks. New checks exercise real egui pointer/wheel events:

- `hardware_monitor_scroll_reaches_lower_cards`: wheel input over actual channel cards changes the bounded panel's vertical offset.
- `hardware_drag_manager_coordinates_and_release_are_not_owned_by_monitor`: different card/row geometries remain independent, and panel cleanup cannot swallow a manager drop.
- `hardware_drag_pointer_start_and_drop_work_in_each_surface`: primary-button press, movement and release initiate and complete channel reordering from both the panel and manager, retaining each origin's own grab offset.

Full test-suite execution is not required for packaging this focused repair. Native screenshot acceptance remains separate: Windows Graphics Capture returned `0x8007041D` in this session, and the current native window provided no accessibility tree. Automated interaction checks are not represented as screenshots or physical acceptance.

## Manual acceptance

1. Open Hardware Monitor with enough advertised channels to exceed the sidebar height. Wheel-scroll over ordinary cards and use the scrollbar to reach lower sections; the dashboard header should remain available.
2. Drag a channel by its handle onto a compatible channel in Hardware Monitor. Confirm the existing grab behavior and persisted order.
3. Open Manage channels while the panel remains visible. Drag a row by its handle onto another compatible row. The panel's duplicate card must not move, the row must retain its own grab geometry, and releasing must persist its order.
4. Repeat with a detached native manager window and cancel a drag over empty space. Right click must remain a context-menu gesture rather than completing a primary-button drag.

## Deployment checkpoint

- Code commit: `ebe4c98558ed1c3f4d9dec8d01b59e627750b607`, pushed to draft PR #46; not merged.
- Canonical packaging: `scripts/package-windows.ps1 -SkipTests -NoUpx` completed, including TypeScript/Vite and PWA validation. Packaging did not run the full suite; the 33 focused checks above ran separately.
- Previous canonical process accepted `{"command":"quit"}` through `/api/ipc` and exited before replacement. New canonical executable launched as PID `39132` on DAVID-PC.
- Executable: `%LOCALAPPDATA%\Programs\Pealayer\bin\pealayer.exe`, SHA-256 `353ea366070af2a1f800824238648e9cdfc3388c717b5c64969fd7b3dfc307d7`.
- Live `/api/update/manifest` matched the code commit and hash with `git_dirty: false`; `/healthz` returned `ok`; `/api/player/status` reported `hardware_connected: true`. The native window restored the previous media title.
- Native capture retry after launch still failed with Windows Graphics Capture `0x8007041D`; no screenshot or manual mouse acceptance is claimed.
- Cafe-PC's `http://cafe-pc:8080/healthz` timed out. No Cafe deployment is claimed; use Pealayer's peer updater when the receiver returns.
