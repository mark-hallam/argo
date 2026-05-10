# claude-cli-routes_session_skills-split — split argo-cli/src/handlers/gateway/routes_session_skills.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 33fa37a2 on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/gateway/routes_session_skills.rs` is at 454/500 LOC. Pre-emptive split keeps headroom for future feature briefs (routes_top, routes_meta, routes_audit pattern).

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by HTTP verb (GET vs POST handlers) or by sub-resource (sessions vs skills). Read the file end-to-end before deciding the cut.
2. **Extract to sibling** `routes_session_skills_<topic>.rs` in the same directory.
3. **Re-export via `pub(crate) use`** so dispatcher import surface stays unchanged.
4. **Keep public API unchanged.**
5. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_session_skills.rs` (must drop below 400 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_session_skills_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/`

## Acceptance criteria

- [ ] `routes_session_skills.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-routes_session_skills-split`
- PR title: `argo-cli: split routes_session_skills.rs below 500-LOC ceiling (claude-cli-routes_session_skills-split)`
