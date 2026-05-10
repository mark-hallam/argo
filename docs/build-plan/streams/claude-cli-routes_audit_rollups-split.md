# claude-cli-routes_audit_rollups-split — split argo-cli/src/handlers/gateway/routes_audit_rollups.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main a7e516d4 on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/gateway/routes_audit_rollups.rs` is at 410/500 LOC. Pre-emptive split keeps headroom.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by rollup-window kind (hourly vs daily) or aggregation helpers.
2. **Extract to sibling** `routes_audit_rollups_<topic>.rs`.
3. **Re-export via `pub(crate) use`**.
4. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_rollups.rs` (must drop below 400 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_audit_rollups_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/`

## Acceptance criteria

- [ ] `routes_audit_rollups.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-routes_audit_rollups-split`
- PR title: `argo-cli: split routes_audit_rollups.rs below 500-LOC ceiling (claude-cli-routes_audit_rollups-split)`
