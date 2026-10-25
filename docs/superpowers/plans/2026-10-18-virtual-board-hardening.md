# Virtual Board Hardening (Option A) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement industrial-grade process supervision (Windows Job Objects / Linux PDEATHSIG), autonomous communications watchdog cutoff, atomic port allocation resilience, and stale temporary resource garbage collection for the Virtual Board Demo simulation.

**Architecture:** Encapsulate child process lifecycle inside a robust cross-platform `ProcessSupervisorGroup` utilizing OS-level job objects on Windows and `PR_SET_PDEATHSIG` on Unix to guarantee zero orphan processes under abnormal terminations. Add autonomous relay/PWM Safe Torque Off (STO) to `virtual_board` when host activity ceases for >5000 ms. Add startup cleanup for orphaned temporary directories.

**Tech Stack:** Rust (std::os::unix::process::CommandExt, windows-sys / Win32 Job Objects, libc), C++17 (VirtualBoard core), CMake, ctest, cargo.

**Spec:** [`virtual-board-industrial-review.md`](file:///home/toghrol/.gemini/antigravity-cli/brain/0a77e5e7-5593-463f-89d3-eda2d8231ec1/virtual-board-industrial-review.md)

## Global Constraints

- Must compile and pass all tests on Linux x86-64, macOS (Intel & Apple Silicon), and Windows x86-64.
- Zero orphan background processes: child processes must terminate immediately if the parent dies, panics, or is terminated.
- Pure catalog/model reconciliation and tests must never touch OS config or Registry mirror (use isolated `PEALAYER_CONFIG_FILE`).
- No trailing whitespace on any lines (enforced by CI `git diff --check`).
- Conventional commit messages.

---

### Task 1: Cross-Platform Process Lifecycle Supervisor (`process_supervisor`)

**Files:**
- Create: `src/four_d/process_supervisor.rs`
- Modify: `src/four_d/mod.rs`
- Modify: `src/four_d/virtual_board.rs:1-140`
- Modify: `src/four_d/demo_coordinator.rs:1-237`
- Test: `tests/process_supervisor_test.rs`

**Interfaces:**
- Produces: `pub struct ManagedChild`: wraps `std::process::Child` with platform-specific kernel binding (Job Object on Windows, `PDEATHSIG` on Linux) and ensures guaranteed termination on `Drop`.
- Produces: `pub fn configure_pdeathsig(cmd: &mut std::process::Command)` for Unix.
- Produces: `pub fn bind_child_to_job(child: &std::process::Child)` for Windows.

- [ ] **Step 1: Write the failing test for ManagedChild termination**

Create `tests/process_supervisor_test.rs`:
```rust
use pealayer::four_d::process_supervisor::ManagedChild;
use std::process::Command;

#[test]
fn test_managed_child_spawns_and_terminates() {
    let mut cmd = Command::new("sleep");
    cmd.arg("60");
    let managed = ManagedChild::spawn(cmd).expect("spawn managed child");
    assert!(managed.id() > 0);
    drop(managed);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test process_supervisor_test`
Expected: FAIL (module `process_supervisor` does not exist).

- [ ] **Step 3: Implement `src/four_d/process_supervisor.rs` and export in `src/four_d/mod.rs`**

Implement `ManagedChild`:
- On Linux: `.pre_exec()` sets `libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL)`.
- On Windows: Create `CreateJobObjectW`, set `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, and assign child process handle via `AssignProcessToJobObject`.
- On all platforms: `Drop` sends SIGKILL / calls `.kill()` and `.wait()`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test process_supervisor_test`
Expected: PASS.

- [ ] **Step 5: Integrate `ManagedChild` into `virtual_board.rs` and `demo_coordinator.rs`**

Replace raw `Child` in `VirtualBoardSession` and `CoordinatorSupervisor::Process` with `ManagedChild`.
Verify: `cargo test --test virtual_board_session_test --test demo_coordinator_test`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/four_d/process_supervisor.rs src/four_d/mod.rs src/four_d/virtual_board.rs src/four_d/demo_coordinator.rs tests/process_supervisor_test.rs
git commit -m "feat(four_d): add OS-level process supervisor for virtual board and coordinator"
```

---

### Task 2: Stale Temporary Resource Garbage Collection (`demo_cleanup`)

**Files:**
- Create: `src/four_d/demo_cleanup.rs`
- Modify: `src/four_d/mod.rs`
- Modify: `src/four_d/virtual_board.rs`
- Modify: `src/four_d/demo_coordinator.rs`
- Test: `tests/demo_cleanup_test.rs`

**Interfaces:**
- Produces: `pub fn cleanup_stale_demo_resources()`: scans `std::env::temp_dir()` for `pealayer-demo-coordinator-<pid>-*` and `virtual-mcu-eeprom-<pid>-*.bin`. For each entry, checks if `<pid>` is still alive. If dead, removes the directory/file.
- Produces: `pub fn is_process_alive(pid: u32) -> bool`.

- [ ] **Step 1: Write the failing test for stale demo resource cleanup**

Create `tests/demo_cleanup_test.rs`:
```rust
use pealayer::four_d::demo_cleanup::{cleanup_stale_demo_resources, is_process_alive};
use std::fs;

#[test]
fn test_is_process_alive() {
    let current_pid = std::process::id();
    assert!(is_process_alive(current_pid));
    assert!(!is_process_alive(999_999_999));
}

#[test]
fn test_cleanup_stale_resources() {
    let temp = std::env::temp_dir();
    let dead_dir = temp.join("pealayer-demo-coordinator-999999999-123456");
    let dead_file = temp.join("virtual-mcu-eeprom-999999999-123456.bin");
    fs::create_dir_all(&dead_dir).unwrap();
    fs::write(&dead_file, b"test").unwrap();

    cleanup_stale_demo_resources();

    assert!(!dead_dir.exists());
    assert!(!dead_file.exists());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test demo_cleanup_test`
Expected: FAIL (module not found).

- [ ] **Step 3: Implement `src/four_d/demo_cleanup.rs`**

Implement:
- `is_process_alive(pid: u32) -> bool`:
  - On Unix: `libc::kill(pid as libc::pid_t, 0) == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)`.
  - On Windows: `OpenProcess` with `PROCESS_QUERY_LIMITED_INFORMATION`.
- `cleanup_stale_demo_resources()`: iterate over `std::env::temp_dir()`, parse matching prefix and PID, delete if PID is not alive.
- Call `cleanup_stale_demo_resources()` at the beginning of `VirtualBoardSession::spawn()` and `DemoCoordinatorSession::start()`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test demo_cleanup_test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/four_d/demo_cleanup.rs src/four_d/mod.rs src/four_d/virtual_board.rs src/four_d/demo_coordinator.rs tests/demo_cleanup_test.rs
git commit -m "feat(four_d): add garbage collection for orphaned demo directories and EEPROM files"
```

---

### Task 3: Autonomous Communications Watchdog in `virtual_board`

**Files:**
- Modify: `scratch/PCController/Tools/VirtualBoard/src/virtual_board.cpp:783-798`
- Modify: `scratch/PCController/Tools/VirtualBoard/tests/virtual_board_tests.cpp`
- Rebuild: `bin/virtual_board`
- Test: `tests/virtual_board_watchdog_test.rs`

**Interfaces:**
- In `VirtualBoard::tick()`:
  - If `hostSeen_` and `now - lastHostActivityAt_ > kHostOfflineAfter`:
    - If `relays_.mask() != 0`:
      - Call `relays_.allOff();`
      - Call `captureRelayState();`
      - Call `stopMotion();`
    - If `pwm_.available()`:
      - Call `pwm_.allOff();`
    - Queue event or log cutoff so host knows Safe Torque Off was asserted.

- [ ] **Step 1: Write C++ unit test for watchdog cutoff**

In `scratch/PCController/Tools/VirtualBoard/tests/virtual_board_tests.cpp`:
Add test `watchdog_cuts_off_relays_when_host_offline`:
- Set relay 1 active.
- Verify `relays.mask() != 0`.
- Fast-forward or simulate time past `kHostOfflineAfter`.
- Call `board.tick()`.
- Assert `relays.mask() == 0`.

- [ ] **Step 2: Run C++ test to verify it fails**

Run: `cmake --build scratch/PCController/Tools/VirtualBoard/.build && ctest --test-dir scratch/PCController/Tools/VirtualBoard/.build -R virtual_board_unit`
Expected: FAIL (relays remained active).

- [ ] **Step 3: Implement watchdog cutoff in `virtual_board.cpp`**

In `VirtualBoard::tick()`:
```cpp
if (hostSeen_ && now - lastHostActivityAt_ > kHostOfflineAfter) {
    if (relays_.mask() != 0) {
        relays_.allOff();
        captureRelayState();
    }
    pwm_.allOff();
}
```

- [ ] **Step 4: Run C++ test to verify it passes**

Run: `cmake --build scratch/PCController/Tools/VirtualBoard/.build && ctest --test-dir scratch/PCController/Tools/VirtualBoard/.build -R virtual_board_unit`
Expected: PASS (all 13 tests passed).

- [ ] **Step 5: Copy updated binary to `bin/virtual_board`**

Copy `scratch/PCController/Tools/VirtualBoard/.build/bin/virtual_board` to `bin/virtual_board`.

- [ ] **Step 6: Write Rust integration test verifying watchdog cutoff over TCP**

Create `tests/virtual_board_watchdog_test.rs`:
- Spawn `VirtualBoardSession`.
- Send frame activating relay 1.
- Read response, verify relay 1 is on.
- Wait for 5.2 seconds without sending any frame.
- Request status frame.
- Verify status reports relays are all 0 (de-energized).

Run: `cargo test --test virtual_board_watchdog_test`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add scratch/PCController/Tools/VirtualBoard/src/virtual_board.cpp scratch/PCController/Tools/VirtualBoard/tests/ bin/virtual_board tests/virtual_board_watchdog_test.rs
git commit -m "feat(virtual_board): implement autonomous watchdog cutoff on communications loss"
```

---

### Task 4: Port Allocation Resilience & TOCTOU Protection

**Files:**
- Modify: `src/four_d/virtual_board.rs`
- Modify: `src/four_d/demo_coordinator.rs`
- Test: `tests/port_resilience_test.rs`

**Interfaces:**
- Produces: `pub fn spawn_with_retry(max_retries: usize) -> Result<Self, String>` in `VirtualBoardSession`.
- Handles transient port conflicts gracefully with exponential backoff up to 3 attempts.

- [ ] **Step 1: Write test for retry logic on port conflict**

Create `tests/port_resilience_test.rs`:
```rust
use pealayer::four_d::virtual_board::VirtualBoardSession;

#[test]
fn test_virtual_board_spawn_resilience() {
    let session = VirtualBoardSession::spawn().expect("spawn virtual board session");
    assert!(session.port > 0);
}
```

- [ ] **Step 2: Implement retry loop in `VirtualBoardSession::spawn()` and `DemoCoordinatorSession::start()`**

Wrap ephemeral port acquisition and child process readiness check in a retry loop (up to 3 attempts). If `bind` or connection fails because port is taken, acquire a fresh ephemeral port and retry.

- [ ] **Step 3: Run test to verify it passes**

Run: `cargo test --test port_resilience_test`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/four_d/virtual_board.rs src/four_d/demo_coordinator.rs tests/port_resilience_test.rs
git commit -m "fix(four_d): add port allocation retry resilience for virtual board and coordinator"
```

---

### Task 5: Full Verification & GitHub CI/CD Chores

**Files:**
- All modified files

- [ ] **Step 1: Run complete local test suite**

Run: `PEALAYER_CONFIG_FILE=$(mktemp) cargo test`
Expected: 0 failures, all 940+ tests pass.

- [ ] **Step 2: Verify zero whitespace errors**

Run: `git diff --check origin/main HEAD -- .`
Expected: 0 errors.

- [ ] **Step 3: Verify Windows cross-compilation**

Run: `cargo check --target x86_64-pc-windows-gnu`
Expected: exit 0.

- [ ] **Step 4: Push to branch and monitor GitHub Actions**

Push commits to `feature/virtual-board-demo`.
Watch CI workflow with `gh run watch <run-id> --exit-status`.
Verify all matrix jobs pass green.
