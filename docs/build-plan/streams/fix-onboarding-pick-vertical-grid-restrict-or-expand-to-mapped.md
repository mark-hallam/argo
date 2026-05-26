# fix-onboarding-pick-vertical-grid-restrict-or-expand-to-mapped

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-pick-vertical-grid-restrict-or-expand-to-mapped`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/onboarding-hire-an-ai-teammate.md` F10.

🟡 **P2 — `/onboarding/pick-vertical` renders 7 vertical cards (Marketplace Seller, Recruiter, Customer Support, Marketing Intern, GTM Engineer, Sales SDR, Accounts Receivable). The demo-mode first-hire short-circuit only maps 4 of those to sample workers; the 3 unmapped verticals (GTM Engineer, Sales SDR, Accounts Receivable) fall through to `DEMO_FALLBACK_WORKER = Mariah`. Picking "Sales SDR" hires Mariah-the-marketplace-lister and renders Mariah's "lists and re-prices items for sale" charter on `/workers/<id>` — a worse-than-F3 vocabulary mismatch the camera could land on.**

## Root cause

`web/lib/onboarding/use-first-hire.ts:120-127` defines `VERTICAL_TO_SAMPLE_WORKER` with 4 entries: `marketplace-seller → Mariah, recruiter → Priya, customer-support → Jamal, marketing-intern → Nori`. The constant `DEMO_FALLBACK_WORKER` is Mariah. `web/app/(setup)/onboarding/pick-vertical/page.tsx:108-158` renders the full vertical grid from the gateway / SEED_VERTICALS fixture, which includes the 7 verticals named above.

When the user picks an unmapped vertical (e.g. Sales SDR), `buildDemoFirstHireResult('sales-sdr')` returns `Mariah`'s id. The redirect lands on `/workers/<mariah-uuid>/overview` showing Mariah's persona — wrong vertical, wrong charter, wrong sample activity. The mismatch is invisible in DevTools (no error) but visible to the camera (the worker page reads "Marketplace Seller" when the user picked "Sales SDR").

Compounds with the broader F1 demo-first-hire issue — even fixing F1 doesn't help if the vertical grid keeps offering 7 options when only 4 land cleanly. This brief is the cheap defence-in-depth.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F10 [P2] — `/onboarding/pick-vertical` lists 7 verticals but only 4 are mapped to sample workers in demo

## Fix recipe

Two paths. Pick (a) for the demo-day landing; (b) for the post-demo polish.

**Path (a) — Restrict grid to the 4 cleanly-mapped verticals (demo-day fix).**

1. In `web/app/(setup)/onboarding/pick-vertical/page.tsx`, when `isDemoMode()` is true, filter the rendered grid to the 4 verticals listed in `VERTICAL_TO_SAMPLE_WORKER` (marketplace-seller, recruiter, customer-support, marketing-intern). The other 3 cards either don't render or render with a `coming soon` chip + disabled click.
2. Add a small explanatory line near the grid: "Demo · 4 verticals available; full catalog ships with live workspace."

**Path (b) — Expand the map to cover all 7 verticals (post-demo polish).**

3. Add Sam (the 5th `SAMPLE_TEAM` worker) + author 2 new sample personas (a Sales SDR named Sasha; an Accounts Receivable named Aria; a GTM Engineer named Gemma — names are placeholder, operator picks). Source data structure: `web/lib/sample-personas.ts` has the shape; add 3 entries.
4. Update `VERTICAL_TO_SAMPLE_WORKER` to 7 entries; remove the fallback-to-Mariah path entirely.
5. Add the 3 new personas to `WorkerPersonalityCard.tsx`'s `PERSONALITIES` map (matching the F3 cluster from the same audit).

Recommend Path (a) for v1.0 demo — it's the 30-LOC change that closes the vocab-mismatch risk; Path (b) becomes a follow-on once the demo ships.

This brief should be paired with `fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker` because that brief, if landed, drops the `VERTICAL_TO_SAMPLE_WORKER` map dependency entirely (the user's typed name becomes the worker, regardless of vertical). In that world this brief's only remaining job is to filter the grid so the camera doesn't land on unsupported verticals during dressing-side sweeps. Honor that ordering: if the F1-cluster brief ships first, this one shrinks to ~10 LOC.

## Acceptance criteria

- [ ] In demo mode, `/onboarding/pick-vertical` renders either (a) exactly 4 verticals or (b) 7 with no fallback-to-Mariah path.
- [ ] In demo mode, no path through the vertical grid lands the user on a `/workers/<id>` whose persona's vertical doesn't match their pick.
- [ ] Vitest pin: in demo mode, picking each rendered vertical and calling `buildDemoFirstHireResult(verticalId)` returns a worker whose `role.vertical` matches.

## May-touch

- `web/app/(setup)/onboarding/pick-vertical/page.tsx` (filter or expand grid)
- `web/lib/onboarding/use-first-hire.ts` (update map if Path b)
- `web/lib/sample-personas.ts` (new personas if Path b)
- `web/components/skycot/worker/WorkerPersonalityCard.tsx` (PERSONALITIES entries if Path b)
- `web/lib/onboarding/__tests__/use-first-hire.test.ts` (new pin)

## Out of scope

- The broader F1 demo-first-hire discarded-prompt fix (covered by `fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker.md`)
- The Peta + Nori personality coverage (covered by `fix-peta-personality-trait-level-lookup-coverage.md` + `fix-worker-personality-coverage-extend-to-nori.md`)

## Effort

S — Path (a) is ~30 LOC + a vitest pin. ~1 hour. Path (b) is ~150 LOC across 4 files. ~3-4 hours. Default: ship Path (a).
