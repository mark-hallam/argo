#!/usr/bin/env bash
# Pre-commit fast-path: only scan when Cargo.lock is staged.
#
# Wire into your pre-commit framework manually, e.g. as a `pre-commit`
# hook in `.git/hooks/` or as an entry in a `.pre-commit-config.yaml`.
#
# Exits 0 (skip) when Cargo.lock is unchanged; otherwise delegates to
# scripts/cargo-audit.sh and forwards its exit code.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"

if ! git diff --cached --name-only | grep -q '^Cargo\.lock$'; then
  exit 0
fi

echo "Cargo.lock changed — running vulnerability scan..."
exec "$REPO_ROOT/scripts/cargo-audit.sh"
