#!/usr/bin/env bash
# Audit heuristic for the claude-config-loader-overhaul scaffold.
#
# Reports every production-Rust callsite under `crates/` and
# `mcp-servers/` that reads an `ARGO_*` env var directly via
# `std::env::var("ARGO_...")` instead of going through
# `argo_config::load()`. The follow-up brief
# `claude-config-loader-migrate-callsites` consumes this list and
# routes each callsite through the loader.
#
# Warning-only: the script always exits 0, prints the count to stdout,
# and a non-zero count surfaces in CI as an annotation rather than a
# build failure. The follow-up brief flips this to a failing gate
# once the audit list reaches zero.
#
# Test files (`tests/`, `*_tests.rs`, `tests.rs`) and the
# `crates/argo-config/` crate itself are exempt: tests legitimately
# manipulate env vars to exercise the loader, and `argo-config` is
# the one place direct reads belong.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

if command -v rg >/dev/null 2>&1; then
  HITS=$(rg -l --type rust 'std::env::var\("ARGO_' crates mcp-servers 2>/dev/null \
    | grep -Ev '(^|/)tests/|_tests\.rs$|/tests\.rs$|^crates/argo-config/' \
    | sort -u || true)
else
  HITS=$(grep -rln --include='*.rs' 'std::env::var("ARGO_' crates mcp-servers 2>/dev/null \
    | grep -Ev '(^|/)tests/|_tests\.rs$|/tests\.rs$|^crates/argo-config/' \
    | sort -u || true)
fi

COUNT=0
if [ -n "$HITS" ]; then
  COUNT=$(printf '%s\n' "$HITS" | wc -l | tr -d ' ')
fi

printf 'check-no-direct-env-reads: %s file(s) read ARGO_* directly outside crates/argo-config/.\n' "$COUNT"
if [ "$COUNT" -gt 0 ]; then
  printf 'These are tracked in docs/skycot/CONFIG_CALLSITE_AUDIT.md and migrated by\n'
  printf 'claude-config-loader-migrate-callsites. Warning-only for now.\n'
  printf '%s\n' "$HITS" | sed 's/^/  /'
fi

exit 0
