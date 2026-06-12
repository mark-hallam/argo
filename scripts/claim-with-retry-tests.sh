#!/usr/bin/env bash
# claim-with-retry-tests.sh — bash smoke tests for claim-with-retry.sh
# (Stream `claude-claim-with-retry`).
#
# Sets up a temp bare repo as `origin`, clones a worker checkout, and
# exercises the script's branches: happy path, not-found, BLOCKED,
# already-CLAIMED, dirty tree, operator-asset volatile dirt, and a
# deterministic non-fast-forward retry that recovers via pull --rebase.

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
    echo "tracked fixture" > "$tmp/seed/TRACKED.txt"
    cat > "$tmp/seed/docs/build-plan/BUILD_STATUS.md" <<EOF
# Build status

| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| test-todo-row | claude | TODO | — | initial state |
| test-other-todo | claude | TODO | — | unrelated row |
| test-empty-col4-row | claude or codex | TODO | | empty col-4 (no PR yet) |
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
echo "[1/9] happy path"
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
echo "[2/9] brief not found"
tmp="$(mktemp -d)"
setup_remote "$tmp"
clone_worker "$tmp" "w1"
rc=0
run_script "$tmp/w1" nonexistent-brief >/tmp/argo-test2.out 2>&1 || rc=$?
assert_eq "$rc" "2" "not-found: rc=2"
rm -rf "$tmp"

# ── Test 3: BLOCKED row ───────────────────────────────────────────
echo "[3/9] BLOCKED row"
tmp="$(mktemp -d)"
setup_remote "$tmp"
clone_worker "$tmp" "w1"
rc=0
run_script "$tmp/w1" test-blocked-row >/tmp/argo-test3.out 2>&1 || rc=$?
assert_eq "$rc" "1" "blocked: rc=1"
assert_grep /tmp/argo-test3.out "BLOCKED" "blocked: surfaces BLOCKED"
rm -rf "$tmp"

# ── Test 4: CLAIMED by other ──────────────────────────────────────
echo "[4/9] CLAIMED by other"
tmp="$(mktemp -d)"
setup_remote "$tmp"
clone_worker "$tmp" "w1"
rc=0
run_script "$tmp/w1" test-claimed-row >/tmp/argo-test4.out 2>&1 || rc=$?
assert_eq "$rc" "1" "claimed-by-other: rc=1"
assert_grep /tmp/argo-test4.out "falling through" "claimed-by-other: HARD RULE 7 hint"
rm -rf "$tmp"

# ── Test 5: dirty tree refusal ────────────────────────────────────
echo "[5/9] dirty tree refusal"
tmp="$(mktemp -d)"
setup_remote "$tmp"
clone_worker "$tmp" "w1"
echo "uncommitted edit" >> "$tmp/w1/TRACKED.txt"
rc=0
run_script "$tmp/w1" test-todo-row >/tmp/argo-test5.out 2>&1 || rc=$?
assert_eq "$rc" "2" "dirty-tree: rc=2"
assert_grep /tmp/argo-test5.out "dirty" "dirty-tree: surfaces 'dirty'"
rm -rf "$tmp"

# ── Test 6: operator marketing dirt is stashed/restored ───────────
echo "[6/9] operator marketing dirt survives"
tmp="$(mktemp -d)"
setup_remote "$tmp"
clone_worker "$tmp" "w1"
mkdir -p "$tmp/w1/docs/skycot/marketing"
echo "mockup" > "$tmp/w1/docs/skycot/marketing/foo.md"
rc=0
run_script "$tmp/w1" test-todo-row >/tmp/argo-test6.out 2>&1 || rc=$?
assert_eq "$rc" "0" "marketing-dirt: rc=0"
[[ -f "$tmp/w1/docs/skycot/marketing/foo.md" ]] && { echo "✓ marketing-dirt: operator file restored"; PASSED=$((PASSED+1)); } || { echo "✗ marketing-dirt: operator file missing"; FAILED=$((FAILED+1)); }
rm -rf "$tmp"

# ── Test 7: web brand dirt is stashed/restored ────────────────────
echo "[7/9] web brand dirt survives"
tmp="$(mktemp -d)"
setup_remote "$tmp"
clone_worker "$tmp" "w1"
mkdir -p "$tmp/w1/web/public/brand/social"
echo "png" > "$tmp/w1/web/public/brand/social/x.png"
rc=0
run_script "$tmp/w1" test-todo-row >/tmp/argo-test7.out 2>&1 || rc=$?
assert_eq "$rc" "0" "brand-dirt: rc=0"
[[ -f "$tmp/w1/web/public/brand/social/x.png" ]] && { echo "✓ brand-dirt: operator file restored"; PASSED=$((PASSED+1)); } || { echo "✗ brand-dirt: operator file missing"; FAILED=$((FAILED+1)); }
rm -rf "$tmp"

