# web-onboarding-f1-design-adoption-port

**Authored against**: `origin/main` `72e782df` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-onboarding-f1-design-adoption-port`.

## Summary

25-audit synthesizer flagged "locked F1 design not yet adopted for `/onboarding`" as a remaining unfixed gap (per operator memory [project_25_audit_sweep_complete.md]). The F1 onboarding design is operator-locked across 5 steps (welcome / workspace / provider / vertical / hire) per [project_onboarding_design_locked.md]. Port the design from `docs/skycot/design-explorations/.../option-F1*.html` to the live `/onboarding` JSX surface.

Pre-condition: F1 design files exist at `docs/skycot/design-explorations/2026-05-19/onboarding-wizard/` (verify before claim).

## Acceptance

- [ ] `/onboarding` (5 steps: welcome → workspace → provider → vertical → hire) renders the F1 editorial-center design.
- [ ] Audiowide wordmark is the only Audiowide on each page (per locked design — operator memory).
- [ ] A-style top bar in light theme, centered editorial single column, Inter weight 200 hero typography (locked register).
- [ ] D3 brand-consistency pass applied (per [project_brand_consistency_pass.md]): purple→pink gradient (not purple→cyan), warm radial-blob background, `#1e1b4b` ink, Audiowide hero. (This memory SUPERSEDES the "Inter only / no Audiowide" claim in the original lock.)
- [ ] Each step preserves the existing form behavior — Zod validation, submit handler, error toasts. Pure visual port.
- [ ] Mobile responsive (≥320px wide). Reduced-motion respected.
- [ ] Existing onboarding tests pass unchanged (or are updated only if assertions referenced the old DOM structure).
- [ ] Locked design HTML rendered side-by-side with the new JSX in the PR body for visual diff.

## May touch

- `web/app/(app)/onboarding/page.tsx` and each of the 5 step pages.
- `web/components/skycot/onboarding/` — new module is fine.
- Onboarding-specific tests if assertions need updating.

## Must NOT touch

- The onboarding form submission logic (it's correct).
- Any non-onboarding page.
- Wizard step navigation contract.

## Notes

- The locked design is at `docs/skycot/design-explorations/2026-05-19/onboarding-wizard/` (operator picked F1 during the optometrist round).
- Operator memories that govern this port: [project_onboarding_design_locked.md], [project_brand_consistency_pass.md], [user_design_exploration_optometrist_method.md].
- This is wave-5 viral-moment work; expect to be brand-loud-editorial register.
- Match: not a 1:1 HTML port — the JSX should preserve form behavior AND match the visual register. Designer-author judgment expected; cite both source mockup and final JSX in PR body.
