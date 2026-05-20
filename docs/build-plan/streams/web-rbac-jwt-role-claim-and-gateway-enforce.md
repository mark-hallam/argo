# web-rbac-jwt-role-claim-and-gateway-enforce — wire JWT role claim end-to-end + gateway per-action enforcement

**Owner**: claude (multi-crate; codex acceptable on Tier-A Rust portions).
**Authored against**: origin/main `a4751af2` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-rbac-jwt-role-claim-and-gateway-enforce`.
**Priority**: P0 — closes the SINGLE biggest unfixed gap from the 75-audit sweep. Wave-3 RBAC audit found 11/18 sensitive actions ungated; Wave-10 procurement audit said it's the deal-ender at item 4 of any security-review call.
**Sizing**: ~800-1400 LOC across `crates/argo-capability` + `crates/argo-cli/src/handlers/gateway/` + `crates/argo-identity` + `crates/argo-store/src/migrations/` + `web/auth.config.ts` + `web/lib/hooks/use-viewer-policy.ts` + tests. Multi-crate. Architectural.

## Why this brief exists

From the Wave-3 RBAC audit + Wave-10 procurement synthesizer:

> **Procurement-review verdict: FAIL.** A live demo where the seed
> guest user (`naomi@auditor.gov`) clicks Freeze on any worker
> succeeds. The 4-role model is marketing copy. Foundation
> (`argo-capability` engine + role defaults) is sound; the wiring
> from bearer token → principal-role → gateway capability check
> does not exist.
>
> 11 of 18 sensitive actions have **NO role gate at all** —
> including hire, pause, resume, archive, approvals decide/undo,
> all worker PATCHes, audit feed/detail, billing.
>
> 5 use a **client-trusted `X-Argo-Role` HTTP header** that any
> caller can self-assign: `curl -H "X-Argo-Role: owner"` bypasses
> every gate.
>
> The bearer-JWT carries no role claim — `auth.config.ts`
> deliberately omits it; `useViewerPolicy` notes this.

After-state (Wave-10 verdict): "the RBAC fix lands → weighted procurement score 5.9 → 7.4 — across the line."

## Pre-flight (run BEFORE claiming)

```bash
# Verify the audit's claims still hold on current main
rg "X-Argo-Role" crates/argo-cli/src/handlers/gateway/ | head -20    # should show the 5 client-trusted gates
rg "X-Argo-Audit-Cross-Tenant" crates/argo-cli/src/handlers/gateway/ # cross-tenant header bypass
grep -rn "role.*claim\|RoleClaim" web/auth.config.ts                 # confirm no role in JWT today
cat crates/argo-capability/src/lib.rs | head -50                     # capability engine + role enum
```

If any of those return zero results that should not be zero, the brief premise has drifted — re-validate against `docs/skycot/audit/rbac/2026-05-20-wave-3-rbac.md` before proceeding.

## Architecture

End-to-end role flow (build this from front-to-back):

```
1. Magic-link verify          → look up membership row → role enum
2. Mint JWT with `role` claim → __Host-session cookie
3. Gateway middleware decodes JWT → extracts role
4. Gateway handler reads role + matches against capability-map
5. 403 if role insufficient, 200 + proceed if role sufficient
6. Web SPA reads role from `/api/auth/me` (NOT from cookie directly)
   and gates client-side affordances via useViewerPolicy
