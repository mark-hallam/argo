# rebase-fix-approvals-drop-hardcoded-action-id-map

**Authored against:** origin/main `a5df90c41` on 2026-05-29 by claude deep-triage-agent-3.
**Validate before claim:** `bash scripts/check-brief-freshness.sh rebase-fix-approvals-drop-hardcoded-action-id-map`

## What

Re-implement the original brief `streams/fix-approvals-drop-hardcoded-action-id-map.md` against current `origin/main` HEAD.

The original PR #2352 (codex) was authored against an older `origin/main` and conflicted irretrievably with main's evolution on the following files:

- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` — 5 merged PRs since #2352 was opened (#2629 Active reviewers KPI derivation, #2570 content column flex-1, #2557 canonical 5-KPI strip, #2471 row grid breakpoints, #2414 inline approve/refuse). File is currently 387 LOC.
- `web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx` — 4 merged PRs since.
- `web/lib/sample-approvals.ts` — 4 merged PRs since.
- `web/lib/api/approvals.ts` — 1 merged PR since.

Current shape on main (verify with `cat` before authoring fix):

- `ApprovalsOpsDashboard.tsx` still carries the hardcoded `SAMPLE_APPROVAL_ACTION_IDS: Record<string, string>` map (lines 32-41) — exactly the surface the original brief targeted. The bug is still load-bearing.
- `web/lib/api/approvals.ts` already maps `action_id` → `actionId` on the live path (lines 151-175); the live wiring is already correct. Only the seeded sample side needs the actionId backfill + the read-only-row branch when actionId is missing.
- `web/components/skycot/approvals/ApprovalRow.tsx` does NOT exist on main — the original PR extracted it as a new file. Re-introducing it is fine, but verify it doesn't collide with current ApprovalsOpsDashboard.tsx row rendering structure (which has been actively iterated for KPI strip + inline approve/refuse + breakpoint grid).

## Acceptance

Same as the original brief: drop `SAMPLE_APPROVAL_ACTION_IDS`, backfill seeded approvals in `web/lib/sample-approvals.ts` with explicit `actionId` fields, rely on `item.actionId` (already mapped from `action_id` for live rows), and render approvals without action ids as read-only with a dev `console.warn`.

Closes scene-6 F8 from UX e2e audit.

## Sensible deviations from original

- The 5-KPI strip + inline approve/refuse + breakpoint-grid shape on main MUST be preserved — do not revert any of those merges. Layer the actionId fix onto the current dashboard shape.
- If the ApprovalRow extraction would push `ApprovalsOpsDashboard.tsx` over the 500-LOC ceiling, the extraction is justified; otherwise inline a tiny `renderRow(item)` helper inside the dashboard and skip the new component.

## May-touch

- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx`
- `web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx`
- `web/lib/sample-approvals.ts`
- `web/lib/api/approvals.ts` (only if the actionId mapping needs adjustment; current shape is already correct)
- new `web/components/skycot/approvals/ApprovalRow.tsx` + `.test.tsx` ONLY if extraction justifies the new file

## Verify

- `pnpm --dir web exec vitest run components/skycot/approvals lib/api/approvals.test.ts`
- `pnpm --dir web typecheck`
- `bash scripts/codex-pre-push.sh`
