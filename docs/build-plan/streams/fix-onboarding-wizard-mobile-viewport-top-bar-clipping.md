# fix-onboarding-wizard-mobile-viewport-top-bar-clipping

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-wizard-mobile-viewport-top-bar-clipping`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/onboarding-hire-an-ai-teammate.md` F9 [P2].

🟡 **P2 — At mobile viewport (375×812), the wizard top bar shows "SKYCOT · STEP 01 OF 05 · [dots]" then the step label "W..." gets cropped + the Help + SAVE & EXIT buttons are off-screen entirely. Demo records on desktop so this isn't a recording blocker, but a mobile visitor hitting the wizard on a phone hits a clipped chrome.**

## Root cause

`web/app/(setup)/onboarding/_OnboardingShell.tsx` lays out the top bar as a single flex/grid row with all elements visible at desktop widths. There are no mobile-specific breakpoints to collapse the progress indicator, hide the Help link, or fold the SAVE & EXIT into a hamburger entry.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F9 [P2] — Mobile viewport clips wizard top bar

## Fix recipe

In `web/app/(setup)/onboarding/_OnboardingShell.tsx` (the top-bar layout):

1. **At `< 640px` (mobile)**:
   - Collapse the dot-progress to a single text pill `1 / 5` (instead of "STEP 01 OF 05 · [5 dots]")
   - Drop the "STEP 01 OF" prefix entirely
   - Hide the "Help" link (the existing floating help button still works)
   - Move SAVE & EXIT into a hamburger menu (or a 3-dot icon button) on the right edge

2. **At `640px ≤ width < 1024px` (tablet)**:
   - Keep the dot-progress + step label
   - Keep Help + SAVE & EXIT visible but smaller (shorter labels: "Help" + "Save")

3. **At `≥ 1024px` (desktop)**: unchanged.

4. **Verify the page body** flows correctly at all 3 breakpoints — already reported as fine in the audit at mobile.

5. **Add a Playwright spec** at `web/playwright/onboarding/mobile-viewport-top-bar.spec.ts` that runs at 375×812 viewport and asserts:
   - SAVE & EXIT (or hamburger) is within viewport bounds
   - Progress pill is visible
   - No element overflows horizontally

## Acceptance criteria

- [ ] At 375×812 viewport, all wizard top-bar elements are within viewport bounds
- [ ] At 640px viewport, transitional layout looks correct
- [ ] At ≥1024px viewport, the existing locked desktop layout is unchanged
- [ ] New Playwright spec passes

## May-touch

- `web/app/(setup)/onboarding/_OnboardingShell.tsx`
- Possibly `web/app/(setup)/onboarding/_OnboardingTopBar.module.css` if it exists
- `web/playwright/onboarding/mobile-viewport-top-bar.spec.ts` (NEW)

## Out of scope

- The page body layout (already mobile-fine per audit)
- The page-specific content on each wizard step (only the shared top bar)
- The desktop layout (locked, do not touch)

## Effort

~30-80 LOC of responsive CSS + 1 spec. ~1-2 hours. P2 polish.
