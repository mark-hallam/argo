//! Schema migration v78 — `workflow_test_run` persistence table.
//! Stream `claude-followup-pr-2021-test-run-gateway-persist` (re-
//! author of closed PR #2021 after the 129/v77 slot collision).
//!
//! The schema body lives in `migrations/130__workflow_test_run.sql`;
//! this wrapper applies it via the versioned migration runner so
//! existing workspaces upgrade through the same path as v77 / v76.
//!
//! Slot rationale: v77 (`workplace_knowledge`) is the highest
//! versioned slot on origin/main; raw SQL filename 129 is also taken
//! by that brief. This re-author takes the next free counters in
//! both axes: filename 130 + versioned v78. Filename and version
//! counters advance independently per the v55 convention.
use argo_types::ArgoError;
use sqlx::Sqlite;

const V78_UP_SQL: &str = include_str!("../../migrations/130__workflow_test_run.sql");

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    let stripped: String = V78_UP_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v78 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V78_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v78 down: {e}")))?;
    }
    Ok(())
}

const V78_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_workflow_test_run_lookup",
    "DROP TABLE IF EXISTS workflow_test_run",
];
