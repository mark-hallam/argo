# claude-cli-handlers-work-loop-split — split work_loop.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main d564971d on 2026-05-09 (conductor-authored after observing work_loop.rs at 498/500 LOC).

## Why

`crates/argo-cli/src/handlers/work_loop.rs` is at 498 LOC on current main — 2 LOC of headroom against the 500-LOC HARD RULE 6 ceiling. Multi-step / orchestration briefs touch this hot path. Preemptive split prevents future ceiling-refusals.

Pure refactor; mirrors proven pattern.

## Approach (concrete)

1. Read `work_loop.rs` end-to-end. Identify a cohesive section (likely state-machine helpers or per-step formatters).
2. Move to `crates/argo-cli/src/handlers/work_loop_<topic>.rs`.
3. Re-export via `pub(crate) use`.

Target: `work_loop.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-cli/src/handlers/work_loop.rs` — narrowly: extract sibling, re-export
- `crates/argo-cli/src/handlers/work_loop_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line if needed

## Files you must NOT touch

- Other work-related handlers
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `work_loop.rs` ≤ 350 LOC (≥ 150 LOC headroom)
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` clean
- [ ] No public API changes

## Branch + PR

- Branch: `claude-cli-handlers-work-loop-split`
- PR title: `cli: split handlers/work_loop.rs below 500-LOC ceiling (claude-cli-handlers-work-loop-split)`

## Unblocks

Future multi-step / orchestration briefs that extend work_loop.rs.
