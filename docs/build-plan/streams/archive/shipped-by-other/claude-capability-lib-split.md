# claude-capability-lib-split — split lib.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main on 2026-05-09 (conductor-authored bulk-replenishment after observing lib.rs at 474/500 LOC).

## Why

`crates/argo-capability/src/lib.rs` is at 474 LOC on current main — 26 LOC of headroom. Pure refactor; mirrors proven pattern.

## Approach

1. Read `lib.rs` end-to-end. Identify a cohesive section.
2. Move to a sibling file `lib_<topic>.rs`.
3. Re-export via `pub(crate) use`.

Target: ≤ 350 LOC.

## Files you may touch

- `crates/argo-capability/src/lib.rs` — extract sibling, re-export
- `crates/argo-capability/src/lib_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-capability/src/lib.rs` — narrowly: 1 `mod` line if needed

## Files you must NOT touch

- Other crate modules
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `lib.rs` ≤ 350 LOC
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-capability --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-capability` clean

## Branch + PR

- Branch: `claude-capability-lib-split`
- PR title: `capability: split lib.rs below 500-LOC ceiling (claude-capability-lib-split)`

## Unblocks

Future briefs that extend `lib.rs` without hitting the ceiling.
