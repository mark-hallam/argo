#!/usr/bin/env bash
# test-worker-loop-protects-volatile-state.sh — hermetic smoke harness for
# the wipe-race defense added to scripts/claim-with-retry.sh by
# ops-prevent-concurrent-worker-git-reset-wipe-race.
#
# Background: claim-with-retry.sh's race-loss recovery path runs
# `git reset --hard "$REMOTE/$BRANCH"`. That reset silently wipes any
# uncommitted edits a concurrent process (conductor heartbeat writer,
# evaluator report writer, auditor) made to volatile-state files during
# the claim window. Round-37 (commit 9b07a2ce6, 2026-05-24) lost
# docs/skycot/audit/scheduled/LATEST.md + the evaluator heartbeat + the
# round's report dir to exactly this race. Round-38 added evaluator
# scheduled reports under docs/skycot/evals/scheduled to the same guard.
#
# This harness sources claim-with-retry.sh (which exposes the helpers
# above its main() guard) and exercises stash_volatile_state +
# restore_volatile_state directly against a temp git repo. We test:
#   1. Edits to each of the 6 protected paths survive a reset --hard
#      (4 original from round-37/38 + 2 added in conductor-tick T08-05:
#      docs/build-plan/BUILD_STATUS.md + docs/build-plan/streams/).
#   2. An edit to a NON-protected file is correctly NOT preserved
#      (negative control — proves the scope is the protected globs, not "stash
#      everything").
#   3. Conflict path: when the reset moves HEAD to a newer main that
#      also touched the same file, the local edit is dropped AND a
#      "⚠ wipe-race observed" line is emitted to stderr.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_SCRIPTS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CLAIM_SCRIPT="$REPO_SCRIPTS_DIR/claim-with-retry.sh"

if [[ ! -f "$CLAIM_SCRIPT" ]]; then
    echo "FAIL: claim-with-retry.sh not found at $CLAIM_SCRIPT" >&2
    exit 2
fi

# shellcheck source=../claim-with-retry.sh
# shellcheck disable=SC1091  # path is resolvable at runtime; CI shellcheck pass can't see it
source "$CLAIM_SCRIPT"

PASSED=0
FAILED=0

pass() { echo "✓ $1"; PASSED=$((PASSED+1)); }
fail() { echo "✗ $1" >&2; FAILED=$((FAILED+1)); }

# assert_rc <actual> <expected> <name>
assert_rc() {
    if [[ "$1" -eq "$2" ]]; then pass "$3"; else fail "$3 (rc=$1, expected $2)"; fi
}

# assert_grep <file> <pattern> <pass-msg> <fail-msg>
assert_grep() {
    if grep -q "$2" "$1" 2>/dev/null; then pass "$3"; else fail "$4"; fi
}

# assert_no_grep <file> <pattern> <pass-msg> <fail-msg>
assert_no_grep() {
    if grep -q "$2" "$1" 2>/dev/null; then fail "$4"; else pass "$3"; fi
}

# setup_repo <tmp> — builds a bare remote + a local clone seeded with the
# protected files committed to main. Echoes the local clone path.
setup_repo() {
    local tmp="$1"
    mkdir -p "$tmp/remote"
    git -C "$tmp/remote" init --bare --initial-branch=main >/dev/null 2>&1

    git clone "$tmp/remote" "$tmp/seed" >/dev/null 2>&1
    git -C "$tmp/seed" config user.name "seed"
    git -C "$tmp/seed" config user.email "seed@test.invalid"
    git -C "$tmp/seed" config commit.gpgsign false

    mkdir -p "$tmp/seed/docs/skycot/agents/heartbeats" \
             "$tmp/seed/docs/skycot/audit/scheduled" \
             "$tmp/seed/docs/skycot/evals/scheduled/2026-05-23"
    echo '{"agent":"evaluator","last_seen":"2026-05-24T00:00:00Z"}' \
        > "$tmp/seed/docs/skycot/agents/heartbeats/evaluator.json"
    echo '# initial latest' \
        > "$tmp/seed/docs/skycot/audit/scheduled/LATEST.md"
    echo '# initial eval latest' \
        > "$tmp/seed/docs/skycot/evals/scheduled/2026-05-23/LATEST.md"
    echo '# shared scratchpad' \
        > "$tmp/seed/docs/skycot/agents/SHARED.md"
    echo 'unrelated file' \
        > "$tmp/seed/docs/skycot/agents/UNRELATED.md"
    git -C "$tmp/seed" add . >/dev/null
    git -C "$tmp/seed" commit -m "init volatile state" >/dev/null
    git -C "$tmp/seed" push origin main >/dev/null 2>&1

    git clone "$tmp/remote" "$tmp/work" >/dev/null 2>&1
    git -C "$tmp/work" config user.name "work"
    git -C "$tmp/work" config user.email "work@test.invalid"
    git -C "$tmp/work" config commit.gpgsign false

    echo "$tmp/work"
}

