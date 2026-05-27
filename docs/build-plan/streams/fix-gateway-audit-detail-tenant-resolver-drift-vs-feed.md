# fix-gateway-audit-detail-tenant-resolver-drift-vs-feed

**Owner:** codex
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-gateway-audit-detail-tenant-resolver-drift-vs-feed`
**Source:** UX e2e audit 2026-05-28-0252 (scene-1-and-2 F02 + scene-6 F2)

🔴 **P0 — Gateway audit-detail route resolves the tenant from a different header + with a different default than the audit-feed route. Every row the feed returns 200 on is 404 on the detail endpoint when called from the SPA — the Scene-6 trust beat fabricates placeholder provenance instead.**

## Root cause

The audit list and detail handlers do not agree on how to pull the caller's tenant.

- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs:443-446` resolves tenant via `rate_limit::extract_tenant(req)` → reads `?tenant=` query param, then `X-Tenant-Id` header, then `argo_config.tenant.default_id`, finally falls back to the literal `"anonymous"`.
- `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs:171-194` resolves tenant via `header_value(req, "x-argo-tenant-id")` (note the different header name) and falls back to `DEFAULT_AUDIT_TENANT_ID = "ws-default"`.

The SPA's `web/lib/api/client.ts:85-88` sends NO tenant header at all — so the list call resolves to `"anonymous"` and the detail call to `"ws-default"`, while every real seeded row lives under the demo tenant UUID `019e6785-654d-7302-9a29-ff6035517c53`. Net effect from the running SPA preview:

```
fetch('/api/v1/audit?limit=2')                                 → 200, real rows like 019e6a5facdd79f2a9a37ae87e22a6e4
fetch('/api/v1/audit/019e6a5facdd79f2a9a37ae87e22a6e4')        → 404 {"error":{"code":"not_found"}}
```

Once the detail returns 404, the SPA falls through to `placeholderAuditDetail()` and the camera reads `mem_synthetic`, `generic-skill 1.0.0`, `generic-policy 1.0`, "Trigger reconstructed from feed row" — fabricated content during the Scene-6 hash-chain narration.

## Findings this brief closes

- scene-1-and-2 F02 [P0] — Gateway `/api/v1/audit/{id}` returns 404 for IDs from its own list
- scene-6 F2 [P0] — List/detail tenant-id resolution drift; same SPA session list-200 vs detail-404

## Fix recipe

1. **Make audit-detail use the same tenant resolver as the feed.** Replace the `header_value(req, "x-argo-tenant-id")` call in `routes_audit_detail.rs:171-194` with `rate_limit::extract_tenant(req)` so both routes share the same query/header/config/default precedence chain.
2. **Keep the legacy `x-argo-tenant-id` header as one of the recognised aliases inside `extract_tenant`** (additive, not breaking) so any caller already setting it continues to work.
3. **Remove `DEFAULT_AUDIT_TENANT_ID = "ws-default"`** or align it with `argo_config.tenant.default_id` so the two routes can never diverge again.
4. **Add an integration test** at `crates/argo-cli/tests/audit_list_detail_tenant_parity.rs` that:
   - Seeds 1 row under tenant `T1` and 1 row under tenant `T2`.
   - With no tenant header set, asserts that the list returns the same set of rows that the detail can successfully resolve.
   - With `X-Tenant-Id: T1` set, asserts list-200 + detail-200 for the `T1` row and list-empty + detail-404 for the `T2` row (negative cross-tenant assertion).
5. **Document the resolver precedence** at the top of both handler files so future drift requires a comment change.

## Acceptance criteria

- [ ] `routes_audit_detail.rs` no longer calls `header_value(req, "x-argo-tenant-id")` directly.
- [ ] Both routes import the same `extract_tenant` helper.
- [ ] New integration test `audit_list_detail_tenant_parity` passes cold.
- [ ] In the running preview SPA, picking any audit row id returned by `GET /api/v1/audit?limit=N` and fetching `GET /api/v1/audit/<that-id>` returns 200, NOT 404.
- [ ] `cargo test -p argo-cli --test audit_list_detail_tenant_parity` green.
- [ ] PR body cites scene-1-and-2 F02 + scene-6 F2.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs`
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` (only if extract_tenant moves)
- `crates/argo-cli/src/handlers/gateway/rate_limit.rs` (additive header alias)
- `crates/argo-cli/tests/audit_list_detail_tenant_parity.rs` (NEW)

## Out of scope

- SPA-side propagation of a verified session tenant (covered by future `fix-spa-send-tenant-from-session-jwt`)
- The placeholder fallback content itself (covered by `fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance`)
- Cross-tenant 404 negative coverage in the strict-auth path (`fix-audit-cross-tenant-cap-bypass-mode-behavior`)

## Effort

M — ~80 LOC handler change + ~150 LOC integration test. ~4-6 hours including the parity test and cold cargo verify.
