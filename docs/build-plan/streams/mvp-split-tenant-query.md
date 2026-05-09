# mvp-split-tenant-query — split tenant_query.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 MVP — proven split pattern.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

`crates/argo-store/src/tenant_query.rs` is at **506 LOC** — 6 over the 500-LOC ceiling. Blocks any further tenant-query helper additions.

## Approach

1. Read `tenant_query.rs` end-to-end. Identify a cohesive section (likely a per-resource family — sessions vs. workers vs. messages vs. costs).
2. Move to `crates/argo-store/src/tenant_query_<topic>.rs`.
3. Re-export via `pub(crate) use`.

Target: `tenant_query.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-store/src/tenant_query.rs`
- `crates/argo-store/src/tenant_query_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-store/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Acceptance criteria

- [ ] `tenant_query.rs` ≤ 350 LOC
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store` clean
- [ ] No public API changes

## Branch + PR

- Branch: `mvp-split-tenant-query`
- PR title: `store: split tenant_query.rs below 500-LOC ceiling (mvp-split-tenant-query)`
