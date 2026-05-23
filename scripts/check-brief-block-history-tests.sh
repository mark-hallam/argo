#!/usr/bin/env bash
# check-brief-block-history-tests.sh — bash smoke tests for
# check-brief-block-history.sh.
#
# Builds an isolated temp git repo, plants synthetic ledger states + commit
# history, and exercises each exit-code branch (0 clean / 1 prior-BLOCK /
# 2 currently-non-TODO). Also asserts the script catches the real-world
# `claude-mcp-connector-empty-dir-cleanup` brief on the live repo.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$SCRIPT_DIR/check-brief-block-history.sh"
LIVE_REPO="$(cd "$SCRIPT_DIR/.." && pwd)"
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

assert_grep() {
  local file="$1" pattern="$2" name="$3"
  if grep -q -- "$pattern" "$file" 2>/dev/null; then
    echo "✓ $name"
    PASSED=$((PASSED+1))
  else
    echo "✗ $name: pattern '$pattern' not found in $file" >&2
    FAILED=$((FAILED+1))
  fi
}

setup_repo() {
  local tmp="$1"
  mkdir -p "$tmp/docs/build-plan"
  git -C "$tmp" init --initial-branch=main >/dev/null 2>&1
  git -C "$tmp" config user.name "tester"
  git -C "$tmp" config user.email "tester@test.invalid"
  git -C "$tmp" config commit.gpgsign false
  cat > "$tmp/docs/build-plan/BUILD_STATUS.md" <<'EOF'
# Build status

| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| seed-row | claude | MERGED | #1 | initial |
EOF
  git -C "$tmp" add . >/dev/null
  git -C "$tmp" commit -m "init ledger" >/dev/null
}

# ── Test 1: clean brief id → exit 0 ───────────────────────────────────
echo "[1/5] clean brief id (no history, no current row)"
tmp="$(mktemp -d)"
setup_repo "$tmp"
rc=0
ARGO_BLOCK_HISTORY_REPO="$tmp" "$SCRIPT" some-brand-new-brief >/tmp/argo-bh-test1.out 2>&1 || rc=$?
assert_eq "$rc" "0" "clean: rc=0"
assert_grep /tmp/argo-bh-test1.out "safe to author" "clean: prints safe-to-author"
rm -rf "$tmp"

# ── Test 2: prior BLOCK in git history, no current ledger row → exit 1 ─
echo "[2/5] prior BLOCK in git history, ledger now clean"
tmp="$(mktemp -d)"
setup_repo "$tmp"
# Plant a TODO row, commit, then BLOCK it, then remove the row entirely.
cat >> "$tmp/docs/build-plan/BUILD_STATUS.md" <<'EOF'
| ghost-brief | claude | TODO | — | newly authored |
EOF
git -C "$tmp" add . >/dev/null
git -C "$tmp" commit -m "status: claim ghost-brief" >/dev/null
sed -i.bak 's/| ghost-brief | claude | TODO |/| ghost-brief | claude | BLOCKED |/' "$tmp/docs/build-plan/BUILD_STATUS.md"
rm "$tmp/docs/build-plan/BUILD_STATUS.md.bak"
git -C "$tmp" add . >/dev/null
git -C "$tmp" commit -m "status: ghost-brief BLOCKED (premise wrong)" >/dev/null
# Now scrub the row out of the live ledger entirely (as if conductor dropped it).
sed -i.bak '/ghost-brief/d' "$tmp/docs/build-plan/BUILD_STATUS.md"
rm "$tmp/docs/build-plan/BUILD_STATUS.md.bak"
git -C "$tmp" add . >/dev/null
git -C "$tmp" commit -m "status: prune ghost-brief row" >/dev/null

rc=0
ARGO_BLOCK_HISTORY_REPO="$tmp" "$SCRIPT" ghost-brief >/tmp/argo-bh-test2.out 2>&1 || rc=$?
assert_eq "$rc" "1" "prior-block: rc=1"
assert_grep /tmp/argo-bh-test2.out "prior BLOCK history" "prior-block: surfaces 'prior BLOCK history'"
assert_grep /tmp/argo-bh-test2.out "Blocking commits" "prior-block: prints commit list"
rm -rf "$tmp"

# ── Test 3: currently non-TODO row on ledger → exit 2 ─────────────────
echo "[3/5] currently CLAIMED row blocks re-author"
tmp="$(mktemp -d)"
setup_repo "$tmp"
cat >> "$tmp/docs/build-plan/BUILD_STATUS.md" <<'EOF'
| live-brief | claude | CLAIMED | — | actively in flight |
EOF
git -C "$tmp" add . >/dev/null
git -C "$tmp" commit -m "status: claim live-brief" >/dev/null

rc=0
ARGO_BLOCK_HISTORY_REPO="$tmp" "$SCRIPT" live-brief >/tmp/argo-bh-test3.out 2>&1 || rc=$?
assert_eq "$rc" "2" "current-row: rc=2"
assert_grep /tmp/argo-bh-test3.out "CLAIMED" "current-row: surfaces current status"
rm -rf "$tmp"

# ── Test 4: TODO row on ledger is fine (no BLOCK history) → exit 0 ────
echo "[4/5] current TODO row does not block re-author"
tmp="$(mktemp -d)"
setup_repo "$tmp"
cat >> "$tmp/docs/build-plan/BUILD_STATUS.md" <<'EOF'
| todo-brief | claude | TODO | — | freshly authored |
EOF
git -C "$tmp" add . >/dev/null
git -C "$tmp" commit -m "briefs: author todo-brief" >/dev/null

rc=0
ARGO_BLOCK_HISTORY_REPO="$tmp" "$SCRIPT" todo-brief >/tmp/argo-bh-test4.out 2>&1 || rc=$?
assert_eq "$rc" "0" "current-TODO: rc=0"
rm -rf "$tmp"

# ── Test 5: real-world acceptance — known prior-BLOCKED brief is caught ─
# The brief acceptance criteria call for this assertion against the live repo.
# Either exit 1 (prior BLOCK in history) or exit 2 (still on ledger as
# BLOCKED) counts as "caught" — both refuse to author.
echo "[5/5] real-world: claude-mcp-connector-empty-dir-cleanup is caught"
rc=0
"$SCRIPT" claude-mcp-connector-empty-dir-cleanup >/tmp/argo-bh-test5.out 2>&1 || rc=$?
if [[ "$rc" == "1" || "$rc" == "2" ]]; then
  echo "✓ real-world: rc=$rc (refused — script catches the known stale-BLOCKED brief)"
  PASSED=$((PASSED+1))
else
  echo "✗ real-world: rc=$rc (expected 1 or 2 — script failed to catch known-blocked brief)" >&2
  FAILED=$((FAILED+1))
fi

# ── Summary ───────────────────────────────────────────────────────────
echo ""
echo "=== Results: $PASSED passed, $FAILED failed ==="
[[ $FAILED -eq 0 ]]
