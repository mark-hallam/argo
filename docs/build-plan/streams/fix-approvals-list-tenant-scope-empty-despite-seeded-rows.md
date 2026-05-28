# fix-approvals-list-tenant-scope-empty-despite-seeded-rows

**Severity**: 🔴 P0 — MVP demo Scene 6 blocker

**Authored against**: `origin/main` SHA `58ea1c93f` on 2026-05-28T03:55Z.

**Discovered during**: 2026-05-28 e2e walkthrough — `docs/skycot/demos/walkthroughs/2026-05-28-e2e-dry-run-with-operator.md` Scene 6.

## Symptom

The `/approvals` SPA page renders an empty state ("No high severity approvals waiting" / "No medium severity approvals waiting" / "No low severity approvals waiting") despite the demo seed creating 8 `approval_request` rows.

Direct verification:

```bash
# Seed output (run earlier same session):
$ argo demo seed --tenant 019e5057-... --skip-pass-a-turns
  approval_requests:          8

# Direct sqlite count by tenant (binary blob hex):
$ sqlite3 ~/.argo/sessions.db "SELECT lower(hex(tenant_id)), COUNT(*) FROM approval_request GROUP BY tenant_id ORDER BY 2 DESC LIMIT 5;"
019e67e4...|19
019e50571d3f7fa29c1bc34ef01cc21e|8       ← workspace tenant has 8 rows from the seed
019e6c72...|8
019e63<...>|3

# Gateway returns empty regardless of headers:
$ curl -s http://127.0.0.1:7400/api/v1/approvals | jq '.items | length'
0

$ curl -s -H "X-Argo-Tenant-Id: 019e5057-1d3f-7fa2-9c1b-c34ef01cc21e" \
       -H "X-Argo-Principal: human:demo-walkthrough@skycot.com" \
       http://127.0.0.1:7400/api/v1/approvals | jq '.items | length'
0

$ curl -s http://127.0.0.1:7400/api/v1/approvals/pending
{"error":"not found"}    # endpoint not mounted
```

## Root cause hypothesis

Same family as `fix-audit-detail-tenant-scope-list-detail-sql-mismatch` (companion brief): the approvals list handler resolves tenant differently than the seed writes. Either:

1. **Write/read tenant mismatch**: seed writes to a tenant resolved one way (e.g. `cwd_write_tenant` from `routes_onboarding_helpers.rs::cwd_write_tenant`), read handler scopes to a different one (e.g. `rate_limit::extract_tenant`)
2. **Tenant_id encoding mismatch**: `approval_request` table stores tenant_id as binary BLOB (per sqlite query above), but the read handler binds a TEXT UUID-string; sqlite then never matches

Search for the actual handler: `crates/argo-cli/src/handlers/gateway/routes_approvals_json::list_response` (per `routes_dispatch_get_api_groups.rs:80-81`).

## Acceptance criteria

- [ ] After `argo demo seed --tenant <workspace-tenant>` writes 8 approval_request rows, `GET /api/v1/approvals` (with workspace-default resolution) returns those 8 rows
- [ ] The 3 severity-tier sections on `/approvals` populate per the seed's severity distribution
- [ ] Channel chip renders on each row (Slack / SMS / Discord / webinbox / email — PR #2569 added this; verify it still works once rows surface)
- [ ] Regression test: `cargo test -p argo-cli --test approvals_list_seeded_rows_surface` — seed 5 rows in tenant T via `IdentityStore::create_approval_request`-equivalent path, GET `/api/v1/approvals` with appropriate principal, assert 5 items in response
- [ ] Cross-tenant negative check still passes (`claim_6_cross_tenant_approval_select_no_existence_leak` — already MERGED via #2113, do not regress)

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs` — likely fix list_response tenant resolution
- `crates/argo-cli/src/handlers/gateway/routes_approvals.rs` (sibling)
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs` — if a new endpoint variant (e.g. `/api/v1/approvals/pending`) is needed
- new regression test: `crates/argo-cli/tests/approvals_list_seeded_rows_surface.rs`

## Must-not-touch

- The cross-tenant invisibility property (`claim_6_cross_tenant_approval_select_no_existence_leak`) — this fix MUST keep cross-tenant leakage closed

## References

- Operator NO-THEATER directive: `feedback_no_theater_in_demo`
- Companion P0 brief: `fix-audit-detail-tenant-scope-list-detail-sql-mismatch` (same root-cause family — gateway tenant-scope drift across list/detail/write)
- PR #2569 (channel chip): `fix-approvals-row-channel-chip-claim-6-visual-proof` — landed but unverifiable until rows surface
- Walkthrough source: `docs/skycot/demos/walkthroughs/2026-05-28-e2e-dry-run-with-operator.md` Scene 6
