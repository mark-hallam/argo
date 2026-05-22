# web-onboarding-f1-route-restore

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `7ce67c46d` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-onboarding-f1-route-restore`.

🟡 **P2 BRAND PROMISE GAP — /onboarding 404s despite F1 design-lock.** Source: evaluator round-6 SHARED.md handoff 2026-05-23T05:30:00Z (Chrome MCP comprehensive E2E).

> "web-onboarding-f1-route-restore — /onboarding 404s despite F1 design-lock"

Context: per operator memory `project_onboarding_design_locked.md`, the F1 editorial-center 5-step onboarding wizard was design-locked 2026-05-19 (welcome / workspace / provider / vertical / hire). The HTML mockup exists at `docs/skycot/design-explorations/2026-05-19/onboarding-wizard/option-F1-editorial-center.html` (or similar) but **no route was ever added to `web/app/(app)/onboarding/`** — every link to `/onboarding` returns 404.

Fresh signups are silently broken — there's no onboarding journey, just the bare app shell.

## Scope

Restore the F1 onboarding flow as 5 routes:

- `web/app/(app)/onboarding/page.tsx` — Step 1 Welcome (editorial hero + "Get started" CTA → step 2)
- `web/app/(app)/onboarding/workspace/page.tsx` — Step 2 Workspace setup (tenant name + slug)
- `web/app/(app)/onboarding/provider/page.tsx` — Step 3 Provider config (Anthropic API key OR Claude Code OAuth handoff)
- `web/app/(app)/onboarding/vertical/page.tsx` — Step 4 Vertical pick (currently shipped packs: sales / customer-support / devrel / project-coordinator — see `packs/`)
- `web/app/(app)/onboarding/hire/page.tsx` — Step 5 First hire (mini hire wizard or deep-link into `/hire/step/1`)

Each step:
1. Renders the F1 editorial-center design (Audiowide hero, Inter weight 200, centered single column, light theme with the brand-loud Audiowide wordmark as only Audiowide element).
2. Shows step indicator (1/5, 2/5, ...).
3. "Next" CTA → next step. "Back" link → previous step (steps 2-5).
4. State persists via URL params + localStorage (don't require server-side session for onboarding draft).

### Layout file

`web/app/(app)/onboarding/layout.tsx` — wraps all 5 routes in a consistent shell. **CRITICAL**: must be a real React component (default-exported), NOT a utility module — Next.js App Router will choke (per the round-5 `web-workflow-build-layout-ts-rename` finding).

### Acceptance smoke

- Hit `http://localhost:3000/onboarding` cold → renders welcome page, not 404.
- Click through all 5 steps without breaking.
- Final step deep-links to `/hire/step/1` with the chosen vertical pre-filled.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: web/app/(app)/onboarding/page.tsx` + 4 sibling step routes.
- `NEW: web/app/(app)/onboarding/layout.tsx` (must be React component, NOT util).
- `NEW: web/components/onboarding/OnboardingShell.tsx` — shared step layout + indicator (~80 LOC).
- `NEW: web/components/onboarding/StepIndicator.tsx` (~40 LOC).
- `NEW: web/__tests__/onboarding/wizard-flow.test.tsx`.
- Reference the F1 HTML mockup at `docs/skycot/design-explorations/2026-05-19/onboarding/*.html`.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- The hire wizard (`/hire/step/*`) routes — onboarding deep-links INTO it, doesn't reinvent.
- Authentication routes (`/auth/*`) — separate surface.
- `/auth/signup` — the existing magic-link flow stays as-is.

## Acceptance criteria

- [ ] All 5 onboarding routes render (no 404s).
- [ ] Each step matches the F1 design lock (editorial-center, Audiowide wordmark only, Inter body).
- [ ] Step indicator shows progress.
- [ ] Next/Back navigation works.
- [ ] Vertical pick lists currently-shipped packs from `packs/` directory.
- [ ] Final step deep-links to `/hire/step/1?vertical=<chosen>`.
- [ ] `layout.tsx` is a React component (default-exported) — Next.js doesn't 500.
- [ ] vitest covers the 5-step click-through.
- [ ] `pnpm typecheck` + `pnpm lint` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `web-onboarding-f1-route-restore`
- PR title: `feat(web): restore /onboarding F1 5-step wizard from design lock (web-onboarding-f1-route-restore)`

## Notes for the implementer

- ~400-600 LOC expected across 5 page + 1 layout + 2 component files. Distribute carefully — each page should be ~80-100 LOC, the OnboardingShell does the heavy lift.
- **Match the F1 mockup literally** — don't reinvent the design language. Operator memory `project_onboarding_design_locked.md` is explicit: A-style top bar in light theme, centered editorial single column, Inter weight 200 hero typography, Audiowide wordmark as the only Audiowide element on the page.
- **Per memory `project_brand_consistency_pass.md` (D3 SUPERSEDES the "Inter only" claim)** — apply Audiowide hero + purple→pink gradient where the F1 mockup calls for it.
- **The vertical pick is data-driven** — read `packs/*/pack.toml` for the catalog rather than hardcoding.
- Cite the evaluator round-6 report verbatim in the PR body.
- This is the biggest of the round-6 briefs — the other 2 are sub-200 LOC each, this is closer to 500.
