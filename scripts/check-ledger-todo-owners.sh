#!/usr/bin/env bash
# Check BUILD_STATUS.md for active TODO rows with non-exact owners.
#
# Usage:
#   bash scripts/check-ledger-todo-owners.sh [--ledger PATH] [--statuses TODO[,STATUS...]]
#   bash scripts/check-ledger-todo-owners.sh --help
#
# Default policy checks only active TODO rows. Historical statuses such
# as MERGED, DROPPED, and BLOCKED are intentionally ignored unless a
# conductor passes them via --statuses. During queue replenishment,
# conductors should flip mixed owners such as `claude or codex`,
# `either-pool`, or `claude/codex` to an exact `claude` or `codex`
# owner before committing the ledger row.
#
# Exit codes:
#   0 = all selected rows have exact owners
#   1 = at least one selected row has a non-exact owner
#   2 = invalid arguments or missing ledger

set -uo pipefail

LEDGER=""
STATUSES="TODO"

for arg in "$@"; do
  case "$arg" in
    --ledger=*) LEDGER="${arg#*=}" ;;
    --statuses=*) STATUSES="${arg#*=}" ;;
    --help|-h)
      sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "FAIL: unknown arg '$arg' (try --help)" >&2
      exit 2
      ;;
  esac
done

if [ -z "$LEDGER" ]; then
  REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
  LEDGER="$REPO_ROOT/docs/build-plan/BUILD_STATUS.md"
fi

if [ ! -f "$LEDGER" ]; then
  echo "FAIL: ledger not found at $LEDGER" >&2
  exit 2
fi

if [ -z "$STATUSES" ]; then
  echo "FAIL: --statuses must not be empty" >&2
  exit 2
fi

findings=$(
  awk -F'|' -v statuses="$STATUSES" '
    BEGIN {
      n = split(statuses, raw, ",")
      for (i = 1; i <= n; i++) {
        s = raw[i]
        gsub(/^ +| +$/, "", s)
        if (s != "") selected[s] = 1
      }
    }
    /^\| [A-Za-z0-9_.-]+[[:space:]]*\|/ {
      brief = $2
      owner = $3
      status = $4
      gsub(/^ +| +$/, "", brief)
      gsub(/^ +| +$/, "", owner)
      gsub(/^ +| +$/, "", status)
      if (!(status in selected)) next
      if (owner != "claude" && owner != "codex") {
        printf "%d:%s owner=%s status=%s\n", FNR, brief, owner, status
      }
    }
  ' "$LEDGER"
)

if [ -n "$findings" ]; then
  echo "FAIL: selected BUILD_STATUS.md rows must use exact owner 'claude' or 'codex':" >&2
  echo "$findings" >&2
  echo >&2
  echo "During replenishment, flip mixed TODO owners to an exact pool before committing." >&2
  exit 1
fi

echo "OK: selected BUILD_STATUS.md rows have exact owners ($STATUSES)."
exit 0
