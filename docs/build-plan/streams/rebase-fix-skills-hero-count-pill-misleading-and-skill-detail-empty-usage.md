# rebase-fix-skills-hero-count-pill-misleading-and-skill-detail-empty-usage

Authored against: origin/main `0c7cff50f5` on 2026-05-29
Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-skills-hero-count-pill-misleading-and-skill-detail-empty-usage

## Why this brief exists

PR #2373 (279 LOC, 2026-05-26) targeted the bug: `/skills` hero pill reads "6 skills · 6 shown" because the SPA falls through to a 6-entry `SEED_SKILLS` array when the gateway returns 0. Approach: expand seed to 24, populate `seedUsage()` with 2–4 deterministic workers per skill, add density vitest pin.

Since then:
- **#2407 partially addressed the underlying gateway issue** — `/api/v1/skills` is now seeded by `argo-demo-seed::marketplace` (7 entries written to the demo tenant on boot). The "perpetual 0" branch is mostly gone — but the SPA fallback still triggers if the gateway is unreachable or the per-tenant seed didn't fire.
- **#2462 + #2350 + #2300** evolved `web/lib/api/skills.ts` — leading the SEED seed description with the runbook phrase + seed learned provenance fallbacks + `/api/v1/skills` prefix.
- Main now has **7 entries in `SEED_SKILLS`** (verified line 63-176 of `web/lib/api/skills.ts`), not the 24 PR #2373 wanted.

The 24-entry density expansion + `seedUsage()` population is STILL load-bearing per the F4 [P1→P2] + F6 [P2] audit findings — when the gateway is offline (mock-offline path) the hero still reads "7 skills · 7 shown" which is sparse for a marketplace surface.

PR #2373 was closed for re-author.

## Source of truth

- Original brief: `docs/build-plan/streams/fix-skills-hero-count-pill-misleading-and-skill-detail-empty-usage.md`
- Closed PR: #2373
- Current main shape:
  - `web/lib/api/skills.ts` lines 63-176 (`SEED_SKILLS` const, 7 entries), line 226 (`seedSkill = SEED_SKILLS.find(...)`), line 246 (`adoption_count: learnedBy ? 1 : 0`).

## Acceptance

Same as original brief: ≥20 `SEED_SKILLS` entries spanning finance / sales / support / project ops / marketing / HR / ops; `seedUsage()` returns 2–4 deterministic workers per skill using djb2 hash + the shared roster (Mariah / Priya / Jamal / Nori / Peta / Patrick / Devon / Alex); plausible `adoption_count` 40–300 deterministic range.

Re-implement against current `origin/main` HEAD. Preserve the runbook-phrase lead from #2462 in any seed descriptions and the `/api/v1/skills` prefix from #2300.

## Boundary

- May touch: `web/lib/api/skills.ts`, `web/tests/skills-page-density.test.ts` (new).
- Coordinate with #2632 (count-pill copy reverse when filter active) — do not regress.

## Verification

- `pnpm test lib/api/skills.test.ts tests/skills-page-density.test.ts`
- `pnpm typecheck`
- `pnpm lint`
