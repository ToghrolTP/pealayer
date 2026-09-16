# 4D Cinema Simulation & Protocol Specification (Phase 1)

## Goal
Implement a robust binary serial protocol (COBS framing + CRC-8 checksum) in Pealayer for hybrid 4D cinema hardware (relays + PWM curves + heartbeat watchdog), and establish a virtual hardware loopback testbed using Linux PTYs.

## Architecture & Interfaces
- **Framing**: Consistent Overhead Byte Stuffing (COBS). Zero byte (`0x00`) acts as unambiguous frame delimiter.
- **Integrity**: CRC-8 appended to each payload before COBS encoding.
- **Message Types**:
  - `0x01 Ping`: 50ms heartbeat watchdog.
  - `0x02 RelaySet`: `[relay_id: u8, state: u8]` (1-8, 0/1).
  - `0x03 PwmSet`: `[channel: u8, duty: u8]` (1-4, 0-255).
  - `0x04 AllOff`: Global hardware kill (Seek / Pause / E-STOP).
  - `0x80 Status`: `[relay_mask: u8, pwm_1: u8, pwm_2: u8, pwm_3: u8, pwm_4: u8]`.

## File Structure
- `src/four_d/protocol.rs`: Low-level COBS framing, CRC-8, and strongly-typed packet codecs.
- `src/four_d/mod.rs`: Exposes `protocol` module.
- `tests/protocol_test.rs`: Unit tests for codecs, corruption rejection, and roundtrips.
- `scripts/sim_bridge.sh`: Sets up virtual serial PTY loopback pair (`/dev/pts/X` <-> `/dev/pts/Y`) for automated simulation.
