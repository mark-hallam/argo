#!/usr/bin/env bash
# Smoke tests for `scripts/check-todo-aging.sh`. Builds a temp
# BUILD_STATUS.md with hand-crafted rows covering every branch
# of the classifier:
#
#   - TODO + P0 + claimed <12h ago    → 🟢 within budget
#   - TODO + P0 + claimed 30h ago     → 🟠 aging
#   - TODO + P0 + claimed 60h ago     → 🔴 OVERDUE
#   - TODO + P0 + date-only NEW token → 🟢 (midnight-UTC anchor)
#   - TODO + 🚨 P0 OPERATOR-LOCKED    → still P0-tagged → classified
#   - TODO without P0                 → not surfaced (only P0 rows)
#   - MERGED row with P0 in narrative → not surfaced (status != TODO)

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$REPO_ROOT/scripts/check-todo-aging.sh"
NOW="2026-05-23T12:00Z"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

cat > "$TMPDIR/BUILD_STATUS.md" <<'EOF'
# Build status — live ledger

| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| row-p0-fresh | claude | TODO | — | 🔴 P0 claimed at 2026-05-23T11:00Z; should be within budget |
| row-p0-aging | claude | TODO | — | 🔴 P0 NEW 2026-05-22T06:00Z; 30h old → aging |
| row-p0-overdue | claude | TODO | — | 🔴 P0 NEW 2026-05-21T00:00Z; 60h old → overdue |
| row-p0-dateonly | claude | TODO | — | 🔴 P0 NEW 2026-05-23 — no time component; anchor at midnight UTC |
| row-operator-locked | claude | TODO | — | 🚨 P0 OPERATOR-LOCKED 2026-05-23T08:00Z by operator |
| row-p1-not-tagged | claude | TODO | — | 🟡 P1 routine work, NEW 2026-05-19T00:00Z — not P0 so silent |
| row-merged-narrative-p0 | claude | MERGED | #1 | post-mortem note mentions the P0 in narrative — status is MERGED, must not surface |
EOF

ACTUAL=$(bash "$SCRIPT" --ledger="$TMPDIR/BUILD_STATUS.md" --now="$NOW" 2>&1)
RC=$?

pass=0
fail=0
expect() {
  local label="$1" pat="$2"
  if echo "$ACTUAL" | grep -q -E "$pat"; then
    echo "  PASS  $label"
    pass=$((pass + 1))
  else
    echo "  FAIL  $label — expected /$pat/ in output"
    fail=$((fail + 1))
  fi
}
deny() {
  local label="$1" pat="$2"
  if echo "$ACTUAL" | grep -q -E "$pat"; then
    echo "  FAIL  $label — unwanted match /$pat/ in output"
    fail=$((fail + 1))
  else
    echo "  PASS  $label"
    pass=$((pass + 1))
  fi
}

echo "=== check-todo-aging.sh smoke ==="
expect "fresh P0 → 🟢 within budget"    "🟢 row-p0-fresh.*within budget.*1h since"
expect "aging P0 → 🟠 aging"            "🟠 row-p0-aging.*aging.*30h since"
expect "overdue P0 → 🔴 OVERDUE"        "🔴 row-p0-overdue.*OVERDUE.*60h since"
expect "date-only NEW → midnight anchor" "🟢 row-p0-dateonly.*within budget.*12h since 2026-05-23T00:00Z"
expect "OPERATOR-LOCKED row picked up"  "row-operator-locked"
deny   "P1 row NOT surfaced"            "row-p1-not-tagged"
deny   "MERGED row NOT surfaced"        "row-merged-narrative-p0"
expect "tally line"                     "^Tally: [0-9]+ within-budget · [0-9]+ aging · [0-9]+ overdue"
if [ "$RC" -eq 0 ]; then
  echo "  PASS  exit 0 (informational, never escalates)"
  pass=$((pass + 1))
else
  echo "  FAIL  expected exit 0, got $RC"
  fail=$((fail + 1))
fi

# Second pass: malformed --now → exit 2.
bash "$SCRIPT" --ledger="$TMPDIR/BUILD_STATUS.md" --now="not-a-time" > /dev/null 2>&1
RC2=$?
if [ "$RC2" -eq 2 ]; then
  echo "  PASS  malformed --now → exit 2"
  pass=$((pass + 1))
else
  echo "  FAIL  malformed --now expected exit 2, got $RC2"
  fail=$((fail + 1))
fi

# Third pass: empty ledger → exit 0 + friendly message.
echo "# empty ledger" > "$TMPDIR/empty.md"
OUT=$(bash "$SCRIPT" --ledger="$TMPDIR/empty.md" --now="$NOW" 2>&1)
RC3=$?
if [ "$RC3" -eq 0 ] && echo "$OUT" | grep -q "nothing to age"; then
  echo "  PASS  empty ledger → exit 0 + nothing-to-age"
  pass=$((pass + 1))
else
  echo "  FAIL  empty ledger: rc=$RC3 out=$OUT"
  fail=$((fail + 1))
fi

echo
echo "Results: $pass passed, $fail failed."
[ "$fail" -eq 0 ]
