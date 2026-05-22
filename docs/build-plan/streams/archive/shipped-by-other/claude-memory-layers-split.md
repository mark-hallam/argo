# claude-memory-layers-split — split layers.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 2bb5464d on 2026-05-10.

## Why

`crates/argo-memory/src/layers.rs` is at 467 LOC. Future memory-layer extensions push this over 500. Pure refactor mirrors proven pattern (#1121 memory-store split set the precedent in this crate).

## Approach

1. Read `layers.rs` end-to-end. Identify a cohesive group (likely a per-layer family — short_term vs. long_term vs. shared).
2. Move to `crates/argo-memory/src/layers_<topic>.rs`.
3. Re-export via `pub(crate) use`.

Target: `layers.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-memory/src/layers.rs` — extract sibling
- `crates/argo-memory/src/layers_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-memory/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Acceptance criteria

- [ ] `layers.rs` ≤ 350 LOC
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-memory --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-memory` clean
- [ ] No public API changes

## Branch + PR

- Branch: `claude-memory-layers-split`
- PR title: `memory: split layers.rs below 500-LOC ceiling (claude-memory-layers-split)`