# ── Test 1: all 6 protected paths survive a stash → reset --hard → restore ──
echo "[1/4] all 6 protected paths survive reset --hard"
tmp1="$(mktemp -d)"
work1="$(setup_repo "$tmp1")"

# Simulate concurrent-process uncommitted edits on each protected path.
echo '{"agent":"evaluator","last_seen":"2026-05-24T08:30:00Z","note":"CONCURRENT_EDIT"}' \
    > "$work1/docs/skycot/agents/heartbeats/evaluator.json"
echo '# latest with CONCURRENT_EDIT_LATEST' \
    > "$work1/docs/skycot/audit/scheduled/LATEST.md"
mkdir -p "$work1/docs/skycot/evals/scheduled/2026-05-24/round-46"
echo '# eval report CONCURRENT_EDIT_EVAL' \
    > "$work1/docs/skycot/evals/scheduled/2026-05-24/round-46/findings.md"
echo '# shared CONCURRENT_EDIT_SHARED' \
    > "$work1/docs/skycot/agents/SHARED.md"
# Conductor-tick T08-05 (2026-05-25) additions: BUILD_STATUS.md ledger flip
# (tracked-file modification) + new brief file in streams/ (untracked).
mkdir -p "$work1/docs/build-plan"
echo '# Build status — CONCURRENT_EDIT_BUILD_STATUS' \
    > "$work1/docs/build-plan/BUILD_STATUS.md"
mkdir -p "$work1/docs/build-plan/streams"
echo '# new brief CONCURRENT_EDIT_NEW_BRIEF' \
    > "$work1/docs/build-plan/streams/fix-emergency-block.md"

# Stash → reset → restore.
stash_volatile_state "$work1"
assert_rc $? 0 "stash_volatile_state returned 0 (created stash)"

git -C "$work1" reset --hard origin/main >/dev/null 2>&1
restore_volatile_state "$work1" 2>/tmp/restore-test1.err

assert_grep "$work1/docs/skycot/agents/heartbeats/evaluator.json" "CONCURRENT_EDIT" \
    "heartbeats/evaluator.json edit survived reset" \
    "heartbeats/evaluator.json was WIPED by reset (regression)"

assert_grep "$work1/docs/skycot/audit/scheduled/LATEST.md" "CONCURRENT_EDIT_LATEST" \
    "audit/scheduled/LATEST.md edit survived reset" \
    "audit/scheduled/LATEST.md was WIPED by reset (regression)"

assert_grep "$work1/docs/skycot/evals/scheduled/2026-05-24/round-46/findings.md" "CONCURRENT_EDIT_EVAL" \
    "evals/scheduled round report survived reset" \
    "evals/scheduled round report was WIPED by reset (regression)"

assert_grep "$work1/docs/skycot/agents/SHARED.md" "CONCURRENT_EDIT_SHARED" \
    "agents/SHARED.md edit survived reset" \
    "agents/SHARED.md was WIPED by reset (regression)"

assert_grep "$work1/docs/build-plan/BUILD_STATUS.md" "CONCURRENT_EDIT_BUILD_STATUS" \
    "BUILD_STATUS.md edit survived reset" \
    "BUILD_STATUS.md was WIPED by reset (regression — T08-05 conductor wipe scenario)"

assert_grep "$work1/docs/build-plan/streams/fix-emergency-block.md" "CONCURRENT_EDIT_NEW_BRIEF" \
    "new brief file in streams/ survived reset" \
    "new brief file in streams/ was WIPED by reset (regression — T08-05 conductor wipe scenario)"

