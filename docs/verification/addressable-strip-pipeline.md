# Addressable strip pipeline

Pealayer deliberately controls the addressable strip through PCController. The
board remains a device endpoint rather than a second coordinator:

```text
Pealayer Hardware Monitor / JSON-RPC
  -> PCController controller.command.execute
  -> serialized serial request and correlated ACK
  -> COBS frame, opcode 0x16
  -> firmware stage/configure/commit handler
  -> AddressableLeds::show()
  -> physical WS281x strip
```

Direct board access is reserved for diagnostics and explicit standalone COBS
workflows. Normal application control must use PCController so its safety,
ownership, cancellation, telemetry, and output-state rules remain authoritative.

## Capability contract

Pealayer renders the strip section only when the connected board and coordinator
advertise a validated `strip` descriptor from `controller.peripherals.get`.
The descriptor defines pixel and FPS limits plus the supported modes. The UI
does not infer support from a command name or from the presence of an effect.

The current contract is:

| Field | Value |
| --- | --- |
| Pixels | 1–100; default 100 |
| Frame rate | 1–30 FPS; default 20 FPS |
| Modes | `solid`, `pixel`, `frame`, `rainbow`, `effect` |

Effect choices are likewise limited to PCController entries advertised as
strip streams. This keeps the UI synchronized with the live coordinator rather
than a duplicated static catalog.

## Commands

Pealayer exposes the same control surface to its native and Web applications and
to local automation through the single JSON-RPC endpoint on port 8080:

- `hardware.strip.configure`
- `hardware.strip.fill`
- `hardware.strip.pixel`
- `hardware.strip.frame`
- `hardware.strip.rainbow`
- `hardware.strip.effect`
- `hardware.strip.stop`
- `hardware.strip.status`
- `hardware.strip.clear`

The Hardware Monitor provides capability-driven mode selection, an LED preview,
pixel/FPS configuration, a live Idle/Streaming badge, an active-operation name,
explicit refresh, contextual Stop, and Clear.

## Transport failure and fix

The physical 100-pixel rainbow initially failed after several seconds with
`context deadline exceeded`. The board framing-error counter increased while
CRC and reset counters remained stable.

Firmware opcode `0x16` stages frame chunks and commits them through
`AddressableLeds::show()`. A 100-pixel WS281x commit masks AVR interrupts for
approximately 3 ms. PCController formerly released its serial write gate after
the write but before the correlated ACK, allowing concurrent status or media
clock requests to enter that interrupt-masked interval and corrupt a COBS frame.

[PCController PR #601](https://github.com/atomicdeploy/PCController/pull/601)
holds a request-wide gate from write through ACK. Its deterministic concurrency
test proves that a second request cannot write before the first acknowledgement.

[PCController PR #602](https://github.com/atomicdeploy/PCController/pull/602)
publishes the capability-gated `strip` descriptor that Pealayer requires and
migrates the former pre-1.0 macro mode `auto` to `host` during configuration
normalization.

[PCController PR #599](https://github.com/atomicdeploy/PCController/pull/599)
keeps a host-rendered strip stream alive across transient request deadline
misses. A missed frame is deliberately discarded instead of queued: the next
frame is rendered from monotonic elapsed time, so the animation remains current
and no stale colors accumulate behind the serial link. Disconnects, protocol
errors, cancellations, and all other non-timeout failures still terminate the
stream and remain visible to clients. Commit `e9991da8` also advances past all
overdue frame boundaries after a slow ACK; it does not consume a buffered timer
tick and immediately burst another full frame onto the link.

## Live evidence

On the production board, the deployed #601 Windows artifact first sustained a
100-pixel rainbow at 20 FPS for 30 seconds while Pealayer's normal media/status
traffic remained connected. After #602 established the capability contract,
the final acceptance run originated from Pealayer through PCController #599
commit `5ad4d7af` and sustained the same rainbow for 100 seconds:

| Counter | Before | After |
| --- | ---: | ---: |
| Framing errors | 233 | 283 |
| CRC errors | 0 | 0 |
| Reset count | 475 | 475 |

Throughout the Pealayer-originated acceptance run, the strip status remained
`running`, the active operation remained `rainbow 100 LEDs at 20 FPS`, Pealayer
remained connected to both PCController and the board, and `/healthz` continued
to return HTTP 200. Six clusters of request deadline misses were observed; each
reported a dropped stale frame followed by recovery, without stopping the
animation or resetting the board.

The increase in framing errors is a real residual transport limitation at the
maximum tested load, not a successful no-error result. CRC and reset counters
remained stable, and request-wide serialization plus timeout recovery prevented
it from becoming an application failure. A future transport optimization should
reduce request overhead or further prioritize strip commits, but it must retain
PCController as the sole driver and must not queue late animation frames.

After deploying the overdue-boundary fix, a follow-up 40-second run stayed live
at every 10-second observation. Framing errors increased from 459 to 473, while
CRC errors remained 0 and reset count remained 475. This is an improvement in
burst behavior but not elimination of the physical-link framing limitation.
Immediately after restarting PCController, the first asynchronous Pealayer RPC
was accepted before the refreshed event channel could start the stream; a second
request after reconnection started normally. Clients should treat the live strip
status, not command queue acceptance alone, as the operation confirmation.
