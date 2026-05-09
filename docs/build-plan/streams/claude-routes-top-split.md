# claude-routes-top-split — split routes_top.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 6a753bae on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/gateway/routes_top.rs` is at 472 LOC. Future top-route additions (auth/permissions surface) push this over 500. Preemptive split mirrors proven pattern (#1122 routes_msgs, #1123 routes_post_writes, #1129 routes_voice, #1137 routes_meta).

## Approach

1. Read `routes_top.rs` end-to-end. Identify a cohesive section (likely a sub-domain group).
2. Move to `crates/argo-cli/src/handlers/gateway/routes_top_<topic>.rs` (mirror existing `routes_*_<topic>.rs` siblings).
3. Re-export via `pub(crate) use` if needed.

Target: `routes_top.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_top.rs` — narrowly: extract sibling
- `crates/argo-cli/src/handlers/gateway/routes_top_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 1 `mod` declaration if needed

## Acceptance criteria

- [ ] `routes_top.rs` ≤ 350 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` clean (boundary note OK for pre-existing flakes outside may-touch per HARD RULE 2)
- [ ] No public route-path changes

## Branch + PR

- Branch: `claude-routes-top-split`
- PR title: `cli: split routes_top.rs below 500-LOC ceiling (claude-routes-top-split)`
