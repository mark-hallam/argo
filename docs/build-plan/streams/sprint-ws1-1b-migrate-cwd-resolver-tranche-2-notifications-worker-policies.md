# sprint-ws1-1b — migrate cwd-resolver tranche 2 (notifications, worker-policies)

**Owner**: claude or codex.
**Depends on**: WS1 keystone #2676 + WS1-2 #2682 (MERGED) — same proven pattern. Independent of tranche-1 (#different files).
**Authored against**: origin/main `4b6877494` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws1-1b-migrate-cwd-resolver-tranche-2-notifications-worker-policies`.

## Why (verified 2026-06-05)
Continues the WS1-1b long-tail. Of the 18 route files still on the old cwd `resolve_workspace_tenant(store)`
resolver (which reads tenant from cwd/store, NOT the user's session JWT → cross-tenant exposure on a public
launch), tranche-1 (billing/approvals/governance) is in flight; this tranche takes the next two user-data routes:
- `crates/argo-cli/src/handlers/gateway/routes_notifications.rs` — 5 old-resolver refs, 251 LOC. User notifications.
- `crates/argo-cli/src/handlers/gateway/routes_worker_policies.rs` — 3 old-resolver refs, 444 LOC. Per-tenant worker policy config.

## Task (follow the keystone pattern #2676 / #2682 — do NOT invent a new approach)
For each file:
1. Thread the request into the handler if not already available, and replace `resolve_workspace_tenant(store)`
   with `resolve_tenant_for_request(req, store)`. Remove the local duplicate `resolve_workspace_tenant` fn
   once its last caller is migrated (kill the parallel copy).
2. **Per-route check (avoid premise-false):** confirm the route serves USER-scoped data (→ JWT tenant is
   correct). If a route is genuinely admin/system and SHOULD stay cwd, do NOT migrate it — document why in
   the PR body and leave it. Grep the handler's data reads to confirm.
3. Preserve behaviour otherwise (same response shapes, same error handling).

## Verify
- A cross-tenant regression test per migrated route (mirror WS1-4 #2696 / `tests/tenant_resolve_from_session_jwt.rs`):
  tenant A's session reads ONLY A's data on the route, never B's, never cwd/demo; JWT wins over an
  `X-Argo-Tenant-Id: B` header on an A session. Must FAIL if the route regresses to cwd.
- `cargo test -p argo-cli -- --test-threads=1` GREEN (serial). clippy clean on touched files (boundary-note the
  pre-existing `await_holding_lock` in `routes_settings_credentials_tests.rs`, untouched).
- Keep each file under the 500-LOC ceiling (worker_policies is at 444 — if the migration would breach 500,
  extract a small helper module rather than inlining).

## Follow-up (NOT this brief — note in PR body)
Still on the old resolver after this: `routes_worker_activity.rs` (496 LOC) + `routes_cohorts.rs` (492 LOC) —
**both at the 500-LOC ceiling, need a split-prep first** = tranche 3. Plus duties/worker-skills/workspace-settings/
gtm-draft-followup/marketing-draft/onboarding-helpers = later tranches.

## PR
Title: `sprint(ws1-1b t2) — tenant — migrate notifications/worker-policies off cwd resolver to JWT`
