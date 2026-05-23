# claude-skill-runs-worker-budget-tenant-scope

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-skill-runs-worker-budget-tenant-scope`.

🔴 **P1 CROSS-TENANT SPEND LEAK.** Source: evaluator round-13 SHARED.md handoff 2026-05-23T12:30:00Z.

> "🔴 skill_runs_query.rs SELECT FROM worker_budget GROUP BY action_id — NO tenant scoping; tenant_b sees tenant_a spending totals."

The query aggregates spending across the `worker_budget` table with no `tenant_id` predicate. Any caller in tenant_b that calls this surface gets tenant_a's spend numbers mixed into the aggregate. Direct financial-data cross-tenant leak.

## Scope

1. Find the query: `crates/argo-store/src/skill_runs_query.rs` (or sibling — confirm via `rg 'FROM worker_budget' crates/argo-store/src/`).
2. Add `tenant_id` parameter to the function signature.
3. Add `AND tenant_id = ?` predicate to the SELECT.
4. Sweep callers (`rg '<fn_name>' crates/`) — thread tenant_id through. ~3-8 callsites likely.
5. **Cross-tenant negative test**: bootstrap tenant_a + tenant_b each with workers + budget rows; call from tenant_b context → assert aggregate ONLY includes tenant_b's rows, NOT tenant_a's.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-store/src/skill_runs_query.rs` — narrowly scoped.
- `crates/argo-store/src/skill_runs_query_tests.rs` (or sibling) — add cross-tenant test.
- Caller sites (find via `rg`).
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `worker_budget` table schema — already has `tenant_id` column per round-13 (substrate is correct, query is the bug).
- Other unrelated argo-store queries.

## Acceptance criteria

- [ ] Query binds `tenant_id`.
- [ ] All callsites updated.
- [ ] Cross-tenant negative test pinned.
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated.

## Branch + PR

- Branch: `claude-skill-runs-worker-budget-tenant-scope`
- PR title: `fix(store): tenant-scope skill_runs worker_budget aggregate (claude-skill-runs-worker-budget-tenant-scope)`

## Notes for the implementer

- ~80-150 LOC. Query + caller sweep + test.
- Cite round-13 verbatim in PR body.
