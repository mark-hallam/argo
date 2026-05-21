# web-workflow-runs-table-pagination-filters

**Authored against**: `origin/main` `32631c0ae` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-runs-table-pagination-filters`.

## Summary

`web-workflow-per-workflow-runs-table` was CLAIMED in tick-20 by a worker. This brief is the natural follow-up: once the table is wired to real data, add pagination + status filters (success / failed / running / queued) + date-range. Workspace-scale tenants will have thousands of runs; an unpaginated table doesn't scale.

Pre-condition: **`web-workflow-per-workflow-runs-table` MERGED.**

## Acceptance

- [ ] Pagination controls: page size 25/50/100 (dropdown), prev/next buttons, "Showing 1-25 of 1247" indicator.
- [ ] Status filter chips: All / Success / Failed / Running / Queued. Multi-select OK; chip toggle.
- [ ] Date-range picker: last 24h / last 7d / last 30d / custom (date inputs). Default to last 7d.
- [ ] Gateway endpoint accepts `?page`, `?page_size`, `?status[]`, `?since_ts`, `?until_ts` query params. Existing endpoint from the pre-condition extended (additive).
- [ ] URL state: filters live in query params so a filtered view is shareable / bookmarkable.
- [ ] Loading skeleton matches the table row shape during fetch.
- [ ] Empty state: "No runs match your filters" + "Clear filters" link.
- [ ] At least one integration test covering the filter combinations.

## May touch

- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (or wherever the runs table lives post-precondition)
- `web/components/skycot/workflows/RunsTable.tsx` (or owned component)
- `web/lib/api/workflow-runs.ts` (extend)
- `crates/argo-gateway/src/routes_workflow_runs.rs` (or equivalent — additive query params)
- Test files

## Must NOT touch

- The runs-table base implementation (pre-condition owns).
- The locked workflows-page design.
- Other workflow surfaces (`/workflows/<id>/build` is owned elsewhere).

## Notes

- Pre-flight: check pre-condition PR has merged + the endpoint shape it landed.
- Pagination pattern reference: `/audit` page already has paginated rows — read that component for the canonical shadcn-Table + page-size pattern.
- Sizing: 300–500 LOC.
