# fix-onboarding-save-and-exit-button-destructive-rename

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-save-and-exit-button-destructive-rename`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/onboarding-hire-an-ai-teammate.md` F4 + F8.

🟡 **P1 — Top bar of every onboarding wizard step shows a button labeled "SAVE & EXIT". Clicking it opens an AlertDialog titled "Discard your in-progress teammate?" with a "Discard and leave" confirm. The button name (SAVE) directly contradicts what it does (discard). On the demo recording day a misclick destroys wizard state in front of the audience. Compounds with F8 — the wizard's `reset()` on submit destroys the same draft a second time, so the only chance to read back the user's typed prompt evaporates the moment the hire commits.**

## Root cause

`web/components/auth/WizardEditorialShell.tsx:357-360` renders the top-right button with label **`SAVE & EXIT`**. Lines 423-430 mount an `AlertDialog` whose body reads "Your draft will be cleared and you'll go back to the approvals inbox" with a destructive `Discard and leave` confirm. There is no save action — the dialog is pure destruction wrapped in a save label.

Separately, `web/app/(setup)/onboarding/first-hire/page.tsx:52` calls `reset()` (the zustand draft-store reset) immediately after the mutation resolves, which clears `firstHirePrompt`, `workspaceName`, `companyName`, `verticalId` etc from localStorage. So even outside demo mode, the moment hire commits, the wizard's draft is gone — the worker-overview page cannot read back what the user typed, which would have been the cheapest way to render "you described them as …".

Both bugs share the same underlying defect: the wizard treats "draft" as something that exists during edit and must be destroyed at boundary moments (exit OR submit), instead of treating it as data the user owns until they choose to discard.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F4 [P1] — SAVE & EXIT button is destructive, not saving
- onboarding-hire-an-ai-teammate F8 [P2] — All 5 step inputs are dropped on completion: `reset()` wipes the draft

## Fix recipe

1. **Rename the button to `EXIT`** at `WizardEditorialShell.tsx:357-360`. The draft already persists to localStorage via `useOnboardingDraft`'s zustand persist middleware — exit is non-destructive if you don't call `reset()`. Side effect: a returning user lands back where they left off, which is the expected behaviour for a wizard.
2. **Rewrite the AlertDialog copy** at `WizardEditorialShell.tsx:423-430`:
   - Title: "Leave onboarding?"
   - Body: "Your progress is saved. You can resume here next time. Pick **Discard** to clear the draft instead."
   - Buttons: primary `Save and exit` (just routes away, no reset), secondary destructive `Discard and leave` (calls reset + routes away).
3. **Defer the `reset()` call** at `first-hire/page.tsx:52`. Hold the draft for ~60 seconds post-hire (or until `/workers/<id>/overview` first-paints), then reset. This lets the worker-overview page (or a future `/onboarding/done` page) read `useOnboardingDraft().firstHirePrompt` to render "you described them as …". Simplest implementation: do not reset on submit; instead, schedule a `setTimeout(() => reset(), 60_000)` AND also reset on the next `welcome` step entry (next time the user starts a new wizard).
4. **Persist `firstHirePrompt` on every keystroke** (closes F12 partially — covered in the F12 brief if separate). The draft store already has the slot; the `first-hire/page.tsx` `<textarea>` should call `setFirstHirePrompt(e.target.value)` on `onChange`, not only on submit.

## Acceptance criteria

- [ ] Wizard top-right button reads `EXIT` (not `SAVE & EXIT`).
- [ ] Clicking EXIT opens an AlertDialog with title "Leave onboarding?" and two options: `Save and exit` (default, non-destructive) and `Discard and leave` (destructive, requires confirm).
- [ ] After completing the wizard and landing on `/workers/<id>/overview`, the draft remains in localStorage for at least 30 seconds (verifiable in DevTools Application → Local Storage).
- [ ] Starting a new wizard from `/onboarding/welcome` after a completed hire resets the draft cleanly (no stale `firstHirePrompt` carryover).
- [ ] Vitest pin: wizard EXIT default action does not call `reset()`; the destructive option does.

## May-touch

- `web/components/auth/WizardEditorialShell.tsx` (button label + dialog copy + button wiring)
- `web/app/(setup)/onboarding/first-hire/page.tsx` (defer reset)
- `web/app/(setup)/onboarding/welcome/page.tsx` (reset on new wizard entry)
- `web/lib/onboarding/use-onboarding-draft.ts` (no shape change; consumer change only)
- `web/components/auth/__tests__/WizardEditorialShell.test.tsx` (new pin)

## Out of scope

- Per-keystroke persistence of `firstHirePrompt` (covered by separate brief `fix-onboarding-wizard-step-back-loses-firsthire-prompt-and-api-key`)
- The first-hire success / celebration page (separate brief: `fix-onboarding-add-done-page-with-meet-your-teammate-handoff`)

## Effort

S — ~50-80 LOC across shell + page + tests. ~1-2 hours. P1 because demo-day misclick risk is high but the path is recoverable; pair with F1 demo-first-hire brief for max impact.
