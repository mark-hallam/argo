//! Schema migration v54 — `skill_rating` table for Stream
//! `mission-skill-ratings-publisher` (MISSION_BACKLOG Section 3
//! Claim 4 #80).
//!
//! Operator thumbs on marketplace-installed skills propagate beyond
//! the rater's tenant boundary by writing a row here keyed on
//! (skill_id, skill_version, rater_tenant_id). The
//! `publisher_tenant` column carries the publisher namespace string
//! parsed from the installed skill's `provenance.source` (format
//! `published:<grade>:<publisher>`); v1 has no first-class publisher
//! tenant id so the namespace handle is the closest stable join key
//! for the publisher dashboard aggregation that ships in a follow-up.
//!
//! Slot rationale: v50 (`mission-skill-min-trust-grade-column` PR
//! #1314), v51 (`mission-cross-tenant-trust-graph` PR #1302), v52
//! (`mission-delegation-budget-tree` PR #1315), and v53
//! (`mission-skill-version-migration-path` PR #1317) occupied the
//! prior versioned slots on `origin/main`, so v54 is the next free
//! versioned slot. Reconciles raw SQL `58__skill_rating.sql`:
//! filename slots 49 (`worker_archived_at`), 50 (`worker_trust_grade`),
//! 51 (`worker_skill` + `audit_worker_deleted_at`), 52
//! (`persona_revision`), 53 (`forge_run_cost`), 54
//! (`skill_min_trust_grade`), 55 (`trust_observation`), 56
//! (`subtask_budget`), and 57 (`skill_version_migration`) are taken
//! on origin/main; 58 is the next free raw filename slot.
//!
//! UP creates the table plus two query indexes:
//! `(skill_id, skill_version)` backs `aggregate_ratings`, and
//! `(publisher_tenant)` backs `list_for_publisher`. DOWN drops both
//! indexes and the table in reverse order. Idempotent via
//! `IF NOT EXISTS` / `IF EXISTS` guards.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V54_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v54 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V54_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v54 down: {e}")))?;
    }
    Ok(())
}

const V54_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS skill_rating (
        skill_id            BLOB NOT NULL,
        skill_version       TEXT NOT NULL,
        publisher_tenant    TEXT NOT NULL,
        rater_tenant_id     BLOB NOT NULL,
        vote                TEXT NOT NULL CHECK (vote IN ('up','down')),
        rated_at            INTEGER NOT NULL,
        comment             TEXT,
        PRIMARY KEY (skill_id, skill_version, rater_tenant_id)
    )",
    "CREATE INDEX IF NOT EXISTS idx_skill_rating_by_skill \
     ON skill_rating(skill_id, skill_version)",
    "CREATE INDEX IF NOT EXISTS idx_skill_rating_by_publisher \
     ON skill_rating(publisher_tenant)",
];

const V54_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_skill_rating_by_publisher",
    "DROP INDEX IF EXISTS idx_skill_rating_by_skill",
    "DROP TABLE IF EXISTS skill_rating",
];
