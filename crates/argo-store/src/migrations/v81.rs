//! Schema migration v81 — add `tenant_id` to `routing_decision`.
//!
//! The schema body lives in
//! `migrations/133__routing_decision_tenant_id.sql`; this wrapper
//! applies it through the versioned migration runner.
use argo_types::ArgoError;
use sqlx::Sqlite;

const V81_UP_SQL: &str = include_str!("../../migrations/133__routing_decision_tenant_id.sql");

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    let stripped: String = V81_UP_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v81 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V81_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v81 down: {e}")))?;
    }
    Ok(())
}

const V81_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_routing_decision_tenant_decided_at",
    "ALTER TABLE routing_decision DROP COLUMN tenant_id",
];
