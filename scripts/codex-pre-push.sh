#!/usr/bin/env bash
# codex-pre-push.sh — blocking workspace-clippy gate for codex worker tabs.
#
# Required by docs/build-plan/SWARM_PROTOCOL.md (Verification section)
# before `git push -u origin <branch>` from a codex tab. Catches lint
# regressions introduced after the crate-scoped check ran — most
# commonly post-rebase / post-conflict-resolution edits (#905 pattern:
# compacted single-line if-chains tripping `collapsible_if`).
#
# Additional vertical-regression gate: when the branch (or local
# staged/uncommitted changes) touches `verticals/**`, also run
# `cargo test -p argo-vertical-regression --test regression_all` so
# fixture↔manifest drift is caught at push-time. The harness is the
# manual runbook command from
# `docs/skycot/runbook/per-vertical-regression.md`; keep it literal so
# any future runbook drift is obvious. No vertical changes → skip the
# extra test to keep the no-op path cheap.
#
# Exit codes:
#   0 — clean. Push.
#   1 — cargo failed OR an `^error` line in the captured output, OR
#       the vertical-regression gate fired and failed. Do not push.

set -euo pipefail

OUT=$(mktemp)
trap 'rm -f "$OUT"' EXIT

echo "Running blocking workspace clippy (codex pre-push gate)..."
echo "  cargo clippy --workspace --all-targets -- -D warnings"
echo

set +e
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee "$OUT"
CARGO_RC=${PIPESTATUS[0]}
set -e

echo

if [ "$CARGO_RC" -ne 0 ]; then
  echo "🚫 Pre-push gate FAILED: cargo exited $CARGO_RC. Fix before pushing."
  exit 1
fi

if grep -qE '^error' "$OUT"; then
  echo "🚫 Pre-push gate FAILED: clippy reported errors above. Fix before pushing."
  exit 1
fi

# Vertical-regression gate. Pre-push usually runs after commits exist,
# so we union three sources of changed paths:
#   - committed changes since `git merge-base HEAD origin/main`
#   - staged but uncommitted changes (`git diff --cached`)
#   - unstaged uncommitted changes (`git diff`)
# Any path with the exact `verticals/` prefix flips the gate on.
vertical_changes_present() {
  local merge_base
  merge_base=$(git merge-base HEAD origin/main 2>/dev/null || true)

  {
    if [ -n "$merge_base" ]; then
      git diff --name-only "$merge_base" HEAD 2>/dev/null || true
    fi
    git diff --cached --name-only 2>/dev/null || true
    git diff --name-only 2>/dev/null || true
  } | grep -E '^verticals/' >/dev/null
}

if vertical_changes_present; then
  echo "Vertical paths changed — running regression harness..."
  echo "  cargo test -p argo-vertical-regression --test regression_all"
  echo
  if ! cargo test -p argo-vertical-regression --test regression_all; then
    echo "🚫 Pre-push gate FAILED: vertical regression harness failed. Fix before pushing."
    exit 1
  fi
  echo
else
  echo "No vertical paths changed — skipping vertical regression harness."
fi

echo "✅ Pre-push gate passed. Safe to push."
