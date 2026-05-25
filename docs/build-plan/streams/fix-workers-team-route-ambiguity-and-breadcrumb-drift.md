# fix-workers-team-route-ambiguity-and-breadcrumb-drift

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workers-team-route-ambiguity-and-breadcrumb-drift`.
**Source**: e2e-ux-audit Agent 1 finding F16 + Agent 2 findings F5 + F15 (`docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-teammate.md` + `docs/skycot/ux-audits/2026-05-25-1100-scene-1-and-2-claim-1-and-2.md`).

🟡 **P1 — sidebar says Workers, URL flickers to /team, breadcrumb says Team. Internal contradiction on every detail page.**

## Root cause

`web/app/(app)/workers/page.tsx:25` does `router.replace("/team")` whenever workers exist (the "shared-empty-state" pattern documented in the file comment). The cascading consequence:

- Clicking the sidebar "Workers" entry silently puts the user on `/team`
- The URL bar flickers from `/workers` to `/team`
- The sidebar highlights "Team" instead of "Workers"
- On a worker-detail page (`/workers/<id>/overview`), the breadcrumb at `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:100` reads "Team › Mariah" while the left nav highlights "Workers"

The internal contradiction (nav says Workers, breadcrumb says Team, URL says Team) IS the bug. The file comment acknowledges "a dedicated workers list with column-level filters is a separate Track-E brief" — that's still pending. In the meantime, the redirect creates user-visible confusion.

## Findings this brief closes

- Agent 1 F16 [P3] — Sidebar `Workers` nav highlights as "Team" because `/workers` 302s to `/team`
- Agent 2 F5 [P0] — `/workers` is a stub that auto-redirects to `/team`
- Agent 2 F15 [P1] — Worker-detail breadcrumb says "Team" while left-nav highlights "Workers"

## Fix recipe

Pick ONE canonical surface and make all four touch-points (nav, URL, breadcrumb, page title) agree.

**Option A (preferred — `/team` is canonical)**:
1. Update the sidebar entry at `web/components/skycot/nav-items.tsx` from `Workers → /workers` to `Team → /team`.
2. Delete `web/app/(app)/workers/page.tsx` (the redirect stub).
3. Verify `[workerId]/...` routes still resolve under `/workers/...` (they should — different segment).
4. Breadcrumb on worker-detail stays "Team › Mariah" — matches nav + URL.

**Option B (preferred for v1.x — `/workers` is canonical, with real list view)**:
1. Drop the `router.replace("/team")` line; render a real workers-list page at `/workers` (Track-E brief).
2. `/team` becomes a 301 redirect to `/workers`.
3. Breadcrumb on worker-detail becomes "Workers › Mariah".
4. Sidebar stays "Workers → /workers".

**Recommendation**: Option A for v1.0 demo (smaller scope), Option B for v1.x once the Track-E workers-list lands. Author this brief for Option A; reopen as a follow-up when the Track-E list ships.

Either way:
- Update `WorkerLayoutShell.tsx:100` so the breadcrumb's first segment label matches the URL prefix's nav-item label.
- Pin in a Playwright / smoke test: "nav highlight + breadcrumb + URL all agree on the same page".

## Acceptance criteria

- [ ] Sidebar nav, URL bar, breadcrumb, and page-title-tab all agree on the canonical surface name (either all say Workers OR all say Team).
- [ ] No `router.replace` between the canonical and the deprecated route (a 301 redirect with no flicker is acceptable; client-side replace is not).
- [ ] Visiting `/workers/<id>/overview` shows a breadcrumb whose first crumb matches the sidebar's highlighted entry.
- [ ] Playwright / smoke test pinning the agreement.

## May-touch

- `web/app/(app)/workers/page.tsx` (delete OR rewrite)
- `web/app/(app)/team/page.tsx` (rewrite as redirect IF Option B is chosen)
- `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx` (breadcrumb label)
- `web/components/skycot/nav-items.tsx` (sidebar entry)
- `web/middleware.ts` (if a 301 is needed instead of client-side replace)

## Out of scope

- Building the full Track-E workers-list page (separate brief, already in the build-plan)
- Tab strip on worker detail (separate brief: `fix-worker-detail-responsibilities-schedules-tab-end-to-end`)

## Effort

~20-50 LOC + 1 smoke test. ~30-60 min for Option A.
