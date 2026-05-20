# web-onboarding-f1-wizard-shell — wrap /onboarding in the locked F1 wizard shell

**Owner**: claude.
**Authored against**: origin/main `546e3072` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-onboarding-f1-wizard-shell`.
**Priority**: P1 — closes the single biggest off-brand surface on the demo path per the wave-2 brand audit (`/onboarding` brand-on score 1/10) and the wave-1 mission-alignment audit (`/onboarding` 3.8/10 — silent on every mission pillar).
**Sizing**: ~250-400 LOC (5 page wraps + 1 layout delete + tests).

## Why this brief exists

The /hire wizard adopted the locked F1 editorial-center design via
`WizardEditorialShell` (Audiowide wordmark + warm radial bg +
purple→pink gradient + trust footer). The /onboarding wizard ships
the bespoke v0 ShadCN placeholder shell — generic Sparkles icon,
no Audiowide, no gradient, no shared SAVE & EXIT affordance, no
trust footer surfacing SOC 2-ready / GDPR.

The wave-3 onboarding audit explicitly recommends this fix:

> Highest-impact single fix: wrap the 5 `/onboarding` pages in
> `<WizardEditorialShell>` and delete the bespoke
> `(setup)/onboarding/layout.tsx`. One change lifts brand-on from
> 1 to 9, surfaces pillar 4 via the shell's trust footer, and
> gives users a consistent SAVE & EXIT affordance across both flows.

## Steps

1. Read `web/components/auth/WizardEditorialShell.tsx` to understand the prop surface.
2. Read each /onboarding step page to identify its content (everything inside the bespoke layout).
3. Delete `web/app/(setup)/onboarding/layout.tsx` (the v0 ShadCN wrapper).
4. Add per-step or single shared layout that wraps children in `<WizardEditorialShell>`.
5. Pass the right step prop (1/5, 2/5, etc.) so the progress strip animates.
6. Surface mission-pillar copy in the welcome step (audit-first / single-binary / enterprise-grade).
7. Verify: `pnpm typecheck && pnpm lint && pnpm test` clean; curl each onboarding step → 200.

## may-touch

- `web/app/(setup)/onboarding/layout.tsx` (DELETE)
- `web/app/(setup)/onboarding/welcome/page.tsx`
- `web/app/(setup)/onboarding/workspace/page.tsx`
- `web/app/(setup)/onboarding/connect-provider/page.tsx`
- `web/app/(setup)/onboarding/pick-vertical/page.tsx`
- `web/app/(setup)/onboarding/first-hire/page.tsx`
- Optional new: `web/app/(setup)/onboarding/_OnboardingShell.tsx`
- Optional new: `web/components/auth/__tests__/WizardEditorialShell.onboarding.test.tsx`

## must-NOT-touch

- `web/components/auth/WizardEditorialShell.tsx` — the locked shared shell (any extension goes via props)
- `web/app/(hire)/*` — the /hire wizard already uses the shell correctly

## Acceptance criteria

- [ ] All 5 /onboarding steps render through `<WizardEditorialShell>` (the same shell as /hire).
- [ ] The bespoke `(setup)/onboarding/layout.tsx` ShadCN wrapper is deleted.
- [ ] Progress strip shows correct N-of-5 on each step.
- [ ] Mission-pillar copy (audit-first / single-binary / enterprise-grade) surfaces somewhere in the welcome step.
- [ ] Brand-on score per the wave-2 audit rubric: 8+ (Audiowide hero + purple→pink gradient + warm radial bg + #1e1b4b ink visible).
- [ ] `pnpm typecheck` + `pnpm lint` + relevant `pnpm test` all clean.
- [ ] curl each step on a running dev server returns 200.
