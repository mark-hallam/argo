# fix-onboarding-workspace-step-celebration-hold-timer-jarring

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-onboarding-workspace-step-celebration-hold-timer-jarring`
**Source:** UX e2e audit 2026-05-28-0252 (onboarding F08)

🟡 **P2 — `/onboarding/workspace` celebrates with a toast ("Workspace created — welcome to Skycot.") then auto-pushes to `/onboarding/connect-provider` 1.2 seconds later. If the user is hovering the Continue button when the toast fires, the page navigates underneath them — mid-click can trigger a wrong-page action. The 1.2s magic number has no skip affordance for returning users.**

## Root cause

`web/app/(setup)/onboarding/workspace/page.tsx:16-49` defines `CELEBRATION_HOLD_MS = 1200` and uses `setTimeout` to push to `/onboarding/connect-provider` after the success toast. The toast itself communicates the success; gating navigation on a timer adds a forced wait with no skip path.

Failure modes:

- User hovering Continue mid-toast → click registers on the next page's CTA (wrong-page action).
- Slow connection extends the delay perceptually.
- Returning user re-doing the wizard hits the same 1.2s wait every time.

## Findings this brief closes

- onboarding F08 [P2] — Workspace step 1.2s celebration auto-push timer is jarring, no skip affordance

## Fix recipe

Pick **Path B** as the v1.0 fix; Path A is the minimum change.

**Path A (minimum) — shorten the hold to 600ms.** Halve the timer; less time for mid-click race. Doesn't fix the "no skip" UX.

**Path B (recommended) — keep the toast, drop the auto-push, let the user advance manually.**

1. **Remove the `setTimeout` push** at `web/app/(setup)/onboarding/workspace/page.tsx:47-49`.
2. **Keep the toast** ("Workspace created — welcome to Skycot.") firing on successful submit.
3. **Auto-focus the Continue button** so the user can press Enter to advance immediately — preserves the "no extra click needed" feel without the timer.
4. **Add a subtle "Press Enter to continue" affordance** under the toast or as a small italic hint near the Continue button.
5. **Returning-user shortcut**: if `useOnboardingDraft()` indicates the user has already completed this step (workspace exists), render a "Welcome back — let's pick up where we left off" mini-banner and auto-advance after 200ms (much shorter than the celebration hold). Returning users want speed; new users want the moment.

**Tests:**

6. **vitest** at `web/app/(setup)/onboarding/workspace/page.test.tsx`:
   - Mock successful submit; assert no auto-navigation occurs (Path B).
   - Assert Continue button has focus after the toast fires.
   - Pressing Enter triggers navigation.
7. **Playwright** at `web/playwright/onboarding/workspace-step-advance.spec.ts`:
   - Walk welcome → workspace; submit; assert URL stays on `/onboarding/workspace` until user presses Enter or clicks Continue.

## Acceptance criteria

- [ ] No `setTimeout`-driven auto-navigation in the workspace step on success path.
- [ ] Toast fires on successful submit and the Continue button auto-focuses.
- [ ] Returning users (workspace already exists) advance within ~300ms.
- [ ] vitest + playwright cover the manual-advance flow.
- [ ] No regression on the form-submit success path.
- [ ] PR body cites onboarding F08.

## May-touch

- `web/app/(setup)/onboarding/workspace/page.tsx` (remove the timer, ~10 LOC)
- `web/app/(setup)/onboarding/workspace/page.test.tsx` (NEW or extend)
- `web/playwright/onboarding/workspace-step-advance.spec.ts` (NEW)
- `web/components/skycot/onboarding/ContinueButton.tsx` (if focus-on-mount helper added, ~5 LOC)

## Out of scope

- The Pillar copy / step-strip semantics (covered by sibling onboarding-wizard polish briefs)
- Workspace name uniqueness checks (separate scope)
- Mobile viewport top-bar clipping (covered by `fix-onboarding-wizard-mobile-viewport-top-bar-clipping`)

## Effort

XS — ~10 LOC fix + ~50 LOC tests. ~1-2 hours.
