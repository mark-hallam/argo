# split-pr-2787-turn-helpers-run-under-500-and-gate

Authored against: origin/main a2fb7bb5f09e3330247c92a4021d03f58f515914 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh split-pr-2787-turn-helpers-run-under-500-and-gate

## Goal

Get PR #2787 FULLY gate-green so it can merge. #2787's security logic
(memory-tool tenant+vertical scope) is adversarially-reviewed CONFIRMED-FIXED,
and its test functions were already split (#2787 commit 0432b230b). The
remaining blocker: after rebase on #2788 (which also grew the same file),
`crates/argo-agent-runner/src/turn_helpers_run.rs` is 503 LOC (>500 hard ceiling).

## Why now

#2787 closes a confirmed cross-vertical memory leak in the in-agent memory tool
(WS1). It has been through 2 NEEDS-FIX cycles (fn-LOC, now file-LOC) because the
worker pipeline does not run the conductor gate before opening. This brief MUST
end the iteration: get it fully gate-clean in one pass.

## How

1. Check out #2787's branch
   (`claude-fix-agent-runtime-memory-tool-tenant-and-vertical-scope`), rebase on
   current main.
2. Extract a self-contained block from `turn_helpers_run.rs` into a sibling
   helper (e.g. the memory-context / namespace-binding construction → a small
   `fn build_memory_ctx(...)`, or move a cohesive helper to a new
   `turn_helpers_run_memory.rs` module) to drop the file under 500 LOC. Pure
   extraction — preserve behavior exactly (the review covered the current logic;
   a behavior change would invalidate it).
3. **BEFORE re-opening: run `ARGO_CONDUCTOR_BUILD_WORKTREE=1 bash
   scripts/conductor-pre-merge-gate.sh` locally and fix EVERY failure** (LOC,
   clippy, fn-LOC, tests) so the PR is green in ONE pass. Do not re-open until
   the gate exits 0.
4. Force-push to #2787's branch.

## Acceptance / verifier

1. `wc -l turn_helpers_run.rs` (+ any new sibling) all < 500.
2. Full `conductor-pre-merge-gate.sh` exits 0 (the worker pastes the PASS tail).
3. `cargo test -p argo-agent-runner` green — the isolation tests still pass.
4. `git diff` shows only mechanical extraction (no logic change to the scoping).

## Size / owner

~mechanical extraction + gate run. Owner: claude. 🔴 P1 — unblocks the
review-CONFIRMED #2787 (memory-tool tenant+vertical isolation, WS1); ends a
3-cycle NEEDS-FIX loop.
