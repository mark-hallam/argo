# mission-genesis-onboarding-pick-vertical-add-describe-own

**Owner**: claude or codex.
**Authored against**: origin/main `<latest>` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-onboarding-pick-vertical-add-describe-own`.
**Depends on**: `mission-genesis-gateway-route-streaming-api` MERGED + `mission-genesis-spa-page-describe-and-watch` MERGED.

🔴 **P0 MISSION CLAIM 3 INTEGRATION — wire Genesis into the onboarding hire-flow.**

Per Agent-5 audit + Agent-1 (Onboarding investor narrative): the onboarding step 4 (`/onboarding/pick-vertical`) shows 5 prefab roles. The product's promise "Describe the job in plain English" is shown on `/landing` but not reachable through the onboarding journey. Result: an investor who walks the onboarding flow sees a static template-picker, not the AI-builds-the-worker moment.

## Scope

Modify `web/app/(setup)/onboarding/pick-vertical/page.tsx`:

1. Add a 6th option at the top of the 5 prefab cards: **"Or describe your own role"** — distinct visual treatment (gradient border, plain-text "AI will build it" sub-label, sparkle icon).
2. Clicking it routes to `/genesis` (the page from `mission-genesis-spa-page-describe-and-watch`) with the onboarding context (`?return=/onboarding/first-hire&workspace=<id>`).
3. On vertical_ready + Hire: route back to `/onboarding/first-hire` with the generated vertical's manifest pre-selected (or skip first-hire entirely if the Hire already happened on /genesis — verify with the worker who claims `mission-genesis-spa-page-describe-and-watch`).
4. The 5 prefab options stay — Genesis is the headline path but the templates are a fast-lane fallback.

## Acceptance criteria

- [ ] /onboarding/pick-vertical shows 6 options (5 prefabs + 1 Genesis CTA).
- [ ] Clicking Genesis CTA routes to /genesis with return URL set.
- [ ] After vertical_ready + Hire, user lands on /workers/{new_id} (or /onboarding/first-hire if that step still applies).
- [ ] Brand-loud Genesis CTA matches memory `project_onboarding_design_locked` F1-editorial-center register; gradient border on the Genesis card is the only brand-loud accent in the otherwise-restrained wizard.
- [ ] Mobile (375px) layout: Genesis CTA renders cleanly above the prefab grid.

## May-touch

- `web/app/(setup)/onboarding/pick-vertical/page.tsx`
- `web/components/skycot/onboarding/pick-vertical/GenesisCta.tsx` (NEW)
- `web/components/skycot/onboarding/pick-vertical/VerticalCard.tsx` (narrow: width + grid changes)

## Out of scope

- Genesis page itself (separate brief)
- Gateway route (separate brief)

## Effort

~80-150 LOC. ~1-2 hours.
