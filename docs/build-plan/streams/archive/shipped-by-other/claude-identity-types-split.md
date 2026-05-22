# claude-identity-types-split — split types.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 6278292d on 2026-05-09 (conductor-authored bulk-replenishment after observing types.rs at 492/500 LOC).

## Why

`crates/argo-identity/src/types.rs` is at 492 LOC on current main — 8 LOC of headroom against the 500-LOC HARD RULE 6 ceiling. Future extensions hit the wall. Pure refactor; mirrors proven pattern (#932, #956, #1087, #1088, #1097, #1102).

## Approach

1. Read `types.rs` end-to-end. Identify a cohesive section (helpers, sub-types, a sub-concern that's mostly orthogonal to the file's main responsibility).
2. Move to `crates/argo-identity/src/types_<topic>.rs` (NEW sibling).
3. Re-export via `pub(crate) use` so call sites are unchanged.

Target: `types.rs` ≤ 350 LOC (≥ 150 LOC headroom).

## Files you may touch

- `crates/argo-identity/src/types.rs` — narrowly: extract sibling, re-export
- `crates/argo-identity/src/types_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-identity/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Files you must NOT touch

- Other modules in the crate — refactor only
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `types.rs` ≤ 350 LOC (≥ 150 LOC headroom)
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-identity --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-identity` clean (no behaviour change)
- [ ] No public API changes; existing call sites unchanged

## Branch + PR

- Branch: `claude-identity-types-split`
- PR title: `identity: split types.rs below 500-LOC ceiling (claude-identity-types-split)`

## Notes for the implementer

- Pattern-match on prior splits — `crates/argo-agent-runner/src/turn.rs` + `turn_helpers.rs` (post-#1097), `crates/argo-tui/src/app.rs` + `app_navigation.rs` (post-#1102) for clean templates.
- Pick a cohesive sub-concern, not a random LOC-count cut.

## Unblocks

Future briefs that extend `types.rs` without hitting the 500-LOC ceiling.
