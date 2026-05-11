//! Schema migration v55 — `skill_flag` table for Stream
//! `mission-forge-operator-feedback-eval-signal` (MISSION_BACKLOG
//! Section 3 Claim 4 #85).
//!
//! Generic per-skill flag store consulted by the Forge ingestion
//! path before pulling operator feedback as eval cases. Today the
//! only flag is `no-forge-feedback` (`1` = skip ingestion); future
//! per-skill flags piggyback on the same table.
//!
//! Slot rationale: v50 (`skill_min_trust_grade`, brief
//! `mission-skill-min-trust-grade-column` PR #1314) and v51
//! (`trust_observation`, brief `mission-cross-tenant-trust-graph`
//! PR #1302) occupied the prior versioned slots when this brief
//! rebased onto current origin/main (head `156e03bf`), so v52 is
//! the next free versioned slot. Reconciles raw SQL
//! `59__skill_flag.sql`: filename slot 54 is taken by
//! `skill_min_trust_grade` and 55 by `trust_observation`, so the
//! raw counter rolls forward to 56. The brief was authored against
//! origin/main `d50e69b2`; #1302, #1312, #1314 merged mid-flight,
//! and the v52/raw 56 retarget is the standard slot-amendment
//! pattern documented in #1302 / #1314 BUILD_STATUS notes.
//!
//! UP creates the table + a `(flag)` index that backs the
//! "list all skills with flag X" lookup used by the Forge
//! ingestion path. Idempotent via `IF NOT EXISTS` guards.
//! DOWN drops both in reverse order.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V52_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v52 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V52_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v52 down: {e}")))?;
    }
    Ok(())
}

const V52_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS skill_flag (
        skill_id        TEXT NOT NULL,
        flag            TEXT NOT NULL,
        value           INTEGER NOT NULL,
        PRIMARY KEY (skill_id, flag)
    )",
    "CREATE INDEX IF NOT EXISTS idx_skill_flag_flag \
     ON skill_flag(flag)",
];

const V52_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_skill_flag_flag",
    "DROP TABLE IF EXISTS skill_flag",
];
