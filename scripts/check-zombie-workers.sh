#!/usr/bin/env bash
# check-zombie-workers.sh — conductor-side READ-ONLY zombie detector.
#
# Background
# ----------
# After tick-38 through tick-45 we observed 4 zombie-worker incidents
# (PIDs 92458 / 17940 / 18871 / 59400). Each followed the same pattern:
#   1. claude finishes its work (PR opened, ledger flipped, worktree cleaned).
#   2. claude emits {"type":"result","subtype":"success"} to stdout.
#   3. claude does NOT exit — process transitions S → SN and just sits.
#   4. The bash `claude | tee | jq` pipeline never closes, so the outer
#      loop never reaches its post-iteration block (quota check + cleanup +
#      next claim).
#   5. Result: PID slot burned, active pulse file, worktree on disk,
#      brief may be left CLAIMED (no PR → conductor manual sweep needed).
#
# Detection heuristic
# -------------------
# This script samples every live `claude --dangerously-skip-permissions`
# process and scores it into one of three tiers based on the SAME signals
# the worker-loop watchdog uses (claude-worker-loop.sh):
#
#   🟢 healthy
#       state R/R+ or S+, OR
#       state SN with recent log activity (≤10min) AND last event is not
#       a terminal result; brief row CLAIMED with active pulse.
#
#   🟡 suspicious
#       state SN, log silent 10-20min, brief still CLAIMED;
#       OR last event was a non-result (tool_use / assistant) — i.e. a
#       legitimate long iteration that's gone unusually quiet. Likely a
#       cold workspace-clippy or a multi-crate test run — check before
#       killing.
#
#   🔴 zombie
#       state SN, log silent ≥20min, last event is `result:success` or
#       `result:error`, brief still CLAIMED. This is the unique zombie
#       signature — same 4-condition gate the worker-loop watchdog fires
#       on. Safe to kill.
#
# What this script DOES NOT do
# ----------------------------
# - It does NOT kill any process. Kill authority stays with the conductor.
# - It does NOT mutate BUILD_STATUS.md. Row releases stay with the worker
#   loop (post-iteration ship-check) or the conductor's sweep tools.
# - It does NOT touch other workers' worktrees or pulse files.
#
# Conductor reaction guide
# ------------------------
# 🟢 — no action.
# 🟡 — leave it. Re-check on the next sweep. If still 🟡 after 20min,
#      tail ~/.argo/worker-logs/*-<brief>.ndjson to confirm it's
#      legitimate work (cargo, test, etc.) and not a slow zombie.
# 🔴 — `kill -TERM <pid>` (escalate to -KILL after 30s). Worker-loop's
#      own post-iteration ship-check will release the brief row to TODO.
#
# Usage
# -----
#   scripts/check-zombie-workers.sh
#   scripts/check-zombie-workers.sh --fixture <tsv-file>     # test mode
#   scripts/check-zombie-workers.sh --help
#
# Output format
# -------------
# Tab-separated, header line first:
#   STATUS<TAB>PID<TAB>BRIEF<TAB>STATE<TAB>ETIME<TAB>LOG_AGE_MIN<TAB>LAST_EVENT
#
# Exit codes
# ----------
#   0 — no zombies (may still have suspicious or healthy workers)
#   1 — one or more 🔴 zombies present
#   2 — invocation error

set -uo pipefail

LOG_DIR="${HOME}/.argo/worker-logs"
FIXTURE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --help|-h)
      sed -n '2,80p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    --fixture)
      FIXTURE="${2:-}"
      shift 2
      ;;
    --fixture=*)
      FIXTURE="${1#--fixture=}"
      shift
      ;;
    *)
      echo "unknown arg: $1" >&2
      exit 2
      ;;
  esac
done

# Portable mtime → epoch (BSD/macOS + GNU). Echoes 0 if file missing.
mtime_epoch() {
  local f="$1"
  [[ -e "$f" ]] || { echo 0; return; }
  stat -f %m "$f" 2>/dev/null || stat -c %Y "$f" 2>/dev/null || echo 0
}

# Classify a worker into 🟢/🟡/🔴 from (state, log_age_min, last_event).
#
# `last_event` after the PID-14468 followup: live mode now reports
# `result:success` if the log contains ANY result event (grep over the
# whole file), not just `tail -1`. This handles the wake-cycle case
# where Claude Code emits `system:init` AFTER a result to re-engage
# the session, and claude then wedges on the wake. We also accept the
# literal `system:init` value here as terminal-equivalent so fixture
# rows can model the wedge-on-init pattern directly without simulating
# the full log history.
classify() {
  local state="$1" log_age_min="$2" last_event="$3"
  local terminal=0
  case "$last_event" in
    result:success|result:error|system:init) terminal=1 ;;
  esac

  # Default state buckets first.
  case "$state" in
    R*|S+|Ss|I*)
      echo "🟢healthy"; return ;;
  esac

  # SN (nice'd sleep) is the zombie state. Apply the 4-condition gate.
  if [[ "$state" == SN* ]] || [[ "$state" == "S" ]]; then
    if [[ "$terminal" == "1" ]] && [[ "$log_age_min" != "?" ]] && [[ "$log_age_min" -ge 20 ]]; then
      echo "🔴zombie"; return
    fi
    if [[ "$log_age_min" != "?" ]] && [[ "$log_age_min" -ge 10 ]]; then
      echo "🟡suspicious"; return
    fi
    echo "🟢healthy"; return
  fi

  # Anything we don't recognise gets flagged for human review.
  echo "🟡suspicious"
}