```

Critical: client-side gates are advisory only. Gateway is the truth.

## Storage — additive migration

New columns on the `workspace_member` table (or whatever holds the principal↔tenant↔role triple — verify the canonical home with `rg "workspace_member\|membership" crates/argo-identity/src/`):

```sql
-- crates/argo-store/src/migrations/NNN__membership_role.sql
ALTER TABLE workspace_member ADD COLUMN role TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('owner','admin','user','guest'));
CREATE INDEX idx_workspace_member_tenant_role ON workspace_member(tenant_id, role);
```

Verify the migration slot is free via `scripts/check-migration-slots.sh`. Use the next free slot after the last committed.

## Step-by-step implementation

### Phase 1 — JWT role claim (web side)

1. Read `web/auth.config.ts`. Find the `jwt()` callback that mints the session JWT after magic-link verify.
2. Add `role` field to the JWT payload type + the cookie that's set. Default to `"user"` if no membership row found (graceful for orphaned auth).
3. Query the membership table from the auth callback. Source: `web/lib/auth/membership.ts` (create if doesn't exist) → calls `GET /api/v1/identity/membership?principal=<email>` on the gateway.
4. The JWT payload now looks like: `{ sub, email, tenant_id, role, exp, iat, jti }`.
5. Update `web/lib/auth/session-token.ts` (or equivalent) to add `role` to the `Session` type.

### Phase 2 — Gateway role decode

1. New file: `crates/argo-cli/src/handlers/gateway/role_from_jwt.rs` (~80 LOC).
   - Function: `pub fn role_from_session_cookie(req: &str) -> Option<UserTenantRole>`
   - Reads the `__Host-session` cookie (same pattern as `session_jwt.rs::session_cookie_from_request`)
   - Decodes JWT, validates signature + exp + jti-not-revoked
   - Returns `UserTenantRole` enum (from `argo-capability`) on success
2. In `crates/argo-cli/src/handlers/gateway/routes.rs` `dispatch_route_inner`: after the existing bearer-auth check, decode role and stash it in a request-scoped context (e.g. `RequestContext { tenant_id, principal_id, role }`).
3. **Critical migration**: every handler that currently reads `X-Argo-Role` from the header must switch to reading from the request context. Files to update (verify with `rg "X-Argo-Role" crates/argo-cli/src/handlers/gateway/`):
   - `routes_workflows.rs` — workflow.edit gate
   - `routes_workers_freeze.rs` — worker.freeze gate
   - `routes_workers_hard_delete.rs` — worker.delete gate
   - `routes_support_tickets.rs` — support.create gate
   - `routes_audit_cross_tenant.rs` — audit.cross_tenant gate
4. **Delete the `X-Argo-Role` header read** from each of those — that's the bypass.

### Phase 3 — Capability map for the 11 currently ungated actions

Open `crates/argo-capability/src/lib.rs`. Verify the action enum has these (add if missing):

```rust
pub enum Action {
    WorkerHire,          // currently: no gate
    WorkerPause,         // currently: no gate
    WorkerResume,        // currently: no gate
    WorkerArchive,       // currently: no gate
    WorkerAutonomyPatch, // currently: no gate
    WorkerChannelsPatch, // currently: no gate
    WorkerQuietHoursPatch, // currently: no gate
    WorkerBudgetPatch,   // currently: no gate
    ApprovalDecide,      // currently: no gate
    ApprovalUndo,        // currently: no gate
    AuditFeedRead,       // currently: no gate
    AuditDetailRead,     // currently: no gate
    SupportTicketCreate, // currently: client-trusted X-Argo-Role
    // ... (verify against the 18-action inventory in the wave-3 audit)
}
```

Per-action minimum role map (operator-decisional — adjust to match the wave-3 audit's recommended floor):

```rust
fn min_role_for_action(action: Action) -> UserTenantRole {
    match action {
        Action::WorkerHire => Owner,           // hiring is owner-only
        Action::WorkerPause | Action::WorkerResume => Admin,
        Action::WorkerArchive | Action::WorkerHardDelete => Owner,
        Action::WorkerAutonomyPatch | Action::WorkerBudgetPatch => Admin,
        Action::WorkerChannelsPatch | Action::WorkerQuietHoursPatch => Admin,
        Action::ApprovalDecide | Action::ApprovalUndo => User, // all members
        Action::AuditFeedRead | Action::AuditDetailRead => User,
        Action::AuditExport => Admin,
        Action::AuditCrossTenant => Owner, // requires explicit capability
        Action::SupportTicketCreate => User,
        Action::WorkflowEdit => Admin,
        Action::WorkflowArchive => Owner,
    }
}
```

### Phase 4 — Gate every action handler

For each of the 18 audited actions, the handler does:

```rust
pub(super) async fn handle_<action>(req: &str, deps: &AgentDeps) -> ResponseResult {
    let ctx = RequestContext::from(req)?;
    if ctx.role < min_role_for_action(Action::<X>) {
        return Ok(forbidden_response("insufficient role"));
    }
    // ... existing handler body
}
```

Files to touch (one per action; verify the full list against the wave-3 audit's 18-action inventory):
- `routes_workers_hire.rs` (`Action::WorkerHire`)
- `routes_workers_pause.rs` (`Action::WorkerPause`)
- `routes_workers_resume.rs` (`Action::WorkerResume`)
- `routes_workers_archive.rs` (`Action::WorkerArchive`)
- `routes_workers_settings_*.rs` (4 PATCHes)
- `routes_approvals_decide.rs` (`Action::ApprovalDecide`)
- `routes_approvals_undo.rs` (`Action::ApprovalUndo`)
- `routes_audit_list.rs` + `routes_audit_detail.rs`
- `routes_audit_export.rs`
- `routes_audit_cross_tenant.rs` (delete header-trust path)
- `routes_workflows.rs` (already gated; verify migration to context)

### Phase 5 — Web-side updates

1. `web/lib/hooks/use-viewer-policy.ts` — replace any `X-Argo-Role` header read with a fetch to `/api/auth/me` that returns `{ email, tenant_id, role }`.
2. New endpoint: `web/app/api/auth/me/route.ts` — decodes the `__Host-session` cookie, returns the principal info + role. Wires to gateway `/api/v1/auth/me` (new — sibling implementation).
3. Hide UI affordances based on role:
   - "Hire teammate" button (Approvals header + /team) → only show if `role >= Admin` (or Owner per the operator's call)
   - "Freeze" + "Pause" + "Archive" buttons → only show if `role >= Admin`
   - "Export audit" → only show if `role >= Admin`

## Tests — REQUIRED before merge

### Rust tests

For EACH of the 18 gated actions, add a negative test:

```rust
#[tokio::test]
async fn worker_hire_rejects_guest_role() {
    let deps = test_deps();
    let req = make_request_with_jwt_role("guest");
    let response = handle_worker_hire(&req, &deps).await.unwrap();
    assert_eq!(response.status, 403);
}

