//! Schema migration v47 — `worker_skill` worker↔skill association
//! table (Stream `mvp-worker-skills-table`). Tier M prereq —
//! unblocks `mission-routing-skillmatch-rolematch` (M4) by giving
//! the routing engine a typed lookup for
//! `TargetSelector::SkillMatch(String)` resolution.
//!
//! Wraps raw SQL `51__worker_skill.sql` so a single source of truth
//! covers both `argo-store::SessionStore::migrate` and the
//! crate-scoped `ensure_schema` pattern in `argo-identity`. Mirrors
//! `v43` (`skill_canary` → `47__skill_canary.sql`) and `v44`
//! (`approvals` → `48__approvals.sql`).
//!
//! Slot ordering: v47 is the next free versioned slot after v46
//! (`worker_trust_grade`). The raw SQL filename lands at 51 because
//! raw slot 49 is taken by `worker_archived_at` (PR #1271) and slot
//! 50 is taken by `worker_trust_grade` (PR #1273). Per the v42/v43
//! docstrings, the two counters move independently.
use argo_types::ArgoError;
use sqlx::Sqlite;

const V47_UP_SQL: &str = include_str!("../../migrations/51__worker_skill.sql");

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for stmt in V47_UP_SQL.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v47 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V47_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v47 down: {e}")))?;
    }
    Ok(())
}

const V47_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS worker_skill_skill_idx",
    "DROP TABLE IF EXISTS worker_skill",
];
