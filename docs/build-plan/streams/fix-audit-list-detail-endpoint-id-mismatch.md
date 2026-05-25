# fix-audit-list-detail-endpoint-id-mismatch

**Owner**: claude.
**Authored against**: origin/main `<latest>` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-list-detail-endpoint-id-mismatch`.
**Source**: e2e-ux-audit Agent 4 finding F2 (`docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md`).

🔴 **P0 DEMO-GATE — Scene 6 drill-in is literally unrecordable. Function-side bug.**

`GET /api/v1/audit?range=7d` and `GET /api/v1/audit/{id}` **disagree about which rows exist**. Agent-4 curl verification on live gateway (schema v81, 5h uptime):

```
$ curl http://127.0.0.1:7400/api/v1/audit?range=7d | jq '.items[0].id'
"019e5dbaec037fa2819eab3f27f3f41e"

$ curl http://127.0.0.1:7400/api/v1/audit/019e5dbaec037fa2819eab3f27f3f41e
{"error":{"code":"not_found","message":"audit row not found"}}
```

`ActivityRow.tsx:30` constructs the audit drill-in href as `/audit/action/${id}` — so clicking ANY real audit row in the SPA lands on `<AuditActionNotFound>` at `web/app/(app)/audit/action/[actionId]/page.tsx:96-106`. Combined with the replay-fabrication bug (`fix-replay-page-fabricated-forensic-report-for-unknown-action-ids`), a curious buyer clicking around the audit feed gets either an "action not found" shell OR a fabricated forensic report.

Per the v1.0 demo runbook Scene 6 step 7: "Click an audit row → real provenance". This is currently unrecordable against the live backend.

## Root cause (hypothesis)

Most likely one of:

1. **Tenant scoping mismatch**: list endpoint reads from one view (no tenant filter, or different tenant context) while detail endpoint applies a strict `WHERE tenant_id = ?` filter that the list-returned row doesn't pass.
2. **Two underlying tables / views**: list reads from an aggregate view that includes denormalised/shadow rows; detail reads from the canonical audit_record table only.
3. **ID format / encoding**: list returns IDs in one format (e.g. lowercase hex); detail expects another (e.g. ULID).

Worker should grep for both endpoint handlers in `crates/argo-cli/src/handlers/gateway/` — likely `routes_audit*.rs` — and diff the SQL queries.

## Fix (worker decides recipe)

Align the two endpoints so any ID the list returns can be successfully fetched by detail:

**Option A (preferred — minimal scope)**: align tenant filter. If list isn't scoping but detail is, add the tenant filter to list. If detail is rejecting rows the list returns (because list reads a richer view), broaden detail's source to match.

**Option B (broader)**: replace the list endpoint's source with a derived query that joins through whatever predicate detail uses — guarantees consistency.

Either way:
- Add a regression test: `tests/audit_list_detail_endpoint_id_alignment.rs` — assert that for every row returned by `GET /api/v1/audit?range=7d`, `GET /api/v1/audit/{id}` returns 200 (not 404).
- Add a cross-tenant test: tenant A's list-returned IDs are NOT fetchable by tenant B's detail endpoint.

## Acceptance criteria

- [ ] Curl test: every ID returned by `/api/v1/audit?range=7d` is fetchable via `/api/v1/audit/{id}` (200, not 404). PR body has before/after curl matrix.
- [ ] Cross-tenant: tenant A's list-returned IDs return 404 (not the row) when fetched as tenant B.
- [ ] Regression test added.
- [ ] No new 500s or panics on either endpoint.
- [ ] PR body cites Agent-4 audit report.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_audit*.rs` (most likely)
- `crates/argo-store/src/audit*.rs` (if the underlying store needs adjustment)
- `crates/argo-cli/tests/audit_list_detail_endpoint_id_alignment.rs` (NEW)
- This brief file (status updates)

## Out of scope

- Replay page fabrication (separate brief: `fix-replay-page-fabricated-forensic-report-for-unknown-action-ids`)
- Hash-chain prev→this rendering (separate brief: `fix-audit-hash-chain-prev-this-hover-affordance`)
- Audit feed visual design (locked + unchanged)

## Effort

~50-150 LOC + regression test + cross-tenant test. ~45-90 min depending on root-cause depth.
