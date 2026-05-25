# fix-onboarding-welcome-step-autofocus-cta

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-welcome-step-autofocus-cta`.
**Source**: e2e-ux-audit Agent (onboarding-hire-an-ai-teammate) findings F11 + F12 (`docs/skycot/ux-audits/2026-05-25-2309/onboarding-hire-an-ai-teammate.md`).

🟡 **P2 — Onboarding wizard advertises `↵` enter-to-continue affordances but the welcome step has no autofocus, so Enter on page load does nothing. Pick-vertical step's Back CTA falls below the fold at 1280×800.**

## Root cause

**F11**: Wizard CTA buttons all carry `<span className="ob-cta-kbd">↵</span>` decorations implying enter-to-continue. On the welcome step (`web/app/(setup)/onboarding/welcome/page.tsx:54-67`), hitting Enter on page load does nothing because the `<button>` has no `autoFocus`. Tabbing requires 5 tabs (Return home → SAVE & EXIT → Help → Get started) to reach the primary CTA.

Workspace step is better — its first input has `autoFocus`. Pick-vertical and first-hire also have it. Welcome is the odd one out.

**F12**: On `/onboarding/pick-vertical` at desktop preset 1280×800, the 5 vertical cards push the "← Back" CTA below the fold. The viewport-bottom radial-blob hides it visually. Users assume they can't go back. Same `.ob-cta-back` is at the TOP in workspace, but pick-vertical places it at the bottom.

Combined as one brief because both are wizard-keyboard / wizard-affordance polish in the same surface tree.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F11 [P2] — Welcome step does NOT autofocus the "Get started" button
- onboarding-hire-an-ai-teammate F12 [P2] — Pick-vertical step's "← Back" CTA falls below the fold at 1280×800

## Fix recipe

**For F11**:
1. Touch site: `web/app/(setup)/onboarding/welcome/page.tsx:54-67`.
2. Add `autoFocus` to the "Get started" `<button>`:
   ```tsx
   <button autoFocus className="ob-cta-primary" onClick={...}>
     Get started <span className="ob-cta-kbd">↵</span>
   </button>
   ```
3. Confirm Enter on page load triggers the click handler (it should, as `autoFocus` lands the button as the active element).

**For F12**:
1. Touch site: `web/app/(setup)/onboarding/pick-vertical/page.tsx:160-170`.
2. Either:
   - **(a)** Move the `.ob-cta-back` row to the TOP of the page (matching workspace), OR
   - **(b)** Reduce the row gap on the 5 vertical cards so the Back row fits at 1280×800, OR
   - **(c)** Float a sticky "Back · Step 3 of 5" bar at the wizard top so navigation is always visible regardless of viewport height.
3. Option (c) is the most consistent UX across all wizard steps. Option (a) is the smallest patch.

**Pin both**:
- `web/tests/onboarding-keyboard.spec.ts` (NEW): for `/onboarding/welcome`, assert `document.activeElement` is the primary CTA on page load; press Enter; assert navigation to next step.
- For pick-vertical, assert the Back CTA is in the viewport at 1280×800.

## Acceptance criteria

- [ ] Welcome step: Enter on page load advances to the next step.
- [ ] Welcome step: `document.activeElement.textContent` includes "Get started" on page load.
- [ ] Pick-vertical step: Back CTA visible at 1280×800 viewport without scroll.
- [ ] Other wizard steps' existing autoFocus behavior is unchanged.
- [ ] Playwright spec passes.

## May-touch

- `web/app/(setup)/onboarding/welcome/page.tsx`
- `web/app/(setup)/onboarding/pick-vertical/page.tsx`
- `web/app/(setup)/onboarding/_OnboardingShell.tsx` (if sticky-bar approach)
- `web/tests/onboarding-keyboard.spec.ts` (NEW)

## Out of scope

- Welcome SOC2 copy fix (separate brief `fix-onboarding-welcome-soc2-day-one-copy`)
- Vertical-list alignment with landing personas (separate brief `fix-onboarding-vertical-list-align-with-landing-personas`)
- Provider-monolithic onboarding (separate brief `fix-onboarding-connect-provider-multi-provider-copy-or-options`)

## Effort

~20-50 LOC + 1 Playwright spec. ~45 min.
