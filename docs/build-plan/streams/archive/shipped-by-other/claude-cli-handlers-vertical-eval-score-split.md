# claude-cli-handlers-vertical-eval-score-split — split argo-cli/src/handlers/vertical_eval_score.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main dd9362a0 on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/vertical_eval_score.rs` is at 449/500 LOC. Pre-emptive split keeps headroom for future feature briefs.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically the score-computation logic vs the rendering/output, or per-eval-kind dispatch. Read the file end-to-end before deciding the cut.
2. **Extract to sibling** `vertical_eval_score_<topic>.rs` in the same directory.
3. **Re-export via `pub(crate) use`** so dispatcher import surface stays unchanged.
4. **Keep public API unchanged.**
5. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/src/handlers/vertical_eval_score.rs` (must drop below 400 LOC)
- `crates/argo-cli/src/handlers/vertical_eval_score_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/`

## Acceptance criteria

- [ ] `vertical_eval_score.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-handlers-vertical-eval-score-split`
- PR title: `argo-cli: split handlers/vertical_eval_score.rs below 500-LOC ceiling (claude-cli-handlers-vertical-eval-score-split)`
