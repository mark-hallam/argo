# fix-onboarding-wizard-footer-hardcoded-companies-stat

**Owner**: claude.
**Authored against**: origin/main `ea161df3a` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-wizard-footer-hardcoded-companies-stat`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/onboarding-hire-an-ai-teammate.md` F11 [P2].

🟡 **P2 — The onboarding wizard footer displays a hardcoded fake stat: `2,847 COMPANIES THIS MONTH` (web/components/auth/WizardEditorialShell.tsx:419). The landing-page sibling (`2,847 teams this month` at `web/app/for/_components/VerticalLandingShell.tsx:635`) was removed via PR #2305 for being fictitious. The wizard footer version was missed.**

Operator brand promise (memory): no fake numbers. On a recorded demo this is camera-visible and reads as theater.

## Findings this brief closes

- onboarding F11 [P2] — Hardcoded "2,847 companies this month" stat in wizard footer

## Fix recipe

1. **Remove the fake stat** from `web/components/auth/WizardEditorialShell.tsx:418-421`. Drop the entire `<span className="wiz-foot-item">` containing `<strong>2,847</strong>COMPANIES THIS MONTH`.

2. **Footer keeps the two real items**: "SOC 2-READY AUDIT TRAIL" and "GDPR ARTICLE 30".

3. **Optional**: if the visual balance suffers from a 2-item footer (the strip is centered with `display:flex`), add a third trust signal that is non-fabricated — `"END-TO-END ENCRYPTED"` or `"MULTI-TENANT BY CONSTRUCTION"` are both true statements derivable from the architecture. Operator preference: prefer trust-language over engagement-language for any insertion.

4. **Match the landing-page removal pattern**. PR #2305 removed the analogous fake stat from the landing hero — match its approach (delete the element + adjust the visual rhythm of the surrounding flex container, no replacement needed if footer reads cleanly with 2 items).

5. **Add a vitest spec** at `web/components/auth/__tests__/WizardEditorialShell.no-fake-stats.test.tsx`:
   - Render `<WizardEditorialShell>` with a minimal draft
   - Assert NO text matching `/2,?847/` or `/companies this month/i` is present
   - Assert the SOC 2 + GDPR items still render

## Acceptance criteria

- [ ] `2,847` and "COMPANIES THIS MONTH" not present in any onboarding wizard step in dev or demo mode
- [ ] SOC 2 + GDPR footer items still render
- [ ] New vitest spec passes
- [ ] `pnpm --dir web exec vitest run components/auth/__tests__/WizardEditorialShell` green
- [ ] `pnpm --dir web exec tsc --noEmit` green

## May-touch

- `web/components/auth/WizardEditorialShell.tsx` (~5 LOC delete + minor CSS tweak)
- `web/components/auth/__tests__/WizardEditorialShell.no-fake-stats.test.tsx` (NEW, ~20 LOC)

## Out of scope

- The landing page variant (already removed via #2305)
- Any other wizard polish (step strip, save & exit affordance — covered by `fix-onboarding-wizard-polish-step-strip-and-save-exit`)
- Vertical-specific onboarding copy (covered separately)

## Effort

~25 LOC change + ~20 LOC test. ~30 minutes. P2 polish; small but operator-flagged as a credibility issue.
