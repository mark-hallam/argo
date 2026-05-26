# fix-marketing-cta-teammate-vs-worker-vocab-drift

**Owner**: claude or codex.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-marketing-cta-teammate-vs-worker-vocab-drift`.
**Source**: ux-e2e-audit-T2309MEL `onboarding-hire-an-ai-teammate.md` F5 [P1] + `whole-site-mvp-claims-coverage.md` F7 [P2].

🟠 **P1 — The "teammate not tool" thesis is Skycot's load-bearing brand promise (CLAUDE.md, MISSION.md Claim 1, brand voice memory). Yet `/for/sales` (and likely the 4 sibling vertical landings) say "Hire your first worker" while `/landing` says "Hire your first teammate". Vocabulary drift undercuts the load-bearing differentiator. Camera-visible if any vertical landing is in the demo.**

## Root cause

Captured by the audit across 6 surfaces:

- `/landing` top-bar CTA + hero CTA: **"Hire your first teammate"**
- `/for/sales` top-bar CTA: **"Hire your first worker"** (likely identical on `/for/{finance,marketing,ops,recruiting,support,marketplace}`)
- `/onboarding/welcome` body CTA: **"Get started"**
- `/hire` Step 1 body CTA: **"Continue"**
- `/hire` Step 4 body CTA: **"Hire {name}"** (e.g. "Hire Mariah")
- `/onboarding/first-hire` body CTA: **"Hire them"**

"Worker" is correctly the **in-product / runtime / SQL primitive** (nav item, `crates/argo-runtime/src/worker/`, `pub struct Worker`, etc.). Marketing surfaces should standardize on "teammate" per `project_skycot_brand_identity` and the operator-locked Claim 1 ("Teammate, not tool").

The cohesive in-funnel CTA pattern should be:

| Surface | CTA |
|---|---|
| Marketing landings (`/`, `/landing`, `/for/*`, `/pricing`, `/compare`) | **"Hire your first teammate"** |
| Authenticated empty state (`/team`, `/workers`) | **"Hire a teammate"** |
| Hire wizard step 1 | **"Continue"** |
| Hire wizard step 4 (final commit) | **"Hire {Name}"** |

## Findings this brief closes

- onboarding-hire-an-ai-teammate F5 [P1] — Inconsistent CTA "teammate" vs "worker"
- whole-site-mvp-claims-coverage F7 [P2] — `/for/sales` reverts to "worker" + vertical landing CTA drift

## Fix recipe

1. **Grep + rewrite.** Find every CTA-shaped string mentioning "Hire your first worker" on a marketing/vertical landing page:

   ```bash
   rg "Hire your first worker" web/app/{landing,for,marketing,about,contact,pricing,compare,security,changelog,customers}/
   rg "Hire a worker" web/app/{landing,for,...}/
   ```

   Replace each with the equivalent "teammate" copy. Verify against the table above.

2. **Specific files (per audit + grep)**:
   - `web/app/for/sales/page.tsx` — top-bar + hero CTAs
   - `web/app/for/finance/page.tsx`, `marketing/page.tsx`, `ops/page.tsx`, `recruiting/page.tsx`, `support/page.tsx`, `marketplace/page.tsx` — same pattern
   - Any vertical-landing CTA component that wraps the button (check `web/components/skycot/marketing/`)

3. **Tighten the rule with a vitest test.** Author `web/__tests__/marketing-cta-vocab.test.tsx` that:
   - Imports every `web/app/(?:landing|for|pricing|compare|customers)/**/page.tsx` source as text
   - Asserts none contain `"Hire your first worker"` or `"Hire a worker"` (case-insensitive)
   - Allows `"Hire {name}"` (the in-wizard commit button) and `"Workers"` (runtime/nav primitive)

4. **Leave `/team` and `/workers` nav rows alone.** "Workers" is the canonical runtime word; the nav still says Workers. This brief is about VERB/CTA copy on outward-facing marketing surfaces.

5. **Brand voice consistency.** Per `project_skycot_brand_identity`: "teammate" is the load-bearing word. Adjacent body copy on the same vertical-landings should also be checked for "worker" used as a synonym for "teammate" (vs. legitimate use as the runtime entity name).

## Acceptance criteria

- [ ] `rg 'Hire your first worker' web/app/{landing,for,pricing,compare,customers,about,contact,security,changelog}/` returns 0 matches
- [ ] `rg 'Hire a worker' web/app/{landing,for,pricing,compare,customers,about,contact,security,changelog}/` returns 0 matches
- [ ] `/for/sales` and `/landing` both display "Hire your first teammate" as the primary CTA
- [ ] In-wizard `/hire/step/4` still says "Hire {Name}" — unchanged
- [ ] `Workers` sidebar nav row label unchanged (in-product runtime word)
- [ ] New vitest spec passes
- [ ] `pnpm --dir web exec tsc --noEmit` green

## May-touch

- `web/app/for/sales/page.tsx` + 5 sibling vertical landings (~6 × 2 LOC string edits)
- Any shared CTA component under `web/components/skycot/marketing/` (~5 LOC if any)
- `web/__tests__/marketing-cta-vocab.test.tsx` (~40 LOC new test)

## Out of scope

- Renaming the in-product "Workers" sidebar row or runtime/SQL primitive (the audit explicitly allows that)
- The wider /hire vs /onboarding wizard unification (covered by `fix-onboarding-hire-parallel-wizards-unify-or-document`)
- Audiowide hero copy ("Build your AI workforce" — that's brand-loud, not CTA copy)

## Effort

S. ~25 LOC string edits + ~40 LOC test. ~30 min. P1 because the load-bearing brand differentiator is undercut on every vertical-landing page.
