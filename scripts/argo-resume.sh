#!/usr/bin/env bash
# scripts/argo-resume.sh
#
# Atomically resume the SKYCOT worker swarm. Clears BOTH halt sentinels:
#
#   /tmp/argo-stop   transient halt
#   ~/.argo/stop     persistent halt
#
# The supervisor's next 30s ThrottleInterval cycle will see no sentinel
# and successfully launch slot subshells. Workers should be back to
# claiming briefs within ~1-2 minutes.
#
# Usage:
#   bash scripts/argo-resume.sh

set -euo pipefail

STOP_TMP="/tmp/argo-stop"
STOP_HOME="$HOME/.argo/stop"

removed_any=0
if [ -e "$STOP_TMP" ]; then
  rm -f "$STOP_TMP"
  echo "Cleared $STOP_TMP"
  removed_any=1
fi
if [ -e "$STOP_HOME" ]; then
  rm -f "$STOP_HOME"
  echo "Cleared $STOP_HOME"
  removed_any=1
fi

if [ "$removed_any" -eq 0 ]; then
  echo "No halt sentinels present — swarm was already running."
else
  echo ""
  echo "Swarm resumed — next launchd respawn (within 30s) will launch slots."
fi

supervisor_pid=$(launchctl list 2>/dev/null | awk '/com\.skycot\.worker-supervisor/ {print $1}')
worker_count=$(pgrep -fc claude-worker-loop.sh 2>/dev/null || echo 0)
echo ""
echo "Current state:"
echo "  supervisor: ${supervisor_pid:-(not running — check launchctl list)}"
echo "  workers:    $worker_count active claude-worker-loop process(es)"
echo ""
echo "Tail the supervisor log to watch slots come up:"
echo "  tail -f /tmp/supervisor.log"
