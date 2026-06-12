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
#   bash scripts/pre-merge-web-typecheck.sh --quiet      # only print on failure
#   bash scripts/pre-merge-web-typecheck.sh --pr 2860    # authoritative file
#                                                        # list from GitHub
#   bash scripts/pre-merge-web-typecheck.sh --pr 2860 --detect-only
#                                                        # print RUN/SKIP + guard
#                                                        # state; do NOT run tsc
#
# Exit codes:
#   0 — tsc clean (or no web files touched in working tree vs origin/main)
#   1 — tsc reported errors; gate must fail
#
# Why --pr exists (gate-integrity defect observed on PR #2860, 2026-06-12):
#   The conductor admin-merge path runs this gate in assorted checkout states
#   (a detached PR head, a PR rebased onto main, a locally-merged main). The
#   git-diff heuristics below go DEGENERATE in some of those states — e.g. when
#   HEAD is already an ancestor of origin/main, `git diff origin/main...HEAD` is
#   empty, so a genuinely-web-only PR reads as "no web files — SKIP" and the
#   only gate step that exercises a web PR is silently bypassed. PR #2860 (three
#   web/** files) hit exactly this; the conductor had to hand-verify tsc.
#   The fix: when a PR number is known, GitHub's own file list
#   (`gh pr view <n> --json files`) is the AUTHORITATIVE answer to "does this PR
#   touch web/**" — it is independent of whatever local ref the gate sits on.
#   The git heuristics remain as the no-PR local-dev fallback. A detection MISS
#   (GitHub says web, local git says zero) must never read as "nothing to
#   check": it forces the typecheck to run AND prints a loud banner.
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
PR_NUMBER=""
DETECT_ONLY=0
GH_BIN="${GH_BIN:-gh}"   # overridable so the smoke harness can stub gh
while [[ $# -gt 0 ]]; do
  case "$1" in
    --quiet) QUIET=1; shift ;;
    --detect-only) DETECT_ONLY=1; shift ;;
    --pr) PR_NUMBER="${2:-}"; shift 2 ;;
    --pr=*) PR_NUMBER="${1#*=}"; shift ;;
    *) shift ;;   # tolerate unknown flags (forward-compat with the gate caller)
  esac
done

log()  { [[ "$QUIET" -eq 1 ]] || printf '\033[1m[web-typecheck]\033[0m %s\n' "$*"; }
warn() { printf '\033[33m[web-typecheck]\033[0m %s\n' "$*" >&2; }
fail() { printf '\033[31m[web-typecheck FAIL]\033[0m %s\n' "$*" >&2; }

# Local git-diff heuristic. Covers no-PR local-dev runs (and cheaply
# corroborates the GitHub list when --pr IS given). Three signals: the PR
# range vs the merge-base with origin/main, the unstaged working tree, and
# the index. Inherently fragile across the conductor's many checkout states
# (see the --pr note above) — which is why GitHub is authoritative below.
local_web=0
if git diff --name-only origin/main...HEAD 2>/dev/null | grep -q '^web/'; then
  local_web=1
fi
if git diff --name-only HEAD 2>/dev/null | grep -q '^web/'; then
  local_web=1
fi
if git diff --cached --name-only 2>/dev/null | grep -q '^web/'; then
  local_web=1
fi

# Authoritative signal: GitHub's own file list for the PR. Independent of
# the local checkout ref, so it is immune to the degenerate-merge-base
# misfire that silently skipped #2860. Only consulted when --pr is given
# and `gh` is usable; a gh outage degrades to the local heuristic.
gh_queried=0
gh_web=0
gh_web_count=0
if [[ -n "$PR_NUMBER" ]] && command -v "$GH_BIN" >/dev/null 2>&1; then
  gh_files="$("$GH_BIN" pr view "$PR_NUMBER" --json files --jq '.files[].path' 2>/dev/null)"
  gh_rc=$?
  if [[ "$gh_rc" -eq 0 ]]; then
    gh_queried=1
    gh_web_count="$(printf '%s\n' "$gh_files" | grep -c '^web/' || true)"
    [[ "$gh_web_count" -gt 0 ]] && gh_web=1
  else
    warn "gh pr view $PR_NUMBER failed (rc=$gh_rc) — falling back to local git detection"
  fi
fi

# Decision + cross-check guard. GitHub is authoritative when consulted.
touched_web=0
guard_tripped=0
if [[ "$gh_queried" -eq 1 ]]; then
  if [[ "$gh_web" -eq 1 ]]; then
    touched_web=1
    if [[ "$local_web" -eq 0 ]]; then
      # DETECTION MISS — the exact #2860 failure. Do NOT silently skip:
      # run the typecheck against the real files and shout about the miss.
      guard_tripped=1
      warn "================================================================"
      warn "DETECTION MISS: GitHub reports $gh_web_count web/** file(s) on PR"
      warn "#$PR_NUMBER but local git diff found ZERO. The local checkout ref"
      warn "is degenerate (HEAD likely an ancestor of origin/main). Running"
      warn "the web typecheck anyway — a miss must never read as 'nothing to"
      warn "check'. Investigate the gate's checkout state."
      warn "================================================================"
    fi
  fi
  # gh_queried && gh_web==0 → trust GitHub: genuinely no web files.
else
  touched_web="$local_web"
fi

if [[ "$DETECT_ONLY" -eq 1 ]]; then
  if [[ "$touched_web" -eq 1 ]]; then
    printf 'decision=RUN'
    [[ "$guard_tripped" -eq 1 ]] && printf ' guard=TRIPPED'
    printf '\n'
  else
    printf 'decision=SKIP\n'
  fi
  exit 0
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
