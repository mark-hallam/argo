# fix-audit-cross-tenant-cap-bypass-mode-behavior

**Owner**: codex.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-cross-tenant-cap-bypass-mode-behavior`.
**Source**: e2e-ux-audit (scene-6-claim-6-trust-and-approvals) finding F7 — `docs/skycot/ux-audits/2026-05-26-0820/scene-6-claim-6-trust-and-approvals.md`.

🟡 **P1 — Cross-tenant audit override path returns rows without enforcing the `AuditCrossTenant` capability check in dev auth-bypass mode. Either the cap auto-grants in bypass (acceptable IF documented) or the check is missing (security defect). Needs operator decision + a banner so operators don't mistake bypass behavior for production behavior.**

## Root cause

`GET /api/v1/audit?as_tenant=<other>` with header `X-Argo-Audit-Cross-Tenant: 1` is supposed to require the `Action::AuditCrossTenant` capability per the comment at `crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:28-34`.

In the dev gateway with auth-bypass enabled, the cross-tenant override succeeds and returns rows from the target tenant. The `cap_granted` check appears to evaluate `true` for the bypass principal `system:dev-bypass`.

Audit reproduction:
```
curl -H "x-argo-tenant-id: tenant_a" -H "X-Argo-Audit-Cross-Tenant: 1" \
     "http://127.0.0.1:7400/api/v1/audit?as_tenant=tenant_b"
→ HTTP 200 with tenant_b rows  (cross-tenant succeeded)

Same call without the X-Argo-Audit-Cross-Tenant header
→ empty array  (correct — no leak without the override header)
```

Two possible root causes:
1. **Intentional**: auth-bypass mode is supposed to grant all caps to system:dev-bypass — fine for dev, but undocumented and easy to mistake for production behavior. Risk: any preview build that accidentally enables bypass will leak cross-tenant data.
2. **Bug**: the cap check is broken — the bypass principal shouldn't auto-grant AuditCrossTenant, and the override should fall back to the workspace tenant (or 403).

Either way, the fix has three parts:
- (a) Operator decides: intentional auto-grant, or accidental bug?
- (b) If intentional: add a prominent SPA banner when auth-bypass is on so operators visually see "auth-bypass mode · cross-tenant override is auto-granted"
- (c) If accidental: enforce the cap check uniformly; bypass principals get no cross-tenant override unless explicitly seeded with the cap

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F7 [P1] — Cross-tenant audit override returns rows without capability enforcement in auth-bypass dev mode

## Fix recipe

**Step 1 — Operator decision** (before any code):
Ask the operator: in auth-bypass mode, should `system:dev-bypass` auto-grant `AuditCrossTenant` (and all other caps), or should it be limited to the explicit cap set in the bypass principal's profile?

**Per operator's locked decisions** (CLAUDE.md core-platform invariants):
- "Multi-tenant by construction (tenant_id plumbed through every primitive)"
- "Capability-based security with first-class RBAC"

Bypass mode is a dev convenience, not a real principal. The strict reading: bypass should NOT auto-grant cross-tenant caps. The pragmatic reading: bypass is for local dev; auto-granting all caps is fine IF a banner is visible.

Suggested resolution: **Auto-grant in bypass IS intentional, but it MUST be banner-visible** so dev operators never mistake the behavior for prod. Document the intent at `crates/argo-cli/src/handlers/gateway/auth.rs` (or wherever bypass principal is defined).

**Step 2 — Add visible SPA banner** when auth-bypass is on:
1. Add a top-of-page red/orange banner component at `web/components/skycot/AuthBypassBanner.tsx`:
   ```
   ⚠ AUTH-BYPASS MODE · Cross-tenant capability auto-granted · Do NOT use in production
   ```
2. Render it at the top of every page when `process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"`.
3. Match the existing mock-provider banner pattern (`fix-mock-provider-persistent-banner-in-app-shell` already exists for parallel work — coordinate visual register).

**Step 3 — Document the intent** in code:
1. Add a comment at `crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:24-43`:
   ```rust
   // SECURITY NOTE: in ARGO_GATEWAY_AUTH_BYPASS mode, the system:dev-bypass
   // principal auto-grants AuditCrossTenant + all other caps. This is
   // intentional for local dev. The SPA renders an AUTH-BYPASS banner so
   // operators see this. NEVER enable bypass in prod or preview builds.
   ```
2. Add a startup guard: if `ARGO_GATEWAY_AUTH_BYPASS=1` AND `ARGO_ENV=production`, the gateway refuses to start with a fatal error. Belt-and-braces.

**Step 4 — Integration test**:
1. New test at `crates/argo-cli/tests/auth_bypass_blocked_in_production.rs` — start the gateway with `ARGO_ENV=production` + `ARGO_GATEWAY_AUTH_BYPASS=1` → assert startup failure with a clear error message.
2. New test at `crates/argo-cli/tests/audit_cross_tenant_in_bypass.rs` — start the gateway in bypass mode, exercise the cross-tenant override, assert it succeeds AND the response includes a header `X-Argo-Bypass-Mode: 1` (so clients can render their banner without env-var sniffing).

## Acceptance criteria

- [ ] `web/components/skycot/AuthBypassBanner.tsx` exists; renders at the top of every page when bypass is on.
- [ ] Banner copy clearly states bypass + cross-tenant auto-grant + "not for production".
- [ ] Gateway emits `X-Argo-Bypass-Mode: 1` response header on every response when bypass is enabled.
- [ ] Code comment at `routes_audit_feed_scope.rs:24-43` documents the intentional auto-grant.
- [ ] Startup guard: `ARGO_GATEWAY_AUTH_BYPASS=1` + `ARGO_ENV=production` → gateway refuses to start.
- [ ] Integration test for production-bypass-refused passes.
- [ ] Integration test for bypass cross-tenant + header passes.
- [ ] SPA banner visible during local dev (operator verifies visually).

## May-touch

- `web/components/skycot/AuthBypassBanner.tsx` (NEW)
- `web/app/(app)/layout.tsx` (mount the banner)
- `crates/argo-cli/src/handlers/gateway/auth.rs` (bypass principal definition + comment)
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs` (SECURITY NOTE comment)
- `crates/argo-cli/src/main.rs` or wherever startup checks live (production guard)
- `crates/argo-cli/tests/auth_bypass_blocked_in_production.rs` (NEW)
- `crates/argo-cli/tests/audit_cross_tenant_in_bypass.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/response_middleware.rs` (X-Argo-Bypass-Mode header — if a middleware layer exists)

## Out of scope

- Cross-tenant audit list-path leak in real prod (separate brief: `fix-audit-list-row-in-tenant-scope-legacy-leak` already exists — covers the legacy-row bucket leak which is different from the bypass-mode behavior here)
- Approvals cross-tenant verification (separate brief: `fix-approvals-cross-tenant-read-path-verification` already exists)
- RBAC enforcement broader audit (Phase 2)

## Effort

~80-150 LOC across SPA banner + gateway middleware + 2 tests. ~2-3 hours. Document operator decision in the PR body.
