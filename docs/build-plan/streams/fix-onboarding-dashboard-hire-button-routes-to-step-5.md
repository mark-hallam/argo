# fix-onboarding-dashboard-hire-button-routes-to-step-5

**Owner**: claude.
**Authored against**: origin/main `4585055d32eba8a24f5872354c0225246c0aa250` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-dashboard-hire-button-routes-to-step-5`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0623/onboarding-hire-an-ai-teammate.md` F4 [P1].

🟠 **P1 — The dashboard's top-right "Hire teammate" button at `web/app/(app)/page.tsx:460` calls `router.push("/onboarding/first-hire")` — i.e. step 5 of the onboarding wizard, the LAST step. Since steps 1-4 haven't been completed, the `useOnboardingDraft` state has `verticalId: null`. Clicking Hire then silently fails ("Pick a template first"), with no visible error. The wizard's progress bar reads "STEP 05 OF 05" but the surrounding context is meaningless. This is a different bug from the older `/hire/step/1` issue (covered by `fix-in-product-hire-ctas-rewire-to-onboarding-canonical`) — that brief addressed the parallel-wizard problem; this one is the new state of the button after that wiring change.**

## Root cause

`web/app/(app)/page.tsx:460`:

```tsx
onClick={() => router.push("/onboarding/first-hire")}
```

The destination `/onboarding/first-hire` is the FIFTH step of the onboarding-welcome → workspace → connect-provider → pick-vertical → first-hire wizard. The wizard uses `useOnboardingDraft` (`web/lib/onboarding/use-onboarding-draft.ts`) for shared state across steps:

- Step 2 sets `workspaceName`
- Step 3 sets `providerKey` / `mockProvider`
- Step 4 sets `verticalId`
- Step 5 reads `verticalId` and uses it to seed the cohort + skills

Skipping straight to step 5 leaves `verticalId === null`. The submit handler at `web/app/(setup)/onboarding/first-hire/page.tsx` checks `verticalId` before POSTing — when null, it returns silently or sets a soft "Pick a template first" announcement that's a11y-only (no visible toast). The user clicks Hire, nothing happens, and the only feedback is the announcement screen-reader text.

The intent at the dashboard hire button was to enter the "first hire" wizard. But:

- For a fresh tenant that hasn't finished onboarding, the wizard should start at `/onboarding/welcome` and walk all 5 steps.
- For a tenant that's already past onboarding, the right destination is the SECOND hire surface (`/hire` or `/genesis`), NOT the onboarding-first-hire step.

Currently the button conflates the two and lands on step 5 with empty state, which is invalid for both cases.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F4 [P1] — Approvals dashboard "Hire teammate" button skips steps 1-4 of the onboarding wizard

## Fix recipe

1. **Decide the right destination based on onboarding completion state** in `web/app/(app)/page.tsx:460`:
   ```tsx
   const { hasCompletedOnboarding } = useOnboardingStatus();
   onClick={() => {
     if (hasCompletedOnboarding) {
       router.push("/hire"); // or "/genesis" once that surface is the canonical second-hire entry
     } else {
       router.push("/onboarding/welcome"); // start at step 1, not step 5
     }
   }}
   ```
2. **Add the `useOnboardingStatus` hook** if it doesn't exist — reads a tenant-scoped completion flag (workspace + provider + at-least-one-worker). For demo mode, default to `hasCompletedOnboarding=true` so the button on `/` in the recording config goes to `/hire` not back into the wizard.
3. **As a defence-in-depth**, in `first-hire/page.tsx` the submit-with-null-verticalId path should redirect the user back to `/onboarding/pick-vertical` instead of silently failing. Currently it sets an a11y announcement only.
4. **Vitest pin**: render `(app)/page.tsx` with `useOnboardingStatus` mocked to `hasCompletedOnboarding=true` → assert the button click pushes `/hire`. Same test with `false` → asserts push to `/onboarding/welcome`.

## Acceptance criteria

- [ ] Dashboard "Hire teammate" button does NOT route to `/onboarding/first-hire` when the tenant has not completed onboarding.
- [ ] Dashboard button routes to `/onboarding/welcome` for first-run tenants and `/hire` (or `/genesis`) for completed tenants.
- [ ] If a user does land on `/onboarding/first-hire` with null `verticalId`, the page redirects to `/onboarding/pick-vertical` rather than silently failing the submit.
- [ ] Vitest covers both branches of the routing logic.

## May-touch

- `web/app/(app)/page.tsx:459-462` (the Hire button)
- `web/lib/onboarding/use-onboarding-status.ts` (NEW or update — completion flag hook)
- `web/app/(setup)/onboarding/first-hire/page.tsx` (defence-in-depth redirect)
- `web/app/(app)/__tests__/page.test.tsx` (vitest pin)

## Out of scope

- The parallel-wizard problem `/hire/step/1` vs `/onboarding/first-hire` (covered by `fix-in-product-hire-ctas-rewire-to-onboarding-canonical` and `fix-onboarding-hire-parallel-wizards-unify-or-document`).
- The first-hire demo-mode short-circuit (covered by `fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect`).
- Genesis discoverability (covered by `fix-genesis-orphan-discoverability-and-provider-key-503`).

## Effort

~50-100 LOC: button rewire + hook + vitest + first-hire defence-in-depth. ~1-2 hours. P1 because the dashboard "Hire teammate" CTA is one of the camera-visible affordances; clicking it in a recording dry-run lands on a wizard step that silently no-ops.
