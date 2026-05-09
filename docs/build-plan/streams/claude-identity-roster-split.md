# claude-identity-roster-split — split identity roster.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 2bb5464d on 2026-05-10.

## Why

`crates/argo-identity/src/roster.rs` is at 465 LOC. Future roster-extension briefs (RBAC roles, role-removal lifecycle, multi-org support) will exceed 500. Pure refactor mirrors proven pattern.

## Approach

1. Read `roster.rs` end-to-end. Identify a cohesive group (likely a CRUD subsection or a helper family).
2. Move to `crates/argo-identity/src/roster_<topic>.rs`.
3. Re-export via `pub(crate) use`.

Target: `roster.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-identity/src/roster.rs` — extract sibling
- `crates/argo-identity/src/roster_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-identity/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Acceptance criteria

- [ ] `roster.rs` ≤ 350 LOC
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-identity --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-identity` clean
- [ ] No public API changes

## Branch + PR

- Branch: `claude-identity-roster-split`
- PR title: `identity: split roster.rs below 500-LOC ceiling (claude-identity-roster-split)`
