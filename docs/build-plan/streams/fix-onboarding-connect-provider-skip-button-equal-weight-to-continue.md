# fix-onboarding-connect-provider-skip-button-equal-weight-to-continue

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-onboarding-connect-provider-skip-button-equal-weight-to-continue`
**Source:** UX e2e audit 2026-05-28-0252 (onboarding F09)

🟡 **P2 — `/onboarding/connect-provider` renders "Skip for now" and "Validate & continue" as same-weight pills with their classes swapping based on whether the last validation error preferred skipping. An operator typing a wrong key gets a MORE prominent Skip button than Continue. Defensible UX (let them escape), but during the demo it reads as Skycot pushing customers into fake mode.**

## Root cause

`web/app/(setup)/onboarding/connect-provider/page.tsx:218-249` renders the two CTAs with their CSS classes (`ob-cta` for primary filled, `ob-cta-ghost` for outline-ghost) swapping based on `error.preferSkip`. The intent is recovery-friendly — if the validation fails in a way the user shouldn't retry, prefer Skip. But the visual weight inversion can read as the system endorsing Skip.

For an investor-watching scenario, the camera sees a more prominent SKIP button than the real-provider Continue button. Mismatch with the "we want you on a real provider" narrative.

Sibling `fix-onboarding-connect-provider-skip-mock-warning-callout` (MERGED #2372) made the mock-provider warning more prominent. This brief is the complement — keep the warning loud, restore the CTA hierarchy so Continue is always visually primary.

## Findings this brief closes

- onboarding F09 [P2] — Skip-for-now button gets same / greater visual weight than Continue under certain error states

## Fix recipe

1. **Remove the class swap** at `web/app/(setup)/onboarding/connect-provider/page.tsx:218-249`. The two CTAs always render with fixed classes:
   - **Continue**: `ob-cta` (filled, primary, gradient).
   - **Skip for now**: `ob-cta-ghost` or a new `ob-cta-text` link-shaped variant — outline-only, no fill.
2. **Keep the error-state messaging** — the validation error can still say "We couldn't validate that key — try a different one or skip for now" without inverting the CTA weight.
3. **Strengthen the mock-provider warning** (already done by sibling MERGED #2372) — the warning callout above the buttons remains the load-bearing signal that Skip = mock mode.
4. **Add a vitest assertion** at `web/app/(setup)/onboarding/connect-provider/page.test.tsx` (NEW or extend):
   - Render with `error.preferSkip = true`.
   - Assert `Continue` button has class `ob-cta` and `Skip for now` has class `ob-cta-ghost` (not the other way around).
5. **Storybook**: add a story for the post-validation-error state to confirm the hierarchy stays right.

## Acceptance criteria

- [ ] Continue button always renders as the filled primary CTA (`ob-cta`).
- [ ] Skip for now always renders as the outline-ghost or text-link variant.
- [ ] No class swap based on `error.preferSkip`.
- [ ] Mock-provider warning callout remains visible above the buttons.
- [ ] vitest pins the CTA hierarchy across error and no-error states.
- [ ] Storybook story for the error state.
- [ ] PR body cites onboarding F09.

## May-touch

- `web/app/(setup)/onboarding/connect-provider/page.tsx` (CTA class fix, ~5 LOC)
- `web/app/(setup)/onboarding/connect-provider/page.test.tsx` (NEW or extend)
- `web/app/(setup)/onboarding/connect-provider/page.stories.tsx` (extend)

## Out of scope

- Mock-provider warning callout (covered by `fix-onboarding-connect-provider-skip-mock-warning-callout` MERGED #2372)
- Multi-provider copy / options (covered by `fix-onboarding-connect-provider-multi-provider-copy-or-options`)
- Test-call claim honesty (covered by `fix-onboarding-connect-provider-test-call-claim-honest`)
- Provider key 500 error friendliness (covered by `fix-onboarding-provider-key-500-friendly-error`)

## Effort

XS — ~5 LOC fix + ~40 LOC test + storybook variant. ~30-45 min.
