# fix-approvals-audit-fetchers-fallback-on-200-empty-in-demo-mode

**Owner**: claude.
**Authored against**: origin/main `74077bf676` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-audit-fetchers-fallback-on-200-empty-in-demo-mode`.
**Source**: ux-audit-2026-05-26-2214 `scene-6-claim-6-trust-and-approvals.md` F-N2 [P1].

🟠 **P1 — Even after `fix-claude-launch-json-env-var-naming-mismatch` lands and demo-mode fires correctly, the approvals + audit fetchers WILL NOT fall back to SAMPLE data because their fallback gate is `404 / network-err`, not `200 OK with empty list`. The gateway returns 200 with `{items:[]}` for the demo tenant (no real approvals seeded), so SAMPLE_APPROVALS + SAMPLE_AUDIT_ROWS never load. Net: every Scene-6 demo-mode fix shipped this week (#2397 / #2404 / #2405 / #2406) is dormant on the live demo path until this fallback logic is widened.**

## Root cause

Two fetchers share the same pattern:

`web/lib/api/approvals.ts:179-201 fetchApprovals()`:

```ts
if (isAuthBypassed()) return SAMPLE_APPROVALS;
try {
  const res = await apiGet<...>(/api/v1/approvals);
  return res;  // ← returns 200-with-empty as-is
} catch (err) {
  if (isMissingOrUnreachable(err)) {
    if (isDemoMode()) return SAMPLE_APPROVALS;
    return [];
  }
  throw err;
}
```

`web/lib/api/audit.ts:345-367 fetchAuditFeed()`: identical shape.

Failure mode: gateway returns 200 with `{items: []}` for a fresh demo tenant (because no real approvals / audit rows have been written). `apiGet` resolves normally. The catch block never fires. SAMPLE_* fallback never loads.

Live evidence:

```
curl -sS -H 'X-Argo-Principal: human:demo@acmeco.com' http://127.0.0.1:7400/api/v1/approvals
# → {"items":[],"total":0}

curl -sS -H 'X-Argo-Principal: human:demo@acmeco.com' http://127.0.0.1:7400/api/v1/audit
# → {"items": [...29 self-trace rows...], "total":29}
```

For approvals: 200-empty → SPA renders 0 cards on Scene 6's KPI strip even though the seeded SAMPLE_APPROVALS would fill them.

For audit: 200-with-29-rows → SPA renders the 29 HTTP self-traces (all `worker.name="unknown"`, all `verb="http.request"`, no provenance) instead of the seeded SAMPLE_AUDIT_ROWS. Then drill-in falls back to the placeholder fabrication path (separate brief `fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance`).

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F-N2 [P1] — Fetchers don't fall back to SAMPLE on 200-with-empty-list under demo mode

## Fix recipe

Operator preference per memory `feedback_no_theater_in_demo`: prefer the real backend seed. But the seed CLI brief is multi-week and the demo MP4 is due now — ship the SPA gate as the interim, author/refresh the seed-side brief as the long-term fix.

**Path A — SPA interim fallback (recommended, ~20 LOC).**

Update both fetchers' success path:

```ts
// approvals.ts
const res = await apiGet<ApprovalsEnvelope>(/api/v1/approvals);
if (res.items.length === 0 && isDemoMode()) {
  return { source: "sample-demo-empty-gateway", items: SAMPLE_APPROVALS };
}
return { source: "gateway", items: res.items };
```

And for audit, similarly — but be more careful with audit because the gateway DOES return 29 self-trace rows. The right gate is "rows exist BUT all are `worker.name === 'unknown'` AND `verb === 'http.request'`":

```ts
// audit.ts
const liveRows = res.items;
const allSelfTraces = liveRows.every(
  (r) => r.worker?.name === "unknown" && r.verb === "http.request"
);
if (liveRows.length === 0 || (allSelfTraces && isDemoMode())) {
  return { source: "sample-demo-no-agent-rows", items: SAMPLE_AUDIT_ROWS };
}
return { source: "gateway", items: liveRows };
```

This preserves real agent-attributed rows (the moment the seed lands them, the fallback de-activates).

**Path B — real seed (long-term, separate brief).**

`argo demo seed --tenant <demo>` should write SAMPLE_APPROVALS-shaped real approval rows + SAMPLE_AUDIT_ROWS-shaped real audit rows at seed time. Then the fetchers never hit the fallback because the gateway truth IS the sample data. Refresh `fix-demo-seed-cli-for-real-roster` to include this scope (or author `fix-demo-seed-approvals-and-audit-rows` as a sibling).

Ship Path A now. Path B is the proper architecture, not blocking the recording.

## Acceptance criteria

- [ ] `fetchApprovals()` returns SAMPLE_APPROVALS when gateway returns 200-empty AND `isDemoMode() === true`
- [ ] `fetchAuditFeed()` returns SAMPLE_AUDIT_ROWS when gateway returns 200-with-all-self-traces AND `isDemoMode() === true`
- [ ] Both functions still return real gateway data when ANY row has `worker.name !== "unknown"` (don't shadow real agent rows)
- [ ] `source` field added to return type so downstream UI / tests can tell which path fired
- [ ] vitest spec covers: (a) bypass on, (b) gateway-200-empty + demo, (c) gateway-200-empty + non-demo, (d) gateway-200-with-real-rows + demo (returns real, not sample), (e) gateway-404 + demo
- [ ] `pnpm --dir web exec tsc --noEmit` green

## May-touch

- `web/lib/api/approvals.ts` (~15 LOC change at fetchApprovals)
- `web/lib/api/audit.ts` (~20 LOC change at fetchAuditFeed)
- `web/__tests__/lib/api/approvals.test.ts` (NEW or extended, ~50 LOC)
- `web/__tests__/lib/api/audit.test.ts` (NEW or extended, ~60 LOC)

## Out of scope

- The env-var rename itself (covered by `fix-claude-launch-json-env-var-naming-mismatch`)
- Backend seed CLI for approvals + audit (long-term, separate brief)
- The placeholder detail fabrication (covered by `fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance`)

## Effort

~35 LOC change + ~110 LOC test. ~1 hour. P1 because Scene 6 KPI strip + audit feed depend on this firing, but it sits behind F1 (env-var rename) in the dependency chain — won't matter until that lands.
