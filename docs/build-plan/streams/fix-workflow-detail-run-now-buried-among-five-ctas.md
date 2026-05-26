# fix-workflow-detail-run-now-buried-among-five-ctas

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-detail-run-now-buried-among-five-ctas`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/scene-4-and-5-claim-4-and-5.md` F14 [P2].

🟡 **P2 — Workflow-detail page renders 5 action buttons: Open builder · Edit in chat · Version history · + Add as Schedule · Run now. The Run button (the only one Scene 5 needs) is fifth. During Scene 5's "Hit Run" pan, the operator's eye drifts across 4 other CTAs first.**

## Root cause

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:239-282` — 5 CTAs in a row, Run-now last.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F14 [P2] — Run-now buried among 5 CTAs

## Fix recipe

1. **Promote "Run now" to leftmost** position with primary visual weight (gradient button).
2. **Demote the other 4 to secondary chrome**: Open builder + Edit in chat + Version history + Add as Schedule → either an overflow kebab menu or smaller ghost buttons.
3. **Web test** asserts Run-now is the first child of the CTA row + has primary variant class.

## Acceptance criteria

- [ ] Run-now is the leftmost / primary CTA on the workflow detail page.
- [ ] Other 4 CTAs accessible but visually demoted.
- [ ] Web test asserts ordering.
- [ ] No regression on the canvas / build / detail page navigation.

## May-touch

- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:239-282` (CTA row)
- `web/components/skycot/workflows/WorkflowActionBar.tsx` (NEW if extracted)
- `web/tests/workflow-detail-cta-order.test.ts` (NEW)

## Out of scope

- Run-now disable gate (separate brief: `fix-workflow-detail-run-now-disabled-when-source-not-gateway`)
- Build canvas behavior

## Effort

~40-60 LOC + 1 test. ~1 hour.
