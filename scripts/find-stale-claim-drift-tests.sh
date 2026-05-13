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
| rebase-pr-stale | claude | IN_REVIEW | #2 | Recovery brief, but **STALE-FIXED**: underlying PR already merged via #3. |
| dup-brief | claude | TODO | — | first row. |
| dup-brief | claude | CLAIMED | — | second row, conflicting. |
| dup-merged | claude | MERGED | #4 | first historical row. |
| dup-merged | claude | MERGED | #5 | second historical row, both terminal so ignored. |
EOF

set +e
OUTPUT="$("$SCRIPT" "$LEDGER" 2>&1)"
STATUS=$?
set -e

[[ "$STATUS" -eq 1 ]]
grep -q "Found 4 drift rows" <<<"$OUTPUT"
grep -q "directive-mismatch=2" <<<"$OUTPUT"
grep -q "stale-recovery=1" <<<"$OUTPUT"
grep -q "duplicate-row=1" <<<"$OUTPUT"
grep -q "\[directive-mismatch\] claimed-stale | CLAIMED | STALE-CLAIM-DEMOTE" <<<"$OUTPUT"
grep -q "\[directive-mismatch\] review-stale | IN_REVIEW | STALE-SWEEP" <<<"$OUTPUT"
grep -q "\[stale-recovery\] rebase-pr-stale | IN_REVIEW | STALE-FIXED" <<<"$OUTPUT"
grep -q "\[duplicate-row\] dup-brief | CLAIMED,TODO | x2" <<<"$OUTPUT"
! grep -q "casual-stale" <<<"$OUTPUT"
! grep -q "meta-doc" <<<"$OUTPUT"
! grep -q "todo-stale" <<<"$OUTPUT"
! grep -q "dup-merged" <<<"$OUTPUT"

EMPTY="$TMP/EMPTY.md"
cat >"$EMPTY" <<'EOF'
| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| claimed-fresh | claude | CLAIMED | — | stale is mentioned casually only. |
EOF
"$SCRIPT" --quiet "$EMPTY" | grep -q "Found 0 drift rows"
"$SCRIPT" --help | grep -q "Cron example:"

echo "find-stale-claim-drift tests passed"
