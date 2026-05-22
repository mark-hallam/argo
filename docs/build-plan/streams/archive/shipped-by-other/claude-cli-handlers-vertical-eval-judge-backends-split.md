# claude-cli-handlers-vertical-eval-judge-backends-split — split vertical_eval_judge_backends.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 828e5e1b on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/vertical_eval_judge_backends.rs` is at 472 LOC. Future judge-backend additions (per-vertical scoring rubrics, hosted-LLM judge plumbing) will push over 500. Pure refactor mirrors proven pattern (#1130, #1135, etc.).

## Approach

1. Read `vertical_eval_judge_backends.rs` end-to-end. Identify a cohesive group (likely one backend implementation — heuristic vs. LLM-based, or a helper family).
2. Move to `crates/argo-cli/src/handlers/vertical_eval_judge_backends_<topic>.rs`.
3. Re-export via `pub(crate) use` if needed.

Target: `vertical_eval_judge_backends.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-cli/src/handlers/vertical_eval_judge_backends.rs` — narrowly: extract sibling
- `crates/argo-cli/src/handlers/vertical_eval_judge_backends_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `mod` declaration if needed

## Acceptance criteria

- [ ] `vertical_eval_judge_backends.rs` ≤ 350 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` clean (boundary note OK for pre-existing flakes outside may-touch per HARD RULE 2)
- [ ] No public API changes

## Branch + PR

- Branch: `claude-cli-handlers-vertical-eval-judge-backends-split`
- PR title: `cli: split vertical_eval_judge_backends.rs below 500-LOC ceiling (claude-cli-handlers-vertical-eval-judge-backends-split)`