rm -rf "$tmp1"

# ── Test 2: no-op when working tree clean ────────────────────────────────
echo "[2/4] stash_volatile_state returns 1 when nothing dirty"
tmp2="$(mktemp -d)"
work2="$(setup_repo "$tmp2")"
stash_volatile_state "$work2"
assert_rc $? 1 "clean tree → return 1 (skip restore)"
rm -rf "$tmp2"

# ── Test 3: NON-protected paths get reset (negative control) ─────────────
echo "[3/4] negative control: non-protected file NOT preserved"
tmp3="$(mktemp -d)"
work3="$(setup_repo "$tmp3")"
echo 'CONCURRENT_NON_PROTECTED_EDIT' \
    > "$work3/docs/skycot/agents/UNRELATED.md"
stash_volatile_state "$work3"
assert_rc $? 1 "non-protected dirt → return 1 (correctly out of scope)"
# Confirm a raw reset wipes that edit (proves we wouldn't have saved it).
git -C "$work3" reset --hard origin/main >/dev/null 2>&1
assert_no_grep "$work3/docs/skycot/agents/UNRELATED.md" "CONCURRENT_NON_PROTECTED_EDIT" \
    "non-protected file correctly wiped by reset (scope is protected globs)" \
    "non-protected file unexpectedly preserved — scope too wide?"
rm -rf "$tmp3"

# ── Test 4: stash-pop conflict → prefer upstream + emit warning ──────────
echo "[4/4] conflict path: upstream wins, '⚠ wipe-race' logged"
tmp4="$(mktemp -d)"
work4="$(setup_repo "$tmp4")"

# Local: uncommitted volatile-state edit.
echo '# LOCAL_VARIANT' > "$work4/docs/skycot/audit/scheduled/LATEST.md"

stash_volatile_state "$work4"
assert_rc $? 0 "conflict-setup: stash returned 0"

# Concurrent writer pushes a DIFFERENT new version of the same file to main.
git clone "$tmp4/remote" "$tmp4/other" >/dev/null 2>&1
git -C "$tmp4/other" config user.name "other"
git -C "$tmp4/other" config user.email "other@test.invalid"
git -C "$tmp4/other" config commit.gpgsign false
echo '# UPSTREAM_VARIANT' > "$tmp4/other/docs/skycot/audit/scheduled/LATEST.md"
git -C "$tmp4/other" add docs/skycot/audit/scheduled/LATEST.md
git -C "$tmp4/other" commit -m "upstream writer" >/dev/null
git -C "$tmp4/other" push origin main >/dev/null 2>&1

# Worker fetches + resets — exactly what claim-with-retry does after a
# rebase --abort.
git -C "$work4" fetch origin main >/dev/null 2>&1
git -C "$work4" reset --hard origin/main >/dev/null 2>&1

restore_volatile_state "$work4" 2>/tmp/restore-test4.err

if grep -q "UPSTREAM_VARIANT" "$work4/docs/skycot/audit/scheduled/LATEST.md" \
   && ! grep -q "LOCAL_VARIANT" "$work4/docs/skycot/audit/scheduled/LATEST.md"; then
    pass "conflict resolved to upstream (LOCAL_VARIANT dropped)"
else
    fail "conflict: expected UPSTREAM_VARIANT to win, got:"
    cat "$work4/docs/skycot/audit/scheduled/LATEST.md" >&2
fi

if grep -q "⚠ wipe-race observed" /tmp/restore-test4.err; then
    pass "wipe-race warning emitted on conflict"
else
    fail "expected '⚠ wipe-race observed' on stderr; got:"
    cat /tmp/restore-test4.err >&2
fi

# Working tree should be clean after conflict resolution (no leftover
# stash entries, no conflict markers).
if [[ -z "$(git -C "$work4" status --porcelain)" ]] \
   && [[ -z "$(git -C "$work4" stash list)" ]]; then
    pass "post-conflict tree clean + stash drained"
else
    fail "post-conflict residue:"
    git -C "$work4" status --short >&2
    git -C "$work4" stash list >&2
fi

rm -rf "$tmp4"

echo ""
echo "=== Results: $PASSED passed, $FAILED failed ==="
[[ $FAILED -eq 0 ]]
