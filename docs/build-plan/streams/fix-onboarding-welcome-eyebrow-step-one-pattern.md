# fix-onboarding-welcome-eyebrow-step-one-pattern

**Owner**: claude.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-welcome-eyebrow-step-one-pattern`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F3 (P2).

🟡 **P2 — Welcome step uses `eyebrow="WELCOME TO SKYCOT"` while every later step uses `eyebrow="STEP TWO · WORKSPACE"`, `STEP THREE · CONNECT AI PROVIDER`, etc. The step-strip already shows `STEP 01 OF 05`, so the inconsistent eyebrow reads as a typo, not a deliberate choice.**

## Root cause

`web/app/(setup)/onboarding/welcome/page.tsx:20` hard-codes `eyebrow="WELCOME TO SKYCOT"` while sibling step pages use the `STEP N · LABEL` pattern. Welcome IS the first step (step 1 of 5 per the step strip), but the eyebrow doesn't follow the pattern. The result is a 5-second double-take when the operator clicks past Welcome and sees the eyebrow shape change to `STEP TWO · ...`.

Dressing-only, but the wizard chrome is supposed to feel like one consistent experience — the eyebrow is the second-most-prominent element after the step strip.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F3 [P2] — Welcome step eyebrow breaks the `STEP X · LABEL` pattern

## Fix recipe

1. Edit `web/app/(setup)/onboarding/welcome/page.tsx:20`: change `eyebrow="WELCOME TO SKYCOT"` to `eyebrow="STEP ONE · WELCOME"`.
2. Verify visual: render `/onboarding/welcome` then `/onboarding/workspace` — both eyebrows now follow the same pattern (`STEP ONE · WELCOME`, `STEP TWO · WORKSPACE`).
3. Optional: extract the eyebrow into a `STEP_EYEBROW(stepNum, label)` helper if the same string concat shows up across 5 step pages — drop accidental drift for the next added step.

## Acceptance criteria

- [ ] `rg "eyebrow=" web/app/(setup)/onboarding/` shows all 5 step pages use the `STEP N · LABEL` pattern (with `STEP ONE · WELCOME`, `STEP TWO · WORKSPACE`, `STEP THREE · CONNECT AI PROVIDER`, `STEP FOUR · PICK VERTICAL`, `STEP FIVE · FIRST HIRE`).
- [ ] Visual diff: welcome step eyebrow matches the rhythm of the subsequent steps.

## May-touch

- `web/app/(setup)/onboarding/welcome/page.tsx` (~1 LOC string edit)
- (optional) a tiny helper in `web/app/(setup)/onboarding/_shared/eyebrow.ts` if extracting the pattern.

## Out of scope

- Other eyebrow inconsistencies on non-onboarding pages.
- The 3-pillar block that follows the eyebrow on the welcome page (separate brief: `fix-onboarding-welcome-pillars-bridge-not-repitch`).

## Effort

XS — 1-line edit. ~5 minutes. P2 / nit; ship in the next dressing-pass batch.
