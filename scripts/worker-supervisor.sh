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
#   bash scripts/argo-pause.sh                    # PREFERRED — atomically sets both sentinels (transient + persistent)
#   touch /tmp/argo-stop                          # transient-only fallback (lost on macOS reboot or /tmp cleanup)
#   pkill -f worker-supervisor.sh                 # hard kill (current iters keep running until they finish)
#
# Resume:
#   bash scripts/argo-resume.sh                   # clears both sentinels atomically
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
# Extend WAIT_LOOP_REGEX when future worker-loop stalls settle into
# another repeated no-progress result phrase.
WAIT_LOOP_REGEX="${WAIT_LOOP_REGEX:-^(Waiting for|Still waiting|Monitor is armed|Build cache is huge)}"
WAIT_LOOP_MIN_CONSECUTIVE="${WAIT_LOOP_MIN_CONSECUTIVE:-3}"
WAIT_LOOP_MIN_ETIME_SECS="${WAIT_LOOP_MIN_ETIME_SECS:-7200}"
WAIT_LOOP_WATCHDOG_INTERVAL_SECS="${WAIT_LOOP_WATCHDOG_INTERVAL_SECS:-60}"
WAIT_LOOP_WATCHDOG_ARTIFACT="${WAIT_LOOP_WATCHDOG_ARTIFACT:-/tmp/argo-watchdog-wait-loop-this-run}"
WAIT_LOOP_RELEASE_CLAIM="${WAIT_LOOP_RELEASE_CLAIM:-1}"

# Returns 0 (true) when either halt sentinel is present. Use as
# `if stop_present; then ... fi` everywhere the old `[ -f $STOP_SENTINEL ]`
# check appeared.
stop_present() {
  [ -f "$STOP_SENTINEL_TMP" ] || [ -f "$STOP_SENTINEL_HOME" ]
}

# STOP sentinel = PAUSE this slot until operator clears it (not a permanent
# halt). Slot subshell stays alive throughout the pause and resumes when the
# sentinel is removed. Operator's relight procedure simplifies to just
# `rm /tmp/argo-stop ~/.argo/stop`; no `launchctl load` needed.
#
# Reason this exists: prior behavior was `exit 0` whenever a slot saw the
# sentinel (at top-of-loop, mid-RAM-wait, or mid-respawn-backoff). That
# permanently killed the slot subshell, and because the supervisor script
# doesn't auto-respawn slot subshells (only launchd auto-restarts the whole
# supervisor process when its PID dies), losing a slot subshell meant losing
# that slot until the next `launchctl load`. Slots ALSO died asymmetrically:
# whichever slot happened to be deep inside `wait "$WORKER_PID"` when the
# operator paused-then-cleared survived; the others didn't. 2026-05-26
# incident: slot-1 died at 01:02:33 local while slot-2 happened to be inside
# a 4h cargo iteration and survived. Operator lost slot-1 for ~7h overnight.
wait_for_stop_to_clear() {
  local logged=0 slot_arg="${1:-?}"
  while stop_present; do
    if [ "$logged" -eq 0 ]; then
      echo "[$(date +%H:%M:%S)] slot-${slot_arg}: STOP sentinel present, pausing (will auto-resume when cleared)." >&2
      logged=1
    fi
    sleep 30
  done
  if [ "$logged" -eq 1 ]; then
    echo "[$(date +%H:%M:%S)] slot-${slot_arg}: STOP sentinel cleared, resuming." >&2
  fi
}

# Returns 0 (true) when available RAM is below RAM_PRESSURE_THRESHOLD_MB.
# Used by slots to back off before launching a new worker iteration
# when the system is already under pressure (prevents the jetsam
# death-spiral that ran 2026-05-24 → 2026-05-25). Tier-2 fix.
#
# Counts all instantly-recyclable pages — free + speculative +
# inactive + purgeable — analogous to Linux's MemAvailable. The
# earlier version only counted free + speculative, which on macOS
# routinely reads 200-800 MB even when 9+ GB of inactive cache is
# trivially recyclable (jetsam-style false-pause; 2026-05-26 fix).
RAM_PRESSURE_THRESHOLD_MB="${RAM_PRESSURE_THRESHOLD_MB:-1024}"  # 1 GB default
ram_under_pressure() {
  local page_size free_pages spec_pages inact_pages purg_pages avail_mb
  # Single vm_stat call; awk parses all fields we need.
  eval "$(vm_stat | awk '
    /page size of/      { print "page_size="$8 }
    /Pages free/        { gsub(/\./,"",$3); print "free_pages="$3 }
    /Pages speculative/ { gsub(/\./,"",$3); print "spec_pages="$3 }
    /Pages inactive/    { gsub(/\./,"",$3); print "inact_pages="$3 }
    /Pages purgeable/   { gsub(/\./,"",$3); print "purg_pages="$3 }
  ')"
  [ -z "$page_size" ] && return 1   # vm_stat unavailable → don't gate
  [ -z "$free_pages" ] && return 1
  [ -z "$spec_pages" ] && spec_pages=0
  [ -z "$inact_pages" ] && inact_pages=0
  [ -z "$purg_pages" ] && purg_pages=0
  avail_mb=$(( (free_pages + spec_pages + inact_pages + purg_pages) * page_size / 1024 / 1024 ))
  [ "$avail_mb" -lt "$RAM_PRESSURE_THRESHOLD_MB" ]
}

