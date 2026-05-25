# fix-audit-list-row-in-tenant-scope-legacy-leak

**Owner**: claude.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-list-row-in-tenant-scope-legacy-leak`.
**Source**: e2e-ux-audit Agent (scene-6-claim-6-trust-and-approvals) finding F5 (`docs/skycot/ux-audits/2026-05-25-2309/scene-6-claim-6-trust-and-approvals.md`).

🔴 **P0 — Cross-tenant confidentiality leak in `GET /api/v1/audit` list path. Claim 6 ("Multi-tenant by construction") structurally violated by the read path.**

## Root cause

`crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:41-48` (`row_in_tenant_scope`) treats `payload.tenant_id == None` as "include this row for whichever tenant is asking". The audit_record table currently has ~1,193 rows in the legacy `anonymous` bucket whose JSON payloads do not embed a `tenant_id` field — so every request, regardless of `x-argo-tenant-id` header, sees them.

Curl proof (from the audit run):

```
$ curl -s "http://127.0.0.1:7400/api/v1/audit?range=7d&limit=5" \
    -H "x-argo-tenant-id: ws-malicious" | jq '.items | length'
5

$ curl -s "http://127.0.0.1:7400/api/v1/audit?range=7d&limit=5" \
    -H "x-argo-tenant-id: ws-tenant-b" | jq '.items[].id'
"019e5dbaec037fa2819eab3f27f3f41e"   ← same id another tenant sees
"019e5db5097b77d0becda2085091918c"
```

Two completely different (and one entirely fabricated) tenant ids see the same rows. The detail endpoint (`routes_audit_detail.rs:74-83`) is correctly strict (`WHERE id = ? AND tenant_id = ?`); the list endpoint is not. The asymmetry is what produced F2 (list/detail id mismatch) in earlier audits — but the security implication is the bigger story: any tenant can inventory the legacy bucket's metadata.

A buyer poking the network panel in mid-demo + swapping the `x-argo-tenant-id` header in a curl will discover the leak instantly. Per the function-vs-dressing rule, this is function-side and the camera doesn't need to see it for the defect to torpedo the trust narrative.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F5 [P0] — Cross-tenant scope leak in `/api/v1/audit` list path; rows with payload `tenant_id == None` are unioned into every tenant's view

## Fix recipe

1. **Tighten `row_in_tenant_scope`**. The default for legacy rows (payload missing `tenant_id`) should be **deny**. The function should only `include_legacy = true` when the caller has explicitly resolved to the `__legacy__` admin bucket — never on a normal tenant request. Touch site: `crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:41-48`.

2. **Migrate or quarantine the 1,193 legacy rows**. Add a one-shot migration that either:
   - (a) backfills `tenant_id = '__legacy__'` into the row's `payload_json` for all rows where it's currently missing, OR
   - (b) tags them on a separate `audit_legacy` view that requires an admin-only resolve path.
   Operator's call; option (a) is the lower-risk default because it preserves the rows for forensic queries via the admin lane.

3. **Tenant-scope tests**. Add `crates/argo-cli/tests/audit_list_cross_tenant_scope.rs`:
   - Seed 2 rows in tenant `ws-a` and 2 rows in tenant `ws-b`.
   - Seed 1 legacy row (no `tenant_id` in payload).
   - `GET /api/v1/audit` as `ws-a` → 2 items, none `ws-b`, none legacy.
   - `GET /api/v1/audit` as `ws-b` → 2 items, none `ws-a`, none legacy.
   - `GET /api/v1/audit` as fabricated `ws-malicious` → 0 items.
   - `GET /api/v1/audit` as `__legacy__` admin → all 5 items.

4. **Regression test** in the `claim_6_live_cross_tenant` evaluator scenario so any future re-loosening trips the live regression suite.

## Acceptance criteria

- [ ] `curl -H "x-argo-tenant-id: ws-malicious" /api/v1/audit` returns `{"items":[]}` (was: returns ~5 legacy rows).
- [ ] `curl -H "x-argo-tenant-id: ws-tenant-b" /api/v1/audit` does NOT return any id that `curl -H "x-argo-tenant-id: ws-tenant-a"` returns (was: identical id lists).
- [ ] New `audit_list_cross_tenant_scope.rs` integration test passes.
- [ ] `claim_6_live_cross_tenant` regression cold-rerun still PASS.
- [ ] Legacy rows are still queryable from the `__legacy__` admin lane (no data loss).

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs`
- `crates/argo-store/src/migrations/` (new migration for legacy backfill)
- `crates/argo-cli/tests/audit_list_cross_tenant_scope.rs` (NEW)
- `crates/argo-eval/tests/` (claim_6 regression extension)

## Out of scope

- `/api/v1/approvals` cross-tenant verification (separate brief `fix-approvals-cross-tenant-read-path-verification`)
- Audit detail endpoint scope (already correct)
- The unrelated `worker.name = "unknown"` rows (separate brief `fix-audit-unknown-worker-and-hipaa-fixture-rows`)

## Effort

~80-150 LOC in `routes_audit_feed_scope.rs` + 1 migration + 1 new integration test. ~2-3 hours. **Ship before next demo recording — security finding.**
