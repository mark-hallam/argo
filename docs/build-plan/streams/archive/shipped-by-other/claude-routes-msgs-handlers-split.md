# claude-routes-msgs-handlers-split — split routes_msgs.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main on 2026-05-09 (conductor-authored bulk-replenishment after observing routes_msgs.rs at 475/500 LOC).

## Why

`crates/argo-cli/src/handlers/gateway/routes_msgs.rs` is at 475 LOC on current main — 25 LOC of headroom. Pure refactor; mirrors proven pattern.

## Approach

1. Read `routes_msgs.rs` end-to-end. Identify a cohesive section.
2. Move to a sibling file `routes_msgs_<topic>.rs`.
3. Re-export via `pub(crate) use`.

Target: ≤ 350 LOC.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_msgs.rs` — extract sibling, re-export
- `crates/argo-cli/src/handlers/gateway/routes_msgs_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/lib.rs` — narrowly: 1 `mod` line if needed

## Files you must NOT touch

- Other crate modules
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `routes_msgs.rs` ≤ 350 LOC
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` clean

## Branch + PR

- Branch: `claude-routes-msgs-handlers-split`
- PR title: `cli: split routes_msgs.rs below 500-LOC ceiling (claude-routes-msgs-handlers-split)`

## Unblocks

Future briefs that extend `routes_msgs.rs` without hitting the ceiling.
