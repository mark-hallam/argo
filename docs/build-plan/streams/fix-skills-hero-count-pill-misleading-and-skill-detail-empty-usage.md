# fix-skills-hero-count-pill-misleading-and-skill-detail-empty-usage

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skills-hero-count-pill-misleading-and-skill-detail-empty-usage`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/scene-4-and-5-claim-4-and-5.md` F4 [P1] + F6 [P2].

🟡 **P2 — `/skills` hero count pill always reads "6 skills · 6 shown" because the gateway returns 0 skills and the SPA falls to a 6-entry seed. Marketplace feel suffers. Skill-detail "Workers using" card also empty in mock-offline.**

## Root cause

`web/lib/api/skills.ts:55` — `SEED_SKILLS` array has exactly 6 entries. Hero pill formula `{skills.length} skills · {filtered.length} shown` in `web/app/(app)/skills/page.tsx:113-117`. Skill detail's `fetchSkillUsage` (`web/lib/api/skills.ts:187-197`) returns `{workers_using: [], adoption_count: 0}` in mock-offline.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F4 [P1] (downgraded to P2 if seed-CLI lands; the seed-CLI brief covers the real gateway side) — hero pill always 6
- scene-4-and-5-claim-4-and-5 F6 [P2] — skill detail "Workers using" empty in mock-offline

## Fix recipe

1. **Expand `SEED_SKILLS`** in `web/lib/api/skills.ts:55` to ~20-30 plausible skills before demo, OR seed the live gateway with the runbook's expected set (covered by `fix-demo-seed-cli-for-real-roster`).
2. **Expand `seedUsage`** in the same file to return 2-4 mock workers per skill so the detail card looks populated.
3. **Tighten hero pill copy** to use plural-formatting more gracefully when N is small (e.g., the current pill is fine; this is mostly a seed expansion).
4. **Web test** asserts `/skills` renders >10 skill cards with seed expansion.

## Acceptance criteria

- [ ] `/skills` shows ≥20 skill cards by default.
- [ ] Skill-detail `/skills/<id>` shows a non-empty "Workers using" card.
- [ ] Hero pill reflects accurate count.
- [ ] Web test asserts both render densities.

## May-touch

- `web/lib/api/skills.ts:55` (expand SEED_SKILLS)
- `web/lib/api/skills.ts:187-197` (expand seedUsage)
- `web/app/(app)/skills/page.tsx` (no change; consumes the expanded seed)
- `web/tests/skills-page-density.test.ts` (NEW)

## Out of scope

- Live gateway skill seeding (separate brief: `fix-demo-seed-cli-for-real-roster`)
- Install button + flow (separate brief: `fix-skills-detail-install-button-and-flow`)
- Competitor-pricing scrape seed (separate brief: `fix-skills-marketplace-search-seed-competitor-pricing`)

## Effort

~80-150 LOC of seed fixtures + 1 test. ~1-2 hours.
