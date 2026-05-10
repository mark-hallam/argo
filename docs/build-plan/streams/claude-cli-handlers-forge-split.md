# claude-cli-handlers-forge-split — split argo-cli/src/handlers/forge.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 3521821d on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/forge.rs` is at 439/500 LOC. Pre-emptive split keeps headroom for future feature briefs.

Note: distinct from `claude-cli-routes_forge-split` (#1187, MERGED) which targeted `gateway/routes_forge.rs` — this brief targets the top-level `handlers/forge.rs` CLI handler.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by ForgeCmd subcommand (e.g. plan vs run vs review) or by argument-parsing vs execution. Read end-to-end.
2. **Extract to sibling** `forge_<topic>.rs`.
3. **Re-export via `pub(crate) use`**.
4. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/src/handlers/forge.rs` (must drop below 400 LOC)
- `crates/argo-cli/src/handlers/forge_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/`

## Acceptance criteria

- [ ] `forge.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-handlers-forge-split`
- PR title: `argo-cli: split handlers/forge.rs below 500-LOC ceiling (claude-cli-handlers-forge-split)`
