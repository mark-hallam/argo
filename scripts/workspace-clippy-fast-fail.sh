#!/usr/bin/env bash
# workspace-clippy-fast-fail.sh — pre-push cross-crate drift detection.
#
# Workers run crate-scoped clippy by default (`-p argo-cli`) which is fast
# but blind to cross-crate drift: struct fields added in crate A break
# exhaustive matches in crate B; trait signature changes ripple silently.
# We've eaten this several times (most notably #965 deps.rs:252 evals
# field) — the per-crate gate passes, the workspace gate fails after merge.
#
# This script runs ONLY workspace clippy (no tests), filtered to surface
# warnings/errors without the "Compiling X v0.0.1" noise. It's the
# minimum-viable cross-crate sanity check before pushing a feature branch.
#
# Cold runtime: ~9 minutes. Warm (post-cache): ~30s-2min.
# Worth the cost for cross-crate drift detection on every push.
#
# Usage:
#   scripts/workspace-clippy-fast-fail.sh
#   scripts/workspace-clippy-fast-fail.sh --quiet     # only print on failure
#
# Exit codes:
#   0 — workspace clippy clean (mergeable from cross-crate-drift perspective)
#   1 — workspace clippy failed (cross-crate drift or genuine lint error)

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

QUIET=0
[[ "${1:-}" == "--quiet" ]] && QUIET=1

start=$SECONDS
[[ "$QUIET" -eq 0 ]] && echo "=== Workspace clippy (fast-fail) ==="

# Step 1: zero-caller-substrate lint (claude-zero-caller-substrate-lint).
# Cheaper than clippy and catches the wire-or-defer drift pattern that
# motivated the lint (PR #2034: substrate crate landed with zero
# consumers, sat unused for 4+ days before the gateway brief unblocked).
# Fail-fast here so a worker doesn't burn 9min on workspace clippy when
# a missing-caller surfaces first.
zc_output=$(bash "$REPO_ROOT/scripts/check-zero-caller-substrate.sh" 2>&1)
zc_rc=$?
if [[ "$zc_rc" -ne 0 ]]; then
    echo "❌ zero-caller-substrate lint FAILED"
    echo ""
    echo "$zc_output"
    exit 1
fi
[[ "$QUIET" -eq 0 ]] && echo "$zc_output"

# Step 2: workspace clippy.
# Filter pattern: keep warnings, errors, source-pointer arrows (-->), Finished line.
# Drop the dominant noise: "Compiling foo v0.0.1", "Checking foo v0.0.1",
# "Downloading", "Updating".
output=$(cargo clippy --workspace --all-targets -- -D warnings 2>&1)
rc=$?

elapsed=$((SECONDS - start))

if [[ "$rc" -eq 0 ]]; then
    [[ "$QUIET" -eq 0 ]] && echo "✅ workspace clippy clean (${elapsed}s)"
    exit 0
fi

# Failed — show filtered output
echo "❌ workspace clippy FAILED (${elapsed}s)"
echo ""
echo "$output" | grep -E '^(warning|error|note|help| *--> |Finished)' | tail -40
echo ""
echo "Re-run unfiltered for the offending crate:"
echo "  cargo clippy -p <crate> --all-targets -- -D warnings"
echo ""
echo "If this is a workspace-only drift (crate-scoped clippy passed but"
echo "workspace failed), the most likely cause is a cross-crate type or"
echo "trait change that isn't reflected in callers. Check the most recent"
echo "merges to crates the failing crate depends on."
exit 1
