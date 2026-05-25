# fix-team-page-empty-state-despite-gateway-worker

**Owner**: claude.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-team-page-empty-state-despite-gateway-worker`.
**Source**: e2e-ux-audit Agent (scene-1-and-2-claim-1-and-2) finding F2 (`docs/skycot/ux-audits/2026-05-25-2309/scene-1-and-2-claim-1-and-2.md`).

🔴 **P0 — `/team` renders the "Hire your first AI teammate" empty-state grid even when the gateway returns Maya. The C2 holographic trading-card deck never paints. Scene 1's first beat is unrecordable.**

## Root cause

Captured live at `http://127.0.0.1:60894/team` while the user has a fresh `__Host-session` cookie minted via `/api/auth/demo-mint`:

- `fetch('/api/v1/team', { credentials: 'include' })` from the same page returns `{"total_count":1, "workers":[Maya]}` → 200.
- Yet the rendered DOM shows page header "Team" + "0 working" subtitle + the `<TeamEmptyState>` "Hire your first AI teammate" CTA grid.

Two candidate root causes:
1. `useTeam().data?.workers` is briefly `[]` on first render and the empty state captures it BEFORE the query settles.
2. `fetchTeam()` is treating the envelope as empty when no auth-bypass flag is set — possibly mis-parsing the `{workers, total_count}` envelope.

Touch sites: `web/lib/api/team.ts:54-78` (envelope parsing) + `web/app/(app)/team/page.tsx:171-186` (render branches). Compounded by the broader gateway CORS gap (separate finding) but reproducible even when the response curl-confirms a 200.

This is the most camera-visible defect in Scene 1: the operator clicks the SKYCOT logo → lands on `/team` → sees "0 working" instead of "5 working" + the trading-card deck.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F2 [P0] — `/team` renders empty-state grid despite the gateway returning Maya

## Fix recipe

1. **Add a `dataSource` chip to the page header**. Right next to "0 working", render a small chip `data-source="gateway" | "mock-offline" | "loading"`. This is for operator-side diagnosis during dev — analogous to how `/workflows` shows `WORKFLOWS · LIVE | MOCK`. Make it conditional on `process.env.NODE_ENV !== 'production'` so it doesn't ship in the demo MP4 if the operator forgets to remove it.

2. **Trace the envelope**. In `web/lib/api/team.ts:54-78`, log (or trace via React DevTools) what the parsed response looks like immediately after the gateway returns. Confirm whether `data.workers` is correctly extracted vs being undefined.

3. **Render-branch fix in `web/app/(app)/team/page.tsx:171-186`**: do NOT render `<TeamEmptyState>` while `teamQuery.isLoading || teamQuery.isError`. Only show empty state when `teamQuery.isSuccess && (teamQuery.data?.workers?.length ?? 0) === 0`. Today's logic appears to render the empty state on any falsy condition.

4. **Pin with a unit test** at `web/lib/api/team.test.ts`:
   - Mock fetch returns `{ workers: [maya], total_count: 1 }` → assert `useTeam().data.workers.length === 1`.
   - Mock fetch returns `{ workers: [], total_count: 0 }` → assert empty.
   - Mock fetch returns network error → assert `isError = true`, NOT `data.workers = []`.

5. **Pin with a Playwright spec** at `web/tests/team-page-shows-real-workers.spec.ts`: mock the gateway to return 5 workers, navigate `/team`, assert 5 trading-card-deck cards render + no `<TeamEmptyState>`.

## Acceptance criteria

- [ ] When `/api/v1/team` returns at least 1 worker, `/team` renders the C2 holographic trading-card deck.
- [ ] The "X working" subtitle correctly reflects `total_count`.
- [ ] Empty state ONLY renders on success-with-zero-workers, never on loading/error.
- [ ] `dataSource` diagnostic chip visible in non-prod builds.
- [ ] Unit test + Playwright spec pass.

## May-touch

- `web/app/(app)/team/page.tsx`
- `web/lib/api/team.ts`
- `web/lib/hooks/use-team.ts`
- `web/lib/api/team.test.ts` (NEW or extend)
- `web/tests/team-page-shows-real-workers.spec.ts` (NEW)

## Out of scope

- Gateway CORS fix (separate brief `mission-gateway-cors-preflight`)
- The 5-persona seed (separate brief `fix-demo-seed-cli-for-real-roster`)
- `/workers` redirect-to-`/team` route ambiguity (separate brief `fix-workers-team-route-ambiguity-and-breadcrumb-drift`)

## Effort

~60-120 LOC + 1 unit test + 1 Playwright spec. ~2 hours. Demo-recording-blocker.
