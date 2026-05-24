# mission-gateway-session-export-tenant-bind

**Owner**: codex
**Priority**: P0 - cross-tenant leak closure
**Authored against**: origin/main `7354f039d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-session-export-tenant-bind`.

## Source

PR #2130's replay audit found two gateway routes that load session messages or
audit chains by session UUID without tenant ownership binding:
`routes_session_get_export.rs` and `routes_audit_session.rs`.

## Goal

Bind both session export surfaces to the authenticated/request tenant. A tenant
must receive 404/empty denial semantics for another tenant's session UUID.

## May Touch

- `crates/argo-cli/src/handlers/gateway/routes_session_get.rs` (dispatcher only, to pass `req` into export handler)
- `crates/argo-cli/src/handlers/gateway/routes_session_get_export.rs`
- `crates/argo-cli/src/handlers/gateway/routes_audit_session.rs`
- `crates/argo-cli/src/handlers/gateway/routes_session_tenant_bind_tests.rs` or a new focused gateway test
- `crates/argo-store/src/**` only for additive tenant-aware helper methods needed by these routes
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not rewrite unrelated session routes.
- Do not change session export format except for denial of cross-tenant access.
- Do not add migrations unless an additive query helper proves impossible without one.

## Acceptance

- [ ] Same-tenant export still succeeds for session messages and audit chain.
- [ ] Cross-tenant session UUID returns not-found/denied semantics and no row data.
- [ ] Route code derives tenant from the existing gateway principal/header helper, not from caller-supplied unchecked input.
- [ ] Tests cover two cross-tenant negatives and two same-tenant baselines.
- [ ] Crate-scoped verification claimed in the PR body with any unrelated clean-main failures clearly separated.

## Notes

Keep this to the two named routes. Wider UUID-bound route clusters have separate
rows.

Conductor acceptance note 2026-05-24: on the merge base for #2192,
`routes_audit_session.rs` was already tenant-bound by a prior cluster fix, so the
remaining code change was the export handler plus the dispatcher request-threading
line in `routes_session_get.rs`. The PR also added audit-session same/cross-tenant
coverage to keep the original two-route acceptance pinned.
