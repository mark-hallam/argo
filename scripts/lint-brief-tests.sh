#!/usr/bin/env bash
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$REPO_ROOT/scripts/lint-brief.sh"

failures=0
fail() {
  echo "FAIL: $1" >&2
  failures=$((failures + 1))
}

assert_exit() {
  local expected="$1"; shift
  local actual
  set +e
  "$@" >/tmp/lint-brief-test.out 2>&1
  actual=$?
  set -e
  if [[ "$actual" -ne "$expected" ]]; then
    fail "expected exit $expected, got $actual for: $*"
    cat /tmp/lint-brief-test.out >&2
  fi
}

assert_grep() {
  local pattern="$1"; shift
  set +e
  output="$("$@" 2>&1)"
  set -e
  if ! grep -qF "$pattern" <<<"$output"; then
    fail "expected output to contain '$pattern' for: $*"
    echo "actual output:" >&2
    echo "$output" >&2
  fi
}

assert_not_grep() {
  local pattern="$1"; shift
  set +e
  output="$("$@" 2>&1)"
  set -e
  if grep -qF "$pattern" <<<"$output"; then
    fail "expected output to NOT contain '$pattern' for: $*"
    echo "$output" >&2
  fi
}

# Setup a known-good fixture brief
GOOD_BRIEF="lint-brief-test-good"
GOOD_PATH="$REPO_ROOT/docs/build-plan/streams/${GOOD_BRIEF}.md"
HEAD_SHA="$(git -C "$REPO_ROOT" rev-parse HEAD)"
TODAY="$(date +%Y-%m-%d)"
cat >"$GOOD_PATH" <<EOF
# ${GOOD_BRIEF} — synthetic fixture for lint-brief tests

**Owner**: claude.
**Authored against**: origin/main \`${HEAD_SHA:0:8}\` on \`${TODAY}\`.
**Validate before claim**: \`bash scripts/check-brief-freshness.sh ${GOOD_BRIEF}\`.

## Scope

A made-up brief that introduces NEW \`SkycotLintBriefFixtureType\` — a name unlikely
to collide with anything in crates/.

## Files you may touch
- nothing real.
EOF

# 1. Good brief: clean, exit 0
assert_exit 0 "$SCRIPT" "$GOOD_BRIEF"
assert_grep "ok" "$SCRIPT" "$GOOD_BRIEF"

# 2. Good brief with --enforce also clean
assert_exit 0 "$SCRIPT" --enforce "$GOOD_BRIEF"

# 3. Bad brief (missing headers)
BAD_BRIEF="lint-brief-test-bad"
BAD_PATH="$REPO_ROOT/docs/build-plan/streams/${BAD_BRIEF}.md"
cat >"$BAD_PATH" <<EOF
# ${BAD_BRIEF}

No headers at all. Just text.
EOF
assert_grep "missing '**Authored against**" "$SCRIPT" "$BAD_BRIEF"
assert_grep "missing '**Validate before claim**" "$SCRIPT" "$BAD_BRIEF"
# WARN mode exits 0 even with findings
assert_exit 0 "$SCRIPT" "$BAD_BRIEF"
# Enforce mode exits 1
assert_exit 1 "$SCRIPT" --enforce "$BAD_BRIEF"

# 4. Parallel-surface detection: brief introduces NEW name that exists in crates/
PARALLEL_BRIEF="lint-brief-test-parallel-surface"
PARALLEL_PATH="$REPO_ROOT/docs/build-plan/streams/${PARALLEL_BRIEF}.md"
cat >"$PARALLEL_PATH" <<EOF
# ${PARALLEL_BRIEF}

**Authored against**: origin/main \`${HEAD_SHA:0:8}\` on \`${TODAY}\`.
**Validate before claim**: \`bash scripts/check-brief-freshness.sh ${PARALLEL_BRIEF}\`.

## Scope

Introduces NEW \`Auction\` to handle bidding. (Should flag — Auction exists in
crates/argo-orchestrator/.)
EOF
assert_grep "potential parallel-surface" "$SCRIPT" "$PARALLEL_BRIEF"
assert_grep "Auction" "$SCRIPT" "$PARALLEL_BRIEF"

# 5. -v2 sibling exemption
V2_BRIEF="${GOOD_BRIEF}-v2"
V2_PATH="$REPO_ROOT/docs/build-plan/streams/${V2_BRIEF}.md"
cat >"$V2_PATH" <<EOF
# ${V2_BRIEF}

**Authored against**: origin/main \`${HEAD_SHA:0:8}\` on \`${TODAY}\`.
**Validate before claim**: \`bash scripts/check-brief-freshness.sh ${V2_BRIEF}\`.

## Scope
Sibling of ${GOOD_BRIEF}; near-name check should skip.
EOF
assert_not_grep "substring" "$SCRIPT" "$V2_BRIEF"
assert_not_grep "overlap" "$SCRIPT" "$V2_BRIEF"
assert_exit 0 "$SCRIPT" "$V2_BRIEF"

# 6. Help works
assert_exit 0 "$SCRIPT" --help
assert_grep "usage:" "$SCRIPT" --help

# 7. Missing brief errors with exit 2
assert_exit 2 "$SCRIPT" lint-brief-test-nonexistent-brief

# Cleanup
rm -f "$GOOD_PATH" "$BAD_PATH" "$PARALLEL_PATH" "$V2_PATH" /tmp/lint-brief-test.out

if [[ "$failures" -gt 0 ]]; then
  echo "lint-brief-tests: ${failures} failure(s)" >&2
  exit 1
fi
echo "lint-brief-tests passed"
