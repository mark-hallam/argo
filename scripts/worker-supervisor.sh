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
# Stop sentinels (checked together via stop_present helper below):
#   /tmp/argo-stop   — transient halt (wiped by macOS reboot).
#   ~/.argo/stop     — persistent halt (survives reboot). Added 2026-05-25
#                      tier-1 fix after the 2026-05-25 recovery session
#                      lost the /tmp halt over the upcoming reboot.

set -euo pipefail

REPO="${REPO:-~/code/argo}"
N_SLOTS="${1:-4}"
WORKER_SCRIPT="$REPO/scripts/claude-worker-loop.sh"
STOP_SENTINEL_TMP="/tmp/argo-stop"
STOP_SENTINEL_HOME="$HOME/.argo/stop"
SUPERVISOR_PID_FILE="/tmp/argo-worker-supervisor.pid"

# Returns 0 (true) when either halt sentinel is present. Use as
# `if stop_present; then ... fi` everywhere the old `[ -f $STOP_SENTINEL ]`
# check appeared.
stop_present() {
  [ -f "$STOP_SENTINEL_TMP" ] || [ -f "$STOP_SENTINEL_HOME" ]
}

# Returns 0 (true) when free RAM is below RAM_PRESSURE_THRESHOLD_MB.
# Used by slots to back off before launching a new worker iteration
# when the system is already under pressure (prevents the jetsam
# death-spiral that ran 2026-05-24 → 2026-05-25). Tier-2 fix.
#
# Implementation: vm_stat returns page counts; on macOS the page size
# is reported in the header. We multiply free + speculative pages
# by page size to get bytes, then compare against threshold.
RAM_PRESSURE_THRESHOLD_MB="${RAM_PRESSURE_THRESHOLD_MB:-1024}"  # 1 GB default
ram_under_pressure() {
  local page_size free_pages spec_pages avail_mb
  # macOS page size lives in `vm_stat`'s first line: "page size of N bytes".
  page_size=$(vm_stat | awk '/page size of/ {print $8}')
  [ -z "$page_size" ] && return 1   # vm_stat unavailable → don't gate
  free_pages=$(vm_stat | awk '/Pages free/ {gsub(/\./,"",$3); print $3}')
  spec_pages=$(vm_stat | awk '/Pages speculative/ {gsub(/\./,"",$3); print $3}')
  [ -z "$free_pages" ] && return 1
  [ -z "$spec_pages" ] && spec_pages=0
  avail_mb=$(( (free_pages + spec_pages) * page_size / 1024 / 1024 ))
  [ "$avail_mb" -lt "$RAM_PRESSURE_THRESHOLD_MB" ]
}

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
echo "  stop:       touch $STOP_SENTINEL_TMP  (transient)"
echo "              touch $STOP_SENTINEL_HOME  (persistent across reboot)"
echo "  ram-gate:   pause slot iteration when avail RAM < ${RAM_PRESSURE_THRESHOLD_MB}MB"
echo "              (exponential backoff: 5min → 15min → 30min cap)"
echo "═══════════════════════════════════════════════════════════════════"

# One supervisor child per slot. Each child loops: launch worker, wait,
# decide on respawn based on exit code, sleep backoff, repeat.
for slot in $(seq 1 "$N_SLOTS"); do
  (
    consecutive_fast_fails=0
    LAST_LAUNCH_TS=0
    ram_backoff_step=0   # 0=initial, 1=5min waited, 2=15min waited, 3=30min cap

    while true; do
      # Check stop sentinel (transient + persistent).
      if stop_present; then
        echo "[$(date +%H:%M:%S)] slot-$slot: STOP sentinel present, halting." >&2
        exit 0
      fi

      # Memory-pressure gate. If avail RAM is below the threshold, the
      # system is likely to jetsam our cargo as soon as we spawn one.
      # Wait progressively longer between re-checks so a transient spike
      # gets a fast retry but a sustained-pressure state doesn't burn
      # tokens spinning.
      while ram_under_pressure; do
        case "$ram_backoff_step" in
          0) ram_wait=300 ;;    # 5 min — first hit
          1) ram_wait=900 ;;    # 15 min — second hit
          *) ram_wait=1800 ;;   # 30 min cap — sustained pressure
        esac
        avail_mb=$(( ($(vm_stat | awk '/Pages free/ {gsub(/\./,"",$3); print $3}') + $(vm_stat | awk '/Pages speculative/ {gsub(/\./,"",$3); print $3}')) * $(vm_stat | awk '/page size of/ {print $8}') / 1024 / 1024 ))
        echo "[$(date +%H:%M:%S)] slot-$slot: RAM-PRESSURE (avail ${avail_mb}MB < ${RAM_PRESSURE_THRESHOLD_MB}MB threshold), sleeping ${ram_wait}s before re-check" >&2
        # At step 3 (30 min cap), also append an alert to SHARED.md so the
        # operator sees sustained pressure in their heartbeat scan.
        if [ "$ram_backoff_step" -ge 3 ] && [ -f "$REPO/docs/skycot/agents/SHARED.md" ]; then
          ts=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
          echo "- $ts [supervisor-ram-pressure] slot-$slot avail=${avail_mb}MB sustained-low for $ram_backoff_step backoff cycles; manual operator attention may be warranted (close apps, kill cargo, or relight workers after pressure clears)." >> "$REPO/docs/skycot/agents/SHARED.md"
        fi
        sleep "$ram_wait"
        ram_backoff_step=$((ram_backoff_step + 1))
        # Re-check stop sentinel mid-pressure-wait so operator can halt.
        if stop_present; then
          echo "[$(date +%H:%M:%S)] slot-$slot: STOP sentinel present during ram-pressure wait, halting." >&2
          exit 0
        fi
      done
      # Pressure cleared — reset backoff counter for next event.
      ram_backoff_step=0

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
            # Long cooldown not permanent halt — root cause (ledger drift, stale lock, transient rate-limit) usually clears within minutes; permanent halt requires a supervisor restart which is a bigger blast radius than necessary.
            echo "[$(date +%H:%M:%S)] slot-$slot: 5 fast-fails in a row; cooling down 15min then retrying." >&2
            BACKOFF=900
            consecutive_fast_fails=0
          else
            BACKOFF=$((30 * (consecutive_fast_fails + 1)))
          fi
          ;;
      esac

      # Re-check stop sentinel before sleeping.
      if stop_present; then
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
