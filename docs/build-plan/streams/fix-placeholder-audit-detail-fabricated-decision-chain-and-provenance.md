# fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance

**Owner**: claude.
**Authored against**: origin/main `74077bf676` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance`.
**Source**: ux-audit-2026-05-26-2214 `scene-6-claim-6-trust-and-approvals.md` F-N1 [P0] + F-N3 [P1].

🔴 **P0 — Demo-mode `placeholderAuditDetail()` fabricates a full 8-step decision chain ("Memory loaded · pricing rules", "trust 92%", "top score 0.87") + fake provenance (`claude-opus-4-7`, `placehold00000000` prompt hash, `policyplaceholder` policy hash) for any audit row id not in the canonical sample table. Today every live gateway audit row (29/29) is ULID-shaped and NOT in the sample table, so EVERY Scene-6 drill-in in demo mode renders this fabricated content. Function-side theatre — the camera reads invented reasoning as if it were real.**

## Root cause

`web/lib/api/audit-detail.ts:103-104` short-circuits to `demoModeDetail()` when `isDemoMode()` returns true. `demoModeDetail()` at lines 82-84 returns `findSampleAuditDetail(actionId) ?? placeholderAuditDetail(actionId)`. The fallback at `web/lib/sample-audit-detail.ts:285-310` returns:

```ts
{
  worker: { id: "wkr_unknown", name: "Unknown worker" },
  verb: "Action detail unavailable — drill into another row for full provenance.",
  decisionChain: defaultChain("generic-skill", "1.0"),  // 8 fabricated stages
  provenance: {
    model: { id: "claude-opus-4-7", version: "2026-04-30" },
    promptTemplate: { id: "generic.placeholder", hash: "placehold00000000" },
    policy: { id: "generic-policy", version: "1.0", hash: "policyplaceholder" },
  },
}
```

The `defaultChain` body at `sample-audit-detail.ts:32-43` renders 8 plausible-sounding lines:

```
Memory loaded · context (cached) · pricing rules
Skill: generic-skill v1.0 (trust 92%) · Confidence above autonomy threshold.
Plan generated · Primary plan + 2 alternatives.
Plan selected by score · Top score 0.87; alternates 0.74 / 0.62.
Policy check · Action permitted by policy.
Approval · Autonomous — no approval required.
Execution · Executor dispatched.
Outcome · Completed successfully.
```

The verb says "unavailable" but the chain reads as a real agent turn. Worse, the audit-row detail endpoint DOES return real `provenance.hash_chain_anchor` for the live gateway HTTP self-traces (verified: `ebccacbea0e21e84162d79cde13205ceebe16480796f153eebac3a8502bf9a41`), but the placeholder discards it and substitutes the fake `placehold00000000` / `policyplaceholder`.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F-N1 [P0] — Demo-mode drill-in fabricates 8-step decision chain
- scene-6-claim-6-trust-and-approvals F-N3 [P1] — `placeholderAuditDetail` lies about provenance (model, policy, prompt template)

## Fix recipe

**Path A (minimum) — collapse the placeholder to honest "unavailable" content.**

Edit `web/lib/sample-audit-detail.ts:285-310 placeholderAuditDetail`:

```ts
export function placeholderAuditDetail(actionId: string): AuditDetail {
  return {
    actionId,
    timestamp: new Date().toISOString(),
    worker: { id: "wkr_unknown", name: "Unknown worker" },
    verb: "Action detail unavailable",
    summary:
      "This action's full provenance is not available in the current view. Drill into another row, or visit the actor's profile for an action with populated decision chain.",
    decisionChain: [],  // EMPTY, not defaultChain — no fabrication
    provenance: null,    // EMPTY, not the fabricated trio
    // ...other fields unchanged
  };
}
```

`web/app/(app)/audit/action/[actionId]/page.tsx` already conditionally renders the decision-chain + provenance cards based on data presence (see `fix-audit-detail-render-empty-decision-chain-and-provenance-fallback` for the empty-state UX). The placeholder should hit the same empty-state path.

**Path B (recommended) — render a dedicated `<AuditActionUnavailable>` block.**

Instead of the 5-card layout with empty cards, render a single hero block with copy "Detail unavailable for this action" + a link "Back to audit feed". The page becomes obviously a placeholder, no risk of mistaken-for-real.

Add `data-placeholder="true"` testid to the page root so QA linters can detect the placeholder rendering.

**Path C (alternative) — pass through real `provenance.hash_chain_anchor` from the gateway response even when other fields are unavailable.**

For the live gateway HTTP self-traces, the `/api/v1/audit/<id>` endpoint DOES return a real hash anchor. The placeholder should preserve that field and drop only the fabricated content:

```ts
const live = await apiGet<GatewayAuditDetail>(...).catch(() => null);
if (!live || !live.decision_chain?.length) {
  return {
    ...placeholderShell(actionId),
    provenance: live?.provenance ? { hash_chain_anchor: live.provenance.hash_chain_anchor } : null,
  };
}
```

This way the camera sees a real hash on the placeholder card — honest about what's known, silent about what isn't.

Ship Path A (or A+B) as the minimum. Path C is an upgrade.

## Acceptance criteria

- [ ] `placeholderAuditDetail()` returns `decisionChain: []` (not fabricated 8 stages)
- [ ] `placeholderAuditDetail()` returns `provenance: null` (or only the real `hash_chain_anchor` if available) — no fake `claude-opus-4-7` / `placehold00000000` / `policyplaceholder` values
- [ ] `/audit/action/<unknown-id>` page renders the audit-empty-state UX (see sibling brief `fix-audit-detail-render-empty-decision-chain-and-provenance-fallback`)
- [ ] vitest spec: `placeholderAuditDetail("any-id")` returns an object whose `decisionChain.length === 0` and `provenance === null`
- [ ] vitest spec asserts no string `"claude-opus-4-7"` / `"placehold00000000"` / `"policyplaceholder"` / `"generic-skill"` / `"trust 92%"` appears in the placeholder return value
- [ ] grep test: `grep -n "defaultChain\|placehold00000000\|policyplaceholder" web/lib/sample-audit-detail.ts` returns nothing inside `placeholderAuditDetail`
- [ ] `pnpm --dir web exec tsc --noEmit` green

## May-touch

- `web/lib/sample-audit-detail.ts:285-310` (collapse the placeholder body)
- `web/lib/sample-audit-detail.ts:32-43` (optionally delete `defaultChain` if it has no remaining callers)
- `web/__tests__/lib/sample-audit-detail.test.ts` (NEW or extended, ~60 LOC)
- `web/app/(app)/audit/action/[actionId]/page.tsx` (optional: render `<AuditActionUnavailable>` block on placeholder shape)

## Out of scope

- The activity-row → action-id pattern that lands on the placeholder (covered by `fix-activity-row-audit-detail-synthesized-fixture-theatre`)
- The gateway-side missing audit rows (covered by `fix-audit-feed-missing-skill-invoked-and-workflow-step-executed-rows`)
- The strict-prod path empty-chain fallback (covered by `fix-audit-detail-render-empty-decision-chain-and-provenance-fallback`)

## Effort

~30 LOC change + ~60 LOC test. ~45 min. P0 because the placeholder fires on 100% of live gateway audit rows in demo mode and renders camera-visible fabricated content during the Scene-6 trust beat.
