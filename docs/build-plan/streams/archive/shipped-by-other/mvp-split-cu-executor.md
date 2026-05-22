# mvp-split-cu-executor — Split argo-executor-computer-use::executor.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `829b783f` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mvp-split-cu-executor`.

> **Tier P (production hardening) — UNBLOCKS 3 mission-CU briefs.** Today
> `crates/argo-executor-computer-use/src/executor.rs` is 494/500 LOC.
> This is causing pre-claim ceiling refusals on `mission-cu-parser-multi-action`,
> `mission-cu-double-screenshot-fix`, and `mission-cu-sandbox-naming-fix` —
> any non-trivial edit pushes the file over HARD RULE 6. This brief
> performs a pure-refactor split so those 3 briefs become claimable.

## Scope

Pure refactor. Reduce `crates/argo-executor-computer-use/src/executor.rs`
from 494 LOC to **≤ 380 LOC** (giving ~120-LOC headroom for downstream
mission-cu briefs to land their multi-action / inline-screenshot /
sandbox-naming changes).

Today the file holds one large `impl ComputerUseExecutor` block (lines
75-460) with the `run()` orchestrator + step-loop helpers + small
`LoopExit` enum + `stringify_verdict` free function at the end. Natural
split candidates:

1. **Extract step-loop helpers to sibling `executor_step_helpers.rs`** —
   move pure-helper functions that don't need `&self` into the new
   module. Imports stay in `executor.rs`. Targets: `stringify_verdict`,
   any other small free fns, plus any `impl` methods that don't
   genuinely need `self` (consider `pub(crate)`-ing them).
2. **Extract `LoopExit` enum + its handlers** — if the file's still
   over budget after step 1, lift the `LoopExit` enum + its
   `Outcome`-conversion logic to `executor_loop_exit.rs`.
3. **Public API stays byte-stable.** All `pub` items remain re-exported
   from `executor.rs`. No external consumer (argo-cli, etc.) needs
   any change.

This is NOT in scope: changing run-loop semantics, modifying the
Anthropic client surface, refactoring sandbox/profile interaction,
adding new helpers, or making behavioral changes. **Pure mechanical
split.**

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-executor-computer-use/src/executor.rs` — REMOVE the
  helpers being extracted; update imports. Target ≤ 380 LOC.
- `crates/argo-executor-computer-use/src/executor_step_helpers.rs`
  (NEW, ≤ 200 LOC) — extracted pure helpers.
- `crates/argo-executor-computer-use/src/executor_loop_exit.rs` (NEW,
  ≤ 150 LOC; ONLY if still over budget after step 1) — extracted
  `LoopExit` enum + handlers.
- `crates/argo-executor-computer-use/src/lib.rs` — narrowly scoped:
  ONE or TWO `pub(crate) mod executor_step_helpers;` (and possibly
  `executor_loop_exit;`) lines. NO other edits.

## Files you must NOT touch

- `crates/argo-executor-computer-use/src/anthropic.rs` /
  `anthropic_parse.rs` — Anthropic client surface, orthogonal
- `crates/argo-executor-computer-use/src/e2b.rs` — sandbox impl,
  orthogonal
- `crates/argo-executor-computer-use/src/pool.rs` /
  `pool_reaper.rs` — sandbox pool, orthogonal
- `crates/argo-executor-computer-use/src/learn.rs` — feedback loop
- Any test file (existing tests must continue to pass byte-identical;
  no new tests are needed for a pure refactor)
- Any non-`argo-executor-computer-use` crate — no public API change
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `executor.rs` is ≤ 380 LOC after the split (record final line
      count in PR body)
- [ ] `executor_step_helpers.rs` exists and is ≤ 200 LOC
- [ ] `executor_loop_exit.rs` exists IFF needed to hit the budget
      (≤ 150 LOC)
- [ ] No public API change observable from outside the crate (verify:
      no test or downstream consumer needs updating)
- [ ] All existing tests in `argo-executor-computer-use` continue
      to pass byte-identical (no test mutations)
- [ ] `cargo build -p argo-executor-computer-use` clean
- [ ] `cargo clippy -p argo-executor-computer-use --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-executor-computer-use` passes (skip
      pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mvp-split-cu-executor`
- PR title: `cu-executor: split executor.rs below 500-LOC ceiling (mvp-split-cu-executor)`

## Notes for the implementer

- The split is mechanical: identify cleanly-extractable functions,
  move them, update imports. If you find a function that "almost"
  needs `self` but uses it for one trivial thing, prefer keeping it
  in the impl block; don't contort the design just to hit LOC.
- The `LoopExit` enum has a few uses scattered across the run-loop;
  if extracting it forces 3+ uses to change, prefer keeping it in
  `executor.rs`. Only extract if you're confident about the LOC math.
- Don't touch the run-loop's structure or order. The acceptance is
  byte-identical behavior; the diff should be easy to review.
- Once merged: 3 mission-cu briefs (parser-multi-action / double-
  screenshot-fix / sandbox-naming-fix) will unblock. The conductor
  flips them back to TODO once this lands.
