# fix-team-page-duplicate-hire-template-grid

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-team-page-duplicate-hire-template-grid`.
**Source**: e2e-ux-audit Agent (whole-site-mvp-claims-coverage) finding F15 (`docs/skycot/ux-audits/2026-05-25-2309/whole-site-mvp-claims-coverage.md`).

🟢 **P2 — `/team` empty state duplicates the `/workers` hire-template grid AND the onboarding wizard's first-hire grid. Three different surfaces serving the same affordance with the same templates is confusing.**

## Root cause

`/team` empty state below the Audiowide hero shows: "Hire your first AI teammate" + 4 "OR START FROM A TEMPLATE" cards (Marketplace seller / Recruiter / Customer support / Sales SDR — outbound). The exact same hire-template grid appears in:
- `/onboarding/first-hire` (wizard step 5)
- `/workers` empty state (per onboarding audit F4)

Three surfaces, same affordance, same templates (with extra inconsistency on which 5 templates each surface lists — see `fix-onboarding-vertical-list-align-with-landing-personas` for that separate problem).

Operator may end up trying to hire from `/team` when the canonical surface is `/workers`. Three duplicate hire CTAs = unclear single-source-of-truth.

## Findings this brief closes

- whole-site-mvp-claims-coverage F15 [P2] — `/team` empty state duplicates the `/workers` hire-template grid

## Fix recipe

Pick a clear separation of concerns:

1. **`/team`** — keep as a **list-of-humans** surface (operator + invited human operators + role assignments). Do NOT render the hire-AI-teammate grid here. If `/team` is supposed to show AI workers too (per the C2 trading-card deck design), the empty state should be a "Hire your first teammate" CTA that routes to `/workers/new` OR `/onboarding/first-hire`, NOT render the templates inline.

2. **`/workers`** — the canonical AI-worker hiring surface. Either the empty state lives here (with the templates) OR redirects to the onboarding wizard for fresh tenants.

3. **`/onboarding/first-hire`** — the wizard step. Templates render here as part of the wizard.

Recommended rule: ONE place renders the template grid (the wizard). The other two surfaces have a CTA that routes to the wizard.

**Steps**:
1. Touch site: `web/app/(app)/team/page.tsx` — drop the "OR START FROM A TEMPLATE" cards from the empty state. Replace with a single primary CTA "Hire your first teammate" → routes to `/workers/new` or `/onboarding/first-hire`.
2. Audit `/workers` empty state — same treatment: drop duplicate templates, single CTA → wizard.
3. Wire the wizard's first-hire step (`web/app/(setup)/onboarding/first-hire/page.tsx`) as the canonical template-grid renderer.
4. Pin with a test at `web/app/(app)/team/page.test.tsx`: render with empty team data, assert no template card is rendered (only the single CTA).

## Acceptance criteria

- [ ] `/team` empty state shows ONE primary CTA, no template cards.
- [ ] `/workers` empty state shows ONE primary CTA, no template cards (consistent with `/team`).
- [ ] `/onboarding/first-hire` remains the only surface rendering the full template grid.
- [ ] The single CTA on `/team` and `/workers` routes to the same destination.
- [ ] Test pins pass.

## May-touch

- `web/app/(app)/team/page.tsx`
- `web/app/(app)/workers/page.tsx` (the empty-state branch)
- `web/components/empty-state/HireStarterPrompts.tsx` (likely consolidated here — delete if no longer used)
- `web/app/(app)/team/page.test.tsx` (NEW or extend)
- `web/app/(app)/workers/page.test.tsx` (extend)

## Out of scope

- The 5-template list alignment between wizard and `/workers` (separate brief `fix-onboarding-vertical-list-align-with-landing-personas`)
- The C2 holographic trading-card deck on `/team` (separate brief `fix-team-page-empty-state-despite-gateway-worker`)
- Workers/team route ambiguity (separate brief `fix-workers-team-route-ambiguity-and-breadcrumb-drift`)

## Effort

~30-80 LOC + 2 test pins. ~1 hour.
