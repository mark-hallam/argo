# fix-onboarding-connect-provider-skip-mock-warning-callout

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-connect-provider-skip-mock-warning-callout`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/onboarding-hire-an-ai-teammate.md` F9 [P2].

🟡 **P2 — Skip button on `/onboarding/connect-provider` routes to a mock-provider experience. The warning text exists but is buried in body copy — easy to miss. Future viewers of the recording who try the wizard may not realise responses are simulated.**

## Root cause

`web/app/(setup)/onboarding/connect-provider/page.tsx:45-49, 148-151` — the Skip button calls `setProvider(false, true)` and pushes to `/onboarding/pick-vertical`. A small note at the bottom reads "Skipping uses a mock provider. Workers will run, but responses are simulated until you add a real key in Settings." Same visual register as the rest of the body — easy to miss.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F9 [P2] — Skip-to-mock-provider warning buried in body copy

## Fix recipe

1. **Move the warning into a yellow callout above the Skip button** in `connect-provider/page.tsx`. Suggested copy: "Mock provider — workers run but responses are simulated. Add a real key later in Settings · Providers."
2. **Use the existing warning-card component** (axe-checked `<AlertCard variant="warning">` or equivalent) for consistent styling.
3. **Web test** asserts the callout is visible above the Skip button (DOM ordering matters for the eye-line).

## Acceptance criteria

- [ ] Warning callout renders above Skip button with yellow accent.
- [ ] Copy matches "Mock provider — workers run but responses are simulated."
- [ ] No regression on other onboarding step renders.
- [ ] Web test covers the DOM ordering.

## May-touch

- `web/app/(setup)/onboarding/connect-provider/page.tsx` (callout placement)
- `web/components/skycot/AlertCard.tsx` (verify variant exists)
- `web/tests/onboarding-connect-provider.test.ts` (NEW or extend)

## Out of scope

- Mock provider runtime behavior
- Provider settings page (lives at `/settings/providers`)

## Effort

~20-30 LOC + 1 test. ~30min.
