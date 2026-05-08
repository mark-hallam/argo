#!/usr/bin/env bash
# claim-with-retry-tests.sh — bash smoke tests for claim-with-retry.sh
# (Stream `claude-claim-with-retry`).
#
# Sets up a temp bare repo as `origin`, clones a worker checkout, and
# exercises the script's branches: happy path, not-found, BLOCKED,
# already-CLAIMED, dirty tree, and a deterministic non-fast-forward
# retry that recovers via pull --rebase.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$SCRIPT_DIR/claim-with-retry.sh"
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

setup_remote() {
    local tmp="$1"
    mkdir -p "$tmp/remote"
    git -C "$tmp/remote" init --bare --initial-branch=main >/dev/null 2>&1

    git clone "$tmp/remote" "$tmp/seed" >/dev/null 2>&1
    git -C "$tmp/seed" config user.name "seed"
    git -C "$tmp/seed" config user.email "seed@test.invalid"
    git -C "$tmp/seed" config commit.gpgsign false
    mkdir -p "$tmp/seed/docs/build-plan" "$tmp/seed/scripts"
    cat > "$tmp/seed/docs/build-plan/BUILD_STATUS.md" <<EOF
# Build status

| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| test-todo-row | claude | TODO | — | initial state |
| test-other-todo | claude | TODO | — | unrelated row |
| test-claimed-row | claude | CLAIMED | — | claimed by other tab |
| test-blocked-row | claude | BLOCKED | — | needs unblock |
EOF
    cp "$SCRIPT" "$tmp/seed/scripts/claim-with-retry.sh"
    chmod +x "$tmp/seed/scripts/claim-with-retry.sh"
    git -C "$tmp/seed" add . >/dev/null
    git -C "$tmp/seed" commit -m "init" >/dev/null
    git -C "$tmp/seed" push origin main >/dev/null 2>&1
}

clone_worker() {
    local tmp="$1" name="$2"
    git clone "$tmp/remote" "$tmp/$name" >/dev/null 2>&1
    git -C "$tmp/$name" config user.name "$name"
    git -C "$tmp/$name" config user.email "$name@test.invalid"
    git -C "$tmp/$name" config commit.gpgsign false
}

run_script() {
    local repo="$1" brief="$2"
    ARGO_CLAIM_REPO="$repo" "$repo/scripts/claim-with-retry.sh" "$brief"
}

# ── Test 1: happy path ────────────────────────────────────────────
echo "[1/6] happy path"
tmp="$(mktemp -d)"
setup_remote "$tmp"
clone_worker "$tmp" "w1"
rc=0
run_script "$tmp/w1" test-todo-row >/tmp/argo-test1.out 2>&1 || rc=$?
assert_eq "$rc" "0" "happy: rc=0"
git -C "$tmp/w1" pull --ff-only origin main >/dev/null 2>&1
assert_grep "$tmp/w1/docs/build-plan/BUILD_STATUS.md" "test-todo-row.*CLAIMED" "happy: row marked CLAIMED"
rm -rf "$tmp"

# ── Test 2: brief not found ───────────────────────────────────────
echo "[2/6] brief not found"
tmp="$(mktemp -d)"
setup_remote "$tmp"
clone_worker "$tmp" "w1"
rc=0
run_script "$tmp/w1" nonexistent-brief >/tmp/argo-test2.out 2>&1 || rc=$?
assert_eq "$rc" "2" "not-found: rc=2"
rm -rf "$tmp"

# ── Test 3: BLOCKED row ───────────────────────────────────────────
echo "[3/6] BLOCKED row"
tmp="$(mktemp -d)"
setup_remote "$tmp"
clone_worker "$tmp" "w1"
rc=0
run_script "$tmp/w1" test-blocked-row >/tmp/argo-test3.out 2>&1 || rc=$?
assert_eq "$rc" "1" "blocked: rc=1"
assert_grep /tmp/argo-test3.out "BLOCKED" "blocked: surfaces BLOCKED"
rm -rf "$tmp"

# ── Test 4: CLAIMED by other ──────────────────────────────────────
echo "[4/6] CLAIMED by other"
tmp="$(mktemp -d)"
setup_remote "$tmp"
clone_worker "$tmp" "w1"
rc=0
run_script "$tmp/w1" test-claimed-row >/tmp/argo-test4.out 2>&1 || rc=$?
assert_eq "$rc" "1" "claimed-by-other: rc=1"
assert_grep /tmp/argo-test4.out "falling through" "claimed-by-other: HARD RULE 7 hint"
rm -rf "$tmp"

# ── Test 5: dirty tree refusal ────────────────────────────────────
echo "[5/6] dirty tree refusal"
tmp="$(mktemp -d)"
setup_remote "$tmp"
clone_worker "$tmp" "w1"
echo "uncommitted edit" >> "$tmp/w1/docs/build-plan/BUILD_STATUS.md"
rc=0
run_script "$tmp/w1" test-todo-row >/tmp/argo-test5.out 2>&1 || rc=$?
assert_eq "$rc" "2" "dirty-tree: rc=2"
assert_grep /tmp/argo-test5.out "dirty" "dirty-tree: surfaces 'dirty'"
rm -rf "$tmp"

# ── Test 6: non-fast-forward race recovers via pull --rebase ──────
# Setup: w1 starts to claim test-todo-row. Before w1 pushes, w2 pushes
# an unrelated commit. w1's first push fails non-fast-forward; w1's
# retry rebases w2's commit and pushes successfully.
echo "[6/6] non-fast-forward race recovers"
tmp="$(mktemp -d)"
setup_remote "$tmp"
clone_worker "$tmp" "w1"
clone_worker "$tmp" "w2"
# Install a pre-push hook in w1 that runs ONCE: pushes an unrelated
# w2 commit so the next push step hits non-fast-forward.
mkdir -p "$tmp/w1/.git/hooks"
cat > "$tmp/w1/.git/hooks/pre-push" <<EOH
#!/bin/bash
MARKER="$tmp/race-fired"
if [[ -f "\$MARKER" ]]; then exit 0; fi
touch "\$MARKER"
echo "x" > "$tmp/w2/UNRELATED.md"
git -C "$tmp/w2" add UNRELATED.md
git -C "$tmp/w2" commit -m "unrelated change" >/dev/null
git -C "$tmp/w2" push origin main >/dev/null 2>&1
exit 0
EOH
chmod +x "$tmp/w1/.git/hooks/pre-push"
rc=0
run_script "$tmp/w1" test-todo-row >/tmp/argo-test6.out 2>&1 || rc=$?
assert_eq "$rc" "0" "race-recover: rc=0"
git -C "$tmp/w1" pull --ff-only origin main >/dev/null 2>&1
assert_grep "$tmp/w1/docs/build-plan/BUILD_STATUS.md" "test-todo-row.*CLAIMED" "race-recover: row CLAIMED after retry"
[[ -f "$tmp/w1/UNRELATED.md" ]] && { echo "✓ race-recover: w2's unrelated commit absorbed"; PASSED=$((PASSED+1)); } || { echo "✗ race-recover: w2 commit missing"; FAILED=$((FAILED+1)); }
rm -rf "$tmp"

# ── Summary ───────────────────────────────────────────────────────
echo ""
echo "=== Results: $PASSED passed, $FAILED failed ==="
[[ $FAILED -eq 0 ]]
