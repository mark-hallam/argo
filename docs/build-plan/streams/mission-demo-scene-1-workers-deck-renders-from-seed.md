# mission-demo-scene-1-workers-deck-renders-from-seed

**Owner**: claude.
**Authored against**: origin/main `838e1141a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-demo-scene-1-workers-deck-renders-from-seed`.
**Depends on**: `mission-demo-seed-data-bootstrap` MERGED (delivers the 5 seeded workers).

🔴 **P0 DEMO-GATE — Scene 1 rendering verification (FUNCTION-SIDE per `feedback_no_theater_in_demo` clarified 2026-05-25T08:10Z).**

`v1-mission-mvp-runbook.md` Scene 1 opens on `/workers` with "five workers already hired (Mariah, Peta, Priya, Jamal, Nori). Brand-loud trading-card deck visible." This brief verifies that **once the seed data lands, the /workers page actually renders the 5 trading-card C2-design deck against real backend data** (no mock fixture fallback).

Per the function-vs-dressing rule (`docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`): the rendering of real worker data IS function-side — the camera shows the cards, the operator narrates "five teammates I hired last week", and a viewer challenging "can I see Mariah for myself?" must be answered by real backend rows. The headline KPI numbers ON the cards CAN be dressing fixtures.

## Scope

1. **Pre-flight**: confirm `mission-demo-seed-data-bootstrap` MERGED on origin/main. If not, do NOT claim — wait.
2. **Run the seed**: `argo demo seed --tenant <demo-tenant-id>` against a fresh workspace. Capture full transcript for PR body.
3. **Verify gateway**: `curl http://localhost:8088/api/v1/team` returns 5 `Worker` rows with names Mariah / Peta / Priya / Jamal / Nori.
4. **Start dev server**: `cd web && npm run dev` against the seeded backend (NOT mock-bypass mode — `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` should be UNSET, so the SPA talks to the real backend).
5. **Render check** — open `/workers` in a headless browser (Claude Preview MCP or curl + grep against the rendered HTML):
   - All 5 worker cards present (5 `<article>` or `<a>` elements matching the card pattern)
   - Each card displays the worker's name (Mariah / Peta / Priya / Jamal / Nori)
   - Layout matches the C2-holographic-showcase design locked in memory `project_workers_page_design_locked` (gradient banners, animated shine, level numbers)
   - **No fallback to web/lib/sample-personas.ts** — the page must show "Error: gateway unreachable" if the gateway is down, NOT a mock-card fallback (operator-locked `feedback_no_theater_in_demo`)
6. **Click-through verification**: clicking Mariah's card navigates to `/workers/{id}` and the worker-detail page renders the C2-personality design locked in memory `project_worker_detail_design_locked` against real backend data (trait chips, great-at, escalate-when, signature behaviors).
7. **Fix any rendering gaps** found — if a card doesn't render, find the gateway endpoint mismatch / envelope shape mismatch (similar to fixes that landed in #2241).

## Acceptance criteria

- [ ] `curl http://localhost:8088/api/v1/team` returns 5 rows with all 5 canon names.
- [ ] `curl -s http://localhost:3000/workers` HTML contains all 5 worker names and ≥5 card-pattern elements.
- [ ] PR body has a screenshot of the rendered `/workers` deck.
- [ ] PR body has a screenshot of one worker-detail click-through (Mariah recommended).
- [ ] No mock-fixture fallback path is hit when the gateway is up (verified by grep'ing dev-server log for `sample-personas` import — should not be triggered).
- [ ] Gateway-down behavior: `/workers` shows a clear error state, NOT a silent mock-card fallback (regression test).

## May-touch

- `web/app/workers/page.tsx` (if rendering gaps found)
- `web/lib/api/workers.ts` or wherever the fetch flow lives (if envelope mismatch)
- This brief file (status updates)

## Out of scope

- Re-implementing the C2 card design (it's locked + presumably already JSX per the 25-audit sweep finding)
- Changing the seed CLI (separate brief)
- Touching mock-fixture files (the goal is to ensure they're NOT used at runtime, not to delete them)

## Effort

~30-60 min if everything renders correctly; +30-90 min per rendering gap.
