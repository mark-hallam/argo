# fix-approvals-cross-tenant-read-path-verification

**Owner**: claude.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-cross-tenant-read-path-verification`.
**Source**: e2e-ux-audit Agent 4 finding F12 (`docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md`).

🟡 **P1 — cross-tenant isolation on `/api/v1/approvals` is unverified for Claim 6 evaluator. Cannot test without seed.**

## Root cause

Agent 4 curl-verified:

```
$ curl /api/v1/approvals                                  → {"items":[]}
$ curl -H "X-Tenant-Id: tenant_b_other" /api/v1/approvals → {"items":[]}
```

Both tenants return empty. With no rows on either side, Agent 4 could only confirm that the read path doesn't crash or leak — not that tenant scoping is actually enforced at the SQL level. For the v1.0 demo evaluator scenario `claim_6_live_cross_tenant`, the gateway needs:

1. Tenant A with seeded approvals.
2. A negative test: tenant B's read returns zero of tenant A's approvals.
3. Same coverage on `/api/v1/audit` (Agent 4 also flagged the audit list returned the same rows for both default and `X-Tenant-Id: tenant_b_other` headers — but couldn't tell whether the header was advisory in dev-bypass).

Until that's curl-able, claim 6's "LIKELY-LIVE" rating on `/approvals` specifically is unverified by this audit.

## Findings this brief closes

- Agent 4 F12 [P2] — Cross-tenant isolation: live `/api/v1/approvals` ignores tenant header on the read path (unverified — needs seed)

## Fix recipe

1. **Seed two tenants with approvals via the demo-seed CLI**: tenant_a with ≥3 pending approvals, tenant_b with ≥2 pending approvals, all with distinct `dollar_at_risk` values so the row identity is obvious.

2. **Author a cross-tenant negative test** at `crates/argo-cli/tests/approvals_cross_tenant_isolation.rs`:
   ```rust
   #[test]
   fn approvals_read_path_isolates_tenants() {
       let resp_a = get("/api/v1/approvals", tenant_a_principal()).expect_200();
       let resp_b = get("/api/v1/approvals", tenant_b_principal()).expect_200();
       assert_eq!(resp_a.items.len(), 3);
       assert_eq!(resp_b.items.len(), 2);
       let a_ids: HashSet<_> = resp_a.items.iter().map(|i| i.id).collect();
       let b_ids: HashSet<_> = resp_b.items.iter().map(|i| i.id).collect();
       assert!(a_ids.is_disjoint(&b_ids));
   }
   ```

3. **Audit-side mirror**: similar test for `/api/v1/audit` cross-tenant isolation.

4. **If the read path turns out NOT to be enforcing tenant scope** (the test fails), fix at the SQL/handler layer in `crates/argo-cli/src/handlers/gateway/routes_approvals*.rs`. Use the recently-shipped `#2257` cross-tenant negative-pin pattern (per the CLAUDE.md status block) as the template.

5. **Curl matrix in PR body**: before/after, with two tenants, showing the items disjoint.

## Acceptance criteria

- [ ] Two-tenant seed populates ≥3 and ≥2 approvals respectively.
- [ ] `approvals_cross_tenant_isolation` test passes cold (cargo test).
- [ ] Mirror audit cross-tenant test (`audit_cross_tenant_isolation`) passes cold.
- [ ] PR body includes curl matrix proving disjoint reads.
- [ ] Claim-6-LIKELY-LIVE flip to LIVE in `MISSION.md` (per the evaluator's pending cold rerun of `claim_6_live_cross_tenant`).

## May-touch

- `crates/argo-cli/src/cli/demo_seed/approvals.rs` (or wherever approval seed lives)
- `crates/argo-cli/tests/approvals_cross_tenant_isolation.rs` (NEW)
- `crates/argo-cli/tests/audit_cross_tenant_isolation.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes_approvals*.rs` (only if the test fails)
- `crates/argo-cli/src/handlers/gateway/routes_audit*.rs` (only if the audit test fails)

## Out of scope

- The KPI strip hardcoded fabrications (separate brief: `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts`)
- Approval rows unclickable (separate brief: `fix-approvals-rows-unclickable-no-detail-route`)
- Cross-tenant tests on other read endpoints (follow-up if uncovered)

## Effort

~100-200 LOC across tests + seed + (maybe) handler. ~1-2 hours.
