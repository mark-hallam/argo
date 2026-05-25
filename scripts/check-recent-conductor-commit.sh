#!/usr/bin/env bash
# check-recent-conductor-commit.sh -- detect a recent peer-conductor commit on
# origin/main and recommend the current conductor short-circuit to avoid race.
#
# Background: ScheduledTask (cron-fired) conductor ticks + interactive
# operator-driven conductor sessions can collide when their windows overlap.
# The singleton heartbeat check (check-conductor-singleton.sh) is a snapshot
# of the heartbeat *file*; it can miss the case where another conductor JUST
# committed + pushed but hasn't yet written the heartbeat (or the heartbeat
# write is still in flight). The worker-supervisor watchdog stashes mid-edit
# working trees every ~2 min, which can orphan an untracked brief file from
# its ledger row if a tick is mid-commit. The conductor's recent-commit
# check below catches both: peer commit landed → short-circuit.
#
# Recommended by operator after the T09-15 cross-conductor near-miss in the
# 2026-05-25 session. Save to memory: feedback-conductor-recent-commit-check.
#
# Usage:
#   scripts/check-recent-conductor-commit.sh           # default 10-min window
#   scripts/check-recent-conductor-commit.sh --window 15  # 15-min window
#   scripts/check-recent-conductor-commit.sh --window 5 --branch main
#
# Exit codes:
#   0  no recent peer conductor commit found; safe to proceed.
#   1  recent peer conductor commit detected on origin/main; short-circuit
#      this tick. Output: SHA + age + subject of the most recent match.

set -euo pipefail

WINDOW_MIN=10
BRANCH="main"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --window)
      if [ -z "${2:-}" ]; then echo "missing --window value" >&2; exit 2; fi
      WINDOW_MIN="$2"
      shift 2
      ;;
    --branch)
      if [ -z "${2:-}" ]; then echo "missing --branch value" >&2; exit 2; fi
      BRANCH="$2"
      shift 2
      ;;
    -h|--help)
      sed -n '2,28p' "$0"
      exit 0
      ;;
    *)
      echo "unknown arg: $1" >&2
      exit 2
      ;;
  esac
done

# Pattern matches the conductor's commit-message convention:
#   "conductor: tick T<HH-MM> ..." or "conductor: <free-form>"
# Excludes worker / supervisor / status / ops / eval-authored commits so we
# don't false-positive on noise. Status commits (`status: ...`) are written
# by both workers and conductors; we filter to the explicit `conductor:`
# prefix to keep precision.
PATTERN='^conductor:'

# Don't fetch here — caller's responsibility. We read what's local.
# Caller does `git fetch origin main` before calling this script.
if ! git rev-parse --verify "origin/$BRANCH" >/dev/null 2>&1; then
  echo "origin/$BRANCH not found; skipping check (treat as safe)"
  exit 0
fi

since="${WINDOW_MIN} minutes ago"
matches="$(git log --since="$since" --grep="$PATTERN" --extended-regexp \
  --pretty='%h|%ar|%s' "origin/$BRANCH" 2>/dev/null || true)"

if [ -z "$matches" ]; then
  exit 0
fi

# Recent peer conductor commit detected.
echo "RECENT_PEER_CONDUCTOR_COMMIT_DETECTED window=${WINDOW_MIN}min branch=origin/$BRANCH"
echo "$matches" | head -3 | while IFS='|' read -r sha age subject; do
  echo "  $sha ($age) -- $subject"
done
echo
echo "Recommendation: short-circuit this tick to avoid collision."
echo "  - Skip queue-replenishment / brief-authoring / ledger-flip mutations."
echo "  - Skip merge actions (peer may have queued PR-comment / state changes)."
echo "  - Heartbeat refresh + read-only status report are still OK."
exit 1
