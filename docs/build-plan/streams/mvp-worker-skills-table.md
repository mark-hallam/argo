# mvp-worker-skills-table — worker_skill association table + lookup API

**Owner**: claude. **Depends on**: nothing. **Tier M prereq — unblocks `mission-routing-skillmatch-rolematch`.**

**Authored against**: origin/main 611c2854 on 2026-05-10.

## Why

`mission-routing-skillmatch-rolematch` (M4) is BLOCKED because there
is no worker↔skill association anywhere — no table, no method. The
routing engine's `TargetSelector::SkillMatch(String)` resolution
needs a way to ask "which workers in tenant T have skill S?" to pick
a target.

This brief adds the table + the lookup. **No routing-engine integration
in this brief** — that lands in M4 once this prereq merges.

## Approach (concrete)

1. **Migration v48 (or next free after v47 if `mvp-worker-trust-grade-column`
   lands first; check at claim time and renumber if needed).** Raw +
   versioned:
   ```sql
   CREATE TABLE worker_skill (
     worker_id BLOB NOT NULL REFERENCES worker(id) ON DELETE CASCADE,
     skill_id  TEXT NOT NULL,
     proficiency INTEGER NOT NULL DEFAULT 1,  -- 1=novice, 5=expert
     created_at TEXT NOT NULL DEFAULT (datetime('now')),
     PRIMARY KEY (worker_id, skill_id)
   );
   CREATE INDEX worker_skill_skill_idx ON worker_skill (skill_id);
   ```
2. **Versioned migration** following v47 pattern.
3. **Schema-tip assertions** bumped under EXHAUSTIVE-MATCH.
4. **Identity API:**
   - `IdentityStore::add_worker_skill(worker_id, skill_id, proficiency)`
   - `IdentityStore::remove_worker_skill(worker_id, skill_id)`
   - `IdentityStore::workers_with_skill(tenant_id, skill_id) ->
      Vec<Worker>` — JOIN against `worker` to scope by tenant
   - `IdentityStore::skills_for_worker(worker_id) -> Vec<(skill_id, proficiency)>`
5. **6 unit tests** in `argo-identity`:
   - `add_worker_skill_persists`
   - `add_worker_skill_is_idempotent` (PK conflict → upsert proficiency)
   - `remove_worker_skill_succeeds`
   - `workers_with_skill_filters_by_tenant`
   - `workers_with_skill_returns_empty_when_no_match`
   - `skills_for_worker_returns_all_associations`

## Files you may touch

- `crates/argo-store/migrations/4N__worker_skill.sql` (NEW; pick next
  free slot at claim time — currently 47 free, will be 48 if trust-grade
  prereq lands first)
- `crates/argo-store/src/migrations/v4N.rs` (NEW, ≤ 80 LOC)
- `crates/argo-store/src/migrations/mod.rs` (1 mod decl, 1 match arm)
- `crates/argo-store/src/lib_tests.rs` + `migration_reconciliation_tests.rs`
  (schema-tip bump)
- `crates/argo-identity/src/store_sql.rs` (extend IDENTITY_SCHEMA with
  the new table)
- `crates/argo-identity/src/store.rs` (4 new methods)
- `crates/argo-identity/src/lib.rs` (re-exports if needed)

## Files you must NOT touch

- `crates/argo-orchestrator/` — routing engine integration is M4, not
  this prereq
- `crates/argo-skill/` — skill schema unchanged; this is a join-table
  on the worker side
- Any vertical manifest

## Acceptance criteria

- [ ] Migration applies cleanly on fresh DB
- [ ] All 4 new methods work + 6 unit tests pass
- [ ] `cargo clippy -p argo-store -p argo-identity --all-targets -- -D warnings` clean
- [ ] No file crosses 500 LOC

## Branch + PR

- Branch: `claude-mvp-worker-skills-table`
- PR title: `identity: worker_skill association table + workers_with_skill lookup (mvp-worker-skills-table)`
