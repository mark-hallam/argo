# claude-routes-post-writes-split — split routes_post_writes.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 6278292d on 2026-05-09 (conductor-authored bulk-replenishment after observing routes_post_writes.rs at 476/500 LOC).

## Why

`crates/argo-cli/src/handlers/gateway/routes_post_writes.rs` is at 476 LOC on current main — 24 LOC of headroom against the 500-LOC HARD RULE 6 ceiling. Future extensions hit the wall. Pure refactor; mirrors proven pattern (#932, #956, #1087, #1088, #1097, #1102).

## Approach

1. Read `routes_post_writes.rs` end-to-end. Identify a cohesive section (helpers, sub-types, a sub-concern that's mostly orthogonal to the file's main responsibility).
2. Move to `crates/argo-cli/src/handlers/gateway/routes_post_writes_<topic>.rs` (NEW sibling).
3. Re-export via `pub(crate) use` so call sites are unchanged.

Target: `routes_post_writes.rs` ≤ 350 LOC (≥ 150 LOC headroom).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_post_writes.rs` — narrowly: extract sibling, re-export
- `crates/argo-cli/src/handlers/gateway/routes_post_writes_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Files you must NOT touch

- Other modules in the crate — refactor only
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `routes_post_writes.rs` ≤ 350 LOC (≥ 150 LOC headroom)
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` clean (no behaviour change)
- [ ] No public API changes; existing call sites unchanged

## Branch + PR

- Branch: `claude-routes-post-writes-split`
- PR title: `cli: split routes_post_writes.rs below 500-LOC ceiling (claude-routes-post-writes-split)`

## Notes for the implementer

- Pattern-match on prior splits — `crates/argo-agent-runner/src/turn.rs` + `turn_helpers.rs` (post-#1097), `crates/argo-tui/src/app.rs` + `app_navigation.rs` (post-#1102) for clean templates.
- Pick a cohesive sub-concern, not a random LOC-count cut.

## Unblocks

Future briefs that extend `routes_post_writes.rs` without hitting the 500-LOC ceiling.
