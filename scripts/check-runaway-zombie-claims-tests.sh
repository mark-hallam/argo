#!/usr/bin/env bash
# check-runaway-zombie-claims-tests.sh — hermetic smoke tests for
# scripts/check-runaway-zombie-claims.sh.
#
# Each case creates a throwaway bare repo as `origin`, clones a worker
# checkout, synthesises a controlled `status: release <brief-id> (zombie;
# no PR opened)` commit history using GIT_AUTHOR_DATE/GIT_COMMITTER_DATE,
# pushes to origin, then runs the check script with ARGO_RUNAWAY_CHECK_REPO
# pointed at the worker checkout. The 6 cases (A-F) match the brief's
# acceptance criteria.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$SCRIPT_DIR/check-runaway-zombie-claims.sh"
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
    cat "$file" >&2
    FAILED=$((FAILED+1))
  fi
}

assert_empty() {
  local file="$1" name="$2"
  if [[ ! -s "$file" ]]; then
    echo "✓ $name"
    PASSED=$((PASSED+1))
  else
    echo "✗ $name: expected empty, got:" >&2
    cat "$file" >&2
    FAILED=$((FAILED+1))
  fi
}

setup_repo() {
  local tmp="$1"
  mkdir -p "$tmp/remote"
  git -C "$tmp/remote" init --bare --initial-branch=main >/dev/null 2>&1
  git clone "$tmp/remote" "$tmp/seed" >/dev/null 2>&1
  git -C "$tmp/seed" config user.name "seed"
  git -C "$tmp/seed" config user.email "seed@test.invalid"
  git -C "$tmp/seed" config commit.gpgsign false
  echo "init" > "$tmp/seed/README.md"
  git -C "$tmp/seed" add . >/dev/null
  git -C "$tmp/seed" commit -m "init" >/dev/null
  git -C "$tmp/seed" push origin main >/dev/null 2>&1
}

clone_worker() {
  local tmp="$1"
  git clone "$tmp/remote" "$tmp/worker" >/dev/null 2>&1
  git -C "$tmp/worker" config user.name "worker"
  git -C "$tmp/worker" config user.email "worker@test.invalid"
  git -C "$tmp/worker" config commit.gpgsign false
}

# zombie_commit <repo> <brief-id> <minutes-ago>
# Adds and pushes an empty commit with the canonical zombie message,
# back-dated by <minutes-ago> minutes.
zombie_commit() {
  local repo="$1" brief="$2" mins="$3"
  local stamp
  stamp="$(date -u -v "-${mins}M" +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null \
        || date -u --date="${mins} minutes ago" +"%Y-%m-%dT%H:%M:%SZ")"
  GIT_AUTHOR_DATE="$stamp" GIT_COMMITTER_DATE="$stamp" \
    git -C "$repo" commit --allow-empty \
      -m "status: release ${brief} (zombie; no PR opened)" >/dev/null
}

run_check() {
  local repo="$1"; shift
  git -C "$repo" push origin main >/dev/null 2>&1
  ARGO_RUNAWAY_CHECK_REPO="$repo" "$SCRIPT" "$@"
}

# ── Case A — clean: 0 zombie commits ─────────────────────────────────
echo "[A/F] clean (0 zombies)"
tmp="$(mktemp -d)"
setup_repo "$tmp"
clone_worker "$tmp"
rc=0
run_check "$tmp/worker" >/tmp/runaway-A.out 2>/tmp/runaway-A.err || rc=$?
assert_eq "$rc" "0" "A: rc=0 on clean"
assert_empty /tmp/runaway-A.err "A: no stderr"
rm -rf "$tmp"

# ── Case B — below threshold: 2 zombies same brief ───────────────────
echo "[B/F] below threshold (2 zombies)"
tmp="$(mktemp -d)"
setup_repo "$tmp"
clone_worker "$tmp"
zombie_commit "$tmp/worker" brief-x 5
zombie_commit "$tmp/worker" brief-x 10
rc=0
run_check "$tmp/worker" >/tmp/runaway-B.out 2>/tmp/runaway-B.err || rc=$?
assert_eq "$rc" "0" "B: rc=0 when count < threshold"
assert_empty /tmp/runaway-B.err "B: no stderr"
rm -rf "$tmp"

