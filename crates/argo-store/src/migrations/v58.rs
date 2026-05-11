//! Schema migration v58 — `operator_risk_overrides` table for Stream
//! `mission-operator-specific-risk-overrides` (MISSION_BACKLOG
//! Section 3 Claim 6 #122).
//!
//! Per-operator override of the manifest-driven default risk
//! classification. The default policy classifies every action via
//! the bound vertical's `[actions.<name>].risk` declaration; an
//! operator can replace that default for a single
//! `(action_kind, vertical_id)` pair by upserting a row here.
//! `vertical_id == ''` means "apply for every vertical this operator
//! touches"; per-vertical wins over all-verticals.
//!
//! Slot rationale: v57 (`worker_role_name`, brief
//! `mission-routing-skillmatch-rolematch`) was the prior tail, so v58
//! is the next free versioned slot. The raw SQL filename slot is 62
//! (after 61 = `worker_role_name`) — filename + version counters
//! advance independently, per the v55 module's documented convention.
//!
//! UP creates the table + supporting operator index. Idempotent via
//! `IF NOT EXISTS`. DOWN drops the index then the table in reverse
//! order.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V58_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v58 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V58_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v58 down: {e}")))?;
    }
    Ok(())
}

const V58_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS operator_risk_overrides (
        operator_id      TEXT    NOT NULL,
        action_kind      TEXT    NOT NULL,
        vertical_id      TEXT    NOT NULL DEFAULT '',
        risk_level       TEXT    NOT NULL,
        allow_downgrade  INTEGER NOT NULL DEFAULT 0,
        created_at       INTEGER NOT NULL,
        PRIMARY KEY (operator_id, action_kind, vertical_id)
    )",
    "CREATE INDEX IF NOT EXISTS idx_operator_risk_overrides_operator \
     ON operator_risk_overrides(operator_id)",
];

const V58_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_operator_risk_overrides_operator",
    "DROP TABLE IF EXISTS operator_risk_overrides",
];
