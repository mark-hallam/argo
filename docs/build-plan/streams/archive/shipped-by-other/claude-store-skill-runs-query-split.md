# claude-store-skill-runs-query-split — split skill_runs_query.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 6a753bae on 2026-05-10.

## Why

`crates/argo-store/src/skill_runs_query.rs` is at 473 LOC. Future skill-runs query helpers (canary routing aggregation, per-tenant rollups) will exceed 500. Pure refactor; mirrors proven pattern (#1121 memory-store, #1131 marketplace-bundle).

## Approach

1. Read `skill_runs_query.rs` end-to-end. Identify a cohesive group (likely a query family — by tenant, by skill, by time-window).
2. Move to `crates/argo-store/src/skill_runs_query_<topic>.rs`.
3. Re-export via `pub(crate) use`.

Target: `skill_runs_query.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-store/src/skill_runs_query.rs` — narrowly: extract sibling, re-export
- `crates/argo-store/src/skill_runs_query_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-store/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Files you must NOT touch

- Any other `skill_runs*.rs` source — keep diff narrow
- Migration files — pure refactor, no schema change
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `skill_runs_query.rs` ≤ 350 LOC (≥ 150 LOC headroom)
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store` clean
- [ ] No public API changes

## Branch + PR

- Branch: `claude-store-skill-runs-query-split`
- PR title: `store: split skill_runs_query.rs below 500-LOC ceiling (claude-store-skill-runs-query-split)`
