# fix-kpi-rollup-preserve-charter-md

**Owner**: claude.
**Authored against**: origin/main `37ba3f226` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-kpi-rollup-preserve-charter-md`.
**Blocks**: `mission-kpi-rollup-from-audit-trail` (PR #2279).
**Source**: evaluator round-55 finding `docs/skycot/evals/scheduled/2026-05-25/round-55-pr-2279-kpi-rollup-charter-clobber-finding.md`.

🔴 **P0 ANTI-THEATER FINDING — production blast radius confirmed.**

PR #2279 ships a `KpiRollupScheduler` whose tick **destroys `worker.charter_md`** every 5 minutes. Root cause at `crates/argo-orchestrator/src/kpi_rollup.rs:160-163`:

```rust
identity
    .update_charter(tenant, worker, None, Some(&kpis_value))
    .await?;
```

The SQL behind `IdentityStore::update_charter` at `crates/argo-identity/src/store.rs:374`:

```sql
UPDATE worker SET charter_md = ?, kpi_json = ? WHERE id = ? AND tenant_id = ?
```

Binding `None` to `charter_md` unconditionally NULLs the column. The 5 bundled tests in `kpi_rollup_test.rs` never seed a charter so they never observe the clobber. Production callers `contract.rs:71` + `worker_charter.rs:34` both pass `Some(&charter_md)` — every hired worker goes through one of these paths.

**Demo blast radius**: Mariah card / `/workers/{id}/overview` page binds `Worker::charter_md`. After 5 minutes of gateway uptime every hired worker's charter is NULL.

## Fix (Option A — preferred per evaluator)

Add a sibling on `IdentityStore` that only writes `kpi_json`:

```rust
pub async fn set_kpis(
    &self,
    tenant: &TenantId,
    worker: &WorkerId,
    kpis: &serde_json::Value,
) -> ArgoResult<()> {
    sqlx::query("UPDATE worker SET kpi_json = ?2 WHERE id = ?1 AND tenant_id = ?3")
        .bind(worker.as_ref())
        .bind(kpis.to_string())
        .bind(tenant.as_ref())
        .execute(&self.pool).await?;
    Ok(())
}
```

Then in `kpi_rollup.rs:160-163` replace the `update_charter` call with `identity.set_kpis(tenant, worker, &kpis_value).await?;`.

Keep `update_charter` unchanged — its CLI-shaped semantics are correct for hire / charter-set; only the rollup tick was misusing it.

## Test additions

Port the evaluator's 122-LOC charter-clobber probe into `crates/argo-orchestrator/tests/kpi_rollup_test.rs` as a 6th case `rollup_does_not_clobber_existing_charter`:

- Seed worker → `IdentityStore::update_charter(tenant, worker, Some("..."), None)`.
- Run one `KpiRollupScheduler::tick`.
- Assert precondition non-vacuous + workers_rolled==1 + postcondition: charter survives.

## Acceptance criteria

- [ ] `crates/argo-identity/src/store.rs` has new `set_kpis` fn that only writes `kpi_json`.
- [ ] `crates/argo-orchestrator/src/kpi_rollup.rs:160-163` calls `identity.set_kpis(...)`.
- [ ] `crates/argo-orchestrator/tests/kpi_rollup_test.rs` has 6th test `rollup_does_not_clobber_existing_charter`.
- [ ] All 6 tests pass cold.
- [ ] Production callers (`contract.rs:71` + `worker_charter.rs:34`) unchanged.
- [ ] `cargo clippy -p argo-orchestrator -p argo-identity --all-targets -- -D warnings` clean.
- [ ] PR body cites `round-55-pr-2279-kpi-rollup-charter-clobber-finding.md`.

## May-touch

- `crates/argo-identity/src/store.rs` (new fn `set_kpis`)
- `crates/argo-orchestrator/src/kpi_rollup.rs` (single line: replace `update_charter` call)
- `crates/argo-orchestrator/tests/kpi_rollup_test.rs` (add 6th test)
- This brief file (status updates)

## Out of scope

- Rewriting the rollup tick
- Adding kpi_json column constraints / migration
- Touching the bundled 5 tests

## Effort

~30-80 LOC + 1 test port. ~30-60 min.
