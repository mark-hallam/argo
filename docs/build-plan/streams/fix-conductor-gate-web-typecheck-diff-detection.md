# fix-conductor-gate-web-typecheck-diff-detection

**Owner**: claude or codex — **P1 GATE-INTEGRITY.**
**Depends on**: none.
**Authored against**: origin/main `c8b69418f` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-conductor-gate-web-typecheck-diff-detection

## Problem (observed gating #2860, 2026-06-12)

`conductor-pre-merge-gate.sh` step [7/9] web-typecheck **SKIPPED a web-only
PR** — it printed `[web-typecheck] no web/** files in diff — SKIP` for
PR #2860 whose diff was THREE `web/**` files
(`web/lib/api/capability-requests.ts`,
`web/components/skycot/worker/WorkerOverviewContent.tsx` + its
`.capability.test.tsx`). The conductor had to verify vitest (4/4) + `tsc
--noEmit` (0 errors) by hand to merge safely.

This is a **gate-integrity defect**: the web typecheck + vitest are the ONLY
gate steps that exercise a web-only PR (the Rust steps all cache-hit on an
unchanged workspace). If the diff-detection misfires, a broken web PR — type
error, failing test — would pass the gate untested. The same misfire likely
affects the vitest run if it shares the detection.

## Likely cause (confirm, don't assume)

The web-typecheck step computes its file list from a `git diff --name-only`.
In the isolated build worktree (`ARGO_CONDUCTOR_BUILD_WORKTREE=1`) the diff
**base** is probably wrong — e.g. it diffs `HEAD~1..HEAD` or
`origin/main..HEAD` against a worktree whose HEAD is the squashed/checked-out
PR with a base that makes the web files fall outside the computed range, so
the `grep web/` finds nothing. (Note the gate ran "on abb020896" — confirm
which ref the web-diff is computed against vs the PR head.)

## Fix
1. Find the web-file-detection in `scripts/conductor-pre-merge-gate.sh`
   (step 7) + any helper it calls.
2. Make the diff base the PR's actual merge-base with `origin/main` (the same
   base the Rust steps use), so `web/**` changes are reliably detected in the
   isolated-worktree path.
3. Add a guard: if the PR's GitHub file list (`gh pr view <n> --json files`)
   contains `web/**` but the gate computed zero web files, FAIL loudly rather
   than silently SKIP — a detection miss must not read as "nothing to check".

## Acceptance
- Re-running the gate on a known web-only PR runs vitest + `tsc` (does NOT
  SKIP). A test/fixture PR with a deliberate web type error FAILS the gate.
- The `gh`-file-list cross-check trips if detection finds zero web files when
  GitHub says there are web files.

~40-100 LOC (shell). **🔴 P1 — until fixed, web PRs are effectively
un-gated; the conductor must hand-verify every web PR (as done for #2860).**
