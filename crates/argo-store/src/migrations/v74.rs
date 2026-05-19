//! Schema migration v74 — `trust_streak_event` view.
//!
//! Stream `mission-pm-trust-ratchet-ui` (P2 #11). Read-only view that
//! filters the audit chain down to the four `approval.*` kinds the
//! streak aggregator walks. The streak count itself is computed in
//! Rust (`crates/argo-store/src/trust_streak.rs`) because SQLite's
//! gap-and-island primitives can't express "reset on denial OR
//! revocation" without a CTE chain that would dwarf the Rust pass.
//!
//! Slot rationale: v73 is the highest registered version on
//! origin/main (`worker_freeze`, slot 119). Raw SQL slot 121
//! (`skill_reeval_history`) is the highest committed filename; slot
//! 122 is the next free slot. Filename and version counters advance
//! independently per the v55 convention.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V74_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v74 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V74_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v74 down: {e}")))?;
    }
    Ok(())
}

const V74_UP: &[&str] = &[
    "CREATE VIEW IF NOT EXISTS trust_streak_event AS \
     SELECT seq, id, kind, worker_id, action_type, payload_json, created_at \
     FROM audit_record \
     WHERE kind IN ('approval.granted', 'approval.revoked', 'approval.consumed', 'approval.replied') \
       AND worker_id IS NOT NULL",
];

const V74_DOWN: &[&str] = &["DROP VIEW IF EXISTS trust_streak_event"];
