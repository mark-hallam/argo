# fix-onboarding-welcome-pillars-bridge-not-repitch

**Owner**: claude.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-welcome-pillars-bridge-not-repitch`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F4 (P2).

🟡 **P2 — `/onboarding/welcome` lists three feature pillars ("Audit-first", "Onboard them like a human", "Enterprise-grade") in the same shape as the `/landing` value-prop strip the user just came from. A user who has *chosen* to sign up doesn't need a second sales beat — the welcome screen should *bridge into doing*, not re-pitch.**

## Root cause

`web/app/(setup)/onboarding/welcome/page.tsx:36-56` re-uses the marketing strip's three-pillar pattern verbatim. The pillars are identical in copy to what sits on `/landing` — same lines, same icons, same order. The information is duplicate; the cognitive load is doubled (the user has to re-read content they already absorbed); the chance to reduce friction at the start of doing-mode is wasted.

The welcome step should answer two questions:
1. **What happens next?** ("Here's the 4-step path: name workspace → connect AI provider → pick template → hire.")
2. **How long?** ("~4 minutes to your first teammate.")

Not "let me pitch you the product again."

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/onboarding-hire-an-ai-teammate.md` F4 [P2] — Three feature pillars overlap with `/landing` pillars verbatim

## Fix recipe

1. Edit `web/app/(setup)/onboarding/welcome/page.tsx:36-56` to replace the 3 marketing pillars with a 4-bullet "here's what we'll do next" preview matching the wizard step list:
   ```
   STEP 2 · Name your workspace · 30 sec
   STEP 3 · Connect AI provider · 1 min
   STEP 4 · Pick a vertical template · 1 min
   STEP 5 · Describe your first teammate · 90 sec
   ```
2. Keep the visual rhythm (3 or 4 columns, same icon style — the pattern is fine, the *content* changes from marketing-claim to do-mode preview).
3. Anchor with a single time-promise eyebrow under the hero: "About 4 minutes."
4. Operator can choose: a) keep one of the pillars + replace two ("Audit-first" alone as the trust anchor + the 4-bullet preview), or b) full replacement.

## Acceptance criteria

- [ ] Welcome page no longer duplicates the `/landing` pillars verbatim — pillar copy differs from the marketing strip.
- [ ] User sees a preview of the 4 remaining steps before they start.
- [ ] Time-to-first-hire promise is visible somewhere on the welcome screen.
- [ ] No regression to the locked F1 editorial-center layout (Audiowide hero unchanged, gradient stays only on accent lines).

## May-touch

- `web/app/(setup)/onboarding/welcome/page.tsx` (~30 LOC content edit; layout untouched)

## Out of scope

- The eyebrow text itself (covered by `fix-onboarding-welcome-eyebrow-step-one-pattern`).
- The step strip rendering (already shipped).
- Adding telemetry on welcome-step engagement.

## Effort

S — ~20 minutes content rewrite + design review. P2 because the welcome screen is the camera's first beat in Scene 3 of the runbook; the experience improvement is "softer landing into doing-mode", not a function fix.
