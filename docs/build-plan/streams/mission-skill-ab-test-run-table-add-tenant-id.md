# mission-skill-ab-test-run-table-add-tenant-id

**Owner**: unassigned.
**Depends on**: none (storage-side migration; runtime-side bind `mission-skill-ab-test-run-tenant-id-bind` is BLOCKED on this brief).
**Authored against**: origin/main `dac9c624` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-ab-test-run-table-add-tenant-id`.

🔴 **P0 ARCHITECTURAL — `skill_ab_test_run` TABLE HAS NO TENANT_ID.** Source: evaluator round-9 PR #2081 finding §2 P0 #2.1, pre-flight discovery during claim of sibling brief `mission-skill-ab-test-run-tenant-id-bind`.

The sibling brief (`mission-skill-ab-test-run-tenant-id-bind`) instructed: "If the `skill_ab_test_run` table itself lacks a tenant_id column, file a parallel brief and BLOCK." That pre-flight check confirmed the table at `crates/argo-store/src/migrations/v21_skill_ab_tests.rs:47-59` has columns `(id, skill_name, variant_a, variant_b, target_runs, started_at, completed_at, status, winner)` — no `tenant_id`. The runtime-side binding cannot proceed until the column exists.

This brief is the storage-side prerequisite: additive migration + write-path threading. The bind-side (`turn_skills_runtime.rs:159-165` SELECT) ships in the sibling brief once this lands.

## Scope

Additive migration adds `tenant_id` to `skill_ab_test_run`, plus an audit of every write/read site of that table to thread tenant_id through.

### Migration

NEW: `crates/argo-store/migrations/<next-slot>__skill_ab_test_run_tenant_id.sql` + `crates/argo-store/src/migrations/v<next>.rs`.

Check `scripts/check-migration-slots.sh` for the next free slot at claim time. The sibling `mission-skill-table-add-tenant-id-column` reserves 131/v79; if it's already landed, take 132/v80. Coordinate with the sibling brief — if both ship in the same wave, the second to land bumps its slot.

```sql
-- <slot>__skill_ab_test_run_tenant_id.sql
ALTER TABLE skill_ab_test_run ADD COLUMN tenant_id TEXT NOT NULL DEFAULT '__migration_placeholder__';
CREATE INDEX idx_skill_ab_test_run_tenant_skill ON skill_ab_test_run(tenant_id, skill_name);
DROP INDEX IF EXISTS idx_skill_ab_test_run_status;
CREATE INDEX idx_skill_ab_test_run_status ON skill_ab_test_run(tenant_id, status, started_at DESC);
```

The `(tenant_id, skill_name)` index supports the read-path bind at `turn_skills_runtime.rs:159-165`. The composite `(tenant_id, status, started_at DESC)` keeps the original `running` lookup tenant-scoped.

Backfill: existing rows get `tenant_id = '__migration_placeholder__'`. Operator re-tenants via separate CLI command (out of scope, mirrors the parent `skill` table pattern in `mission-skill-table-add-tenant-id-column`).

### Write-path sweep

`rg 'INSERT INTO skill_ab_test_run|UPDATE skill_ab_test_run|FROM skill_ab_test_run' crates/`. Known callsites at brief-author time:

- `crates/argo-skill/src/ab_test.rs:145` — `create_run` INSERT (needs tenant_id column + parameter).
- `crates/argo-skill/src/ab_test.rs:195` — `get_run` SELECT by id (id is UUIDv7, globally unique; arguably no tenant predicate needed, but defence-in-depth says add `AND tenant_id = ?` and require caller to pass it).
- `crates/argo-skill/src/ab_test_quality.rs:89` — `UPDATE skill_ab_test_run SET status='completed', ...`.
- `crates/argo-skill/src/ab_test_tests.rs:239` — test fixture UPDATE.
- `crates/argo-agent-runner/src/turn_skills_runtime.rs:159` — the SELECT the sibling brief binds.

### API surface

`create_run`, `get_run`, `promote`, `maybe_complete`, `maybe_complete_with_quality` in `argo-skill` must take `TenantId`. Adopt the additive-constructor pattern from `feedback_brief_authoring_preflight_rules.md` for high-caller-count cases (`_with_tenant` variants that wrap the originals temporarily) if a single-step API break is too invasive — but the preferred path is a single coordinated break given the small caller set.

### Cross-tenant negative tests

- Bootstrap tenant_a + tenant_b. Each creates an A/B-test run for skill `slack-dm` with different variants.
- `get_run(tenant_b_principal, run_id_from_tenant_a)` → `None` (not a leaked variant).
- `promote(tenant_b_principal, run_id_from_tenant_a)` → `Err`, not silent cross-tenant promotion.
- Sibling-bind cross-tenant negative lands with `mission-skill-ab-test-run-tenant-id-bind` — keep it focused on the runtime SELECT.

## Parallelism

- Manager-worker allowed: no (single coordinated migration + sweep)
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [`crates/argo-skill/src/ab_test.rs`, `crates/argo-skill/src/ab_test_quality.rs`]

## Files you may touch

- `NEW: crates/argo-store/migrations/<slot>__skill_ab_test_run_tenant_id.sql`
- `NEW: crates/argo-store/src/migrations/v<next>.rs`
- `crates/argo-store/src/migrations/registry_compliance.rs` (add new version to expected set)
- `crates/argo-store/src/lib.rs` (EXPECTED_SCHEMA_VERSION bump)
- `crates/argo-skill/src/ab_test.rs` (create_run / get_run / promote signatures + SQL)
- `crates/argo-skill/src/ab_test_quality.rs` (UPDATE adds tenant predicate)
- `crates/argo-skill/src/ab_test_tests.rs` (fixture updates + cross-tenant negative)
- Any caller-site updates (find via `rg 'create_run\|get_run\|promote' crates/argo-skill crates/argo-cli`)
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `crates/argo-agent-runner/src/turn_skills_runtime.rs` — that's the sibling brief's surface; this brief unblocks it but does not edit it.
- The skill IR / promotion-decision logic — purely tenancy at the storage + API layer.
- Operator-facing CLI for re-tenanting placeholder rows — separate brief (parallel to the `skill` table version).

## Acceptance criteria

- [ ] Migration applies cleanly on `:memory:` AND on existing operator DB (additive, default value).
- [ ] EXPECTED_SCHEMA_VERSION bumped + registry updated.
- [ ] Every SQL query on `skill_ab_test_run` binds tenant_id (writes + reads + updates).
- [ ] `create_run` / `get_run` / `promote` API takes `TenantId`.
- [ ] Cross-tenant negative test: tenant_b cannot read/promote tenant_a's run.
- [ ] `cargo clippy -p argo-store -p argo-skill --all-targets -- -D warnings` clean.
- [ ] Workspace clippy clean (`scripts/workspace-clippy-fast-fail.sh`).
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.
- [ ] PR body unblocks sibling `mission-skill-ab-test-run-tenant-id-bind` (call it out).

## Branch + PR

- Branch: `mission-skill-ab-test-run-table-add-tenant-id`
- PR title: `fix(skill): add tenant_id to skill_ab_test_run table + sweep API surface (mission-skill-ab-test-run-table-add-tenant-id)`

## Notes for the implementer

- ~250-450 LOC expected. Migration ~30, API sweep ~80, callsite updates ~50, tests ~150.
- **This brief unblocks the runtime-side `mission-skill-ab-test-run-tenant-id-bind`.** Once this lands, the sibling brief is a one-callsite fix (add `AND tenant_id = ?` to the SELECT at `turn_skills_runtime.rs:159-165`).
- Coordinate slot number with `mission-skill-table-add-tenant-id-column` if both are in-flight at claim time.
- Placeholder backfill (`__migration_placeholder__`) intentionally mirrors the parent `skill` table approach so the operator's re-tenant CLI covers both tables in one pass.
- Cite round-9 evaluator audit + #2081 §2.1 verbatim + this brief's BLOCKING relationship with `mission-skill-ab-test-run-tenant-id-bind`.
