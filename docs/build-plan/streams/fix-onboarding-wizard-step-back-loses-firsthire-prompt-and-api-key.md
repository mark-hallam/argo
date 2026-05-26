# fix-onboarding-wizard-step-back-loses-firsthire-prompt-and-api-key

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-wizard-step-back-loses-firsthire-prompt-and-api-key`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/onboarding-hire-an-ai-teammate.md` F12.

🟡 **P2 — Onboarding draft is persisted to localStorage on `setStep` / `setWorkspace` / `setVertical` / `setFirstHirePromptInStore` calls, so refresh-survival is good for most fields. BUT: the connect-provider step keeps `apiKey` in local component state (never on the draft), and the `firstHirePrompt` is only written to the store inside `onSubmit` (not on `onChange`). A user who navigates back from first-hire → pick-vertical → first-hire loses what they typed (only the last submitted prompt is restored). Same for any user who refreshes during the connect-provider step.**

## Root cause

`web/app/(setup)/onboarding/first-hire/page.tsx:34-45` declares `firstHirePrompt` as `useState` local to the page; the draft store's `setFirstHirePrompt` is only called inside `onSubmit` (not bound to the textarea's `onChange`). So intermediate typing never reaches localStorage.

`web/app/(setup)/onboarding/connect-provider/page.tsx:87` declares `apiKey` as `useState` local to the page; no draft-store slot exists for it (intentional, per the security memory — keys should not be persisted in localStorage indefinitely). But the absence of any persistence means navigating away mid-edit costs the user the key.

These are two related-but-different defects:
- **Prompt persistence**: should persist on `onChange` (debounced). Low security cost — the prompt is plain English.
- **Key persistence**: needs care. localStorage is wrong (security). Session-storage with a tab-lifetime is acceptable; or a sessionStorage encrypted-by-tenant-id wrap; or no persistence at all but a clearer visual warning "if you navigate away, paste the key again".

## Findings this brief closes

- onboarding-hire-an-ai-teammate F12 [P2] — Step-back navigation loses values in two of three transitions

## Fix recipe

**Part 1 — Persist `firstHirePrompt` on every keystroke (debounced).**

1. In `web/app/(setup)/onboarding/first-hire/page.tsx`, bind the textarea's `onChange` to call `setFirstHirePrompt(e.target.value)` directly (no debounce needed for localStorage). Drop the local `useState` for `firstHirePrompt` and read it from the draft store instead.
2. On mount, hydrate the textarea from `draft.firstHirePrompt` so refresh + back-navigation restore the value.
3. Vitest pin: typing into the textarea, calling `await waitFor(() => expect(useOnboardingDraft.getState().firstHirePrompt).toBe(typed))` resolves.

**Part 2 — Persist `apiKey` to sessionStorage (NOT localStorage).**

4. In `web/app/(setup)/onboarding/connect-provider/page.tsx`, on `apiKey` change call `sessionStorage.setItem('skycot:onboarding:apiKey', value)` (gated on `!isDemoMode()` — demo keys can use localStorage since they're mock).
5. On mount, hydrate `apiKey` from `sessionStorage.getItem(...)`.
6. On successful submit (or skip), call `sessionStorage.removeItem(...)`.
7. Add a small note next to the input: "Your key stays in this browser tab. Closing the tab clears it." This signals the security model.

## Acceptance criteria

- [ ] Typing in the first-hire textarea, navigating Back, then Forward — the typed prompt is still in the textarea.
- [ ] Typing in the provider-key input, refreshing the page — the typed key is still in the input. Closing the tab and reopening — the key is gone.
- [ ] Vitest pin: first-hire prompt is in `useOnboardingDraft.getState().firstHirePrompt` after each keystroke.
- [ ] In non-demo mode, the API key is never written to `localStorage` (verify via Playwright DevTools check).

## May-touch

- `web/app/(setup)/onboarding/first-hire/page.tsx` (bind onChange to store; drop local useState)
- `web/app/(setup)/onboarding/connect-provider/page.tsx` (sessionStorage hydrate + persist; teardown on submit/skip)
- `web/lib/onboarding/use-onboarding-draft.ts` (no shape change expected)
- `web/app/(setup)/onboarding/first-hire/__tests__/page.test.tsx` (new pin)
- `web/app/(setup)/onboarding/connect-provider/__tests__/page.test.tsx` (sessionStorage hydration pin)

## Out of scope

- The wizard back-button step-2-jump-welcome bug (covered by `fix-onboarding-wizard-back-button-step-2-jump-welcome.md`)
- Encrypting the sessionStorage entry (separate hardening pass)
- The SAVE & EXIT rename (covered by `fix-onboarding-save-and-exit-button-destructive-rename.md`)

## Effort

S — ~60-80 LOC across SPA + pins. ~1-2 hours. P2 because step-back is a polish path the demo recording doesn't traverse, but it's the cheapest path to making the wizard feel reliable to a real first user.
