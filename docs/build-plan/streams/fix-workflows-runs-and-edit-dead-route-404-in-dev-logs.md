# fix-workflows-runs-and-edit-dead-route-404-in-dev-logs

**Owner**: claude.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflows-runs-and-edit-dead-route-404-in-dev-logs`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/scene-4-and-5-claim-4-and-5.md` F11.

🟦 **P2 — Dev-server logs show recurring 404s for `/workflows/runs` (74ms), `/workflows/edit` (59ms), and `/skills-canvas` (63ms) — routes that don't exist but are linked from somewhere in the SPA. Pre-record QA who opens DevTools sees a stream of 404 errors. Camera unlikely to see, but a curious viewer of the demo MP4 might catch the network panel.**

## Root cause

Dev-server compile log emits 404 hits for three SPA routes:
```
GET /workflows/runs 404 in 74ms
GET /workflows/edit 404 in 59ms
GET /skills-canvas 404 in 63ms
```

`web/app/(app)/workflows/runs/page.tsx` — does not exist.
`web/app/(app)/workflows/edit/page.tsx` — does not exist.
`web/app/(app)/skills-canvas/page.tsx` — does not exist (the nested `/workers/[workerId]/skills/[skillId]/canvas` does).

Something in the SPA is linking to these dead routes. Candidates:
- Old hash links in `/audit` row drill-ins or `/cohorts` cohort cards
- Stale `Link` href in a sidebar / breadcrumb component
- Test fixtures that leak into prod
- 404 prefetches by Next.js for prerender

The `/skills-canvas` flat route is covered by the existing brief `fix-skills-canvas-flat-route-404-or-redirect.md` (which proposes a redirect). This brief covers the OTHER two routes (`/workflows/runs` and `/workflows/edit`).

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F11 [P2] — `/workflows/runs` + `/workflows/edit` + `/skills-canvas` dev-log 404s

## Fix recipe

1. **Grep for callers.**
   ```bash
   grep -rE 'href="/workflows/runs"|href="/workflows/edit"|to="/workflows/(runs|edit)"|push.."/workflows/(runs|edit)' web/
   ```
   Identify every component / route that links to these URLs.

2. **For each caller, decide:**
   - **Stale link** (old prototype reference): delete or redirect to the most-relevant current page (e.g. `/workflows/runs` → `/audit?range=7d&filter=workflow.execute`; `/workflows/edit` → `/workflows/build` or `/workflows/[v]/[n]/build`).
   - **Intentional but unbuilt**: create a minimal page that either (a) implements the surface, OR (b) renders a "coming soon" with brand-voiced copy (matching the worker-tab placeholder pattern).

3. **Possible quick fix paths:**
   - `/workflows/runs` — likely "show all workflow run history". The data lives in audit rows with `action_type IN ('workflow.execute', 'workflow.completed')`. Either build the page (sibling brief if substantial) or redirect to `/audit?...`.
   - `/workflows/edit` — possibly meant as a generic editor. Probably stale; redirect to `/workflows/build`.
   - `/skills-canvas` — covered separately by `fix-skills-canvas-flat-route-404-or-redirect.md`. Out of scope for this brief.

4. **Stub routes (if needed).** For each route that should exist as a navigational target but isn't built:
   - Create `web/app/(app)/<route>/page.tsx` with a `<SoonShell>` body (per `fix-worker-tabs-placeholder-bodies-leak-brief-ids-e4.md`).
   - Or add a `next.config.ts` redirect entry.

5. **CI guard.** Add a smoke test to `scripts/check-spa-routes.sh` (NEW or extend existing) that curls every `href="..."` in the app's route tree and asserts no 404s.

## Acceptance criteria

- [ ] Dev server logs do NOT emit 404 hits for `/workflows/runs` or `/workflows/edit` on a normal click-through of /audit, /workflows, /cohorts, /skills.
- [ ] Grep confirms no `href="/workflows/runs"` or `href="/workflows/edit"` strings remain in the SPA (except in tests where intentional).
- [ ] CI guard catches new dead links in PR diffs.

## May-touch

- Whatever component currently links to the dead routes (TBD by grep)
- `web/app/(app)/workflows/runs/page.tsx` (NEW — option a)
- `web/app/(app)/workflows/edit/page.tsx` (NEW — option a)
- `next.config.ts` (redirect entry — option b)
- `scripts/check-spa-routes.sh` (NEW or extend)

## Out of scope

- `/skills-canvas` flat route (covered by `fix-skills-canvas-flat-route-404-or-redirect.md`)
- Major workflow-runs history page implementation (file separately if substantial)
- `/docs/*` sub-page 404s (covered by `fix-docs-sub-routes-all-404.md`)

## Effort

S — depends on grep findings. Typical: ~30-80 LOC + redirects + CI guard. ~45-90 min.
