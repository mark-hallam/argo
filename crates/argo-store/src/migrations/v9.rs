//! Schema migration v9 — additive `skill.vertical_id TEXT`
//! column for vertical-scoped skill lookup (#678).
//!
//! Background: today the `skill` table has only `learned_in`
//! (a session id, NULL for bundled skills). Bundled skills end
//! up workspace-global — a session bound to vertical A could
//! see a skill bundled by vertical B if both ran in the same
//! workspace. This violates the refined-mission rule 5
//! ("Bundled skills belong only to the vertical that shipped
//! them").
//!
//! This migration adds an additive `vertical_id TEXT NULL`
//! column. Existing skill rows stay valid (column nullable,
//! default NULL). The `vertical_attach::install_and_attach_bundle`
//! handler (#653) is updated to populate it on install; new
//! `SkillStore::list_skills_bundled_by_vertical` reads from it
//! to filter at lookup time.
//!
//! Down migration drops the column + the supporting index.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V9_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v9 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V9_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v9 down: {e}")))?;
    }
    Ok(())
}

const V9_UP: &[&str] = &[
    "ALTER TABLE skill ADD COLUMN vertical_id TEXT",
    // Hot path: per-vertical skill lookup at session-bind
    // time. Index `vertical_id` so the WHERE filter is O(rows
    // bundled by vertical) instead of O(all skills).
    "CREATE INDEX idx_skill_vertical_id ON skill(vertical_id)",
];

const V9_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_skill_vertical_id",
    "ALTER TABLE skill DROP COLUMN vertical_id",
];
