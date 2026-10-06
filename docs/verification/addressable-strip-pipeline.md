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

## Live evidence

On the production board, the deployed #601 Windows artifact first sustained a
100-pixel rainbow at 20 FPS for 30 seconds while Pealayer's normal media/status
traffic remained connected. The final acceptance run then originated from
Pealayer through the deployed #602 host and sustained the same rainbow for 40
seconds:

| Counter | Before | After |
| --- | ---: | ---: |
| Framing errors | 58 | 58 |
| CRC errors | 0 | 0 |
| Reset count | 475 | 475 |

Throughout the Pealayer-originated acceptance run, the strip status remained
`running`, the active operation remained `rainbow 100 LEDs at 20 FPS`, Pealayer
remained connected to both PCController and the board, and `/healthz` continued
to return HTTP 200. The live framing, CRC, and reset counters did not increase.
