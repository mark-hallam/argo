//! Schema migration v51 — cross-tenant trust-graph observation
//! table (Stream `mission-cross-tenant-trust-graph`, MISSION_BACKLOG
//! Section 3 Claim 4 #81).
//!
//! Wraps raw SQL `55__trust_observation.sql` so a single source of
//! truth covers both `argo-store::SessionStore::migrate` and any
//! crate-scoped `ensure_schema` callers. Mirrors v43 (`skill_canary`)
//! and v44 (`approvals`) precedents.
//!
//! Slot ordering: v48 (`persona_revision` PR #1299), v49
//! (`forge_run_cost` PR #1313), and v50 (`skill_min_trust_grade`
//! PR #1314) occupied the versioned slots that were free when this
//! brief first landed, so v51 is the next free versioned slot. The
//! raw SQL filename lands at 55 because raw slots 52..=54 are taken
//! by `persona_revision.sql`, `forge_run_cost.sql`, and
//! `skill_min_trust_grade.sql` respectively. Per the v42/v43
//! docstrings, the two counters move independently.
use argo_types::ArgoError;
use sqlx::Sqlite;

const V51_UP_SQL: &str = include_str!("../../migrations/55__trust_observation.sql");

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for stmt in V51_UP_SQL.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v51 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V51_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v51 down: {e}")))?;
    }
    Ok(())
}

const V51_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS trust_observation_observed_at_idx",
    "DROP INDEX IF EXISTS trust_observation_source_idx",
    "DROP INDEX IF EXISTS trust_observation_consumer_idx",
    "DROP TABLE IF EXISTS trust_observation",
];
