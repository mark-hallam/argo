# claude-cli-routes_orchestrator-split — split argo-cli/src/handlers/gateway/routes_orchestrator.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main a7e516d4 on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/gateway/routes_orchestrator.rs` is at 412/500 LOC. Pre-emptive split keeps headroom (mirror of routes_top, routes_meta, routes_audit, routes_forge, routes_session_skills, routes_lists, routes_messages_rollups pattern).

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by orchestrator sub-resource (e.g. routing-rule CRUD vs handoff actions).
2. **Extract to sibling** `routes_orchestrator_<topic>.rs`.
3. **Re-export via `pub(crate) use`**.
4. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_orchestrator.rs` (must drop below 400 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_orchestrator_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/`

## Acceptance criteria

- [ ] `routes_orchestrator.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-routes_orchestrator-split`
- PR title: `argo-cli: split routes_orchestrator.rs below 500-LOC ceiling (claude-cli-routes_orchestrator-split)`
