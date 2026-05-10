# claude-cli-routes_forge-split — split argo-cli/src/handlers/gateway/routes_forge.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 04b7d2e0 on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/gateway/routes_forge.rs` is at 462/500 LOC, tight against the hard ceiling. Pre-emptive split keeps headroom for future forge-related route additions and mirrors the proven gateway-routes split pattern (#910 routes_channels, #932 routes_post, #956 routes_get).

## Approach (concrete)

1. **Identify a cohesive sibling group** — by HTTP-method (GET vs POST handlers), or by forge sub-resource (manifest vs blueprint vs runs vs assets), or extract the `#[cfg(test)]` mod via `#[path]`. Read end-to-end before deciding the cut.
2. **Extract to sibling** `routes_forge_<topic>.rs` in the same directory (`crates/argo-cli/src/handlers/gateway/`).
3. **Re-export via `pub(crate) use`** (or `pub use` for public API). The dispatcher in `routes_dispatch_*.rs` must continue to resolve forge handlers identically.
4. **Keep public API unchanged** — every existing import must resolve identically.
5. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green (skip pre-existing flakes per HARD RULE 2).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_forge.rs` (must drop below 400 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_forge_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/`

## Acceptance criteria

- [ ] `routes_forge.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green (modulo HARD RULE 2 known flakes)
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-routes_forge-split`
- PR title: `argo-cli: split routes_forge.rs below 500-LOC ceiling (claude-cli-routes_forge-split)`
