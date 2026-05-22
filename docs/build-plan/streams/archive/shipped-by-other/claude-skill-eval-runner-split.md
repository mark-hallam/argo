# claude-skill-eval-runner-split — split eval_runner.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 2bb5464d on 2026-05-10.

## Why

`crates/argo-skill/src/eval_runner.rs` is at 469 LOC. Future eval-rubric / scorer additions push this over 500. Pure refactor mirrors proven pattern.

## Approach

1. Read `eval_runner.rs` end-to-end. Identify a cohesive group (likely a scorer family or rubric helper block).
2. Move to `crates/argo-skill/src/eval_runner_<topic>.rs`.
3. Re-export via `pub(crate) use`.

Target: `eval_runner.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-skill/src/eval_runner.rs` — extract sibling
- `crates/argo-skill/src/eval_runner_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-skill/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Acceptance criteria

- [ ] `eval_runner.rs` ≤ 350 LOC
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-skill --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-skill` clean
- [ ] No public API changes

## Branch + PR

- Branch: `claude-skill-eval-runner-split`
- PR title: `skill: split eval_runner.rs below 500-LOC ceiling (claude-skill-eval-runner-split)`
