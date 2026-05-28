# fix-audit-detail-tenant-scope-list-detail-sql-mismatch

**Severity**: 🔴 P0 — MVP demo Scene 2 + Scene 6 drill-in blocker

**Authored against**: `origin/main` SHA `58ea1c93f` on 2026-05-28T03:55Z.

**Discovered during**: 2026-05-28 e2e walkthrough — `docs/skycot/demos/walkthroughs/2026-05-28-e2e-dry-run-with-operator.md` Scene 2.

## Symptom

`GET /api/v1/audit?limit=5` returns real audit-row items (e.g. `019e6cd2073978219924ce38a5777547` for Nori's `worker.kpi_rolled`).
`GET /api/v1/audit/019e6cd2073978219924ce38a5777547` returns `{"error":{"code":"not_found","message":"audit row not found"}}`.

Same gateway, same auth, same request flow. **List sees rows; detail 404s on the SAME id.**

## Why PR #2589 didn't fully close this

PR #2589 (MERGED 2026-05-28 — `fix-gateway-audit-detail-tenant-resolver-drift-vs-feed`) made `routes_audit_detail::detail_response` and `routes_audit_feed::list_response` use a shared `rate_limit::extract_tenant(req)` helper. That fixed the TENANT-EXTRACTION drift (both endpoints now resolve the request's caller-tenant via the same precedence chain: `?tenant=` → `X-Tenant-Id` → `X-Argo-Tenant-Id` → config default → `"anonymous"`).

But the SQL filtering in `fetch_row` (routes_audit_detail.rs:254-275) requires the row's stored `tenant_id` to EQUAL the request's tenant:

```sql
SELECT id, ... FROM audit_record
WHERE id = ?
  AND (tenant_id = ?       -- row's tenant_id must match request's
       OR (? = ? AND tenant_id = ?))   -- legacy: request=DEFAULT and row=LEGACY
LIMIT 1
```

Bindings: `id_bytes, request_tenant, request_tenant, DEFAULT_AUDIT_TENANT_ID, LEGACY_AUDIT_TENANT_ID`.

The list endpoint returns rows that DON'T satisfy this filter. Either:
1. The list endpoint is querying with a broader tenant scope than detail (no tenant filter, or "anonymous-bypass", or DB-default-tenant)
2. The seeded rows have `tenant_id` values that don't match what extract_tenant returns when no `X-Argo-Tenant-Id` header is present

## Diagnostic dump (from the live workspace 2026-05-28T03:55Z)

```bash
$ curl -s http://127.0.0.1:7400/api/v1/audit?limit=2 | jq '.items[].id'
"019e6cd69b1a7f33ba78200a671c91e9"
"019e6cd69b147d0196f48766ccbc0f16"

$ curl -s http://127.0.0.1:7400/api/v1/audit/019e6cd69b1a7f33ba78200a671c91e9
{"error":{"code":"not_found","message":"audit row not found"}}

# Direct sqlite count by tenant_id:
$ sqlite3 ~/.argo/sessions.db "SELECT lower(hex(tenant_id)), COUNT(*) FROM audit_record GROUP BY tenant_id ORDER BY 2 DESC LIMIT 5;"
019e50571d3f7fa29c1bc34ef01cc21e|<count>  ← the cwd workspace tenant (binary blob hex)
```

The list endpoint scopes to one resolved tenant; detail-via-curl resolves to a different default ("anonymous"); the binary-vs-text tenant_id storage format may also play a role.

## Acceptance criteria

- [ ] `GET /api/v1/audit?limit=5` returns rows with ids X1, X2, ...
- [ ] `GET /api/v1/audit/X1` returns a 200 with the full AuditDetail body (decision chain, payload, provenance, output, refs)
- [ ] Same `X-Argo-Tenant-Id` header on both calls returns matching results — no rows in the list that can't be drilled into
- [ ] Regression test: `cargo test -p argo-cli --test audit_list_detail_tenant_parity` — extend the existing parity test (added in #2589) to seed 3 rows in tenant T, query list (count == 3), then query detail for each (all 200 OK, none 404)
- [ ] Demo path verification: with `argo demo seed` populated tenant + a fresh `/audit` SPA navigation, clicking any list row leads to a real AuditDetail page, not "Action not found"

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs` — likely fix `fetch_row` SQL to match whatever scope the list endpoint uses
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` / `routes_audit_feed_indexed.rs` — to align the read side
- `crates/argo-cli/tests/audit_list_detail_tenant_parity.rs` — extend the regression to actually exercise list→detail drill-in
- `crates/argo-cli/src/handlers/gateway/audit_query.rs` — possibly the shared scope resolver lives here

## Must-not-touch

- The audit chain hash-verification logic — that's correctness-critical, leave it alone

## References

- Operator NO-THEATER directive: `feedback_no_theater_in_demo`
- PR #2589 (incomplete fix): MERGED 2026-05-28 — closed the extract_tenant helper drift but not the underlying SQL-filter divergence
- PR #2515 (MERGED 2026-05-27): `fix-audit-action-not-found-page-fallback-routing` — closes the SPA's empty-state routing for unknown ids, doesn't close the list/detail SQL gap
- PR #2405 (MERGED 2026-05-26): `fix-audit-action-detail-demo-mode-fallback` — demo-mode fallback, doesn't help real-tenant rows
- Walkthrough source: `docs/skycot/demos/walkthroughs/2026-05-28-e2e-dry-run-with-operator.md` Scene 2 + Scene 6
