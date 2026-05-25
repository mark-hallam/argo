# fix-worker-detail-page-title-and-server-client-tab-helper

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-detail-page-title-and-server-client-tab-helper`.
**Source**: e2e-ux-audit Agent 1 finding F5 (secondary error) + Agent 2 findings F8 + F19 (`docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-teammate.md` + `docs/skycot/ux-audits/2026-05-25-1100-scene-1-and-2-claim-1-and-2.md`).

🟡 **P1 — worker detail tab title is the raw UUID; `isWorkerTabId()` server/client boundary mismatch causes a console error.**

## Root cause

Two coupled defects on the worker-detail route:

1. **`document.title` is the raw UUID**. On `/workers/<id>/overview` the title is `"018f67d0-0000-7000-8000-000000000001 · Overview"` (and on the responsibilities route: `"018f67d0-... · responsibilities"` — note lowercase). The Next.js metadata template is using the URL segment as the title fallback instead of the worker name. The camera captures this in the browser tab + cmd-Tab. `web/app/(app)/workers/[workerId]/layout.tsx` has no `generateMetadata` defined — falls back to the route segment.

2. **`isWorkerTabId()` server/client mismatch**. Agent 1 captured the console error:
   > Attempted to call isWorkerTabId() from the server but isWorkerTabId is on the client
   
   The `[tab]/page.tsx` server route calls the client-marked helper from `WorkerTabs.tsx` (which has `"use client"` at the top). Server components can't import client-only helpers.

3. **Title-case inconsistency on tab segment**. `/workers/<id>/responsibilities` → title shows lowercase `responsibilities`. `/workers/<id>/overview` → title shows capitalized `Overview`. Reveals the placeholder template logic.

## Findings this brief closes

- Agent 1 F5 [P0] secondary error — `isWorkerTabId()` server/client mismatch
- Agent 2 F8 [P1] — Worker-detail page title is the raw UUID
- Agent 2 F19 [P2] — `document.title` template format inconsistent

## Fix recipe

1. **Add `generateMetadata` to the worker layout** at `web/app/(app)/workers/[workerId]/layout.tsx`:
   ```ts
   export async function generateMetadata({ params }: { params: { workerId: string } }) {
     const overview = await fetchWorkerOverview(params.workerId);
     return { title: `${overview.name} · ${capitalize(activeTab(params))}` };
   }
   ```
   Worker name should be fetched server-side; if `fetchWorkerOverview` is client-only, build a server-side variant.

2. **Move `isWorkerTabId()` out of the client component**. At `web/components/skycot/WorkerTabs.tsx`, the helper is currently inside a `"use client"` file. Extract it to a server-safe sibling:
   - Create `web/components/skycot/worker-tabs-shared.ts` (no `"use client"` directive) exporting `WORKER_TAB_IDS`, `isWorkerTabId`, `activeWorkerTab`.
   - Update `WorkerTabs.tsx` to import from the shared file.
   - Update the server `[tab]/page.tsx` to import from the shared file too.

3. **Title-case the tab segment uniformly**. Use a `capitalize()` helper or a `WORKER_TAB_LABELS[tab]` lookup (already defined in `WorkerTabs.tsx`) so the title always shows the label, not the raw URL segment.

## Acceptance criteria

- [ ] Visiting `/workers/<id>/overview` shows tab title "{worker name} · Overview" (NOT the UUID).
- [ ] Visiting `/workers/<id>/responsibilities` shows tab title "{worker name} · Schedules" (capitalized label per `project_vocabulary_duty_vs_schedule`).
- [ ] No console error: "Attempted to call isWorkerTabId() from the server but isWorkerTabId is on the client".
- [ ] Server-only consumers of `WORKER_TAB_IDS` / `isWorkerTabId` import from `worker-tabs-shared.ts`, not `WorkerTabs.tsx`.
- [ ] cmd-Tab / browser-tab view shows recognizable worker name, not UUID.

## May-touch

- `web/app/(app)/workers/[workerId]/layout.tsx` (add `generateMetadata`)
- `web/components/skycot/worker-tabs-shared.ts` (NEW — server-safe helpers + constants)
- `web/components/skycot/WorkerTabs.tsx` (re-export from shared)
- `web/app/(app)/workers/[workerId]/[tab]/page.tsx` (update import)
- `web/lib/api/worker-overview.ts` (server variant if needed)

## Out of scope

- The `charter` contract mismatch (separate brief: `fix-worker-detail-charter-object-vs-string-contract-mismatch`)
- Tab list itself missing Responsibilities/Evals (separate brief: `fix-worker-detail-responsibilities-schedules-tab-end-to-end`)
- Worker personality map deletion (separate brief: `fix-worker-personality-and-level-derive-from-real-data`)

## Effort

~50-120 LOC across layout + tabs + server route. ~1-2 hours.
