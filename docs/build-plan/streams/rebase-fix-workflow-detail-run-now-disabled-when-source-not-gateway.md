# rebase-fix-workflow-detail-run-now-disabled-when-source-not-gateway

Authored against: origin/main `0c7cff50f5` on 2026-05-29
Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-workflow-detail-run-now-disabled-when-source-not-gateway

## Why this brief exists

PR #2358 (608 LOC, codex-authored 2026-05-26) targeted the same load-bearing bug — `/workflows/[verticalId]/[name]` shows `Run now` disabled when `source !== "gateway"` (because the workflow is a seed-source row, not a published gateway row), so the demo-recording user cannot trigger a run from seed workflows.

The original PR's approach — `publishRawToml` helper + auto-publish-then-run flow — STILL applies to current main, but it conflicted irretrievably with main's evolution in two areas:

1. `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` is now 481 LOC after PR #2543 (toast-error layer for run-now failures) + PR #2574 (workflow version shape drift between card and detail) + PR #2469 (empty state for mock-offline). The auto-publish flow has to interleave with the new `describeRunNowError()` path and the `lastRunError` state machine (lines 277, 437, 458 of `page.tsx`).
2. `web/lib/api/workflows.ts` evolved through #2608 (author-type discriminant chip) + #2440 (seeded workflow on `/api/v1/*` endpoints) + #2409 (workflow builder regression cluster). `publishRawToml` has to coexist with the new endpoints.

PR #2358 was closed for re-author.

## Source of truth

- Original brief: `docs/build-plan/streams/fix-workflow-detail-run-now-disabled-when-source-not-gateway.md`
- Closed PR: #2358
- Current main shape:
  - `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` lines 109 (`const isLive = source === "gateway"`), 270 (`disabled={running || !isLive}`), 272 (tooltip explains why disabled).
  - `web/lib/api/workflows.ts` already has rich publish helpers but no raw-TOML promote-to-head shortcut.

## Acceptance

Same as original brief — `Run now` button is enabled for seed-source workflows in demo mode; clicking it auto-publishes the seed TOML to the demo tenant and triggers a run; existing `describeRunNowError` toast/chip layer keeps surfacing failures.

Re-implement against current `origin/main` HEAD. Coordinate with the existing `lastRunError` state machine — do not regress the toast / chip behaviour that #2543 shipped.

## Boundary

- May touch: same files as original brief plus the test file `web/tests/workflows-run-now.test.ts`.
- Must not regress the 7 tests added by #2543 in `WorkflowDetail.test.tsx`.

## Verification

- `pnpm exec vitest run lib/api/workflows.test.ts 'app/(app)/workflows/[verticalId]/[name]/WorkflowDetail.test.tsx' tests/workflows-run-now.test.ts` (extend with new seed publish-then-run case)
- `pnpm exec tsc --noEmit`
- `bash scripts/codex-pre-push.sh` (if codex-claimed) or `bash scripts/claude-pre-push.sh`
