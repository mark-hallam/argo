#!/usr/bin/env bash
# main-verify-sweep.sh — the post-merge main-verification daemon.
#
# WHY: CI is permanently OFF (operator decision; GitHub billing blocked). Nothing
# re-checks origin/main AFTER a merge, so a reviewed+merged fix can be silently
# reverted by a later concurrent merge and go unnoticed (this happened to the
# #2936 convergence fix — caught only by a manual soak). This daemon closes the
# DEC-042 build-loop gap: "every loop ends in a CHECKABLE verifier" — applied to
# origin/main itself.
#
# WHAT: whenever origin/main advances, check out the NEW head in an ISOLATED
# worktree (never the swarm's shared checkout), run the curated critical suite,
# and on RED set a merge-freeze sentinel + alert so the conductor stops merging
# onto a broken main until it is green again.
#
# Safety-critical signal is the sentinel + status.json under ~/.argo (immune to
# the workers' `git reset --hard` of the shared tree); the SHARED.md line is a
# best-effort human alert. All timestamps UTC (protocol).
set -uo pipefail

REPO="${MAIN_VERIFY_REPO:-$HOME/code/argo}"
STATE_DIR="$HOME/.argo/main-verify"
FREEZE="$STATE_DIR/merge-freeze"
STATUS="$STATE_DIR/status.json"
LAST_VERIFIED="$STATE_DIR/last-verified-sha"
LAST_CHECKED="$STATE_DIR/last-checked-sha"
LOCK="/tmp/argo-main-verify.lock"
POLL_SECS="${MAIN_VERIFY_POLL_SECS:-120}"
RAM_THRESHOLD_MB="${MAIN_VERIFY_RAM_MB:-1500}"
SHARED="$REPO/docs/skycot/agents/SHARED.md"
export CONDUCTOR_BUILD_WORKTREE_DIR="${CONDUCTOR_BUILD_WORKTREE_DIR:-$HOME/.cache/argo-main-verify-wt}"
export CONDUCTOR_BUILD_TARGET_DIR="${CONDUCTOR_BUILD_TARGET_DIR:-$HOME/.cache/argo-main-verify-target}"

mkdir -p "$STATE_DIR" "$HOME/.argo/logs"

now_utc() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }
log() { echo "$(now_utc) [main-verify] $*"; }
stop_present() { [ -f /tmp/argo-stop ] || [ -f "$HOME/.argo/stop" ] || [ -f "$HOME/.argo/main-verify-stop" ]; }
ram_mb() { bash "$REPO/scripts/free-ram-mb.sh" 2>/dev/null || echo 99999; }
ram_under_pressure() { local m; m="$(ram_mb)"; [ -n "$m" ] && [ "$m" -lt "$RAM_THRESHOLD_MB" ]; }

# Singleton (mkdir is atomic) — a manual run and the launchd instance can't double-build.
if ! mkdir "$LOCK" 2>/dev/null; then log "another sweep holds $LOCK; exiting"; exit 0; fi
trap 'rmdir "$LOCK" 2>/dev/null' EXIT INT TERM HUP

write_status() {  # $1=state(GREEN|RED) $2=sha $3=verified_at $4=duration $5=fails
  local state="$1" sha="$2" vat="$3" dur="$4" fails="$5"
  local fj=""; if [ -n "$fails" ]; then fj="$(printf '%s' "$fails" | tr ' ' '\n' | grep -v '^$' | sed 's/.*/"&"/' | paste -sd, -)"; fi
  cat > "$STATUS" <<JSON
{
  "agent": "main-verify",
  "state": "$state",
  "last_checked_sha": "$sha",
  "last_verified_sha": "$(cat "$LAST_VERIFIED" 2>/dev/null || echo "")",
  "checked_at_utc": "$(now_utc)",
  "verified_at_utc": "$vat",
  "duration_seconds": $dur,
  "merge_freeze": $([ -f "$FREEZE" ] && echo true || echo false),
  "failing": [${fj}]
}
JSON
}

alert_shared() {  # best-effort human alert; the sentinel is the load-bearing gate
  [ -f "$SHARED" ] || return 0
  echo "- $(now_utc) [main-verify-$1] $2" >> "$SHARED" 2>/dev/null || true
}

