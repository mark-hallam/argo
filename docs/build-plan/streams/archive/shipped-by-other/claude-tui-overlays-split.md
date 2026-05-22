# claude-tui-overlays-split — split overlays.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 6278292d on 2026-05-09 (conductor-authored bulk-replenishment after observing overlays.rs at 478/500 LOC).

## Why

`crates/argo-tui/src/overlays.rs` is at 478 LOC on current main — 22 LOC of headroom against the 500-LOC HARD RULE 6 ceiling. Future extensions hit the wall. Pure refactor; mirrors proven pattern (#932, #956, #1087, #1088, #1097, #1102).

## Approach

1. Read `overlays.rs` end-to-end. Identify a cohesive section (helpers, sub-types, a sub-concern that's mostly orthogonal to the file's main responsibility).
2. Move to `crates/argo-tui/src/overlays_<topic>.rs` (NEW sibling).
3. Re-export via `pub(crate) use` so call sites are unchanged.

Target: `overlays.rs` ≤ 350 LOC (≥ 150 LOC headroom).

## Files you may touch

- `crates/argo-tui/src/overlays.rs` — narrowly: extract sibling, re-export
- `crates/argo-tui/src/overlays_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-tui/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Files you must NOT touch

- Other modules in the crate — refactor only
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `overlays.rs` ≤ 350 LOC (≥ 150 LOC headroom)
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-tui --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tui` clean (no behaviour change)
- [ ] No public API changes; existing call sites unchanged

## Branch + PR

- Branch: `claude-tui-overlays-split`
- PR title: `tui: split overlays.rs below 500-LOC ceiling (claude-tui-overlays-split)`

## Notes for the implementer

- Pattern-match on prior splits — `crates/argo-agent-runner/src/turn.rs` + `turn_helpers.rs` (post-#1097), `crates/argo-tui/src/app.rs` + `app_navigation.rs` (post-#1102) for clean templates.
- Pick a cohesive sub-concern, not a random LOC-count cut.

## Unblocks

Future briefs that extend `overlays.rs` without hitting the 500-LOC ceiling.
