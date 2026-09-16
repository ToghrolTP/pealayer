#!/usr/bin/env bash
# ==============================================================================
# Virtual Serial Loopback Harness (Linux PTY) for Pealayer 4D Cinema Simulation
#
# Usage:
#   ./scripts/sim_bridge.sh
#
# Description:
#   Creates a bidirectional virtual serial port pair using `socat`.
#   `socat` allocates two pseudo-terminals (PTYs), typically /dev/pts/X and /dev/pts/Y,
#   and establishes a raw transparent loopback tunnel between them without echo.
#
# Endpoints:
#   - Endpoint 1 (e.g. /dev/pts/1): Attach Pealayer serial driver (configured in UI or config)
#   - Endpoint 2 (e.g. /dev/pts/2): Attach VirtualBoard or mock hardware responder
#
# Example Workflow:
#   1. Run this script in a dedicated terminal window:
#        ./scripts/sim_bridge.sh
#   2. Note the two PTY device paths printed in the socat logs, for example:
#        2026/09/16 20:30:00 socat[12345] N PTY is /dev/pts/2
#        2026/09/16 20:30:00 socat[12345] N PTY is /dev/pts/3
#   3. In Pealayer, set serial port to /dev/pts/2.
#   4. Run your mock hardware responder / VirtualBoard on /dev/pts/3.
#   5. All binary COBS frames sent by Pealayer will be received by the responder,
#      and status replies will be forwarded back to Pealayer.
#
# Requirements:
#   socat (install via: sudo apt-get install socat  # Debian/Ubuntu)
#                      (sudo dnf install socat      # Fedora/RHEL)
#                      (sudo pacman -S socat        # Arch Linux)
#                      (brew install socat          # macOS)
# ==============================================================================

set -euo pipefail

# Check if socat is installed on the host system
if ! command -v socat &>/dev/null; then
    echo "==================================================================" >&2
    echo "Error: 'socat' command was not found in PATH." >&2
    echo "socat is required to create virtual serial (PTY) loopback pairs." >&2
    echo "" >&2
    echo "To install socat:" >&2
    echo "  Debian/Ubuntu: sudo apt-get install -y socat" >&2
    echo "  Fedora/RHEL:   sudo dnf install -y socat" >&2
    echo "  Arch Linux:    sudo pacman -S --needed socat" >&2
    echo "  openSUSE:      sudo zypper install socat" >&2
    echo "  macOS:         brew install socat" >&2
    echo "==================================================================" >&2
    exit 1
fi

echo "=================================================================="
echo "Pealayer 4D Simulation Bridge"
echo "Modes:"
echo "  1) PTY <-> PTY (default):      ./scripts/sim_bridge.sh"
echo "  2) PTY <-> TCP (VirtualBoard): ./scripts/sim_bridge.sh --tcp [PORT]"
echo "  3) Spawn VirtualBoard + Bridge: ./scripts/sim_bridge.sh --spawn [PORT]"
echo "=================================================================="

MODE="${1:-pty}"
PORT="${2:-8765}"
VB_BIN="scratch/PCController/Tools/VirtualBoard/.build/release/bin/virtual_board"

if [ "$MODE" = "--spawn" ] || [ "$MODE" = "spawn" ]; then
    if [ ! -f "$VB_BIN" ]; then
        echo "Error: VirtualBoard binary not found at $VB_BIN" >&2
        echo "Please build it with cmake/ninja first." >&2
        exit 1
    fi
    echo "Spawning VirtualBoard on 127.0.0.1:$PORT..."
    "$VB_BIN" --bind 127.0.0.1 --port "$PORT" --quiet &
    VB_PID=$!
    trap 'echo "Stopping VirtualBoard (PID $VB_PID)..."; kill "$VB_PID" 2>/dev/null || true' EXIT INT TERM
    sleep 0.2
    echo "Bridging PTY <-> TCP 127.0.0.1:$PORT..."
    exec socat -d -d pty,raw,echo=0 tcp:127.0.0.1:"$PORT"
elif [ "$MODE" = "--tcp" ] || [ "$MODE" = "tcp" ]; then
    echo "Bridging PTY <-> TCP 127.0.0.1:$PORT..."
    exec socat -d -d pty,raw,echo=0 tcp:127.0.0.1:"$PORT"
else
    echo "Starting raw PTY <-> PTY loopback pair..."
    exec socat -d -d pty,raw,echo=0 pty,raw,echo=0
fi

