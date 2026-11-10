# Libmpv Teardown and Windows Test Isolation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the Windows GNU CI access violation (`0xc0000005`), eliminate worker thread and libmpv client leaks in test fixtures, and fix the sub-pixel layout assertion failure in `src/ui/layout.rs`.

**Architecture:**
- In `src/app.rs`, make `PealayerApp::default()` an entirely inert test fixture using `crate::four_d::engine::memory_only_test_engine()` and the shared test libmpv client (`TEST_MPV_CLIENT`), preventing worker threads and ephemeral libmpv client handles from leaking in both unit and integration tests.
- In `src/ui/layout.rs`, update `program_monitor_fits_its_scroll_viewport_at_all_sizes_and_scales` tolerance to allow 1 logical unit of physical pixel rounding at fractional scale factors.
- Verify full test suite locally under `PEALAYER_CONFIG_FILE` isolation, push to remote branch, and monitor Windows GNU CI on GitHub Actions via `gh`.

**Tech Stack:** Rust (2024 edition), libmpv2, egui / eframe, GitHub Actions (Windows GNU runner).

**Spec:** GitHub Issues #116 (libmpv teardown access violation and background thread cleanup) and #30 (Windows GNU readiness gate).

## Global Constraints
- Alpha contract: Maintain single current API and data contract; no legacy migrations or obsolete shims.
- Test isolation: Always provide an isolated `PEALAYER_CONFIG_FILE` before running tests; never point fixtures at OS config or Registry.
- Continuity: Never overwrite or discard unpushed work; push to tracked remote branch and record exact GitHub commit references and run receipts.
- Verification before completion: Evidence before assertions always. Do not claim CI passes until confirmed with `gh run view`.

---

### Task 1: Eliminate Worker Thread Leaks and Teardown Races in `PealayerApp::default()`

**Files:**
- Modify: `src/app.rs:9080-9145`

**Interfaces:**
- Consumes: `crate::four_d::engine::memory_only_test_engine()`
- Produces: Inert `PealayerApp::default()` fixture with zero background worker threads and stable shared `mpv_client`.

- [ ] **Step 1: Check existing `default_mpv_client` and `PealayerApp::default()` implementation**
- [ ] **Step 2: Update `default_mpv_client` to reuse `shared_test_mpv_client()` across all test contexts (including integration tests)**
- [ ] **Step 3: Update `PealayerApp::default()` to use `memory_only_test_engine()` and remove `attach_playback_clock`**
- [ ] **Step 4: Verify that `PealayerApp::default()` creates an inert engine without spawning threads**

---

### Task 2: Fix Fractional Scale Rounding Tolerance in `src/ui/layout.rs`

**Files:**
- Modify: `src/ui/layout.rs:8445-8455`

**Interfaces:**
- Consumes: `available_rect_before_wrap()`, `ScrollArea::show`
- Produces: Deterministic passing test for `program_monitor_fits_its_scroll_viewport_at_all_sizes_and_scales` across all scale factors.

- [ ] **Step 1: Inspect `scroll.content_size.y <= available.height() + (1.0 / scale).max(1.0)`**
- [ ] **Step 2: Edit `src/ui/layout.rs` line 8451 to use `(1.0 / scale).max(1.0)`**
- [ ] **Step 3: Run the test locally: `PEALAYER_CONFIG_FILE=/tmp/cfg.json cargo test --lib program_monitor_fits_its_scroll_viewport_at_all_sizes_and_scales`**

---

### Task 3: Comprehensive Local Test Suite Verification

**Files:**
- Test execution across workspace

- [ ] **Step 1: Run full unit test suite: `PEALAYER_CONFIG_FILE=/tmp/cfg.json cargo test --lib -- --test-threads=1`**
- [ ] **Step 2: Run all integration tests: `PEALAYER_CONFIG_FILE=/tmp/cfg.json cargo test --tests -- --test-threads=1`**
- [ ] **Step 3: Confirm 0 failures across all tests**

---

### Task 4: Git Commit, Push, and CI Monitoring

**Files:**
- Git repository branch `investigate/libmpv-teardown-access-violation`

- [ ] **Step 1: Check `git status` and `git diff` for clean, minimal changes**
- [ ] **Step 2: Commit with conventional commit message referencing #116 and #30**
- [ ] **Step 3: Push to `origin/investigate/libmpv-teardown-access-violation`**
- [ ] **Step 4: Monitor GitHub Actions run using `gh run list` and `gh run view`**
- [ ] **Step 5: Inspect Windows GNU job output and report results**
