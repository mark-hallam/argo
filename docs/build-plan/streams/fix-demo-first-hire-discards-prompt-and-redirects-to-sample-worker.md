# fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/onboarding-hire-an-ai-teammate.md` F1 + F2 + F7.

🔴 **P0 — Demo-mode first-hire silently discards the user's prompt and redirects them to a fixed sample worker keyed off `verticalId`. The user types "A marketing intern named Maya …" and lands on `/workers/<nori-uuid>/overview` showing "How Nori works". The roster on `/workers` doesn't grow — same 5 sample tiles before and after onboarding. This is function-side fabrication the camera will see.**

## Root cause

`web/lib/onboarding/use-first-hire.ts:120-154` defines `VERTICAL_TO_SAMPLE_WORKER` (4-entry map: marketplace-seller → Mariah, recruiter → Priya, customer-support → Jamal, marketing-intern → Nori) plus `buildDemoFirstHireResult(verticalId)` which returns a hardcoded `{id, name}`. The `firstHire(req)` mutation at line 152 checks `isDemoMode()` and short-circuits to that result without ever sending `req.description` to the gateway, without appending the synthesised hire to `appendDemoWorker()` (the demo-roster store that `/hire` and `/team` already use), and without writing the user's typed name anywhere.

Compounding effect: `web/app/(app)/workers/page.tsx:25-27` immediately `router.replace("/team")` whenever `workers.length > 0` (always true — `SAMPLE_TEAM` has 5). `/team` reads `useTeam()` which in auth-bypass mode reads `SAMPLE_TEAM + listDemoWorkers()`. Because the wizard never calls `appendDemoWorker(...)`, the new hire is never in `listDemoWorkers()` and the roster doesn't grow.

Asymmetry that confirms the diagnosis: the provider-key step (`use-provider-key.ts:56-59` + `connect-provider/page.tsx:200-202`) honestly mocks with the green banner **"Mock provider connected — workers run with simulated responses for the demo."** The first-hire step does no such honest-mocking — it pretends a hire happened.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F1 [P0] — First-hire prompt silently discarded in demo mode; user redirected to fixed sample worker
- onboarding-hire-an-ai-teammate F2 [P0] — Hired worker does not appear on `/workers`
- onboarding-hire-an-ai-teammate F7 [P1] — Provider-key honestly-mocked but first-hire silently-mocked

## Fix recipe

1. **Append the synthesised hire to the demo roster.** In `web/lib/onboarding/use-first-hire.ts:120-154`, before the redirect, build a `SampleWorker` + `WorkerOverview` from the user's `req.description` (parse name via the same `name_from_description` regex the gateway uses, or accept the new optional `name` field from `fix-onboarding-first-hire-name-parser-prefers-named-pattern`). Call `appendDemoWorker({ worker, overview })` from `web/lib/api/demo-roster-store.ts`. Use the user's typed description as the worker's tagline + persona.bio.
2. **Stop redirecting `/workers` to `/team` when the user just hired.** In `web/app/(app)/workers/page.tsx:25-27`, gate the `router.replace("/team")` on `searchParams.get('just-hired') !== '1'`. The first-hire redirect at `first-hire/page.tsx:55-57` then routes to `/workers?just-hired=1&new=<id>` so the page renders the workers grid (with the highlight scroll target) rather than bouncing to `/team`.
3. **Add an honest "investor-demo · sample worker" banner on the worker overview after demo first-hire.** Mirroring the provider-key green banner, render a one-line dismissable affordance on `/workers/<id>/overview` when the worker was created via the demo short-circuit. Suggested copy: "**Demo · staged hire** — real provisioning runs against a live workspace. Your description is saved as the persona bio." Source the flag from a query param `?demo-hire=1` set by the redirect in step 2.
4. **(Optional) Drop the redirect-to-existing-sample-worker behaviour entirely** when step 1 lands. The `VERTICAL_TO_SAMPLE_WORKER` map becomes dead code — the user's hire IS the new worker, regardless of vertical. Keep the fallback only for the path where the user has no description (empty submit), which today shouldn't be reachable but is worth defending.

## Acceptance criteria

- [ ] In demo mode, typing "Maya, a marketing intern who drafts blog posts" and submitting the first-hire form results in a worker named "Maya" on `/workers` (or `/team`) — NOT a redirect to the canonical Nori card.
- [ ] After demo first-hire, `/workers` renders 6 cards (5 SAMPLE_TEAM + 1 new), and the new card is visually highlighted (the existing `?new=<id>` query param wire is reused).
- [ ] `/workers/<new-id>/overview` renders the user's typed description as the worker's tagline + persona bio.
- [ ] A dismissable demo-mode banner is visible on the worker overview reading "Demo · staged hire — real provisioning runs against a live workspace."
- [ ] In non-demo mode the path is unchanged (no regression to the real `/api/v1/onboarding/first-hire` gateway call).
- [ ] Vitest pin: in demo mode, calling `firstHire({verticalId: 'marketing-intern', description: 'Maya, a marketing intern'})` appends a `Maya` entry to `listDemoWorkers()`.

## May-touch

- `web/lib/onboarding/use-first-hire.ts` (replace short-circuit body; keep `isDemoMode()` gate)
- `web/lib/api/demo-roster-store.ts` (no shape change; reuse `appendDemoWorker`)
- `web/app/(app)/workers/page.tsx` (gate the `/team` redirect on `?just-hired=1`)
- `web/app/(setup)/onboarding/first-hire/page.tsx` (update post-submit redirect URL)
- `web/app/(app)/workers/[workerId]/overview/page.tsx` (demo-hire banner)
- `web/lib/onboarding/__tests__/use-first-hire.test.ts` (new pin)

## Out of scope

- Name parsing (covered by `fix-onboarding-first-hire-name-parser-prefers-named-pattern.md`)
- Nori/Sam personality coverage (covered by `fix-worker-personality-coverage-extend-to-nori.md` + `fix-peta-personality-trait-level-lookup-coverage.md`)
- Genesis-plan invocation on the real path (covered by `fix-onboarding-first-hire-invoke-genesis-plan.md`)
- `/onboarding/done` celebration page (separate brief: `fix-onboarding-add-done-page-with-meet-your-teammate-handoff`)

## Effort

M — ~100-150 LOC across SPA + tests. ~2-3 hours. P0 because Scene 3 of the demo runbook lands on a sample-worker profile with the wrong name + sterile personality if not fixed.
