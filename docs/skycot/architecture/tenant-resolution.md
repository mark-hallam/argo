# Tenant resolution: `X-Argo-Tenant-Id` header contract

**Authored against:** origin/main on 2026-05-27.
**Companion to:** [`20-multi-tenant-data-flow.md`](./20-multi-tenant-data-flow.md) (where `TenantId` flows once the gateway has resolved it).
**Origin:** brief `fix-spa-client-forward-x-argo-tenant-id-header`, closing scene-6 audit finding F11 [P3].

This doc fixes the contract between the SPA, the gateway, and the future workspace-switcher track so the three can be evolved independently without re-litigating the header semantics.

## Why this matters

Today the gateway derives the tenant from the bearer-token / session-JWT principal via `extract_tenant(req)` and `dev_bypass_principal()` (see `crates/argo-cli/src/handlers/gateway/routes_auth.rs`). That is correct for single-workspace-per-session.

When the workspace-switcher track lands — operators belong to multiple tenants and pick which one is "active" — the user-level identity must stay stable (same JWT, same principal) while the workspace-level scope flips. The clean separation is:

- The bearer token / session JWT identifies the **user**.
- The `X-Argo-Tenant-Id` header identifies the **currently-selected workspace**.

Without this separation, the only way to switch workspaces is to mint a new JWT per workspace — which leaks the user's tenant identity into long-lived auth material and breaks session continuity across switches.

## Gateway behaviour (the contract)

Implemented in `crates/argo-cli/src/handlers/gateway/routes_auth.rs`. Locked by structural pins in `crates/argo-cli/tests/gateway_tenant_header_explicit.rs`.

| `X-Argo-Tenant-Id` presence | Bound token (`service_token_binding` row) | Unbound token / dev-bypass |
| --- | --- | --- |
| absent | tenant = bound row's `tenant_id` | tenant = `workspace_default_tenant()` (the all-zero UUID) |
| present + UUID matches the bound tenant | tenant = bound row's `tenant_id` (header is honoured but redundant) | tenant = header value |
| present + UUID does NOT match the bound tenant | **403 `tenant-header mismatch`** (`enforce_header_match`) | tenant = header value (no bound row to disagree with) |
| present but not a valid UUID | **403 `tenant-header mismatch`** | tenant = `workspace_default_tenant()` (parse failure falls through) |

The 403 on cross-tenant mismatch is load-bearing: it surfaces workspace-switcher bugs loudly rather than silently corrupting reads/writes against the wrong workspace.

## SPA behaviour (the wiring)

Implemented in `web/lib/api/tenant.ts` + `web/lib/api/client.ts`. Locked by `web/lib/api/tenant.test.ts`.

```
┌────────────────────────────────────────────────┐
│ web/lib/api/tenant.ts                          │
│                                                │
│   resolveTenantId(peek)                        │
│     1. workspace-switcher override (in-mem)    │
│     2. peek.tenantId (from session JSON)       │
│     3. undefined → omit header                 │
│                                                │
│   setTenantOverride(id)   ← workspace-switcher │
│   clearTenantOverride()   ← sign-out hook      │
│   getTenantOverride()     ← inspect for UI     │
└────────────────────────────────────────────────┘
                    │
                    ▼
┌────────────────────────────────────────────────┐
│ web/lib/api/client.ts → request()              │
│                                                │
│   if (!headers.has("X-Argo-Tenant-Id")) {      │
│     const id = resolveTenantId(peek);          │
│     if (id) headers.set("X-Argo-Tenant-Id",id);│
│   }                                            │
│                                                │
│   ── caller-supplied header wins ──            │
└────────────────────────────────────────────────┘
```

### Precedence

1. **Explicit per-call header.** Anything the caller passes in `init.headers` is preserved verbatim. Used by test fixtures and any future "view-as-another-tenant" admin tooling.
2. **Workspace-switcher override.** A module-level mutable set by the workspace switcher when the operator picks a workspace. Lives for the page session; cleared on sign-out via `clearTenantOverride()`.
3. **Session payload.** `session.tenantId` if the magic-link JWT carries one. Today's JWT doesn't — the field is wired through so the workspace-switcher track can populate it without further client-side changes.
4. **Omit the header.** Gateway falls back to the principal-derived tenant — today's single-workspace behaviour.

## Backward-compatibility guarantees

- **Today's flows are unchanged.** With no override and no `session.tenantId`, the resolver returns `undefined` and the header is never stamped — every existing route exercises the same gateway code path as before this brief.
- **`claim_6_live_cross_tenant` still passes.** The cross-tenant negative pin in the live test suite (`crates/argo-cli/tests/live_claim_6_cross_tenant.rs`) keeps the bound-token cross-tenant guard exercised end-to-end.
- **Gateway never trusts the header alone.** When a `service_token_binding` row exists for the bearer, the bound tenant is the authority; the header can only confirm it (200) or contradict it (403). The header NEVER overrides a bound tenant.

## Workspace-switcher migration checklist

When the workspace-switcher track picks up:

1. Add `tenantId?: string` to the `Session` interface declaration in `web/auth.config.ts` so the type narrowing in `readSessionPeek` becomes load-bearing.
2. Populate `tenantId` in `readMagicLinkClaims` (`web/lib/auth/session-cookie.ts`) from a new `tnt` JWT claim issued by the gateway's magic-link verify route.
3. On workspace-pick in the switcher UI, call `setTenantOverride(workspaceTenantId)`. The next request stamps the header automatically.
4. On sign-out, call `clearTenantOverride()`.
5. Gateway: extend the bound-token model to allow a bearer to be valid across multiple tenants (or mint per-workspace bound tokens at switch time). Today a bound token pins exactly one tenant, so the 403 fires the moment the switcher picks a different workspace. Decide which side carries the multi-tenant relation.
6. Update this doc's "Gateway behaviour" table once step 5 lands.

## Related code

- Gateway: `crates/argo-cli/src/handlers/gateway/routes_auth.rs` (the contract).
- Gateway: `crates/argo-cli/src/handlers/gateway/tenant_header.rs` (shared header reader for tenant-bound audit/session routes).
- SPA: `web/lib/api/tenant.ts` + `web/lib/api/client.ts` (the wiring).
- Pins: `crates/argo-cli/tests/gateway_tenant_header_explicit.rs` (structural) + `web/lib/api/tenant.test.ts` (behavioural).
- Source audit: `docs/skycot/ux-audits/2026-05-27-0415/scene-6-claim-6-trust-and-approvals.md` F11.
