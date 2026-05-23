# mission-worker-budget-write-tenant-pinned

**Owner**: claude.
**Depends on**: nothing.
**Authored against**: origin/main `3df52142a` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-worker-budget-write-tenant-pinned`.

🟡 **P2 LATENT-LEAK** (defense-in-depth). `worker_budget_write.rs`
runs three `WHERE id = ?` queries on the `worker` table (read +
UPDATE auto_approval_json + read again) without binding `tenant_id`,
even though `worker` has a `tenant_id` column. `worker.id` is
UUID-unique so cross-tenant collisions are structurally impossible,
but the schema gates a SQL-level enforcement that the storage layer
should provide.

## Why

PR #2116 §4 finding 5 named this brief. Re-verified at
`3df52142a` in `eval-argo-store-tenant-bind-sweep-remaining-tables`
§3.2 row 5.

Also affected (same shape, same priority):
- `worker_presence.rs:71` —
  `UPDATE worker SET last_active_at = strftime('%s','now') WHERE id = ?`.

## Scope

- Add `tenant_id: TenantId` parameter to:
  - `worker_budget_write::set_auto_approval`
  - `worker_budget_write::current_spend_for_worker` (and any sibling
    SELECTs on `worker` keyed on id alone)
  - `worker_presence::update_last_active` (called from
    `argo-audit::sink_chain` + worker_fleet handlers)
- WHERE clauses bind `tenant_id`.
- Thread tenant through callers:
  - `crates/argo-cli/src/handlers/worker_budget.rs`
  - `crates/argo-cli/src/handlers/doctor_workers.rs`
  - `crates/argo-cli/src/handlers/worker_fleet.rs`
  - `crates/argo-audit/src/sink_chain.rs`
- Cross-tenant negative test (worker_id collision is structurally
  impossible so the test pins the SQL behavior under a synthetic
  UUID-clash scenario).
- ~200-300 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/worker_budget_write.rs`
- `crates/argo-store/src/worker_presence.rs`
- `crates/argo-store/tests/` (new pin test)
- `crates/argo-cli/src/handlers/{worker_budget,doctor_workers,worker_fleet}.rs`
- `crates/argo-audit/src/sink_chain.rs`
- `docs/build-plan/BUILD_STATUS.md`

## Acceptance criteria

- [ ] Both helpers reject cross-tenant calls at SQL.
- [ ] Pin test passes.
- [ ] `cargo clippy -p argo-store -p argo-cli -p argo-audit --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
