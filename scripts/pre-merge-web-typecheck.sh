#!/usr/bin/env bash
# pre-merge-web-typecheck.sh — verify the SPA TypeScript surface compiles.
# Closes the gap where the conductor pre-merge gate is Rust-only, so a
# PR that breaks `pnpm tsc --noEmit` can admin-merge under the web-only
# or evaluator-pre-verified lanes and only surface on the next evaluator
# round (rounds 85 + 86 each caught one such regression — #2404 + #2366).
#
# Invoked by `scripts/conductor-pre-merge-gate.sh` as gate 7.5
# (between test-compile and workspace clippy). Skips with a green log
# line when the PR touches no `web/**` files — the typical Rust-only
# PR pays no cost.
#
# Usage:
#   bash scripts/pre-merge-web-typecheck.sh
#   bash scripts/pre-merge-web-typecheck.sh --quiet     # only print on failure
#
# Exit codes:
#   0 — tsc clean (or no web files touched in working tree vs origin/main)
#   1 — tsc reported errors; gate must fail

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

QUIET=0
[[ "${1:-}" == "--quiet" ]] && QUIET=1

log()  { [[ "$QUIET" -eq 1 ]] || printf '\033[1m[web-typecheck]\033[0m %s\n' "$*"; }
warn() { printf '\033[33m[web-typecheck]\033[0m %s\n' "$*" >&2; }
fail() { printf '\033[31m[web-typecheck FAIL]\033[0m %s\n' "$*" >&2; }

# Detect whether the current diff touches web/ at all. If working tree
# is clean vs origin/main and HEAD itself touches no web/ files, skip.
touched_web=0
if git diff --name-only origin/main...HEAD 2>/dev/null | grep -q '^web/'; then
  touched_web=1
fi
if git diff --name-only HEAD 2>/dev/null | grep -q '^web/'; then
  touched_web=1
fi
if git diff --cached --name-only 2>/dev/null | grep -q '^web/'; then
  touched_web=1
fi

if [[ "$touched_web" -eq 0 ]]; then
  log "no web/** files in diff — SKIP"
  exit 0
fi

if [[ ! -d web ]]; then
  warn "no web/ directory found — SKIP (unexpected: diff says web/ touched)"
  exit 0
fi

if ! command -v pnpm >/dev/null 2>&1; then
  fail "pnpm not found; install with: npm install -g pnpm"
  exit 1
fi

log "running pnpm --dir web exec tsc --noEmit"
tsc_out="$(pnpm --dir web exec tsc --noEmit 2>&1)"
tsc_rc=$?
if [[ "$tsc_rc" -ne 0 ]]; then
  fail "pnpm tsc --noEmit exited $tsc_rc"
  echo "$tsc_out" | head -20 >&2
  exit 1
fi

log "pnpm tsc --noEmit clean"
exit 0
