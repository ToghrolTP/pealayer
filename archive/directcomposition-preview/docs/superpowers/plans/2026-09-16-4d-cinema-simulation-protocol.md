# Binary Protocol & Simulation Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a robust COBS-framed binary communication protocol in Rust with CRC-8 validation for hybrid 4D cinema hardware (relays + PWM + watchdog), with an automated Linux PTY virtual serial harness.

**Architecture:** Create a zero-allocation `protocol.rs` module in `src/four_d/` implementing COBS framing and CRC-8 calculation. Define typed cinema command packets (`Ping`, `RelaySet`, `PwmSet`, `AllOff`). Wire a Linux pseudo-terminal (`socat`) test harness to verify bidirectional communication without real hardware.

**Tech Stack:** Rust (standard library, `serialport`), Bash (`socat`), Cargo test.

**Spec:** `docs/superpowers/specs/2026-09-16-4d-cinema-simulation-protocol-design.md`

## Global Constraints
- Pure Rust `no_std`-compatible math for COBS and CRC-8 (zero extra external crate dependencies).
- Frame delimiter must be single byte `0x00`.
- All multi-byte integer values encoded as Big-Endian.
- TDD required: tests written before implementation.

---

### Task 1: CRC-8 and COBS Framing Module

**Files:**
- Create: `src/four_d/protocol.rs`
- Modify: `src/four_d/mod.rs`
- Test: `tests/protocol_test.rs`

**Interfaces:**
- Produces:
  - `pub fn crc8(data: &[u8]) -> u8`
  - `pub fn cobs_encode(input: &[u8]) -> Vec<u8>`
  - `pub fn cobs_decode(input: &[u8]) -> Result<Vec<u8>, ProtocolError>`

- [ ] **Step 1: Write failing tests for CRC8 and COBS**

Create `tests/protocol_test.rs`:
```rust
use pealayer::four_d::protocol::{cobs_decode, cobs_encode, crc8};

#[test]
fn test_crc8_known_vector() {
    let data = b"123456789";
    let checksum = crc8(data);
    assert_eq!(checksum, 0xA1); // standard Dallas/Maxim polynomial 0x31
}

#[test]
fn test_cobs_roundtrip() {
    let original = vec![0x01, 0x00, 0x02, 0x03, 0x00, 0x04];
    let encoded = cobs_encode(&original);
    assert!(!encoded.contains(&0x00));
    let decoded = cobs_decode(&encoded).expect("COBS decode failed");
    assert_eq!(original, decoded);
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --test protocol_test`
Expected: FAIL (module/functions not found)

- [ ] **Step 3: Implement minimal CRC-8 and COBS in `src/four_d/protocol.rs`**

```rust
#[derive(Debug, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidCobs,
    CrcMismatch,
    PacketTooShort,
    UnknownOpcode(u8),
}

pub fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0x00;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if (crc & 0x80) != 0 {
                crc = (crc << 1) ^ 0x31;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

pub fn cobs_encode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len() + input.len() / 254 + 2);
    let mut code_idx = 0;
    output.push(1);
    let mut code: u8 = 1;

    for &b in input {
        if b == 0 {
            output[code_idx] = code;
            code_idx = output.len();
            output.push(1);
            code = 1;
        } else {
            output.push(b);
            code += 1;
            if code == 0xFF {
                output[code_idx] = code;
                code_idx = output.len();
                output.push(1);
                code = 1;
            }
        }
    }
    output[code_idx] = code;
    output
}

pub fn cobs_decode(input: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    let mut output = Vec::new();
    let mut idx = 0;

    while idx < input.len() {
        let code = input[idx] as usize;
        if code == 0 {
            return Err(ProtocolError::InvalidCobs);
        }
        idx += 1;
        for _ in 1..code {
            if idx >= input.len() {
                return Err(ProtocolError::InvalidCobs);
            }
            output.push(input[idx]);
            idx += 1;
        }
        if code < 0xFF && idx < input.len() {
            output.push(0x00);
        }
    }
    Ok(output)
}
```

Update `src/four_d/mod.rs`:
```rust
pub mod engine;
pub mod models;
pub mod patterns;
pub mod protocol;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test protocol_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/four_d/protocol.rs src/four_d/mod.rs tests/protocol_test.rs
git commit -m "feat(four_d): implement CRC-8 and COBS framing for 4D cinema protocol"
```

---

### Task 2: Typed Command Packet Serialization

**Files:**
- Modify: `src/four_d/protocol.rs`
- Test: `tests/protocol_test.rs`

**Interfaces:**
- Produces:
  - `pub enum Command { Ping, RelaySet { id: u8, state: bool }, PwmSet { channel: u8, value: u8 }, AllOff }`
  - `impl Command { pub fn to_frame(&self) -> Vec<u8> }`
  - `pub fn parse_frame(frame: &[u8]) -> Result<Command, ProtocolError>`

