#!/usr/bin/env bash
# Tests for check-ledger-no-empty-col4-todo.sh — the lint that keeps an
# empty col-4 TODO row (`| TODO | |`) out of BUILD_STATUS.md (the format
# that silently starved the whole swarm, root-caused 2026-06-12).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$SCRIPT_DIR/check-ledger-no-empty-col4-todo.sh"
PASSED=0
FAILED=0

assert_eq() {
  local actual="$1" expected="$2" name="$3"
  if [[ "$actual" == "$expected" ]]; then
    echo "✓ $name"; PASSED=$((PASSED+1))
  else
    echo "✗ $name: expected '$expected', got '$actual'" >&2; FAILED=$((FAILED+1))
  fi
}

assert_grep() {
  local file="$1" pattern="$2" name="$3"
  if grep -q -- "$pattern" "$file" 2>/dev/null; then
    echo "✓ $name"; PASSED=$((PASSED+1))
  else
    echo "✗ $name: pattern '$pattern' not in $file" >&2; FAILED=$((FAILED+1))
  fi
}

write_ledger() {
  local path="$1"; shift
  cat > "$path" <<'EOF'
# Build status

| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
EOF
  printf '%s\n' "$@" >> "$path"
}

# ── Test 1: clean ledger (em-dash placeholders) passes ────────────
echo "[1/4] clean ledger passes"
tmp="$(mktemp -d)"
write_ledger "$tmp/L.md" \
  "| good-todo | claude | TODO | — | unstarted |" \
  "| done-row | claude | MERGED | #123 | shipped |"
rc=0
"$SCRIPT" --ledger="$tmp/L.md" --quiet >/tmp/argo-col4-1.out 2>&1 || rc=$?
assert_eq "$rc" "0" "clean: rc=0"
rm -rf "$tmp"

# ── Test 2: empty col-4 TODO row is rejected ──────────────────────
echo "[2/4] empty col-4 TODO rejected"
tmp="$(mktemp -d)"
write_ledger "$tmp/L.md" \
  "| bad-todo | claude | TODO | | starves the swarm |"
rc=0
"$SCRIPT" --ledger="$tmp/L.md" >/tmp/argo-col4-2.out 2>&1 || rc=$?
assert_eq "$rc" "1" "empty-col4: rc=1"
assert_grep /tmp/argo-col4-2.out "bad-todo" "empty-col4: names offending brief"
assert_grep /tmp/argo-col4-2.out "EMPTY col-4" "empty-col4: explains the format bug"
rm -rf "$tmp"

# ── Test 3: empty col-4 on NON-TODO rows is ignored ───────────────
# (a MERGED/BLOCKED row with empty col-4 doesn't starve anything.)
echo "[3/4] empty col-4 on non-TODO ignored"
tmp="$(mktemp -d)"
write_ledger "$tmp/L.md" \
  "| blocked-row | claude | BLOCKED | | needs unblock |" \
  "| good-todo | claude | TODO | — | unstarted |"
rc=0
"$SCRIPT" --ledger="$tmp/L.md" --quiet >/tmp/argo-col4-3.out 2>&1 || rc=$?
assert_eq "$rc" "0" "non-todo-empty: rc=0"
rm -rf "$tmp"

# ── Test 4: a whitespace-only col-4 TODO is still rejected ─────────
echo "[4/4] whitespace-only col-4 TODO rejected"
tmp="$(mktemp -d)"
write_ledger "$tmp/L.md" \
  "| ws-todo | claude | TODO |   | two spaces |"
rc=0
"$SCRIPT" --ledger="$tmp/L.md" >/tmp/argo-col4-4.out 2>&1 || rc=$?
assert_eq "$rc" "1" "whitespace-col4: rc=1"
rm -rf "$tmp"

echo ""
echo "=== Results: $PASSED passed, $FAILED failed ==="
[[ $FAILED -eq 0 ]]
