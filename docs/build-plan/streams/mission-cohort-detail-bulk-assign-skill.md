# mission-cohort-detail-bulk-assign-skill

**Owner**: claude.
**Depends on**: `mission-cohort-detail-real-data-wire` MERGED (✅ via #2020).
**Authored against**: origin/main `b1f23a73e` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cohort-detail-bulk-assign-skill`.

🟡 **P2 COHORT MGMT.** Source: follow-up to #2020 (cohort detail drawer wire). #2020 shipped the read-side: members + activity + invite CTA. The operator can SEE the cohort, but can't BULK-assign a skill to all members. Today the only path is to open each worker individually + assign the skill — N clicks for a cohort of N workers. Add bulk-select + bulk-assign-skill.

## Scope

In the cohort detail drawer (`CohortDetailDrawer.tsx` from #2020), add:

1. **Bulk-select**: a leading checkbox column on the members table. Header checkbox toggles all visible members. Row checkboxes toggle individual members.
2. **Bulk-assign-skill button**: appears when ≥1 row is selected. Opens a small modal: skill picker (search dropdown of skills available to this tenant) + confirm button.
3. **Backend endpoint**: extend the gateway with `POST /api/v1/cohorts/:id/bulk-assign-skill` accepting `{ worker_ids: [string], skill_id: string }`. Returns `{ assigned: N, already_had: M, errors: [...] }`.
4. **Optimistic UI**: after click, members shows a small "✓ skill X added" badge for ~3s on each affected row. Then the activity feed refreshes (one of the `skill_assigned` events appears in the feed for each member).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_cohorts.rs` — add new POST endpoint.
- `crates/argo-cli/src/handlers/gateway/routes_cohorts_tests.rs` — integration test for the endpoint (200 happy path + 403 tenant-isolation + 404 cohort-not-found).
- `crates/argo-store/src/cohort_assignments.rs` (or equivalent — find via `rg 'fn assign_skill' crates/argo-store/src/`) — narrowly scoped: extend the assignment writer with a `bulk_assign_skill_to_workers(cohort_id, worker_ids, skill_id)` method if one doesn't exist. Tenant-scoped at the row level.
- `web/components/cohorts/CohortDetailDrawer.tsx` — extend with checkbox column + bulk action button.
- `web/components/cohorts/BulkAssignSkillModal.tsx` (NEW) — the modal.
- `web/lib/api/cohorts.ts` — narrowly scoped: add `bulkAssignSkill(cohortId, workerIds, skillId)` client wrapper.
- `web/__tests__/cohorts/CohortDetailDrawer-bulk.test.tsx` (NEW) — UI test for bulk-select + bulk-assign flow.

## Files you must NOT touch

- Any other cohort surface (cohort list page, cohort creation flow).
- The skill registry / skill schema.
- The `IdentityStore` worker mutations beyond the bulk-assign-skill writer.
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] Checkbox column appears in the members table.
- [ ] Header checkbox toggles all visible members.
- [ ] Bulk-assign-skill button appears when ≥1 row is selected, hidden when none.
- [ ] Button opens modal with skill-picker (search filter works on partial name).
- [ ] Confirm fires `POST /api/v1/cohorts/:id/bulk-assign-skill` with worker_ids + skill_id.
- [ ] Endpoint validates tenant scope: cohort must belong to the caller's tenant; worker_ids must all belong to that cohort.
- [ ] Endpoint writes one `skill_assigned` audit row per worker.
- [ ] Response shape `{ assigned: N, already_had: M, errors: [] }`.
- [ ] Optimistic UI shows "✓ skill X added" badge on each affected row for ~3s.
- [ ] Activity feed auto-refresh shows the new `skill_assigned` events.
- [ ] Cross-tenant negative test: caller in tenant A cannot bulk-assign-skill to a cohort in tenant B. Returns 403, no rows touched.
- [ ] `cargo clippy -p argo-cli -p argo-store --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli routes_cohorts` passes (incl. new bulk-assign test).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (final sanity).
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm test --run` passes the new test file.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-cohort-detail-bulk-assign-skill`
- PR title: `cohort: bulk-select + bulk-assign-skill on detail drawer (mission-cohort-detail-bulk-assign-skill)`

## Notes for the implementer

- ~300-450 LOC expected. Gateway + store (~150 LOC), UI (~200 LOC), tests (~100 LOC).
- The cross-tenant negative test is LOAD-BEARING. Last week's #1983 audit-detail PR shipped a real cross-tenant data-exposure bug; the operator-level expectation is that EVERY new endpoint includes a fresh cross-tenant negative test, not just the happy-path one.
- If `assign_skill_to_worker` already exists as a single-worker writer, the bulk variant should be a thin loop around it inside a single SQL transaction, NOT a parallel implementation. Find the existing writer first; don't fork the logic.
- "✓ skill X added" badge — use the existing toast / inline-badge component family. Don't invent a new one.
- Idempotency: if a worker already has the skill, the writer skips (count returned in `already_had`). Do not error on idempotent re-application.
- Modal closes after confirm + activity feed refreshes. The optimistic badge is on the row, NOT inside the modal.
