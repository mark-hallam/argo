# mvp-split-memory-handler — split handlers/memory.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 MVP — proven split pattern.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/memory.rs` is at **533 LOC**. Blocks memory-CLI extension briefs.

## Approach

1. Read `memory.rs` end-to-end. Identify a cohesive section (likely query/list vs. write/upsert vs. compose).
2. Move to `crates/argo-cli/src/handlers/memory_<topic>.rs`.
3. Re-export via `pub(crate) use`.

Target: `memory.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-cli/src/handlers/memory.rs`
- `crates/argo-cli/src/handlers/memory_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `mod` declaration

## Acceptance criteria

- [ ] `memory.rs` ≤ 350 LOC
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` clean
- [ ] No public CLI flag or response-shape changes

## Branch + PR

- Branch: `mvp-split-memory-handler`
- PR title: `cli: split handlers/memory.rs below 500-LOC ceiling (mvp-split-memory-handler)`
