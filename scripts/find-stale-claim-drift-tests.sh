#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$REPO_ROOT/scripts/find-stale-claim-drift.py"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

LEDGER="$TMP/BUILD_STATUS.md"
cat >"$LEDGER" <<'EOF'
| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| claimed-stale | codex | CLAIMED | — | **STALE-CLAIM-DEMOTE**: no remote-branch push in 2h. Returning to TODO. |
| review-stale | claude | IN_REVIEW | #1 | **STALE-SWEEP** stale, demote after review timeout; note keeps a literal `a|b` pipe. |
| claimed-fresh | claude | CLAIMED | — | claimed via claim-with-retry; no stale directive. |
| casual-stale | claude | CLAIMED | — | Mentions stale data in passing but has no marker or release verb. |
| meta-doc | claude | CLAIMED | — | NEW tool flags rows whose note carries STALE-CLAIM-DEMOTE marker plus "Returning to TODO" verb. |
| todo-stale | claude | TODO | — | **STALE-ROAR-INVALIDATION** Returning to TODO already done. |
EOF

set +e
OUTPUT="$("$SCRIPT" "$LEDGER" 2>&1)"
STATUS=$?
set -e

[[ "$STATUS" -eq 1 ]]
grep -q "Found 2 rows with stale-claim drift" <<<"$OUTPUT"
grep -q "claimed-stale | CLAIMED | STALE-CLAIM-DEMOTE" <<<"$OUTPUT"
grep -q "review-stale | IN_REVIEW | STALE-SWEEP" <<<"$OUTPUT"
! grep -q "casual-stale" <<<"$OUTPUT"
! grep -q "meta-doc" <<<"$OUTPUT"
! grep -q "todo-stale" <<<"$OUTPUT"

EMPTY="$TMP/EMPTY.md"
cat >"$EMPTY" <<'EOF'
| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| claimed-fresh | claude | CLAIMED | — | stale is mentioned casually only. |
EOF
"$SCRIPT" --quiet "$EMPTY" | grep -q "Found 0 rows with stale-claim drift"
"$SCRIPT" --help | grep -q "Cron example:"

echo "find-stale-claim-drift tests passed"
