# claude-design-system-component-extension — 15 new design-system components for marketing + onboarding + product (FOUNDATION)

**Owner**: claude. **Depends on**: `claude-brand-identity-system` MERGED.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-design-system-component-extension`.
**Source**: §4 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Foundation for marketing site + onboarding wizard + signup flow.** Without these primitives, downstream UI briefs end up reinventing wheels and breaking design consistency.
- **Brand discipline enforcement.** Each component consumes brand tokens; no inline color/spacing literals.

## Scope

P0 FOUNDATION. The existing `web/components/ui/` has 11 baseline components (Button, Card, Avatar, AlertDialog, Breadcrumb, DataCell, NavDrawer, OAuthConnectorRow, ProgressDots, StatusPill, Toast). 

To ship marketing + onboarding + signup at Jonny-Ive quality, we need 15 more component primitives. All consume brand tokens from `claude-brand-identity-system`. All shadcn-style: Radix primitives + Tailwind v4 + class-variance-authority for variants.

Components to ship (15):

1. **LandingHero** — marketing hero. Display headline + sub-copy + 2 CTAs (primary + secondary).
2. **SignupForm** — magic-link email entry. Single input + button. Loading/success/error states.
3. **OnboardingStep** — wizard step layout. ProgressDots + headline + body + actions.
4. **PricingCard** — pricing tier display. Plan name + price + feature checklist + CTA.
5. **FeatureSection** — feature highlight. Icon + headline + body, in alternating left/right layout.
6. **TestimonialCard** — customer quote display. Avatar + name + role + company + quote.
7. **KeyFact** — single stat. Large number + label + context.
8. **CodeBlock** — syntax-highlighted code. Mono font, copy button, optional language label.
9. **Diagram** — wraps an inline SVG diagram. Caption + responsive sizing.
10. **Footer** — site footer. Sections (Product / Docs / Company / Legal) + social icons + copyright.
11. **Nav** — top navigation. Logo + nav items + sign-in CTA. Sticky on scroll. Mobile burger.
12. **ChannelBadge** — display Slack/email/SMS/voice icons with label.
13. **AuditTimeline** — chronological audit-row display. Timestamp + kind + payload preview.
14. **WorkerCard** — display worker portrait + name + role + status + duty list.
15. **ApprovalCard** — pending-approval display. Mobile-friendly. Approve/deny buttons + payload preview.

Each component:
- TypeScript + React 19.
- `'use client'` only where interactivity required.
- Variants via `class-variance-authority`.
- Tailwind classes consume brand tokens (NO hex/RGB literals).
- Accessibility: keyboard navigable, ARIA-labeled, semantic HTML.
- Storybook story OR live preview page at `/brand/components`.
- 1+ test (Vitest + Testing Library).

LOC: ~600 (15 components × ~40 LOC average + tests + preview page).

## Files you may touch

**Components (`web/components/ui/`):**
- `LandingHero.tsx` (NEW). ≤50 LOC.
- `SignupForm.tsx` (NEW). ≤60 LOC.
- `OnboardingStep.tsx` (NEW). ≤50 LOC.
- `PricingCard.tsx` (NEW). ≤55 LOC.
- `FeatureSection.tsx` (NEW). ≤45 LOC.
- `TestimonialCard.tsx` (NEW). ≤40 LOC.
- `KeyFact.tsx` (NEW). ≤30 LOC.
- `CodeBlock.tsx` (NEW). ≤55 LOC.
- `Diagram.tsx` (NEW). ≤35 LOC.
- `Footer.tsx` (NEW). ≤60 LOC.
- `Nav.tsx` (NEW). ≤70 LOC.
- `ChannelBadge.tsx` (NEW). ≤35 LOC.
- `AuditTimeline.tsx` (NEW). ≤55 LOC.
- `WorkerCard.tsx` (NEW). ≤50 LOC.
- `ApprovalCard.tsx` (NEW). ≤60 LOC.

**Preview page:**
- `web/app/(app)/brand/components/page.tsx` (NEW) — live preview of all 15 components in each variant. ≤200 LOC.

**Tests:**
- `web/components/ui/__tests__/*.test.tsx` (NEW) — 1 test per component covering primary use + accessibility. ≤300 LOC combined.

## Files you must NOT touch

- The 11 existing baseline components — leave alone unless they need refactor for new tokens (and only NARROWLY).
- Brand tokens in `web/app/globals.css` — those belong to `claude-brand-identity-system`.
- Marketing site files (separate brief).
- Backend Rust code.

## Acceptance criteria

- [ ] 15 new components shipped, each with TypeScript types + variants via `class-variance-authority`.
- [ ] Each component imports from `@/lib/utils` (existing helpers) — no parallel utility files.
- [ ] NO hex/RGB literals in any new component file. All colors via brand tokens.
- [ ] NO inline `style={...}` props for design properties. Tailwind classes only.
- [ ] `pnpm lint` clean. `pnpm type-check` clean.
- [ ] `pnpm test` passes (Vitest tests for each component).
- [ ] `/brand/components` preview page renders all 15 components in each variant; no broken layouts.
- [ ] Lighthouse score ≥ 95 on `/brand/components`.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-design-system-component-extension`
- PR title: `web: 15 design-system components for marketing + onboarding (claude-design-system-component-extension) — FOUNDATION P0`

## Notes for the implementer

- **shadcn pattern:** Radix primitive + Tailwind tokens + cva variants. Reference existing `Button.tsx` + `AlertDialog.tsx` for the established style.
- **No magic values:** if you find yourself typing a color hex or a px value, stop. Use a brand token.
- **Restraint:** components should feel quiet. The product speaks through its work. Avoid heavy borders, deep shadows, gratuitous gradients.
- **Mobile-first:** every component must work on a 360px viewport.
- **Dark mode:** every component must work in both light + dark. Test BOTH.
- **Don't reinvent existing components.** If `Button` works, use it. The 15 here are NEW primitives, not replacements.
- After this brief, downstream briefs (marketing-site, onboarding-wizard, signup-flow, pricing-cards) compose these primitives. They should not need to author new design components.
