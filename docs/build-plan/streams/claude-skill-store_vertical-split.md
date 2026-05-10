# claude-skill-store_vertical-split — split argo-skill/src/store_vertical.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 444da442 on 2026-05-10.

## Why

`crates/argo-skill/src/store_vertical.rs` is at 440/500 LOC. Pre-emptive split keeps headroom — this file is heavily extended (`#936` judge wiring, `#966` eval-runner integration, etc.).

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by `#[cfg(test)]` mod (already extracted as `store_vertical_judge_tests.rs` per #936; check current state) OR by propose vs review vs promote logic. Read end-to-end.
2. **Extract to sibling** `store_vertical_<topic>.rs`.
3. **Re-export via `pub use`** (this is part of argo-skill's public surface).
4. **Test.** `cargo clippy -p argo-skill --all-targets -- -D warnings` clean. `cargo test -p argo-skill` green.

## Files you may touch

- `crates/argo-skill/src/store_vertical.rs` (must drop below 400 LOC)
- `crates/argo-skill/src/store_vertical_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-skill/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-skill/`

## Acceptance criteria

- [ ] `store_vertical.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-skill --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-skill` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-skill-store_vertical-split`
- PR title: `argo-skill: split store_vertical.rs below 500-LOC ceiling (claude-skill-store_vertical-split)`
