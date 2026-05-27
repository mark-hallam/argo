# fix-skill-detail-learning-loop-telemetry-claim-4-evidence

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-skill-detail-learning-loop-telemetry-claim-4-evidence`
**Source:** UX e2e audit 2026-05-28-0252 (scene-4-and-5 F5)

🟠 **P1 — `/skills/<id>` detail page renders only a static `Trust · A` chip with no version timeline, no trust-grade history, no adoption trend, no "Learned via N successful runs" telemetry. Claim 4 ("Skills shared AND LEARNED") has no learning-loop surface on the canonical detail page — it's a brochure, not evidence.**

## Root cause

`web/app/(app)/skills/[skillId]/page.tsx:38-122` renders three sections: PROVENANCE (Authored by SKYCOT · Trust A), WORKERS USING THIS SKILL, and a usage count pill ("Used by 0 workers"). It does not render:

- Trust-grade history (A ← B ← C with timestamps).
- Version-history list with promotion reasons.
- Adoption-trend mini-chart (last 30 days).
- "Promoted from beta after N successful runs" telemetry line.

The only learning-loop affordance anywhere in the SPA is a `useSkillsLearnedThisMonth` count chip on the `/skills` index page (count-based filter, no detail). The detail page contains the page real-estate but none of the data.

For the v1.0 demo Scene 4 narration ("the skill marketplace ships, and your skills learn"), the camera drilling into a skill currently reads "Used by 0 workers" + "Trust A" with no evidence of HOW the skill got to A. The "learned" half of Claim 4 has no demo path.

Sibling `fix-skills-hero-count-pill-misleading-and-skill-detail-empty-usage` (IN_REVIEW #2373) reframes the empty `0 workers` count to a softer copy. This brief is the additive learning-loop content beat — distinct concern, can co-exist.

## Findings this brief closes

- scene-4-and-5 F5 [P1] — Skill detail "Trust · A" chip is render-time, not learning-loop evidence; no version history / drift / "Learned via N runs"

## Fix recipe

1. **Add a `<SkillLearningLoopCard>` component** at `web/components/skycot/skill/SkillLearningLoopCard.tsx`:
   - Header: "Learning loop · Trust grade A"
   - Subheader: "Earned A on 2026-04-12 after 23 successful runs across 4 workers"
   - Three sub-blocks:
     - **Trust-grade history**: vertical timeline (B → A) with date + sample-size + verdict-summary per promotion event.
     - **Adoption trend**: sparkline of `runs/day` over the last 30 days. Use the simplest possible inline SVG sparkline; no charting library needed.
     - **Recent runs**: bulleted list of last 3 runs with `(worker, verdict, latency)` tuple.
2. **Data source decision** — the gateway endpoints for skill telemetry are unconfirmed. Two paths:
   - **Path A (preferred)**: try `GET /api/v1/skills/<id>/telemetry` and `…/history`. If 404, fall back to Path B.
   - **Path B (fixture)**: pull from a NEW `web/lib/sample-skill-telemetry.ts` keyed by skill id. Per `feedback_no_theater_in_demo` the dressing side can be fixtures IF the camera doesn't drill into receipts. For the v1.0 demo this is acceptable; mark the file with a top-level TODO comment pointing at a follow-up brief.
3. **Slot the card on the detail page** between PROVENANCE and WORKERS USING — so the camera sees it without scrolling.
4. **Loading + empty states**: if the skill is brand-new (0 runs) the card shows "No runs yet — this skill ships at Trust B and earns A through real workers."
5. **Vitest + Storybook**:
   - `SkillLearningLoopCard.test.tsx` — covers the 3 sub-blocks with mock data.
   - `SkillLearningLoopCard.stories.tsx` — covers "fresh skill (0 runs)", "promoted skill (B→A)", "high-adoption skill" variants.

## Acceptance criteria

- [ ] `/skills/<id>` page renders a learning-loop card visible above the fold.
- [ ] The card shows trust-grade history, adoption trend, and recent runs sections.
- [ ] Fresh skill (0 runs) renders the "earns A through real workers" empty state — no fabricated history.
- [ ] No fabricated trust-grade dates / sample sizes — fixture content is clearly seed data with a TODO comment for the real-telemetry follow-up.
- [ ] vitest + Storybook pass.
- [ ] PR body cites scene-4-and-5 F5 and notes the dressing-side fixture path.

## May-touch

- `web/app/(app)/skills/[skillId]/page.tsx` (slot the card, ~10 LOC)
- `web/components/skycot/skill/SkillLearningLoopCard.tsx` (NEW, ~180 LOC)
- `web/components/skycot/skill/SkillLearningLoopCard.test.tsx` (NEW)
- `web/components/skycot/skill/SkillLearningLoopCard.stories.tsx` (NEW)
- `web/lib/sample-skill-telemetry.ts` (NEW if Path B)
- `web/lib/api/skills.ts` (add `fetchSkillTelemetry` if Path A endpoint exists)

## Out of scope

- Gateway telemetry endpoints themselves (separate codex brief if Path A endpoints don't exist)
- The `0 workers` empty-state copy (covered by `fix-skills-hero-count-pill-misleading-and-skill-detail-empty-usage` IN_REVIEW #2373)
- Per-worker Skills tab Active/Library/Learned data (covered by `fix-persona-fixture-drift-mariah-cohort-traits-skills-cross-surface` IN_REVIEW #2556)
- Install button (covered by `fix-skills-detail-install-button-and-flow` BLOCKED)

## Effort

M — ~180 LOC new component + ~80 LOC tests + storybook + small detail-page wiring. ~5-6 hours including fixture content.
