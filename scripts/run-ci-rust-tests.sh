#!/usr/bin/env bash
set -euo pipefail

# Keep native lifecycle failures diagnosable rather than rerunning blindly.
# Sampling is read-only and does not change the test result or kill a test.
if [[ "${RUNNER_OS:-}" == "macOS" ]]; then
  (
    for attempt in 1 2; do
      sleep 300
      echo "::group::Native test stack sample ${attempt}"
      # Match running test binaries, not cargo/rustc or this diagnostic shell.
      for native_pid in $(pgrep -f '/debug/deps/.*--test-threads=1' || true); do
        if [[ "$native_pid" =~ ^[0-9]+$ ]]; then
          sample "$native_pid" 1 1 -file /dev/stdout || true
        fi
      done
      echo "::endgroup::"
    done
  ) &
  diagnostic_pid=$!
  trap 'kill "$diagnostic_pid" 2>/dev/null || true' EXIT
fi

cargo test --locked -- --test-threads=1
