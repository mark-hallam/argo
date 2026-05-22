#!/usr/bin/env bash
# Smoke tests for `scripts/check-findings-only-fix-by.sh`. Builds
# a temp BUILD_STATUS.md with four hand-crafted rows covering every
# branch of the date-parser + classifier:
#
#   1. row WITHOUT a `fix-by` token         → 🟢 informational
#   2. row with `fix-by` in the future      → 🟢 within budget
#   3. row with `fix-by` matching today     → 🟡 due today
#   4. row with `fix-by` in the past        → 🔴 overdue
#
# A fifth row (status MERGED, not MERGED-FINDINGS-ONLY) confirms
# the row-selection anchors on the status column and does NOT
# accidentally pattern-match the literal `MERGED-FINDINGS-ONLY`
# string embedded in a normal MERGED note's history text — the
# regression that motivated the brief's pre-flight note.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$REPO_ROOT/scripts/check-findings-only-fix-by.sh"
TODAY="2026-05-23"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

cat > "$TMPDIR/BUILD_STATUS.md" <<'EOF'
# Build status — live ledger

| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| row-no-fix-by | claude | MERGED-FINDINGS-ONLY | #1 | No deadline carried; back-compat informational. |
| row-future | claude | MERGED-FINDINGS-ONLY | #2 | Fix-brief queued; fix-by 2026-12-31 — well in the future. |
| row-today | claude | MERGED-FINDINGS-ONLY | #3 | Due today: fix-by 2026-05-23 — should classify yellow. |
| row-overdue | claude | MERGED-FINDINGS-ONLY | #4 | Past deadline fix-by 2026-05-01 — should classify red. |
| row-already-merged-with-FINDINGS-text | claude | MERGED | #5 | This row is MERGED. It mentions MERGED-FINDINGS-ONLY in narrative context — the sweep must NOT pick it up. |
EOF

ACTUAL=$(bash "$SCRIPT" --ledger="$TMPDIR/BUILD_STATUS.md" --today="$TODAY" 2>&1)
RC=$?

pass=0
fail=0
expect() {
  local label="$1" pattern="$2"
  if echo "$ACTUAL" | grep -q -E "$pattern"; then
    echo "  PASS  $label"
    pass=$((pass + 1))
  else
    echo "  FAIL  $label — expected /$pattern/ in output"
    fail=$((fail + 1))
  fi
}

echo "=== check-findings-only-fix-by.sh smoke ==="
expect "informational row green"            "🟢 row-no-fix-by"
expect "future deadline green"              "🟢 row-future.*within budget"
expect "today deadline yellow"              "🟡 row-today  due today"
expect "overdue deadline red"               "🔴 row-overdue  overdue"
expect "non-FINDINGS-ONLY row not surfaced" "^Tally: 1 within-budget · 1 due-today · 1 overdue · 1 informational"
if [ "$RC" -eq 1 ]; then
  echo "  PASS  exit code is 1 (overdue present)"
  pass=$((pass + 1))
else
  echo "  FAIL  exit code is $RC, expected 1"
  fail=$((fail + 1))
fi

# Second pass: drop the overdue row and confirm exit 0.
grep -v row-overdue "$TMPDIR/BUILD_STATUS.md" > "$TMPDIR/no-overdue.md"
bash "$SCRIPT" --ledger="$TMPDIR/no-overdue.md" --today="$TODAY" > /dev/null
RC2=$?
if [ "$RC2" -eq 0 ]; then
  echo "  PASS  exit code is 0 when no overdue rows"
  pass=$((pass + 1))
else
  echo "  FAIL  exit code is $RC2 without overdue, expected 0"
  fail=$((fail + 1))
fi

# Third pass: empty ledger → exit 0, friendly message.
echo "# empty ledger" > "$TMPDIR/empty.md"
OUT=$(bash "$SCRIPT" --ledger="$TMPDIR/empty.md" --today="$TODAY" 2>&1)
RC3=$?
if [ "$RC3" -eq 0 ] && echo "$OUT" | grep -q "nothing to sweep"; then
  echo "  PASS  empty ledger → exit 0 + nothing-to-sweep message"
  pass=$((pass + 1))
else
  echo "  FAIL  empty ledger: rc=$RC3 out=$OUT"
  fail=$((fail + 1))
fi

# Fourth pass: malformed --today → exit 2.
bash "$SCRIPT" --ledger="$TMPDIR/BUILD_STATUS.md" --today="not-a-date" > /dev/null 2>&1
RC4=$?
if [ "$RC4" -eq 2 ]; then
  echo "  PASS  malformed --today → exit 2"
  pass=$((pass + 1))
else
  echo "  FAIL  malformed --today: rc=$RC4, expected 2"
  fail=$((fail + 1))
fi

echo
echo "Results: $pass passed, $fail failed."
[ "$fail" -eq 0 ]
