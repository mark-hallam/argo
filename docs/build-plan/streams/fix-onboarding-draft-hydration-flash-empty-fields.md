# fix-onboarding-draft-hydration-flash-empty-fields

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-draft-hydration-flash-empty-fields`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/onboarding-hire-an-ai-teammate.md` F8 [P1].

🟠 **P1 — On hard reload of any wizard step, the input fields flash empty for ~100-200ms before the persisted localStorage values restore. The state IS preserved, but the visual preserve-and-restore is flaky. On a slow device or recorded screen capture the empty-then-full flash is visible.**

## Root cause

Zustand persist middleware at `web/lib/onboarding/use-onboarding-draft.ts:64-94` restores from localStorage asynchronously. On hard reload of any wizard step:

1. First paint: `useState(initialWorkspace)` reads `workspaceName: ""` (the INITIAL state).
2. Zustand persist middleware reads localStorage on mount.
3. Second paint: hydration completes, the saved value populates the field.

Result: a brief flash of empty inputs (typically 100-200ms but device-dependent). Standard zustand-persist hydration anti-pattern.

`web/app/(setup)/onboarding/workspace/page.tsx:26-27` reads the store's initial values via `useState`, which captures the un-hydrated default on first paint.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F8 [P1] — Persisted draft hydrates from localStorage AFTER first render — fields flash empty on hard reload

## Fix recipe

1. **Gate the form on hydration** at each wizard step that reads persisted draft state. Apply this pattern at `web/app/(setup)/onboarding/workspace/page.tsx:26-27` and any sibling step:
   ```tsx
   const hasHydrated = useOnboardingDraft.persist.hasHydrated();
   if (!hasHydrated) {
     return <WizardSkeleton />;
   }
   const workspaceName = useOnboardingDraft((s) => s.workspaceName);
   ```
2. **Build `WizardSkeleton`** (NEW) at `web/components/auth/WizardSkeleton.tsx` — a brand-restrained loader that matches the wizard shell's layout (hero typography placeholder, input placeholder rectangle, CTA button placeholder). Avoid `<Loader2 />` spinning; the skeleton blends visually with the final form.

3. **Verify with the zustand store factory** at `web/lib/onboarding/use-onboarding-draft.ts:65-110`. If the persist API doesn't expose `hasHydrated`, add an `onHydrationFinished` callback and track via local state:
   ```ts
   import { persist } from "zustand/middleware";
   const useDraft = create(persist(
     (set) => ({ ... }),
     {
       name: "onboarding-draft",
       onRehydrateStorage: () => (state) => { state?.setHasHydrated(true); },
     },
   ));
   ```

4. **Apply the same gate to step 5 (first-hire)** at `web/app/(setup)/onboarding/first-hire/page.tsx` so the prompt textarea doesn't flash empty either.

5. **Add a Playwright spec** `web/playwright/onboarding/draft-hydration-no-flash.spec.ts`:
   - Walk to step 2, fill workspace name.
   - Navigate to step 3, then hit Cmd-R (hard reload).
   - Assert the input field never shows empty during a 500ms window after page load.
   - One way: poll the input value every 16ms for 500ms; assert no frame shows the empty string after the skeleton transitions away.

## Acceptance criteria

- [ ] On hard reload of any wizard step, the form input fields never flash empty — they either show a skeleton or the persisted value.
- [ ] No regression on first-visit flow (un-persisted state continues to render the empty form immediately).
- [ ] New Playwright spec passes.
- [ ] Skeleton visually blends with the wizard shell (no jarring transition).

## May-touch

- `web/lib/onboarding/use-onboarding-draft.ts` (verify hasHydrated API)
- `web/components/auth/WizardSkeleton.tsx` (NEW)
- `web/app/(setup)/onboarding/workspace/page.tsx`
- `web/app/(setup)/onboarding/connect-provider/page.tsx`
- `web/app/(setup)/onboarding/pick-vertical/page.tsx`
- `web/app/(setup)/onboarding/first-hire/page.tsx`
- `web/playwright/onboarding/draft-hydration-no-flash.spec.ts` (NEW)

## Out of scope

- The wizard step pip semantics (covered by `fix-onboarding-wizard-step-pip-semantics-and-a11y`).
- Save-and-exit safety (covered by `fix-onboarding-save-and-exit-button-safety`).
- Wizard back-button (covered by `fix-onboarding-wizard-back-button-step-2-jump-welcome`).

## Effort

~80-150 LOC + 1 spec. ~3-5 hours. P1 — polish that ships the F1 editorial-center lock as intended on cold reload.
