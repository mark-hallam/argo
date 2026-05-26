# fix-pricing-page-tier-cards-link-to-subscribe

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pricing-page-tier-cards-link-to-subscribe`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/whole-site-mvp-claims-coverage.md` F10 [P2].

🟢 **P2 — `/pricing` shows three tiers (Free / Pro / Enterprise) with "Hire your first teammate" CTAs, but no "Subscribe" or "Choose Pro" affordance that maps to the in-app `/billing/subscribe` page. For the POC investor demo this is fine (Stripe checkout is out of v1.0 scope), but the public Pricing page implies a buyable plan with no path to buy.**

## Root cause

`web/app/pricing/page.tsx` (verify path) renders three tier cards. Each card carries a "Hire your first teammate" primary CTA that routes to `/auth/signup`. There's no per-tier `?plan=pro` or `/billing/subscribe?tier=pro` deep-link.

The in-app `/billing/subscribe` page exists (verified via the route table from the audit) but is unreachable from the public `/pricing` page. So:

- A signed-out visitor on `/pricing` sees three tiers + one universal CTA that drops them into signup, ignoring tier selection.
- A signed-in user has no path from `/pricing` to `/billing/subscribe`.

For the v1.0 demo this is dressing — the camera doesn't drill into Pricing. But the public-marketing claim implies plans are purchasable; the conversion funnel breaks at the tier-select step.

## Findings this brief closes

- whole-site-mvp-claims-coverage F10 [P2] — `/pricing` shows three tiers but in-app `/billing/subscribe` is not linked

## Fix recipe

1. **Per-tier CTAs**: each tier card gets a "Choose <Tier>" button with a tier-aware deep-link:
   - Free: `/auth/signup?plan=free` (or no plan param)
   - Pro: `/auth/signup?plan=pro` (signed-out) OR `/billing/subscribe?tier=pro` (signed-in)
   - Enterprise: `/contact?reason=enterprise-pricing` (talk-to-sales path)
2. **Signed-in branch**: detect session via the existing `useSession` hook (or middleware-side). If signed-in, the Pro CTA routes directly to `/billing/subscribe?tier=pro`. If signed-out, route to `/auth/signup?plan=pro` and persist the plan choice in the onboarding draft store so post-signup lands on `/billing/subscribe`.
3. **Onboarding-draft store** at `web/lib/onboarding/use-onboarding-draft.ts` (verify path) — add a `selectedPlan: "free" | "pro" | null` field.
4. **Post-signup hop**: if `selectedPlan === "pro"`, route from the welcome step to `/billing/subscribe?tier=pro` once the workspace exists; otherwise continue the normal 5-step wizard.
5. **Per-tier highlighting**: add a `featuredPlan` query param so external campaigns can deep-link to `/pricing?featured=pro` and the Pro card visually expands.

## Acceptance criteria

- [ ] Each tier card has a tier-specific CTA (not a universal "Hire your first teammate").
- [ ] Pro CTA from signed-in state routes to `/billing/subscribe?tier=pro`.
- [ ] Pro CTA from signed-out state routes to `/auth/signup?plan=pro` and persists the choice through the wizard.
- [ ] Enterprise CTA routes to a talk-to-sales surface.
- [ ] Playwright spec at `web/playwright/marketing/pricing-tier-ctas.spec.ts` covers signed-out + signed-in branches.

## May-touch

- `web/app/pricing/page.tsx` (per-tier CTA rewrite)
- `web/lib/onboarding/use-onboarding-draft.ts` (selectedPlan field)
- `web/app/(setup)/onboarding/welcome/page.tsx` (post-signup hop)
- `web/playwright/marketing/pricing-tier-ctas.spec.ts` (NEW)
- `web/app/(app)/billing/subscribe/page.tsx` (verify it accepts `?tier=` query param)

## Out of scope

- Stripe / payment processor integration (v1.1+ track)
- Pricing-page redesign (separate dressing brief if needed)
- Enterprise CRM-integration for talk-to-sales (separate concern)

## Effort

~100-180 LOC + 1 Playwright spec. ~3-4 hours. P2 because public-funnel polish; demo recording doesn't drill into Pricing.