# Print a single worker row given the raw inputs.
# Args: pid state etime brief log_age_min last_event
print_row() {
  local pid="$1" state="$2" etime="$3" brief="$4" log_age_min="$5" last_event="$6"
  local tier
  tier=$(classify "$state" "$log_age_min" "$last_event")
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$tier" "$pid" "$brief" "$state" "$etime" "$log_age_min" "$last_event"
}

# Fixture mode: read tab-separated rows of (pid, state, etime, brief,
# log_age_min, last_event) and classify each. Used by the smoke test.
if [[ -n "$FIXTURE" ]]; then
  if [[ ! -r "$FIXTURE" ]]; then
    echo "fixture not readable: $FIXTURE" >&2
    exit 2
  fi
  printf 'STATUS\tPID\tBRIEF\tSTATE\tETIME\tLOG_AGE_MIN\tLAST_EVENT\n'
  zombie_count=0
  while IFS=$'\t' read -r pid state etime brief log_age last_event; do
    [[ -z "${pid:-}" ]] && continue
    [[ "$pid" == "#"* ]] && continue
    row=$(print_row "$pid" "$state" "$etime" "$brief" "$log_age" "$last_event")
    echo "$row"
    [[ "$row" == "🔴zombie"* ]] && zombie_count=$((zombie_count + 1))
  done < "$FIXTURE"
  [[ "$zombie_count" -gt 0 ]] && exit 1 || exit 0
fi

# Live mode.
PIDS=$(pgrep -f "claude --dangerously-skip-permissions" 2>/dev/null || true)
if [[ -z "$PIDS" ]]; then
  echo "(no claude workers alive)"
  exit 0
fi

printf 'STATUS\tPID\tBRIEF\tSTATE\tETIME\tLOG_AGE_MIN\tLAST_EVENT\n'

NOW_EPOCH=$(date +%s)
ZOMBIE_COUNT=0

for pid in $PIDS; do
  state=$(ps -o state= -p "$pid" 2>/dev/null | tr -d ' ')
  [[ -z "$state" ]] && continue  # process died between pgrep and ps
  etime=$(ps -o etime= -p "$pid" 2>/dev/null | tr -d ' ')

  # The claude `-p "You are a SKYCOT $FAMILY worker... Brief: $BRIEF_ID
  # (already CLAIMED..."` includes a literal "Brief: <id>" marker. Pull it
  # out of the full command line.
  args=$(ps -o command= -p "$pid" 2>/dev/null || true)
  brief=$(printf '%s' "$args" | grep -oE 'Brief: [a-zA-Z0-9_-]+' | head -1 | sed 's/^Brief: //')
  [[ -z "$brief" ]] && brief="(unknown)"

  # Latest log file for this brief. Brief ids are [a-zA-Z0-9_-] (validated
  # by the ledger format), so `ls -t` glob is safe — no surprising shell
  # metachars. shellcheck disable=SC2012.
  log=""
  if [[ -d "$LOG_DIR" ]] && [[ "$brief" != "(unknown)" ]]; then
    # shellcheck disable=SC2012
    log=$(ls -t "$LOG_DIR"/*-"$brief".ndjson 2>/dev/null | head -1)
  fi
  if [[ -n "$log" ]]; then
    log_mtime=$(mtime_epoch "$log")
    log_age_min=$(( (NOW_EPOCH - log_mtime) / 60 ))
    # PID-14468 followup: derive last_event from the WHOLE log, not just
    # the last line. If any result:success/error event ever appeared, we
    # report that — even if the last raw line is `system:init` or a
    # `task_notification` from Claude Code's background-task wake
    # mechanism. The classify() function treats this as terminal so the
    # 20-min log-staleness + SN state combination correctly flags zombies
    # that wedged on a post-result init wake.
    if grep -qE '"type":"result"[^{}]*"subtype":"error"|"subtype":"error"[^{}]*"type":"result"' "$log" 2>/dev/null; then
      last_event="result:error"
    elif grep -qE '"type":"result"[^{}]*"subtype":"success"|"subtype":"success"[^{}]*"type":"result"' "$log" 2>/dev/null; then
      last_event="result:success"
    else
      last_line=$(tail -1 "$log" 2>/dev/null || echo "")
      if command -v jq >/dev/null 2>&1; then
        last_event=$(printf '%s' "$last_line" | jq -r '
          if .type == "assistant" then "assistant"
          elif .type == "system" then "system:" + (.subtype // "ok")
          else (.type // "?") end' 2>/dev/null || echo "?")
      else
        case "$last_line" in
          *'"type":"assistant"'*) last_event="assistant" ;;
          *'"type":"system"'*'"subtype":"init"'*) last_event="system:init" ;;
          *)                      last_event="?" ;;
        esac
      fi
    fi
  else
    log_age_min="?"
    last_event="(no log)"
  fi

  row=$(print_row "$pid" "$state" "$etime" "$brief" "$log_age_min" "$last_event")
  echo "$row"
  [[ "$row" == "🔴zombie"* ]] && ZOMBIE_COUNT=$((ZOMBIE_COUNT + 1))
done

[[ "$ZOMBIE_COUNT" -gt 0 ]] && exit 1 || exit 0
