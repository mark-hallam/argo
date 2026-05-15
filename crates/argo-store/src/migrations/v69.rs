//! Schema migration v69 — skill provenance log for
//! `mission-skill-explanation-provenance`.
//!
//! Raw SQL slot 69 is reserved for this brief in the live ledger
//! header. The migration backfills one `Bootstrap` row for every
//! existing `skill` version so `argo skill explain` works on old
//! workspaces immediately after migration.

use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V69_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v69 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V69_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v69 down: {e}")))?;
    }
    Ok(())
}

const V69_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS skill_provenance (
        skill_name      TEXT    NOT NULL,
        skill_version   TEXT    NOT NULL,
        provenance_kind TEXT    NOT NULL,
        actor           TEXT,
        parent_id       TEXT,
        change_reason   TEXT,
        breaking_change INTEGER NOT NULL DEFAULT 0,
        ts              INTEGER NOT NULL,
        PRIMARY KEY (skill_name, skill_version)
    )",
    "CREATE INDEX IF NOT EXISTS idx_skill_provenance_name \
     ON skill_provenance(skill_name)",
    "INSERT OR IGNORE INTO skill_provenance \
        (skill_name, skill_version, provenance_kind, actor, parent_id, \
         change_reason, breaking_change, ts) \
     SELECT name, version, 'Bootstrap', NULL, NULL, 'migration backfill', 0, created_at \
     FROM skill",
];

const V69_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_skill_provenance_name",
    "DROP TABLE IF EXISTS skill_provenance",
];
