# fix-wizard-shell-dark-mode-bg-min-height-100vh-exposes-near-black-band

**Owner:** either
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-wizard-shell-dark-mode-bg-min-height-100vh-exposes-near-black-band`
**Source:** ux-audit-2026-05-27-2351

🟡 **P1 — In dark color-scheme, the wizard's cream/lavender gradient ends at content-height and exposes the document body's near-black background as a stripe below. Reads as a video glitch.**

## Root cause

`web/components/auth/WizardEditorialShell.tsx` paints a soft cream/lavender gradient as its background. The shell's height is determined by content (eyebrow + hero + body + CTA). On a 900px-tall viewport with the welcome step (~1062px content height), the wizard scrolls — and the area below the wizard's gradient is the document body background, which goes to near-black `rgb(2,7,11)` in dark color-scheme (verified via `preview_resize colorScheme=dark`).

Visually this reads as a recorded-video glitch ("why does the bottom half of the page go black?") rather than a designed transition. The investor demo will most likely be recorded in light mode, but this regresses for any reviewer whose system preference is dark.

— `onboarding-hire-an-ai-teammate.md` F7

## Findings this brief closes

- `onboarding-hire-an-ai-teammate.md` F7 [P1] — In dark color-scheme, the wizard's light gradient ends and exposes a near-black band below the wizard on tall pages

## Fix recipe

1. **Add `min-height: 100vh` unconditionally** to `.wiz-editorial` in `WIZARD_STYLES` (in `web/components/auth/WizardEditorialShell.tsx`).
   - Today the rule may exist for medium viewports only per some media queries; make it unconditional.
2. **Verify gradient extends** — the gradient should fill the full viewport regardless of content height:
   - `background: linear-gradient(180deg, …) no-repeat fixed`
   - Or set the gradient on `html.dark` body to match the wizard.
3. **Smoke test at three breakpoints + light/dark**:
   - 1440 x 900, light
   - 1440 x 900, dark
   - 375 x 667, light
   - 375 x 667, dark
   - On each, the gradient should fill below content.
4. **Playwright spec** `web/playwright/onboarding/wizard-dark-mode-no-black-stripe.spec.ts`:
   - Set `colorScheme=dark` via preview chrome.
   - Visit `/onboarding/welcome`.
   - Read computed bg color at the bottom of the page (y=899 of a 900-tall viewport).
   - Assert the computed value is NOT close to `rgb(2,7,11)` — it should match the gradient's bottom stop.

## Acceptance criteria

- [ ] In dark color-scheme on tall content, the wizard's gradient fills 100vh — no near-black stripe visible.
- [ ] Light mode is unchanged.
- [ ] Mobile + desktop both render correctly.
- [ ] Playwright spec passes cold.
- [ ] PR body cites finding F7 + screenshot before/after.

## May-touch

- `web/components/auth/WizardEditorialShell.tsx`
- `web/playwright/onboarding/wizard-dark-mode-no-black-stripe.spec.ts` (NEW)

## Out of scope

- Auth pages light-theme drift (covered by `fix-auth-pages-light-theme-to-match-wizard`)
- Wizard mobile viewport top-bar clipping (covered by `fix-onboarding-wizard-mobile-viewport-top-bar-clipping`)
- SAVE & EXIT button safety (covered by `fix-onboarding-save-and-exit-button-safety`)

## Effort

XS to S — 1 CSS rule change + 1 spec. ~30-60 minutes.
