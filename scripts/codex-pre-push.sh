#!/usr/bin/env bash
# codex-pre-push.sh — blocking workspace-clippy gate for codex worker tabs.
#
# Required by docs/build-plan/SWARM_PROTOCOL.md (Verification section)
# before `git push -u origin <branch>` from a codex tab. Catches lint
# regressions introduced after the crate-scoped check ran — most
# commonly post-rebase / post-conflict-resolution edits (#905 pattern:
# compacted single-line if-chains tripping `collapsible_if`).
#
# Exit codes:
#   0 — clean. Push.
#   1 — cargo failed OR an `^error` line in the captured output.
#       Do not push.

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

echo "✅ Pre-push gate passed. Safe to push."
