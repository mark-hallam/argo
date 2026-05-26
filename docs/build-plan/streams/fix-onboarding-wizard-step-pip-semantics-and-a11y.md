# fix-onboarding-wizard-step-pip-semantics-and-a11y

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-wizard-step-pip-semantics-and-a11y`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/onboarding-hire-an-ai-teammate.md` F9 [P1] + F15 [P3].

🟠 **P1 — The wizard's step progress strip ("STEP 05 OF 05" + 5 pips) lights all pips on arrival at each step, before the operator has done anything. The first paint of step 5 shows the progress complete even though the operator has not yet typed the prompt or clicked Hire. The visual makes the progress strip read as decorative rather than informative. Separately (P3 a11y nit), the step indicator's a11y tree splits "0" and "5" of "STEP 05 OF 05" into separate StaticText nodes — screen-reader users may hear "step one, of, zero, five".**

## Root cause

### F9 — pips lit on arrival

Each wizard page hard-codes `step={N}` on its `OnboardingShell`:
- `web/app/(setup)/onboarding/welcome/page.tsx` — `step={1}`
- `web/app/(setup)/onboarding/workspace/page.tsx` — `step={2}`
- `web/app/(setup)/onboarding/connect-provider/page.tsx` — `step={3}`
- `web/app/(setup)/onboarding/pick-vertical/page.tsx` — `step={4}`
- `web/app/(setup)/onboarding/first-hire/page.tsx:81-83` — `step={5}`

`web/components/auth/WizardEditorialShell.tsx` renders pips lit when `pipIndex <= step`. So at step 5, all 5 pips are lit on arrival. The locked F1 editorial-center design memory doesn't explicitly define the semantics; the audit recommends:
- (a) "lit = on this step OR completed" — current behavior, with a half-fill / outline state for the active pip.
- (b) "lit = completed only" — active pip is distinct, final pip lights only after Hire-them is clicked.

Either is valid; both improve over "all pips lit on arrival, no distinction". Recommended: (a) — keep current model but add a visual half-state for the active pip so the operator can see "I'm on step 5, but I haven't finished it yet."

### F15 — a11y split nodes

The step indicator renders "STEP 0" + "5 OF 0" + "5" as separate `<span>` children, splitting the digit groups. Each span produces a `StaticText` a11y node. Screen readers read them out one at a time.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F9 [P1] — Wizard chrome shows progress 5/5 on the FIRST hire-step paint
- onboarding-hire-an-ai-teammate F15 [P3] — Welcome a11y tree splits "STEP 01 OF 05" into separate static-text nodes

## Fix recipe

### Pip semantics

1. **Define `activeStep` vs `completedSteps`** in `web/components/auth/WizardEditorialShell.tsx`:
   ```tsx
   type WizardShellProps = {
     step: number;       // active step (1-indexed)
     totalSteps: number;
     completedSteps?: number;  // defaults to step - 1
   };
   ```
2. **Render three pip states**:
   - `completed` — fully lit + check icon.
   - `active` — half-lit / outline (current behavior of "lit" but with `aria-current="step"`).
   - `pending` — empty outline.
3. **At the first-hire page**, pass `completedSteps={4}` (steps 1-4 are done; step 5 is active but not complete). On submit success, briefly transition to `completedSteps={5}` before navigating away.

### A11y collapse

4. **Collapse the step indicator's text into a single node** at `WizardEditorialShell.tsx`:
   ```tsx
   <p aria-label={`Step ${step} of ${totalSteps}`}>
     STEP {String(step).padStart(2, "0")} OF {String(totalSteps).padStart(2, "0")}
   </p>
   ```
   Single text node, single a11y label, no split.

5. **Add a Playwright + axe-core spec** at `web/playwright/onboarding/wizard-pip-semantics-and-a11y.spec.ts`:
   - On step 1, assert pip 1 is `active`, pips 2-5 are `pending`.
   - On step 5, assert pips 1-4 are `completed`, pip 5 is `active`.
   - Run axe-core; assert the step indicator has exactly one accessible name "Step 5 of 5".

## Acceptance criteria

- [ ] On arrival at each wizard step, only completed prior steps show as fully lit; the current step shows as `active`.
- [ ] All pip states are visually distinct.
- [ ] Step indicator emits exactly one a11y text node with `aria-label="Step N of 5"`.
- [ ] axe-core passes on every wizard step.
- [ ] New Playwright spec passes.

## May-touch

- `web/components/auth/WizardEditorialShell.tsx` (pip rendering + a11y label)
- `web/app/(setup)/onboarding/welcome/page.tsx`
- `web/app/(setup)/onboarding/workspace/page.tsx`
- `web/app/(setup)/onboarding/connect-provider/page.tsx`
- `web/app/(setup)/onboarding/pick-vertical/page.tsx`
- `web/app/(setup)/onboarding/first-hire/page.tsx`
- `web/playwright/onboarding/wizard-pip-semantics-and-a11y.spec.ts` (NEW)

## Out of scope

- Wizard back-button behavior (covered by `fix-onboarding-wizard-back-button-step-2-jump-welcome`).
- Save-and-exit confirmation (covered by `fix-onboarding-save-and-exit-button-safety`).
- Step-5 first-hire submit logic (covered by sister briefs).

## Effort

~80-150 LOC + 1 spec. ~3-5 hours. P1 (F9) — design polish that ships the "Onboarding F1 editorial-center" design lock as intended. F15 is a P3 a11y nit folded into the same shell change.
