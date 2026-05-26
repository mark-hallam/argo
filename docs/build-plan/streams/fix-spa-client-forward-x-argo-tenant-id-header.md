# fix-spa-client-forward-x-argo-tenant-id-header

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-client-forward-x-argo-tenant-id-header`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/scene-6-claim-6-trust-and-approvals.md` F11 [P3].

🟢 **P3 — `web/lib/api/client.ts:155-177` never sends an `x-argo-tenant-id` header. The gateway derives the tenant from the bearer-token / session-JWT principal via `extract_tenant(req)` — which is correct for single-workspace-per-session. But if the SPA ever adds a workspace switcher (multi-tenant identity per user), the client will need to forward an explicit `x-argo-tenant-id`.**

## Root cause

`web/lib/api/client.ts:155-177` (verify line range) constructs request headers and includes `Content-Type` + `Authorization` but does NOT include `x-argo-tenant-id`. The gateway-side canonical header for tenant disambiguation is `x-argo-tenant-id` (per `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs:192-193` and the matching replay route).

Today the gateway extracts tenant from the principal in the auth path — no SPA-side header needed. But this couples tenant identity to session identity. The locked product direction includes a workspace-switcher (operator can be member of multiple tenants); when that lands, the SPA needs to forward an explicit tenant header so the user-level identity stays stable while the workspace-level scope changes.

This is forward-looking — NOT a current bug. The audit caught it as a fragile coupling worth flagging before the workspace-switcher track starts.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F11 [P3] — Gateway tenant isolation IS enforced on `x-argo-tenant-id`; SPA never sends it

## Fix recipe

1. **Add a tenant-id resolver helper** at `web/lib/api/tenant.ts` (NEW). Reads from a session-scoped state (Zustand or React context). Today, returns the principal's tenant id; once a workspace switcher exists, returns the currently-selected workspace id.
2. **Extend `web/lib/api/client.ts:155-177`** to read the resolver and inject `x-argo-tenant-id: <id>` on every API call.
3. **Gateway-side test** at `crates/argo-cli/tests/gateway_tenant_header_explicit.rs`: confirm the gateway honours `x-argo-tenant-id` when supplied AND falls back to principal-derived tenant when not. Cross-tenant negative pin: a header pointing at a tenant the principal does NOT have access to MUST 403, not silently downgrade.
4. **Document the header contract** in the audit-trust runbook + the multi-workspace track design doc.

## Acceptance criteria

- [ ] Every SPA → gateway request includes `x-argo-tenant-id`.
- [ ] The header value equals the current workspace selection (or, in single-workspace mode, the principal's tenant).
- [ ] Gateway-side test asserts: header supplied + valid → respected; header supplied + cross-tenant → 403; no header → fall back to principal-derived tenant.
- [ ] No regression on single-workspace flows (today's `claim_6_live_cross_tenant` test still passes).
- [ ] Documentation captures the contract for the workspace-switcher track.

## May-touch

- `web/lib/api/tenant.ts` (NEW)
- `web/lib/api/client.ts:155-177` (header injection)
- `crates/argo-cli/tests/gateway_tenant_header_explicit.rs` (NEW)
- `docs/skycot/architecture/tenant-resolution.md` (verify path; document the contract)

## Out of scope

- Workspace-switcher UI (separate track; this brief is foundation only)
- Per-tenant cross-tenant audit row enforcement (already pinned via #2257 and `claim_6_live_cross_tenant`)
- Session-token race conditions (covered by `fix-worker-detail-session-token-race-not-found`)

## Effort

~80-150 LOC + 1 gateway test + 1 doc update. ~3-4 hours. P3 because it's preventive — not blocking the v1.0 demo, but cheap insurance for the multi-workspace track.
