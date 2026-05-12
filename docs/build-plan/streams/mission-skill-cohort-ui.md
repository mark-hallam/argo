# Mission load-bearing wire — Skill-cohort UI

**Owner**: claude.
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 6 #198. Skills propagate across workers within a cohort (same vertical, same trust-grade). Operators want to see which skills are active across their fleet at a glance, with cohort-membership visualized. Today: read individual workers' skill lists one at a time.

## Scope

NEW `/cohorts` route + fleet-wide skill view:

1. **Backend** `GET /api/v1/cohorts` returns `[{ cohort_id, vertical, trust_grade_band, member_count, skill_count }]` aggregating from `worker` + `worker_skill` tables.
2. **Backend** `GET /api/v1/cohorts/<id>` returns `{ ...summary, members: [worker_summary], skills: [{ skill_name, version, member_coverage_pct }] }`.
3. **Frontend** `/cohorts` page with a card grid (one card per cohort) + detail drawer showing member workers + skill coverage matrix.
4. **Skill-coverage matrix** — a table showing rows = skills, cols = members, cell = ✓/✗/version-mismatch. Highlights workers missing a cohort-common skill.

This is NOT in scope: cohort-modification (operator UI for moving workers between cohorts — separate brief); cross-vertical cohort merge; skill-promotion-across-cohort (different feature: cross-tenant brief).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_cohorts.rs` (NEW, ≤ 380 LOC) + tests
- `crates/argo-store/src/cohort.rs` (NEW, ≤ 280 LOC) — aggregation helpers
- `web/app/(app)/cohorts/page.tsx` (NEW, ≤ 380 LOC)
- `web/components/skycot/CohortCard.tsx` (NEW, ≤ 180 LOC)
- `web/components/skycot/CohortDetailDrawer.tsx` (NEW, ≤ 320 LOC) — coverage matrix
- `web/lib/api/cohorts.ts` (NEW, ≤ 180 LOC)
- `web/lib/hooks/use-cohorts.ts` (NEW, ≤ 180 LOC)
- `web/lib/sample-cohorts.ts` (NEW, ≤ 200 LOC) — mock dataset for dev/sample mode
- AppShell nav-items list (1 line) — add /cohorts entry
- `docs/skycot/RUNBOOK.md` — small note

## Files you must NOT touch

- Worker individual skill-tab (E4-e already MERGED); the skill-propagation backend (this is read-only fleet view)

## Acceptance criteria

- [ ] Cohorts route enumerates worker groupings by (vertical, trust_band)
- [ ] Detail drawer shows member workers + skill-coverage matrix
- [ ] Skill-coverage cell distinguishes ✓ / ✗ / version-mismatch
- [ ] 6 backend tests + 8 frontend tests; clippy + pnpm test clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-skill-cohort-ui`
- PR title: `web+gateway: skill-cohort fleet view (mission-skill-cohort-ui)`

## Notes

- "Cohort" is operator-meaningful: group of workers with shared vertical + similar trust grade. Definition lives backend-side; frontend just renders.
- The coverage matrix is the load-bearing operator-insight surface — shows at a glance where the fleet's skill graph has holes.
- This pairs with the skill-propagation infrastructure (already shipped) — viewing is the consumer of the propagation system.