log "sweep started (poll=${POLL_SECS}s, ram_floor=${RAM_THRESHOLD_MB}MB, wt=$CONDUCTOR_BUILD_WORKTREE_DIR)"
prev_state="$(grep -oE '"state"[^,]*' "$STATUS" 2>/dev/null | grep -oE 'GREEN|RED' || echo UNKNOWN)"

while true; do
  while stop_present; do sleep 30; done

  if ! git -C "$REPO" fetch origin main -q 2>/dev/null; then sleep "$POLL_SECS"; continue; fi
  NEW_HEAD="$(git -C "$REPO" rev-parse origin/main 2>/dev/null)"
  [ -z "$NEW_HEAD" ] && { sleep "$POLL_SECS"; continue; }
  if [ "$NEW_HEAD" = "$(cat "$LAST_CHECKED" 2>/dev/null || echo)" ]; then sleep "$POLL_SECS"; continue; fi

  # RAM backoff ladder (5 -> 15 -> 30 min cap), copied from worker-supervisor
  step=0
  while ram_under_pressure; do
    case $step in 0) w=300;; 1) w=900;; *) w=1800;; esac
    log "RAM under ${RAM_THRESHOLD_MB}MB ($(ram_mb)MB) — backing off ${w}s"
    sleep "$w"; step=$((step+1)); while stop_present; do sleep 30; done
  done

  log "origin/main advanced to ${NEW_HEAD:0:9} — verifying in isolated worktree"
  if ! eval "$(bash "$REPO/scripts/conductor-build-worktree.sh" --ref "$NEW_HEAD" 2>/dev/null)"; then
    log "worktree sync failed; will retry next poll"; sleep "$POLL_SECS"; continue
  fi
  echo "$NEW_HEAD" > "$LAST_CHECKED"

  t0="$(date +%s)"
  # Run the suite + its config FROM the synced worktree (not $REPO): the worktree
  # always matches the commit under test and is always present, so the daemon does
  # NOT depend on the volatile shared checkout having the new scripts. (The two
  # bootstrap helpers free-ram-mb.sh / conductor-build-worktree.sh come from $REPO
  # — they predate this and are always present even in a stale checkout.)
  suite_out="$(CARGO_TARGET_DIR="$CONDUCTOR_BUILD_TARGET" bash "$CONDUCTOR_BUILD_WT/scripts/main-verify-suite.sh" --worktree "$CONDUCTOR_BUILD_WT" 2>&1)"
  suite_ec=$?
  dur=$(( $(date +%s) - t0 ))
  printf '%s\n' "$suite_out" | tail -8

  if [ "$suite_ec" -eq 0 ]; then
    echo "$NEW_HEAD" > "$LAST_VERIFIED"
    rm -f "$FREEZE"
    write_status GREEN "$NEW_HEAD" "$(now_utc)" "$dur" ""
    log "GREEN ${NEW_HEAD:0:9} (${dur}s)"
    [ "$prev_state" = "RED" ] && alert_shared "GREEN" "origin/main@${NEW_HEAD:0:9} is GREEN again — merge freeze CLEARED."
    prev_state="GREEN"
  else
    fails="$(printf '%s\n' "$suite_out" | sed -nE 's/^FAILS=//p' | tail -1)"
    touch "$FREEZE"
    write_status RED "$NEW_HEAD" "$(cat "$LAST_VERIFIED" >/dev/null 2>&1; now_utc)" "$dur" "$fails"
    log "RED ${NEW_HEAD:0:9} (${dur}s) — fails:$fails — MERGE FREEZE set"
    # Alert on entry-into-RED or when the failing set changed (avoid per-poll noise)
    if [ "$prev_state" != "RED" ]; then
      alert_shared "RED" "origin/main@${NEW_HEAD:0:9} FAILS critical suite:${fails}. MERGE FREEZE active (~/.argo/main-verify/merge-freeze) — do NOT admin-merge until green. See ~/.argo/main-verify/status.json."
    fi
    prev_state="RED"
  fi
  sleep "$POLL_SECS"
done
