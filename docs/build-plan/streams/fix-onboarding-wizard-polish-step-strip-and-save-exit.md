# fix-onboarding-wizard-polish-step-strip-and-save-exit

**Owner**: claude.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-wizard-polish-step-strip-and-save-exit`.
**Source**: e2e-ux-audit (onboarding-hire-an-ai-teammate) findings F8 + F10 — `docs/skycot/ux-audits/2026-05-26-0820/onboarding-hire-an-ai-teammate.md`.

🟡 **P2 — Wizard step-strip eyebrow inconsistency between welcome and the four numbered steps + SAVE & EXIT silently discards draft state without confirming.**

## Root cause

Two small wizard-chrome polish gaps:

**F8 — step-strip eyebrow drift**:
The wizard's `WizardEditorialShell` (`web/components/auth/WizardEditorialShell.tsx:348`) renders "STEP 01 OF 05" pips. But the body eyebrow text on each step is inconsistent:
- Step 1 (welcome) eyebrow = `WELCOME TO SKYCOT` — no step indicator
- Step 2 (workspace) eyebrow = `STEP TWO · WORKSPACE`
- Step 3 (connect-provider) eyebrow = `STEP THREE · CONNECT AI PROVIDER`
- Step 4 (pick-vertical) eyebrow = `STEP FOUR · PICK A VERTICAL`
- Step 5 (first-hire) eyebrow = `STEP FIVE · YOUR FIRST HIRE`

The mismatch is camera-visible on a wide shot — viewer sees all 5 pips marked "STEP 01 OF 05" but the welcome page itself doesn't carry the step indicator the other 4 do.

**F10 — SAVE & EXIT discards draft without confirming**:
`WizardEditorialShell.tsx:310` only triggers the confirm dialog when `unsavedHire === true`. But `_OnboardingShell.tsx` never passes that prop. So on every onboarding step, clicking SAVE & EXIT (or the SKYCOT brand wordmark) takes the user straight to `/` without warning. The zustand persist DOES keep the draft, so users can resume — but the UX feels like data loss.

Reproduction: fill workspace name → click SAVE & EXIT → land on `/` (Approvals page) → browser-back returns to wizard with draft intact, but the user has no idea their work was saved.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F8 [P2] — Welcome eyebrow `WELCOME TO SKYCOT` doesn't match the `STEP TWO · WORKSPACE` pattern of steps 2-5
- onboarding-hire-an-ai-teammate F10 [P2] — SAVE & EXIT discards in-progress draft (visually) with no confirmation; brand wordmark click has same issue

## Fix recipe

**For F8** — eyebrow consistency:
1. Touch `web/app/(setup)/onboarding/welcome/page.tsx` — change the `eyebrow` prop on `<WizardEditorialShell>` from `"WELCOME TO SKYCOT"` to `"STEP ONE · WELCOME"`.
2. Move "WELCOME TO SKYCOT" into the body hero copy (so the brand framing is still visible, just not as the step eyebrow).
3. Visual smoke: run `curl -s http://localhost:3000/onboarding/welcome | rg -o 'STEP [A-Z]+ · [A-Z]+'` — should print `STEP ONE · WELCOME`.

**For F10** — SAVE & EXIT confirmation:
1. Touch `web/app/(setup)/onboarding/_OnboardingShell.tsx`. Read the current draft state from `useOnboardingDraft()` and compute:
   ```ts
   const hasUnsavedDraft = Boolean(
     workspaceName || verticalId || firstHirePrompt || providerKey
   );
   ```
2. Pass `unsavedHire={hasUnsavedDraft}` into `<WizardEditorialShell>` so the existing confirm-dialog branch lights up.
3. While there, audit `WizardEditorialShell.tsx:310` — the prop is named `unsavedHire` but the gating applies to any wizard step, not just first-hire. Rename to `hasUnsavedChanges` for clarity (cascade through callers).
4. Add a toast on confirmed exit: "Draft saved — resume from /onboarding/welcome whenever you're ready." Uses the existing sonner toast system. This addresses the "feels like data loss" issue even when the user clicks through the confirm.
5. Wire the brand wordmark click (top-left SKYCOT logo) through the same exit-with-confirm path — currently it's a bare `<Link>`.

## Acceptance criteria

**F8**:
- [ ] `/onboarding/welcome` eyebrow renders `STEP ONE · WELCOME` (not `WELCOME TO SKYCOT`).
- [ ] All 5 wizard steps follow the same eyebrow pattern: `STEP {ONE..FIVE} · {LABEL}`.
- [ ] Brand "WELCOME TO SKYCOT" copy is preserved in the body hero of the welcome page (not lost).

**F10**:
- [ ] Filling a step's field then clicking SAVE & EXIT triggers a confirm dialog ("Save and exit? Your draft will be available when you return.").
- [ ] Confirming the dialog → toast: "Draft saved — resume from /onboarding/welcome".
- [ ] Cancelling the dialog → stay on the current step with field state preserved.
- [ ] Clicking the SKYCOT brand wordmark mid-wizard triggers the same confirm path.
- [ ] Reload + return to `/onboarding/welcome` resumes from the persisted draft (existing behavior — no regression).
- [ ] Prop name `unsavedHire` renamed to `hasUnsavedChanges` across `WizardEditorialShell` callers.

## May-touch

- `web/app/(setup)/onboarding/welcome/page.tsx` (eyebrow)
- `web/app/(setup)/onboarding/_OnboardingShell.tsx` (unsaved-draft prop)
- `web/components/auth/WizardEditorialShell.tsx` (rename prop, gate the wordmark click)
- `web/lib/onboarding/draft.ts` (if `useOnboardingDraft` exposes the field for the hasUnsavedDraft computation; add if missing)

## Out of scope

- Wizard validation gates on Continue (already wired; no change)
- Multi-provider copy variants (`fix-onboarding-connect-provider-multi-provider-copy-or-options`)
- Hire flow → onboarding flow unification (`fix-onboarding-hire-parallel-wizards-unify-or-document`)

## Effort

~40-80 LOC. ~1 hour. Bundle two small P2s into one PR.
