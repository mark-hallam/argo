# web-a11y-heading-order-sweep — h1 on remaining 16 pages + heading-order audit

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-a11y-heading-order-sweep`.
**Priority**: P2 — wave-2 a11y audit found 18 pages without a load-bearing `<h1>`. Wave-2 fix landed h1 on / and /team. This brief handles the remaining 16.
**Sizing**: ~100-200 LOC (one h-element swap per page + tests).

## Why this brief exists

From the wave-2 a11y audit:

> 18 heading-structure failures: pages without an h1, h3 used as page
> heading, skipped levels (h1→h3).
>
> Top fixes:
> - /workers/[id]/overview uses h3 page heading
> - /audit uses h3 page heading
> - /workflows uses h3 page heading
> - /cohorts uses h3 page heading
> - /skills uses h3 page heading
> - /settings uses h3 page heading
> - /hire wizard steps skip levels
> - /onboarding steps skip levels
> - /workflows/templates uses h3

WCAG 2.1 AA 1.3.1 (Info and Relationships) requires a single load-bearing
h1 per page + ordered hierarchy.

## Goals

1. Each page has exactly one `<h1>`.
2. No skipped heading levels (h1 → h3 forbidden; h1 → h2 → h3 is fine).
3. Audit: `rg '<h[1-6]' web/app/ --type tsx` per page; verify hierarchy is correct.

## Pages to fix (priority)

1. /workers/[id]/overview + sibling tabs (memory/skills/audit/policies/escalation)
2. /audit + /audit/action/[id] + /audit/replay/[id]
3. /workflows + /workflows/templates + /workflows/[v]/[n] + /workflows/[v]/[n]/build
4. /cohorts + /cohorts/[id]
5. /skills + /skills/[id]
6. /settings + sub-tabs
7. /hire step pages
8. /onboarding step pages

## Steps

1. Run `rg '<h[1-6]' web/app/ --type tsx -B 2 -A 1` to list all heading callsites.
2. For each page with no h1 or wrong-level page heading: swap to h1.
3. For each page with skipped levels: insert intermediate h2 OR demote h3 to h2 so the hierarchy is contiguous.
4. Add a smoke-test that scans each demo-path page rendering and asserts exactly one h1.

## may-touch

- Each page's tsx file (h-tag swap)
- Possibly `<PageHeader>`-style shared components if they're shared across pages
- New: `web/__tests__/a11y-headings.test.ts`

## must-NOT-touch

- `/` (Approvals) and `/team` (already fixed in wave-2)
- Visual styling — h1 should look identical to the current h3 (use `className` to control size)

## Acceptance criteria

- [ ] Every demo-path + product page has exactly one h1.
- [ ] No skipped heading levels anywhere in /web/app/.
- [ ] axe-core scan reports 0 `heading-order` violations.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] Visual diff is null (h1 styled to match the prior h3 visual).

## References

- Audit: `docs/skycot/audit/a11y/2026-05-20-wave-2-a11y.md`
- WCAG 2.1 AA 1.3.1
