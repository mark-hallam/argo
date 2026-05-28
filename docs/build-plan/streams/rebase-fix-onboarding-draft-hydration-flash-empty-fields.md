# rebase-fix-onboarding-draft-hydration-flash-empty-fields

Re-implement the original brief
`streams/fix-onboarding-draft-hydration-flash-empty-fields.md` against
current `origin/main` HEAD.

## Original PR

PR #2599 (closed for re-author) — `fix(onboarding): gate wizard inputs
on zustand-persist hasHydrated (F8)`, opened 2026-05-28 by claude.
Conflicted on two key files because the onboarding draft store +
first-hire page have evolved substantially since the PR was authored:

- `web/lib/onboarding/use-onboarding-draft.ts` — 15 conflict markers
  (5 hunks). Current main shipped `selectedPlan: SelectedPlan | null`
  + `setSelectedPlan` (the pricing-tier preselect from `/pricing`).
  PR #2599's `hasHydrated` + `setHasHydrated` additions landed on the
  same five line ranges (state shape declaration, setter signature,
  partialize list, INITIAL block, store body). Both fields are
  net-additive but git's auto-merge can't reconcile the parallel
  insertions.
- `web/app/(setup)/onboarding/first-hire/page.tsx` — 3 conflict
  markers (1 hunk). The PR added a local `EXAMPLE_PROMPT` constant
  and split the page into `<FirstHirePage>` + `<FirstHireBody>` so
  `useState(initialPrompt)` captures the rehydrated draft. Current
  main uses `exampleForVertical(verticalId)` (post-#2619) and reads
  the prompt straight from the draft store (post-#2477), so the
  `useState`-capture pattern from the PR no longer matches the file's
  shape.

The other 5 touched files (`connect-provider/page.tsx`,
`pick-vertical/page.tsx`, `workspace/page.tsx`, the demo-mode-relabel
test, and the new Playwright spec) auto-merged but only because they
sit on lines main hasn't touched recently — they would still need
re-validation against the post-rebase shape.

## Authored against: origin/main `a5df90c41` on 2026-05-29

## Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-onboarding-draft-hydration-flash-empty-fields

## Bug is still load-bearing

UX-AUDIT-T2152MEL F8 [P1] is still live. The zustand-persist
middleware in `web/lib/onboarding/use-onboarding-draft.ts:78` reads
from localStorage asynchronously after the React tree first paints;
on hard reload of `/onboarding/workspace`, `/connect-provider`,
`/pick-vertical`, or `/first-hire`, the input components mount with
an empty-string `useState` capture and then re-render with the
rehydrated value once persist resolves. On a slow device or recorded
screen capture, the empty-then-full flash is visible. This is a
demo-blocker for the v1.0 MP4 if the operator chooses to reload
mid-recording.

## Acceptance (same as original brief)

- `web/lib/onboarding/use-onboarding-draft.ts`:
  - Add `hasHydrated: boolean` field to state (`false` initial,
    excluded from `partialize`).
  - Add `setHasHydrated(hasHydrated: boolean)` setter.
  - Wire `onRehydrateStorage` in the persist middleware config to
    call `setHasHydrated(true)` once rehydration completes.
  - `reset()` flips `hasHydrated: true` (test reset path stays
    rendering the form, not the skeleton).
- Each of the four affected pages splits into a top-level skeleton
  gate + a `*FormBody` child:
  - `/onboarding/workspace/page.tsx`
  - `/onboarding/connect-provider/page.tsx` (no persisted input but
    still gated for chrome consistency)
  - `/onboarding/pick-vertical/page.tsx`
  - `/onboarding/first-hire/page.tsx`
  - The body's `useState(initialFoo)` captures the rehydrated value
    on first render. Early-return inside one component leaves the
    un-hydrated `useState` capture in place — exactly the flash we
    are fixing.
- New Playwright spec at `web/tests/e2e/onboarding-draft-hydration-no-flash.spec.ts`:
  seed a persisted draft into localStorage, navigate cold to
  `/onboarding/workspace`, poll the workspace-name input every ~16ms
  for 500ms. Assert: no frame shows the input present with an empty
  value (skeleton-only frames OK; populated frames OK). Input ends
  the window populated.
- Adjacent vitest in
  `web/app/(setup)/onboarding/pick-vertical/__tests__/demo-mode-relabel.test.tsx`
  needs `hasHydrated: true` in its hoisted mock state so it renders
  the form instead of the skeleton.

## Shape of main on touched files

- `web/lib/onboarding/use-onboarding-draft.ts` (current 215 LOC) —
  carries `selectedPlan: SelectedPlan | null` (from `/pricing` tier
  preselect) + `setSelectedPlan` + `partialize` keys including
  `selectedPlan`. The `hasHydrated` additions sit ALONGSIDE these
  (not replacing), each on its own line.
- `web/app/(setup)/onboarding/first-hire/page.tsx` (current 235 LOC) —
  uses `useOnboardingDraft((s) => s.firstHirePrompt)` selector +
  `exampleForVertical(pickedVerticalId)` for the placeholder. The
  `FirstHireBody` split keeps the same selector / placeholder logic;
  the `EXAMPLE_PROMPT` constant from the original PR is NOT needed
  (use `exampleForVertical`).
- The 3 other page files (`workspace`, `connect-provider`,
  `pick-vertical`) auto-merge against current main — the split
  pattern is straightforward there.
- `web/__tests__/onboarding/wizard.test.tsx` (separate from the touched
  list — but heavily exercises the same flow) — verify it still
  passes after `hasHydrated: false` initial; if it breaks, add a
  hoisted-mock override in the same shape as the demo-mode-relabel
  test fix.

## Notes for the worker

- LOC budget: ~250 LOC across 7 files. Pages-as-shell + body
  pattern keeps each page under 80 LOC (HARD RULE 6).
- The `onRehydrateStorage` setter wiring in zustand-persist is the
  load-bearing trick — its callback receives the rehydrated state +
  is called AFTER React first paint. Calling `setHasHydrated(true)`
  there is what flips the gate.
- The skeleton: use the existing `WizardSkeleton` component
  (`web/components/skycot/onboarding/WizardSkeleton.tsx` — check
  current export name and shape; the original PR referenced it but
  the file might have moved).
- Playwright `testDir` is `./tests/e2e`, NOT `./playwright/onboarding/`.
  The spec lives at `web/tests/e2e/onboarding-draft-hydration-no-flash.spec.ts`.
- The freshness gate on the playwright test: the test creates its own
  persisted draft blob via `page.addInitScript` before navigation, so
  it doesn't depend on the wizard having been used previously.
