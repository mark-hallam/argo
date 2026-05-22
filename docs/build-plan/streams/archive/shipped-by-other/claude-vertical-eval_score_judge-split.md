# claude-vertical-eval_score_judge-split — split argo-vertical/src/eval_score_judge.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 7157bae4 on 2026-05-10.

## Why

`crates/argo-vertical/src/eval_score_judge.rs` is at 423/500 LOC. Pre-emptive split keeps headroom for future eval-judge backend additions.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by judge backend (e.g. mock vs real) or by score-aggregation logic.
2. **Extract to sibling** `eval_score_judge_<topic>.rs`.
3. **Re-export via `pub(crate) use`**.
4. **Test.** `cargo clippy -p argo-vertical --all-targets -- -D warnings` clean. `cargo test -p argo-vertical` green.

## Files you may touch

- `crates/argo-vertical/src/eval_score_judge.rs` (must drop below 400 LOC)
- `crates/argo-vertical/src/eval_score_judge_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-vertical/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-vertical/`

## Acceptance criteria

- [ ] `eval_score_judge.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-vertical --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-vertical` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-vertical-eval_score_judge-split`
- PR title: `argo-vertical: split eval_score_judge.rs below 500-LOC ceiling (claude-vertical-eval_score_judge-split)`
