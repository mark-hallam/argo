# fix-approvals-seed-decided-approvals-for-ring

**Owner**: claude.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-seed-decided-approvals-for-ring`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/scene-6-claim-6-trust-and-approvals.md` F7 [P1].

🟠 **P1 — The 7-day approval-rate ring on /approvals is the page's most decorative element. All 8 seeded `SAMPLE_APPROVALS` have `decision: null`, so `computeApprovalRate(items)` returns null and the ring renders as "—". The ring never animates on camera unless the operator clicks approve mid-take.**

## Root cause

`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:90-101`:
```ts
const approvalRate = computeApprovalRate(pending);
// returns null when every item has decision: null
```

`web/lib/sample-approvals.ts` (or equivalent — the file holding `SAMPLE_APPROVALS`): 8 entries, all `status: "pending"`, all `decision: null`. The dashboard is designed to surface a "this week's approval rate" metric, but the seed never includes resolved approvals.

For the v1.0 demo MP4, Scene 6's pan across /approvals lands on:
- 5 KPI tiles with numbers (good)
- 1 KPI tile permanently "—" ("Active reviewers", covered separately by `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts`)
- 1 approval-rate ring permanently "—" with body "No resolved approvals yet" (this brief)

The ring is right-aligned and visually prominent. Empty ring = page feels unfinished.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F7 [P1] — Approval-rate ring shows `—` because every seeded approval is pending

## Fix recipe

1. **Seed 2-3 SAMPLE_APPROVALS as resolved** in `web/lib/sample-approvals.ts`:
   - 2 entries with `status: "approved", decision: "approve", decided_at: <2 days ago>`.
   - 1 entry with `status: "denied", decision: "deny", decided_at: <yesterday>`.
   - 5 remaining as pending (the inbox needs items to act on during the demo).

2. **Match the demo-seed CLI** so the gateway-side seed (`fix-demo-seed-cli-for-real-roster` follow-up) emits the same 3:5 ratio on the live path. The SPA fixture and the gateway seed should match — otherwise live-mode demos see one shape, mock-mode demos see another.

3. **Verify ring renders post-seed**: `computeApprovalRate` should return `2/3 ≈ 0.667` based on 2 approved out of 3 resolved (denies counted in total). Ring shows 67%.

4. **Add explicit decided_at fixtures** so the "7-day window" filter has meaningful rows; without it the rate computation might still null-out for "no resolutions in last 7 days".

5. **Animation polish**: the ring already animates on mount via Framer Motion (verify in `web/components/skycot/approvals/ApprovalRateRing.tsx`). With non-null data, the demo recording captures the ring sweeping from 0% to 67%.

6. **Tests** `web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx`:
   - Mount with 3 resolved + 5 pending → ring shows 67%.
   - Mount with all pending → ring shows "—" + body "No resolved approvals yet" (existing behavior preserved).

## Acceptance criteria

- [ ] `/approvals` on first paint renders the ring with a real percentage, not "—".
- [ ] Ring animates from 0% to its target value via Framer Motion mount transition.
- [ ] Demo MP4's Scene 6 pan captures a populated ring without operator intervention.
- [ ] Live gateway demo seed emits the same 3:5 ratio.
- [ ] Empty-state path (all-pending) still renders "—" gracefully for fresh tenants.
- [ ] Unit tests cover populated + empty states.

## May-touch

- `web/lib/sample-approvals.ts` (3 entries → resolved)
- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` (no logic change; verify ring shape)
- `web/components/skycot/approvals/ApprovalRateRing.tsx` (verify animation; tiny tweak if needed)
- `web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx` (extend)
- `crates/argo-demo-seed/src/approvals.rs` or similar (gateway-side parity — verify via grep)

## Out of scope

- "Active reviewers" KPI showing "—" (covered by `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts`)
- $ at risk donut shape (covered by `fix-approvals-ops-dashboard-dollar-at-risk-donut-shape-implies-percent`)
- Approval row click destination (covered by `fix-approvals-rows-unclickable-no-detail-route`)

## Effort

~50-90 LOC across fixture file + tests + gateway-seed mirror. ~1-2 hours.
