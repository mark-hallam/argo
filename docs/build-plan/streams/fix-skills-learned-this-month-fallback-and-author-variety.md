# fix-skills-learned-this-month-fallback-and-author-variety

**Owner**: claude.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skills-learned-this-month-fallback-and-author-variety`.
**Source**: e2e-ux-audit (scene-4-and-5-claim-4-and-5) findings F7 + F9 — `docs/skycot/ux-audits/2026-05-26-0820/scene-4-and-5-claim-4-and-5.md`.

🟡 **P1/P2 — Two skills-marketplace fixture gaps that undermine Claim 4's learning-loop narrative: "Skills my workers learned this month" filter chip shows empty grid; every seed skill shows "Authored by SKYCOT" instead of a mix of org / SKYCOT / learned-by-worker provenance.**

## Root cause

**F7 — learned-this-month filter empties the grid** (P1):
The chip exists at `web/app/(app)/skills/page.tsx:133-143` and toggles a filter against `learnedQuery.data?.learned.skill_ids`. The hook at `web/lib/api/skills.ts:232-252` falls back to an empty array when the gateway is unreachable. So in mock-offline (which is the actual dev state per the F4 env-var issue), pressing the chip filters against 0 ids → grid empties to "No skills installed yet" or "No skills match…".

Scene 4 narrates the learning loop ("the new ones learned by one teammate become available to the rest") — pressing the filter chip shows nothing, undermining the talk track at the precise beat where the camera dwells on the filter result.

**F9 — every seed skill says "Authored by SKYCOT"** (P2):
`web/lib/api/skills.ts:187-197` (`seedUsage`) returns `author_label: "Authored by SKYCOT"` for every skill when the gateway is unreachable. The skill-detail page on lines 66-67 prefers `usage?.author_label` and only falls back to "Authored by your org" when there's no usage data.

With mock-offline, every skill detail page shows "Authored by SKYCOT" — fine for marketplace skills but inconsistent with the learning loop narrative that some skills are first-party (org), some are marketplace (SKYCOT), some are learned (by a specific worker).

Both are pure fixture-data improvements; no API change needed.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F7 [P1] — "Skills my workers learned this month" filter empties the grid in mock-offline
- scene-4-and-5-claim-4-and-5 F9 [P2] — All seed skills show "Authored by SKYCOT"; no provenance variety

## Fix recipe

**For F7** (learned-this-month fallback):
1. Touch `web/lib/api/skills.ts:232-252` (the `useLearnedSkillIds` hook or equivalent).
2. When `source === "mock-offline"` (or any non-gateway data path), return a fixed sample of skill IDs marked as "learned this month":
   ```ts
   const MOCK_LEARNED_SKILL_IDS = ["support.triage", "lead.enrich"];
   ```
3. Flip the corresponding skills in `SEED_SKILLS` (`web/lib/api/skills.ts:55-116`) to have `provenance_source: "learned"` (or whatever the existing field is named).
4. Add a `learned_by` field on the seed skill (e.g. `learned_by: { worker_id: "...", worker_name: "Mariah", learned_at: "2026-05-23T..." }`).

**For F9** (author label variety):
1. Touch `seedUsage` at `web/lib/api/skills.ts:187-197`.
2. Vary `author_label` across the 6 seed skills (+ the 3 new competitor-pricing seeds from `fix-skills-marketplace-search-seed-competitor-pricing`):
   - `approval.route` → "Authored by SKYCOT"
   - `invoice.extract` → "Authored by SKYCOT"
   - `candidate.screen` → "Authored by SKYCOT"
   - `deal.qualify` → "Authored by your org"
   - `support.triage` → "Learned by Mariah"
   - `lead.enrich` → "Learned by Priya"
   - `competitor.pricing.scrape` → "Authored by SKYCOT"
   - `competitor.pricing.compare` → "Authored by your org"
   - `competitor.pricing.alert` → "Learned by Peta"
3. The provenance chip on the skill-detail page should render the matching variant — verify `web/app/(app)/skills/[skillId]/page.tsx` reads `author_label` correctly.

**Combined acceptance**:
1. Visiting `/skills` and clicking the "learned this month" chip → grid shows 2-3 skills (the ones flagged as learned).
2. Visiting `/skills/support.triage` → chip reads "Learned by Mariah · 2026-05-23".
3. Visiting `/skills/approval.route` → chip reads "Authored by SKYCOT".
4. Visual variety across the marketplace makes the provenance range observable.

## Acceptance criteria

- [ ] `useLearnedSkillIds` returns a non-empty array in mock-offline mode (at least 2 IDs).
- [ ] Pressing the "Skills my workers learned this month" chip on `/skills` shows the matching skills (not empty grid).
- [ ] `seedUsage` returns at least 3 distinct `author_label` values across the seed catalog (Authored by SKYCOT / Authored by your org / Learned by <worker>).
- [ ] At least one "Learned by <worker>" skill includes a worker name from the canon roster (Mariah / Peta / Priya / Jamal / Nori).
- [ ] No skill detail page shows "Authored by SKYCOT" for a skill flagged as `learned` provenance.
- [ ] Empty-state copy unchanged when ALL skills are filtered out (don't break the legit empty state).
- [ ] Component test or snapshot covering the variety.

## May-touch

- `web/lib/api/skills.ts` (SEED_SKILLS, seedUsage, useLearnedSkillIds)
- `web/app/(app)/skills/[skillId]/page.tsx` (verify author_label rendering)
- `web/app/(app)/skills/page.tsx` (filter chip — verify no change needed)

## Out of scope

- Real gateway `/api/v1/skills/learned-this-month` endpoint (covered by `fix-gateway-workflows-and-skills-marketplace-endpoints-404` if it ships that endpoint)
- Skill install flow (separate brief: `fix-skills-detail-install-button-and-flow` already exists)
- Competitor pricing seed skills (separate brief: `fix-skills-marketplace-search-seed-competitor-pricing` already exists)

## Effort

~40-80 LOC across seed fixtures + 1 hook. ~1-2 hours. Pure fixture change, no API or schema work.
