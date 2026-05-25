#!/usr/bin/env bash
# Smoke tests for scripts/check-ledger-todo-owners.sh.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$REPO_ROOT/scripts/check-ledger-todo-owners.sh"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

pass=0
fail=0

record_pass() {
  echo "  PASS  $1"
  pass=$((pass + 1))
}

record_fail() {
  echo "  FAIL  $1" >&2
  fail=$((fail + 1))
}

expect_rc() {
  local label="$1" expected="$2"
  shift 2
  local out rc
  out="$("$@" 2>&1)"
  rc=$?
  printf '%s' "$out" > "$TMPDIR/out.txt"
  if [ "$rc" -eq "$expected" ]; then
    record_pass "$label rc=$expected"
  else
    record_fail "$label expected rc=$expected, got rc=$rc output=$out"
  fi
}

expect_output() {
  local label="$1" pattern="$2"
  if grep -q -E "$pattern" "$TMPDIR/out.txt"; then
    record_pass "$label"
  else
    record_fail "$label missing /$pattern/ in $(cat "$TMPDIR/out.txt")"
  fi
}

cat > "$TMPDIR/exact.md" <<'EOF'
# Build status

| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| codex-row | codex | TODO | — | ok |
| claude-row | claude | TODO | — | ok |
| mixed-merged | claude or codex | MERGED | #1 | historical mixed owner |
| mixed-blocked | either-pool | BLOCKED | — | historical mixed owner |
| mixed-dropped | claude/codex | DROPPED | — | historical mixed owner |
EOF

cat > "$TMPDIR/mixed-todo.md" <<'EOF'
# Build status

| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| exact-codex | codex | TODO | — | ok |
| mixed-active | claude or codex | TODO | — | should fail |
| exact-claude | claude | TODO | — | ok |
EOF

echo "=== check-ledger-todo-owners.sh smoke ==="

expect_rc "exact claude/codex TODO rows accepted" 0 \
  bash "$SCRIPT" --ledger="$TMPDIR/exact.md"
expect_output "success message names exact owners" "OK: selected BUILD_STATUS.md rows"

expect_rc "mixed active TODO owner rejected" 1 \
  bash "$SCRIPT" --ledger="$TMPDIR/mixed-todo.md"
expect_output "failure names mixed row" "mixed-active owner=claude or codex status=TODO"
expect_output "failure gives conductor replenishment guidance" \
  "flip mixed TODO owners to an exact pool"

expect_rc "historical mixed owners ignored by default" 0 \
  bash "$SCRIPT" --ledger="$TMPDIR/exact.md"

expect_rc "historical statuses checked when explicitly configured" 1 \
  bash "$SCRIPT" --ledger="$TMPDIR/exact.md" --statuses=TODO,MERGED,BLOCKED,DROPPED
expect_output "explicit status check catches MERGED mixed owner" \
  "mixed-merged owner=claude or codex status=MERGED"
expect_output "explicit status check catches BLOCKED mixed owner" \
  "mixed-blocked owner=either-pool status=BLOCKED"
expect_output "explicit status check catches DROPPED mixed owner" \
  "mixed-dropped owner=claude/codex status=DROPPED"

expect_rc "missing ledger is exit 2" 2 \
  bash "$SCRIPT" --ledger="$TMPDIR/missing.md"
expect_output "missing ledger explains failure" "ledger not found"

echo
echo "Results: $pass passed, $fail failed."
[ "$fail" -eq 0 ]
