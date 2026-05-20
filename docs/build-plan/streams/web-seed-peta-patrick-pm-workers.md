# web-seed-peta-patrick-pm-workers — add Peta + Patrick PM-worker seed data

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-seed-peta-patrick-pm-workers`.
**Priority**: P2 — wave-4 data-integrity: "Peta and Patrick PM workers have briefs + mockups but no seed." Cohorts page references the Project & Ops department but the workers don't exist in /team data.
**Sizing**: ~150-250 LOC (seed entries + audit history + cohort membership).

## Why this brief exists

From the wave-4 data-integrity audit + operator memory
`project_pm_cohorts_peta_patrick.md`:

> Peta (Project Coordinator L2 · Linear+GitHub) + Patrick (Status
> Reporter L3). Naming convention: all PM workers are P-names.
> Briefs at M21/M22, worker-detail mockups built, cohorts page
> updated with Project & Ops as 7th dept tab.

But the seed data never landed. Visit /team → no Peta, no Patrick.
The cohorts tab for Project & Ops shows empty.

## Goals

1. Add Peta (Project Coordinator L2) to sample-team.ts with full persona (traits, headline, KPIs).
2. Add Patrick (Status Reporter L3) similarly.
3. Add audit-row history for both (similar volume to existing workers).
4. Add to Project & Ops cohort.
5. Add worker-detail data (memory + skills + activity) so /workers/wkr_01J0PETA/overview renders.

## Steps

1. Read `project_pm_cohorts_peta_patrick.md` (operator memory) for the canonical persona details.
2. Add 2 entries to `web/lib/sample-team.ts`.
3. Add audit-row history to `web/lib/sample-audit.ts` (~10 actions per worker).
4. Add to `web/lib/sample-cohorts.ts` under Project & Ops department.
5. Add overview/memory/skills sample data files.
6. Verify: /team has 8 workers now (was 6), Project & Ops cohort tile populated, /workers/wkr_01J0PETA/overview renders.

## may-touch

- `web/lib/sample-team.ts`
- `web/lib/sample-audit.ts`
- `web/lib/sample-cohorts.ts`
- `web/lib/sample-worker-overview.ts`
- `web/lib/sample-worker-memory.ts`
- `web/lib/sample-worker-skills.ts`
- `web/lib/sample-worker-activity.ts`

## must-NOT-touch

- Existing 6 workers' seed (Mariah, Priya, Jamal, Sam, Nori, Rio)
- /team page rendering code

## Acceptance criteria

- [ ] /team renders 8 workers (6 original + Peta + Patrick).
- [ ] /workers/wkr_01J0PETA/overview renders 200 with full persona.
- [ ] /workers/wkr_01J0PATRICK/overview renders 200.
- [ ] Project & Ops cohort tile shows 2 members.
- [ ] /audit shows actions from both new workers.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/data-integrity/2026-05-20-wave-4-data-integrity.md`
- Memory: `~/.claude/.../project_pm_cohorts_peta_patrick.md`