#[tokio::test]
async fn worker_hire_rejects_curl_xargorole_header_bypass() {
    let deps = test_deps();
    // JWT role = guest, but X-Argo-Role header = owner (the bypass)
    let req = make_request_with_jwt_role("guest").with_header("X-Argo-Role", "owner");
    let response = handle_worker_hire(&req, &deps).await.unwrap();
    assert_eq!(response.status, 403);
    // The whole point: header trust is gone.
}

#[tokio::test]
async fn worker_hire_accepts_owner_role() {
    let deps = test_deps();
    let req = make_request_with_jwt_role("owner");
    let response = handle_worker_hire(&req, &deps).await.unwrap();
    assert_eq!(response.status, 200);
}
```

Multiply by 18 actions × ~3 cases each = ~54 unit tests + 1 integration smoke that exercises the full chain (JWT mint → cookie → gateway → 403/200).

### Web tests

- `web/lib/auth/__tests__/role-from-session.test.ts` — JWT role-claim parsing
- `web/lib/hooks/__tests__/use-viewer-policy.test.tsx` — affordance hiding based on role
- E2E (Playwright): `web/tests/e2e/rbac-guest-cannot-freeze.spec.ts`

## may-touch

- `crates/argo-cli/src/handlers/gateway/` — every routes_*.rs file with a sensitive action (~18 files)
- New: `crates/argo-cli/src/handlers/gateway/role_from_jwt.rs`
- `crates/argo-capability/src/lib.rs` (Action enum + min_role_for_action map)
- `crates/argo-identity/src/membership.rs` (role column + getter)
- `crates/argo-store/src/migrations/NNN__membership_role.sql` (additive)
- `web/auth.config.ts` (JWT mint with role claim)
- New: `web/lib/auth/membership.ts`
- New: `web/app/api/auth/me/route.ts`
- `web/lib/hooks/use-viewer-policy.ts` (read from /api/auth/me)
- ~5 components that show role-gated affordances (TeammateCard, ApprovalsPageHeader, AuditExportButton, etc.)

## must-NOT-touch

- Existing migration files (additive-only)
- `argo-capability` engine internals (just extend the Action enum)
- `__Host-session` cookie format (only add a field)
- The `X-Argo-Role` header — REMOVE all reads but don't break anyone still sending it (the gateway should ignore the header, not throw)

## Acceptance criteria

- [ ] Additive migration adds `role` column to membership table
- [ ] Magic-link verify mints JWT with `role` claim sourced from membership table
- [ ] Gateway middleware decodes role from JWT cookie, stashes in request context
- [ ] All 18 sensitive actions check role from request context (no `X-Argo-Role` header read remaining)
- [ ] `curl -H "X-Argo-Role: owner" -b "__Host-session=<guest-jwt>" /api/v1/workers/<id>/freeze` returns 403
- [ ] Web SPA hides hire/freeze/pause/archive/export affordances for sub-admin roles
- [ ] Cross-tenant audit (`X-Argo-Audit-Cross-Tenant: 1`) requires explicit capability check from JWT, not header presence
- [ ] 54+ unit tests covering negative + positive cases per action
- [ ] 1 E2E smoke (guest user cannot freeze a worker)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `bash scripts/conductor-pre-merge-gate.sh` clean
- [ ] Wave-3 RBAC audit verdict moves FAIL → PASS on re-run

## Sizing notes

- Phase 1 (web JWT mint): ~150 LOC + 3 tests
- Phase 2 (gateway role decode): ~120 LOC + 4 tests
- Phase 3 (capability map): ~80 LOC + table-driven tests
- Phase 4 (gate 18 handlers): ~200 LOC across 18 files + 54 tests
- Phase 5 (web affordance hiding): ~150 LOC + 5 tests
- Additive migration: ~30 LOC
- Total: ~730 LOC prod + ~470 LOC tests = ~1200 LOC. Land in 1 PR if possible (the role flow is interdependent).

## Gotchas / known pitfalls

1. **The `X-Argo-Role` header used to be load-bearing in dev** (`ARGO_GATEWAY_AUTH_BYPASS=1` short-circuits gateway auth but the header was the only role signal). After this brief, the bypass path must mint a synthetic JWT with `role=owner` to preserve dev-mode UX.
2. **Test fixtures with bearer tokens but no JWT** — many existing tests use `Authorization: Bearer <api-key>` for service-to-service calls. Those skip the cookie path and need their own role-source (probably via service-account principal with a fixed role). Don't break them.
3. **Magic-link verify race** — first signup creates the membership row with `role=owner` (the workspace creator). The current handler in `routes_auth_verify.rs` may not insert the membership row; verify + extend.
4. **Existing `useViewerPolicy` callers** — about 8 files read it today. They'll all need to handle the case where `/api/auth/me` hasn't returned yet (loading state).
5. **CI flakes** — the JWT signing key is per-process random in tests; ensure the test helper exposes a deterministic key.

## References

- Audit: `docs/skycot/audit/rbac/2026-05-20-wave-3-rbac.md`
- Wave-10 procurement: `docs/skycot/audit/procurement-security/2026-05-21-wave-10-procurement.md`
- Existing pattern: `crates/argo-cli/src/handlers/gateway/session_jwt.rs` (cookie decode)
- Existing pattern: `crates/argo-capability/src/lib.rs` (capability engine)
- Existing pattern: `crates/argo-cli/src/handlers/gateway/middleware_session_verify.rs` (if PR #1867 is merged; otherwise this brief adds the analog)
