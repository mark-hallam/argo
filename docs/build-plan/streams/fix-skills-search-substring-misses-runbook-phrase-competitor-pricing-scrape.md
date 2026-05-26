# fix-skills-search-substring-misses-runbook-phrase-competitor-pricing-scrape

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skills-search-substring-misses-runbook-phrase-competitor-pricing-scrape`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/scene-4-and-5-claim-4-and-5.md` F1.

🔴 **P0 — The Scene 4 demo runbook directs the operator to type the literal phrase `competitor pricing scrape` into `/skills` hero search. The page filter is case-insensitive substring across `name + description + layer`. The seeded anchor skill is `pricing.competitor_scrape` with description starting "Scrape competitor product pricing across listed channels…". None of `name + description + layer` contains that exact substring in that word order — the description has `competitor product pricing`, not `competitor pricing scrape`. Result: count pill flips to `7 skills · 0 shown` and the empty-state panel renders "No skills match 'competitor pricing scrape'". Scene 4 demo unrecordable as scripted.**

## Root cause

`web/app/(app)/skills/page.tsx:65-78` defines the filter:

```ts
const matches = (skill) => {
  const q = query.toLowerCase();
  return skill.name.toLowerCase().includes(q)
      || skill.description.toLowerCase().includes(q)
      || skill.layer.toLowerCase().includes(q);
};
```

`web/lib/api/skills.ts:64-73` defines the anchor skill:

```ts
{
  name: "pricing.competitor_scrape",
  description: "Scrape competitor product pricing across listed channels…",
  layer: "pricing",
  ...
}
```

The runbook phrase `competitor pricing scrape` doesn't substring-match any single field — the description is `competitor product pricing`, not `competitor pricing scrape`. Typing `competitor` alone returns the expected 1 card; typing the full phrase returns 0.

This is the cheapest demo-blocker on the audit. Two fixes; pick whichever lands fastest:

- **(a) Rewrite the description** to include the literal runbook phrase as the lead clause.
- **(b) Switch the filter to a word-tokenized AND-match** so any word-order variation works.

Recommend (a) for v1.0 demo — 5-minute fix, no regression risk on other queries. (b) is the proper fix; pair it as a follow-on.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F1 [P0] — Scene 4 search beat returns zero results because word order doesn't substring-match the seed

## Fix recipe

**Recommended path (a): rewrite the description to include the runbook phrase.**

1. In `web/lib/api/skills.ts:64-73`, prepend the description so it reads:
   ```
   "Competitor pricing scrape — pull competitor product pricing across listed channels and surface deltas as memory."
   ```
   This keeps the original semantic content but lands the literal phrase first. The skill's `name`, `layer`, and other fields stay unchanged.
2. Verify the description still reads naturally for the skill-detail card.
3. Vitest pin: search `competitor pricing scrape` returns the expected 1 card.

**Optional path (b): switch the filter to word-tokenized AND-match.**

4. In `web/app/(app)/skills/page.tsx:65-78`, rewrite `matches` to tokenize on whitespace and require every word to appear somewhere across `name + description + layer`. `lodash` or a tiny inline tokenizer (`q.toLowerCase().split(/\s+/).filter(Boolean).every(w => combined.includes(w))`).
5. Vitest pin: search `competitor pricing scrape`, `pricing competitor scrape`, `scrape competitor pricing` all return the same expected card.

**Recommended**: ship Path (a) first as the 5-minute demo-day fix; then ship Path (b) as a follow-on polish brief.

## Acceptance criteria

- [ ] In demo mode + post-fix, typing `competitor pricing scrape` into `/skills` hero search returns at least 1 card (the `pricing.competitor_scrape` skill).
- [ ] No regression on other queries: typing `competitor` alone still returns 1 card; typing `pricing` returns the same 1 card; typing nothing returns all skills.
- [ ] Vitest pin: assert the filter behaviour for the runbook phrase + the original single-word queries.
- [ ] If path (b) ships: typing any word-order permutation of `competitor pricing scrape` returns the expected card.

## May-touch

- `web/lib/api/skills.ts` (description text — Path a)
- `web/app/(app)/skills/page.tsx` (filter — Path b, optional)
- `web/app/(app)/skills/__tests__/page.test.tsx` (pin)
- `web/lib/api/__tests__/skills.test.ts` (pin)

## Out of scope

- The skills install button (covered by `fix-skills-detail-install-button-and-flow.md`)
- The skill icon rotation (covered by `fix-skills-marketplace-card-icon-rotation-semantic-name-map`)
- Backend skills boot-seed (covered by `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution.md`)

## Effort

XS — Path (a) is a 1-LOC change. ~10 minutes. Path (b) is ~30 LOC + pin. ~30 minutes. P0 because Scene 4's search beat is dead-as-scripted without it.
