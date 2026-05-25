#!/usr/bin/env bash
# test-codex-handoff-no-cargo-gate.sh — hermetic smoke harness for
# scripts/check-codex-handoff-no-cargo-gate.sh.
#
# Covers:
#   1. Positive: the current (canonical) handoff prompt passes.
#   2. Negative: a mutated prompt with an imperative "Run cargo …"
#      directive in the codex section fails.
#   3. Negative: a mutated prompt with "Run scripts/conductor-pre-merge-gate.sh"
#      as an imperative fails.
#   4. Negative: a mutated prompt with "Execute `cargo clippy --workspace`"
#      fails.
#   5. Negative: a mutated prompt with the CODEX CONDUCTOR OVERRIDE
#      header removed fails (boundary anchor missing).
#   6. Negative: a mutated prompt with the "Do NOT run workspace clippy"
#      prohibition softened to "Avoid running workspace clippy" fails
#      (required-anchor lost).
#   7. Positive: worker-facing language elsewhere in the doc is
#      permitted — appending a worker-mode "Run cargo test" inside a
#      worker-clearly-marked block must not trip the lint when the
#      directive carries a negation guard. (We assert this by adding a
#      "do not run cargo" sentence and verifying lint still passes.)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_SCRIPTS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$REPO_SCRIPTS_DIR/.." && pwd)"
LINT="$REPO_SCRIPTS_DIR/check-codex-handoff-no-cargo-gate.sh"
CANONICAL="$REPO_ROOT/docs/skycot/operator/codex-handoff-prompt.md"

if [[ ! -x "$LINT" ]]; then
    echo "FAIL: lint not found or not executable: $LINT" >&2
    exit 2
fi
if [[ ! -f "$CANONICAL" ]]; then
    echo "FAIL: canonical prompt not found: $CANONICAL" >&2
    exit 2
fi

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

PASSED=0
FAILED=0
pass() { echo "✓ $1"; PASSED=$((PASSED + 1)); }
fail() { echo "✗ $1" >&2; FAILED=$((FAILED + 1)); }

# expect_pass <prompt_path> <name>
expect_pass() {
    local prompt="$1" name="$2"
    if bash "$LINT" "$prompt" > /dev/null 2>&1; then
        pass "$name (lint passes)"
    else
        fail "$name (expected lint to pass, but it failed)"
    fi
}

# expect_fail <prompt_path> <name>
expect_fail() {
    local prompt="$1" name="$2"
    if bash "$LINT" "$prompt" > /dev/null 2>&1; then
        fail "$name (expected lint to fail, but it passed)"
    else
        pass "$name (lint fails as expected)"
    fi
}

# Test 1 — canonical prompt passes.
expect_pass "$CANONICAL" "canonical prompt passes lint"

# Test 2 — imperative "Run cargo test" in the conductor section fails.
fixture2="$TMPDIR/case-run-cargo.md"
cp "$CANONICAL" "$fixture2"
# Insert a forbidden directive after the override header.
awk '
    /^CODEX CONDUCTOR OVERRIDE/ {
        print
        print ""
        print "Run cargo test before merging each PR."
        next
    }
    { print }
' "$CANONICAL" > "$fixture2"
expect_fail "$fixture2" "imperative 'Run cargo test' is rejected"

# Test 3 — imperative "Run scripts/conductor-pre-merge-gate.sh" fails.
fixture3="$TMPDIR/case-run-pre-merge-gate.md"
awk '
    /^CODEX CONDUCTOR OVERRIDE/ {
        print
        print ""
        print "Run scripts/conductor-pre-merge-gate.sh on every queue tick."
        next
    }
    { print }
' "$CANONICAL" > "$fixture3"
expect_fail "$fixture3" "imperative 'Run scripts/conductor-pre-merge-gate.sh' is rejected"

# Test 4 — "Execute `cargo clippy --workspace`" fails.
fixture4="$TMPDIR/case-execute-cargo-clippy.md"
awk '
    /^CODEX CONDUCTOR OVERRIDE/ {
        print
        print ""
        print "Execute `cargo clippy --workspace --all-targets` before each merge."
        next
    }
    { print }
' "$CANONICAL" > "$fixture4"
expect_fail "$fixture4" "imperative 'Execute \`cargo clippy\`' is rejected"

# Test 5 — removing the CODEX CONDUCTOR OVERRIDE header fails (anchor lost).
fixture5="$TMPDIR/case-missing-header.md"
grep -v "CODEX CONDUCTOR OVERRIDE" "$CANONICAL" > "$fixture5"
expect_fail "$fixture5" "missing 'CODEX CONDUCTOR OVERRIDE' header is rejected"

# Test 6 — softening "Do NOT run workspace clippy" loses the anchor.
fixture6="$TMPDIR/case-softened-prohibition.md"
sed 's/Do NOT run workspace clippy/Avoid running workspace clippy/' "$CANONICAL" > "$fixture6"
expect_fail "$fixture6" "softened 'Do NOT run workspace clippy' is rejected"

# Test 7 — adding a NEGATED directive (worker-style guidance) still
# passes; the directive verb appears, but in negation context.
fixture7="$TMPDIR/case-negated-directive.md"
awk '
    /^CODEX-SPECIFIC NOTES:/ {
        print
        print ""
        print "- Codex conductor sessions should never run cargo clippy directly;"
        print "  workers own that gate."
        next
    }
    { print }
' "$CANONICAL" > "$fixture7"
expect_pass "$fixture7" "negated directive ('never run cargo clippy') is permitted"

echo
echo "Passed: $PASSED   Failed: $FAILED"
if (( FAILED > 0 )); then
    exit 1
fi
