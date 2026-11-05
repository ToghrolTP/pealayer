# Strip designer

## Start with a small design

1. In Effects Library, create a Lighting effect. Choose **Segmented design** if the draft is empty.
2. Set the pixel count, frame rate and duration. The connected host advertises its limits. Density (pixels per metre) describes the installation; it does not change pixel addresses.
3. Select pixels for a segment. All, Clear, Invert and Reverse order help with non-contiguous layouts. The displayed numbers start at 1; stored indices start at 0.
4. Add layers targeting those segments. Choose Solid, Gradient, Fade, Blink / Strobe or Breathing; edit colors, cycle, phase, duty cycle, easing and direction as applicable.
5. Preview and scrub locally. **Offline preview never drives hardware.** Later enabled layers overwrite earlier layers where their segments overlap; unselected pixels remain black.
6. **Publish** saves to the connected PCController host. **Publish & Run** saves first and runs only after acknowledgement. Stop ends the host stream. Publishing requires the host, but not a connected board; running requires an available board and respects E-STOP.
7. Drag the published library effect onto the Lighting effects timeline track, or use its existing Add cue action. The cue references `effect:<id>` and retains its exact start and active duration. Repeating strip windows are resizable; the host prepares frames before playback and clears the strip at the cue end.

The editor exists in native Effects Designer and the Web Effects Library/Studio. Each uses the same save/run command path. Only the working draft is retained by Pealayer; it is not a second published library. A malformed technical JSON draft stays intact and cannot be published or run.

## Examples and diagnostics

Rainbow is the only built-in strip animation. **Add example effects** explicitly asks PCController to add Police red/blue, White thunder and Converging red dots as editable user effects. It does not overwrite existing effects. Fresh host configurations do not seed them; existing saved user definitions are deliberately preserved.

Hardware Monitor's Addressable lighting card provides Rainbow, configure pixel count, FPS, color order, stop, clear and refresh status. Color-order choices come from the host. Streaming counters show complete-frame acknowledgements and transfer failures, not measured physical LED refresh. Offline previews are illustrative, not proof that the board accepted a frame.

## Current shared program contract

The `layers` primitive contains `period_ms`, `pixels_per_meter`, ordered `segments` and ordered `layers`. Each segment has a stable `id`, display `name` and ordered, unique `pixels`. Each layer has a stable `id`, `name`, `segment_id`, `enabled`, `primitive`, RGB `primary`/`secondary`, `period_ms`, `phase_ms`, `duty_percent`, `easing` and `reverse`.

Limits: 1–100 pixels, 1–120 requested FPS, 100 ms–1 hour effect duration, 1–64 segments/layers, 1–1000 pixels/metre. Actual selectable hardware limits come from the catalog. Each cycle is 50 ms–1 hour, with phase below its period and duty 0–100%. Easing is linear, ease-in, ease-out or ease-in-out. Segment pixel order determines gradient direction. All three renderers (host Go, native Rust, Web TypeScript) have deterministic pattern/order/bounds tests.

API additions reuse the existing IPC/RPC/Web command router:

| Command | Purpose |
| --- | --- |
| `controller_effect.strip.edit` (`reference` optional) | Open the native strip editor without replacing an active draft or recording |
| `controller_effect.save` / `controller_effect.publish_and_run` | Existing authoritative publish / acknowledged publish-and-run |
| `controller_effect.restore_examples` | Explicit host-owned, non-overwriting example seeding |
| `hardware.strip.color_order` (`order`) | Set a host-advertised RGB component order |
| `hardware.strip.rainbow`, `.stop`, `.status`, `.configure` | Existing diagnostics and stream lifecycle |

## Acceptance and remaining gates

- Passed: Rust strip/model/cue/API tests, native editor width checks at 340/768/1200 points, TypeScript check, Web pattern/draft validation tests, Go host publish/prepare/render tests. Prepared fixture retains the 10,000 ms start and 11,000 ms stop.
- Verified locally: packaged native designer opened through IPC and captured with the read-only Win32 helper; canonical updater restart confirmed through API with the original libmpv retained. A follow-up pins the shared action bar outside the editor scroll area after that capture exposed its visibility issue.
- Pending: final action-bar capture, Web visual review, paired host/production deployment, physical color-order/segment/Rainbow tests and observed board timing. These are not implied by source tests.
- Standalone strip/Rainbow stream counters are implemented. Prepared timeline progress remains on the existing media-timeline acknowledgement/status contract; those steps do not increment the standalone counters. The concurrent RGB-input receiver has its own telemetry. Unifying their presentation is a follow-up, not a physical refresh-rate claim.
- Cafe's SSH endpoint refused the connection during this pass. Local installation used the graceful updater with explicit restart approval; a denied tool launch was completed by the user, not bypassed.
