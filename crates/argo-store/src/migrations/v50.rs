//! Schema migration v50 — additive `skill.min_trust_grade`
//! (INTEGER, nullable) column for Stream
//! `mission-skill-min-trust-grade-column` (Tier M6 precursor 1/3).
//!
//! Unblocks `mission-skill-propagation-loop` by giving the discovery
//! pipeline a skill-side gate the M6 chain can join against
//! `worker.trust_grade` (added in v46).
//!
//! `min_trust_grade` is NULL by default — "no restriction" is the
//! v1 default and operators opt in to gating by setting a value via
//! `SkillStore::set_min_trust_grade`. The integer stores the
//! `argo_skill::trust::TrustGrade` enum's numeric discriminant; the
//! Rust setter/getter round-trips the typed enum.
//!
//! v49 (`forge_run_cost`, brief `mission-forge-cost-attribution` PR
//! #1313) occupied the prior versioned slot when this brief landed;
//! v50 is the next free slot. Reconciles raw SQL
//! `54__skill_min_trust_grade.sql` (filename slot 54 picks the next
//! free SQL filename — 49 / 50 / 51 / 52 / 53 are taken by
//! `worker_archived_at` / `worker_trust_grade` /
//! `worker_skill`+`audit_worker_deleted_at` / `persona_revision` /
//! `forge_run_cost` respectively, and the two counters move
//! independently per the v42 + v43 docstrings).
//!
//! UP probes existing columns via `PRAGMA table_info(skill)` before
//! `ALTER TABLE` so the migration is idempotent when the raw SQL
//! file has already been replayed (mirrors v45's `column_exists`
//! pattern). DOWN drops the column; SQLite >= 3.35 supports
//! `ALTER TABLE ... DROP COLUMN` and the column is nullable with no
//! FK references so dropping is safe.
use argo_types::ArgoError;
use sqlx::{Row, Sqlite};

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    if !column_exists(tx, "skill", "min_trust_grade").await? {
        exec(
            tx,
            "v50 up",
            "ALTER TABLE skill ADD COLUMN min_trust_grade INTEGER",
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    if column_exists(tx, "skill", "min_trust_grade").await? {
        exec(
            tx,
            "v50 down",
            "ALTER TABLE skill DROP COLUMN min_trust_grade",
        )
        .await?;
    }
    Ok(())
}

async fn exec<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
    ctx: &str,
    sql: &str,
) -> Result<(), ArgoError> {
    sqlx::query(sql)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("{ctx}: {e}")))?;
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
        .map_err(|e| ArgoError::Provider(format!("v50 column probe {table}.{column}: {e}")))?;
    Ok(rows.iter().any(|row| {
        let name: String = row.get("name");
        name == column
    }))
}
