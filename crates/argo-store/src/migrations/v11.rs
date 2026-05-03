//! Schema migration v11 — additive `skill.lifecycle_state`
//! column for the learned-skill lifecycle (#681, work order #3).
//!
//! Background: per the refined mission, learned skills must
//! flow through a review gate before becoming active:
//!
//!   draft → pending_review → active
//!                         ↘ rejected
//!                         ↘ archived
//!
//! Today every freshly-installed skill is immediately active
//! and reusable. That means an agent can synthesize a skill
//! during a turn and have it influence the very next turn —
//! no human review. This PR adds the column + the state
//! enum + the storage primitives. The agent runner's filter
//! (skip non-active rows in the lookup) lands in the same PR
//! to close the loop.
//!
//! Down migration drops the column.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V11_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v11 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V11_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v11 down: {e}")))?;
    }
    Ok(())
}

const V11_UP: &[&str] = &[
    // `lifecycle_state` defaults to 'active' for backward
    // compat — every existing skill row keeps its current
    // visibility. New learned skills installed via
    // `install_for_review` start as 'pending_review' and
    // require explicit `transition_skill_lifecycle` to flip.
    "ALTER TABLE skill ADD COLUMN lifecycle_state TEXT NOT NULL DEFAULT 'active'",
    // Hot path: per-vertical lookup with WHERE
    // lifecycle_state = 'active'.
    "CREATE INDEX idx_skill_lifecycle ON skill(lifecycle_state)",
];

const V11_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_skill_lifecycle",
    "ALTER TABLE skill DROP COLUMN lifecycle_state",
];
