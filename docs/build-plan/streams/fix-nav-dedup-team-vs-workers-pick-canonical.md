# fix-nav-dedup-team-vs-workers-pick-canonical

**Priority**: P1 — the app sidebar exposes BOTH "Team" (`/team`) and "Workers" (`/workers`) as separate entries for what is effectively the same roster surface. A buyer reads it as two different features; clicking between them flickers the URL bar. Confirmed live on the 2026-05-29 full-site walkthrough.
**Source**: 2026-05-29 item-1 audit re-author (replaces the closed, oversized PR #2494 — 551 LOC / 12 files with a stale "redirect-only stub" premise). This brief is the small, focused version.
**Authored against**: origin/main `93597ebff` on 2026-05-29
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-nav-dedup-team-vs-workers-pick-canonical`
**Authored by**: claude conductor (interactive session)

## The issue (verified live)

`web/components/skycot/nav-items.tsx:24-25` declares two adjacent nav entries:

```tsx
{ href: "/team", label: "Team", icon: <Users /> },
{ href: "/workers", label: "Workers", icon: <Workflow /> },
```

Both lead to the same roster surface (`/workers` historically redirected to `/team`). Two entries for one concept is confusing in the sidebar and on camera.

**Scope-narrowing vs the closed PR #2494:** do NOT rewrite `workers/page.tsx` (it already renders the empty-state inline on current main — the closed PR's 478-line rewrite was built on a stale premise). This brief is *only* the nav-list dedup + a redirect so neither URL 404s.

## Scope / fix

1. **Pick one canonical sidebar entry.** Keep a single roster entry. Recommended: keep `Team` (matches the page header "5 teammates, one team" + the operator vocabulary; `/team` is already the rendered page). Drop the `Workers` sidebar entry. (If the operator prefers "Workers" as the label, keep that one instead — but ship exactly ONE.)
2. **Keep both URLs working.** `/workers` should still resolve (existing deep-links, the `/workers/<id>/overview` detail route, and `?just-hired=1`): leave the `/workers` route in place / redirecting to the canonical roster; only the duplicate *sidebar entry* is removed. Do not break `/workers/<id>/overview`.
3. **Update the nav-order pin test** to expect the single entry.

## Acceptance

- [ ] Sidebar shows exactly ONE roster entry (no duplicate Team/Workers pair).
- [ ] `/team`, `/workers`, and `/workers/<id>/overview` all still resolve (no new 404; worker-detail deep-link intact).
- [ ] `web/tests/nav-sidebar-order.test.ts` (or the relevant nav-order pin) updated + green.
- [ ] `pnpm exec tsc --noEmit` + `pnpm exec next lint` clean for touched files.
- [ ] Diff is small — `nav-items.tsx` + the nav-order test + (if needed) a redirect entry. NOT a `workers/page.tsx` rewrite.

## Out of scope

- Any rewrite of `workers/page.tsx` (that was the closed #2494's stale premise).
- Worker-detail page behavior.

## May-touch

- `web/components/skycot/nav-items.tsx`
- `web/tests/nav-sidebar-order.test.ts` (or the nav-order pin file)
- `web/next.config.ts` redirects (only if a redirect is needed to keep `/workers` resolving)

## Operator authority

Web-only, ≤5 files — eligible for the web-only admin-merge lane with vitest green in the PR body per the 2026-05-26 operator authorization.