# ── Test 8: operator demo dirt is stashed/restored ────────────────
echo "[8/9] operator demo dirt survives"
tmp="$(mktemp -d)"
setup_remote "$tmp"
clone_worker "$tmp" "w1"
mkdir -p "$tmp/w1/docs/skycot/demos"
echo "audit" > "$tmp/w1/docs/skycot/demos/pre-record-ux-audit-2026-05-25.md"
rc=0
run_script "$tmp/w1" test-todo-row >/tmp/argo-test8.out 2>&1 || rc=$?
assert_eq "$rc" "0" "demo-dirt: rc=0"
[[ -f "$tmp/w1/docs/skycot/demos/pre-record-ux-audit-2026-05-25.md" ]] && { echo "✓ demo-dirt: operator file restored"; PASSED=$((PASSED+1)); } || { echo "✗ demo-dirt: operator file missing"; FAILED=$((FAILED+1)); }
rm -rf "$tmp"

# ── Test 9: non-fast-forward race recovers via pull --rebase ──────
# Setup: w1 starts to claim test-todo-row. Before w1 pushes, w2 pushes
# an unrelated commit. w1's first push fails non-fast-forward; w1's
# retry rebases w2's commit and pushes successfully.
echo "[9/9] non-fast-forward race recovers"
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
run_script "$tmp/w1" test-todo-row >/tmp/argo-test9.out 2>&1 || rc=$?
assert_eq "$rc" "0" "race-recover: rc=0"
git -C "$tmp/w1" pull --ff-only origin main >/dev/null 2>&1
assert_grep "$tmp/w1/docs/build-plan/BUILD_STATUS.md" "test-todo-row.*CLAIMED" "race-recover: row CLAIMED after retry"
[[ -f "$tmp/w1/UNRELATED.md" ]] && { echo "✓ race-recover: w2's unrelated commit absorbed"; PASSED=$((PASSED+1)); } || { echo "✗ race-recover: w2 commit missing"; FAILED=$((FAILED+1)); }
rm -rf "$tmp"

# ── Test 10: empty col-4 TODO row is claimable ────────────────────
# Regression for the swarm-starvation bug (root-caused 2026-06-12): a TODO
# row with an EMPTY col-4 (`| TODO | |`) is a legal "no PR yet" state and
# MUST claim cleanly. Before the tolerant-regex fix the `[^|]+` col-4 match
# rejected it (rc=2), fast-failing every worker.
echo "[10/11] empty col-4 TODO is claimable"
tmp="$(mktemp -d)"
setup_remote "$tmp"
clone_worker "$tmp" "w1"
rc=0
run_script "$tmp/w1" test-empty-col4-row >/tmp/argo-test10.out 2>&1 || rc=$?
assert_eq "$rc" "0" "empty-col4: rc=0"
git -C "$tmp/w1" pull --ff-only origin main >/dev/null 2>&1
assert_grep "$tmp/w1/docs/build-plan/BUILD_STATUS.md" "test-empty-col4-row.*CLAIMED" "empty-col4: row marked CLAIMED"
# col-4 must be rewritten from empty to the em-dash placeholder on claim.
assert_grep "$tmp/w1/docs/build-plan/BUILD_STATUS.md" "test-empty-col4-row | claude or codex | CLAIMED | — |" "empty-col4: col-4 rewritten to —"
rm -rf "$tmp"

# ── Test 11: claim-regex reject on an existing TODO row alerts ────
# The alert_claim_regex_reject helper must emit a LOUD stderr marker AND
# append a [claim-regex-reject] line to SHARED.md so a future un-anticipated
# ledger-format reject surfaces in the next conductor tick, not as silent
# rc=2 + backoff noise. Source the script (like the volatile-state helpers)
# and exercise the helper directly against a throwaway SHARED.md.
echo "[11/11] claim-regex-reject emits SHARED.md alert"
tmp="$(mktemp -d)"
mkdir -p "$tmp/repo/docs/skycot/agents"
: > "$tmp/repo/docs/skycot/agents/SHARED.md"
# shellcheck source=/dev/null
source "$SCRIPT"
export SHARED_MD_PATH="$tmp/repo/docs/skycot/agents/SHARED.md"
alert_claim_regex_reject "$tmp/repo" "test-malformed-brief" 2>/tmp/argo-test11.err
unset SHARED_MD_PATH
assert_grep /tmp/argo-test11.err "LEDGER-FORMAT BUG" "alert: loud stderr marker"
assert_grep "$tmp/repo/docs/skycot/agents/SHARED.md" "\[claim-regex-reject\]" "alert: SHARED.md line appended"
assert_grep "$tmp/repo/docs/skycot/agents/SHARED.md" "test-malformed-brief" "alert: SHARED.md names the brief"
rm -rf "$tmp"

# ── Summary ───────────────────────────────────────────────────────
echo ""
echo "=== Results: $PASSED passed, $FAILED failed ==="
[[ $FAILED -eq 0 ]]