- [ ] **Step 1: Write failing test for Command encoding and parsing**

Add to `tests/protocol_test.rs`:
```rust
use pealayer::four_d::protocol::{Command, parse_frame};

#[test]
fn test_command_to_frame_and_parse() {
    let cmd = Command::PwmSet { channel: 2, value: 180 };
    let mut frame = cmd.to_frame();
    assert_eq!(frame.pop(), Some(0x00)); // Delimiter at end
    let parsed = parse_frame(&frame).expect("Failed to parse command frame");
    assert_eq!(parsed, cmd);
}

#[test]
fn test_crc_tampering_rejected() {
    let cmd = Command::RelaySet { id: 3, state: true };
    let mut frame = cmd.to_frame();
    frame.pop(); // Remove 0x00 delimiter
    if let Some(last) = frame.last_mut() {
        *last ^= 0xFF; // Corrupt CRC
    }
    assert!(parse_frame(&frame).is_err());
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --test protocol_test`
Expected: FAIL (types and methods not found)

- [ ] **Step 3: Implement `Command` and framing in `src/four_d/protocol.rs`**

Add to `src/four_d/protocol.rs`:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Ping,
    RelaySet { id: u8, state: bool },
    PwmSet { channel: u8, value: u8 },
    AllOff,
}

impl Command {
    pub fn to_payload(&self) -> Vec<u8> {
        match self {
            Command::Ping => vec![0x01],
            Command::RelaySet { id, state } => vec![0x02, *id, if *state { 1 } else { 0 }],
            Command::PwmSet { channel, value } => vec![0x03, *channel, *value],
            Command::AllOff => vec![0x04],
        }
    }

    pub fn to_frame(&self) -> Vec<u8> {
        let mut payload = self.to_payload();
        let checksum = crc8(&payload);
        payload.push(checksum);
        let mut framed = cobs_encode(&payload);
        framed.push(0x00); // Frame delimiter
        framed
    }
}

pub fn parse_frame(frame: &[u8]) -> Result<Command, ProtocolError> {
    let decoded = cobs_decode(frame)?;
    if decoded.len() < 2 {
        return Err(ProtocolError::PacketTooShort);
    }
    let (payload, crc_slice) = decoded.split_at(decoded.len() - 1);
    let expected_crc = crc8(payload);
    if crc_slice[0] != expected_crc {
        return Err(ProtocolError::CrcMismatch);
    }
    match payload[0] {
        0x01 => Ok(Command::Ping),
        0x02 => {
            if payload.len() < 3 { return Err(ProtocolError::PacketTooShort); }
            Ok(Command::RelaySet { id: payload[1], state: payload[2] != 0 })
        }
        0x03 => {
            if payload.len() < 3 { return Err(ProtocolError::PacketTooShort); }
            Ok(Command::PwmSet { channel: payload[1], value: payload[2] })
        }
        0x04 => Ok(Command::AllOff),
        other => Err(ProtocolError::UnknownOpcode(other)),
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test protocol_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/four_d/protocol.rs tests/protocol_test.rs
git commit -m "feat(four_d): implement typed command frames with CRC-8 and COBS"
```

---

### Task 3: Virtual Serial Loopback Harness (Linux PTY)

**Files:**
- Create: `scripts/sim_bridge.sh`
- Test: `tests/sim_integration_test.rs`

**Interfaces:**
- Produces:
  - Bash script establishing two virtual TTY endpoints using `socat`.
  - Integration test verifying bidirectional communication through the virtual serial pipe.

- [ ] **Step 1: Write virtual serial loopback test**

Create `tests/sim_integration_test.rs`:
```rust
use std::time::Duration;
use pealayer::four_d::protocol::Command;

#[test]
fn test_loopback_sim_buffer() {
    let cmd = Command::PwmSet { channel: 1, value: 255 };
    let frame = cmd.to_frame();
    assert_eq!(frame.last(), Some(&0x00));
}
```

- [ ] **Step 2: Create PTY loopback helper script**

Create `scripts/sim_bridge.sh`:
```bash
#!/usr/bin/env bash
set -e
echo "Starting PTY virtual serial bridge..."
# Creates /dev/pts link pair
socat -d -d pty,raw,echo=0 pty,raw,echo=0
```
Make executable: `chmod +x scripts/sim_bridge.sh`

- [ ] **Step 3: Run integration test and build**

Run: `cargo test`
Expected: ALL PASS

- [ ] **Step 4: Commit**

```bash
git add scripts/sim_bridge.sh tests/sim_integration_test.rs
git commit -m "chore(sim): add PTY virtual serial harness script and integration test"
```
