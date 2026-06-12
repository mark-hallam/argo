#!/usr/bin/env bash
# ledger-first-todo-tests.sh — bash assertions for the shared, column-anchored
# ledger row-parser (scripts/ledger-first-todo.sh) used by BOTH the brief
# picker (claude-worker-loop.sh) and the claimer (claim-with-retry.sh).
#
# Regression for the 2026-06-12 SECOND swarm-starvation incident: a greedy
# whole-line picker regex (`/^\| .* \| .* \| TODO \|/`) mis-picked an IN_REVIEW
# row whose DESCRIPTION cell quoted a literal `| TODO |` substring, the claimer
# correctly refused, and the swarm starved. The fix parses the STATUS column
# (field 4) directly. These tests pin: (a) a description that quotes `| TODO |`
# is NOT picked, (b) the real TODO below it IS picked, (c) an empty col-4 TODO
# is still picked (parity with #2864), and (d) the picker (ledger_todo_ids) and
# the claimer (ledger_status_of) AGREE on every row — the property that, when
# violated, caused the starvation.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=scripts/ledger-first-todo.sh disable=SC1091
. "$SCRIPT_DIR/ledger-first-todo.sh"
PASSED=0
FAILED=0

assert_eq() {
    local actual="$1" expected="$2" name="$3"
    if [[ "$actual" == "$expected" ]]; then
        echo "✓ $name"
        PASSED=$((PASSED+1))
    else
        echo "✗ $name: expected '$expected', got '$actual'" >&2
        FAILED=$((FAILED+1))
    fi
}

# Fixture ledger. The decoy IN_REVIEW row's DESCRIPTION cell embeds a literal
# `| TODO |` sequence — exactly the malformed shape that starved the swarm.
LEDGER="$(mktemp)"
cat > "$LEDGER" <<'EOF'
# Build status

| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| decoy-in-review | claude | IN_REVIEW | #2864 | fixed the malformed row shape, quoted | TODO | pipes included |
| real-todo | claude | TODO | — | the genuine first claimable row |
| empty-col4-todo | claude or codex | TODO | | empty col-4, no PR yet |
| a-claimed | claude | CLAIMED | — | taken by another tab |
| a-blocked | codex | BLOCKED | — | needs unblock |
EOF

# ── Test 1: first TODO is the real row, NOT the |TODO|-quoting decoy ──
echo "[1/8] first TODO skips the description-quoting decoy"
assert_eq "$(ledger_first_todo "$LEDGER")" "real-todo" "first-todo: returns real-todo (not decoy-in-review)"

# ── Test 2: the full TODO list excludes the decoy, includes both real TODOs ──
echo "[2/8] ledger_todo_ids order + membership"
ids="$(ledger_todo_ids "$LEDGER")"
assert_eq "$ids" "$(printf 'real-todo\nempty-col4-todo')" "todo-ids: real-todo then empty-col4-todo, no decoy"

# ── Test 3: the decoy IN_REVIEW row is never listed as TODO ──
echo "[3/8] decoy never appears in the TODO list"
if grep -q '^decoy-in-review$' <<<"$ids"; then
    echo "✗ decoy-absent: decoy-in-review leaked into the TODO list" >&2
    FAILED=$((FAILED+1))
else
    echo "✓ decoy-absent: decoy-in-review excluded"
    PASSED=$((PASSED+1))
fi

# ── Test 4: empty col-4 TODO is still picked (parity with #2864) ──
echo "[4/8] empty col-4 TODO is claimable"
assert_eq "$(ledger_status_of "$LEDGER" empty-col4-todo)" "todo" "empty-col4: classified todo"

# ── Test 5: the decoy's status is read from the column, not the description ──
echo "[5/8] decoy status read from column 4, not description"
assert_eq "$(ledger_status_of "$LEDGER" decoy-in-review)" "other" "decoy: classified other (IN_REVIEW), not todo"

# ── Test 6: other status tokens classify correctly ──
echo "[6/8] claimed / blocked / not-found tokens"
assert_eq "$(ledger_status_of "$LEDGER" a-claimed)" "claimed" "claimed: classified claimed"
assert_eq "$(ledger_status_of "$LEDGER" a-blocked)" "blocked" "blocked: classified blocked"
assert_eq "$(ledger_status_of "$LEDGER" no-such-brief)" "not-found" "missing: classified not-found"

# ── Test 7: real-todo classifies as todo (round-trip with the picker) ──
echo "[7/8] picked row classifies as todo"
assert_eq "$(ledger_status_of "$LEDGER" real-todo)" "todo" "real-todo: classified todo"

# ── Test 8: picker and claimer AGREE on every brief row ──
# This is the load-bearing invariant: a row is in the picker's TODO list IFF
# the claimer classifies it 'todo'. Disagreement here is the starvation bug.
echo "[8/8] picker ⇔ claimer agreement across all rows"
agree=1
for brief in decoy-in-review real-todo empty-col4-todo a-claimed a-blocked; do
    in_list=0
    grep -q "^${brief}\$" <<<"$ids" && in_list=1
    is_todo=0
    [[ "$(ledger_status_of "$LEDGER" "$brief")" == "todo" ]] && is_todo=1
    if [[ "$in_list" != "$is_todo" ]]; then
        echo "✗ agreement: '$brief' picker=$in_list claimer-todo=$is_todo" >&2
        agree=0
    fi
done
if [[ "$agree" == "1" ]]; then
    echo "✓ agreement: picker and claimer agree on every row"
    PASSED=$((PASSED+1))
else
    FAILED=$((FAILED+1))
fi

rm -f "$LEDGER"

# ── Summary ───────────────────────────────────────────────────────
echo ""
echo "=== Results: $PASSED passed, $FAILED failed ==="
[[ $FAILED -eq 0 ]]
