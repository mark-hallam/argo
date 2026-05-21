# mission-cohort-detail-real-data-wire

**Authored against**: `origin/main` `32631c0ae` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cohort-detail-real-data-wire`.

## Summary

`/cohorts` ships the locked D3 magazine-tile layout (#1940). Clicking a tile opens a `CohortDetailDrawer` populated with **seed data only**. Wire the drawer to the gateway: list cohort members, current skills, recent activity rows, learning-loop chips. Mission claim 3 ("Platform builds workers") needs a credible cohort view to support the procurement narrative.

## Acceptance

- [ ] `CohortDetailDrawer` fetches real cohort data from the gateway when opened. Loading skeleton during fetch.
- [ ] Members section lists actual workers in the cohort (sourced from `worker.cohort_id` field — verify it exists; if not, additive migration via [docs/skycot/operator/migrations.md](docs/skycot/operator/migrations.md)).
- [ ] Skills section lists skills shared across the cohort (workers' skill intersection via `worker_skill` table).
- [ ] Recent activity: 3-5 most recent audit rows scoped to this cohort's workers.
- [ ] Empty-cohort state: drawer renders gracefully with "No members yet" + invite CTA.
- [ ] No regression on the locked D3 register (per `project_cohorts_page_design_locked.md`).
- [ ] At least one integration test on the new endpoint.

## May touch

- `web/components/skycot/cohorts/CohortDetailDrawer.tsx`
- `web/lib/api/cohorts.ts` (gateway client)
- `crates/argo-gateway/src/routes_cohorts.rs` (or equivalent — additive endpoint)
- `crates/argo-store/migrations/` (only if `worker.cohort_id` doesn't exist — verify first)
- New integration test under the gateway crate's `tests/`

## Must NOT touch

- `web/components/skycot/cohorts/Cohort{Lead,Stacked,Compact}Tile.tsx` (locked layout)
- `CohortMagazineGrid.tsx` (layout settled)
- The cohort registry / dept taxonomy (operator memory `project_cohorts_page_design_locked.md`)

## Notes

- Pre-flight: `rg 'cohort_id' crates/` to find canonical scoping.
- Sizing: 400–700 LOC. Backend additive endpoint + UI hydration + test.
- Pairs with `mission-skill-discovery-marketplace-surface` (similar substrate-vs-surface gap).
