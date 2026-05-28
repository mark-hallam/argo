# rebase-fix-workflows-build-vertical-picker-add-marketplace

Re-implement the original brief
`streams/fix-workflows-build-vertical-picker-add-marketplace.md` against
current `origin/main` HEAD.

## Original PR

PR #2537 (closed for re-author) — `fix: workflows/build picker — add
marketplace + source from /api/v1/verticals`, created 2026-05-27 against
an older `main`. Conflicted on:

- `web/app/(app)/workflows/build/page.tsx` — main has been converted to a
  client component by #2470 (`99dff03c9`) which uses the existing
  `useVerticals()` hook from `@/lib/api/verticals.ts` and a 7-entry
  `FALLBACK_VERTICALS` list (sales/support/gtm-engineer/people-ops/
  finance/marketing/compliance). The PR's new `useBuildVerticals()` from
  a brand-new `verticals-build.ts` is a parallel data-layer that
  conflicts with what already shipped.
- `web/lib/api/verticals-build.ts` — would be a new file at 221 LOC.
  Main already has `web/lib/api/verticals.ts` (332 LOC, catalog
  endpoint) that the page.tsx now consumes. A second
  `verticals-build.ts` would create parallel abstractions (forbidden by
  `CLAUDE.md` "No parallel abstractions" + Phase A canonicalization).

## Authored against: origin/main `a5df90c41` on 2026-05-29

## Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-workflows-build-vertical-picker-add-marketplace

## Bug is still load-bearing

Confirmed on `a5df90c41` — `FALLBACK_VERTICALS` in
`web/app/(app)/workflows/build/page.tsx` lines 15-23 has 7 entries:
sales, support, gtm-engineer, people-ops, finance, marketing,
compliance. **`marketplace` is missing.** The canonical Scene-5 demo
workflow `customer_research_followup` is seeded under
`marketplace` (`crates/argo-demo-seed/src/workflow.rs:26`), so an
operator cannot author a new marketplace workflow from the canvas
entry page.

The gateway side is correct — `useVerticals()` would surface
marketplace when the gateway returns it — but the fallback when the
gateway returns nothing (or is unreachable in demo-mode) doesn't
include marketplace, so the operator never sees a tile for it.

## Acceptance (same as original brief)

- `marketplace` tile renders on `/workflows/build` even on a fresh
  tenant with no gateway data.
- The picker continues to consume `useVerticals()` (the canonical
  catalog hook) — DO NOT introduce a parallel data-layer.
- The 7-entry fallback grows to 8 (adds `marketplace`) OR the page
  unions gateway response with fallback such that marketplace is
  always present.
- Regression test at
  `web/app/(app)/workflows/build/__tests__/vertical-picker-marketplace.test.tsx`
  asserts (a) `marketplace` is in the rendered tile grid, (b)
  gateway-only verticals from a non-empty response append after the
  canonical entries.
- The marketplace tile routes to `/workflows/marketplace/untitled/build`
  (or the canonical workflow-canvas route the rest of the picker uses).

## Shape of main on touched files

```ts
// web/app/(app)/workflows/build/page.tsx (current, lines 15-23)
const FALLBACK_VERTICALS: ReadonlyArray<readonly [string, string, string]> = [
  ["sales", "Sales", "Lead follow-up, research, qualification, and CRM handoffs."],
  ["support", "Support", "Ticket triage, customer follow-up, and escalation workflows."],
  ["gtm-engineer", "GTM", "Campaign operations, launch checklists, and market scans."],
  ["people-ops", "People Ops", "Candidate screens, onboarding checklists, and policy reviews."],
  ["finance", "Finance", "Invoice review, approval routing, and month-end controls."],
  ["marketing", "Marketing", "Content drafts, social posts, and competitive scans with brand-voice checks."],
  ["compliance", "Compliance", "Policy reviews, audit prep, and regulatory deadline tracking."],
];
```

The page already does the right thing structurally:

```ts
const useGateway = source === "gateway" && verticals.length > 0;
const entries = useGateway ? <gateway-list> : FALLBACK_VERTICALS;
```

The minimal fix: add a `["marketplace", "Marketplace",
"<description>"]` row to `FALLBACK_VERTICALS` and ensure the gateway
path also surfaces marketplace when present.

## Notes for the worker

- DO NOT create `web/lib/api/verticals-build.ts` — it would be a
  parallel abstraction to `web/lib/api/verticals.ts`. Add marketplace
  to the existing fallback list inline in `page.tsx`.
- If marketplace needs to come from the gateway response too, ensure
  the existing `useVerticals()` hook surfaces it (audit
  `web/lib/api/verticals.ts` against the gateway's `/verticals`
  response — marketplace should be a first-class vertical).
- Keep the file under the 500-LOC ceiling (`page.tsx` currently sits
  around 100 LOC; this should add ~10 LOC).
