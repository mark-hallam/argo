#!/usr/bin/env bash
# check-runaway-zombie-claims.sh — detect cross-iteration zombie-claim storms.
#
# Workers pick the first-eligible TODO row by document order. When a worker
# fails to ship (zombie release), the next iteration immediately re-claims
# the same row. With nothing breaking that cycle, a single broken brief can
# burn the full worker pool indefinitely (R102 storm: 83 zombie cycles on
# one brief in 16 min). This check is what worker-supervisor.sh calls
# between iterations to short-circuit the loop.
#
# Counts, per brief-id, how many `status: release <brief-id> (zombie; no PR
# opened)` commits hit origin/main in the last N minutes, then returns a
# 3-state signal:
#
# Usage:
#   bash scripts/check-runaway-zombie-claims.sh \
#     [--window-minutes N] [--threshold M] [--block-threshold K]
#
# Exit codes:
#   0 — clean (max-zombie-count < threshold)
#   2 — throttle (threshold ≤ max-count < block-threshold) — skip this iter
#   3 — block-required (max-count ≥ block-threshold) — force row to BLOCKED
#
# Defaults: --window-minutes 30 --threshold 3 --block-threshold 6
#
# Stdout (non-zero exit, machine-readable):
#   brief=<id> count=<N> window=<X>min action=<skip|block>
# Stderr (non-zero exit):
#   [supervisor-block] brief=<id> reason=runaway-zombie-claims count=<N> window=<X>min action=<skip|block>

set -uo pipefail

WINDOW_MIN=30
THRESHOLD=3
BLOCK_THRESHOLD=6

while [[ $# -gt 0 ]]; do
  case "$1" in
    --window-minutes) WINDOW_MIN="$2"; shift 2 ;;
    --threshold) THRESHOLD="$2"; shift 2 ;;
    --block-threshold) BLOCK_THRESHOLD="$2"; shift 2 ;;
    -h|--help)
      sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown arg: $1" >&2
      exit 64
      ;;
  esac
done

REPO_ROOT="${ARGO_RUNAWAY_CHECK_REPO:-$(cd "$(dirname "$0")/.." && pwd)}"

# Worktrees use a .git file (not a dir), so use rev-parse instead of `-d`.
if ! git -C "$REPO_ROOT" rev-parse --git-dir >/dev/null 2>&1; then
  # No git history → no storm possible. Stay quiet (treat as clean).
  exit 0
fi

# Collect zombie-release brief-ids in the window. The release commit shape
# (from scripts/claude-worker-loop.sh lines 696-730) is:
#   "status: release <brief-id> (zombie; no PR opened)"
# We extract just <brief-id> and count occurrences.
COUNTS=$(
  git -C "$REPO_ROOT" log \
      --since="${WINDOW_MIN} minutes ago" \
      --pretty="%s" \
      origin/main 2>/dev/null \
    | sed -nE 's/^status: release ([^ ]+) \(zombie; no PR opened\)$/\1/p' \
    | sort \
    | uniq -c \
    | sort -rn
)

if [[ -z "$COUNTS" ]]; then
  exit 0
fi

# Top line wins (uniq -c | sort -rn) — that's the worst-offender brief.
TOP_LINE=$(echo "$COUNTS" | head -1)
TOP_COUNT=$(echo "$TOP_LINE" | awk '{print $1}')
TOP_BRIEF=$(echo "$TOP_LINE" | awk '{print $2}')

if [[ "$TOP_COUNT" -lt "$THRESHOLD" ]]; then
  exit 0
fi

if [[ "$TOP_COUNT" -ge "$BLOCK_THRESHOLD" ]]; then
  ACTION=block
  RC=3
else
  ACTION=skip
  RC=2
fi

echo "brief=${TOP_BRIEF} count=${TOP_COUNT} window=${WINDOW_MIN}min action=${ACTION}"
echo "[supervisor-block] brief=${TOP_BRIEF} reason=runaway-zombie-claims count=${TOP_COUNT} window=${WINDOW_MIN}min action=${ACTION}" >&2
exit "$RC"
