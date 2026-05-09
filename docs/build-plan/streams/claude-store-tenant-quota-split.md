# claude-store-tenant-quota-split — split tenant_quota.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 013298af on 2026-05-09 (conductor-authored after observing tenant_quota.rs at 499/500 LOC post-#1074 extension).

## Why

`crates/argo-store/src/tenant_quota.rs` is at 499 LOC after #1074 added the `check_or_reject` enforcement helper. Future quota briefs (per-route quotas, time-windowed quotas, audit integration) will hit the ceiling. Note: there's already a `tenant_quota_kinds.rs` sibling (added in #1074), so the project's accepted pattern is the `tenant_quota_<topic>.rs` sibling shape.

Pure refactor; mirrors proven pattern.

## Approach (concrete)

1. Read `tenant_quota.rs` end-to-end. Identify a cohesive section (likely query helpers or accounting math).
2. Move to `crates/argo-store/src/tenant_quota_<topic>.rs`.
3. Re-export via `pub(crate) use`.

Target: `tenant_quota.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-store/src/tenant_quota.rs` — narrowly: extract sibling, re-export
- `crates/argo-store/src/tenant_quota_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-store/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Files you must NOT touch

- `crates/argo-store/src/tenant_quota_kinds.rs` (existing sibling)
- `crates/argo-store/tests/tenant_quota_enforcement_e2e.rs` — refactor only the source
- Any other test file

## Acceptance criteria

- [ ] `tenant_quota.rs` ≤ 350 LOC (≥ 150 LOC headroom)
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store` clean
- [ ] No public API changes

## Branch + PR

- Branch: `claude-store-tenant-quota-split`
- PR title: `store: split tenant_quota.rs below 500-LOC ceiling (claude-store-tenant-quota-split)`

## Unblocks

Future quota briefs (per-route, time-windowed, audit-integration).
