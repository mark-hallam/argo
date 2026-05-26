# fix-onboarding-save-and-exit-button-safety

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-save-and-exit-button-safety`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/onboarding-hire-an-ai-teammate.md` F10 [P2].

🟡 **P2 — Top-bar SAVE & EXIT button (visible on all 5 wizard pages) is declared as a plain `<button>` without `aria-haspopup` or `aria-controls`. Operator clicking it during a demo recording could blow away the run if the side-effect is unclear. Destination is not labeled.**

## Root cause

The SAVE & EXIT button in the wizard top bar (in `web/app/(setup)/onboarding/_OnboardingShell.tsx` or sibling) is rendered as:
```tsx
<button onClick={handleSaveAndExit}>SAVE & EXIT</button>
```

No `aria-haspopup` (does it open a confirm dialog?), no `aria-controls`, no visible target route (where does "exit" go?), no confirmation step before discarding wizard progress. A demo-recording click could destroy state silently.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F10 [P2] — `Save & exit` button has no `aria-haspopup`, no confirmation, no visible target route

## Fix recipe

1. **Define explicit behaviour for SAVE & EXIT**:
   - Save current form state (sessionStorage) so the operator can resume on next visit
   - Navigate to `/workers` (if any workers exist) or `/landing` (if not)
   - Show a confirmation modal first if the operator has unsaved input that hasn't yet been persisted to the gateway

2. **Implement the confirmation modal** (use existing modal pattern):
   ```tsx
   <ConfirmDialog
     trigger={<Button variant="ghost">Exit setup</Button>}
     title="Save setup and exit?"
     body="We'll save your progress so you can resume later."
     onConfirm={() => { saveProgress(); router.push(destination); }}
     confirmLabel="Save & exit"
     cancelLabel="Stay in setup"
   />
   ```

3. **Add ARIA attributes** for screen readers:
   - `aria-haspopup="dialog"` on the trigger button
   - `aria-controls="save-exit-dialog"` linking to the dialog id

4. **Rename button label** from "SAVE & EXIT" to "Exit setup" — the destination is the operator's prior context, and "save" implies an explicit form save which the wizard already does between steps.

5. **Add a Playwright spec** at `web/playwright/onboarding/save-exit-button.spec.ts`:
   - Click Exit setup → confirmation modal appears
   - Click Stay → modal closes, URL unchanged
   - Click Confirm → URL changes to `/workers` or `/landing`, form state persisted to sessionStorage
   - Re-visit `/onboarding/workspace` → form is pre-filled from saved state

## Acceptance criteria

- [ ] SAVE & EXIT button renamed to "Exit setup"
- [ ] Click opens a confirmation modal (or for low-friction option: shows a toast "Your progress is saved" and navigates immediately)
- [ ] Operator cannot lose wizard state without a confirmation step
- [ ] ARIA attributes present
- [ ] New Playwright spec passes

## May-touch

- `web/app/(setup)/onboarding/_OnboardingShell.tsx` (button + modal trigger)
- `web/components/ui/ConfirmDialog.tsx` (use existing or add if missing)
- `web/lib/onboarding/use-save-progress.ts` (NEW)
- `web/playwright/onboarding/save-exit-button.spec.ts` (NEW)

## Out of scope

- The Resume banner on the destination page (`fix-onboarding-wizard-back-button-step-2-jump-welcome` Option B may add this)
- Mobile-viewport hamburger fold-in (covered by `fix-onboarding-wizard-mobile-viewport-top-bar-clipping`)

## Effort

~80-150 LOC + 1 spec. ~2-3 hours. P2 polish but important safety net.
