#!/usr/bin/env bash
# worker-midrun-death-detect-tests.sh — bash assertions for the one-shot
# mid-run-death backstop (scripts/worker-midrun-death-detect.sh,
# brief fix-worker-oneshot-background-wait-midrun-death).
#
# Sets up a temp bare repo as `origin`, clones a worker checkout, sources the
# detector, and exercises its three outcomes:
#   (a) CLAIMED row + branch with claim-commit-only (no work) -> auto-released
#       to TODO + a [worker-midrun-death] SHARED.md line.
#   (b) CLAIMED row + branch with a real work commit -> branch PUSHED to origin
#       + SHARED.md line, row left CLAIMED (preserve the work, conductor salvages).
#   (c) IN_REVIEW row -> untouched (no false positive on the happy path).
#
# No `gh` and no network are needed: the detector's work-commit count (not a PR
# query) is the discriminator, and pushes target the local bare origin.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DETECTOR="$SCRIPT_DIR/worker-midrun-death-detect.sh"
PASSED=0
FAILED=0

assert_grep() {
    local file="$1" pattern="$2" name="$3"
    if grep -qE -- "$pattern" "$file" 2>/dev/null; then
        echo "✓ $name"; PASSED=$((PASSED+1))
    else
        echo "✗ $name: pattern '$pattern' not found in $file" >&2; FAILED=$((FAILED+1))
    fi
}

assert_not_grep() {
    local file="$1" pattern="$2" name="$3"
    if ! grep -qE -- "$pattern" "$file" 2>/dev/null; then
        echo "✓ $name"; PASSED=$((PASSED+1))
    else
        echo "✗ $name: pattern '$pattern' UNEXPECTEDLY present in $file" >&2; FAILED=$((FAILED+1))
    fi
}

assert_remote_has_branch() {
    local repo="$1" branch="$2" name="$3"
    if [ -n "$(git -C "$repo" ls-remote --heads origin "$branch" 2>/dev/null)" ]; then
        echo "✓ $name"; PASSED=$((PASSED+1))
    else
        echo "✗ $name: branch '$branch' not on origin" >&2; FAILED=$((FAILED+1))
    fi
}

# Build a bare origin + a worker checkout with a BUILD_STATUS.md + SHARED.md.
setup_repo() {
    local tmp="$1"
    mkdir -p "$tmp/remote"
    git -C "$tmp/remote" init --bare --initial-branch=main >/dev/null 2>&1

    git clone "$tmp/remote" "$tmp/seed" >/dev/null 2>&1
    git -C "$tmp/seed" config user.name seed
    git -C "$tmp/seed" config user.email seed@test.invalid
    git -C "$tmp/seed" config commit.gpgsign false
    mkdir -p "$tmp/seed/docs/build-plan" "$tmp/seed/docs/skycot/agents"
    cat > "$tmp/seed/docs/build-plan/BUILD_STATUS.md" <<'EOF'
# Build status

| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| test-midrun-nowork | claude or codex | CLAIMED | — | claimed; no work done |
| test-midrun-work | claude or codex | CLAIMED | — | claimed; work in flight |
| test-midrun-inreview | claude or codex | IN_REVIEW | #999 | shipped happy path |
EOF
    printf '# SHARED handoffs\n' > "$tmp/seed/docs/skycot/agents/SHARED.md"
    git -C "$tmp/seed" add . >/dev/null
    git -C "$tmp/seed" commit -m init >/dev/null
    git -C "$tmp/seed" push origin main >/dev/null 2>&1

    git clone "$tmp/remote" "$tmp/w1" >/dev/null 2>&1
    git -C "$tmp/w1" config user.name w1
    git -C "$tmp/w1" config user.email w1@test.invalid
    git -C "$tmp/w1" config commit.gpgsign false
}

# Create a feature branch in the worker checkout, optionally with a work commit,
# then return the checkout to main (mirrors a dead worker's left-behind ref).
make_branch() {
    local repo="$1" branch="$2" with_work="$3"
    git -C "$repo" checkout -b "$branch" >/dev/null 2>&1
    if [ "$with_work" = "work" ]; then
        echo "real work" > "$repo/NEWFILE.txt"
        git -C "$repo" add NEWFILE.txt >/dev/null
        git -C "$repo" commit -m "feat: real work commit" >/dev/null
    fi
    git -C "$repo" checkout main >/dev/null 2>&1
}

# shellcheck source=/dev/null
source "$DETECTOR"

# ── Test (a): no-work branch -> auto-released to TODO ──────────────
echo "[1/3] no-work branch auto-releases to TODO"
tmp="$(mktemp -d)"
setup_repo "$tmp"
make_branch "$tmp/w1" "claude-test-midrun-nowork" "nowork"
worker_midrun_death_check "test-midrun-nowork" "claude-test-midrun-nowork" "$tmp/w1" >/tmp/wmd-a.out 2>&1
assert_grep "$tmp/w1/docs/build-plan/BUILD_STATUS.md" "test-midrun-nowork \| claude or codex \| TODO \|" "no-work: row flipped to TODO"
assert_grep "$tmp/w1/docs/build-plan/BUILD_STATUS.md" "AUTO-RELEASED \(midrun death\)" "no-work: AUTO-RELEASED note present"
assert_grep "$tmp/w1/docs/skycot/agents/SHARED.md" "\[worker-midrun-death\].*test-midrun-nowork" "no-work: SHARED.md line written"
rm -rf "$tmp"

# ── Test (b): work-commit branch -> pushed, row left CLAIMED ───────
echo "[2/3] work branch is preserved (pushed), row left CLAIMED"
tmp="$(mktemp -d)"
setup_repo "$tmp"
make_branch "$tmp/w1" "claude-test-midrun-work" "work"
worker_midrun_death_check "test-midrun-work" "claude-test-midrun-work" "$tmp/w1" >/tmp/wmd-b.out 2>&1
assert_grep "$tmp/w1/docs/build-plan/BUILD_STATUS.md" "test-midrun-work \| claude or codex \| CLAIMED \|" "work: row left CLAIMED"
assert_remote_has_branch "$tmp/w1" "claude-test-midrun-work" "work: branch pushed to origin"
assert_grep "$tmp/w1/docs/skycot/agents/SHARED.md" "\[worker-midrun-death\].*test-midrun-work.*REAL work" "work: SHARED.md preserve line written"
rm -rf "$tmp"

# ── Test (c): IN_REVIEW row -> untouched ──────────────────────────
echo "[3/3] IN_REVIEW row is untouched (no false positive)"
tmp="$(mktemp -d)"
setup_repo "$tmp"
worker_midrun_death_check "test-midrun-inreview" "claude-test-midrun-inreview" "$tmp/w1" >/tmp/wmd-c.out 2>&1
assert_grep "$tmp/w1/docs/build-plan/BUILD_STATUS.md" "test-midrun-inreview \| claude or codex \| IN_REVIEW \|" "in-review: row unchanged"
assert_not_grep "$tmp/w1/docs/skycot/agents/SHARED.md" "test-midrun-inreview" "in-review: no SHARED.md line"
rm -rf "$tmp"

echo ""
echo "=== Results: $PASSED passed, $FAILED failed ==="
[ "$FAILED" -eq 0 ]
