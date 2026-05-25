#!/usr/bin/env bash
# Tests for the vertical-regression gate inside scripts/codex-pre-push.sh.
#
# Locks two behaviors:
#
#   1. When the branch (committed) or working tree (staged/unstaged)
#      touches `verticals/**`, the script runs
#      `cargo test -p argo-vertical-regression --test regression_all`
#      AFTER the existing workspace clippy step.
#
#   2. When nothing under `verticals/` changed, the script does NOT
#      run the regression test. The workspace clippy gate still runs.
#
# Strategy: stand up a hermetic git repo with a fake `origin/main`
# remote and a feature branch, shim `cargo` onto PATH so every
# invocation is logged + zero-exit, then exercise the script and
# inspect the log.
#
# Exit codes:
#   0   all assertions passed
#   1   first failed assertion

set -uo pipefail

PASS=0
FAIL=0
FAIL_NAMES=()

assert_eq() {
  local name="$1" want="$2" got="$3"
  if [ "$want" = "$got" ]; then
    echo "  OK: $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $name"
    echo "        want: $want"
    echo "        got:  $got"
    FAIL=$((FAIL + 1))
    FAIL_NAMES+=("$name")
  fi
}

# Resolve the production script — the test runs the real thing.
THIS_DIR="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$THIS_DIR/../codex-pre-push.sh"
if [ ! -x "$SCRIPT" ]; then
  echo "FAIL: cannot find $SCRIPT (or not executable)"
  exit 1
fi

FIX="$(mktemp -d)"
trap 'rm -rf "$FIX"' EXIT

# ──────────────────────────────────────────────── mock cargo
# Every invocation appends one line of its arguments to $CARGO_LOG,
# then exits 0. Production output is suppressed; the script's own
# `tee` captures stdout into its tempfile, which is fine — we are
# locking the invocation pattern, not the output text.
MOCK_BIN="$FIX/bin"
mkdir -p "$MOCK_BIN"
cat > "$MOCK_BIN/cargo" <<'EOF'
#!/usr/bin/env bash
echo "cargo $*" >> "$CARGO_LOG"
exit 0
EOF
chmod +x "$MOCK_BIN/cargo"
export PATH="$MOCK_BIN:$PATH"

# ──────────────────────────────────────────────── git fixture
# Build a repo with `main` (pointing at the merge base) and a
# feature branch on top. Mark `origin/main` as a real ref so the
# script's `git merge-base HEAD origin/main` resolves cleanly.
REPO="$FIX/repo"
mkdir -p "$REPO"
cd "$REPO"

git init -q -b main
git config user.email tester@example.com
git config user.name tester

mkdir -p crates/foo verticals/example
echo "fn main() {}" > crates/foo/lib.rs
echo "{}" > verticals/example/manifest.toml
git add .
git commit -q -m "initial"

# Pin `origin/main` to this commit so merge-base resolves without
# needing an actual remote.
git update-ref refs/remotes/origin/main HEAD

git checkout -q -b feature

# ──────────────────────────────────────────────── case 1: no verticals
# Commit a change outside `verticals/` only — gate must NOT fire.
echo "fn helper() {}" >> crates/foo/lib.rs
git add crates/foo/lib.rs
git commit -q -m "non-vertical change"

CARGO_LOG="$FIX/cargo-clean.log"
export CARGO_LOG
: > "$CARGO_LOG"

echo
echo "Case 1: no vertical changes — regression must NOT run"
"$SCRIPT" >/dev/null 2>&1
rc=$?
assert_eq "clean-path exit 0" "0" "$rc"

clippy_hits=$(grep -c '^cargo clippy ' "$CARGO_LOG" || true)
regression_hits=$(grep -c 'argo-vertical-regression' "$CARGO_LOG" || true)
assert_eq "clean-path: clippy ran once"            "1" "$clippy_hits"
assert_eq "clean-path: regression test NOT run"    "0" "$regression_hits"

# ──────────────────────────────────────────────── case 2: committed vertical
# A committed change under `verticals/` between merge-base and HEAD
# must fire the gate.
echo "edited" > verticals/example/manifest.toml
git add verticals/example/manifest.toml
git commit -q -m "edit vertical manifest"

CARGO_LOG="$FIX/cargo-committed.log"
export CARGO_LOG
: > "$CARGO_LOG"

echo
echo "Case 2: committed vertical change — regression MUST run"
"$SCRIPT" >/dev/null 2>&1
rc=$?
assert_eq "committed-path exit 0" "0" "$rc"

clippy_hits=$(grep -c '^cargo clippy ' "$CARGO_LOG" || true)
regression_hits=$(grep -c -- '-p argo-vertical-regression --test regression_all' "$CARGO_LOG" || true)
assert_eq "committed-path: clippy ran once"        "1" "$clippy_hits"
assert_eq "committed-path: regression ran once"    "1" "$regression_hits"

# ──────────────────────────────────────────────── case 3: staged-only vertical
# Reset back to merge-base and only stage a vertical change. The
# gate must still fire on staged-but-uncommitted edits.
git reset -q --hard "$(git merge-base HEAD origin/main)"
echo "staged edit" > verticals/example/manifest.toml
git add verticals/example/manifest.toml

CARGO_LOG="$FIX/cargo-staged.log"
export CARGO_LOG
: > "$CARGO_LOG"

echo
echo "Case 3: staged-only vertical change — regression MUST run"
"$SCRIPT" >/dev/null 2>&1
rc=$?
assert_eq "staged-path exit 0" "0" "$rc"

regression_hits=$(grep -c -- '-p argo-vertical-regression --test regression_all' "$CARGO_LOG" || true)
assert_eq "staged-path: regression ran once"       "1" "$regression_hits"

# ──────────────────────────────────────────────── case 4: unstaged-only vertical
# Reset everything (including the index) and modify a vertical file
# without staging. The gate must still fire.
git reset -q --hard "$(git merge-base HEAD origin/main)"
echo "dirty edit" > verticals/example/manifest.toml

CARGO_LOG="$FIX/cargo-unstaged.log"
export CARGO_LOG
: > "$CARGO_LOG"

echo
echo "Case 4: unstaged-only vertical change — regression MUST run"
"$SCRIPT" >/dev/null 2>&1
rc=$?
assert_eq "unstaged-path exit 0" "0" "$rc"

regression_hits=$(grep -c -- '-p argo-vertical-regression --test regression_all' "$CARGO_LOG" || true)
assert_eq "unstaged-path: regression ran once"     "1" "$regression_hits"

# ──────────────────────────────────────────────── final summary
echo
echo "──────────────────────────────────────"
echo "  passed: $PASS"
echo "  failed: $FAIL"
if [ "$FAIL" -gt 0 ]; then
  echo
  echo "Failing assertions:"
  for n in "${FAIL_NAMES[@]:-}"; do echo "  - $n"; done
  exit 1
fi
echo
echo "✓ codex-pre-push-vertical-regression.test.sh PASSED ($PASS assertions)"