active_worker_ndjson_log() {
  local slot_log="$1"
  [ -f "$slot_log" ] || return 1
  grep -Eo 'Log: .+\.ndjson' "$slot_log" 2>/dev/null | tail -1 | sed 's/^Log: //'
}

wait_loop_brief_id() {
  basename "$1" | sed -E 's/^[0-9]{8}T[0-9]{6}Z-(.*)\.ndjson$/\1/'
}

wait_loop_log_start_epoch() {
  basename "$1" | python3 -c 'import datetime, re, sys
name = sys.stdin.read().strip()
m = re.match(r"^(\d{8}T\d{6}Z)-.+\.ndjson$", name)
if not m:
    sys.exit(1)
dt = datetime.datetime.strptime(m.group(1), "%Y%m%dT%H%M%SZ").replace(tzinfo=datetime.timezone.utc)
print(int(dt.timestamp()))'
}

wait_loop_consecutive_count() {
  local ndjson_log="$1"
  [ -f "$ndjson_log" ] || return 1
  python3 - "$ndjson_log" "$WAIT_LOOP_REGEX" <<'PY'
import json
import re
import sys

path, pattern = sys.argv[1:3]
regex = re.compile(pattern)
consecutive = 0

with open(path, encoding="utf-8") as handle:
    for line in handle:
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if event.get("type") != "result" or event.get("subtype") != "success":
            continue
        result = event.get("result") or ""
        if not result.strip():
            continue
        if regex.search(result):
            consecutive += 1
        else:
            consecutive = 0

print(consecutive)
PY
}

wait_loop_should_fire() {
  local ndjson_log="$1"
  local now_epoch="${2:-$(date +%s)}"
  [ -f "$ndjson_log" ] || return 1

  local start_epoch count etime
  start_epoch="$(wait_loop_log_start_epoch "$ndjson_log" 2>/dev/null || true)"
  [ -n "$start_epoch" ] || return 1
  etime=$((now_epoch - start_epoch))
  [ "$etime" -ge "$WAIT_LOOP_MIN_ETIME_SECS" ] || return 1

  count="$(wait_loop_consecutive_count "$ndjson_log" 2>/dev/null || echo 0)"
  [ "$count" -ge "$WAIT_LOOP_MIN_CONSECUTIVE" ]
}

terminate_process_tree() {
  local root_pid="$1"
  local child
  for child in $(pgrep -P "$root_pid" 2>/dev/null || true); do
    terminate_process_tree "$child"
  done
  kill -TERM "$root_pid" 2>/dev/null || true
}

kill_process_tree_after_grace() {
  local root_pid="$1"
  local child
  sleep 5
  for child in $(pgrep -P "$root_pid" 2>/dev/null || true); do
    kill_process_tree_after_grace "$child"
  done
  kill -KILL "$root_pid" 2>/dev/null || true
}

release_wait_loop_claim() {
  local brief_id="$1"
  local ndjson_log="$2"

  [ "$WAIT_LOOP_RELEASE_CLAIM" = "1" ] || return 0
  [ -n "$brief_id" ] || return 0
  [ -d "$REPO/.git" ] || return 0

  set +e
  git -C "$REPO" fetch origin main >/dev/null 2>&1
  git -C "$REPO" checkout main >/dev/null 2>&1
  git -C "$REPO" pull --ff-only origin main >/dev/null 2>&1

  if grep -qE "^\| ${brief_id} \| (claude|codex) \| CLAIMED \|" "$REPO/docs/build-plan/BUILD_STATUS.md" 2>/dev/null; then
    python3 - "$brief_id" "$REPO/docs/build-plan/BUILD_STATUS.md" "$ndjson_log" <<'PY'
import pathlib
import re
import sys

brief_id, ledger_path, ndjson_log = sys.argv[1:4]
path = pathlib.Path(ledger_path)
text = path.read_text()
note = (
    " **RELEASED by worker-supervisor wait-loop watchdog:** "
    "3+ consecutive no-progress result events after >2h; "
    f"/tmp/{brief_id} may have salvageable WIP. Log: {ndjson_log}."
)
pattern = re.compile(
    rf"^(\| {re.escape(brief_id)} \| (?:claude|codex)) \| CLAIMED \|(.*) \|$",
    re.MULTILINE,
)
new, count = pattern.subn(
    lambda m: f"{m.group(1)} | TODO |{m.group(2)}{note} |",
    text,
    count=1,
)
if count:
    path.write_text(new)
PY
    git -C "$REPO" add docs/build-plan/BUILD_STATUS.md >/dev/null 2>&1
    if ! git -C "$REPO" diff --cached --quiet >/dev/null 2>&1; then
      git -C "$REPO" commit -m "status: release $brief_id (wait-loop watchdog)" >/dev/null 2>&1
      git -C "$REPO" pull --rebase origin main >/dev/null 2>&1
      git -C "$REPO" push origin main >/dev/null 2>&1
    fi
  fi
  set -e
}

