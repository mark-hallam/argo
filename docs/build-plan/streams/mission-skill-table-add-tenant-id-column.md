# mission-skill-table-add-tenant-id-column

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-table-add-tenant-id-column`.

🔴 **P0 ARCHITECTURAL — SKILL TABLE HAS NO TENANT_ID.** Source: evaluator round-9 PR #2081 finding §2 P0 #2.3.

> "`skill` table at `migrations/v1.rs:107-119` has no `tenant_id` column at all → workspace-global skill leak across tenants (architectural — needs additive migration)."

The skill table is workspace-global. Every tenant's workers can see every other tenant's skills. Mission claim 6 (multi-tenant by construction) is **architecturally violated** at the skill primitive.

## Scope

Additive migration adding `tenant_id` to the skill table, plus a sweep of every `skill` SELECT/UPDATE/DELETE to bind it.

### Migration

NEW: `crates/argo-store/migrations/<next-slot>__skill_tenant_id.sql` + `crates/argo-store/src/migrations/v<next>.rs`.

Check `scripts/check-migration-slots.sh` for the next free slot at claim time (latest on main is 130 / v78 after #2056; this brief uses 131 / v79).

```sql
-- 131__skill_tenant_id.sql
ALTER TABLE skill ADD COLUMN tenant_id TEXT NOT NULL DEFAULT '__migration_placeholder__';
CREATE INDEX idx_skill_tenant_id ON skill(tenant_id);
```

Then a backfill: existing rows get `tenant_id = '__migration_placeholder__'` (the operator will need to manually re-tenant via a separate CLI command — out of brief scope). All NEW writes MUST provide a real tenant_id.

### SQL sweep

`rg 'sqlx::query.*skill|FROM skill|UPDATE skill|INSERT INTO skill|DELETE FROM skill' crates/`. For each query:
- SELECT: add `AND tenant_id = ?` predicate.
- INSERT: include tenant_id in the column list, bind from context.
- UPDATE / DELETE: same.

### API layer sweep

`SkillStore::available_for_worker(worker_id, ...)` (cited as P0 #4.1 in #2081) — needs tenant_id parameter, scope its query to the worker's tenant.

`SkillStore::create_*` / `SkillStore::update_*` / `SkillStore::delete_*` — same shape.

### Cross-tenant negative tests

For each `SkillStore` method:
- Bootstrap tenant_a + tenant_b each with a worker + a skill.
- Call from tenant_b's principal → assert tenant_a's skills are NOT visible.

This is the bedrock invariant. **Test must be byte-identical-or-empty between cross-tenant and nonexistent-id.**

## Parallelism

- Manager-worker allowed: no (single coordinated migration + sweep)
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [`crates/argo-store/src/skill_store.rs`, `crates/argo-skill/src/store.rs`]

## Files you may touch

- `NEW: crates/argo-store/migrations/131__skill_tenant_id.sql`
- `NEW: crates/argo-store/src/migrations/v79.rs`
- `crates/argo-store/src/migrations/registry_compliance.rs` (versions 57..=79)
- `crates/argo-store/src/lib.rs` (EXPECTED_SCHEMA_VERSION 78→79)
- `crates/argo-skill/src/store.rs` (every SQL query)
- `crates/argo-skill/src/store_*.rs` siblings
- `crates/argo-skill/src/tests/*.rs` — cross-tenant negative tests for every method
- Any caller-site updates (find via `rg 'SkillStore::' crates/`)
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- The skill content / IR / promotion logic — purely tenancy at the storage layer.
- `argo-runtime` skill resolution path — that's a follow-up brief (`mission-skill-resolution-tenant-bind`).
- Operator-facing CLI for re-tenanting placeholder rows — separate brief.

## Acceptance criteria

- [ ] Migration applies cleanly on `:memory:` AND on the existing operator DB (additive, default value).
- [ ] EXPECTED_SCHEMA_VERSION bumped + registry updated.
- [ ] Every SQL query on `skill` binds tenant_id.
- [ ] `SkillStore::available_for_worker` + every other public method takes tenant_id.
- [ ] Cross-tenant negative test for EVERY public store method.
- [ ] `cargo clippy -p argo-store -p argo-skill --all-targets -- -D warnings` clean.
- [ ] Workspace clippy clean.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-skill-table-add-tenant-id-column`
- PR title: `fix(skill): add tenant_id to skill table + sweep SQL surface (mission-skill-table-add-tenant-id-column)`

## Notes for the implementer

- ~400-700 LOC expected. Migration ~30, store sweep ~100, tests ~300, caller updates ~100.
- **This is architectural** — mission claim 6 violated at the primitive. Don't ship a partial fix.
- **Placeholder backfill value** is intentional: existing skills get a placeholder tenant so the column is NOT NULL on insert; a separate operator CLI re-tenants them. Document this in PR body.
- Cite round-9 evaluator audit + #2081 §2.3 verbatim.
- Pair brief: `mission-skill-ab-test-run-tenant-id-bind` (the `turn_skills_runtime.rs:159-165` site) lands separately — it's the runtime side of this same architectural fix.
