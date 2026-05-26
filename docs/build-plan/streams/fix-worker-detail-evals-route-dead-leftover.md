# fix-worker-detail-evals-route-dead-leftover

**Owner**: claude or codex.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-detail-evals-route-dead-leftover`.
**Source**: ux-e2e-audit-T2309MEL `scene-1-and-2-claim-1-and-2.md` F12 [P3].

🟢 **P3 — `/workers/[workerId]/evals/page.tsx` still exists alongside `/activity/page.tsx`. Likely leftover from before the Activity tab was renamed/replaced. The tab nav no longer surfaces "Evals", but the route is still reachable by typing the URL and ships with the SPA bundle. Dead code with a tiny attack surface.**

## Root cause

The worker-detail tab list at `web/components/skycot/WorkerTabs.tsx` (or `web/app/(app)/workers/[workerId]/layout.tsx`) currently exposes: Overview / Activity / Memory / Skills / Trust / Escalation / Settings. The audit confirms no "Evals" tab is rendered. But the route file `web/app/(app)/workers/[workerId]/evals/page.tsx` exists — likely a survivor from when the Activity tab was named Evals.

The audit's note: "stale tab; check if anything links to it." A direct URL `/workers/<id>/evals` either renders the orphaned page (worse — confuses anyone who bookmarked it) or 404s through Next.js (fine).

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F12 [P3] — `/workers/[workerId]/evals/page.tsx` exists alongside `/activity/page.tsx`; dead route

## Fix recipe

1. **Audit the link graph.**
   ```bash
   rg "/evals\b" web/
   rg "workers/.*evals\|/workers/\[workerId\]/evals" web/
   ```
   If zero non-route-file matches → the route file is fully dead.

2. **Delete or redirect.**
   - **Option A (preferred — delete)**: `git rm web/app/(app)/workers/[workerId]/evals/page.tsx` (and any sibling `layout.tsx`/`loading.tsx`)
   - **Option B (redirect for bookmarks)**: keep the file but make it a server-side redirect to `/workers/[workerId]/activity` so any external bookmark still lands somewhere sensible:
     ```tsx
     import { redirect } from "next/navigation";
     export default function EvalsRedirect({ params }: { params: { workerId: string } }) {
       redirect(`/workers/${params.workerId}/activity`);
     }
     ```

3. **Verify no test references.** `rg 'workers.*evals' web/**/__tests__` should be zero.

4. **Confirm via curl post-fix.** `curl -I http://localhost:3000/workers/018f67d0-0000-7000-8000-000000000001/evals` returns 404 (Option A) or 308 (Option B).

## Acceptance criteria

- [ ] `rg "/evals\b" web/` returns no non-route-file matches outside the redirect (Option B) or no matches at all (Option A)
- [ ] Direct URL navigation to `/workers/<id>/evals` returns 404 or redirects to /activity
- [ ] No vitest spec references the evals route
- [ ] `pnpm --dir web exec tsc --noEmit` green
- [ ] `pnpm --dir web exec next build` green

## May-touch

- `web/app/(app)/workers/[workerId]/evals/page.tsx` (~5 LOC if redirect; deleted if Option A)
- `web/app/(app)/workers/[workerId]/evals/layout.tsx` (verify exists; delete or update)
- Any test referencing the route (likely none)

## Out of scope

- Adding an "Evals" tab back (not requested; the renaming to Activity was deliberate)
- Refactoring the worker-detail tab system (covered by `fix-worker-detail-responsibilities-schedules-tab-end-to-end` + `fix-worker-dynamic-tab-page-dead-code-and-personality-silent-fallback`)

## Effort

XS. ~5 LOC (delete or redirect). ~10 min. P3 cleanup — does not affect the v1.0 demo recording but removes a tripwire for future agents.
