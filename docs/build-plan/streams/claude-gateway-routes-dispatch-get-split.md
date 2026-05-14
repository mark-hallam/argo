# claude-gateway-routes-dispatch-get-split — split routes_dispatch_get.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `7447d8d8` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-gateway-routes-dispatch-get-split`.

## Scope

Pure refactor split for `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` (521 LOC, OVER ceiling). High-traffic dispatch file — currently blocks any new GET route from being wired (claude-error-rate-dashboard-v2 was BLOCKED on adding `GET /api/v1/dashboards/error-rate`).

Mechanical split — group routes by domain:

1. Identify clusters of related routes (e.g. worker_* / approval_* / audit_* / skill_*).
2. Move each cluster to a sibling dispatcher file (e.g. `routes_dispatch_get_workers.rs`, `routes_dispatch_get_approvals.rs`).
3. The original `routes_dispatch_get.rs` becomes a thin re-export + top-level routing dispatcher.
4. Add the new `mod` declarations to `gateway/mod.rs`.

Public surface unchanged; each route still reaches the same handler.

**Not in scope**: changing any route's handler body; renaming routes; introducing new routes.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` (extract content).
- NEW sibling files `routes_dispatch_get_<domain>.rs`.
- `crates/argo-cli/src/handlers/gateway/mod.rs` (mod declarations).

## Files you must NOT touch

- Any route handler module (the individual `routes_*.rs` files).
- Any caller; the dispatcher's public surface stays.

## Acceptance criteria

- [ ] routes_dispatch_get.rs ends under 200 LOC (just imports + top-level dispatcher).
- [ ] Each sibling file under 400 LOC.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli` passes.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-gateway-routes-dispatch-get-split`
- PR title: `refactor: split routes_dispatch_get.rs below 500-LOC ceiling (claude-gateway-routes-dispatch-get-split)`

## Notes for the implementer

- ~30 min mechanical refactor.
- Pure shape change; no behavior change.
- Unblocks claude-error-rate-dashboard-v2 and any future GET-route briefs.
