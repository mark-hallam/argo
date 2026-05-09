#!/usr/bin/env bash
# local-ci-check.sh — full CI gauntlet runner for local trust-ladder use.
#
# GitHub Actions workflows are intentionally disabled (operator cost
# control; .github/workflows/*.yml.disabled). Local verification is the
# canonical trust signal for merging. This script runs the same checks
# the disabled ci.yml workflow would, in the same order, with the same
# gates. See docs/skycot/CI_FALLBACK_RUNBOOK.md for the trust ladder
# and re-enabling instructions.
#
# Usage:
#   scripts/local-ci-check.sh                # run full gauntlet
#   scripts/local-ci-check.sh --fast         # skip workspace tests (clippy + fmt + deny + budgets only)
#   scripts/local-ci-check.sh --tier 2       # equivalent to --fast (Tier 2 trust level)
#   scripts/local-ci-check.sh --pr 1234      # check out PR 1234 in a worktree, verify, clean up
#
# Exit codes:
#   0   — all gates passed (mergeable per Trust ladder)
#   1   — one or more gates failed
#   2   — script invocation error
#
# See docs/skycot/CI_FALLBACK_RUNBOOK.md for trust-ladder semantics.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

MODE="full"
PR_NUMBER=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --fast|--tier|--tier=2)
            MODE="fast"
            [[ "$1" == "--tier" && "${2:-}" == "2" ]] && shift
            shift
            ;;
        --tier=*)
            MODE="fast"
            shift
            ;;
        --pr)
            PR_NUMBER="${2:-}"
            shift 2
            ;;
        --help|-h)
            sed -n '2,18p' "$0" | sed 's/^# \?//'
            exit 0
            ;;
        *)
            echo "unknown arg: $1" >&2
            exit 2
            ;;
    esac
done

# PR mode: spin up a worktree, verify there, clean up
WORKTREE=""
cleanup() {
    if [[ -n "$WORKTREE" && -d "$WORKTREE" ]]; then
        git worktree remove "$WORKTREE" --force >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

if [[ -n "$PR_NUMBER" ]]; then
    BRANCH="$(gh pr view "$PR_NUMBER" --json headRefName --jq '.headRefName' 2>/dev/null)"
    if [[ -z "$BRANCH" ]]; then
        echo "FAIL: could not resolve PR #$PR_NUMBER" >&2
        exit 2
    fi
    WORKTREE="/tmp/local-ci-pr$PR_NUMBER"
    rm -rf "$WORKTREE"
    git fetch origin "$BRANCH" >/dev/null 2>&1
    git worktree add "$WORKTREE" "origin/$BRANCH" >/dev/null
    cd "$WORKTREE"
    echo "Verifying PR #$PR_NUMBER ($BRANCH) in $WORKTREE"
fi

# Track per-gate results so we report all failures rather than stopping
# at the first.
PASS=()
FAIL=()
gate() {
    local name="$1"; shift
    local start=$SECONDS
    echo ""
    echo "=== Gate: $name ==="
    if "$@"; then
        local elapsed=$((SECONDS - start))
        PASS+=("$name (${elapsed}s)")
        echo "✅ $name passed (${elapsed}s)"
    else
        local rc=$?
        local elapsed=$((SECONDS - start))
        FAIL+=("$name (rc=$rc, ${elapsed}s)")
        echo "❌ $name FAILED (rc=$rc, ${elapsed}s)"
    fi
}

# 1. Formatting
fmt_check() { cargo fmt --check; }
gate "cargo fmt --check" fmt_check

# 2. Workspace clippy
clippy_check() {
    cargo clippy --workspace --all-targets -- -D warnings 2>&1 \
      | grep -vE '^\s+(Compiling|Checking|Downloading|Updating|Compiling|warning: unused) ' \
      | tail -50
    # propagate cargo's exit code, not grep's
    [[ ${PIPESTATUS[0]} -eq 0 ]]
}
gate "cargo clippy --workspace -D warnings" clippy_check

# 3. cargo deny (supply chain)
deny_check() {
    if ! command -v cargo-deny >/dev/null 2>&1; then
        echo "  (cargo-deny not installed; skipping. Install: cargo install cargo-deny)"
        return 0
    fi
    cargo deny check 2>&1 | tail -30
    [[ ${PIPESTATUS[0]} -eq 0 ]]
}
gate "cargo deny check" deny_check

# 4. File LOC budget — no source file > 500 LOC
loc_budget_check() {
    local violators
    violators=$(find crates -name "*.rs" -not -path "*/target/*" \
                | xargs wc -l 2>/dev/null \
                | awk '$1 > 500 && $2 != "total" { print $1, $2 }' \
                | head -20)
    if [[ -n "$violators" ]]; then
        echo "$violators" | while read -r loc path; do
            echo "  ⚠️  $loc LOC: $path"
        done
        return 1
    fi
    echo "  all .rs files ≤ 500 LOC"
    return 0
}
gate "loc-budget (≤500 LOC per .rs)" loc_budget_check

# 5. Function LOC budget — no fn > 80 LOC. Approximate via brace-depth scan.
fn_budget_check() {
    local script="$REPO_ROOT/scripts/check-fn-loc.sh"
    if [[ -x "$script" ]]; then
        "$script"
        return $?
    fi
    echo "  (no scripts/check-fn-loc.sh; skipping fn-LOC gate)"
    return 0
}
gate "fn-budget (≤80 LOC per fn)" fn_budget_check

# 6. Prepared-statements gate (argo-store query layer)
prep_stmts_check() {
    local script="$REPO_ROOT/scripts/check-prepared-statements.sh"
    if [[ -x "$script" ]]; then
        "$script"
        return $?
    fi
    echo "  (no scripts/check-prepared-statements.sh; skipping)"
    return 0
}
gate "prepared-statements" prep_stmts_check

# 7. Span-naming gate
span_naming_check() {
    local script="$REPO_ROOT/scripts/check-span-naming.sh"
    if [[ -x "$script" ]]; then
        "$script"
        return $?
    fi
    echo "  (no scripts/check-span-naming.sh; skipping)"
    return 0
}
gate "span-naming" span_naming_check

# 8. Workspace tests (skipped in --fast mode)
if [[ "$MODE" != "fast" ]]; then
    test_check() {
        cargo test --workspace --no-fail-fast 2>&1 | tail -50
        [[ ${PIPESTATUS[0]} -eq 0 ]]
    }
    gate "cargo test --workspace" test_check
fi

# Summary
echo ""
echo "=========================================="
echo "Local CI check summary"
echo "=========================================="
for p in "${PASS[@]}"; do echo "✅ $p"; done
for f in "${FAIL[@]}"; do echo "❌ $f"; done
echo ""

if (( ${#FAIL[@]} == 0 )); then
    echo "✅ All gates passed."
    if [[ "$MODE" == "fast" ]]; then
        echo "   (--fast: workspace tests skipped — Tier 2 trust level)"
    else
        echo "   Full gauntlet — Tier 3 trust level (highest while CI is broken)"
    fi
    exit 0
else
    echo "❌ ${#FAIL[@]} gate(s) failed. Fix before merging."
    exit 1
fi
