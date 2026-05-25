#!/usr/bin/env bash
# scripts/argo-pause.sh
#
# Atomically pause the SKYCOT worker swarm. Sets BOTH halt sentinels:
#
#   /tmp/argo-stop   transient halt (wiped by macOS reboot)
#   ~/.argo/stop     persistent halt (survives reboot, added in a352b4bfb)
#
# Either one halts the supervisor's slot subshells (per stop_present() in
# scripts/worker-supervisor.sh and the matching check in
# scripts/claude-worker-loop.sh). Setting BOTH is robust against:
#
#   - macOS reboot wiping /tmp
#   - accidental `rm /tmp/argo-stop` from some other tool
#   - periodic /tmp cleanup utilities
#
# In-flight worker iterations finish their current brief first (the
# sentinel check happens between iterations, not mid-cargo). Hard kill
# would be `pkill -f claude-worker-loop` but that loses any uncommitted
# work — prefer the soft halt.
#
# Usage:
#   bash scripts/argo-pause.sh
#
# To resume: bash scripts/argo-resume.sh

set -euo pipefail

STOP_TMP="/tmp/argo-stop"
STOP_HOME="$HOME/.argo/stop"

mkdir -p "$HOME/.argo"
touch "$STOP_TMP" "$STOP_HOME"

# Snapshot the live state so the operator can see what they're pausing.
supervisor_pid=$(launchctl list 2>/dev/null | awk '/com\.skycot\.worker-supervisor/ {print $1}')
worker_count=$(pgrep -fc claude-worker-loop.sh 2>/dev/null || echo 0)

echo "Swarm paused — both halt sentinels set."
echo "  transient:  $STOP_TMP"
echo "  persistent: $STOP_HOME"
echo ""
echo "Current state:"
echo "  supervisor: ${supervisor_pid:-(not running)}"
echo "  workers:    $worker_count active claude-worker-loop process(es)"
echo ""
echo "In-flight iterations will finish their current brief, then halt at"
echo "the next sentinel check. The supervisor itself will keep cycling"
echo "(launchd KeepAlive) but slots will exit immediately on respawn."
echo ""
echo "To resume: bash scripts/argo-resume.sh"