# ── Case C — throttle: 4 zombies same brief ──────────────────────────
echo "[C/F] throttle (4 zombies)"
tmp="$(mktemp -d)"
setup_repo "$tmp"
clone_worker "$tmp"
zombie_commit "$tmp/worker" brief-y 5
zombie_commit "$tmp/worker" brief-y 10
zombie_commit "$tmp/worker" brief-y 15
zombie_commit "$tmp/worker" brief-y 20
rc=0
run_check "$tmp/worker" >/tmp/runaway-C.out 2>/tmp/runaway-C.err || rc=$?
assert_eq "$rc" "2" "C: rc=2 (throttle)"
assert_grep /tmp/runaway-C.err "brief=brief-y reason=runaway-zombie-claims count=4" "C: stderr has count=4"
assert_grep /tmp/runaway-C.err "action=skip" "C: stderr action=skip"
assert_grep /tmp/runaway-C.out "brief=brief-y count=4 window=30min action=skip" "C: stdout machine-readable line"
rm -rf "$tmp"

# ── Case D — block: 7 zombies same brief ─────────────────────────────
echo "[D/F] block (7 zombies)"
tmp="$(mktemp -d)"
setup_repo "$tmp"
clone_worker "$tmp"
for m in 2 4 6 8 10 12 14; do
  zombie_commit "$tmp/worker" brief-z "$m"
done
rc=0
run_check "$tmp/worker" >/tmp/runaway-D.out 2>/tmp/runaway-D.err || rc=$?
assert_eq "$rc" "3" "D: rc=3 (block)"
assert_grep /tmp/runaway-D.err "brief=brief-z" "D: stderr names brief"
assert_grep /tmp/runaway-D.err "count=7" "D: stderr count=7"
assert_grep /tmp/runaway-D.err "action=block" "D: stderr action=block"
rm -rf "$tmp"

# ── Case E — outside window: 8 zombies all >30min ago ────────────────
echo "[E/F] outside window (all >30min)"
tmp="$(mktemp -d)"
setup_repo "$tmp"
clone_worker "$tmp"
for m in 40 50 60 70 80 90 100 110; do
  zombie_commit "$tmp/worker" brief-old "$m"
done
rc=0
run_check "$tmp/worker" >/tmp/runaway-E.out 2>/tmp/runaway-E.err || rc=$?
assert_eq "$rc" "0" "E: rc=0 when zombies outside window"
assert_empty /tmp/runaway-E.err "E: no stderr"
rm -rf "$tmp"

# ── Case F — interleaved: 5 zombies X, 2 zombies Y → reports X ──────
echo "[F/F] interleaved (5 X + 2 Y → reports X)"
tmp="$(mktemp -d)"
setup_repo "$tmp"
clone_worker "$tmp"
zombie_commit "$tmp/worker" brief-X 2
zombie_commit "$tmp/worker" brief-Y 4
zombie_commit "$tmp/worker" brief-X 6
zombie_commit "$tmp/worker" brief-X 8
zombie_commit "$tmp/worker" brief-Y 10
zombie_commit "$tmp/worker" brief-X 12
zombie_commit "$tmp/worker" brief-X 14
rc=0
run_check "$tmp/worker" >/tmp/runaway-F.out 2>/tmp/runaway-F.err || rc=$?
assert_eq "$rc" "2" "F: rc=2 (throttle on max-only)"
assert_grep /tmp/runaway-F.err "brief=brief-X" "F: stderr names max-only brief"
assert_grep /tmp/runaway-F.err "count=5" "F: stderr count=5"
rm -rf "$tmp"

# ── Summary ──────────────────────────────────────────────────────────
echo ""
echo "=== Results: $PASSED passed, $FAILED failed ==="
[[ $FAILED -eq 0 ]]
