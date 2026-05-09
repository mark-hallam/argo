# claude-memory-store-split — split store.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 6278292d on 2026-05-09 (conductor-authored bulk-replenishment after observing store.rs at 497/500 LOC).

## Why

`crates/argo-memory/src/store.rs` is at 497 LOC on current main — 3 LOC of headroom against the 500-LOC HARD RULE 6 ceiling. Future extensions hit the wall. Pure refactor; mirrors proven pattern (#932, #956, #1087, #1088, #1097, #1102).

## Approach

1. Read `store.rs` end-to-end. Identify a cohesive section (helpers, sub-types, a sub-concern that's mostly orthogonal to the file's main responsibility).
2. Move to `crates/argo-memory/src/store_<topic>.rs` (NEW sibling).
3. Re-export via `pub(crate) use` so call sites are unchanged.

Target: `store.rs` ≤ 350 LOC (≥ 150 LOC headroom).

## Files you may touch

- `crates/argo-memory/src/store.rs` — narrowly: extract sibling, re-export
- `crates/argo-memory/src/store_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-memory/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Files you must NOT touch

- Other modules in the crate — refactor only
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `store.rs` ≤ 350 LOC (≥ 150 LOC headroom)
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-memory --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-memory` clean (no behaviour change)
- [ ] No public API changes; existing call sites unchanged

## Branch + PR

- Branch: `claude-memory-store-split`
- PR title: `memory: split store.rs below 500-LOC ceiling (claude-memory-store-split)`

## Notes for the implementer

- Pattern-match on prior splits — `crates/argo-agent-runner/src/turn.rs` + `turn_helpers.rs` (post-#1097), `crates/argo-tui/src/app.rs` + `app_navigation.rs` (post-#1102) for clean templates.
- Pick a cohesive sub-concern, not a random LOC-count cut.

## Unblocks

Future briefs that extend `store.rs` without hitting the 500-LOC ceiling.
