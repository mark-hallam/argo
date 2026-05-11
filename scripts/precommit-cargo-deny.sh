#!/usr/bin/env bash
# Opt-in pre-push hook: run cargo-deny if it is installed.
#
# Wire manually, e.g.:
#   git config core.hooksPath .githooks
#   ln -s ../../scripts/precommit-cargo-deny.sh .githooks/pre-push
#
# Skips silently when cargo-deny is absent — we don't want to block a
# push just because the developer hasn't installed the tool. To enforce
# installation, run scripts/cargo-deny.sh once first (it installs).

set -euo pipefail

if ! cargo deny --version >/dev/null 2>&1; then
  echo "cargo-deny not installed; skipping pre-push deny check."
  echo "Run scripts/cargo-deny.sh once to install + scan."
  exit 0
fi

REPO_ROOT="$(git rev-parse --show-toplevel)"
exec "$REPO_ROOT/scripts/cargo-deny.sh"