monitor_wait_loop_watchdog() {
  local worker_pid="$1"
  local slot_log="$2"

  while kill -0 "$worker_pid" 2>/dev/null; do
    sleep "$WAIT_LOOP_WATCHDOG_INTERVAL_SECS"
    kill -0 "$worker_pid" 2>/dev/null || break

    local ndjson_log brief_id count start_epoch etime ts
    ndjson_log="$(active_worker_ndjson_log "$slot_log" || true)"
    [ -n "$ndjson_log" ] && [ -f "$ndjson_log" ] || continue
    wait_loop_should_fire "$ndjson_log" || continue

    brief_id="$(wait_loop_brief_id "$ndjson_log")"
    count="$(wait_loop_consecutive_count "$ndjson_log" 2>/dev/null || echo 0)"
    start_epoch="$(wait_loop_log_start_epoch "$ndjson_log" 2>/dev/null || date +%s)"
    etime=$(($(date +%s) - start_epoch))
    ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf '%s wait-loop brief=%s worker_pid=%s count=%s etime=%ss log=%s\n' \
      "$ts" "$brief_id" "$worker_pid" "$count" "$etime" "$ndjson_log" >> "$WAIT_LOOP_WATCHDOG_ARTIFACT"

    echo "[$(date +%H:%M:%S)] slot watchdog: wait-loop detected for $brief_id (count=$count etime=${etime}s); sending SIGTERM..." >&2
    terminate_process_tree "$worker_pid"
    kill_process_tree_after_grace "$worker_pid" &
    release_wait_loop_claim "$brief_id" "$ndjson_log"
    break
  done
}

# Test harnesses source this file to exercise the wait-loop helpers
# without starting the long-running supervisor.
if [ "${ARGO_SUPERVISOR_SOURCE_ONLY:-0}" = "1" ]; then
  if [ "${BASH_SOURCE[0]:-$0}" != "$0" ]; then
    return 0
  fi
  exit 0
fi

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
      # Check stop sentinel (transient + persistent). PAUSE not HALT — slot
      # subshell stays alive across the operator's pause and resumes on its
      # own when sentinel is cleared.
      wait_for_stop_to_clear "$slot"

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
        avail_mb=$(vm_stat | awk '
          /page size of/      { ps=$8 }
          /Pages free/        { gsub(/\./,"",$3); fp=$3 }
          /Pages speculative/ { gsub(/\./,"",$3); sp=$3 }
          /Pages inactive/    { gsub(/\./,"",$3); ip=$3 }
          /Pages purgeable/   { gsub(/\./,"",$3); pp=$3 }
          END { printf "%d", (fp+sp+ip+pp)*ps/1024/1024 }
        ')
        echo "[$(date +%H:%M:%S)] slot-$slot: RAM-PRESSURE (avail ${avail_mb}MB < ${RAM_PRESSURE_THRESHOLD_MB}MB threshold), sleeping ${ram_wait}s before re-check" >&2
        # At step 3 (30 min cap), also append an alert to SHARED.md so the
        # operator sees sustained pressure in their heartbeat scan.
        if [ "$ram_backoff_step" -ge 3 ] && [ -f "$REPO/docs/skycot/agents/SHARED.md" ]; then
          ts=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
          echo "- $ts [supervisor-ram-pressure] slot-$slot avail=${avail_mb}MB sustained-low for $ram_backoff_step backoff cycles; manual operator attention may be warranted (close apps, kill cargo, or relight workers after pressure clears)." >> "$REPO/docs/skycot/agents/SHARED.md"
        fi
        sleep "$ram_wait"
        ram_backoff_step=$((ram_backoff_step + 1))
        # Re-check stop sentinel mid-pressure-wait — pause if set.
        wait_for_stop_to_clear "$slot"
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
      bash "$WORKER_SCRIPT" > "$LOG" 2>&1 &
      WORKER_PID=$!
      monitor_wait_loop_watchdog "$WORKER_PID" "$LOG" &
      WAIT_LOOP_WATCHDOG_PID=$!
      wait "$WORKER_PID"
      RC=$?
      kill "$WAIT_LOOP_WATCHDOG_PID" 2>/dev/null || true
      wait "$WAIT_LOOP_WATCHDOG_PID" 2>/dev/null || true
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

      # Re-check stop sentinel before sleeping — pause if set.
      wait_for_stop_to_clear "$slot"

      echo "[$(date +%H:%M:%S)] slot-$slot: respawn in ${BACKOFF}s..."
      sleep "$BACKOFF"
    done
  ) &
done

# Wait for all slot children. If they all die or supervisor gets a signal,
# fall out and clean up via the EXIT trap.
wait
