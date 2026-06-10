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
#
# Worktree manual check (gate-6 smoke harness doesn't cover gate 7 cheaply):
#   git worktree add /tmp/gate-wt origin/main
#   cd /tmp/gate-wt && printf '\n' >> web/package.json   # make the diff touch web/
#   bash scripts/pre-merge-web-typecheck.sh              # links node_modules, runs tsc
#   # expect: "linked web/node_modules from main checkout" then a PASS/FAIL on
#   # the CODE — never "Command tsc not found". cd "$REPO" && git worktree remove.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

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

# Worktree support. `web/node_modules` is gitignored and NOT shared by git
# worktrees, so a gate run from `git worktree add /tmp/<id> origin/main` (the
# B-1 isolated-checkout pattern) finds no `.bin/tsc` and `pnpm exec tsc` fails
# with a cryptic "Command tsc not found" (exit 254) — a false-FAIL on a green
# PR, which pushes conductors toward gate-overrides. If the main checkout (the
# git common-dir's sibling) has an installed node_modules, link it in
# (worktree-local, gitignored path, idempotent). Honest FAIL if neither the
# worktree nor the main checkout has tsc installed.
main_nm="$(git rev-parse --path-format=absolute --git-common-dir)/../web/node_modules"
if [[ ! -x web/node_modules/.bin/tsc && -x "$main_nm/.bin/tsc" ]]; then
  ln -sfn "$main_nm" web/node_modules
  warn "linked web/node_modules from main checkout (worktree run)"
fi
if [[ ! -x web/node_modules/.bin/tsc ]]; then
  fail "web/node_modules/.bin/tsc not found — run 'pnpm install' in web/"
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

# Bounded grep guard: reject brief-IDs (`E<digit>-<letter>`) inside
# user-facing strings in the worker-detail UI. Microseconds to run.
log "running scripts/check-worker-ui-no-brief-ids.sh"
if ! bash scripts/check-worker-ui-no-brief-ids.sh; then
  fail "worker UI brief-ID guard failed — see output above"
  exit 1
fi

exit 0
