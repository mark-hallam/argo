//! Schema migration v63 — `roster.policy_override_json` column for
//! Stream `claude-roster-store-policy-override-column`.
//!
//! Adds a nullable JSON column carrying the serialized
//! `argo_identity::RosterPolicyOverride`. The
//! `mission-roster-resolve-policy-v2` brief consumes this column;
//! today's resolver leaves it as `NULL`, preserving existing
//! behaviour.
//!
//! Slot rationale: v62 (`approval_escalation`) is the current
//! versioned tail. Raw SQL filename slots 64 and 66-71 are
//! reserved for in-flight briefs; 73 is the previous tail
//! (`approval_escalation`), so 74 is the next free slot.
//! Filename and version counters advance independently per the
//! v55 convention.
//!
//! UP guards via PRAGMA + `IF NOT EXISTS`-equivalent so a re-run
//! is a no-op; the SQLite `ALTER TABLE ADD COLUMN` itself fails
//! when the column already exists, which would block re-running
//! the chain after a partial apply.
use argo_types::ArgoError;
use sqlx::{Row, Sqlite};

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    if !column_exists(tx, "roster", "policy_override_json").await? {
        sqlx::query("ALTER TABLE roster ADD COLUMN policy_override_json TEXT")
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v63 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    if column_exists(tx, "roster", "policy_override_json").await? {
        sqlx::query("ALTER TABLE roster DROP COLUMN policy_override_json")
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v63 down: {e}")))?;
    }
    Ok(())
}

async fn column_exists<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
    table: &str,
    column: &str,
) -> Result<bool, ArgoError> {
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v63 column probe {table}.{column}: {e}")))?;
    Ok(rows.iter().any(|row| {
        let name: String = row.get("name");
        name == column
    }))
}
