//! Schema migration v82 — add `worker.kpis_rolled_at` watermark for
//! `argo-orchestrator::kpi_rollup`. Stream
//! `mission-kpi-rollup-from-audit-trail`, mission claim 2 evidence.
//!
//! The schema body lives in `migrations/134__worker_kpis_rolled_at.sql`;
//! this wrapper applies it via the versioned migration runner so
//! existing workspaces upgrade through the same path as v80 / v81.
//!
//! Slot rationale: the brief reserved v81, but slot v81 landed first
//! as `routing_decision_tenant_id` (#2264). This brief rolls forward
//! to v82 per the swarm-protocol "second-to-land bumps" rule. The
//! raw filename slot is 134 — independently assigned (see
//! `scripts/check-migration-slots.sh`).
//!
//! Down-migration drops the index then the column. SQLite's
//! ALTER TABLE DROP COLUMN works for additive columns since 3.35
//! (April 2021); the runtime ships modern sqlx which targets a
//! recent enough libsqlite.
use argo_types::ArgoError;
use sqlx::Sqlite;

const V82_UP_SQL: &str = include_str!("../../migrations/134__worker_kpis_rolled_at.sql");

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    let stripped: String = V82_UP_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v82 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V82_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v82 down: {e}")))?;
    }
    Ok(())
}

const V82_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_worker_kpis_rolled_at",
    "ALTER TABLE worker DROP COLUMN kpis_rolled_at",
];
