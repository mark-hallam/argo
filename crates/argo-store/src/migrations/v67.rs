//! Schema migration v67 — `audit_record` filter columns + composite
//! indexes for Stream `mission-enterprise-audit-full-text-search`.
//!
//! Promotes the 7 audit-feed filter dimensions (worker / channel /
//! action_type / outcome_status / approver / cost_band /
//! classification) to first-class nullable columns so the
//! `GET /api/v1/audit` endpoint can pre-prune rows before the
//! canonical payload_json filter pass.
//!
//! Backfill posture: nullable, no backfill. Historical rows stay NULL
//! so audit-chain hashes remain valid; new writes populate the columns
//! from the same payload fields the reader already parses.
//!
//! Slot rationale: current main is at v66. Raw filename slot is 77
//! because slots 65, 66, and 72-76 are already occupied on main.
//! Filename and version counters advance independently per the v55
//! module's documented convention.
//!
//! DOWN drops the indexes and otherwise keeps the columns in place:
//! SQLite versions before 3.35 cannot drop columns, and rollback past
//! this migration is best-effort like earlier additive column slots.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V67_UP {
        match sqlx::query(sql).execute(&mut **tx).await {
            Ok(_) => {}
            Err(e) if is_duplicate_column(&e) => {}
            Err(e) => return Err(ArgoError::Provider(format!("v67 up: {e}"))),
        }
    }
    Ok(())
}

fn is_duplicate_column(err: &sqlx::Error) -> bool {
    err.to_string().contains("duplicate column name")
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V67_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v67 down: {e}")))?;
    }
    Ok(())
}

const V67_UP: &[&str] = &[
    "ALTER TABLE audit_record ADD COLUMN worker_id TEXT",
    "ALTER TABLE audit_record ADD COLUMN channel TEXT",
    "ALTER TABLE audit_record ADD COLUMN action_type TEXT",
    "ALTER TABLE audit_record ADD COLUMN outcome_status TEXT",
    "ALTER TABLE audit_record ADD COLUMN approver_id TEXT",
    "ALTER TABLE audit_record ADD COLUMN cost_band TEXT",
    "ALTER TABLE audit_record ADD COLUMN data_classification TEXT",
    "CREATE INDEX IF NOT EXISTS idx_audit_record_created_at_worker \
     ON audit_record(created_at, worker_id)",
    "CREATE INDEX IF NOT EXISTS idx_audit_record_created_at_action \
     ON audit_record(created_at, action_type)",
    "CREATE INDEX IF NOT EXISTS idx_audit_record_created_at_status \
     ON audit_record(created_at, outcome_status)",
];

const V67_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_audit_record_created_at_worker",
    "DROP INDEX IF EXISTS idx_audit_record_created_at_action",
    "DROP INDEX IF EXISTS idx_audit_record_created_at_status",
];
