# claude-identity-policy-split — split argo-identity/src/policy.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 0dcb8cff on 2026-05-10.

## Why

`crates/argo-identity/src/policy.rs` is at 407/500 LOC. Pre-emptive split keeps headroom for future identity-policy additions.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by policy kind (RBAC vs ABAC) or by `#[cfg(test)]` mod extraction.
2. **Extract to sibling** `policy_<topic>.rs`.
3. **Re-export via `pub use`** since it's part of identity's public surface.
4. **Test.** `cargo clippy -p argo-identity --all-targets -- -D warnings` clean. `cargo test -p argo-identity` green.

## Files you may touch

- `crates/argo-identity/src/policy.rs` (must drop below 400 LOC)
- `crates/argo-identity/src/policy_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-identity/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-identity/`

## Acceptance criteria

- [ ] `policy.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-identity --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-identity` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-identity-policy-split`
- PR title: `argo-identity: split policy.rs below 500-LOC ceiling (claude-identity-policy-split)`
