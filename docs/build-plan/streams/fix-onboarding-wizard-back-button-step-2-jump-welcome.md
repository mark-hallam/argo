# fix-onboarding-wizard-back-button-step-2-jump-welcome

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-wizard-back-button-step-2-jump-welcome`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/onboarding-hire-an-ai-teammate.md` F8 [P1].

🟡 **P2 — Wizard back-button on step 2 (`/onboarding/workspace`) routes to `/onboarding/welcome`, a marketing-style page. Workspace provisioning is already complete at this point, so going back to a welcome screen is jarring. Most multi-step wizards either disable back on step 1 of substantive form OR step back to the previous in-form state.**

## Root cause

`web/app/(setup)/onboarding/workspace/page.tsx:110`:
```ts
onClick={() => router.push("/onboarding/welcome")}
```

The wizard's first substantive step (workspace name + company name) treats `/onboarding/welcome` as the "previous step" — but welcome is a marketing landing, not a form. Going back from workspace step erases the operator's work (workspace name + company name) and dumps them on a sales pitch.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F8 [P1] — Wizard back-button on step 2 jumps to marketing welcome instead of meaningful previous state

## Fix recipe

Two acceptable options — pick one:

**Option A (recommended): remove the back button on step 2.** Step 2 is the first substantive form. There's no meaningful "previous" to navigate to. Replace the back-arrow button with no button, OR with a "Restart" CTA that prompts confirmation before discarding state.

**Option B: turn the back button into a "Save & exit" target.** If the operator clicks it, save their progress (workspace name + company name to sessionStorage), navigate to `/landing` or wherever they came from, and have a "Resume setup" banner on next visit.

In `web/app/(setup)/onboarding/workspace/page.tsx:106-114`:
- Option A: remove the back button entirely OR replace with `<Button variant="ghost" onClick={confirmRestart}>Restart setup</Button>`
- Option B: extract to a `useSaveAndExit()` hook that handles the persistence + redirect

## Acceptance criteria

- [ ] Clicking back on `/onboarding/workspace` either: (a) does nothing meaningful (Option A; button removed), or (b) saves + exits with a banner-style resume mechanism (Option B)
- [ ] Operator never lands on `/onboarding/welcome` from a form step with progress already entered
- [ ] No data loss without explicit confirmation
- [ ] Playwright spec covers the chosen behaviour

## May-touch

- `web/app/(setup)/onboarding/workspace/page.tsx:106-114`
- `web/lib/onboarding/use-save-and-exit.ts` (NEW, if Option B)
- `web/playwright/onboarding/back-button-behavior.spec.ts` (NEW)

## Out of scope

- The Save & Exit top-bar button (covered by `fix-onboarding-save-and-exit-button-safety`)
- Mobile layout for the back button (covered by `fix-onboarding-wizard-mobile-viewport-top-bar-clipping`)
- The decision between A and B is operator-decisional; pick A unless the operator overrides

## Effort

~20-80 LOC. ~1-2 hours. P2 polish, but unblocks any specs that assert wizard back-navigation behaviour.
