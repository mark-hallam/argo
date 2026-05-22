# web-cohort-skill-removal

**Owner**: claude.
**Depends on**: `mission-cohort-detail-real-data-wire` MERGED (✅ via #2020). PAIRS with the in-flight `mission-cohort-detail-bulk-assign-skill` (TODO) — preferable but not required to land in either order.
**Authored against**: origin/main `cbd19bbb3` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-cohort-skill-removal`.

🟢 **P3 COHORT MGMT POLISH.** Source: observed gap in #2020 + the in-flight bulk-assign-skill brief. The cohort drawer can ADD skills to workers (per-row from worker detail + bulk via the sibling brief), but there's NO way to REMOVE a skill once assigned. Operators end up with stale skills on workers and have to drop into the CLI or DB.

## Scope

In the cohort detail drawer's members table (from #2020), add a small "remove skill" affordance on each member's skill chip / pill. Click the × on a chip → confirm modal → `DELETE /api/v1/workers/:id/skills/:skill_id` (or whatever the existing single-worker skill-removal endpoint is named — if none exists, add it).

Cross-tenant invariant test is load-bearing per conductor pattern: a worker in tenant A's skill cannot be removed by a caller in tenant B.

Pair this with the bulk-remove modal if the sibling brief `mission-cohort-detail-bulk-assign-skill` has already landed (a "bulk-remove" mirror of "bulk-assign"). If bulk-assign hasn't landed, ship the per-chip × in this brief and the bulk variant lands in a sibling follow-up.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_workers_skills.rs` (or similar — find with `rg 'fn.*skill' crates/argo-cli/src/handlers/gateway/`) — add the DELETE handler if missing.
- `crates/argo-cli/src/handlers/gateway/routes_workers_skills_tests.rs` — DELETE happy-path test + cross-tenant negative test.
- `crates/argo-store/src/skill_assignment.rs` (or equivalent — verify path) — narrowly scoped: add `remove_skill_from_worker(tenant_id, worker_id, skill_id)` if not present. Returns count of rows removed.
- `web/components/cohorts/CohortDetailDrawer.tsx` — narrowly scoped: add × button on each skill chip + confirm modal trigger.
- `web/components/cohorts/RemoveSkillConfirmModal.tsx` (NEW) — small confirm dialog.
- `web/lib/api/workers.ts` — narrowly scoped: add `removeSkillFromWorker(workerId, skillId)` client wrapper.
- `web/__tests__/cohorts/CohortDetailDrawer-skill-removal.test.tsx` (NEW) — UI test.

## Files you must NOT touch

- Any other cohort surface (list page, creation flow).
- `IdentityStore` worker-mutation paths beyond skill_assignment.
- The skill registry / skill schema.
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] × button visible on each skill chip in the cohort member list (default state — not gated behind a toggle).
- [ ] Click × opens "Remove skill X from worker Y?" modal with cancel + confirm.
- [ ] Confirm fires `DELETE /api/v1/workers/:id/skills/:skill_id`.
- [ ] On success: chip disappears with a small fade-out; one `skill_removed` audit row written; activity feed refreshes within 3s.
- [ ] On 404 (already removed): chip disappears anyway (idempotent UX); no error toast.
- [ ] On 403 (cross-tenant): clear error toast "You don't have permission to modify that worker."
- [ ] Cross-tenant negative test: caller in tenant A cannot remove a skill from a worker in tenant B. Returns 403, no rows touched.
- [ ] `cargo clippy -p argo-cli -p argo-store --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli routes_workers_skills` passes (incl. cross-tenant test).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm test --run` passes new test file.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-web-cohort-skill-removal`
- PR title: `cohort: × button on skill chips for inline removal (web-cohort-skill-removal)`

## Notes for the implementer

- ~250-350 LOC expected. Backend ~120, UI ~150, tests ~80.
- The cross-tenant negative test is LOAD-BEARING per conductor pattern post-#1983. Don't ship without it.
- If `mission-cohort-detail-bulk-assign-skill` has already landed when you claim, mirror its layout: the bulk-assign button gets a sibling bulk-remove button. Otherwise just the per-chip × is enough; the bulk variant is a follow-up.
- Use the existing skill-chip component if one already exists; don't fork it for the × variant.
- Audit row format: `skill_removed` audit kind, payload `{ worker_id, skill_id, removed_by, removed_at }`. If `skill_removed` doesn't exist as an `AuditKind` variant, add it (and bump variant count in the test, per the `event.rs`-split brief's pattern).
- Idempotency: if the skill is already removed (race with another tab / actor), the endpoint returns 200 with `removed: 0` — UI treats this the same as `removed: 1` for purposes of chip disappearance.
