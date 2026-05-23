#!/usr/bin/env bash
# check-brief-block-history.sh — refuse to (re-)author stale-BLOCKED briefs.
#
# Step 8 (queue replenishment) re-authored `claude-mcp-connector-empty-dir-cleanup`
# to TODO on 2026-05-23 despite a BLOCKED commit (7418b5045) from earlier the
# same day. Two claude workers then wasted iterations rediscovering the same
# impossible premise. This guard catches the case before re-author.
#
# Usage:
#   scripts/check-brief-block-history.sh <brief-id>
#
# Exit codes:
#   0 — no prior BLOCK in git history for this brief-id (safe to author)
#   1 — prior BLOCK found in git history (refuse — prints SHAs + dates)
#   2 — ledger already has a non-TODO row for this brief-id (refuse — re-author would clobber)
#
# Run before writing a brief stub to docs/build-plan/streams/ AND before
# inserting a TODO row into docs/build-plan/BUILD_STATUS.md.

set -uo pipefail

BRIEF_ID="${1:-}"
if [[ -z "$BRIEF_ID" ]]; then
  echo "usage: $0 <brief-id>" >&2
  exit 3
fi

REPO_ROOT="${ARGO_BLOCK_HISTORY_REPO:-$(cd "$(dirname "$0")/.." && pwd)}"
LEDGER="$REPO_ROOT/docs/build-plan/BUILD_STATUS.md"

if [[ ! -f "$LEDGER" ]]; then
  echo "FAIL: ledger not found at $LEDGER" >&2
  exit 3
fi

# 1. Pre-insert check: ledger currently carries a non-TODO row for this id.
#    If so, re-authoring would either clobber a live row or duplicate it.
#    Match the pipe-delimited table form: "| <brief-id> | <owner> | <status> |".
CURRENT_ROW=$(grep -E "^\| ${BRIEF_ID} \| [^|]+ \| [A-Z_-]+ \|" "$LEDGER" | head -1 || true)
if [[ -n "$CURRENT_ROW" ]]; then
  CURRENT_STATUS=$(echo "$CURRENT_ROW" | awk -F'|' '{gsub(/^[ \t]+|[ \t]+$/, "", $4); print $4}')
  if [[ "$CURRENT_STATUS" != "TODO" && -n "$CURRENT_STATUS" ]]; then
    echo "🚫 REFUSE: $BRIEF_ID already has a $CURRENT_STATUS row on the ledger." >&2
    echo "   Re-authoring would clobber the live row. Resolve the existing row first." >&2
    exit 2
  fi
fi

# 2. Pre-author check: prior commits flipped this brief to BLOCKED.
#    Two passes — the brief spec calls for both:
#    (a) commit-message grep for the brief-id, filtered to BLOCKED messages.
#    (b) commit-content grep (-S) for the brief-id on the ledger, filtered to
#        commits whose message says "blocked" (catches BLOCK rows landed via
#        non-status-prefixed commits).
HITS=$(
  {
    git -C "$REPO_ROOT" log --all --oneline --grep="$BRIEF_ID" -- docs/build-plan/BUILD_STATUS.md 2>/dev/null \
      | grep -iE "blocked"
    git -C "$REPO_ROOT" log --all --oneline -S"$BRIEF_ID" -- docs/build-plan/BUILD_STATUS.md 2>/dev/null \
      | grep -iE "blocked"
  } | sort -u
)

if [[ -n "$HITS" ]]; then
  echo "🚫 REFUSE: $BRIEF_ID has prior BLOCK history on origin/main." >&2
  echo "" >&2
  echo "Blocking commits (most recent first):" >&2
  while IFS= read -r line; do
    SHA=$(echo "$line" | awk '{print $1}')
    DATE=$(git -C "$REPO_ROOT" show -s --format=%cI "$SHA" 2>/dev/null || echo "?")
    echo "  • $SHA  $DATE  ${line#* }" >&2
  done <<< "$HITS"
  echo "" >&2
  echo "Do NOT re-author. Pick a different brief or escalate to the operator." >&2
  exit 1
fi

echo "✅ $BRIEF_ID has no prior BLOCK history — safe to author." >&2
exit 0
