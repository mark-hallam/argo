# claude-config-sections-split — split argo-config/src/sections.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 7157bae4 on 2026-05-10.

## Why

`crates/argo-config/src/sections.rs` is at 422/500 LOC. Pre-emptive split keeps headroom for future config-section additions.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by config section (e.g. routing vs auth vs limits) or by parser-vs-validator split.
2. **Extract to sibling** `sections_<topic>.rs`.
3. **Re-export via `pub use`** since sections is part of config's public API.
4. **Test.** `cargo clippy -p argo-config --all-targets -- -D warnings` clean. `cargo test -p argo-config` green.

## Files you may touch

- `crates/argo-config/src/sections.rs` (must drop below 400 LOC)
- `crates/argo-config/src/sections_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-config/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-config/`

## Acceptance criteria

- [ ] `sections.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-config --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-config` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-config-sections-split`
- PR title: `argo-config: split sections.rs below 500-LOC ceiling (claude-config-sections-split)`
