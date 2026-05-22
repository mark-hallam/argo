# claude-cli-handlers-activity_since-split — split argo-cli/src/handlers/activity_since.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 9d8a18df on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/activity_since.rs` is at 433/500 LOC. Pre-emptive split keeps headroom for future feature briefs.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by activity-source aggregator (e.g. session vs handoff vs audit) or by output formatter. Read end-to-end.
2. **Extract to sibling** `activity_since_<topic>.rs`.
3. **Re-export via `pub(crate) use`**.
4. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/src/handlers/activity_since.rs` (must drop below 400 LOC)
- `crates/argo-cli/src/handlers/activity_since_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/`

## Acceptance criteria

- [ ] `activity_since.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-handlers-activity_since-split`
- PR title: `argo-cli: split handlers/activity_since.rs below 500-LOC ceiling (claude-cli-handlers-activity_since-split)`
