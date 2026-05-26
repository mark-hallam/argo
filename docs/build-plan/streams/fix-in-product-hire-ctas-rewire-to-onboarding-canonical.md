# fix-in-product-hire-ctas-rewire-to-onboarding-canonical

**Owner**: claude.
**Authored against**: origin/main `9c407cd20039adf25a986343b7c8cdba76650a52` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-in-product-hire-ctas-rewire-to-onboarding-canonical`.
**Source**: UX-AUDIT-T0015 synthesis (`docs/skycot/ux-audits/2026-05-27-0015/onboarding-hire-an-ai-teammate.md` F7 [P1]).

🟠 **P1 — Every in-product "hire" CTA (dashboard, /team, /team/error, CommandPalette, TeamEmptyState, WorkflowEdgeStates) still routes to `/hire/step/1..5` (the parallel wizard) while the canonical demo wizard is `/onboarding/first-hire`. The marketing /landing CTA has been fixed (2309-F6 closed via /auth/signup re-route), but the in-product side is untouched. So once a buyer is inside the app, every "Hire a worker" affordance bounces them into the OTHER wizard — different copy, different state shape, different completion shape. CLAUDE.md drift-prevention rule: "no parallel abstractions; two pages with overlapping wizard step definitions in different routes is a bug".**

## Root cause

`grep -rnE '(href="/hire"|router\.push\("/hire"\))' web/app web/components` returns 12 callsites:

- `web/app/(app)/page.tsx:460` — dashboard top-bar "Hire your first teammate"
- `web/app/(app)/team/page.tsx:268` — Team page header CTA
- `web/app/(app)/team/page.tsx:469` — Team page empty-state CTA
- `web/app/(app)/team/error.tsx:38` — Team error-boundary fallback CTA
- `web/components/skycot/CommandPalette.tsx:449` — ⌘K → "Hire a worker"
- `web/components/skycot/TeamEmptyState.tsx:54` — shared empty-state
- `web/components/skycot/workflows/WorkflowEdgeStates.tsx:173` — workflows empty/error fallback CTA

All 7 unique surfaces send the user to `/hire/step/1` rather than `/onboarding/first-hire` (or `/genesis` once that surface is wired per `fix-genesis-orphan-discoverability-and-provider-key-503`). The two wizards drift:

- `/onboarding/welcome` (first-run, F1 editorial-center): 5 steps including trust pillars + workspace + connect-provider + pick-vertical + first-hire
- `/hire/step/1` (in-product N-th hire): different shape, different step count, content-drift documented in `fix-onboarding-hire-parallel-wizards-unify-or-document`

The dual-flow problem is also flagged in the prior 2026-05-26-2309 onboarding audit (F2, marked PARTIAL). This brief covers the in-product callsite rewire half of the unification.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0015/onboarding-hire-an-ai-teammate.md` F7 [P1] — In-product "hire" CTAs still push to /hire (the parallel wizard); only /landing was rewired

## Fix recipe

This brief is the callsite rewire ONLY. It depends on the operator-decision in `fix-onboarding-hire-parallel-wizards-unify-or-document` (which wizard is canonical). Default assumption per audit: `/onboarding/*` wins because (a) it owns the F1-locked editorial-center design + (b) marketing already routes there via `/auth/signup`.

1. Decide the canonical N-th-hire entry. Two valid candidates:
   - `/onboarding/first-hire` (skip the welcome + workspace + connect-provider + pick-vertical steps for repeat hires)
   - `/genesis` (operator-locked investor-moment surface per `feedback_genesis_is_ai_teammate_onboarding_studio`)
2. Replace every callsite. Suggested mapping:

   | File | Current | New |
   |---|---|---|
   | `web/app/(app)/page.tsx:460` | `/hire` | `/onboarding/first-hire` |
   | `web/app/(app)/team/page.tsx:268` | `/hire` | `/onboarding/first-hire` |
   | `web/app/(app)/team/page.tsx:469` | `/hire` | `/onboarding/first-hire` |
   | `web/app/(app)/team/error.tsx:38` | `/hire` | `/onboarding/first-hire` |
   | `web/components/skycot/CommandPalette.tsx:449` | `/hire` | `/onboarding/first-hire` |
   | `web/components/skycot/TeamEmptyState.tsx:54` | `/hire` | `/onboarding/first-hire` |
   | `web/components/skycot/workflows/WorkflowEdgeStates.tsx:173` | `/hire` | `/onboarding/first-hire` |

3. After the rewire, `/hire/step/*` either (a) becomes a 301-redirect-to-onboarding shim, OR (b) is deleted (operator-decisional). For v1.0 demo prefer the shim — keeps any old bookmarks alive.
4. Verify the `/onboarding/first-hire` entry behaves correctly when reached without going through the prior 4 steps (i.e. for N-th hire, the workspace/provider/vertical are already configured). The page already handles this via `useFirstHire`'s tenant resolution.

## Acceptance criteria

- [ ] `grep -rnE '(href="/hire"|router\.push\("/hire"\))' web/app web/components` returns 0 matches (or only the 301 shim's own re-export).
- [ ] Clicking "Hire a worker" from ⌘K → lands on `/onboarding/first-hire` (or `/genesis`).
- [ ] Clicking the dashboard top-bar "Hire your first teammate" → same destination.
- [ ] /team header + empty-state + error CTAs → same destination.
- [ ] /workflows empty/error CTA → same destination.
- [ ] `/hire` either 301s to the canonical entry or returns the existing wizard until deletion is operator-approved.
- [ ] `pnpm --dir web exec tsc --noEmit` green.
- [ ] `pnpm --dir web exec playwright test web/playwright/team` regression-passes (no broken click-throughs).
- [ ] Add a vitest unit that fails if any new `href="/hire"` (or `router.push("/hire")`) appears in `web/app` or `web/components` — prevents regression.

## May-touch

- 7 SPA callsites listed above (~3-7 LOC each)
- Optional 301 shim at `web/app/(setup)/hire/[[...rest]]/page.tsx` or in `next.config.ts` (~20 LOC)
- One vitest regression check at `web/lib/__tests__/no-hire-route-callsites.test.ts` (~30 LOC) using `glob` + `readFile`

## Out of scope

- The wizard-content unification itself (covered by `fix-onboarding-hire-parallel-wizards-unify-or-document`)
- Deletion of `/hire/*` files (operator-decisional; keep the shim for v1.0)
- /landing CTA already fixed; not in scope
- Genesis nav entry (covered by `fix-genesis-orphan-discoverability-and-provider-key-503`)

## Effort

S. ~50 LOC across 7 SPA callsites + 30 LOC test. ~1-2 hours. P1 because the demo-record cut-list assumes one wizard; if the operator clicks ⌘K → Hire during a take they land on the wrong one.
