# web-hire-wizard-f1-polish — apply locked F1 editorial-center to /hire wizard

**Owner**: claude. **Sizing**: ~400-600 LOC across the 5-step wizard.
**Authored against**: origin/main `3121e32e` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-hire-wizard-f1-polish`.
**Priority**: P1 — onboarding is the second-most-important brand moment after the public landing. Currently the /hire wizard renders a minimal Q&A pattern (textarea + Continue) instead of the locked F1 editorial-center treatment.

## Locked design reference

`docs/skycot/design-explorations/2026-05-18/onboarding-wizard/option-F1-editorial-center.html`.

Key elements:
- Audiowide hero typography that adapts per step ("Build your AI workforce.", "Connect a provider.", etc.)
- Step progress row (01 OF 05 + colored progress dots)
- Centered editorial layout (matches the auth/signup language)
- Warm radial-blob background
- Top-left brand mark, top-right "Help" + "SAVE & EXIT"
- Footer trust strip (NO CREDIT CARD · SOC 2 TYPE II · GDPR · 2,847 COMPANIES THIS MONTH)
- Big purple-gradient CTA buttons

## Scope

Files to touch (per `web/app/(hire)/`):
- `web/app/(hire)/hire/page.tsx` — wizard entry (currently the "What do you need help with?" question)
- `web/app/(hire)/hire/step/[step]/page.tsx` — per-step content
- `web/app/(hire)/hire/done/[workerId]/page.tsx` — completion celebration

Per-step adaptations (5 steps):
1. **Welcome** — "Build your AI workforce." (matches F1 mockup verbatim)
2. **Workspace** — "Name your team." (or similar)
3. **Connect provider** — "Pick a brain." (provider selection)
4. **Pick vertical** — "What do they do?" (vertical chooser)
5. **First hire** — "Meet your first teammate." (worker preview)
6. **Done** — celebration screen with "Open SKYCOT" CTA

Reuse `AuthEditorialShell` from `web/components/auth/AuthEditorialShell.tsx` if the layout is compatible (eyebrow + hero + body + footer). Otherwise factor out a `WizardEditorialShell` companion component.

## Acceptance criteria

- [ ] Every step renders Audiowide hero with gradient-emphasis word.
- [ ] Progress strip visible on all steps (`STEP 0N OF 05` + colored dots).
- [ ] Save & Exit + Help links in top-right of every step.
- [ ] Trust strip in footer on every step.
- [ ] Brand mark in top-left links to /landing for unauthed users / / for authed.
- [ ] Each step's CTA is the gradient pill style.
- [ ] No regression to the wizard's actual onboarding logic (provider key save, vertical selection, worker spawn).
- [ ] `pnpm typecheck && pnpm test` clean.

## Out of scope

- Adding new wizard steps.
- Changing the onboarding LOGIC (just the visual treatment).
