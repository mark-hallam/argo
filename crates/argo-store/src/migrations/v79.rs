//! Schema migration v79 — add `tenant_id` to `skill_ab_test_run`.
//!
//! Brief `mission-skill-ab-test-run-table-add-tenant-id` (round-9
//! finding §2.1, #2081). The raw SQL lives in
//! `migrations/132__skill_ab_test_run_tenant_id.sql`; this wrapper
//! applies it via the versioned runner so existing workspaces upgrade
//! through the same path as v77 / v78.
//!
//! Slot rationale: the versioned counter is contiguous (the
//! migrations runner walks `current → current-1` on rollback and
//! errors if a slot is missing). The sibling brief
//! `mission-skill-table-add-tenant-id-column` also reserves v79 in
//! its in-flight branch; per the swarm protocol, the second to land
//! bumps to v80 since the first-landed `EXPECTED_SCHEMA_VERSION`
//! freezes the slot. The SQL filename is 132 — independently
//! assigned (see `scripts/check-migration-slots.sh`).
use argo_types::ArgoError;
use sqlx::Sqlite;

const V79_UP_SQL: &str = include_str!("../../migrations/132__skill_ab_test_run_tenant_id.sql");

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    let stripped: String = V79_UP_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v79 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V79_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v79 down: {e}")))?;
    }
    Ok(())
}

// Reverse the additive shape: drop the tenant-scoped indexes,
// peel off the `tenant_id` column (SQLite ≥ 3.35 supports
// `ALTER TABLE ... DROP COLUMN`), and restore the pre-v79
// status-only composite index. Mirrors the down-migration pattern
// in `v6` / `v9` / `v46` / `v71` / `v75`.
const V79_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_skill_ab_test_run_tenant_skill",
    "DROP INDEX IF EXISTS idx_skill_ab_test_run_status",
    "ALTER TABLE skill_ab_test_run DROP COLUMN tenant_id",
    "CREATE INDEX IF NOT EXISTS idx_skill_ab_test_run_status \
     ON skill_ab_test_run(status, started_at DESC)",
];
