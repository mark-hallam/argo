# mission-gateway-session-export-tenant-bind

**Authored against:** `origin/main` 2af54930f on 2026-05-23 (conductor tick-110).
**Validate before claim:** `bash scripts/check-brief-freshness.sh mission-gateway-session-export-tenant-bind`
**Owner:** claude
**Priority:** 🔴 P0 — cross-tenant data leak in gateway routes
**Source:** PR #2130 finding (eval-argo-replay-after-1992-fix), §FINDINGS reported by claude worker.

## The problem

Two gateway routes load session data without binding `tenant_id`:

1. `crates/argo-cli/src/handlers/gateway/routes_session_get_export.rs`
   - calls `store.list_messages(session_id)` (no tenant filter)
   - calls `argo_audit::rows_for_session(audit_pool, session_id)` (no tenant filter)
2. `crates/argo-cli/src/handlers/gateway/routes_audit_session.rs`
   - calls `argo_audit::rows_for_session(audit_pool, session_id)` (no tenant filter)

Effect: tenant_b can `GET /api/v1/sessions/<tenant_a-session-id>/export` (or the audit equivalent) and receive tenant_a's messages + audit chain. Same shape as the round-3 audit-export pattern that shipped a bug earlier this month.

## What to do

1. In both routes, pre-flight: load the `Session` row, assert `session.tenant_id == principal.tenant_id`; if mismatch → 404 with the same body shape as the genuine-not-found case. **Do not 403** — 404 is the agreed contract (existence-leak avoidance).
2. Add cross-tenant negative test for each route in the relevant `tests/` dir:
   - tenant_a inserts session + messages + audit rows
   - tenant_b principal calls the route with tenant_a's `session_id`
   - assert HTTP 404 + zero rows in response body
3. Add a positive same-tenant baseline so the test fails loudly if the 404 path triggers on legit requests.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_session_get_export.rs`
- `crates/argo-cli/src/handlers/gateway/routes_audit_session.rs`
- `crates/argo-cli/tests/gateway/` (or sibling test home — find the convention by reading the dir)
- Possibly `crates/argo-store/src/sessions.rs` if a helper for tenant-scoped session-load doesn't exist yet (prefer extending an existing helper to introducing a new pub fn).

**Do NOT** add a new `pub fn list_messages_for_tenant` parallel to `list_messages` — if the existing helper needs an extra arg, add it additively (default arg pattern or new sibling on the same impl block, not a parallel free fn).

## Acceptance

- Both routes 404 cleanly on cross-tenant access; existing same-tenant export still works.
- 2 new cross-tenant negative tests pass; 2 new same-tenant baselines pass.
- Per-crate clippy clean.
- Workspace clippy clean.
- No new `pub struct` / `pub enum` introduced (additive args only).
- Existing PR #2130's `#[ignore]`'d FINDINGS-ONLY test in `crates/argo-replay/tests/...` flipped from `#[ignore]` to live and now passes (or stays ignored with a one-line note explaining why it's still simulating a different leak shape).

## Citations

- PR #2130 body §FINDINGS
- `crates/argo-cli/src/handlers/gateway/routes_session_get_export.rs`
- `crates/argo-cli/src/handlers/gateway/routes_audit_session.rs`

## LOC budget

~250-400 LOC (small additive change in 2 routes + 4 new tests).
