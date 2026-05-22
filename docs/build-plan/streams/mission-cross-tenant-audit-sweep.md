# mission-cross-tenant-audit-sweep

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `cbd19bbb3` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cross-tenant-audit-sweep`.

🟡 **P2 SECURITY HYGIENE.** Source: pattern observation across ticks 38-40 — every recently-authored brief includes a "cross-tenant negative test required" note because last week's #1983 PR shipped a real cross-tenant data-exposure bug. The conductor expectation is now documented (every new endpoint must include the negative test), but **older endpoints predating the rule have never been audited**. Sweep them all and flag any that lack one.

## Scope

This is an **AUDIT brief**, not an implementation brief. Output is:
1. A markdown report at `docs/skycot/audit/cross-tenant-endpoint-coverage.md` listing every gateway endpoint and whether its test file includes a cross-tenant negative test.
2. Up to 3 follow-up brief stubs in `streams/` named `mission-cross-tenant-test-<area>` for the highest-risk gaps (e.g. an endpoint that reads `IdentityStore` rows without filtering by caller tenant).
3. NO actual code changes to handlers — this brief produces findings + follow-up briefs; the actual fix work happens in those follow-ups.

The audit covers:
- Every `*_tests.rs` file under `crates/argo-cli/src/handlers/gateway/` and `crates/argo-cli/tests/`.
- Grep pattern: `tenant_a` + `tenant_b` (or `t1`/`t2`) + 403/empty-result assertion in the same test fn.
- Endpoints without that pattern are flagged "no cross-tenant test".
- For each flagged endpoint, the report estimates the data-exposure risk class (low / medium / high) based on what the endpoint reads or mutates.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: docs/skycot/audit/cross-tenant-endpoint-coverage.md` — the audit report.
- `NEW: streams/mission-cross-tenant-test-<area>.md` × up to 3 — follow-up briefs for the highest-risk gaps. Each follow-up brief contains a properly-shaped TODO row spec (the conductor inserts the row into BUILD_STATUS.md in a separate replenishment commit; this brief just AUTHORS the brief files).
- `NEW: scripts/audit-cross-tenant-coverage.sh` — the grep+report script used to generate the audit. ~30-50 LOC. Idempotent — running it again produces the same report (modulo new endpoints since last run).

## Files you must NOT touch

- Any handler source under `crates/argo-cli/src/handlers/`.
- Any test source under `crates/argo-cli/tests/`.
- Any production code — this brief is read-only / report-generating.
- BUILD_STATUS.md outside the standard row flip on this brief's own row.

## Acceptance criteria

- [ ] `docs/skycot/audit/cross-tenant-endpoint-coverage.md` exists with: endpoint table (path | handler file | test file | has-cross-tenant-test? | risk-class), top-of-doc summary (X endpoints total, Y have cross-tenant test, Z don't).
- [ ] Report's "high-risk gaps" section names ≤3 endpoints with the highest data-exposure if a cross-tenant leak existed (e.g. reading worker rows, audit rows, skill rows, identity rows).
- [ ] Up to 3 follow-up briefs authored in `streams/`, each with the proper template + freshness header + may-touch list. Each follow-up brief is sized 200-500 LOC and adds the cross-tenant test for ONE endpoint.
- [ ] `scripts/audit-cross-tenant-coverage.sh` exists, is idempotent, and the report regenerates from it.
- [ ] PR body's "Next conductor action" section names: "insert TODO rows for the N follow-up briefs into BUILD_STATUS.md in the next replenishment commit".
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-cross-tenant-audit-sweep`
- PR title: `audit: cross-tenant endpoint coverage sweep + follow-up briefs (mission-cross-tenant-audit-sweep)`

## Notes for the implementer

- ~200-400 LOC expected (most in the report markdown + 3 follow-up briefs).
- This brief produces a SECURITY-relevant report. Land it cleanly — false negatives (missing a real gap) are more costly than false positives.
- The grep heuristic is approximate. If a test mocks tenants in a way the grep misses, the report can say "needs manual review" rather than flag as "no test". Document the heuristic's limits in the report.
- DON'T try to fix the gaps in this brief. The follow-up briefs are the fixes. Conductors prefer smaller, targeted PRs to one mega-PR.
- The first follow-up brief should target whichever gap has the HIGHEST data-exposure risk (e.g. an endpoint that returns full worker rows without tenant filter). Subsequent follow-ups can ship in any order.
- If the audit finds ZERO gaps, the brief still ships — the report becomes the proof-of-coverage doc. Mention that explicitly in the PR body as "0 gaps found, all N endpoints have cross-tenant tests" so the next conductor doesn't re-run the audit.
