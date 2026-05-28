# rebase-fix-demo-banner-copy-branch-on-data-source-or-suppress

**Authored against:** origin/main `a5df90c41` on 2026-05-29 by claude deep-triage-agent-3.
**Validate before claim:** `bash scripts/check-brief-freshness.sh rebase-fix-demo-banner-copy-branch-on-data-source-or-suppress`

## What

Re-implement the original brief `streams/fix-demo-banner-copy-branch-on-data-source-or-suppress.md` against current `origin/main` HEAD.

The original PR #2427 (claude) was authored against an older `origin/main` and conflicts irretrievably with main's evolution on:

- `web/components/skycot/DemoBanner.tsx` — 4 merged PRs since #2427 was opened (#2504 wizard polish — pathname-based suppression for /onboarding/* added, #2559 mock-fixture-mode console.warn added, #2448 X dismiss z-index above FreezeButton).
- `web/lib/api/team.ts` / `web/lib/api/approvals.ts` / `web/lib/api/audit.ts` — 2-3 merged PRs since touching the fetcher signatures the PR's `data-source-context` infra would hook into.

Current shape on main (verify with `cat` before authoring fix):

- `DemoBanner.tsx` still renders the single hardcoded "showing sample data" string (line 74) — the original bug target is still load-bearing.
- DemoBanner now has pathname suppression for `/onboarding/*` (line 46) + recordingModeReadinessWarning console.warn (line 34-38) — keep both, layer the data-source-aware copy on top.
- `lib/api/*.ts` fetchers do NOT publish a data-source signal; the new `web/lib/skycot/data-source-context.tsx` store from the original PR is still net-additive.
- localStorage → sessionStorage dismissal-persistence change is unaffected by the recent churn; preserve it.

## Acceptance

Same as the original brief:
- Add `web/lib/skycot/data-source-context.tsx` subscribable store.
- `lib/api/team.ts` / `lib/api/approvals.ts` / `lib/api/audit.ts` publish source ('live' | 'mock-offline' | 'mock-bypass') on return.
- DemoBanner reads via `useSyncExternalStore` and branches copy on source (4 branches as specified).
- Add `NEXT_PUBLIC_HIDE_DEMO_BANNER=1` env-var to suppress banner entirely (documented in `docs/skycot/demos/v1-mission-mvp-runbook.md`).
- Dismissal persistence flips from localStorage → sessionStorage.

Closes scene-6 F8 [P1] + scene-4-and-5 F13 [P3].

## Sensible deviations

- Preserve the recent pathname-suppression for `/onboarding/*` (lines 46 on main) — the `useEffect` ordering must keep that early return BEFORE the data-source-context subscription, or the subscription will fire on onboarding pages it should ignore.
- Preserve the `recordingModeReadinessWarning()` console.warn from #2559.
- The 4-fetcher publish-call is additive — verify each fetcher's existing return shape is unchanged for tests that pin signatures.

## May-touch

- new `web/lib/skycot/data-source-context.tsx`
- `web/components/skycot/DemoBanner.tsx`
- `web/components/skycot/__tests__/DemoBanner.test.tsx`
- `web/lib/api/team.ts` + `web/lib/api/approvals.ts` + `web/lib/api/audit.ts` (publish-call only)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (pre-flight env-var note)

## Verify

- `pnpm --dir web exec vitest run components/skycot/__tests__/DemoBanner.test.tsx lib/api/team.test.ts lib/api/approvals.test.ts lib/api/audit.test.ts`
- `pnpm --dir web exec tsc --noEmit` (tolerate pre-existing errors in `ProvenanceBlock.tsx` + `use-provision.ts`)
- `pnpm --dir web exec next lint`
