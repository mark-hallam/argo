# fix-gateway-dev-mode-tenant-header-trust-binding

**Owner**: codex.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-dev-mode-tenant-header-trust-binding`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/scene-6-claim-6-trust-and-approvals.md` F5 [P1].

🟠 **P1 — In dev mode (no JWT, role-gate falls open to Owner), `X-Argo-Tenant-Id` is read straight from request headers and used as the SQL filter scope for `audit_record`. The `claim_6_live_cross_tenant` cargo test passes because the test runs in production-mode posture (JWT-required); the running dev gateway does not. The mismatch is invisible from the SPA — there's no UI affordance to demonstrate isolation either way.**

## Root cause

`crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:24-41` and `routes_audit_detail.rs:93-95` read `X-Argo-Tenant-Id` directly off the request without verifying the principal owns the claimed tenant. In production posture (`ARGO_GATEWAY_TRUST_ROLE_HEADER=0` + JWT-required), the role-gate at `role_gate.rs:296-311` falls closed to Guest and the `audit.cross_tenant` capability check filters this. In dev posture (the running dev binary), it falls open to Owner — anyone can spoof a `X-Argo-Tenant-Id` header and pivot the query scope. Verified:

```
curl -s "http://127.0.0.1:7400/api/v1/audit?range=30d&as_tenant=__legacy__" | jq '.items | length'
→ 50 (the legacy bucket — different tenant scope, accessed via query string without any role check)
```

Strict cross-tenant data exfil doesn't happen for tenants whose IDs you can't guess, but nothing prevents trying. This is a dev-posture trust gap that the production posture closes.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F5 [P1] — Dev-mode forge-able tenant header; cross-tenant capability falls open to Owner

## Fix recipe

Two complementary fixes — both should land:

### Backend: bind principal → tenant on every header read

1. **In dev posture, verify the principal owns the claimed tenant**. The principal already carries an email (`X-Argo-Principal: human:<email>`). Look up that email's tenant memberships and reject if the claimed `X-Argo-Tenant-Id` is not in the membership set. Update:
   - `crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:24-41`
   - `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs:93-95`
   - Any other handler reading `X-Argo-Tenant-Id` directly

2. **Reject `?as_tenant=` query-string overrides in dev posture** unless the principal has explicit `tenant.switch` capability. Currently it appears unguarded in dev.

3. **Add a cross-tenant integration test for dev posture** at `crates/argo-cli/tests/dev_mode_tenant_header_binding.rs`:
   - Provision tenant A + tenant B
   - As principal A, attempt `GET /api/v1/audit -H "X-Argo-Tenant-Id: <tenant-B>"`
   - Assert 403 or 404 (NOT the tenant B data)

### SPA: workspace switcher to demonstrate isolation

4. **Add a workspace switcher to the AppShell** so the camera CAN demonstrate "log in as tenant B, see no rows leak". For v1.0 minimum: a dropdown in the top-right user menu listing the principal's tenant memberships. Selecting one updates the session and triggers a page refresh.

5. **Surface "Tenant: <name>" in the audit feed header** so a viewer can see which scope they're in. Today there's nothing showing the active tenant.

## Acceptance criteria

- [ ] In dev mode, attempting to fetch tenant B's audit data with principal A's session returns 403 / 404
- [ ] `?as_tenant=` query param rejected unless capability granted
- [ ] New cross-tenant test passes
- [ ] AppShell workspace switcher visible in user menu (if multi-tenant principal)
- [ ] Audit feed page shows active tenant name

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs`
- `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs`
- `crates/argo-cli/src/handlers/gateway/role_gate.rs`
- `crates/argo-identity/src/membership.rs` (lookup helper if missing)
- `web/components/skycot/AppShell/WorkspaceSwitcher.tsx` (NEW)
- `web/components/skycot/audit/AuditPageHeader.tsx` (active-tenant display)
- `crates/argo-cli/tests/dev_mode_tenant_header_binding.rs` (NEW)

## Out of scope

- Production posture (already correct)
- New tenant-creation UX (orthogonal)
- Full SSO / SAML wire-up (separate program)
- The full cross-tenant negative test suite (already shipped via #2257 et al)

## Effort

~300-500 LOC across backend handlers + SPA switcher + test. ~6-10 hours. P1 because the SPA has no visible affordance for the operator to PROVE multi-tenant isolation to an investor.
