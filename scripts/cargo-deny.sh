#!/usr/bin/env bash
# Local supply-chain / license / advisory scan via cargo-deny.
#
# Idempotent install of cargo-deny, then runs the 4 cargo-deny check
# kinds (advisories, bans, licenses, sources) against the workspace.
# Honors the repo-root `deny.toml`. Exits non-zero on any check failure.
#
# Usage:
#   scripts/cargo-deny.sh           # run all 4 checks
#   scripts/cargo-deny.sh --help    # show this help

set -euo pipefail

print_help() {
  sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  print_help
  exit 0
fi

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

# Idempotent install — only invokes `cargo install` when missing.
if ! cargo deny --version >/dev/null 2>&1; then
  echo "Installing cargo-deny (one-time)..."
  cargo install cargo-deny --locked
fi

cd "$REPO_ROOT"

CHECKS=(advisories bans licenses sources)
FAILED=()

for check in "${CHECKS[@]}"; do
  echo
  echo "=== cargo deny check ${check} ==="
  if ! cargo deny check "$check"; then
    FAILED+=("$check")
  fi
done

echo
echo "=== Summary ==="
if [[ ${#FAILED[@]} -eq 0 ]]; then
  echo "All 4 cargo-deny checks passed."
  exit 0
else
  echo "FAILED: ${FAILED[*]}"
  echo "See 'docs/skycot/runbook/cargo-deny.md' for triage guidance."
  exit 1
fi
