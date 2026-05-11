//! Schema migration v53 — `skill_version_migration` table for
//! Stream `mission-skill-version-migration-path` (MISSION_BACKLOG
//! Section 3 Claim 4 #79).
//!
//! Publisher-side record marking v2 as the successor of v1, used by
//! `argo skill upgrade <id>` (tenant-side resolver) and the agent
//! session-boot hint that emits a `tracing::info!` per pending
//! migration so operators see "v2 of `analytics_dashboard` is
//! available" without auto-execution.
//!
//! Slot rationale: v49 (`mission-forge-cost-attribution`,
//! `forge_run_cost` columns) was the prior versioned slot when this
//! brief landed; v53 is the next free versioned slot. Reconciles raw
//! SQL `57__skill_version_migration.sql` (filename slots 49..56 are
//! taken by `worker_archived_at`, `worker_trust_grade`,
//! `audit_worker_deleted_at` + `worker_skill`, `persona_revision`,
//! and `forge_run_cost` respectively).
//!
//! UP creates the table with composite PK
//! `(skill_id, from_version, to_version)` plus a
//! `(skill_id, to_version)` secondary index for "what migrations
//! land me at v2?" lookups; the PK already covers
//! `(skill_id, from_version)` lookups via prefix match. DOWN drops
//! the index and table in reverse order. Idempotent via
//! `IF NOT EXISTS` / `IF EXISTS` guards.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V53_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v50 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V53_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v50 down: {e}")))?;
    }
    Ok(())
}

const V53_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS skill_version_migration (
        skill_id            BLOB NOT NULL,
        from_version        TEXT NOT NULL,
        to_version          TEXT NOT NULL,
        publisher_tenant_id BLOB NOT NULL,
        reason              TEXT,
        auto_upgrade        INTEGER NOT NULL DEFAULT 0,
        created_at          INTEGER NOT NULL,
        PRIMARY KEY (skill_id, from_version, to_version)
    )",
    "CREATE INDEX IF NOT EXISTS idx_skill_version_migration_to \
     ON skill_version_migration(skill_id, to_version)",
];

const V53_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_skill_version_migration_to",
    "DROP TABLE IF EXISTS skill_version_migration",
];
