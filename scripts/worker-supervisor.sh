#!/usr/bin/env bash
# scripts/worker-supervisor.sh
#
# Keeps N claude-worker-loop.sh processes alive. When a worker exits
# (spin-loop guard, claude session ending, transient crash) the
# supervisor sleeps a short backoff then respawns it. Pool stays at
# target size without operator intervention.
#
# Usage:
#   bash scripts/worker-supervisor.sh             # default: 4 slots
#   bash scripts/worker-supervisor.sh 6           # 6 slots
#
# Recommended launch (detached from terminal so it survives tab close):
#   nohup bash scripts/worker-supervisor.sh 4 > /tmp/supervisor.log 2>&1 &
#   disown
#
# Halt:
#   touch /tmp/argo-stop                          # supervisor sees this, all slots finish current iter + halt
#   pkill -f worker-supervisor.sh                 # hard kill (current iters keep running until they finish)
#
# Exit-code interpretation (per claude-worker-loop.sh):
#   exit 0       — clean: spin-loop guard, signal trap, or normal end. Respawn after short delay.
#   exit 2       — ANTHROPIC_API_KEY hard guard. **DO NOT respawn** (real config error; operator must fix).
#   exit other   — unexpected. Respawn with backoff.
#
# Stop sentinel:
#   /tmp/argo-stop — if present, supervisor halts all slots cleanly.

set -euo pipefail

REPO="${REPO:-~/code/argo}"
N_SLOTS="${1:-4}"
WORKER_SCRIPT="$REPO/scripts/claude-worker-loop.sh"
STOP_SENTINEL="/tmp/argo-stop"
SUPERVISOR_PID_FILE="/tmp/argo-worker-supervisor.pid"

# Fail-fast guards
if [ ! -x "$WORKER_SCRIPT" ]; then
  echo "❌ worker script not executable: $WORKER_SCRIPT" >&2
  exit 1
fi
if [ -n "${ANTHROPIC_API_KEY:-}" ]; then
  echo "❌ supervisor refused: ANTHROPIC_API_KEY is set (would propagate to workers and trigger their hard guard)." >&2
  echo "   Comment it out in ~/.zshenv and re-open this terminal." >&2
  exit 2
fi
# Mirror the worker-loop's PATH guard so the supervisor itself refuses to
# start with the broken env that caused the 2026-05-23 doom-loop incident.
# Auto-repair the common case (PATH missing ~/.local/bin), hard-fail if
# still missing — children inherit our PATH, so getting it right here
# protects every slot.
if ! command -v claude >/dev/null 2>&1; then
  export PATH="$HOME/.local/bin:$PATH"
  if ! command -v claude >/dev/null 2>&1; then
    echo "❌ supervisor refused: 'claude' binary not on PATH (tried \$PATH + \$HOME/.local/bin)." >&2
    echo "   Re-launch from a fresh terminal so .zshrc reloads, or set PATH explicitly before launching." >&2
    exit 2
  fi
fi

# Single-supervisor lock — don't double-spawn.
if [ -f "$SUPERVISOR_PID_FILE" ] && kill -0 "$(cat "$SUPERVISOR_PID_FILE")" 2>/dev/null; then
  echo "❌ supervisor already running (pid $(cat "$SUPERVISOR_PID_FILE"))." >&2
  echo "   To restart: pkill -f worker-supervisor.sh && rm -f $SUPERVISOR_PID_FILE && bash scripts/worker-supervisor.sh $N_SLOTS" >&2
  exit 3
fi
echo "$$" > "$SUPERVISOR_PID_FILE"

# Clean exit: release the lock so a fresh supervisor can start.
trap 'rm -f "$SUPERVISOR_PID_FILE"; echo "[$(date +%H:%M:%S)] supervisor exiting (pid $$)"; exit 0' EXIT INT TERM HUP

echo "═══════════════════════════════════════════════════════════════════"
echo "  worker-supervisor.sh — keeping $N_SLOTS workers alive"
echo "  pid:        $$"
echo "  worker:     $WORKER_SCRIPT"
echo "  stop:       touch $STOP_SENTINEL"
echo "═══════════════════════════════════════════════════════════════════"

# One supervisor child per slot. Each child loops: launch worker, wait,
# decide on respawn based on exit code, sleep backoff, repeat.
for slot in $(seq 1 "$N_SLOTS"); do
  (
    consecutive_fast_fails=0
    LAST_LAUNCH_TS=0

    while true; do
      # Check stop sentinel.
      if [ -f "$STOP_SENTINEL" ]; then
        echo "[$(date +%H:%M:%S)] slot-$slot: STOP sentinel present, halting." >&2
        exit 0
      fi

      # Stagger initial launches by ~3s so all N slots don't race the
      # ledger at the same moment.
      if [ "$LAST_LAUNCH_TS" -eq 0 ]; then
        sleep $((slot * 3))
      fi

      LOG="/tmp/claude-worker-supervised-slot${slot}-$(date +%Y%m%dT%H%M%S).log"
      echo "[$(date +%H:%M:%S)] slot-$slot: launching worker → $LOG"
      LAUNCH_TS=$(date +%s)
      LAST_LAUNCH_TS=$LAUNCH_TS

      # Run the worker. Worker writes its own logs to stderr/stdout;
      # we tee everything into the slot's log file too.
      set +e
      bash "$WORKER_SCRIPT" > "$LOG" 2>&1
      RC=$?
      set -e

      EXIT_TS=$(date +%s)
      LIFETIME=$((EXIT_TS - LAUNCH_TS))
      echo "[$(date +%H:%M:%S)] slot-$slot: worker exited rc=$RC after ${LIFETIME}s"

      # Decide respawn behavior.
      case "$RC" in
        2)
          # ANTHROPIC_API_KEY hard guard — real config error. Don't respawn.
          # Other slots may still be fine; let them keep running.
          echo "[$(date +%H:%M:%S)] slot-$slot: rc=2 means ANTHROPIC_API_KEY was set; halting THIS slot only." >&2
          exit 2
          ;;
        0)
          # Clean exit. Most common when spin-loop guard fires (claim race
          # with conductor edits) or signal-trap fires.
          BACKOFF=30
          ;;
        *)
          # Unexpected. Use exponential backoff if it keeps happening fast.
          if [ "$LIFETIME" -lt 60 ]; then
            consecutive_fast_fails=$((consecutive_fast_fails + 1))
          else
            consecutive_fast_fails=0
          fi

          if [ "$consecutive_fast_fails" -ge 5 ]; then
            echo "[$(date +%H:%M:%S)] slot-$slot: 5 fast-fails in a row; halting THIS slot to avoid log spam." >&2
            exit 1
          fi
          BACKOFF=$((30 * (consecutive_fast_fails + 1)))
          ;;
      esac

      # Re-check stop sentinel before sleeping.
      if [ -f "$STOP_SENTINEL" ]; then
        echo "[$(date +%H:%M:%S)] slot-$slot: STOP sentinel present (mid-backoff), halting." >&2
        exit 0
      fi

      echo "[$(date +%H:%M:%S)] slot-$slot: respawn in ${BACKOFF}s..."
      sleep "$BACKOFF"
    done
  ) &
done

# Wait for all slot children. If they all die or supervisor gets a signal,
# fall out and clean up via the EXIT trap.
wait
